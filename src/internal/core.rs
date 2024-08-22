// SPDX-License-Identifier: MPL-2.0

//! Core model and functions
//! to write a functional PubGrub algorithm.

use std::collections::HashSet as Set;
use std::sync::Arc;

use crate::internal::{
    Arena, DecisionLevel, IncompDpId, Incompatibility, PartialSolution, Relation, SatisfierSearch,
    SmallVec,
};
use crate::{DependencyProvider, DerivationTree, Map, NoSolutionError, VersionSet};

/// Current state of the PubGrub algorithm.
#[derive(Clone)]
pub(crate) struct State<DP: DependencyProvider> {
    root_package: DP::P,
    root_version: DP::V,

    #[allow(clippy::type_complexity)]
    incompatibilities: Map<DP::P, Vec<IncompDpId<DP>>>,

    /// Store the ids of incompatibilities that are already contradicted.
    /// For each one keep track of the decision level when it was found to be contradicted.
    /// These will stay contradicted until we have backtracked beyond its associated decision level.
    contradicted_incompatibilities: Map<IncompDpId<DP>, DecisionLevel>,

    /// All incompatibilities expressing dependencies,
    /// with common dependents merged.
    #[allow(clippy::type_complexity)]
    merged_dependencies: Map<(DP::P, DP::P), SmallVec<IncompDpId<DP>>>,

    /// Partial solution.
    /// TODO: remove pub.
    pub(crate) partial_solution: PartialSolution<DP>,

    /// The store is the reference storage for all incompatibilities.
    pub(crate) incompatibility_store: Arena<Incompatibility<DP::P, DP::VS, DP::M>>,

    /// This is a stack of work to be done in `unit_propagation`.
    /// It can definitely be a local variable to that method, but
    /// this way we can reuse the same allocation for better performance.
    unit_propagation_buffer: SmallVec<DP::P>,
}

impl<DP: DependencyProvider> State<DP> {
    /// Initialization of PubGrub state.
    pub(crate) fn init(root_package: DP::P, root_version: DP::V) -> Self {
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
    pub(crate) fn add_incompatibility(&mut self, incompat: Incompatibility<DP::P, DP::VS, DP::M>) {
        let id = self.incompatibility_store.alloc(incompat);
        self.merge_incompatibility(id);
    }

    /// Add an incompatibility to the state.
    pub(crate) fn add_incompatibility_from_dependencies(
        &mut self,
        package: DP::P,
        version: DP::V,
        deps: impl IntoIterator<Item = (DP::P, DP::VS)>,
    ) -> std::ops::Range<IncompDpId<DP>> {
        // Create incompatibilities and allocate them in the store.
        let new_incompats_id_range =
            self.incompatibility_store
                .alloc_iter(deps.into_iter().map(|dep| {
                    Incompatibility::from_dependency(
                        package.clone(),
                        <DP::VS as VersionSet>::singleton(version.clone()),
                        dep,
                    )
                }));
        // Merge the newly created incompatibilities with the older ones.
        for id in IncompDpId::<DP>::range_to_iter(new_incompats_id_range.clone()) {
            self.merge_incompatibility(id);
        }
        new_incompats_id_range
    }

    /// Unit propagation is the core mechanism of the solving algorithm.
    /// CF <https://github.com/dart-lang/pub/blob/master/doc/solver.md#unit-propagation>
    pub(crate) fn unit_propagation(&mut self, package: DP::P) -> Result<(), NoSolutionError<DP>> {
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
                        // Add `package_almost` to the `unit_propagation_buffer` set.
                        // Putting items in `unit_propagation_buffer` more than once waste cycles,
                        // but so does allocating a hash map and hashing each item.
                        // In practice `unit_propagation_buffer` is small enough that we can just do a linear scan.
                        if !self.unit_propagation_buffer.contains(&package_almost) {
                            self.unit_propagation_buffer.push(package_almost.clone());
                        }
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
                let (package_almost, root_cause) =
                    self.conflict_resolution(incompat_id)
                        .map_err(|terminal_incompat_id| {
                            self.build_derivation_tree(terminal_incompat_id)
                        })?;
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

    /// Return the root cause or the terminal incompatibility.
    /// CF <https://github.com/dart-lang/pub/blob/master/doc/solver.md#unit-propagation>
    #[allow(clippy::type_complexity)]
    fn conflict_resolution(
        &mut self,
        incompatibility: IncompDpId<DP>,
    ) -> Result<(DP::P, IncompDpId<DP>), IncompDpId<DP>> {
        let mut current_incompat_id = incompatibility;
        let mut current_incompat_changed = false;
        loop {
            if self.incompatibility_store[current_incompat_id]
                .is_terminal(&self.root_package, &self.root_version)
            {
                return Err(current_incompat_id);
            } else {
                let (package, satisfier_search_result) = self.partial_solution.satisfier_search(
                    &self.incompatibility_store[current_incompat_id],
                    &self.incompatibility_store,
                );
                match satisfier_search_result {
                    SatisfierSearch::DifferentDecisionLevels {
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
                    SatisfierSearch::SameDecisionLevels { satisfier_cause } => {
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
        incompat: IncompDpId<DP>,
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
    /// PubGrub collapses identical dependencies from adjacent package versions
    /// into individual incompatibilities.
    /// This substantially reduces the total number of incompatibilities
    /// and makes it much easier for PubGrub to reason about multiple versions of packages at once.
    ///
    /// For example, rather than representing
    /// foo 1.0.0 depends on bar ^1.0.0 and
    /// foo 1.1.0 depends on bar ^1.0.0
    /// as two separate incompatibilities,
    /// they are collapsed together into the single incompatibility {foo ^1.0.0, not bar ^1.0.0}
    /// (provided that no other version of foo exists between 1.0.0 and 2.0.0).
    /// We could collapse them into { foo (1.0.0 ∪ 1.1.0), not bar ^1.0.0 }
    /// without having to check the existence of other versions though.
    fn merge_incompatibility(&mut self, mut id: IncompDpId<DP>) {
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

    fn build_derivation_tree(
        &self,
        incompat: IncompDpId<DP>,
    ) -> DerivationTree<DP::P, DP::VS, DP::M> {
        let mut all_ids: Set<IncompDpId<DP>> = Set::default();
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
