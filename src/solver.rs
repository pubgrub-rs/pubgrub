// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! PubGrub version solving algorithm.
//!
//! It consists in efficiently finding a set of packages and versions
//! that satisfy all the constraints of a given project dependencies.
//! In addition, when that is not possible,
//! PubGrub tries to provide a very human-readable and clear
//! explanation as to why that failed.
//! Below is an example of explanation present in
//! the introductory blog post about PubGrub
//!
//! ```txt
//! Because dropdown >=2.0.0 depends on icons >=2.0.0 and
//!   root depends on icons <2.0.0, dropdown >=2.0.0 is forbidden.
//!
//! And because menu >=1.1.0 depends on dropdown >=2.0.0,
//!   menu >=1.1.0 is forbidden.
//!
//! And because menu <1.1.0 depends on dropdown >=1.0.0 <2.0.0
//!   which depends on intl <4.0.0, every version of menu
//!   requires intl <4.0.0.
//!
//! So, because root depends on both menu >=1.0.0 and intl >=5.0.0,
//!   version solving failed.
//! ```
//!
//! The algorithm is generic and works for any type of dependency system
//! as long as packages (P) and versions (V) implement
//! the `Package` and `Version` traits.
//! `Package` is strictly equivalent and automatically generated
//! for any type that implement `Clone + Eq + Hash + Debug + Display`.
//! `Version` simply states that versions are ordered,
//! that there should be
//! a minimal `lowest` version (like 0.0.0 in semantic versions),
//! and that for any version, it is possible to compute
//! what the next version closest to this one is (`bump`).
//! For semantic versions, `bump` corresponds to an increment of the patch number.
//!
//!
//! ## API
//!
//! ```
//! # use pubgrub::solver::{resolve, OfflineDependencyProvider};
//! # use pubgrub::version::NumberVersion;
//! # use pubgrub::error::PubGrubError;
//! #
//! # fn try_main() -> Result<(), PubGrubError<&'static str, NumberVersion>> {
//! #     let dependency_provider = OfflineDependencyProvider::<&str, NumberVersion>::new();
//! #     let package = "root";
//! #     let version = 1;
//! let solution = resolve(&dependency_provider, package, version)?;
//! #     Ok(())
//! # }
//! # fn main() {
//! #     assert!(matches!(try_main(), Err(PubGrubError::NoSolution(_))));
//! # }
//! ```
//!
//! Where `dependency_provider` supplies the list of available packages and versions,
//! as well as the dependencies of every available package
//! by implementing the [DependencyProvider] trait.
//! The call to [resolve] for a given package at a given version
//! will compute the set of packages and versions needed
//! to satisfy the dependencies of that package and version pair.
//! If there is no solution, the reason will be provided as clear as possible.

use std::collections::BTreeSet as Set;
use std::collections::HashMap as Map;
use std::error::Error;
use std::hash::Hash;

use crate::error::PubGrubError;
use crate::internal::core::State;
use crate::internal::incompatibility::Incompatibility;
use crate::internal::partial_solution::PartialSolution;
use crate::package::Package;
use crate::range::Range;
use crate::version::Version;

/// Main function of the library.
/// Finds a set of packages satisfying dependency bounds for a given package + version pair.
pub fn resolve<P: Package, V: Version>(
    dependency_provider: &impl DependencyProvider<P, V>,
    package: P,
    version: impl Into<V>,
) -> Result<Map<P, V>, PubGrubError<P, V>> {
    let mut state = State::init(package.clone(), version.into());
    let mut next = package;
    loop {
        state.unit_propagation(next)?;

        // Pick the next package.
        let (p, term) = match state.partial_solution.pick_package() {
            None => {
                return state
                    .partial_solution
                    .extract_solution()
                    .ok_or(PubGrubError::Failure(
                        "How did we end up with no package to choose but no solution?".into(),
                    ))
            }
            Some(x) => x,
        };
        next = p.clone();
        let available_versions =
            dependency_provider
                .list_available_versions(&p)
                .map_err(|err| PubGrubError::ErrorRetrievingVersions {
                    package: p.clone(),
                    source: err,
                })?;

        // Pick the next compatible version.
        let v = match PartialSolution::<P, V>::pick_version(&available_versions[..], &term) {
            None => {
                state.add_incompatibility(|id| {
                    Incompatibility::no_version(id, p.clone(), term.clone())
                });
                continue;
            }
            Some(x) => x,
        };

        // Retrieve that package dependencies.
        let dependencies = match dependency_provider
            .get_dependencies(&p, &v)
            .map_err(|err| PubGrubError::ErrorRetrievingDependencies {
                package: p.clone(),
                version: v.clone(),
                source: err,
            })? {
            None => {
                state.add_incompatibility(|id| {
                    Incompatibility::unavailable_dependencies(id, p.clone(), v.clone())
                });
                continue;
            }
            Some(x) => x,
        };

        // Add that package and version if the dependencies are not problematic.
        let start_id = state.incompatibility_store.len();
        let dep_incompats =
            Incompatibility::from_dependencies(start_id, p.clone(), v.clone(), &dependencies);
        for incompat in dep_incompats.iter() {
            state.add_incompatibility(|_| incompat.clone());
        }
        if dep_incompats
            .iter()
            .any(|incompat| state.is_terminal(incompat))
        {
            // For a dependency incompatibility to be terminal,
            // it can only mean that root depend on not root?
            Err(PubGrubError::Failure(
                "Root package depends on itself at a different version?".into(),
            ))?;
        }
        state.partial_solution.add_version(p, v, &dep_incompats);
    }
}

