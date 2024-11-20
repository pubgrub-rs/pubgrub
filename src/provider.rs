use std::borrow::Borrow;
use std::cmp::Reverse;
use std::convert::Infallible;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::ops::Bound;

use crate::{
    helpers::PackageVersionWrapper, resolve, Dependencies, DependencyProvider, FxIndexSet, Map,
    PackageArena, PackageId, PubGrubError, Ranges, VersionIndex, VersionSet,
};

/// Version range.
pub trait VersionRanges: Debug + Display {
    /// Associated version type.
    type V: Debug + Display + Clone + Ord;

    /// Returns true if this version range contains the specified value.
    fn contains(&self, version: &Self::V) -> bool;

    /// Returns true if this version range contains the specified values.
    fn contains_many<'s, I, BV>(&'s self, versions: I) -> impl Iterator<Item = bool> + 's
    where
        I: IntoIterator<Item = BV> + 's,
        BV: Borrow<Self::V> + 's;

    /// Returns the bounding range of this version range.
    #[allow(clippy::type_complexity)]
    fn bounding_range(&self) -> Option<(Bound<&Self::V>, Bound<&Self::V>)>;

    /// Returns a version range for the provided ordered versions.
    fn from_ordered_versions(versions: impl IntoIterator<Item = (Self::V, bool)> + Clone) -> Self;
}

impl<V: Debug + Display + Clone + Ord> VersionRanges for Ranges<V> {
    type V = V;

    fn contains(&self, version: &Self::V) -> bool {
        self.contains(version)
    }

    fn contains_many<'s, I, BV>(&'s self, versions: I) -> impl Iterator<Item = bool> + 's
    where
        I: IntoIterator<Item = BV> + 's,
        BV: Borrow<Self::V> + 's,
    {
        self.contains_many(versions.into_iter())
    }

    fn bounding_range(&self) -> Option<(Bound<&Self::V>, Bound<&Self::V>)> {
        self.bounding_range()
    }

    fn from_ordered_versions(versions: impl IntoIterator<Item = (Self::V, bool)> + Clone) -> Self {
        let all_iter = versions.clone().into_iter();
        let versions = versions.into_iter();
        let mut range = Ranges::empty();
        for (v, ok) in versions {
            if ok {
                range = range.union(&Ranges::singleton(v));
            }
        }
        range.simplify(all_iter.map(|(v, _)| v))
    }
}

/// A basic implementation of [DependencyProvider].
#[derive(Debug, Clone, Default)]
pub struct OfflineDependencyProvider<P: Debug + Display + Clone + Eq + Hash, R: VersionRanges> {
    #[allow(clippy::type_complexity)]
    dependencies: Map<P, Vec<(R::V, Map<P, R>)>>,
    conflicts: Map<P, u64>,
}

#[cfg(feature = "serde")]
impl<P: Debug + Display + Clone + Eq + Hash, R: VersionRanges> serde::Serialize
    for OfflineDependencyProvider<P, R>
where
    P: serde::Serialize,
    R::V: serde::Serialize,
    R: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use std::collections::BTreeMap;

        self.dependencies
            .iter()
            .map(|(p, versions)| (p, versions.iter().map(|(v, dmap)| (v, dmap)).collect()))
            .collect::<Map<&P, BTreeMap<&R::V, &Map<P, R>>>>()
            .serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, P: Debug + Display + Clone + Eq + Hash, R: VersionRanges> serde::Deserialize<'de>
    for OfflineDependencyProvider<P, R>
where
    P: serde::Deserialize<'de>,
    R::V: serde::Deserialize<'de>,
    R: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use std::collections::BTreeMap;

        Ok(Self {
            dependencies: <Map<P, BTreeMap<R::V, Map<P, R>>>>::deserialize(deserializer)?
                .into_iter()
                .map(|(p, versions)| (p, versions.into_iter().collect()))
                .collect(),
            conflicts: Map::default(),
        })
    }
}

impl<P: Debug + Display + Clone + Eq + Hash, R: VersionRanges> OfflineDependencyProvider<P, R> {
    /// Creates an empty OfflineDependencyProvider with no dependencies.
    pub fn new() -> Self {
        Self {
            dependencies: Map::default(),
            conflicts: Map::default(),
        }
    }

    /// Registers the dependencies of a package and version pair.
    /// Dependencies must be added with a single call to
    /// [add_dependencies](OfflineDependencyProvider::add_dependencies).
    /// All subsequent calls to
    /// [add_dependencies](OfflineDependencyProvider::add_dependencies) for a given
    /// package version pair will replace the dependencies by the new ones.
    ///
    /// The API does not allow to add dependencies one at a time to uphold an assumption that
    /// [OfflineDependencyProvider.get_dependencies(p, v)](OfflineDependencyProvider::get_dependencies)
    /// provides all dependencies of a given package (p) and version (v) pair.
    pub fn add_dependencies<I: IntoIterator<Item = (P, R)>>(
        &mut self,
        package: P,
        version: impl Into<R::V>,
        dependencies: I,
    ) {
        let version = version.into();
        let pkg_deps = self.dependencies.entry(package).or_default();

        match pkg_deps.binary_search_by(|(v, _)| v.cmp(&version)) {
            Ok(idx) => pkg_deps[idx].1 = dependencies.into_iter().collect(),
            Err(idx) => pkg_deps.insert(idx, (version, dependencies.into_iter().collect())),
        }
    }

