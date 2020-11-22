use std::{
    cell::RefCell, collections::HashMap, error::Error, fs::File, io::Write, num::NonZeroU64,
};

use crates_index::{BareIndexRepo, DependencyKind};
use pubgrub::{
    error::PubGrubError,
    range::Range,
    report::{DefaultStringReporter, Reporter},
    solver::choose_package_with_fewest_versions,
    solver::resolve,
    solver::{Dependencies, DependencyConstraints, DependencyProvider, OfflineDependencyProvider},
    version::SemanticVersion,
    version::Version,
};
use ron::ser::PrettyConfig;

#[derive(Hash, Eq, PartialEq, Clone, Ord, PartialOrd)]
struct Semantic {
    semver: semver::Version,
    pubgrub: SemanticVersion,
}

impl serde::Serialize for Semantic {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.pubgrub.serialize(serializer)
    }
}

impl std::fmt::Display for Semantic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.semver.fmt(f)
    }
}

impl std::fmt::Debug for Semantic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.semver, f)
    }
}

impl Semantic {
    fn new(ver: &str) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            semver: ver.parse()?,
            pubgrub: ver.parse()?,
        })
    }
}

impl pubgrub::version::Version for Semantic {
    fn lowest() -> Self {
        let pubgrub = SemanticVersion::lowest();
        let semver = pubgrub.to_string().parse().unwrap();
        Self { semver, pubgrub }
    }

    fn bump(&self) -> Self {
        let pubgrub = self.pubgrub.bump();
        let semver = pubgrub.to_string().parse().unwrap();
        Self { semver, pubgrub }
    }
}

/// A type that represents when cargo treats two Versions as compatible.
/// Versions `a` and `b` are compatible if their left-most nonzero digit is the
/// same.
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug, PartialOrd, Ord)]
pub enum SemverCompatibility {
    Major(NonZeroU64),
    Minor(NonZeroU64),
    Patch(u64),
}

impl From<&semver::Version> for SemverCompatibility {
    fn from(ver: &semver::Version) -> Self {
        if let Some(m) = NonZeroU64::new(ver.major) {
            return SemverCompatibility::Major(m);
        }
        if let Some(m) = NonZeroU64::new(ver.minor) {
            return SemverCompatibility::Minor(m);
        }
        SemverCompatibility::Patch(ver.patch)
    }
}

impl From<&Semantic> for SemverCompatibility {
    fn from(ver: &Semantic) -> Self {
        SemverCompatibility::from(&ver.semver)
    }
}

impl From<&SemverCompatibility> for Semantic {
    fn from(ver: &SemverCompatibility) -> Self {
        Semantic::new(&match ver {
            SemverCompatibility::Major(i) => format!("{}.0.0", i),
            SemverCompatibility::Minor(i) => format!("0.{}.0", i),
            SemverCompatibility::Patch(i) => format!("0.0.{}", i),
        })
        .unwrap()
    }
}

#[derive(Clone, Eq, PartialEq, Hash)]
enum Names {
    Bucket(String, SemverCompatibility, bool),
    BucketFeatures(String, SemverCompatibility, String),
    Wide(String, semver::VersionReq, String, SemverCompatibility),
    WideFeatures(
        String,
        semver::VersionReq,
        String,
        SemverCompatibility,
        String,
    ),
    Links(String),
}

impl Names {
    fn crate_(&self) -> &str {
        match self {
            Names::Bucket(c, _, _) => c,
            Names::BucketFeatures(c, _, _) => c,
            Names::Wide(c, _, _, _) => c,
            Names::WideFeatures(c, _, _, _, _) => c,
            Names::Links(_) => panic!(),
        }
    }
    fn with_features(&self, feat: impl Into<String>) -> Self {
        let feat = feat.into();
        match self {
            Names::Bucket(a, b, _) => Names::BucketFeatures(a.clone(), b.clone(), feat),
            Names::BucketFeatures(a, b, _) => Names::BucketFeatures(a.clone(), b.clone(), feat),
            Names::Wide(a, b, c, d) => {
                Names::WideFeatures(a.clone(), b.clone(), c.clone(), d.clone(), feat)
            }
            Names::WideFeatures(a, b, c, d, _) => {
                Names::WideFeatures(a.clone(), b.clone(), c.clone(), d.clone(), feat)
            }
            Names::Links(_) => panic!(),
        }
    }
}

