// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! A Memory acts like a structured partial solution
//! where terms are regrouped by package in a [Map](crate::Map).

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
    derivations: Vec<Term<V>>,
}

impl<P: Package, V: Version> Memory<P, V> {
    /// Initialize an empty memory.
    pub fn empty() -> Self {
        Self {
            assignments: Map::default(),
        }
    }

    /// Retrieve terms in memory related to package.
    pub fn terms_for_package(&self, package: &P) -> impl Iterator<Item = &Term<V>> {
        self.assignments.get(package).into_iter().flat_map(|a| {
            let decision_iter = a.decision.iter().map(|(_, term)| term);
            decision_iter.chain(a.derivations.iter())
        })
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
        pa.derivations.push(term);
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
            derivations: Vec::new(),
        }
    }

    /// If a partial solution has, for every positive derivation,
    /// a corresponding decision that satisfies that assignment,
    /// it's a total solution and version solving has succeeded.
    fn is_valid(&self) -> bool {
        match self.decision {
            None => self.derivations.iter().all(|t| t.is_negative()),
            Some(_) => true,
        }
    }
}
