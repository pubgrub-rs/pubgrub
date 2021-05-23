// SPDX-License-Identifier: MPL-2.0

//! The partial solution is the current state
//! of the solution being built by the algorithm.

use crate::internal::arena::Arena;
use crate::internal::incompatibility::{IncompId, Incompatibility, Relation};
use crate::internal::memory::Memory;
use crate::package::Package;
use crate::range::Range;
use crate::term::Term;
use crate::type_aliases::{Map, SelectedDependencies};
use crate::version::Version;

use super::partial_solution_bis::Assignment::{self, Decision, Derivation};
use super::partial_solution_bis::DecisionLevel;

#[derive(Clone, Debug)]
pub struct DatedAssignment<P: Package, V: Version> {
    decision_level: DecisionLevel,
    assignment: Assignment<P, V>,
}

/// The partial solution is the current state
/// of the solution being built by the algorithm.
/// It is composed of a succession of assignments,
/// defined as either decisions or derivations.
#[derive(Clone, Debug)]
pub struct PartialSolution<P: Package, V: Version> {
    decision_level: DecisionLevel,
    /// Each assignment is stored with its decision level in the history.
    /// The order in which assignments where added in the vec is kept,
    /// so the oldest assignments are at the beginning of the vec.
    history: Vec<DatedAssignment<P, V>>,
    memory: Memory<P, V>,
}

impl<P: Package, V: Version> PartialSolution<P, V> {
    /// Initialize an empty partial solution.
    pub fn empty() -> Self {
        Self {
            decision_level: DecisionLevel(0),
            history: Vec::new(),
            memory: Memory::empty(),
        }
    }

    /// Add a decision to the partial solution.
    pub fn add_decision(&mut self, package: P, version: V) {
        self.decision_level = self.decision_level.increment();
        self.memory.add_decision(package.clone(), version.clone());
        self.history.push(DatedAssignment {
            decision_level: self.decision_level,
            assignment: Decision { package, version },
        });
    }

    /// Add a derivation to the partial solution.
    pub fn add_derivation(
        &mut self,
        package: P,
        cause: IncompId<P, V>,
        store: &Arena<Incompatibility<P, V>>,
    ) {
        self.memory.add_derivation(
            package.clone(),
            store[cause].get(&package).unwrap().negate(),
        );
        self.history.push(DatedAssignment {
            decision_level: self.decision_level,
            assignment: Derivation { package, cause },
        });
    }

    /// If a partial solution has, for every positive derivation,
    /// a corresponding decision that satisfies that assignment,
    /// it's a total solution and version solving has succeeded.
    pub fn extract_solution(&self) -> Option<SelectedDependencies<P, V>> {
        self.memory.extract_solution()
    }

    /// Compute, cache and retrieve the intersection of all terms for this package.
    pub fn term_intersection_for_package(&self, package: &P) -> Option<&Term<V>> {
        self.memory.term_intersection_for_package(package)
    }

    /// Backtrack the partial solution to a given decision level.
    pub fn backtrack(
        &mut self,
        decision_level: DecisionLevel,
        store: &Arena<Incompatibility<P, V>>,
    ) {
        let pos = self
            .history
            .binary_search_by(|probe| {
                probe
                    .decision_level
                    .cmp(&decision_level)
                    // `binary_search_by` does not guarantee which element to return when more
                    // then one match. By all ways claiming that it does not match we ensure we
                    // get the last one.
                    .then(std::cmp::Ordering::Less)
            })
            .unwrap_or_else(|x| x);

        self.history.truncate(pos);
        self.decision_level = self.history.last().expect("no history left").decision_level;
        self.memory.clear();
        let mem = &mut self.memory;
        self.history
            .iter()
            .for_each(|da| mem.add_assignment(&da.assignment, store));
    }

    /// Extract potential packages for the next iteration of unit propagation.
    /// Return `None` if there is no suitable package anymore, which stops the algorithm.
    pub fn potential_packages(&self) -> Option<impl Iterator<Item = (&P, &Range<V>)>> {
        let mut iter = self.memory.potential_packages().peekable();
        if iter.peek().is_some() {
            Some(iter)
        } else {
            None
        }
    }

    /// We can add the version to the partial solution as a decision
    /// if it doesn't produce any conflict with the new incompatibilities.
    /// In practice I think it can only produce a conflict if one of the dependencies
    /// (which are used to make the new incompatibilities)
    /// is already in the partial solution with an incompatible version.
    pub fn add_version(
        &mut self,
        package: P,
        version: V,
        new_incompatibilities: std::ops::Range<IncompId<P, V>>,
        store: &Arena<Incompatibility<P, V>>,
    ) {
        let exact = &Term::exact(version.clone());
        let not_satisfied = |incompat: &Incompatibility<P, V>| {
            incompat.relation(|p| {
                if p == &package {
                    Some(exact)
                } else {
                    self.memory.term_intersection_for_package(p)
                }
            }) != Relation::Satisfied
        };

        // Check none of the dependencies (new_incompatibilities)
        // would create a conflict (be satisfied).
        if store[new_incompatibilities].iter().all(not_satisfied) {
            self.add_decision(package, version);
        }
    }

