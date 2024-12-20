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
//! the [Package] and Version traits.
//! [Package] is strictly equivalent and automatically generated
//! for any type that implement [Clone] + [Eq] + [Hash] + [Debug] + [Display].
//!
//! ## API
//!
//! ```
//! # use std::convert::Infallible;
//! # use pubgrub::{resolve, OfflineDependencyProvider, PubGrubError, Ranges};
//! #
//! # type NumVS = Ranges<u32>;
//! #
//! # fn try_main() -> Result<(), PubGrubError<OfflineDependencyProvider<&'static str, NumVS>>> {
//! #     let dependency_provider = OfflineDependencyProvider::<&str, NumVS>::new();
//! #     let package = "root";
//! #     let version = 1u32;
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
use std::fmt::{Debug, Display};

use log::{debug, info};

use crate::internal::{Id, Incompatibility, State};
use crate::{DependencyConstraints, Map, Package, PubGrubError, SelectedDependencies, VersionSet};

/// Statistics on how often a package conflicted with other packages.
#[derive(Debug, Default, Clone)]
pub struct PackageResolutionStatistics {
    // We track these fields separately but currently don't expose them separately to keep the
    // stable API slim. Please be encouraged to try different combinations of them and report if
    // you find better metrics that should be exposed.
    //
    // Say we have packages A and B, A having higher priority than B. We first decide A and then B,
    // and then find B to conflict with A. We call be B "affected" and A "culprit" since the
    // decisions for B is being rejected due to the decision we made for A earlier.
    //
    // If B is rejected due to its dependencies conflicting with A, we increase
    // `dependencies_affected` for B and for `dependencies_culprit` A. If B is rejected in unit
    // through an incompatibility with B, we increase `unit_propagation_affected` for B and for
    // `unit_propagation_culprit` A.
    unit_propagation_affected: u32,
    unit_propagation_culprit: u32,
    dependencies_affected: u32,
    dependencies_culprit: u32,
}

impl PackageResolutionStatistics {
    /// The number of conflicts this package was involved in.
    ///
    /// Processing packages with a high conflict count earlier usually speeds up resolution.
    ///
    /// Whenever a package is part of the root cause incompatibility of a conflict, we increase its
    /// count by one. Since the structure of the incompatibilities may change, this count too may
    /// change in the future.
    pub fn conflict_count(&self) -> u32 {
        self.unit_propagation_affected
            + self.unit_propagation_culprit
            + self.dependencies_affected
            + self.dependencies_culprit
    }
}

/// Main function of the library.
/// Finds a set of packages satisfying dependency bounds for a given package + version pair.
#[cold]
pub fn resolve<DP: DependencyProvider>(
    dependency_provider: &DP,
    package: DP::P,
    version: impl Into<DP::V>,
) -> Result<SelectedDependencies<DP>, PubGrubError<DP>> {
    let mut state: State<DP> = State::init(package.clone(), version.into());
    let mut conflict_tracker: Map<Id<DP::P>, PackageResolutionStatistics> = Map::default();
    let mut added_dependencies: Map<Id<DP::P>, Set<DP::V>> = Map::default();
    let mut next = state.root_package;
    loop {
        dependency_provider
            .should_cancel()
            .map_err(|err| PubGrubError::ErrorInShouldCancel(err))?;

        info!(
            "unit_propagation: {:?} = '{}'",
            &next, state.package_store[next]
        );
        let root_causes = state.unit_propagation(next)?;
        for (affected, incompat) in root_causes {
            conflict_tracker
                .entry(affected)
                .or_default()
                .unit_propagation_affected += 1;
            for (conflict_package, _) in state.incompatibility_store[incompat].iter() {
                if conflict_package == affected {
                    continue;
                }
                conflict_tracker
                    .entry(conflict_package)
                    .or_default()
                    .unit_propagation_culprit += 1;
            }
        }

        debug!(
            "Partial solution after unit propagation: {}",
            state.partial_solution.display(&state.package_store)
        );

        let Some(highest_priority_pkg) =
            state.partial_solution.pick_highest_priority_pkg(|p, r| {
                dependency_provider.prioritize(
                    &state.package_store[p],
                    r,
                    conflict_tracker.entry(p).or_default(),
                )
            })
        else {
            return Ok(state
                .partial_solution
                .extract_solution()
                .map(|(p, v)| (state.package_store[p].clone(), v))
                .collect());
        };
        next = highest_priority_pkg;

        let term_intersection = state
            .partial_solution
            .term_intersection_for_package(next)
            .ok_or_else(|| {
                PubGrubError::Failure("a package was chosen but we don't have a term.".into())
            })?;
        let decision = dependency_provider
            .choose_version(
                &state.package_store[next],
                term_intersection.unwrap_positive(),
            )
            .map_err(PubGrubError::ErrorChoosingPackageVersion)?;

        info!(
            "DP chose: {:?} = '{}' @ {:?}",
            &next, state.package_store[next], decision
        );

        // Pick the next compatible version.
        let v = match decision {
            None => {
                let inc = Incompatibility::no_versions(next, term_intersection.clone());
                state.add_incompatibility(inc);
                continue;
            }
            Some(x) => x,
        };

        if !term_intersection.contains(&v) {
            return Err(PubGrubError::Failure(
                "choose_package_version picked an incompatible version".into(),
            ));
        }

        let is_new_dependency = added_dependencies
            .entry(next)
            .or_default()
            .insert(v.clone());

        if is_new_dependency {
            // Retrieve that package dependencies.
            let p = next;
            let dependencies = dependency_provider
                .get_dependencies(&state.package_store[p], &v)
                .map_err(|err| PubGrubError::ErrorRetrievingDependencies {
                    package: state.package_store[p].clone(),
                    version: v.clone(),
                    source: err,
                })?;

            let dependencies = match dependencies {
                Dependencies::Unavailable(reason) => {
                    state.add_incompatibility(Incompatibility::custom_version(
                        p,
                        v.clone(),
                        reason,
                    ));
                    continue;
                }
                Dependencies::Available(x) => x,
            };

            // Add that package and version if the dependencies are not problematic.
            if let Some(conflict) =
                state.add_package_version_dependencies(p, v.clone(), dependencies)
            {
                conflict_tracker.entry(p).or_default().dependencies_affected += 1;
                for (incompat_package, _) in state.incompatibility_store[conflict].iter() {
                    if incompat_package == p {
                        continue;
                    }
                    conflict_tracker
                        .entry(incompat_package)
                        .or_default()
                        .dependencies_culprit += 1;
                }
            }
        } else {
            // `dep_incompats` are already in `incompatibilities` so we know there are not satisfied
            // terms and can add the decision directly.
            info!(
                "add_decision (not first time): {:?} = '{}' @ {}",
                &next, state.package_store[next], v
            );
            state.partial_solution.add_decision(next, v);
        }
    }
}

