// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Cache packages information that has already been loaded.

use std::collections::BTreeSet as Set;
use std::collections::HashMap as Map;
use std::hash::Hash;

use crate::package::Package;
use crate::range::Range;
use crate::version::Version;

/// Trait for a packages and dependencies cache
/// to be used by the solver algorithm.
pub trait Cache<P: Package, V: Version> {
    /// Register in cache a package + version pair as existing.
    /// TODO: use Into<P> and Into<V>
    fn add_package_version(&mut self, package: P, version: impl Into<V>);

    /// Register in cache the dependencies of a package and version pair.
    /// Dependencies must be added with a single call to `add_dependencies`.
    /// All subsequent calls to `add_dependencies` for a given
    /// package version pair will replace the dependencies by the new ones.
    ///
    /// The API does not allow to add dependencies one at a time
    /// because users of the Cache trait make the assumption that
    /// a call to `cache.dependencies(p, v)` provides all dependencies
    /// of a given package (p) and version (v) pair.
    ///
    /// Since dependencies are supposed to be immutable,
    /// this can enable an optimization in the solver implementation.
    /// If you use a cache when implementing `Solver::get_dependencies`,
    /// you do not need to request package dependencies
    /// if the call to `cache.dependencies(p, v)` returns `Some(_)`.
    fn add_dependencies<I: IntoIterator<Item = (P, Range<V>)>>(
        &mut self,
        package: P,
        version: impl Into<V>,
        dependencies: I,
    );

    // Read stuff.

    /// Number of unique pairs of package and version in cache.
    fn nb_package_versions(&self) -> usize;

    /// Number of dependency entries (1 per package and version pair) in cache.
    fn nb_dependencies(&self) -> usize;

    /// List versions of a package already in cache.
    /// Return `None` if no information is available regarding that package.
    fn versions(&self, package: &P) -> Option<Set<V>>;

    /// List dependencies of a given package and version.
    /// Return `None` if no information is available regarding that package and version pair.
    fn dependencies(&self, package: &P, version: &V) -> Option<Map<P, Range<V>>>;
}

/// Basic default implementation of a Cache.
/// Remark: versions also need to implement Hash.
pub struct SimpleCache<P: Package, V: Version + Hash> {
    package_versions: Map<P, Set<V>>,
    dependencies: Map<(P, V), Map<P, Range<V>>>,
}

impl<P: Package, V: Version + Hash> SimpleCache<P, V> {
    /// Create an empty cache.
    pub fn new() -> Self {
        Self {
            package_versions: Map::new(),
            dependencies: Map::new(),
        }
    }
}

impl<P: Package, V: Version + Hash> Cache<P, V> for SimpleCache<P, V> {
    fn add_package_version(&mut self, package: P, version: impl Into<V>) {
        let v_set = self.package_versions.entry(package).or_insert(Set::new());
        v_set.insert(version.into());
    }

    fn add_dependencies<I: IntoIterator<Item = (P, Range<V>)>>(
        &mut self,
        package: P,
        version: impl Into<V>,
        dependencies: I,
    ) {
        let package_deps = dependencies.into_iter().collect();
        let v = version.into();
        self.add_package_version(package.clone(), v.clone());
        self.dependencies.insert((package, v), package_deps);
    }

    // Read stuff.

    fn nb_package_versions(&self) -> usize {
        self.package_versions.values().map(|set| set.len()).sum()
    }

    fn nb_dependencies(&self) -> usize {
        self.dependencies.len()
    }

    fn versions(&self, package: &P) -> Option<Set<V>> {
        self.package_versions.get(package).cloned()
    }

    fn dependencies(&self, package: &P, version: &V) -> Option<Map<P, Range<V>>> {
        self.dependencies
            .get(&(package.clone(), version.clone()))
            .cloned()
    }
}