    /// Check if the terms in the partial solution satisfy the incompatibility.
    pub fn relation(&self, incompat: &Incompatibility<P, V>) -> Relation<P> {
        incompat.relation(|package| self.memory.term_intersection_for_package(package))
    }

    /// Find satisfier and previous satisfier decision level.
    pub fn find_satisfier_and_previous_satisfier_level(
        &self,
        incompat: &Incompatibility<P, V>,
        store: &Arena<Incompatibility<P, V>>,
    ) -> (Assignment<P, V>, DecisionLevel, DecisionLevel) {
        debug_assert!(matches!(
            self.relation(incompat),
            crate::internal::incompatibility::Relation::Satisfied
        ));
        let satisfier_map = Self::find_satisfier(incompat, &self.history, store);
        assert_eq!(
            satisfier_map.len(),
            incompat.len(),
            "We should always find a satisfier if called in the right context."
        );
        let &satisfier_idx = satisfier_map.values().max().unwrap();
        let satisfier = &self.history[satisfier_idx];
        let previous_satisfier_level = Self::find_previous_satisfier(
            incompat,
            &satisfier.assignment,
            satisfier_map,
            &self.history[0..satisfier_idx],
            store,
        );
        (
            satisfier.assignment.clone(),
            satisfier.decision_level,
            previous_satisfier_level,
        )
    }

    /// A satisfier is the earliest assignment in partial solution such that the incompatibility
    /// is satisfied by the partial solution up to and including that assignment.
    ///
    /// Returns a map indicating for each package term, when that was first satisfied in history.
    /// If we effectively found a satisfier, the returned map must be the same size that incompat.
    fn find_satisfier<'a>(
        incompat: &Incompatibility<P, V>,
        history: &'a [DatedAssignment<P, V>],
        store: &Arena<Incompatibility<P, V>>,
    ) -> Map<P, usize> {
        let mut accum_satisfied: Map<P, Term<V>> = incompat
            .iter()
            .map(|(p, _)| (p.clone(), Term::any()))
            .collect();
        let mut satisfied = Map::with_capacity_and_hasher(incompat.len(), Default::default());
        for (idx, dated_assignment) in history.iter().enumerate() {
            let package = dated_assignment.assignment.package();
            if satisfied.contains_key(package) {
                continue; // If that package term is already satisfied, no need to check.
            }
            let incompat_term = match incompat.get(package) {
                // We only care about packages related to the incompatibility.
                None => continue,
                Some(i) => i,
            };
            let accum_term = match accum_satisfied.get_mut(package) {
                // We only care about packages related to the accum_satisfied.
                None => continue,
                Some(i) => i,
            };

            // Check if that incompat term is satisfied by our accumulated terms intersection.
            *accum_term = accum_term.intersection(&dated_assignment.assignment.as_term(store));
            // Check if we have found the satisfier
            // (that all terms are satisfied).
            if accum_term.subset_of(incompat_term) {
                satisfied.insert(package.clone(), idx);
                if satisfied.len() == incompat.len() {
                    break;
                }
            }
        }
        satisfied
    }

    /// Earliest assignment in the partial solution before satisfier
    /// such that incompatibility is satisfied by the partial solution up to
    /// and including that assignment plus satisfier.
    fn find_previous_satisfier<'a>(
        incompat: &Incompatibility<P, V>,
        satisfier: &Assignment<P, V>,
        mut satisfier_map: Map<P, usize>,
        previous_assignments: &'a [DatedAssignment<P, V>],
        store: &Arena<Incompatibility<P, V>>,
    ) -> DecisionLevel {
        let package = satisfier.package().clone();
        let mut accum_term = satisfier.as_term(store);
        let incompat_term = incompat.get(&package).expect("package not in satisfier");
        // Search previous satisfier.
        let mut there_is_no_previous_assignment = true;
        for (idx, dated_assignment) in previous_assignments.iter().enumerate() {
            if dated_assignment.assignment.package() != &package {
                // We only care about packages related to the incompatibility.
                continue;
            }
            there_is_no_previous_assignment = false;
            // Check if that incompat term is satisfied by our accumulated terms intersection.
            accum_term = accum_term.intersection(&dated_assignment.assignment.as_term(store));
            // Check if we have found the satisfier
            if accum_term.subset_of(incompat_term) {
                satisfier_map.insert(package.clone(), idx);
                break;
            }
        }
        // If there exists at least one assignment related to the incompatibility
        // prior to the satisfier, whether or not it is the same package than the satisfier,
        // we are guaranted to have a previous satisfier.
        // So the only way there could be no previous satisfier is if:
        //  - There is only one package (the satisfier one) in incompat
        //  - There is no derivation prior to the satisfier for that package
        if satisfier_map.len() == 1 && there_is_no_previous_assignment {
            DecisionLevel(1)
        } else if there_is_no_previous_assignment {
            let satisfier_idx = previous_assignments.len();
            let map_without_satisfier = satisfier_map.values().filter(|v| **v < satisfier_idx);
            previous_assignments[*map_without_satisfier.max().unwrap()]
                .decision_level
                .max(DecisionLevel(1))
        } else {
            previous_assignments[*satisfier_map.values().max().unwrap()]
                .decision_level
                .max(DecisionLevel(1))
        }
    }
}
