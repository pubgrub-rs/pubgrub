// SPDX-License-Identifier: MPL-2.0

//! Core model and functions
//! to write a functional PubGrub algorithm.

use std::error::Error;
use std::sync::Arc;

use crate::error::PubGrubError;
use crate::internal::arena::Arena;
use crate::internal::incompatibility::{IncompId, Incompatibility, Relation};
use crate::internal::partial_solution::SatisfierSearch::{
    DifferentDecisionLevels, SameDecisionLevels,
};
use crate::internal::partial_solution::{DecisionLevel, PartialSolution};
use crate::internal::small_vec::SmallVec;
use crate::package::Package;
use crate::report::DerivationTree;
use crate::type_aliases::{DependencyConstraints, Map, Set};
use crate::version_set::VersionSet;

/// Current state of the PubGrub algorithm.
#[derive(Clone)]
pub struct State<P: Package, VS: VersionSet, Priority: Ord + Clone> {
    root_package: P,
    root_version: VS::V,

    incompatibilities: Map<P, Vec<IncompId<P, VS>>>,

    /// Store the ids of incompatibilities that are already contradicted.
    /// For each one keep track of the decision level when it was found to be contradicted.
    /// These will stay contradicted until we have backtracked beyond its associated decision level.
    contradicted_incompatibilities: Map<IncompId<P, VS>, DecisionLevel>,

    /// All incompatibilities expressing dependencies,
    /// with common dependents merged.
    merged_dependencies: Map<(P, P), SmallVec<IncompId<P, VS>>>,

    /// Partial solution.
    /// TODO: remove pub.
    pub partial_solution: PartialSolution<P, VS, Priority>,

    /// The store is the reference storage for all incompatibilities.
    pub incompatibility_store: Arena<Incompatibility<P, VS>>,

    /// This is a stack of work to be done in `unit_propagation`.
    /// It can definitely be a local variable to that method, but
    /// this way we can reuse the same allocation for better performance.
    unit_propagation_buffer: SmallVec<P>,
}

impl<P: Package, VS: VersionSet, Priority: Ord + Clone> State<P, VS, Priority> {
    /// Initialization of PubGrub state.
    pub fn init(root_package: P, root_version: VS::V) -> Self {
        let mut incompatibility_store = Arena::new();
        let not_root_id = incompatibility_store.alloc(Incompatibility::not_root(
            root_package.clone(),
            root_version.clone(),
        ));
        let mut incompatibilities = Map::default();
        incompatibilities.insert(root_package.clone(), vec![not_root_id]);
        Self {
            root_package,
            root_version,
            incompatibilities,
            contradicted_incompatibilities: Map::default(),
            partial_solution: PartialSolution::empty(),
            incompatibility_store,
            unit_propagation_buffer: SmallVec::Empty,
            merged_dependencies: Map::default(),
        }
    }

    /// Add an incompatibility to the state.
    pub fn add_incompatibility(&mut self, incompat: Incompatibility<P, VS>) {
        let id = self.incompatibility_store.alloc(incompat);
        self.merge_incompatibility(id);
    }

    /// Add an incompatibility to the state.
    pub fn add_incompatibility_from_dependencies(
        &mut self,
        package: P,
        version: VS::V,
        deps: &DependencyConstraints<P, VS>,
    ) -> std::ops::Range<IncompId<P, VS>> {
        // Create incompatibilities and allocate them in the store.
        let new_incompats_id_range =
            self.incompatibility_store
                .alloc_iter(deps.iter().map(|dep| {
                    Incompatibility::from_dependency(
                        package.clone(),
                        VS::singleton(version.clone()),
                        dep,
                    )
                }));
        // Merge the newly created incompatibilities with the older ones.
        for id in IncompId::range_to_iter(new_incompats_id_range.clone()) {
            self.merge_incompatibility(id);
        }
        new_incompats_id_range
    }