/// Trait that allows the algorithm to retrieve available packages and their dependencies.
/// An implementor needs to be supplied to the [resolve] function.
pub trait DependencyProvider<P: Package, V: Version> {
    /// Lists available versions for a given package.
    /// The strategy of which version should be preferably picked in the list of available versions
    /// is implied by the order of the list: first version in the list will be tried first.
    fn list_available_versions(&self, package: &P) -> Result<Vec<V>, Box<dyn Error>>;

    /// Retrieves the package dependencies.
    /// Return None if its dependencies are unknown.
    fn get_dependencies(
        &self,
        package: &P,
        version: &V,
    ) -> Result<Option<Map<P, Range<V>>>, Box<dyn Error>>;
}

/// A basic implementation of [DependencyProvider].
pub struct OfflineDependencyProvider<P: Package, V: Version + Hash> {
    package_versions: Map<P, Set<V>>,
    dependencies: Map<(P, V), Map<P, Range<V>>>,
}

impl<P: Package, V: Version + Hash> OfflineDependencyProvider<P, V> {
    /// Creates an empty OfflineDependencyProvider with no dependencies.
    pub fn new() -> Self {
        Self {
            package_versions: Map::new(),
            dependencies: Map::new(),
        }
    }

    /// Registers the dependencies of a package and version pair.
    /// Dependencies must be added with a single call to `add_dependencies`.
    /// All subsequent calls to `add_dependencies` for a given
    /// package version pair will replace the dependencies by the new ones.
    ///
    /// The API does not allow to add dependencies one at a time
    /// to uphold an assumption that `OfflineDependencyProvider.get_dependencies(p, v)`
    /// provides all dependencies of a given package (p) and version (v) pair.
    pub fn add_dependencies<I: IntoIterator<Item = (P, Range<V>)>>(
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

    // TODO: use Into<P> and Into<V>
    fn add_package_version(&mut self, package: P, version: impl Into<V>) {
        let v_set = self.package_versions.entry(package).or_insert(Set::new());
        v_set.insert(version.into());
    }

    /// Lists versions of saved packages.
    /// Returns `None` if no information is available regarding that package.
    fn versions(&self, package: &P) -> Option<Set<V>> {
        self.package_versions.get(package).cloned()
    }

    /// Lists dependencies of a given package and version.
    /// Returns `None` if no information is available regarding that package and version pair.
    fn dependencies(&self, package: &P, version: &V) -> Option<Map<P, Range<V>>> {
        self.dependencies
            .get(&(package.clone(), version.clone()))
            .cloned()
    }
}

/// An implementation of [DependencyProvider] that
/// contains all dependency information available in memory.
/// Versions are listed with the newest versions first.
impl<P: Package, V: Version + Hash> DependencyProvider<P, V> for OfflineDependencyProvider<P, V> {
    fn list_available_versions(&self, package: &P) -> Result<Vec<V>, Box<dyn Error>> {
        Ok(self
            .versions(package)
            .map(|v| v.into_iter().rev().collect())
            .unwrap_or(Vec::new()))
    }

    fn get_dependencies(
        &self,
        package: &P,
        version: &V,
    ) -> Result<Option<Map<P, Range<V>>>, Box<dyn Error>> {
        Ok(self.dependencies(package, version))
    }
}