    /// Lists packages that have been saved.
    pub fn packages(&self) -> impl Iterator<Item = &P> {
        self.dependencies.keys()
    }

    /// Lists versions of saved packages in sorted order.
    /// Returns [None] if no information is available regarding that package.
    pub fn versions(&self, p: &P) -> Option<impl Iterator<Item = &R::V> + Clone> {
        Some(self.dependencies.get(p)?.iter().map(|(v, _)| v))
    }

    /// Lists dependencies of a given package and version.
    pub fn dependencies(&self, p: &P, v: &R::V) -> Option<&Map<P, R>> {
        let pkg_deps = self.dependencies.get(p)?;
        pkg_deps
            .binary_search_by(|(version, _)| version.cmp(v))
            .map(|idx| &pkg_deps[idx].1)
            .ok()
    }

    /// Returns the resolve parameters from a root package and version.
    pub fn resolve_parameters(
        &self,
        p: P,
        v: impl Into<R::V>,
    ) -> Option<(PackageVersionWrapper<P>, VersionIndex)> {
        let versions = self.dependencies.get(&p)?;

        let v = v.into();
        let true_version_index = self
            .dependencies
            .get(&p)?
            .iter()
            .enumerate()
            .find(|&(_, (pv, _))| pv == &v)
            .map(|(i, _)| i as u64)?;

        let (root_pkg, root_version_index) = PackageVersionWrapper::new_pkg(
            p,
            true_version_index,
            (versions.len() as u64).try_into().unwrap(),
        );

        Some((root_pkg, root_version_index))
    }

    /// Finds a set of packages satisfying dependency bounds for a given package + version pair.
    pub fn resolve(
        &mut self,
        p: P,
        v: impl Into<R::V>,
    ) -> Result<Map<P, R::V>, PubGrubError<Self>> {
        let (root_pkg, root_version_index) = self
            .resolve_parameters(p, v)
            .ok_or_else(|| PubGrubError::NoRoot)?;

        let res = resolve(self, root_pkg, root_version_index)?;

        Ok(res
            .into_iter()
            .filter_map(|(wrapper, version_index)| {
                let (pkg, true_version_index) = wrapper.into_inner(version_index)?;

                let version = self
                    .dependencies
                    .get(&pkg)
                    .into_iter()
                    .flat_map(|versions| versions.iter().map(|(v, _)| v))
                    .nth(true_version_index as usize)?;

                Some((pkg, version.clone()))
            })
            .collect())
    }
}