impl std::fmt::Display for Names {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Names::Bucket(n, m, a) => {
                f.write_str("Bucket:")?;
                f.write_str(n)?;
                f.write_str("@")?;
                f.write_str(&match m {
                    SemverCompatibility::Major(i) => format!("{}.x.y", i),
                    SemverCompatibility::Minor(i) => format!("0.{}.x", i),
                    SemverCompatibility::Patch(i) => format!("0.0.{}", i),
                })?;
                if *a {
                    f.write_str("/All-FEATURES")?;
                }
                Ok(())
            }
            Names::BucketFeatures(n, m, o) => {
                f.write_str("Bucket:")?;
                f.write_str(n)?;
                f.write_str("@")?;
                f.write_str(&match m {
                    SemverCompatibility::Major(i) => format!("{}.x.y", i),
                    SemverCompatibility::Minor(i) => format!("0.{}.x", i),
                    SemverCompatibility::Patch(i) => format!("0.0.{}", i),
                })?;
                f.write_str("/")?;
                f.write_str(o)
            }
            Names::Wide(c, range, parent, parent_com) => {
                f.write_str("Range:")?;
                f.write_str(c)?;
                f.write_str("(From:")?;
                f.write_str(parent)?;
                f.write_str("@")?;
                f.write_str(&match parent_com {
                    SemverCompatibility::Major(i) => format!("{}.x.y", i),
                    SemverCompatibility::Minor(i) => format!("0.{}.x", i),
                    SemverCompatibility::Patch(i) => format!("0.0.{}", i),
                })?;
                f.write_str("):")?;
                f.write_str(&range.to_string())
            }
            Names::WideFeatures(c, range, parent, parent_com, feat) => {
                f.write_str("Range:")?;
                f.write_str(c)?;
                f.write_str("(From:")?;
                f.write_str(parent)?;
                f.write_str("@")?;
                f.write_str(&match parent_com {
                    SemverCompatibility::Major(i) => format!("{}.x.y", i),
                    SemverCompatibility::Minor(i) => format!("0.{}.x", i),
                    SemverCompatibility::Patch(i) => format!("0.0.{}", i),
                })?;
                f.write_str("):")?;
                f.write_str(&range.to_string())?;
                f.write_str("/")?;
                f.write_str(feat)
            }
            Names::Links(name) => {
                f.write_str("Links:")?;
                f.write_str(name)
            }
        }
    }
}

impl std::fmt::Debug for Names {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self, f)
    }
}

impl serde::Serialize for Names {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

struct Index<'a> {
    index: &'a BareIndexRepo<'a>,
    versions: RefCell<HashMap<String, &'static [Semantic]>>,
    dependencies: RefCell<HashMap<(Names, Semantic), Dependencies<Names, Semantic>>>,
    links: RefCell<HashMap<String, usize>>,
}

impl<'a> Index<'a> {
    fn new(index: &'a BareIndexRepo<'a>) -> Self {
        Self {
            index,
            versions: Default::default(),
            dependencies: Default::default(),
            links: Default::default(),
        }
    }

    fn get_versions(&self, name: &str) -> &'static [Semantic] {
        self.versions
            .borrow_mut()
            .entry(name.to_string())
            .or_insert_with(|| {
                if let Some(cra) = self.index.crate_(name) {
                    let mut out: Vec<Semantic> = cra
                        .versions()
                        .iter()
                        .filter_map(|v| Semantic::new(v.version()).ok())
                        .collect();
                    out.sort();
                    out.reverse();
                    out.leak()
                } else {
                    &mut []
                }
            })
            .clone()
    }
}

impl<'a> DependencyProvider<Names, Semantic> for Index<'a> {
    fn choose_package_version<
        T: std::borrow::Borrow<Names>,
        U: std::borrow::Borrow<Range<Semantic>>,
    >(
        &self,
        potential_packages: impl Iterator<Item = (T, U)>,
    ) -> Result<(T, Option<Semantic>), Box<dyn Error>> {
        Ok(choose_package_with_fewest_versions(
            |p| match p {
                Names::Links(name) => Box::new(
                    (0..self.links.borrow().get(name).cloned().unwrap_or(0))
                        .map(|i| Semantic::new(&format!("{}.0.0", i)).unwrap()),
                ) as Box<dyn Iterator<Item = Semantic>>,
                Names::Wide(_, req, _, _) | Names::WideFeatures(_, req, _, _, _) => Box::new(
                    // one version for each bucket that match req
                    self.get_versions(p.crate_())
                        .into_iter()
                        .filter(|v| req.matches(&v.semver))
                        .map(|v| SemverCompatibility::from(v))
                        .collect::<std::collections::BTreeSet<_>>()
                        .into_iter()
                        .rev()
                        .map(|v| Semantic::from(&v)),
                )
                    as Box<dyn Iterator<Item = Semantic>>,
                _ => Box::new(self.get_versions(p.crate_()).into_iter().cloned())
                    as Box<dyn Iterator<Item = Semantic>>,
            },
            potential_packages,
        ))
    }

