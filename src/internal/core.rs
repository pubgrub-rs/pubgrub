// SPDX-License-Identifier: MPL-2.0

//! Core model and functions
//! to write a functional PubGrub algorithm.

use std::sync::Arc;

use smallvec::SmallVec;

use crate::{
    internal::{
        DecisionLevel, IncompatArena, IncompatId, Incompatibility, PartialSolution, Relation,
        SatisfierSearch,
    },
    DependencyProvider, DerivationTree, Map, PackageArena, PackageId, Set, Term, VersionIndex,
    VersionSet,
};

/// Current state of the PubGrub algorithm.
pub(crate) struct State<DP: DependencyProvider> {
    root_package_id: PackageId,
    root_version_index: VersionIndex,

    incompatibilities: Vec<SmallVec<[IncompatId; 4]>>,

    /// All incompatibilities expressing dependencies,
    /// with common dependents merged.
    merged_dependencies: Map<(PackageId, PackageId), SmallVec<[IncompatId; 4]>>,

    /// Partial solution.
    /// TODO: remove pub.
    pub(crate) partial_solution: PartialSolution<DP>,

    /// The store is the reference storage for all incompatibilities.
    pub(crate) incompatibility_store: IncompatArena<DP::M>,

    /// This is a stack of work to be done in `unit_propagation`.
    /// It can definitely be a local variable to that method, but
    /// this way we can reuse the same allocation for better performance.
    unit_propagation_buffer: Vec<PackageId>,
}

impl<DP: DependencyProvider> State<DP> {
    /// Initialization of PubGrub state.
    pub(crate) fn init(root_package_id: PackageId, root_version_index: VersionIndex) -> Self {
        let mut incompatibility_store = IncompatArena::new();
        let not_root_id = incompatibility_store.push(Incompatibility::not_root(
            root_package_id,
            root_version_index,
        ));
        let root_package_idx = root_package_id.get() as usize;
        let mut incompatibilities = vec![SmallVec::new(); root_package_idx + 1];
        incompatibilities[root_package_idx].push(not_root_id);
        Self {
            root_package_id,
            root_version_index,
            incompatibilities,
            partial_solution: PartialSolution::empty(root_package_id),
            incompatibility_store,
            unit_propagation_buffer: Vec::new(),
            merged_dependencies: Map::default(),
        }
    }

    /// Add an incompatibility to the state.
    pub(crate) fn add_incompatibility(&mut self, incompat: Incompatibility<DP::M>) {
        let id = self.incompatibility_store.push(incompat);
        self.merge_incompatibility(id);
    }

    /// Add an incompatibility to the state.
    #[cold]
    pub(crate) fn add_incompatibility_from_dependencies(
        &mut self,
        package_id: PackageId,
        version_index: VersionIndex,
        deps: impl IntoIterator<Item = (PackageId, VersionSet)>,
    ) -> std::ops::Range<IncompatId> {
        // Create incompatibilities and allocate them in the store.
        let vs = VersionSet::singleton(version_index);
        let new_incompats_id_range = self.incompatibility_store.extend(
            deps.into_iter()
                .map(|dep| Incompatibility::from_dependency(package_id, vs, dep)),
        );
        // Merge the newly created incompatibilities with the older ones.
        for id in IncompatId::range_to_iter(new_incompats_id_range.clone()) {
            self.merge_incompatibility(id);
        }
        new_incompats_id_range
    }

    /// Unit propagation is the core mechanism of the solving algorithm.
    /// CF <https://github.com/dart-lang/pub/blob/master/doc/solver.md#unit-propagation>
    #[cold]
    pub(crate) fn unit_propagation(
        &mut self,
        package_id: PackageId,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &mut DP,
    ) -> Result<(), DerivationTree<DP::M>> {
        self.unit_propagation_buffer.clear();
        self.unit_propagation_buffer.push(package_id);
        while let Some(current_package) = self.unit_propagation_buffer.pop() {
            // Iterate over incompatibilities in reverse order
            // to evaluate first the newest incompatibilities.
            let mut conflict_id = None;
            // We only care about incompatibilities if it contains the current package.
            let idx = current_package.get() as usize;
            for &incompat_id in self.incompatibilities[idx].iter().rev() {
                let current_incompat = &mut self.incompatibility_store[incompat_id];
                if self.partial_solution.is_contradicted(current_incompat) {
                    continue;
                }
                match self.partial_solution.relation(current_incompat) {
                    // If the partial solution satisfies the incompatibility
                    // we must perform conflict resolution.
                    Relation::Satisfied => {
                        log::info!(
                            "Start conflict resolution because incompat satisfied:\n   {}",
                            current_incompat.display(package_store, dependency_provider)
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
                            self.unit_propagation_buffer.push(package_almost);
                        }
                        // Add (not term) to the partial solution with incompat as cause.
                        self.partial_solution.add_derivation(
                            package_almost,
                            incompat_id,
                            current_incompat.get(package_almost).unwrap(),
                        );
                        // With the partial solution updated, the incompatibility is now contradicted.
                        self.partial_solution.contradict(current_incompat);
                    }
                    Relation::Contradicted(_) => {
                        self.partial_solution.contradict(current_incompat);
                    }
                    _ => {}
                }
            }
            if let Some(incompat_id) = conflict_id {
                let (package_almost, root_cause) = self
                    .conflict_resolution(incompat_id, package_store, dependency_provider)
                    .map_err(|terminal_incompat_id| {
                        self.build_derivation_tree(terminal_incompat_id)
                    })?;
                dependency_provider.register_conflict(
                    self.incompatibility_store[root_cause]
                        .iter()
                        .map(|(pid, _)| pid),
                    package_store,
                );
                self.unit_propagation_buffer.clear();
                self.unit_propagation_buffer.push(package_almost);
                let root_incompat = &mut self.incompatibility_store[root_cause];
                // Add to the partial solution with incompat as cause.
                self.partial_solution.add_derivation(
                    package_almost,
                    root_cause,
                    root_incompat.get(package_almost).unwrap(),
                );
                // After conflict resolution and the partial solution update,
                // the root cause incompatibility is now contradicted.
                self.partial_solution.contradict(root_incompat);
            }
        }
        // If there are no more changed packages, unit propagation is done.
        Ok(())
    }

