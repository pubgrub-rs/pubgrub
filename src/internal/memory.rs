// SPDX-License-Identifier: MPL-2.0

//! A Memory acts like a structured partial solution
//! where terms are regrouped by package in a [Map](crate::type_aliases::Map).

use crate::internal::assignment::Assignment::{self, Decision, Derivation};
use crate::package::Package;
use crate::term::Term;
use crate::type_aliases::Map;
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
struct PackageAssignments<V: Version> {
    decision: Option<(V, Term<V>)>,
    derivations_intersected: Term<V>,
    derivations_not_intersected_yet: Vec<Term<V>>,
}

impl<P: Package, V: Version> Memory<P, V> {
    /// Initialize an empty memory.
    pub fn empty() -> Self {
        Self {
            assignments: Map::default(),
        }
    }

    /// Retrieve intersection of terms in memory related to package.
    pub fn term_intersection_for_package(&mut self, package: &P) -> Term<V> {
        match self.assignments.get_mut(package) {
            None => Term::any(),
            Some(pa) => pa.assignment_intersection(),
        }
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
        let pa = self
            .assignments
            .entry(package)
            .or_insert(PackageAssignments::new());
        pa.decision = Some((version.clone(), Term::exact(version)));
    }

    /// Remove a decision from a Memory.
    pub fn remove_decision(&mut self, package: &P) {
        self.assignments
            .get_mut(package)
            .map(|pa| pa.decision = None);
    }

    /// Add a derivation to a Memory.
    fn add_derivation(&mut self, package: P, term: Term<V>) {
        let pa = self
            .assignments
            .entry(package)
            .or_insert(PackageAssignments::new());
        pa.derivations_not_intersected_yet.push(term);
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
    pub fn extract_solution(&self) -> Option<Map<P, V>> {
        if self.assignments.values().all(|pa| pa.is_valid()) {
            Some(
                self.assignments
                    .iter()
                    .filter_map(|(p, pa)| pa.decision.as_ref().map(|v| (p.clone(), v.0.clone())))
                    .collect(),
            )
        } else {
            None
        }
    }
}

impl<V: Version> PackageAssignments<V> {
    /// Empty package assignment
    fn new() -> Self {
        Self {
            decision: None,
            derivations_intersected: Term::any(),
            derivations_not_intersected_yet: Vec::new(),
        }
    }

    /// If a partial solution has, for every positive derivation,
    /// a corresponding decision that satisfies that assignment,
    /// it's a total solution and version solving has succeeded.
    fn is_valid(&self) -> bool {
        match self.decision {
            None => {
                self.derivations_intersected.is_negative()
                    && self
                        .derivations_not_intersected_yet
                        .iter()
                        .all(|t| t.is_negative())
            }
            Some(_) => true,
        }
    }

    /// Returns intersection of all assignments (decision included).
    /// Mutates itself to store the intersection result.
    fn assignment_intersection(&mut self) -> Term<V> {
        self.derivation_intersection();
        match &self.decision {
            None => self.derivations_intersected.clone(),
            Some((_, decision_term)) => decision_term.intersection(&self.derivations_intersected),
        }
    }

    /// Returns intersection of all derivation terms.
    /// Mutates itself to store the intersection result.
    fn derivation_intersection(&mut self) -> &Term<V> {
        for derivation in self.derivations_not_intersected_yet.iter() {
            self.derivations_intersected = self.derivations_intersected.intersection(derivation);
        }
        self.derivations_not_intersected_yet.clear();
        &self.derivations_intersected
    }

    /// A package is a potential pick if there isn't an already
    /// selected version (no "decision")
    /// and if it contains at least one positive derivation term
    /// in the partial solution.
    fn potential_package_filter<'a, 'b, P: Package>(
        &'a mut self,
        package: &'b P,
    ) -> Option<(&'b P, &'a Term<V>)> {
        if self.decision == None
            && (self.derivations_intersected.is_positive()
                || self
                    .derivations_not_intersected_yet
                    .iter()
                    .any(|t| t.is_positive()))
        {
            Some((package, self.derivation_intersection()))
        } else {
            None
        }
    }
}