    /// Unit propagation is the core mechanism of the solving algorithm.
    /// CF <https://github.com/dart-lang/pub/blob/master/doc/solver.md#unit-propagation>
    pub fn unit_propagation<E: Error>(&mut self, package: P) -> Result<(), PubGrubError<P, VS, E>> {
        self.unit_propagation_buffer.clear();
        self.unit_propagation_buffer.push(package);
        while let Some(current_package) = self.unit_propagation_buffer.pop() {
            // Iterate over incompatibilities in reverse order
            // to evaluate first the newest incompatibilities.
            let mut conflict_id = None;
            // We only care about incompatibilities if it contains the current package.
            for &incompat_id in self.incompatibilities[&current_package].iter().rev() {
                if self
                    .contradicted_incompatibilities
                    .contains_key(&incompat_id)
                {
                    continue;
                }
                let current_incompat = &self.incompatibility_store[incompat_id];
                match self.partial_solution.relation(current_incompat) {
                    // If the partial solution satisfies the incompatibility
                    // we must perform conflict resolution.
                    Relation::Satisfied => {
                        log::info!(
                            "Start conflict resolution because incompat satisfied:\n   {}",
                            current_incompat
                        );
                        conflict_id = Some(incompat_id);
                        break;
                    }
                    Relation::AlmostSatisfied(package_almost) => {
                        assert!(
                            self.unit_propagation_buffer
                                .iter()
                                .filter(|p| p == &&package_almost)
                                .count()
                                <= 10,
                        );
                        // if !self.unit_propagation_buffer.contains(&package_almost) {
                        self.unit_propagation_buffer.push(package_almost.clone());
                        // }
                        // Add (not term) to the partial solution with incompat as cause.
                        self.partial_solution.add_derivation(
                            package_almost,
                            incompat_id,
                            &self.incompatibility_store,
                        );
                        // With the partial solution updated, the incompatibility is now contradicted.
                        self.contradicted_incompatibilities
                            .insert(incompat_id, self.partial_solution.current_decision_level());
                    }
                    Relation::Contradicted(_) => {
                        self.contradicted_incompatibilities
                            .insert(incompat_id, self.partial_solution.current_decision_level());
                    }
                    _ => {}
                }
            }
            if let Some(incompat_id) = conflict_id {
                let (package_almost, root_cause) = self.conflict_resolution(incompat_id)?;
                self.unit_propagation_buffer.clear();
                self.unit_propagation_buffer.push(package_almost.clone());
                // Add to the partial solution with incompat as cause.
                self.partial_solution.add_derivation(
                    package_almost,
                    root_cause,
                    &self.incompatibility_store,
                );
                // After conflict resolution and the partial solution update,
                // the root cause incompatibility is now contradicted.
                self.contradicted_incompatibilities
                    .insert(root_cause, self.partial_solution.current_decision_level());
            }
        }
        // If there are no more changed packages, unit propagation is done.
        Ok(())
    }

    /// Return the root cause and the backtracked model.
    /// CF <https://github.com/dart-lang/pub/blob/master/doc/solver.md#unit-propagation>
    #[allow(clippy::type_complexity)]
    fn conflict_resolution<E: Error>(
        &mut self,
        incompatibility: IncompId<P, VS>,
    ) -> Result<(P, IncompId<P, VS>), PubGrubError<P, VS, E>> {
        let mut current_incompat_id = incompatibility;
        let mut current_incompat_changed = false;
        loop {
            if self.incompatibility_store[current_incompat_id]
                .is_terminal(&self.root_package, &self.root_version)
            {
                return Err(PubGrubError::NoSolution(
                    self.build_derivation_tree(current_incompat_id),
                ));
            } else {
                let (package, satisfier_search_result) = self.partial_solution.satisfier_search(
                    &self.incompatibility_store[current_incompat_id],
                    &self.incompatibility_store,
                );
                match satisfier_search_result {
                    DifferentDecisionLevels {
                        previous_satisfier_level,
                    } => {
                        let package = package.clone();
                        self.backtrack(
                            current_incompat_id,
                            current_incompat_changed,
                            previous_satisfier_level,
                        );
                        log::info!("backtrack to {:?}", previous_satisfier_level);
                        return Ok((package, current_incompat_id));
                    }
                    SameDecisionLevels { satisfier_cause } => {
                        let prior_cause = Incompatibility::prior_cause(
                            current_incompat_id,
                            satisfier_cause,
                            package,
                            &self.incompatibility_store,
                        );
                        log::info!("prior cause: {}", prior_cause);
                        current_incompat_id = self.incompatibility_store.alloc(prior_cause);
                        current_incompat_changed = true;
                    }
                }
            }
        }
    }

