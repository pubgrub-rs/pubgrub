use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::convert::Infallible;

use crate::{
    Dependencies, DependencyConstraints, DependencyProvider, Map, Package,
    PackageResolutionStatistics, VersionSet,
};

/// A basic implementation of [DependencyProvider].
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "VS::V: serde::Serialize, VS: serde::Serialize, P: serde::Serialize",
        deserialize = "VS::V: serde::Deserialize<'de>, VS: serde::Deserialize<'de>, P: serde::Deserialize<'de>"
    ))
)]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct OfflineDependencyProvider<P: Package, VS: VersionSet> {
    dependencies: Map<P, BTreeMap<VS::V, DependencyConstraints<P, VS>>>,
}

impl<P: Package, VS: VersionSet> OfflineDependencyProvider<P, VS> {
    /// Creates an empty OfflineDependencyProvider with no dependencies.
    pub fn new() -> Self {
        Self {
            dependencies: Map::default(),
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
    pub fn add_dependencies<I: IntoIterator<Item = (P, VS)>>(
        &mut self,
        package: P,
        version: impl Into<VS::V>,
        dependencies: I,
    ) {
        let package_deps = dependencies.into_iter().collect();
        let v = version.into();
        *self
            .dependencies
            .entry(package)
            .or_default()
            .entry(v)
            .or_default() = package_deps;
    }

    /// Lists packages that have been saved.
    pub fn packages(&self) -> impl Iterator<Item = &P> {
        self.dependencies.keys()
    }

    /// Lists versions of saved packages in sorted order.
    /// Returns [None] if no information is available regarding that package.
    pub fn versions(&self, package: &P) -> Option<impl Iterator<Item = &VS::V>> {
        self.dependencies.get(package).map(|k| k.keys())
    }

    /// Lists dependencies of a given package and version.
    /// Returns [None] if no information is available regarding that package and version pair.
    fn dependencies(&self, package: &P, version: &VS::V) -> Option<DependencyConstraints<P, VS>> {
        self.dependencies.get(package)?.get(version).cloned()
    }
}

/// Counts how many of a package's versions `range` contains.
///
/// Kept out of `prioritize`, which is inlined into the solver's priority loop, where a second loop
/// costs more registers than the call it saves.
fn count_in_range<'a, VS: VersionSet>(
    range: &VS,
    versions: impl Iterator<Item = &'a VS::V>,
) -> usize
where
    VS::V: 'a,
{
    range.contains_many(versions).filter(|&c| c).count()
}

/// An implementation of [DependencyProvider] that
/// contains all dependency information available in memory.
/// Currently packages are picked with the fewest versions contained in the constraints first.
/// But, that may change in new versions if better heuristics are found.
/// Versions are picked with the newest versions first.
impl<P: Package, VS: VersionSet> DependencyProvider for OfflineDependencyProvider<P, VS> {
    type P = P;
    type V = VS::V;
    type VS = VS;
    type M = String;

    type Err = Infallible;

    #[inline]
    fn choose_version(&self, package: &P, range: &VS) -> Result<Option<VS::V>, Infallible> {
        Ok(self
            .dependencies
            .get(package)
            .and_then(|versions| versions.keys().rev().find(|v| range.contains(v)).cloned()))
    }

    type Priority = (u32, Reverse<usize>);

    #[inline]
    fn prioritize(
        &self,
        package: &Self::P,
        range: &Self::VS,
        package_statistics: &PackageResolutionStatistics,
    ) -> Self::Priority {
        let version_count = self
            .dependencies
            .get(package)
            .map(|versions| match versions.len() {
                // One version is not worth batching.
                1 => {
                    let version = versions.keys().next().expect("length checked");
                    usize::from(range.contains(version))
                }
                _ => count_in_range(range, versions.keys()),
            })
            .unwrap_or(0);
        if version_count == 0 {
            return (u32::MAX, Reverse(0));
        }
        (package_statistics.conflict_count(), Reverse(version_count))
    }

    #[inline]
    fn get_dependencies(
        &self,
        package: &P,
        version: &VS::V,
    ) -> Result<Dependencies<P, VS, Self::M>, Infallible> {
        Ok(match self.dependencies(package, version) {
            None => {
                Dependencies::Unavailable("its dependencies could not be determined".to_string())
            }
            Some(dependencies) => Dependencies::Available(dependencies),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Ranges;

    /// A wrong count only reorders decisions, so no solver test catches it.
    #[test]
    fn prioritize_counts_versions_in_range() {
        let mut provider = OfflineDependencyProvider::<u8, Ranges<u32>>::new();
        for version in [1u32, 3, 5, 7] {
            provider.add_dependencies(0, version, []);
        }
        let statistics = PackageResolutionStatistics::default();

        let cases = [
            (Ranges::empty(), 0),
            (Ranges::full(), 4),
            (Ranges::singleton(3u32), 1),
            (Ranges::from_range_bounds(2u32..=6), 2),
            (Ranges::singleton(1u32).union(&Ranges::singleton(7u32)), 2),
            (Ranges::higher_than(9u32), 0),
        ];

        for (range, count) in cases {
            let expected = if count == 0 {
                (u32::MAX, Reverse(0))
            } else {
                (0, Reverse(count))
            };
            assert_eq!(
                provider.prioritize(&0, &range, &statistics),
                expected,
                "{range}"
            );
        }

        let unknown_package = provider.prioritize(&1, &Ranges::full(), &statistics);
        assert_eq!(unknown_package, (u32::MAX, Reverse(0)));
    }

    /// Covers the single-version path.
    #[test]
    fn prioritize_counts_a_single_version() {
        let mut provider = OfflineDependencyProvider::<u8, Ranges<u32>>::new();
        provider.add_dependencies(0, 3u32, []);
        let statistics = PackageResolutionStatistics::default();

        for (range, expected) in [
            (Ranges::full(), (0, Reverse(1))),
            (Ranges::singleton(3u32), (0, Reverse(1))),
            (Ranges::singleton(4u32), (u32::MAX, Reverse(0))),
            (Ranges::empty(), (u32::MAX, Reverse(0))),
        ] {
            assert_eq!(
                provider.prioritize(&0, &range, &statistics),
                expected,
                "{range}"
            );
        }
    }
}
