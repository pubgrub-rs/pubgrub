// SPDX-License-Identifier: MPL-2.0

//! A Memory acts like a structured partial solution
//! where terms are regrouped by package in a [Map](crate::type_aliases::Map).

use std::fmt::{Debug, Display};
use std::hash::BuildHasherDefault;

use priority_queue::PriorityQueue;
use rustc_hash::FxHasher;

use crate::internal::{
    Arena, HashArena, Id, IncompDpId, IncompId, Incompatibility, Relation, SmallMap, SmallVec,
};
use crate::{DependencyProvider, Package, Term, VersionSet};

type FnvIndexMap<K, V> = indexmap::IndexMap<K, V, BuildHasherDefault<FxHasher>>;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) struct DecisionLevel(pub(crate) u32);

impl DecisionLevel {
    pub(crate) fn increment(self) -> Self {
        Self(self.0 + 1)
    }
}

/// The partial solution contains all package assignments,
/// organized by package and historically ordered.
#[derive(Clone, Debug)]
pub(crate) struct PartialSolution<DP: DependencyProvider> {
    next_global_index: u32,
    current_decision_level: DecisionLevel,
    /// `package_assignments` is primarily a HashMap from a package to its
    /// `PackageAssignments`. But it can also keep the items in an order.
    ///  We maintain three sections in this order:
    /// 1. `[..current_decision_level]` Are packages that have had a decision made sorted by the `decision_level`.
    ///    This makes it very efficient to extract the solution, And to backtrack to a particular decision level.
    /// 2. `[current_decision_level..changed_this_decision_level]` Are packages that have **not** had there assignments
    ///    changed since the last time `prioritize` has been called. Within this range there is no sorting.
    /// 3. `[changed_this_decision_level..]` Contains all packages that **have** had there assignments changed since
    ///    the last time `prioritize` has been called. The inverse is not necessarily true, some packages in the range
    ///    did not have a change. Within this range there is no sorting.
    #[allow(clippy::type_complexity)]
    package_assignments: FnvIndexMap<Id<DP::P>, PackageAssignments<DP::P, DP::VS, DP::M>>,
    /// `prioritized_potential_packages` is primarily a HashMap from a package with no desition and a positive assignment
    /// to its `Priority`. But, it also maintains a max heap of packages by `Priority` order.
    prioritized_potential_packages:
        PriorityQueue<Id<DP::P>, DP::Priority, BuildHasherDefault<FxHasher>>,
    changed_this_decision_level: usize,
    has_ever_backtracked: bool,
}

impl<DP: DependencyProvider> PartialSolution<DP> {
    pub fn display<'a>(&'a self, package_store: &'a HashArena<DP::P>) -> impl Display + 'a {
        struct PSDisplay<'a, DP: DependencyProvider>(&'a PartialSolution<DP>, &'a HashArena<DP::P>);

        impl<DP: DependencyProvider> Display for PSDisplay<'_, DP> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut assignments: Vec<_> = self
                    .0
                    .package_assignments
                    .iter()
                    .map(|(p, pa)| format!("{:?} = '{}': {}", p, self.1[*p], pa))
                    .collect();
                assignments.sort();
                write!(
                    f,
                    "next_global_index: {}\ncurrent_decision_level: {:?}\npackage_assignments:\n{}",
                    self.0.next_global_index,
                    self.0.current_decision_level,
                    assignments.join("\t\n")
                )
            }
        }

        PSDisplay(self, package_store)
    }
}

/// Package assignments contain the potential decision and derivations
/// that have already been made for a given package,
/// as well as the intersection of terms by all of these.
#[derive(Clone, Debug)]
struct PackageAssignments<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> {
    smallest_decision_level: DecisionLevel,
    highest_decision_level: DecisionLevel,
    dated_derivations: SmallVec<DatedDerivation<P, VS, M>>,
    assignments_intersection: AssignmentsIntersection<VS>,
}

impl<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> Display
    for PackageAssignments<P, VS, M>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let derivations: Vec<_> = self
            .dated_derivations
            .iter()
            .map(|dd| dd.to_string())
            .collect();
        write!(
            f,
            "decision range: {:?}..{:?}\nderivations:\n  {}\n,assignments_intersection: {}",
            self.smallest_decision_level,
            self.highest_decision_level,
            derivations.join("\n  "),
            self.assignments_intersection
        )
    }
}

