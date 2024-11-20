// SPDX-License-Identifier: MPL-2.0

//! A Memory acts like a structured partial solution where terms are regrouped by package in a [Map].

use std::fmt::{self, Debug, Display};
use std::hash::BuildHasherDefault;

use priority_queue::PriorityQueue;
use rustc_hash::FxHasher;
use smallvec::{smallvec, SmallVec};

use crate::{
    internal::{IncompatArena, IncompatId, Incompatibility, Relation, SmallMap},
    DependencyProvider, FxIndexSet, Map, PackageArena, PackageId, SelectedDependencies, Term,
    VersionIndex, VersionSet,
};

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[repr(transparent)]
pub(crate) struct DecisionLevel(u32);

impl DecisionLevel {
    pub(crate) const MAX: Self = Self(u32::MAX);

    fn get(self) -> u32 {
        self.0
    }

    fn increment(self) -> Self {
        Self(self.0 + 1)
    }
}

/// The partial solution contains all package assignments,
/// organized by package and historically ordered.
#[derive(Debug)]
pub(crate) struct PartialSolution<DP: DependencyProvider> {
    next_global_index: u32,
    current_decision_level: DecisionLevel,
    package_assignments: Vec<PackageAssignments>,
    package_assignments_indices: Vec<u32>,
    package_assignments_lengths: Vec<u32>,
    /// A package is a potential pick if there isn't an already selected version (no "decision")
    /// and if it contains at least one positive derivation term in the partial solution.
    potential_picks: FxIndexSet<PackageId>,
    /// `prioritized_potential_packages` is primarily a HashMap from a package with no decision and a positive assignment
    /// to its `Priority`. But, it also maintains a max heap of packages by `Priority` order.
    prioritized_potential_packages:
        PriorityQueue<PackageId, DP::Priority, BuildHasherDefault<FxHasher>>,
    last_valid_decision_levels: Vec<DecisionLevel>,
}

impl<DP: DependencyProvider> PartialSolution<DP> {
    pub(crate) fn display<'a>(
        &'a self,
        package_store: &'a PackageArena<DP::P>,
        dependency_provider: &'a DP,
    ) -> PartialSolutionDisplay<'a, DP> {
        PartialSolutionDisplay {
            partial_solution: self,
            package_store,
            dependency_provider,
        }
    }
}

pub(crate) struct PartialSolutionDisplay<'a, DP: DependencyProvider> {
    partial_solution: &'a PartialSolution<DP>,
    package_store: &'a PackageArena<DP::P>,
    dependency_provider: &'a DP,
}

impl<DP: DependencyProvider> Display for PartialSolutionDisplay<'_, DP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let partial_solution = self.partial_solution;

        writeln!(
            f,
            "Package assignments at next_global_index={}, {:?}:",
            partial_solution.next_global_index, partial_solution.current_decision_level,
        )?;

        for &pa_idx in &partial_solution.package_assignments_indices {
            let Some(pa) = partial_solution.package_assignments.get(pa_idx as usize) else {
                continue;
            };
            let pid = pa.package_id;
            let pn = self.package_store.pkg(pid).unwrap();
            let pa = pa.display(self.package_store, self.dependency_provider);
            write!(f, "- package `{pn}`:\n{pa}")?;
        }

        Ok(())
    }
}

/// Package assignments contain the potential decision and derivations
/// that have already been made for a given package,
/// as well as the intersection of terms by all of these.
#[derive(Debug)]
struct PackageAssignments {
    package_id: PackageId,
    highest_decision_level: DecisionLevel,
    dated_derivations: SmallVec<[DatedDerivation; 1]>,
    assignments_intersection: AssignmentsIntersection,
}

impl PackageAssignments {
    fn display<'a, DP: DependencyProvider>(
        &'a self,
        package_store: &'a PackageArena<DP::P>,
        dependency_provider: &'a DP,
    ) -> PackageAssignmentsDisplay<'a, DP> {
        PackageAssignmentsDisplay {
            package_assignment: self,
            package_store,
            dependency_provider,
        }
    }
}

