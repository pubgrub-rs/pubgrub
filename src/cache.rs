// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Cache packages information that has already been loaded.

use std::collections::BTreeSet as Set;
use std::collections::HashMap as Map;
use std::hash::Hash;

use crate::range::Range;
use crate::version::Version;

/// Trait for a packages and dependencies cache
/// to be used by the solver algorithm.
pub trait Cache<P, V>
where
    P: Clone + Eq + Hash,
    V: Clone + Ord + Version,
{
    /// Create an empty new cache.
    fn new() -> Self;

    /// Register in cache a package + version pair as existing.
    fn add_package_version(&mut self, package: P, version: V);

    /// Register in cache the dependencies of a package and version pair.
    /// Dependencies must be added with a single call to `add_dependencies`.
    /// All subsequent calls to `add_dependencies` for a given
    /// package version pair will replace the dependencies by the new ones.
    ///
    /// The API does not allow to add dependencies one at a time
    /// because users of the Cache trait make the assumption that
    /// a call to `cache.dependencies(p, v)` provides all dependencies
    /// of a given package (p) and version (v) pair.
    /// Since dependencies are supposed to be immutable,
    /// this enables an optimization in the solver code,
    /// which does not need to request package dependencies
    /// if the call to `cache.dependencies(p, v)` returns `Some(_)`.
    fn add_dependencies<I: Iterator<Item = (P, Range<V>)>>(
        &mut self,
        package: P,
        version: V,
        dependencies: I,
    );

    // Read stuff.

    /// Number of unique pairs of package and version in cache.
    fn nb_package_versions(&self) -> usize;

    /// Number of dependency entries (1 per package and version pair) in cache.
    fn nb_dependencies(&self) -> usize;

    /// List versions of a package already in cache.
    /// Return `None` if no information is available regarding that package.
    fn versions(&self, package: P) -> Option<&Set<V>>;

    /// List dependencies of a given package and version.
    /// Return `None` if no information is available regarding that package and version pair.
    fn dependencies(&self, package: P, version: V) -> Option<&Map<P, Range<V>>>;
}