#[derive(Clone, Debug)]
struct DatedDerivation<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> {
    global_index: u32,
    decision_level: DecisionLevel,
    cause: IncompId<P, VS, M>,
    accumulated_intersection: Term<VS>,
}

impl<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> Display
    for DatedDerivation<P, VS, M>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}, cause: {:?}", self.decision_level, self.cause)
    }
}

#[derive(Clone, Debug)]
enum AssignmentsIntersection<VS: VersionSet> {
    Decision((u32, VS::V, Term<VS>)),
    Derivations(Term<VS>),
}

impl<VS: VersionSet> Display for AssignmentsIntersection<VS> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Decision((lvl, version, _)) => {
                write!(f, "Decision: level {}, v = {}", lvl, version)
            }
            Self::Derivations(term) => write!(f, "Derivations term: {}", term),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum SatisfierSearch<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> {
    DifferentDecisionLevels {
        previous_satisfier_level: DecisionLevel,
    },
    SameDecisionLevels {
        satisfier_cause: IncompId<P, VS, M>,
    },
}

type SatisfiedMap<P, VS, M> = SmallMap<Id<P>, (Option<IncompId<P, VS, M>>, u32, DecisionLevel)>;

impl<DP: DependencyProvider> PartialSolution<DP> {
    /// Initialize an empty PartialSolution.
    pub(crate) fn empty() -> Self {
        Self {
            next_global_index: 0,
            current_decision_level: DecisionLevel(0),
            package_assignments: FnvIndexMap::default(),
            prioritized_potential_packages: PriorityQueue::default(),
            changed_this_decision_level: 0,
            has_ever_backtracked: false,
        }
    }

    /// Add a decision.
    pub(crate) fn add_decision(&mut self, package: Id<DP::P>, version: DP::V) {
        // Check that add_decision is never used in the wrong context.
        if cfg!(debug_assertions) {
            match self.package_assignments.get_mut(&package) {
                None => panic!("Derivations must already exist"),
                Some(pa) => match &pa.assignments_intersection {
                    // Cannot be called when a decision has already been taken.
                    AssignmentsIntersection::Decision(_) => panic!("Already existing decision"),
                    // Cannot be called if the versions is not contained in the terms' intersection.
                    AssignmentsIntersection::Derivations(term) => {
                        debug_assert!(
                            term.contains(&version),
                            "{:?}: {} was expected to be contained in {}",
                            package,
                            version,
                            term,
                        )
                    }
                },
            }
            assert_eq!(
                self.changed_this_decision_level,
                self.package_assignments.len()
            );
        }
        let new_idx = self.current_decision_level.0 as usize;
        self.current_decision_level = self.current_decision_level.increment();
        let (old_idx, _, pa) = self
            .package_assignments
            .get_full_mut(&package)
            .expect("Derivations must already exist");
        pa.highest_decision_level = self.current_decision_level;
        pa.assignments_intersection = AssignmentsIntersection::Decision((
            self.next_global_index,
            version.clone(),
            Term::exact(version),
        ));
        // Maintain that the beginning of the `package_assignments` Have all decisions in sorted order.
        if new_idx != old_idx {
            self.package_assignments.swap_indices(new_idx, old_idx);
        }
        self.next_global_index += 1;
    }

