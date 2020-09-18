// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! The partial solution is the current state
//! of the solution being built by the algorithm.

use std::collections::HashMap as Map;
use std::hash::Hash;

use crate::internal::assignment::Assignment;
use crate::internal::incompatibility::Incompatibility;
use crate::internal::incompatibility::Relation;
use crate::internal::memory::Memory;
use crate::internal::term::Term;
use crate::range::Range;
use crate::version::Version;

/// The partial solution is the current state
/// of the solution being built by the algorithm.
/// It is composed of a succession of assignments,
/// defined as either decisions or derivations.
///
/// TODO: make sure that when I use the history,
/// it is in the correct direction.
#[derive(Clone)]
pub struct PartialSolution<'a, P, V>
where
    P: Clone + Eq + Hash,
    V: Clone + Ord + Version,
{
    decision_level: usize,
    history: Vec<Assignment<'a, P, V>>,
    memory: Memory<P, V>,
}

impl<'a, P, V> PartialSolution<'a, P, V>
where
    P: Clone + Eq + Hash,
    V: Clone + Ord + Version,
{
    /// Initialize an empty partial solution.
    pub fn empty() -> Self {
        Self {
            decision_level: 0,
            history: Vec::new(),
            memory: Memory::empty(),
        }
    }

    fn from_assignments(assignments: Vec<Assignment<'a, P, V>>) -> Self {
        let mut decision_level = 0;
        let memory = assignments.iter().fold(Memory::empty(), |mut m, a| {
            m.add_assignment(a);
            decision_level = decision_level.max(a.decision_level);
            m
        });
        Self {
            decision_level,
            history: assignments,
            memory,
        }
    }

    /// If a partial solution has, for every positive derivation,
    /// a corresponding decision that satisfies that assignment,
    /// it's a total solution and version solving has succeeded.
    pub fn extract_solution(&self) -> Option<Map<P, V>> {
        self.memory.extract_solution()
    }

    /// Add a decision to the partial solution.
    pub fn add_decision(&mut self, package: P, version: V) {
        self.decision_level += 1;
        self.history.push(Assignment::new_decision(
            self.decision_level,
            package.clone(),
            version.clone(),
        ));
        self.memory.add_decision(package, version);
    }

    /// Add a derivation to the partial solution.
    pub fn add_derivation(
        &mut self,
        package: P,
        term: Term<V>,
        cause: &'a Incompatibility<'a, P, V>,
    ) {
        self.history.push(Assignment::new_derivation(
            self.decision_level,
            package.clone(),
            term.clone(),
            cause,
        ));
        self.memory.add_derivation(package, term);
    }

    /// Backtrack the partial solution to a given decision level.
    pub fn backtrack(self, decision_level: usize) -> Self {
        let mut history = self.history;
        let index =
            Self::find_level_index(history.as_slice(), decision_level + 1).unwrap_or(history.len());
        history.truncate(index);
        Self::from_assignments(history)
    }

    // TODO: improve with dichotomic search.
    fn find_level_index(history: &[Assignment<'a, P, V>], decision_level: usize) -> Option<usize> {
        for (index, level) in history.iter().map(|a| a.decision_level).enumerate() {
            if level >= decision_level {
                return Some(index);
            }
        }
        None
    }

    /// Heuristic to pick the next package to add to the partial solution.
    /// This should be a package with a positive derivation but no decision yet.
    /// If multiple choices are possible, use a heuristic.
    ///
    /// Pub chooses the package with the fewest versions
    /// matching the outstanding constraint.
    /// This tends to find conflicts earlier if any exist,
    /// since these packages will run out of versions to try more quickly.
    /// But there's likely room for improvement in these heuristics.
    ///
    /// Here we just pick the first one.
    /// TODO: improve?
    /// TODO: do not introduce any side effect if trying to improve.
    pub fn pick_package(&self) -> Option<(P, Term<V>)> {
        self.potential_packages().next().map(|(p, all_terms)| {
            let all_terms_intersection =
                Term::intersect_all(all_terms.iter()).unwrap_or(Term::Negative(Range::none()));
            (p.clone(), all_terms_intersection)
        })
    }

    /// Extract all packages that may potentially be picked next
    /// to continue solving package dependencies.
    /// A package is a potential pick if there isn't an already
    /// version selected (no "decision")
    /// and if it contains at least one positive derivation term
    /// in the partial solution.
    pub fn potential_packages(&self) -> impl Iterator<Item = (&P, &[Term<V>])> {
        self.memory.potential_packages()
    }

    /// Pub chooses the latest matching version of the package
    /// that match the outstanding constraint.
    ///
    /// Here we just pick the first one that satisfies the terms.
    /// It is the responsibility of the provider of `availableVersions`
    /// to list them with preferred versions first.
    pub fn pick_version(available_versions: &[V], partial_solution_term: &Term<V>) -> Option<V> {
        available_versions
            .iter()
            .find(|v| partial_solution_term.accept_version(v))
            .map(|v| v.clone())
    }

    /// We can add the version to the partial solution as a decision
    /// if it doesn't produce any conflict with the new incompatibilities.
    /// In practice I think it can only produce a conflict if one of the dependencies
    /// (which are used to make the new incompatibilities)
    /// is already in the partial solution with an incompatible version.
    pub fn add_version(
        &self,
        package: P,
        version: V,
        new_incompatibilities: &[Incompatibility<'a, P, V>],
    ) -> Option<PartialSolution<'a, P, V>> {
        let mut updated_partial_solution = self.clone();
        updated_partial_solution.add_decision(package, version);
        if updated_partial_solution.satisfies_any_of(new_incompatibilities) {
            None
        } else {
            Some(updated_partial_solution)
        }
    }

    fn satisfies_any_of(&self, incompatibilities: &[Incompatibility<'a, P, V>]) -> bool {
        incompatibilities
            .iter()
            .any(|incompat| self.relation(incompat) == Relation::Satisfied)
    }

    /// Check if the terms in the partial solution satisfy the incompatibility.
    pub fn relation(&self, incompat: &Incompatibility<'a, P, V>) -> Relation<P, V> {
        incompat.relation(&mut self.memory.all_terms())
    }

    /// Find satisfier and previous satisfier decision level.
    pub fn find_satisfier_and_previous_satisfier_level(
        &'a self,
        incompat: &Incompatibility<'a, P, V>,
    ) -> (&'a Assignment<'a, P, V>, usize) {
        let (satisfier, previous_assignments) =
            Self::find_satisfier(incompat, self.history.as_slice())
                .expect("We should always find a satisfier if called in the right context.");
        let previous_satisfier_level =
            Self::find_previous_satisfier(incompat, satisfier, previous_assignments)
                .map_or(1, |(previous_satisfier, _)| {
                    previous_satisfier.decision_level.max(1)
                });
        (satisfier, previous_satisfier_level)
    }

    /// A satisfier is the earliest assignment in partial solution such that the incompatibility
    /// is satisfied by the partial solution up to and including that assignment.
    /// Also returns all assignments earlier than the satisfier.
    fn find_satisfier(
        incompat: &Incompatibility<'a, P, V>,
        history: &'a [Assignment<'a, P, V>],
    ) -> Option<(&'a Assignment<'a, P, V>, &'a [Assignment<'a, P, V>])> {
        let mut accum_satisfier: Map<P, (bool, Term<V>)> = incompat
            .iter()
            .map(|(p, _)| (p.clone(), (false, Term::Negative(Range::none()))))
            .collect();
        Self::find_satisfier_helper(incompat, &mut accum_satisfier, history)
    }

    /// Earliest assignment in the partial solution before satisfier
    /// such that incompatibility is satisfied by the partial solution up to
    /// and including that assignment plus satisfier.
    fn find_previous_satisfier(
        incompat: &Incompatibility<'a, P, V>,
        satisfier: &Assignment<'a, P, V>,
        previous_assignments: &'a [Assignment<'a, P, V>],
    ) -> Option<(&'a Assignment<'a, P, V>, &'a [Assignment<'a, P, V>])> {
        let mut accum_satisfier: Map<P, (bool, Term<V>)> = incompat
            .iter()
            .map(|(p, _)| (p.clone(), (false, Term::Negative(Range::none()))))
            .collect();
        // Add the satisfier to accum_satisfier.
        let incompat_term = incompat.get(&satisfier.package).expect("This should exist");
        let satisfier_term = satisfier.as_term();
        let is_satisfied = satisfier_term.subset_of(incompat_term);
        accum_satisfier.insert(satisfier.package.clone(), (is_satisfied, satisfier_term));
        // Search previous satisfier.
        Self::find_satisfier_helper(incompat, &mut accum_satisfier, previous_assignments)
    }

    /// Iterate over the assignments (oldest must be first)
    /// until we find the first one such that the set of all assignments until this one (included)
    /// satisfies the given incompatibility.
    pub fn find_satisfier_helper(
        incompat: &Incompatibility<'a, P, V>,
        accum_satisfier: &mut Map<P, (bool, Term<V>)>,
        all_assignments: &'a [Assignment<'a, P, V>],
    ) -> Option<(&'a Assignment<'a, P, V>, &'a [Assignment<'a, P, V>])> {
        for (idx, assignment) in all_assignments.iter().enumerate() {
            // We only care about packages related to the incompatibility.
            if let Some(incompat_term) = incompat.get(&assignment.package) {
                // Check if that incompat term is satisfied by our accumulated terms intersection.
                match accum_satisfier.get_mut(&assignment.package) {
                    None => panic!("A package in incompat should always exist in accum_satisfier"),
                    Some((true, _)) => {} // If that package term is already satisfied, no need to check.
                    Some((is_satisfied, accum_term)) => {
                        accum_term.intersection(&assignment.as_term());
                        *is_satisfied = accum_term.subset_of(incompat_term);
                        // Check if we have found the satisfier
                        // (all booleans in accum_satisfier are true).
                        if *is_satisfied
                            && accum_satisfier.iter().all(|(_, (satisfied, _))| *satisfied)
                        {
                            return Some((assignment, &all_assignments[0..idx]));
                        }
                    }
                }
            }
        }
        None
    }
}