struct PackageAssignmentsDisplay<'a, DP: DependencyProvider> {
    package_assignment: &'a PackageAssignments,
    package_store: &'a PackageArena<DP::P>,
    dependency_provider: &'a DP,
}

impl<DP: DependencyProvider> Display for PackageAssignmentsDisplay<'_, DP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let PackageAssignments {
            package_id,
            highest_decision_level,
            dated_derivations,
            assignments_intersection,
        } = self.package_assignment;

        let package = self.package_store.pkg(*package_id).unwrap();

        writeln!(f, "dated derivations at {highest_decision_level:?}:")?;
        for dd in dated_derivations {
            writeln!(f, "  {}", dd.display(package, self.dependency_provider))?;
        }
        writeln!(
            f,
            "assignments_intersection: {}",
            assignments_intersection.display(package, self.dependency_provider)
        )?;

        Ok(())
    }
}

#[derive(Clone, Debug)]
struct DatedDerivation {
    global_index: u32,
    decision_level: DecisionLevel,
    cause: IncompatId,
    accumulated_intersection: Term,
}

impl DatedDerivation {
    fn display<'a, DP: DependencyProvider>(
        &'a self,
        package: &'a DP::P,
        dependency_provider: &'a DP,
    ) -> DatedDerivationDisplay<'a, DP> {
        DatedDerivationDisplay {
            dated_derivation: self,
            package,
            dependency_provider,
        }
    }
}

struct DatedDerivationDisplay<'a, DP: DependencyProvider> {
    dated_derivation: &'a DatedDerivation,
    package: &'a DP::P,
    dependency_provider: &'a DP,
}

impl<DP: DependencyProvider> Display for DatedDerivationDisplay<'_, DP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let DatedDerivation {
            global_index,
            decision_level,
            cause: _,
            accumulated_intersection,
        } = self.dated_derivation;

        write!(f, "global_index: {global_index}, ")?;
        write!(f, "{decision_level:?}, ")?;

        let pvs = self
            .dependency_provider
            .package_version_set_display(self.package, accumulated_intersection.version_set());

        if accumulated_intersection.is_positive() {
            write!(f, "accumulated_intersection: {pvs}")?;
        } else {
            write!(f, "accumulated_intersection: Not ( {pvs} )")?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
struct AssignmentsIntersection {
    term: Term,
    is_decision: bool,
    decision_global_index: u32,
    decision_version_index: VersionIndex,
}

impl AssignmentsIntersection {
    fn decision(global_index: u32, version_index: VersionIndex) -> Self {
        Self {
            term: Term::exact(version_index),
            is_decision: true,
            decision_global_index: global_index,
            decision_version_index: version_index,
        }
    }

    fn derivations(term: Term) -> Self {
        Self {
            term,
            ..Default::default()
        }
    }

    fn display<'a, DP: DependencyProvider>(
        &'a self,
        package: &'a DP::P,
        dependency_provider: &'a DP,
    ) -> AssignmentsIntersectionDisplay<'a, DP> {
        AssignmentsIntersectionDisplay {
            assignments_intersection: self,
            package,
            dependency_provider,
        }
    }
}

struct AssignmentsIntersectionDisplay<'a, DP: DependencyProvider> {
    assignments_intersection: &'a AssignmentsIntersection,
    package: &'a DP::P,
    dependency_provider: &'a DP,
}