    /// Add a derivation.
    pub(crate) fn add_derivation(
        &mut self,
        package: Id<DP::P>,
        cause: IncompDpId<DP>,
        store: &Arena<Incompatibility<DP::P, DP::VS, DP::M>>,
    ) {
        use indexmap::map::Entry;
        let mut dated_derivation = DatedDerivation {
            global_index: self.next_global_index,
            decision_level: self.current_decision_level,
            cause,
            accumulated_intersection: store[cause].get(package).unwrap().negate(),
        };
        self.next_global_index += 1;
        let pa_last_index = self.package_assignments.len().saturating_sub(1);
        match self.package_assignments.entry(package) {
            Entry::Occupied(mut occupied) => {
                let idx = occupied.index();
                let pa = occupied.get_mut();
                pa.highest_decision_level = self.current_decision_level;
                match &mut pa.assignments_intersection {
                    // Check that add_derivation is never called in the wrong context.
                    AssignmentsIntersection::Decision(_) => {
                        panic!("add_derivation should not be called after a decision")
                    }
                    AssignmentsIntersection::Derivations(t) => {
                        *t = t.intersection(&dated_derivation.accumulated_intersection);
                        dated_derivation.accumulated_intersection = t.clone();
                        if t.is_positive() {
                            // we can use `swap_indices` to make `changed_this_decision_level` only go down by 1
                            // but the copying is slower then the larger search
                            self.changed_this_decision_level =
                                std::cmp::min(self.changed_this_decision_level, idx);
                        }
                    }
                }
                pa.dated_derivations.push(dated_derivation);
            }
            Entry::Vacant(v) => {
                let term = dated_derivation.accumulated_intersection.clone();
                if term.is_positive() {
                    self.changed_this_decision_level =
                        std::cmp::min(self.changed_this_decision_level, pa_last_index);
                }
                v.insert(PackageAssignments {
                    smallest_decision_level: self.current_decision_level,
                    highest_decision_level: self.current_decision_level,
                    dated_derivations: SmallVec::One([dated_derivation]),
                    assignments_intersection: AssignmentsIntersection::Derivations(term),
                });
            }
        }
    }

    #[cold]
    pub(crate) fn pick_highest_priority_pkg(
        &mut self,
        prioritizer: impl Fn(Id<DP::P>, &DP::VS) -> DP::Priority,
    ) -> Option<Id<DP::P>> {
        let check_all = self.changed_this_decision_level
            == self.current_decision_level.0.saturating_sub(1) as usize;
        let current_decision_level = self.current_decision_level;
        let prioritized_potential_packages = &mut self.prioritized_potential_packages;
        self.package_assignments
            .get_range(self.changed_this_decision_level..)
            .unwrap()
            .iter()
            .filter(|(_, pa)| {
                // We only actually need to update the package if it has been changed
                // since the last time we called prioritize.
                // Which means it's highest decision level is the current decision level,
                // or if we backtracked in the meantime.
                check_all || pa.highest_decision_level == current_decision_level
            })
            .filter_map(|(&p, pa)| pa.assignments_intersection.potential_package_filter(p))
            .for_each(|(p, r)| {
                let priority = prioritizer(p, r);
                prioritized_potential_packages.push(p, priority);
            });
        self.changed_this_decision_level = self.package_assignments.len();
        prioritized_potential_packages.pop().map(|(p, _)| p)
    }

