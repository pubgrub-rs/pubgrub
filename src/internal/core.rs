// SPDX-License-Identifier: MPL-2.0

//! Core model and functions
//! to write a functional PubGrub algorithm.

use std::{collections::HashSet as Set, rc::Rc};

use crate::error::PubGrubError;
use crate::internal::assignment::Assignment::{Decision, Derivation};
use crate::internal::incompatibility::{Incompatibility, Relation};
use crate::internal::partial_solution::{DecisionLevel, PartialSolution};
use crate::package::Package;
use crate::report::DerivationTree;
use crate::version::Version;

use id_arena::{Arena, Id};

/// Current state of the PubGrub algorithm.
#[derive(Clone)]
pub struct State<P: Package, V: Version> {
    root_package: P,
    root_version: V,

    /// TODO: remove pub.
    pub incompatibilities: Rc<Vec<Id<Incompatibility<P, V>>>>,

    /// Partial solution.
    /// TODO: remove pub.
    pub partial_solution: PartialSolution<P, V>,

    /// The store is the reference storage for all incompatibilities.
    pub incompatibility_store: Arena<Incompatibility<P, V>>,

    /// This is a stack of work to be done in `unit_propagation`.
    /// It can definitely be a local variable to that method, but
    /// this way we can reuse the same allocation for better performance.
    unit_propagation_buffer: Vec<P>,
}

impl<P: Package, V: Version> State<P, V> {
    /// Initialization of PubGrub state.
    pub fn init(root_package: P, root_version: V) -> Self {
        let mut incompatibility_store = Arena::with_capacity(2);
        let not_root_id = incompatibility_store.alloc(Incompatibility::not_root(
            root_package.clone(),
            root_version.clone(),
        ));
        Self {
            root_package,
            root_version,
            incompatibilities: Rc::new(vec![not_root_id]),
            partial_solution: PartialSolution::empty(),
            incompatibility_store,
            unit_propagation_buffer: vec![],
        }
    }

    /// Add an incompatibility to the state.
    pub fn add_incompatibility(&mut self, incompat: Incompatibility<P, V>) {
        Incompatibility::merge_into(
            self.incompatibility_store.alloc(incompat),
            Rc::make_mut(&mut self.incompatibilities),
        );
    }
    /// Check if an incompatibility is terminal.
    pub fn is_terminal(&self, incompatibility: &Incompatibility<P, V>) -> bool {
        incompatibility.is_terminal(&self.root_package, &self.root_version)
    }

    /// Unit propagation is the core mechanism of the solving algorithm.
    /// CF <https://github.com/dart-lang/pub/blob/master/doc/solver.md#unit-propagation>
    pub fn unit_propagation(&mut self, package: P) -> Result<(), PubGrubError<P, V>> {
        self.unit_propagation_buffer.clear();
        self.unit_propagation_buffer.push(package);
        while let Some(current_package) = self.unit_propagation_buffer.pop() {
            // Iterate over incompatibilities in reverse order
            // to evaluate first the newest incompatibilities.
            for &incompat_id in Rc::clone(&self.incompatibilities).iter().rev() {
                // We only care about that incompatibility if it contains the current package.
                if self.incompatibility_store[incompat_id].get(&current_package) == None {
                    continue;
                }
                match self
                    .partial_solution
                    .relation(&self.incompatibility_store[incompat_id])
                {
                    // If the partial solution satisfies the incompatibility
                    // we must perform conflict resolution.
                    Relation::Satisfied => {
                        let (package_almost, root_cause) = self.conflict_resolution(incompat_id)?;
                        self.unit_propagation_buffer.clear();
                        self.unit_propagation_buffer.push(package_almost.clone());
                        // Add to the partial solution with incompat as cause.
                        self.partial_solution.add_derivation(
                            package_almost,
                            root_cause,
                            &self.incompatibility_store,
                        );
                    }
                    Relation::AlmostSatisfied(package_almost) => {
                        self.unit_propagation_buffer.push(package_almost.clone());
                        // Add (not term) to the partial solution with incompat as cause.
                        self.partial_solution.add_derivation(
                            package_almost,
                            incompat_id,
                            &self.incompatibility_store,
                        );
                    }
                    _ => {}
                }
            }
        }
        // If there are no more changed packages, unit propagation is done.
        Ok(())
    }

    /// Return the root cause and the backtracked model.
    /// CF <https://github.com/dart-lang/pub/blob/master/doc/solver.md#unit-propagation>
    fn conflict_resolution(
        &mut self,
        incompatibility: Id<Incompatibility<P, V>>,
    ) -> Result<(P, Id<Incompatibility<P, V>>), PubGrubError<P, V>> {
        let mut current_incompat = incompatibility;
        let mut current_incompat_changed = false;
        loop {
            if self.incompatibility_store[current_incompat]
                .is_terminal(&self.root_package, &self.root_version)
            {
                return Err(PubGrubError::NoSolution(
                    self.build_derivation_tree(current_incompat),
                ));
            } else {
                let (satisfier, satisfier_level, previous_satisfier_level) = self
                    .partial_solution
                    .find_satisfier_and_previous_satisfier_level(
                        &self.incompatibility_store[current_incompat],
                        &self.incompatibility_store,
                    );
                match satisfier {
                    Decision { package, .. } => {
                        self.backtrack(
                            current_incompat,
                            current_incompat_changed,
                            previous_satisfier_level,
                        );
                        return Ok((package, current_incompat));
                    }
                    Derivation { cause, package } => {
                        if previous_satisfier_level != satisfier_level {
                            self.backtrack(
                                current_incompat,
                                current_incompat_changed,
                                previous_satisfier_level,
                            );
                            return Ok((package, current_incompat));
                        } else {
                            let prior_cause = Incompatibility::prior_cause(
                                current_incompat,
                                cause,
                                &package,
                                &self.incompatibility_store,
                            );
                            current_incompat = self.incompatibility_store.alloc(prior_cause);
                            current_incompat_changed = true;
                        }
                    }
                }
            }
        }
    }

    /// Backtracking.
    fn backtrack(
        &mut self,
        incompat: Id<Incompatibility<P, V>>,
        incompat_changed: bool,
        decision_level: DecisionLevel,
    ) {
        self.partial_solution
            .backtrack(decision_level, &self.incompatibility_store);
        if incompat_changed {
            Incompatibility::merge_into(incompat, Rc::make_mut(&mut self.incompatibilities));
        }
    }

    // Error reporting #########################################################

    fn build_derivation_tree(&self, incompat: Id<Incompatibility<P, V>>) -> DerivationTree<P, V> {
        let shared_ids = self.find_shared_ids(incompat);
        Incompatibility::build_derivation_tree(incompat, &shared_ids, &self.incompatibility_store)
    }

    fn find_shared_ids(
        &self,
        incompat: Id<Incompatibility<P, V>>,
    ) -> Set<Id<Incompatibility<P, V>>> {
        let mut all_ids = Set::new();
        let mut shared_ids = Set::new();
        let mut stack = vec![incompat];
        while let Some(i) = stack.pop() {
            if let Some((id1, id2)) = self.incompatibility_store[i].causes() {
                if all_ids.contains(&i) {
                    shared_ids.insert(i);
                } else {
                    all_ids.insert(i);
                    stack.push(id1);
                    stack.push(id2);
                }
            }
        }
        shared_ids
    }
}