/// An enum used by [DependencyProvider] that holds information about package dependencies.
/// For each [Package] there is a set of versions allowed as a dependency.
#[derive(Clone)]
pub enum Dependencies<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> {
    /// Package dependencies are unavailable with the reason why they are missing.
    Unavailable(M),
    /// Container for all available package versions.
    Available(DependencyConstraints<P, VS>),
}

/// Trait that allows the algorithm to retrieve available packages and their dependencies.
/// An implementor needs to be supplied to the [resolve] function.
pub trait DependencyProvider {
    /// How this provider stores the name of the packages.
    type P: Package;

    /// How this provider stores the versions of the packages.
    ///
    /// A common choice is [`SemanticVersion`][crate::version::SemanticVersion].
    type V: Debug + Display + Clone + Ord;

    /// How this provider stores the version requirements for the packages.
    /// The requirements must be able to process the same kind of version as this dependency provider.
    ///
    /// A common choice is [`Ranges`][version_ranges::Ranges].
    type VS: VersionSet<V = Self::V>;

    /// Type for custom incompatibilities.
    ///
    /// There are reasons in user code outside pubgrub that can cause packages or versions
    /// to be unavailable. Examples:
    /// * The version would require building the package, but builds are disabled.
    /// * The package is not available in the cache, but internet access has been disabled.
    /// * The package uses a legacy format not supported anymore.
    ///
    /// The intended use is to track them in an enum and assign them to this type. You can also
    /// assign [`String`] as placeholder.
    type M: Eq + Clone + Debug + Display;

    /// [Decision making](https://github.com/dart-lang/pub/blob/master/doc/solver.md#decision-making)
    /// is the process of choosing the next package
    /// and version that will be appended to the partial solution.
    ///
    /// Every time such a decision must be made, the resolver looks at all the potential valid
    /// packages that have changed, and a asks the dependency provider how important each one is.
    /// For each one it calls `prioritize` with the name of the package and the current set of
    /// acceptable versions.
    /// The resolver will then pick the package with the highes priority from all the potential valid
    /// packages.
    ///
    /// The strategy employed to prioritize packages
    /// cannot change the existence of a solution or not,
    /// but can drastically change the performances of the solver,
    /// or the properties of the solution.
    /// The documentation of Pub (PubGrub implementation for the dart programming language)
    /// states the following:
    ///
    /// > Pub chooses the latest matching version of the package
    /// > with the fewest versions that match the outstanding constraint.
    /// > This tends to find conflicts earlier if any exist,
    /// > since these packages will run out of versions to try more quickly.
    /// > But there's likely room for improvement in these heuristics.
    ///
    /// The `package_conflicts_counts` argument provides access to some other heuristics that
    /// are production users have found useful. Although the exact meaning/efficacy of those arguments may change.
    ///
    /// If two packages have the same priority, PubGrub will biased toward a breadth first search.
    ///
    /// Note: the resolver may call this even when the range has not changed,
    /// if it is more efficient for the resolvers internal data structures.
    fn prioritize(
        &self,
        package: &Self::P,
        range: &Self::VS,
        package_conflicts_counts: &PackageResolutionStatistics,
    ) -> Self::Priority;
    /// The type returned from `prioritize`. The resolver does not care what type this is
    /// as long as it can pick a largest one and clone it.
    ///
    /// [`Reverse`](std::cmp::Reverse) can be useful if you want to pick the package with
    /// the fewest versions that match the outstanding constraint.
    type Priority: Ord + Clone;

    /// The kind of error returned from these methods.
    ///
    /// Returning this signals that resolution should fail with this error.
    type Err: Error + 'static;

    /// Once the resolver has found the highest `Priority` package from all potential valid
    /// packages, it needs to know what version of that package to use. The most common pattern
    /// is to select the largest version that the range contains.
    fn choose_version(
        &self,
        package: &Self::P,
        range: &Self::VS,
    ) -> Result<Option<Self::V>, Self::Err>;

    /// Retrieves the package dependencies.
    /// Return [Dependencies::Unavailable] if its dependencies are unavailable.
    #[allow(clippy::type_complexity)]
    fn get_dependencies(
        &self,
        package: &Self::P,
        version: &Self::V,
    ) -> Result<Dependencies<Self::P, Self::VS, Self::M>, Self::Err>;

    /// This is called fairly regularly during the resolution,
    /// if it returns an Err then resolution will be terminated.
    /// This is helpful if you want to add some form of early termination like a timeout,
    /// or you want to add some form of user feedback if things are taking a while.
    /// If not provided the resolver will run as long as needed.
    fn should_cancel(&self) -> Result<(), Self::Err> {
        Ok(())
    }
}
