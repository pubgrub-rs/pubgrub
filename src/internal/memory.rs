// SPDX-License-Identifier: MPL-2.0

//! A Memory acts like a structured partial solution
//! where terms are regrouped by package in a [Map](crate::type_aliases::Map).

use crate::internal::assignment::Assignment::{self, Decision, Derivation};
use crate::package::Package;
use crate::term::Term;
use crate::type_aliases::{Map, SelectedDependencies};
use crate::version::Version;

/// A memory is the set of all assignments in the partial solution,
/// organized by package instead of historically ordered.
///
/// Contrary to PartialSolution, Memory does not store derivations causes, only the terms.
#[derive(Clone)]
pub struct Memory<P: Package, V: Version> {
    assignments: Map<P, PackageAssignments<V>>,
}

/// A package memory contains the potential decision and derivations
/// that have already been made for a given package.
#[derive(Clone)]
enum PackageAssignments<V: Version> {
    Decision((V, Term<V>)),
    Derivations {
        intersected: Term<V>,
        not_intersected_yet: Vec<Term<V>>,
    },
}

impl<P: Package, V: Version> Memory<P, V> {
    /// Initialize an empty memory.
    pub fn empty() -> Self {
        Self {
            assignments: Map::default(),
        }
    }

    /// Retrieve intersection of terms in memory related to package.
    pub fn term_intersection_for_package(&mut self, package: &P) -> Option<&Term<V>> {
        self.assignments
            .get_mut(package)
            .map(|pa| pa.assignment_intersection())
    }

    /// Building step of a Memory from a given assignment.
    pub fn add_assignment(&mut self, assignment: &Assignment<P, V>) {
        match assignment {
            Decision { package, version } => self.add_decision(package.clone(), version.clone()),
            Derivation { package, term, .. } => self.add_derivation(package.clone(), term.clone()),
        }
    }

    /// Add a decision to a Memory.
    fn add_decision(&mut self, package: P, version: V) {
        if cfg!(debug_assertions) {
            match self.assignments.get(&package) {
                Some(PackageAssignments::Decision(v)) => assert_eq!(v.0, version),
                Some(PackageAssignments::Derivations {
                    intersected,
                    not_intersected_yet,
                }) => {
                    debug_assert!(intersected.contains(&version));
                    for term in not_intersected_yet {
                        debug_assert!(term.contains(&version));
                    }
                }
                _ => {}
            }
        }

        self.assignments.insert(
            package,
            PackageAssignments::Decision((version.clone(), Term::exact(version))),
        );
    }

    /// Add a derivation to a Memory.
    fn add_derivation(&mut self, package: P, term: Term<V>) {
        let pa =
            self.assignments
                .entry(package)
                .or_insert_with(|| PackageAssignments::Derivations {
                    intersected: Term::any(),
                    not_intersected_yet: Vec::with_capacity(1),
                });
        match pa {
            PackageAssignments::Decision((version, _)) => {
                if cfg!(debug_assertions) {
                    debug_assert!(term.contains(&version))
                }
            }
            PackageAssignments::Derivations {
                intersected: _,
                not_intersected_yet,
            } => {
                not_intersected_yet.push(term);
            }
        }
    }

    /// Extract all packages that may potentially be picked next
    /// to continue solving package dependencies.
    /// A package is a potential pick if there isn't an already
    /// selected version (no "decision")
    /// and if it contains at least one positive derivation term
    /// in the partial solution.
    pub fn potential_packages(&mut self) -> impl Iterator<Item = (&P, &Term<V>)> {
        self.assignments
            .iter_mut()
            .filter_map(|(p, pa)| pa.potential_package_filter(p))
    }

    /// If a partial solution has, for every positive derivation,
    /// a corresponding decision that satisfies that assignment,
    /// it's a total solution and version solving has succeeded.
    pub fn extract_solution(&self) -> Option<SelectedDependencies<P, V>> {
        if self.assignments.iter().all(|(_, pa)| match pa {
            PackageAssignments::Decision(_) => true,
            PackageAssignments::Derivations {
                intersected,
                not_intersected_yet,
            } => intersected.is_negative() && not_intersected_yet.iter().all(|t| t.is_negative()),
        }) {
            Some(
                self.assignments
                    .iter()
                    .filter_map(|(p, pa)| match pa {
                        PackageAssignments::Decision((v, _)) => Some((p.clone(), v.clone())),
                        PackageAssignments::Derivations { .. } => None,
                    })
                    .collect(),
            )
        } else {
            None
        }
    }
}

impl<V: Version> PackageAssignments<V> {
    /// Returns intersection of all assignments (decision included).
    /// Mutates itself to store the intersection result.
    fn assignment_intersection(&mut self) -> &Term<V> {
        match self {
            PackageAssignments::Decision((_, term)) => &*term,
            PackageAssignments::Derivations {
                intersected,
                not_intersected_yet,
            } => {
                for derivation in not_intersected_yet.drain(..) {
                    *intersected = intersected.intersection(&derivation);
                }
                &*intersected
            }
        }
    }

    /// A package is a potential pick if there isn't an already
    /// selected version (no "decision")
    /// and if it contains at least one positive derivation term
    /// in the partial solution.
    fn potential_package_filter<'a, P: Package>(
        &'a mut self,
        package: &'a P,
    ) -> Option<(&'a P, &'a Term<V>)> {
        match self {
            PackageAssignments::Decision(_) => None,
            PackageAssignments::Derivations {
                intersected,
                not_intersected_yet,
            } => {
                if intersected.is_positive() || not_intersected_yet.iter().any(|t| t.is_positive())
                {
                    Some((package, self.assignment_intersection()))
                } else {
                    None
                }
            }
        }
    }
}