impl<DP: DependencyProvider> Display for AssignmentsIntersectionDisplay<'_, DP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let AssignmentsIntersection {
            term,
            is_decision,
            decision_global_index,
            decision_version_index,
        } = *self.assignments_intersection;

        if is_decision {
            write!(
                f,
                "Decision at global_index={decision_global_index}: {}",
                self.dependency_provider
                    .package_version_display(self.package, decision_version_index)
            )?;
        } else {
            let pvs = self
                .dependency_provider
                .package_version_set_display(self.package, term.version_set());

            if term.is_positive() {
                write!(f, "Derivation:: {pvs}")?;
            } else {
                write!(f, "Derivation:: Not ( {pvs} )")?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) enum SatisfierSearch {
    DifferentDecisionLevels {
        previous_satisfier_level: DecisionLevel,
    },
    SameDecisionLevels {
        satisfier_cause: IncompatId,
    },
}

type SatisfiedMap = SmallMap<PackageId, (Option<IncompatId>, u32, DecisionLevel)>;

impl<DP: DependencyProvider> PartialSolution<DP> {
    /// Initialize an empty `PartialSolution`.
    pub(crate) fn empty(root_package: PackageId) -> Self {
        Self {
            next_global_index: 0,
            current_decision_level: DecisionLevel(0),
            potential_picks: FxIndexSet::default(),
            package_assignments: Vec::new(),
            package_assignments_indices: vec![u32::MAX; root_package.get() as usize + 1],
            package_assignments_lengths: Vec::new(),
            prioritized_potential_packages: PriorityQueue::default(),
            last_valid_decision_levels: vec![DecisionLevel(0)],
        }
    }

    /// Check if an incompatibility is contradicted.
    pub(crate) fn is_contradicted(&self, incompat: &Incompatibility<DP::M>) -> bool {
        incompat.is_contradicted(&self.last_valid_decision_levels)
    }

    /// Contradict an incompatibility.
    pub(crate) fn contradict(&self, incompat: &mut Incompatibility<DP::M>) {
        incompat.set_contradication_info(
            self.current_decision_level,
            self.last_valid_decision_levels.len() as u32,
        );
    }

    /// Add a decision.
    pub(crate) fn add_decision(&mut self, package_id: PackageId, version_index: VersionIndex) {
        let len = self.package_assignments.len();
        let pa = &mut self
            .package_assignments
            .get_mut(self.package_assignments_indices[package_id.get() as usize] as usize)
            .expect("Derivations must already exist");

        // Check that add_decision is never used in the wrong context.
        if cfg!(debug_assertions) {
            let pai = &pa.assignments_intersection;
            // Cannot be called when a decision has already been taken.
            assert!(!pai.is_decision, "Already existing decision");
            // Cannot be called if the versions is not contained in the terms' intersection.
            assert!(
                pai.term.contains(version_index),
                "{} was expected to be contained in {}",
                version_index.get(),
                pai.term,
            );
            assert!(self.potential_picks.is_empty());
        }

        self.package_assignments_lengths.push(len as u32);
        self.current_decision_level = self.current_decision_level.increment();
        pa.highest_decision_level = self.current_decision_level;
        pa.assignments_intersection =
            AssignmentsIntersection::decision(self.next_global_index, version_index);
        self.next_global_index += 1;
    }

    /// Add a derivation.
    pub(crate) fn add_derivation(
        &mut self,
        package_id: PackageId,
        cause: IncompatId,
        incompat_term: Term,
    ) {
        let mut dated_derivation = DatedDerivation {
            global_index: self.next_global_index,
            decision_level: self.current_decision_level,
            cause,
            accumulated_intersection: incompat_term.negate(),
        };
        self.next_global_index += 1;

        self.resize_package_assignments_indices(package_id);
        let pa_idx = &mut self.package_assignments_indices[package_id.get() as usize];

        if let Some(pa) = self.package_assignments.get_mut(*pa_idx as usize) {
            pa.highest_decision_level = self.current_decision_level;
            // Check that add_derivation is never called in the wrong context.
            assert!(
                !pa.assignments_intersection.is_decision,
                "add_derivation should not be called after a decision",
            );
            let term = &mut pa.assignments_intersection.term;
            *term = term.intersection(dated_derivation.accumulated_intersection);
            dated_derivation.accumulated_intersection = *term;
            pa.dated_derivations.push(dated_derivation);
            if term.is_positive() {
                self.potential_picks.insert(package_id);
            }
        } else {
            let term = dated_derivation.accumulated_intersection;
            *pa_idx = self.package_assignments.len() as u32;
            self.package_assignments.push(PackageAssignments {
                package_id,
                highest_decision_level: self.current_decision_level,
                dated_derivations: smallvec![dated_derivation],
                assignments_intersection: AssignmentsIntersection::derivations(term),
            });
            if term.is_positive() {
                self.potential_picks.insert(package_id);
            }
        }
    }

    #[cold]
    pub(crate) fn pick_highest_priority_pkg(
        &mut self,
        mut prioritizer: impl FnMut(PackageId, VersionSet) -> DP::Priority,
    ) -> Option<PackageId> {
        self.prioritized_potential_packages
            .extend(self.potential_picks.iter().map(|&pid| {
                let vs = self
                    .package_assignments
                    .get(self.package_assignments_indices[pid.get() as usize] as usize)
                    .expect("potential picks should have valid assignments")
                    .assignments_intersection
                    .term
                    .version_set();

                (pid, prioritizer(pid, vs))
            }));
        self.potential_picks.clear();
        self.prioritized_potential_packages
            .pop()
            .map(|(pid, _)| pid)
    }

    /// If a partial solution has, for every positive derivation,
    /// a corresponding decision that satisfies that assignment,
    /// it's a total solution and version solving has succeeded.
    pub(crate) fn extract_solution(
        &self,
        package_store: PackageArena<DP::P>,
    ) -> SelectedDependencies<DP> {
        let used = self
            .package_assignments_indices
            .iter()
            .filter_map(|&pa_idx| {
                let pa = self.package_assignments.get(pa_idx as usize)?;
                if !pa.assignments_intersection.is_decision {
                    return None;
                }
                let version_index = pa.assignments_intersection.decision_version_index;
                Some((pa.package_id, version_index))
            })
            .collect::<Map<_, _>>();

        package_store
            .into_iter()
            .enumerate()
            .filter_map(|(i, p)| used.get(&PackageId(i as u32)).map(|&v| (p, v)))
            .collect()
    }

    /// Backtrack the partial solution to a given decision level.
    pub(crate) fn backtrack(&mut self, decision_level: DecisionLevel) {
        self.current_decision_level = decision_level;
        self.potential_picks.clear();

        let max_len = self.package_assignments_lengths[decision_level.get() as usize] as usize;

        for pa in &self.package_assignments[max_len..] {
            self.package_assignments_indices[pa.package_id.get() as usize] = u32::MAX;
        }

        self.package_assignments_lengths
            .truncate(decision_level.get() as usize);

        self.package_assignments.truncate(max_len);

        for pa in &mut self.package_assignments {
            if pa.highest_decision_level > decision_level {
                // Since decision_level < highest_decision_level,
                // we can be certain that there will be no decision in this package assignments
                // after backtracking, because such decision would have been the last
                // assignment and it would have the "highest_decision_level".

                // Truncate the history.
                let last_idx = pa
                    .dated_derivations
                    .partition_point(|dd| dd.decision_level <= decision_level);

                pa.dated_derivations.truncate(last_idx);

                let last = pa.dated_derivations.last().unwrap();

                // Update highest_decision_level.
                pa.highest_decision_level = last.decision_level;

                // Reset the assignments intersection.
                pa.assignments_intersection =
                    AssignmentsIntersection::derivations(last.accumulated_intersection);
            }

            let pai = &pa.assignments_intersection;
            if !pai.is_decision && pai.term.is_positive() {
                self.potential_picks.insert(pa.package_id);
            }
        }

        // Throw away all stored priority levels, And mark that they all need to be recomputed.
        self.prioritized_potential_packages.clear();

        // Update list of last valid contradicted decision levels
        self.last_valid_decision_levels
            .push(DecisionLevel(u32::MAX));

        let index = self
            .last_valid_decision_levels
            .partition_point(|&l| l <= decision_level);

        self.last_valid_decision_levels[index..].fill(decision_level);
    }

    /// We can add the version to the partial solution as a decision
    /// if it doesn't produce any conflict with the new incompatibilities.
    /// In practice I think it can only produce a conflict if one of the dependencies
    /// (which are used to make the new incompatibilities)
    /// is already in the partial solution with an incompatible version.
    pub(crate) fn add_version(
        &mut self,
        package_id: PackageId,
        version_index: VersionIndex,
        new_incompatibilities: std::ops::Range<IncompatId>,
        store: &IncompatArena<DP::M>,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) {
        let max_idx = store[new_incompatibilities.clone()]
            .iter()
            .flat_map(|incompat| incompat.iter().map(|(p, _)| p.0))
            .max()
            .unwrap_or(0);

        self.resize_package_assignments_indices(PackageId(max_idx));

        if self.last_valid_decision_levels.len() == 1 {
            // Nothing has yet gone wrong during this resolution. This call is unlikely to be the first problem.
            // So let's live with a little bit of risk and add the decision without checking the dependencies.
            // The worst that can happen is we will have to do a full backtrack which only removes this one decision.
            log::info!(
                "add_decision: {} without checking dependencies",
                dependency_provider
                    .package_version_display(package_store.pkg(package_id).unwrap(), version_index)
            );
            self.add_decision(package_id, version_index);
        } else {
            // Check if any of the new dependencies preclude deciding on this crate version.
            let exact = Term::exact(version_index);
            let not_satisfied = |incompat: &Incompatibility<DP::M>| {
                incompat.relation(|pid| {
                    if pid == package_id {
                        Some(exact)
                    } else {
                        self.term_intersection_for_package(pid)
                    }
                }) != Relation::Satisfied
            };

            // Check none of the dependencies (new_incompatibilities)
            // would create a conflict (be satisfied).
            if store[new_incompatibilities].iter().all(not_satisfied) {
                log::info!(
                    "add_decision: {}",
                    dependency_provider.package_version_display(
                        package_store.pkg(package_id).unwrap(),
                        version_index
                    )
                );
                self.add_decision(package_id, version_index);
            } else {
                log::info!(
                    "not adding {} because of its dependencies",
                    dependency_provider.package_version_display(
                        package_store.pkg(package_id).unwrap(),
                        version_index
                    )
                );
            }
        }
    }

    fn resize_package_assignments_indices(&mut self, package: PackageId) {
        let idx = package.get() as usize;
        if idx + 1 > self.package_assignments_indices.len() {
            self.package_assignments_indices.resize(idx + 1, u32::MAX);
        }
    }

    /// Check if the terms in the partial solution satisfy the incompatibility.
    pub(crate) fn relation(&self, incompat: &Incompatibility<DP::M>) -> Relation {
        incompat.relation(|package_id| self.term_intersection_for_package(package_id))
    }

    /// Retrieve intersection of terms related to package.
    pub(crate) fn term_intersection_for_package(&self, package_id: PackageId) -> Option<Term> {
        self.package_assignments
            .get(self.package_assignments_indices[package_id.get() as usize] as usize)
            .map(|pa| pa.assignments_intersection.term)
    }

    /// Figure out if the satisfier and previous satisfier are of different decision levels.
    pub(crate) fn satisfier_search(
        &self,
        incompat: &Incompatibility<DP::M>,
        store: &IncompatArena<DP::M>,
        package_store: &PackageArena<DP::P>,
    ) -> (PackageId, SatisfierSearch) {
        let satisfied_map = self.find_satisfier(incompat, package_store);
        let (&satisfier_pid, &(satisfier_cause, _, satisfier_decision_level)) = satisfied_map
            .iter()
            .max_by_key(|(_, (_, global_index, _))| global_index)
            .unwrap();
        let previous_satisfier_level = self.find_previous_satisfier(
            incompat,
            satisfier_pid,
            satisfied_map,
            store,
            package_store,
        );
        let search_result = if previous_satisfier_level >= satisfier_decision_level {
            SatisfierSearch::SameDecisionLevels {
                satisfier_cause: satisfier_cause.unwrap(),
            }
        } else {
            SatisfierSearch::DifferentDecisionLevels {
                previous_satisfier_level,
            }
        };
        (satisfier_pid, search_result)
    }

    /// A satisfier is the earliest assignment in partial solution such that the incompatibility
    /// is satisfied by the partial solution up to and including that assignment.
    ///
    /// Returns a map indicating for each package term, when that was first satisfied in history.
    /// If we effectively found a satisfier, the returned map must be the same size that incompat.
    ///
    /// Question: This is possible since we added a "global_index" to every dated_derivation.
    /// It would be nice if we could get rid of it, but I don't know if then it will be possible
    /// to return a coherent previous_satisfier_level.
    fn find_satisfier(
        &self,
        incompat: &Incompatibility<DP::M>,
        package_store: &PackageArena<DP::P>,
    ) -> SatisfiedMap {
        let mut satisfied = SmallMap::Empty;
        for (package_id, incompat_term) in incompat.iter() {
            let satisfied_info = self
                .package_assignments
                .get(self.package_assignments_indices[package_id.get() as usize] as usize)
                .unwrap()
                .satisfier::<DP>(package_id, incompat_term.negate(), package_store);

            satisfied.insert(package_id, satisfied_info);
        }
        satisfied
    }

    /// Earliest assignment in the partial solution before satisfier
    /// such that incompatibility is satisfied by the partial solution up to
    /// and including that assignment plus satisfier.
    fn find_previous_satisfier(
        &self,
        incompat: &Incompatibility<DP::M>,
        satisfier_pid: PackageId,
        mut satisfied_map: SatisfiedMap,
        store: &IncompatArena<DP::M>,
        package_store: &PackageArena<DP::P>,
    ) -> DecisionLevel {
        // First, let's retrieve the previous derivations and the initial accum_term.
        let satisfier_pa = self
            .package_assignments
            .get(self.package_assignments_indices[satisfier_pid.get() as usize] as usize)
            .unwrap();

        let satisfier_cause = satisfied_map.get(&satisfier_pid).unwrap().0;

        let accum_term = if let Some(cause) = satisfier_cause {
            store[cause].get(satisfier_pid).unwrap().negate()
        } else {
            assert!(
                satisfier_pa.assignments_intersection.is_decision,
                "must be a decision",
            );
            satisfier_pa.assignments_intersection.term
        };

        let incompat_term = incompat
            .get(satisfier_pid)
            .expect("satisfier package not in incompat");

        satisfied_map.insert(
            satisfier_pid,
            satisfier_pa.satisfier::<DP>(
                satisfier_pid,
                accum_term.intersection(incompat_term.negate()),
                package_store,
            ),
        );

        // Finally, let's identify the decision level of that previous satisfier.
        let (_, &(_, _, decision_level)) = satisfied_map
            .iter()
            .max_by_key(|(_p, (_, global_index, _))| global_index)
            .unwrap();
        decision_level.max(DecisionLevel(1))
    }
}

impl PackageAssignments {
    fn satisfier<DP: DependencyProvider>(
        &self,
        package_id: PackageId,
        start_term: Term,
        package_store: &PackageArena<DP::P>,
    ) -> (Option<IncompatId>, u32, DecisionLevel) {
        // Indicate if we found a satisfier in the list of derivations, otherwise it will be the decision.
        let idx = self
            .dated_derivations
            .partition_point(|dd| !dd.accumulated_intersection.is_disjoint(start_term));
        if let Some(dd) = self.dated_derivations.get(idx) {
            debug_assert_eq!(
                dd.accumulated_intersection.intersection(start_term),
                Term::empty(),
            );
            return (Some(dd.cause), dd.global_index, dd.decision_level);
        }
        // If it wasn't found in the derivations, it must be the decision which is last (if called in the right context).
        let AssignmentsIntersection {
            term,
            is_decision,
            decision_global_index,
            ..
        } = self.assignments_intersection;
        if !is_decision {
            let p = package_store.pkg(package_id).unwrap();
            unreachable!(
                "while processing package {p}: \
                accum_term = {term} has overlap with incompat_term = {start_term}, \
                which means the last assignment should have been a decision, \
                but instead it was a derivation. This shouldn't be possible! \
                (Maybe your Version ordering is broken?)"
            )
        }
        (None, decision_global_index, self.highest_decision_level)
    }
}
