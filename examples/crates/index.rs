use core::borrow::Borrow;
use crates_index;
use pubgrub::range::Range;
use pubgrub::solver::{Dependencies, DependencyProvider};
use pubgrub::type_aliases::Map;
use pubgrub::version::SemanticVersion as V;
use semver::{ReqParseError, SemVerError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet as Set};
use std::convert::TryFrom;
use thiserror::Error;

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Index {
    pub crates: Map<Crate, BTreeMap<V, CrateDeps>>,
}

#[derive(Serialize, Deserialize)]
pub struct CrateDeps {
    pub mandatory_deps: Deps,
    pub feature_deps: Map<Feature, Deps>,
}

#[derive(Serialize, Deserialize)]
pub struct Deps {
    normal: Map<Crate, Dep>,
    dev: Map<Crate, Dep>,
    build: Map<Crate, Dep>,
}

#[derive(Serialize, Deserialize)]
struct Dep {
    range: Range<V>,
    features: Set<Feature>,
}

pub type Feature = String;
pub type Crate = String;

#[derive(Error, Debug)]
pub enum ConvertCrateError {
    #[error("version parse error")]
    VersionParseError(SemVerError),

    #[error("dependency error")]
    DepError(ConvertDepError),
}

#[derive(Error, Debug)]
pub enum ConvertDepError {
    #[error("pre-release")]
    ContainsPreRealease(String),

    #[error("version requirement parse error")]
    ReqParseError(ReqParseError),
}

impl TryFrom<&crates_index::Version> for CrateDeps {
    type Error = ConvertCrateError;
    fn try_from(crate_ver: &crates_index::Version) -> Result<Self, Self::Error> {
        Ok(CrateDeps {
            mandatory_deps: Deps::mandatory_from(crate_ver.dependencies())
                .map_err(ConvertCrateError::DepError)?,
            feature_deps: Deps::featured_from(crate_ver.dependencies())
                .map_err(ConvertCrateError::DepError)?,
        })
    }
}

impl Deps {
    fn mandatory_from(deps: &[crates_index::Dependency]) -> Result<Deps, ConvertDepError> {
        let mut normal = Map::default();
        let mut dev = Map::default();
        let mut build = Map::default();
        for dep in deps
            .iter()
            // filter out optional or target-specific dependencies
            .filter(|d| !d.is_optional() && d.target().is_none())
        {
            let name = dep.crate_name().to_string();
            let d = Dep::try_from(dep)?;
            match dep.kind() {
                crates_index::DependencyKind::Normal => normal.insert(name, d),
                crates_index::DependencyKind::Dev => dev.insert(name, d),
                crates_index::DependencyKind::Build => build.insert(name, d),
            };
        }
        Ok(Deps { normal, dev, build })
    }

    fn featured_from(
        deps: &[crates_index::Dependency],
    ) -> Result<Map<Feature, Deps>, ConvertDepError> {
        // todo!()
        Ok(Map::default())
    }
}

impl TryFrom<&crates_index::Dependency> for Dep {
    type Error = ConvertDepError;
    fn try_from(dep: &crates_index::Dependency) -> Result<Self, Self::Error> {
        let req =
            semver::VersionReq::parse(dep.requirement()).map_err(ConvertDepError::ReqParseError)?;
        let range = req_to_range(&req).map_err(ConvertDepError::ContainsPreRealease)?;
        let mut features: Set<Feature> = dep.features().iter().cloned().collect();
        if dep.has_default_features() {
            // Default features correspond to the empty string "" feature.
            features.insert("".to_string());
        }
        Ok(Dep { range, features })
    }
}

fn req_to_range(req: &semver::VersionReq) -> Result<Range<V>, String> {
    // VersionReq is a union of [Range]
    // Range in an intersection of [Predicate]
    let ranges = req.ranges_of_predicates();
    if ranges.is_empty() {
        return Ok(Range::any());
    }

    let mut range = Range::none();
    for r in ranges.iter() {
        range = range.union(&predicates_intersection(r)?);
    }
    Ok(range)
}

fn predicates_intersection(predicates: &[semver::Predicate]) -> Result<Range<V>, String> {
    let mut range = Range::any();
    for p in predicates.iter() {
        if !p.pre.is_empty() {
            return Err("Pre-release".into());
        }
        let version = V::new(p.major as u32, p.minor as u32, p.patch as u32);
        let r = match p.op {
            semver::Op::Ex => Range::exact(version),
            semver::Op::Gt => Range::higher_than(version.bump_patch()),
            semver::Op::GtEq => Range::higher_than(version),
            semver::Op::Lt => Range::strictly_lower_than(version),
            semver::Op::LtEq => Range::strictly_lower_than(version.bump_patch()),
        };
        range = range.intersection(&r);
    }
    Ok(range)
}

impl Index {
    pub fn available_versions(&self, package: &String) -> impl Iterator<Item = &V> {
        self.crates
            .get(package)
            .map(|k| k.keys())
            .into_iter()
            .flatten()
            .rev()
    }
}

impl DependencyProvider<String, V> for Index {
    fn choose_package_version<T: Borrow<String>, U: Borrow<Range<V>>>(
        &self,
        potential_packages: impl Iterator<Item = (T, U)>,
    ) -> Result<(T, Option<V>), Box<dyn std::error::Error>> {
        let mut potential_packages = potential_packages;
        let (package, range) = potential_packages.next().unwrap();
        let (package_name, features) = from_crate_id(package.borrow());
        let version = self
            .available_versions(&package_name.to_string())
            .filter(|v| range.borrow().contains(v))
            .next();
        drop(features);
        Ok((package, version.cloned()))
    }

    fn get_dependencies(
        &self,
        package: &String,
        version: &V,
    ) -> Result<Dependencies<String, V>, Box<dyn std::error::Error>> {
        let (package_name, features) = from_crate_id(package.as_str());
        match self.crates.get(package_name).and_then(|p| p.get(version)) {
            None => Ok(Dependencies::Unknown),
            Some(crate_deps) => {
                let mut all_deps = Map::default();
                // TODO: also add features dependencies.
                for (dep_name, dep) in crate_deps.mandatory_deps.normal.iter() {
                    let dep_id = to_crate_id(dep_name, &dep.features);
                    all_deps.insert(dep_id, dep.range.clone());
                }
                Ok(Dependencies::Known(all_deps))
            }
        }
    }
}

pub fn to_crate_id(pkg: &str, features: &Set<Feature>) -> String {
    let features_str: Vec<&str> = features.iter().map(|f| f.as_str()).collect();
    format!("{}:{}", pkg, features_str.join(","))
}

pub fn from_crate_id(id: &str) -> (&str, impl Iterator<Item = &str>) {
    let mut parts = id.split(':');
    let name = parts.next().unwrap();
    let feats = parts.next().unwrap();
    (name, feats.split(','))
}