    /// If a partial solution has, for every positive derivation,
    /// a corresponding decision that satisfies that assignment,
    /// it's a total solution and version solving has succeeded.
    pub(crate) fn extract_solution(&self) -> impl Iterator<Item = (Id<DP::P>, DP::V)> + '_ {
        self.package_assignments
            .iter()
            .take(self.current_decision_level.0 as usize)
            .map(|(&p, pa)| match &pa.assignments_intersection {
                AssignmentsIntersection::Decision((_, v, _)) => (p, v.clone()),
                AssignmentsIntersection::Derivations(_) => {
                    panic!("Derivations in the Decision part")
                }
            })
    }

    /// Backtrack the partial solution to a given decision level.
    pub(crate) fn backtrack(&mut self, decision_level: DecisionLevel) {
        self.current_decision_level = decision_level;
        self.package_assignments.retain(|_, pa| {
            if pa.smallest_decision_level > decision_level {
                // Remove all entries that have a smallest decision level higher than the backtrack target.
                false
            } else if pa.highest_decision_level <= decision_level {
                // Do not change entries older than the backtrack decision level target.
                true
            } else {
                // smallest_decision_level <= decision_level < highest_decision_level
                //
                // Since decision_level < highest_decision_level,
                // We can be certain that there will be no decision in this package assignments
                // after backtracking, because such decision would have been the last
                // assignment and it would have the "highest_decision_level".

                // Truncate the history.
                while pa.dated_derivations.last().map(|dd| dd.decision_level) > Some(decision_level)
                {
                    pa.dated_derivations.pop();
                }
                debug_assert!(!pa.dated_derivations.is_empty());

                let last = pa.dated_derivations.last().unwrap();

                // Update highest_decision_level.
                pa.highest_decision_level = last.decision_level;

                // Reset the assignments intersection.
                pa.assignments_intersection =
                    AssignmentsIntersection::Derivations(last.accumulated_intersection.clone());
                true
            }
        });
        // Throw away all stored priority levels, And mark that they all need to be recomputed.
        self.prioritized_potential_packages.clear();
        self.changed_this_decision_level = self.current_decision_level.0.saturating_sub(1) as usize;
        self.has_ever_backtracked = true;
    }

    /// We can add the version to the partial solution as a decision
    /// if it doesn't produce any conflict with the new incompatibilities.
    /// In practice I think it can only produce a conflict if one of the dependencies
    /// (which are used to make the new incompatibilities)
    /// is already in the partial solution with an incompatible version.
    pub(crate) fn add_version(
        &mut self,
        package: Id<DP::P>,
        version: DP::V,
        new_incompatibilities: std::ops::Range<IncompId<DP::P, DP::VS, DP::M>>,
        store: &Arena<Incompatibility<DP::P, DP::VS, DP::M>>,
    ) {
        if !self.has_ever_backtracked {
            // Nothing has yet gone wrong during this resolution. This call is unlikely to be the first problem.
            // So let's live with a little bit of risk and add the decision without checking the dependencies.
            // The worst that can happen is we will have to do a full backtrack which only removes this one decision.
            log::info!("add_decision: {package:?} @ {version} without checking dependencies");
            self.add_decision(package, version);
        } else {
            // Check if any of the new dependencies preclude deciding on this crate version.
            let exact = Term::exact(version.clone());
            let not_satisfied = |incompat: &Incompatibility<DP::P, DP::VS, DP::M>| {
                incompat.relation(|p| {
                    if p == package {
                        Some(&exact)
                    } else {
                        self.term_intersection_for_package(p)
                    }
                }) != Relation::Satisfied
            };

            // Check none of the dependencies (new_incompatibilities)
            // would create a conflict (be satisfied).
            if store[new_incompatibilities].iter().all(not_satisfied) {
                log::info!("add_decision: {package:?} @ {version}");
                self.add_decision(package, version);
            } else {
                log::info!("not adding {package:?} @ {version} because of its dependencies",);
            }
        }
    }

    /// Check if the terms in the partial solution satisfy the incompatibility.
    pub(crate) fn relation(
        &self,
        incompat: &Incompatibility<DP::P, DP::VS, DP::M>,
    ) -> Relation<DP::P> {
        incompat.relation(|package| self.term_intersection_for_package(package))
    }

    /// Retrieve intersection of terms related to package.
    pub(crate) fn term_intersection_for_package(
        &self,
        package: Id<DP::P>,
    ) -> Option<&Term<DP::VS>> {
        self.package_assignments
            .get(&package)
            .map(|pa| pa.assignments_intersection.term())
    }

    /// Figure out if the satisfier and previous satisfier are of different decision levels.
    #[allow(clippy::type_complexity)]
    pub(crate) fn satisfier_search(
        &self,
        incompat: &Incompatibility<DP::P, DP::VS, DP::M>,
        store: &Arena<Incompatibility<DP::P, DP::VS, DP::M>>,
    ) -> (Id<DP::P>, SatisfierSearch<DP::P, DP::VS, DP::M>) {
        let satisfied_map = Self::find_satisfier(incompat, &self.package_assignments);
        let (&satisfier_package, &(satisfier_cause, _, satisfier_decision_level)) = satisfied_map
            .iter()
            .max_by_key(|(_p, (_, global_index, _))| global_index)
            .unwrap();
        let previous_satisfier_level = Self::find_previous_satisfier(
            incompat,
            satisfier_package,
            satisfied_map,
            &self.package_assignments,
            store,
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
        (satisfier_package, search_result)
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
    #[allow(clippy::type_complexity)]
    fn find_satisfier(
        incompat: &Incompatibility<DP::P, DP::VS, DP::M>,
        package_assignments: &FnvIndexMap<Id<DP::P>, PackageAssignments<DP::P, DP::VS, DP::M>>,
    ) -> SatisfiedMap<DP::P, DP::VS, DP::M> {
        let mut satisfied = SmallMap::Empty;
        for (&package, incompat_term) in incompat.iter() {
            let pa = package_assignments.get(&package).expect("Must exist");
            satisfied.insert(package, pa.satisfier(package, &incompat_term.negate()));
        }
        satisfied
    }

    /// Earliest assignment in the partial solution before satisfier
    /// such that incompatibility is satisfied by the partial solution up to
    /// and including that assignment plus satisfier.
    #[allow(clippy::type_complexity)]
    fn find_previous_satisfier(
        incompat: &Incompatibility<DP::P, DP::VS, DP::M>,
        satisfier_package: Id<DP::P>,
        mut satisfied_map: SatisfiedMap<DP::P, DP::VS, DP::M>,
        package_assignments: &FnvIndexMap<Id<DP::P>, PackageAssignments<DP::P, DP::VS, DP::M>>,
        store: &Arena<Incompatibility<DP::P, DP::VS, DP::M>>,
    ) -> DecisionLevel {
        // First, let's retrieve the previous derivations and the initial accum_term.
        let satisfier_pa = package_assignments.get(&satisfier_package).unwrap();
        let (satisfier_cause, _gidx, _dl) = satisfied_map.get(&satisfier_package).unwrap();

        let accum_term = if let &Some(cause) = satisfier_cause {
            store[cause].get(satisfier_package).unwrap().negate()
        } else {
            match &satisfier_pa.assignments_intersection {
                AssignmentsIntersection::Derivations(_) => panic!("must be a decision"),
                AssignmentsIntersection::Decision((_, _, term)) => term.clone(),
            }
        };

        let incompat_term = incompat
            .get(satisfier_package)
            .expect("satisfier package not in incompat");

        satisfied_map.insert(
            satisfier_package,
            satisfier_pa.satisfier(
                satisfier_package,
                &accum_term.intersection(&incompat_term.negate()),
            ),
        );

        // Finally, let's identify the decision level of that previous satisfier.
        let (_, &(_, _, decision_level)) = satisfied_map
            .iter()
            .max_by_key(|(_p, (_, global_index, _))| global_index)
            .unwrap();
        decision_level.max(DecisionLevel(1))
    }

    pub(crate) fn current_decision_level(&self) -> DecisionLevel {
        self.current_decision_level
    }
}

impl<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> PackageAssignments<P, VS, M> {
    fn satisfier(
        &self,
        package: Id<P>,
        start_term: &Term<VS>,
    ) -> (Option<IncompId<P, VS, M>>, u32, DecisionLevel) {
        let empty = Term::empty();
        // Indicate if we found a satisfier in the list of derivations, otherwise it will be the decision.
        let idx = self
            .dated_derivations
            .as_slice()
            .partition_point(|dd| !dd.accumulated_intersection.is_disjoint(start_term));
        if let Some(dd) = self.dated_derivations.get(idx) {
            debug_assert_eq!(dd.accumulated_intersection.intersection(start_term), empty);
            return (Some(dd.cause), dd.global_index, dd.decision_level);
        }
        // If it wasn't found in the derivations,
        // it must be the decision which is last (if called in the right context).
        match &self.assignments_intersection {
            AssignmentsIntersection::Decision((global_index, _, _)) => {
                (None, *global_index, self.highest_decision_level)
            }
            AssignmentsIntersection::Derivations(accumulated_intersection) => {
                unreachable!(
                    concat!(
                        "while processing package {:?}: ",
                        "accum_term = {} has overlap with incompat_term = {}, ",
                        "which means the last assignment should have been a decision, ",
                        "but instead it was a derivation. This shouldn't be possible! ",
                        "(Maybe your Version ordering is broken?)"
                    ),
                    package, accumulated_intersection, start_term
                )
            }
        }
    }
}

impl<VS: VersionSet> AssignmentsIntersection<VS> {
    /// Returns the term intersection of all assignments (decision included).
    fn term(&self) -> &Term<VS> {
        match self {
            Self::Decision((_, _, term)) => term,
            Self::Derivations(term) => term,
        }
    }

    /// A package is a potential pick if there isn't an already
    /// selected version (no "decision")
    /// and if it contains at least one positive derivation term
    /// in the partial solution.
    fn potential_package_filter<P: Package>(&self, package: Id<P>) -> Option<(Id<P>, &VS)> {
        match self {
            Self::Decision(_) => None,
            Self::Derivations(term_intersection) => {
                if term_intersection.is_positive() {
                    Some((package, term_intersection.unwrap_positive()))
                } else {
                    None
                }
            }
        }
    }
}