    fn get_dependencies(
        &self,
        package: &Names,
        version: &Semantic,
    ) -> Result<Dependencies<Names, Semantic>, Box<dyn Error>> {
        Ok(self
            .dependencies
            .borrow_mut()
            .entry((package.clone(), version.clone()))
            .or_insert_with(|| match package {
                Names::Bucket(name, major, all_features) => {
                    assert_eq!(major, &SemverCompatibility::from(version));
                    let index_ver = self
                        .index
                        .crate_(name)
                        .unwrap()
                        .versions()
                        .iter()
                        .find(|v| Semantic::new(v.version()).ok().as_ref() == Some(version))
                        .cloned()
                        .unwrap();
                    if index_ver.is_yanked() {
                        return Dependencies::Unknown;
                    }
                    let mut deps = DependencyConstraints::default();
                    if let Some(link) = index_ver.links() {
                        let mut links = self.links.borrow_mut();
                        let i = links.entry(link.to_string()).or_default();
                        deps.insert(
                            Names::Links(link.to_owned()),
                            Range::exact(Semantic::new(&format!("{}.0.0", i)).unwrap()),
                        );
                        *i += 1;
                    }
                    for dep in index_ver.dependencies() {
                        if dep.kind() == DependencyKind::Dev {
                            continue;
                        }
                        if dep.is_optional() && !all_features {
                            continue; // handled in Names::Features
                        }
                        let dep_requirement: semver::VersionReq =
                            if let Ok(x) = dep.requirement().parse() {
                                x
                            } else {
                                return Dependencies::Unknown;
                            };
                        let cray = Names::Wide(
                            dep.crate_name().to_owned(),
                            dep_requirement.clone(),
                            package.crate_().to_owned(),
                            SemverCompatibility::from(version),
                        );
                        deps.insert(cray.clone(), Range::any());

                        if dep.has_default_features() {
                            deps.insert(cray.with_features("default"), Range::any());
                        }
                        for f in dep.features() {
                            deps.insert(cray.with_features(f), Range::any());
                        }
                    }
                    Dependencies::Known(deps)
                }
                Names::BucketFeatures(name, major, feat) => {
                    assert_eq!(major, &SemverCompatibility::from(version));
                    let index_ver = self
                        .index
                        .crate_(name)
                        .unwrap()
                        .versions()
                        .iter()
                        .find(|v| Semantic::new(v.version()).ok().as_ref() == Some(version))
                        .cloned()
                        .unwrap();
                    if index_ver.is_yanked() {
                        return Dependencies::Unknown;
                    }
                    let mut compatibilitys: HashMap<String, Vec<(String, _)>> = HashMap::new();

                    for dep in index_ver.dependencies() {
                        if dep.kind() == DependencyKind::Dev {
                            continue;
                        }
                        let dep_requirement: semver::VersionReq =
                            if let Ok(x) = dep.requirement().parse() {
                                x
                            } else {
                                return Dependencies::Unknown;
                            };
                        compatibilitys
                            .entry(dep.name().to_owned())
                            .or_default()
                            .push((dep.crate_name().to_owned(), dep_requirement.clone()));

                        if dep.is_optional() && dep.name() == feat {
                            let mut deps = DependencyConstraints::default();
                            let cray = Names::Wide(
                                dep.crate_name().to_owned(),
                                dep_requirement.clone(),
                                package.crate_().to_owned(),
                                SemverCompatibility::from(version),
                            );
                            deps.insert(cray.clone(), Range::any());

                            if dep.has_default_features() {
                                deps.insert(cray.with_features("default"), Range::any());
                            }
                            for f in dep.features() {
                                deps.insert(cray.with_features(f), Range::any());
                            }

                            return Dependencies::Known(deps);
                        }
                    }

                    if let Some(vals) = index_ver.features().get(feat) {
                        return Dependencies::Known({
                            let mut deps = DependencyConstraints::default();
                            deps.insert(
                                Names::Bucket(
                                    name.clone(),
                                    SemverCompatibility::from(version),
                                    false,
                                ),
                                Range::exact(version.clone()),
                            );
                            for val in vals {
                                if val.contains('/') {
                                    let val: Vec<_> = val.split('/').collect();
                                    assert!(val.len() == 2);
                                    for com in compatibilitys.get(val[0]).unwrap() {
                                        deps.insert(
                                            Names::WideFeatures(
                                                com.0.clone(),
                                                com.1.clone(),
                                                package.crate_().to_owned(),
                                                SemverCompatibility::from(version),
                                                val[1].to_string(),
                                            ),
                                            Range::any(),
                                        );
                                    }
                                } else {
                                    deps.insert(
                                        Names::BucketFeatures(
                                            name.clone(),
                                            SemverCompatibility::from(version),
                                            val.to_string(),
                                        ),
                                        Range::exact(version.clone()),
                                    );
                                }
                            }

                            deps
                        });
                    }
                    if feat == "default" {
                        return Dependencies::Known(
                            Some((
                                Names::Bucket(
                                    name.clone(),
                                    SemverCompatibility::from(version),
                                    false,
                                ),
                                Range::exact(version.clone()),
                            ))
                            .into_iter()
                            .collect(),
                        );
                    }
                    Dependencies::Unknown
                }
                Names::Wide(_, req, _, _) => {
                    let compatibility = SemverCompatibility::from(version);
                    let vers: Vec<_> = self
                        .get_versions(package.crate_())
                        .into_iter()
                        .filter(|v| req.matches(&v.semver))
                        .filter(|v| SemverCompatibility::from(*v) == compatibility)
                        .cloned()
                        .collect();
                    if vers.is_empty() {
                        return Dependencies::Unknown;
                    }

                    let min = vers.iter().min().unwrap().clone().clone();
                    let max = vers.iter().max().unwrap();

                    let range = Range::between(min, max.bump());
                    let mut deps = DependencyConstraints::default();

                    deps.insert(
                        Names::Bucket(package.crate_().to_owned(), compatibility, false),
                        range.clone(),
                    );
                    Dependencies::Known(deps)
                }
                Names::WideFeatures(_, req, parent, parent_com, feat) => {
                    let compatibility = SemverCompatibility::from(version);
                    let vers: Vec<_> = self
                        .get_versions(package.crate_())
                        .into_iter()
                        .filter(|v| req.matches(&v.semver))
                        .filter(|v| SemverCompatibility::from(*v) == compatibility)
                        .cloned()
                        .collect();
                    if vers.is_empty() {
                        return Dependencies::Unknown;
                    }

                    let min = vers.iter().min().unwrap().clone().clone();
                    let max = vers.iter().max().unwrap();

                    let range = Range::between(min, max.bump());
                    let mut deps = DependencyConstraints::default();

                    deps.insert(
                        Names::Wide(
                            package.crate_().to_owned(),
                            req.clone(),
                            parent.clone(),
                            parent_com.clone(),
                        ),
                        Range::exact(version.clone()),
                    );

                    deps.insert(
                        Names::BucketFeatures(
                            package.crate_().to_owned(),
                            compatibility,
                            feat.clone(),
                        ),
                        range.clone(),
                    );
                    Dependencies::Known(deps)
                }
                Names::Links(_) => Dependencies::Known(DependencyConstraints::default()),
            })
            .clone())
    }
}