    /// Backtracking.
    fn backtrack(
        &mut self,
        incompat: IncompId<P, VS>,
        incompat_changed: bool,
        decision_level: DecisionLevel,
    ) {
        self.partial_solution.backtrack(decision_level);
        // Remove contradicted incompatibilities that depend on decisions we just backtracked away.
        self.contradicted_incompatibilities
            .retain(|_, dl| *dl <= decision_level);
        if incompat_changed {
            self.merge_incompatibility(incompat);
        }
    }

    /// Add this incompatibility into the set of all incompatibilities.
    ///
    /// Pub collapses identical dependencies from adjacent package versions
    /// into individual incompatibilities.
    /// This substantially reduces the total number of incompatibilities
    /// and makes it much easier for Pub to reason about multiple versions of packages at once.
    ///
    /// For example, rather than representing
    /// foo 1.0.0 depends on bar ^1.0.0 and
    /// foo 1.1.0 depends on bar ^1.0.0
    /// as two separate incompatibilities,
    /// they are collapsed together into the single incompatibility {foo ^1.0.0, not bar ^1.0.0}
    /// (provided that no other version of foo exists between 1.0.0 and 2.0.0).
    /// We could collapse them into { foo (1.0.0 ∪ 1.1.0), not bar ^1.0.0 }
    /// without having to check the existence of other versions though.
    fn merge_incompatibility(&mut self, mut id: IncompId<P, VS>) {
        if let Some((p1, p2)) = self.incompatibility_store[id].as_dependency() {
            // If we are a dependency, there's a good chance we can be merged with a previous dependency
            let deps_lookup = self
                .merged_dependencies
                .entry((p1.clone(), p2.clone()))
                .or_default();
            if let Some((past, merged)) = deps_lookup.as_mut_slice().iter_mut().find_map(|past| {
                self.incompatibility_store[id]
                    .merge_dependents(&self.incompatibility_store[*past])
                    .map(|m| (past, m))
            }) {
                let new = self.incompatibility_store.alloc(merged);
                for (pkg, _) in self.incompatibility_store[new].iter() {
                    self.incompatibilities
                        .entry(pkg.clone())
                        .or_default()
                        .retain(|id| id != past);
                }
                *past = new;
                id = new;
            } else {
                deps_lookup.push(id);
            }
        }
        for (pkg, term) in self.incompatibility_store[id].iter() {
            if cfg!(debug_assertions) {
                assert_ne!(term, &crate::term::Term::any());
            }
            self.incompatibilities
                .entry(pkg.clone())
                .or_default()
                .push(id);
        }
    }

    // Error reporting #########################################################

    fn build_derivation_tree(&self, incompat: IncompId<P, VS>) -> DerivationTree<P, VS> {
        let mut all_ids = Set::default();
        let mut shared_ids = Set::default();
        let mut stack = vec![incompat];
        while let Some(i) = stack.pop() {
            if let Some((id1, id2)) = self.incompatibility_store[i].causes() {
                if all_ids.contains(&i) {
                    shared_ids.insert(i);
                } else {
                    stack.push(id1);
                    stack.push(id2);
                }
            }
            all_ids.insert(i);
        }
        // To avoid recursion we need to generate trees in topological order.
        // That is to say we need to ensure that the causes are processed before the incompatibility they effect.
        // It happens to be that sorting by their ID maintains this property.
        let mut sorted_ids = all_ids.into_iter().collect::<Vec<_>>();
        sorted_ids.sort_unstable_by_key(|id| id.into_raw());
        let mut precomputed = Map::default();
        for id in sorted_ids {
            let tree = Incompatibility::build_derivation_tree(
                id,
                &shared_ids,
                &self.incompatibility_store,
                &precomputed,
            );
            precomputed.insert(id, Arc::new(tree));
        }
        // Now the user can refer to the entire tree from its root.
        Arc::into_inner(precomputed.remove(&incompat).unwrap()).unwrap()
    }
}
