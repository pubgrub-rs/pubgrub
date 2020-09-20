// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! A Memory acts like a structured partial solution
//! where terms are regrouped by package in a hashmap.

use std::collections::HashMap as Map;
use std::hash::Hash;

use crate::internal::assignment::Assignment;
use crate::internal::assignment::Kind;
use crate::internal::term::Term;
use crate::range::Range;
use crate::version::Version;

/// A memory is the set of all assignments in the partial solution,
/// organized by package instead of historically ordered.
///
/// Contrary to PartialSolution, Memory does not store derivations causes, only the terms.
#[derive(Clone)]
pub struct Memory<P, V>
where
    P: Clone + Eq + Hash,
    V: Clone + Ord + Version,
{
    assignments: Map<P, PackageAssignments<V>>,
}

/// A package memory contains the potential decision and derivations
/// that have already been made for a given package.
#[derive(Clone)]
struct PackageAssignments<V: Clone + Ord + Version> {
    decision: Option<V>,
    derivations: Vec<Term<V>>,
}

impl<P, V> Memory<P, V>
where
    P: Clone + Eq + Hash,
    V: Clone + Ord + Version,
{
    /// Initialize an empty memory.
    pub fn empty() -> Self {
        Self {
            assignments: Map::new(),
        }
    }

    /// Initialize a Memory from a decision.
    ///
    /// TODO: is this actually used?
    pub fn from_decision(package: P, version: V) -> Self {
        let mut assignments = Map::new();
        assignments.insert(
            package,
            PackageAssignments {
                decision: Some(version),
                derivations: Vec::new(),
            },
        );
        Memory { assignments }
    }

    /// Initialize a Memory from a derivation.
    ///
    /// TODO: is this actually used?
    pub fn from_derivation(package: P, term: Term<V>) -> Self {
        let mut assignments = Map::new();
        assignments.insert(
            package,
            PackageAssignments {
                decision: None,
                derivations: vec![term],
            },
        );
        Memory { assignments }
    }

    /// Retrieve all terms in memory.
    pub fn all_terms(&self) -> Map<P, impl Iterator<Item = Term<V>> + '_> {
        let identity = |x| x;
        self.assignments
            .iter()
            .map(|(p, a)| match &a.decision {
                None => (
                    p.clone(),
                    // The once(None).filter_map(identity) is just to produce
                    // the same type on both branches of the match,
                    // otherwise the code would not compile.
                    std::iter::once(None)
                        .filter_map(identity)
                        .chain(a.derivations.iter().cloned()),
                ),
                Some(version) => (
                    p.clone(),
                    std::iter::once(Some(Term::Positive(Range::exact(version.clone()))))
                        .filter_map(identity)
                        .chain(a.derivations.iter().cloned()),
                ),
            })
            .collect()
    }

    /// Building step of a Memory from a given assignment.
    pub fn add_assignment(&mut self, assignment: &Assignment<P, V>) {
        match &assignment.kind {
            Kind::Decision(version) => {
                self.add_decision(assignment.package.clone(), version.clone())
            }
            Kind::Derivation { term, .. } => {
                self.add_derivation(assignment.package.clone(), term.clone())
            }
        }
    }

    /// Add a decision to a Memory.
    pub fn add_decision(&mut self, package: P, version: V) {
        match self.assignments.get_mut(&package) {
            None => {
                self.assignments.insert(
                    package,
                    PackageAssignments {
                        decision: Some(version),
                        derivations: Vec::new(),
                    },
                );
            }
            Some(package_assignments) => package_assignments.decision = Some(version),
        }
    }

    /// Remove a decision from a Memory.
    pub fn remove_decision(&mut self, package: &P) {
        match self.assignments.get_mut(package) {
            None => {}
            Some(package_assignments) => package_assignments.decision = None,
        }
    }

    /// Add a derivation to a Memory.
    pub fn add_derivation(&mut self, package: P, term: Term<V>) {
        match self.assignments.get_mut(&package) {
            None => {
                self.assignments.insert(
                    package,
                    PackageAssignments {
                        decision: None,
                        derivations: vec![term],
                    },
                );
            }
            Some(package_assignments) => package_assignments.derivations.push(term),
        }
    }

    /// Extract all packages that may potentially be picked next
    /// to continue solving package dependencies.
    /// A package is a potential pick if there isn't an already
    /// selected version (no "decision")
    /// and if it contains at least one positive derivation term
    /// in the partial solution.
    pub fn potential_packages(&self) -> impl Iterator<Item = (&P, &[Term<V>])> {
        self.assignments
            .iter()
            .filter_map(|(p, pa)| Self::potential_package_filter(p, pa))
    }

    fn potential_package_filter<'a, 'b>(
        package: &'a P,
        package_assignments: &'b PackageAssignments<V>,
    ) -> Option<(&'a P, &'b [Term<V>])> {
        if &package_assignments.decision == &None
            && package_assignments
                .derivations
                .iter()
                .any(|t| t.is_positive())
        {
            Some((package, package_assignments.derivations.as_slice()))
        } else {
            None
        }
    }

    /// If a partial solution has, for every positive derivation,
    /// a corresponding decision that satisfies that assignment,
    /// it's a total solution and version solving has succeeded.
    pub fn extract_solution(&self) -> Option<Map<P, V>> {
        if self.assignments.iter().all(|(_, pa)| pa.is_valid()) {
            Some(
                self.assignments
                    .iter()
                    .filter_map(|(p, pa)| pa.decision.as_ref().map(|v| (p.clone(), v.clone())))
                    .collect(),
            )
        } else {
            None
        }
    }
}

impl<V: Clone + Ord + Version> PackageAssignments<V> {
    pub fn is_valid(&self) -> bool {
        if self.decision == None {
            !(self.derivations.iter().any(|t| t.is_positive()))
        } else {
            true
        }
    }
}