    /// Return the root cause or the terminal incompatibility.
    /// CF <https://github.com/dart-lang/pub/blob/master/doc/solver.md#unit-propagation>
    #[cold]
    fn conflict_resolution(
        &mut self,
        incompatibility: IncompatId,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) -> Result<(PackageId, IncompatId), IncompatId> {
        let mut current_incompat_id = incompatibility;
        let mut current_incompat_changed = false;
        loop {
            if self.incompatibility_store[current_incompat_id]
                .is_terminal(self.root_package_id, self.root_version_index)
            {
                return Err(current_incompat_id);
            }

            let (package_id, satisfier_search_result) = self.partial_solution.satisfier_search(
                &self.incompatibility_store[current_incompat_id],
                &self.incompatibility_store,
                package_store,
            );
            match satisfier_search_result {
                SatisfierSearch::DifferentDecisionLevels {
                    previous_satisfier_level,
                } => {
                    self.backtrack(
                        current_incompat_id,
                        current_incompat_changed,
                        previous_satisfier_level,
                    );
                    log::info!("backtrack to {:?}", previous_satisfier_level);
                    return Ok((package_id, current_incompat_id));
                }
                SatisfierSearch::SameDecisionLevels { satisfier_cause } => {
                    let prior_cause = Incompatibility::prior_cause(
                        current_incompat_id,
                        satisfier_cause,
                        package_id,
                        &self.incompatibility_store,
                    );
                    log::info!(
                        "prior cause: {}",
                        prior_cause.display(package_store, dependency_provider)
                    );
                    current_incompat_id = self.incompatibility_store.push(prior_cause);
                    current_incompat_changed = true;
                }
            }
        }
    }

    /// Backtracking.
    fn backtrack(
        &mut self,
        incompat_id: IncompatId,
        incompat_changed: bool,
        decision_level: DecisionLevel,
    ) {
        self.partial_solution.backtrack(decision_level);
        if incompat_changed {
            self.merge_incompatibility(incompat_id);
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
    fn merge_incompatibility(&mut self, mut id: IncompatId) {
        fn get_or_default(
            v: &mut Vec<SmallVec<[IncompatId; 4]>>,
            package_id: PackageId,
        ) -> &mut SmallVec<[IncompatId; 4]> {
            let pkg_idx = package_id.get() as usize;
            if pkg_idx + 1 > v.len() {
                v.resize(pkg_idx + 1, SmallVec::new());
            }
            &mut v[pkg_idx]
        }

        if let Some((pid1, pid2)) = self.incompatibility_store[id].as_dependency() {
            // If we are a dependency, there's a good chance we can be merged with a previous dependency
            let deps_lookup = self.merged_dependencies.entry((pid1, pid2)).or_default();
            if let Some((past, merged)) = deps_lookup.iter_mut().find_map(|past| {
                self.incompatibility_store[id]
                    .merge_dependents(&self.incompatibility_store[*past])
                    .map(|m| (past, m))
            }) {
                let new = self.incompatibility_store.push(merged);
                for (package_id, _) in self.incompatibility_store[new].iter() {
                    get_or_default(&mut self.incompatibilities, package_id).retain(|id| id != past);
                }
                *past = new;
                id = new;
            } else {
                deps_lookup.push(id);
            }
        }
        for (package_id, term) in self.incompatibility_store[id].iter() {
            debug_assert_ne!(term, Term::any());
            get_or_default(&mut self.incompatibilities, package_id).push(id);
        }
    }

    // Error reporting #########################################################

    fn build_derivation_tree(&self, incompat: IncompatId) -> DerivationTree<DP::M> {
        let mut all_ids: Set<IncompatId> = Set::default();
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
        sorted_ids.sort_unstable_by_key(|id| id.0);
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