fn main() {
    let index = crates_index::BareIndex::new_cargo_default();
    let index = index.open_or_clone().unwrap();
    let dp = Index::new(&index);

    if let Some(ver) = index.crate_("zuse").unwrap().versions().last() {
        if ver.dependencies().len() > 1 {
            if let Ok(sem) = Semantic::new(ver.version()) {
                dbg!(ver.name());
                match resolve(
                    &dp,
                    Names::Bucket(ver.name().to_owned(), SemverCompatibility::from(&sem), true),
                    sem,
                ) {
                    Ok(map) => {
                        dbg!(map);
                    }

                    Err(PubGrubError::NoSolution(derivation)) => {
                        eprintln!("{}", DefaultStringReporter::report(&derivation));
                    }
                    Err(_) => {
                        dbg!("Err");
                    }
                }
            }
        }
    }

    for crt in index.crates() {
        if let Some(ver) = crt.versions().last() {
            if ver.dependencies().len() > 1 {
                if let Ok(sem) = Semantic::new(ver.version()) {
                    dbg!(ver.name());
                    let _ = resolve(
                        &dp,
                        Names::Bucket(ver.name().to_owned(), SemverCompatibility::from(&sem), true),
                        sem,
                    );
                }
            }
        }
    }

    let mut dependency_provider = OfflineDependencyProvider::new();
    for ((package, version), deps) in dp.dependencies.borrow().iter() {
        if let Dependencies::Known(dependencies) = deps {
            dependency_provider.add_dependencies(
                package.clone(),
                version.clone(),
                dependencies.clone(),
            );
        }
    }

    let out = ron::ser::to_string_pretty(&dependency_provider, PrettyConfig::new()).unwrap();
    let mut file = File::create(&("crates_io_str_SemanticVersion.ron")).unwrap();
    file.write_all(out.as_bytes()).unwrap();
}
