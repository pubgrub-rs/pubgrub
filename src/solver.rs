// SPDX-License-Identifier: MPL-2.0

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
//! the [Package](crate::package::Package) and [Version](crate::version::Version) traits.
//! [Package](crate::package::Package) is strictly equivalent and automatically generated
//! for any type that implement [Clone] + [Eq] + [Hash] + [Debug] + [Display](std::fmt::Display).
//! [Version](crate::version::Version) simply states that versions are ordered,
//! that there should be
//! a minimal [lowest](crate::version::Version::lowest) version (like 0.0.0 in semantic versions),
//! and that for any version, it is possible to compute
//! what the next version closest to this one is ([bump](crate::version::Version::bump)).
//! For semantic versions, [bump](crate::version::Version::bump) corresponds to
//! an increment of the patch number.
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
use std::error::Error;
use std::hash::Hash;

use crate::error::PubGrubError;
use crate::internal::core::State;
use crate::internal::incompatibility::Incompatibility;
use crate::internal::partial_solution::PartialSolution;
use crate::package::Package;
use crate::range::Range;
use crate::type_aliases::{Map, SelectedDependencies};
use crate::version::Version;

/// Main function of the library.
/// Finds a set of packages satisfying dependency bounds for a given package + version pair.
pub fn resolve<P: Package, V: Version>(
    dependency_provider: &impl DependencyProvider<P, V>,
    package: P,
    version: impl Into<V>,
) -> Result<SelectedDependencies<P, V>, PubGrubError<P, V>> {
    let mut state = State::init(package.clone(), version.into());
    let mut added_dependencies: Map<P, Set<V>> = Map::default();
    let mut next = package;
    loop {
        dependency_provider
            .should_cancel()
            .map_err(|err| PubGrubError::ErrorShouldCancel(err))?;

        state.unit_propagation(next)?;

        // Pick the next package.
        let (p, term) = match state.partial_solution.pick_package(dependency_provider)? {
            None => {
                return state.partial_solution.extract_solution().ok_or_else(|| {
                    PubGrubError::Failure(
                        "How did we end up with no package to choose but no solution?".into(),
                    )
                })
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

        if added_dependencies
            .entry(p.clone())
            .or_default()
            .insert(v.clone())
        {
            // Retrieve that package dependencies.
            let dependencies =
                match dependency_provider
                    .get_dependencies(&p, &v)
                    .map_err(|err| PubGrubError::ErrorRetrievingDependencies {
                        package: p.clone(),
                        version: v.clone(),
                        source: err,
                    })? {
                    Dependencies::Unknown => {
                        state.add_incompatibility(|id| {
                            Incompatibility::unavailable_dependencies(id, p.clone(), v.clone())
                        });
                        continue;
                    }
                    Dependencies::Known(x) => {
                        if x.contains_key(&p) {
                            return Err(PubGrubError::SelfDependency {
                                package: p.clone(),
                                version: v.clone(),
                            });
                        }
                        if let Some((dependent, _)) = x.iter().find(|(_, r)| r == &&Range::none()) {
                            return Err(PubGrubError::DependencyOnTheEmptySet {
                                package: p.clone(),
                                version: v.clone(),
                                dependent: dependent.clone(),
                            });
                        }
                        x
                    }
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
                return Err(PubGrubError::Failure(
                    "Root package depends on itself at a different version?".into(),
                ));
            }
        }
        state.partial_solution.add_decision(p, v);
    }
}

/// An enum used by [DependencyProvider] that holds information about package dependencies.
/// For each [Package] there is a [Range] of concrete versions it allows as a dependency.
#[derive(Clone)]
pub enum Dependencies<P: Package, V: Version> {
    /// Package dependencies are unavailable.
    Unknown,
    /// Container for all available package versions.
    Known(DependencyConstraints<P, V>),
}

/// Subtype of [Dependencies] which holds information about
/// all possible versions a given package can accept.
/// There is a difference in semantics between an empty [Map<P, Range<V>>](crate::type_aliases::Map)
/// inside [DependencyConstraints] and [Dependencies::Unknown]:
/// the former means the package has no dependencies and it is a known fact,
/// while the latter means they could not be fetched by [DependencyProvider].
pub type DependencyConstraints<P, V> = Map<P, Range<V>>;

/// Trait that allows the algorithm to retrieve available packages and their dependencies.
/// An implementor needs to be supplied to the [resolve] function.
pub trait DependencyProvider<P: Package, V: Version> {
    /// Lists available versions for a given package.
    /// The strategy of which version should be preferably picked in the list of available versions
    /// is implied by the order of the list: first version in the list will be tried first.
    fn list_available_versions(&self, package: &P) -> Result<Vec<V>, Box<dyn Error>>;

    /// Retrieves the package dependencies.
    /// Return [Dependencies::Unknown] if its dependencies are unknown.
    fn get_dependencies(
        &self,
        package: &P,
        version: &V,
    ) -> Result<Dependencies<P, V>, Box<dyn Error>>;

    /// This is called fairly regularly during the resolution,
    /// if it returns an Err then resolution will be terminated.
    /// This is helpful if you want to add some form of early termination like a timeout,
    /// or you want to add some form of user feedback if things are taking a while.
    /// If not provided the resolver will run as long as needed.
    fn should_cancel(&self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

/// A basic implementation of [DependencyProvider].
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct OfflineDependencyProvider<P: Package, V: Version + Hash> {
    dependencies: Map<P, Map<V, DependencyConstraints<P, V>>>,
}

impl<P: Package, V: Version + Hash> OfflineDependencyProvider<P, V> {
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
    pub fn add_dependencies<I: IntoIterator<Item = (P, Range<V>)>>(
        &mut self,
        package: P,
        version: impl Into<V>,
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

    /// Lists packages that have bean saved.
    pub fn packages(&self) -> impl Iterator<Item = &P> {
        self.dependencies.keys()
    }

    /// Lists versions of saved packages.
    /// Returns [None] if no information is available regarding that package.
    fn versions(&self, package: &P) -> Option<Set<V>> {
        self.dependencies
            .get(package)
            .map(|k| k.keys().cloned().collect())
    }

    /// Lists dependencies of a given package and version.
    /// Returns [None] if no information is available regarding that package and version pair.
    fn dependencies(&self, package: &P, version: &V) -> Option<DependencyConstraints<P, V>> {
        self.dependencies.get(package)?.get(version).cloned()
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
            .unwrap_or_default())
    }

    fn get_dependencies(
        &self,
        package: &P,
        version: &V,
    ) -> Result<Dependencies<P, V>, Box<dyn Error>> {
        Ok(match self.dependencies(package, version) {
            None => Dependencies::Unknown,
            Some(dependencies) => Dependencies::Known(dependencies),
        })
    }
}
