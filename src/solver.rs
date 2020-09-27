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
//! ```ignore
//! solution = solver.run(package, version)?;
//! ```
//!
//! Where `solver` provides the list of available packages and versions,
//! as well as the dependencies of every available package
//! by implementing the `Solver` trait.
//! The call to `run` for a given package at a given version
//! will compute the set of packages and versions needed
//! to satisfy the dependencies of that package and version pair.
//! If there is no solution, the reason will be provided as clear as possible.

use std::collections::HashMap as Map;
use std::error::Error;

use crate::cache::Cache;
use crate::error::PubGrubError;
use crate::internal::core::State;
use crate::internal::incompatibility::Incompatibility;
use crate::internal::partial_solution::PartialSolution;
use crate::package::Package;
use crate::range::Range;
use crate::version::Version;

/// Solver trait.
/// Given functions to retrieve the list of available versions of a package,
/// and their dependencies, this provides a `run` method,
/// able to compute a complete set of direct and indirect dependencies
/// satisfying the chosen package constraints.
///
/// Remark: for ease of use, the `Solver` trait is automatically implemented
/// for any type that implements `Cache` such as `SimpleCache`.
pub trait Solver<P: Package, V: Version> {
    /// List available versions for a given package.
    /// The strategy of which version should be preferably picked in the list of available versions
    /// is implied by the order of the list: first version in the list will be tried first.
    fn list_available_versions(&mut self, package: &P) -> Result<Vec<V>, Box<dyn Error>>;

    /// Retrieve the package dependencies.
    /// Return None if its dependencies are unknown.
    fn get_dependencies(
        &mut self,
        package: &P,
        version: &V,
    ) -> Result<Option<Map<P, Range<V>>>, Box<dyn Error>>;

    /// Solve dependencies of a given package.
    fn run(&mut self, package: P, version: impl Into<V>) -> Result<Map<P, V>, PubGrubError<P, V>> {
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
            let available_versions = self.list_available_versions(&p).map_err(|err| {
                PubGrubError::ErrorRetrievingVersions {
                    package: p.clone(),
                    source: err,
                }
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
            let dependencies = match self.get_dependencies(&p, &v).map_err(|err| {
                PubGrubError::ErrorRetrievingDependencies {
                    package: p.clone(),
                    version: v.clone(),
                    source: err,
                }
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
}

/// Automatically implement Config if your type implements Cache.
/// Versions are listed with newest versions first.
impl<P: Package, V: Version, C: Cache<P, V>> Solver<P, V> for C {
    fn list_available_versions(&mut self, package: &P) -> Result<Vec<V>, Box<dyn Error>> {
        Ok(self
            .versions(package)
            .map(|v| v.into_iter().rev().collect())
            .unwrap_or(Vec::new()))
    }

    fn get_dependencies(
        &mut self,
        package: &P,
        version: &V,
    ) -> Result<Option<Map<P, Range<V>>>, Box<dyn Error>> {
        Ok(self.dependencies(package, version))
    }
}