/// An implementation of [DependencyProvider] that
/// contains all dependency information available in memory.
/// Currently packages are picked with the fewest versions contained in the constraints first.
/// But, that may change in new versions if better heuristics are found.
/// Versions are picked with the newest versions first.
impl<P: Debug + Display + Clone + Eq + Hash, R: VersionRanges> DependencyProvider
    for OfflineDependencyProvider<P, R>
{
    type P = PackageVersionWrapper<P>;
    type M = &'static str;

    type Err = Infallible;

    #[inline]
    fn choose_version(
        &mut self,
        _: PackageId,
        set: VersionSet,
        _: &PackageArena<Self::P>,
    ) -> Result<Option<VersionIndex>, Infallible> {
        Ok(set.last())
    }

    type Priority = Reverse<u64>;

    #[inline]
    fn prioritize(
        &mut self,
        package_id: PackageId,
        set: VersionSet,
        package_store: &PackageArena<Self::P>,
    ) -> Self::Priority {
        let version_count = set.count();
        if version_count == 0 {
            return Reverse(0);
        }
        let pkg = match package_store.pkg(package_id).unwrap() {
            PackageVersionWrapper::Pkg(p) => p.pkg(),
            PackageVersionWrapper::VirtualPkg(p) => p.pkg(),
            PackageVersionWrapper::VirtualDep(p) => p.pkg(),
        };
        let conflict_count = self.conflicts.get(pkg).copied().unwrap_or_default();

        Reverse(((u32::MAX as u64).saturating_sub(conflict_count) << 6) + version_count as u64)
    }

    fn get_dependencies(
        &mut self,
        package_id: PackageId,
        version_index: VersionIndex,
        package_store: &mut PackageArena<Self::P>,
    ) -> Result<Dependencies<Self::M>, Infallible> {
        let mut dep_map = Map::default();

        let wrapper = package_store.pkg(package_id).unwrap();
        let inner = wrapper.inner(version_index).map(|(p, v)| (p.clone(), v));

        if let Some((d, vs)) = wrapper.dependency(version_index) {
            dep_map.insert(package_store.insert(d), vs);
        }

        let Some((pkg, true_version_index)) = inner else {
            return Ok(Dependencies::Available(dep_map));
        };

        let msg = "dependencies could not be determined";

        let Some(pkg_deps) = self.dependencies.get(&pkg) else {
            return Ok(Dependencies::Unavailable(msg));
        };
        let Some(deps) = pkg_deps
            .iter()
            .map(|(_, dmap)| dmap)
            .nth(true_version_index as usize)
        else {
            return Ok(Dependencies::Unavailable(msg));
        };

        for (dep, r) in deps {
            let empty_vec = vec![];
            let versions = self.dependencies.get(dep).unwrap_or(&empty_vec);
            let version_count = versions.len() as u64;
            let dep = dep.clone();

            let (d, vs) = match r.bounding_range() {
                None => PackageVersionWrapper::new_empty_dep(dep),
                Some((Bound::Unbounded, Bound::Unbounded)) => {
                    PackageVersionWrapper::new_dep(dep, 0..version_count, version_count)
                }
                Some((Bound::Included(start), Bound::Included(end))) if start == end => {
                    if let Ok(idx) = versions.binary_search_by(|(v, _)| v.cmp(start)) {
                        PackageVersionWrapper::new_singleton_dep(dep, idx as u64, version_count)
                    } else {
                        PackageVersionWrapper::new_empty_dep(dep)
                    }
                }
                Some((start, end)) => {
                    let start_idx = match start {
                        Bound::Unbounded => 0,
                        Bound::Included(start) | Bound::Excluded(start) => {
                            versions.partition_point(|(v, _)| v < start)
                        }
                    };
                    let end_idx = match end {
                        Bound::Unbounded => version_count as usize,
                        Bound::Included(end) | Bound::Excluded(end) => {
                            versions.partition_point(|(v, _)| v <= end)
                        }
                    };

                    let true_version_indices = r
                        .contains_many(versions[start_idx..end_idx].iter().map(|(v, _)| v))
                        .enumerate()
                        .filter(|&(_, ok)| ok)
                        .map(|(i, _)| (start_idx + i) as u64);

                    PackageVersionWrapper::new_dep(dep, true_version_indices, version_count)
                }
            };

            dep_map.insert(package_store.insert(d), vs);
        }

        Ok(Dependencies::Available(dep_map))
    }

    fn package_version_display<'a>(
        &'a self,
        package: &'a Self::P,
        version_index: VersionIndex,
    ) -> impl Display + 'a {
        match package.inner(version_index) {
            None => format!("{package} @ {}", version_index.get()),
            Some((pkg, true_version_index)) => {
                if let Some(version) = self
                    .dependencies
                    .get(pkg)
                    .into_iter()
                    .flat_map(|versions| versions.iter().map(|(v, _)| v))
                    .nth(true_version_index as usize)
                {
                    format!("{pkg} @ {version}")
                } else {
                    format!("{pkg} @ <unknown>")
                }
            }
        }
    }

    fn package_version_set_display<'a>(
        &'a self,
        package: &'a Self::P,
        version_set: VersionSet,
    ) -> impl Display + 'a {
        match package.inner_pkg() {
            Some(p) => {
                let true_version_indices = version_set
                    .iter()
                    .map(|v| package.inner(v).unwrap().1 as usize)
                    .collect::<FxIndexSet<_>>();

                let versions = self
                    .dependencies
                    .get(p)
                    .map(|versions| {
                        R::from_ordered_versions(versions.iter().enumerate().map(|(i, (v, _))| {
                            let ok = true_version_indices.contains(&i);
                            (v.clone(), ok)
                        }))
                    })
                    .unwrap_or_else(|| R::from_ordered_versions([]));

                format!("{p} @ {versions}")
            }
            _ => {
                let version_indices = version_set
                    .iter()
                    .map(|version_index| version_index.get())
                    .collect::<Vec<_>>();

                format!("{package} @ {version_indices:?}")
            }
        }
    }

    #[inline]
    fn register_conflict(
        &mut self,
        package_ids: impl Iterator<Item = PackageId>,
        package_store: &PackageArena<Self::P>,
    ) {
        for package_id in package_ids {
            let pkg = match package_store.pkg(package_id).unwrap() {
                PackageVersionWrapper::Pkg(p) => p.pkg(),
                PackageVersionWrapper::VirtualPkg(p) => p.pkg(),
                PackageVersionWrapper::VirtualDep(p) => p.pkg(),
            };
            *self.conflicts.entry(pkg.clone()).or_default() += 1;
        }
    }
}
