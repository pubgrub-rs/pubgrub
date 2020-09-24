// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Core model and functions
//! to write a functional PubGrub algorithm.

use std::collections::HashSet as Set;

use crate::error::PubGrubError;
use crate::internal::assignment::Assignment::{Decision, Derivation};
use crate::internal::incompatibility::{Incompatibility, Relation};
use crate::internal::partial_solution::PartialSolution;
use crate::package::Package;
use crate::report::DerivationTree;
use crate::version::Version;

/// Current state of the PubGrub algorithm.
#[derive(Clone)]
pub struct State<P: Package, V: Version> {
    root_package: P,
    root_version: V,

    /// Partial solution.
    /// TODO: remove pub.
    pub partial_solution: PartialSolution<P, V>,

    incompatibilities: Vec<(usize, Incompatibility<P, V>)>,

    /// The store is the reference storage for all incompatibilities.
    /// The id field in one incompatibility refers
    /// to the position in the `incompatibility_store` vec,
    /// NOT the position in the `incompatibilities` vec.
    incompatibility_store: Vec<Incompatibility<P, V>>,
}

impl<P: Package, V: Version> State<P, V> {
    /// Initialization of PubGrub state.
    pub fn init(root_package: P, root_version: V) -> Self {
        let not_root_incompat =
            Incompatibility::not_root(root_package.clone(), root_version.clone());
        Self {
            root_package,
            root_version,
            incompatibilities: vec![(0, not_root_incompat.clone())],
            partial_solution: PartialSolution::empty(),
            incompatibility_store: vec![not_root_incompat],
        }
    }

    /// Add an incompatibility to the state.
    pub fn add_incompatibility(&mut self, incompat: Incompatibility<P, V>) -> usize {
        let id = self.store_incompatibility(incompat.clone());
        incompat.merge_into(id, &mut self.incompatibilities);
        id
    }

    /// Store an incompatibility (careful diffence with add_incompatibility).
    fn store_incompatibility(&mut self, incompat: Incompatibility<P, V>) -> usize {
        let id = self.incompatibility_store.len();
        self.incompatibility_store.push(incompat.clone());
        id
    }

    /// Check if an incompatibility is terminal.
    pub fn is_terminal(&self, incompatibility: &Incompatibility<P, V>) -> bool {
        incompatibility.is_terminal(&self.root_package, &self.root_version)
    }

    /// Unit propagation is the core mechanism of the solving algorithm.
    /// CF https://github.com/dart-lang/pub/blob/master/doc/solver.md#unit-propagation
    pub fn unit_propagation(&mut self, package: P) -> Result<(), PubGrubError<P, V>> {
        let mut current_package = package.clone();
        let mut changed = vec![package];
        loop {
            // Iterate over incompatibilities in reverse order
            // to evaluate first the newest incompatibilities.
            // PS: we are cloning self.incompatibilities because we will mutate it
            // and we don't want that to interfere with the loop over
            // incompatibilities before the mutation, that we call loop_incompatibilities.
            let mut loop_incompatibilities = self.incompatibilities.clone();
            while let Some((incompat_id, incompat)) = loop_incompatibilities.pop() {
                // We only care about that incompatibility if it contains the current package.
                if incompat.get(&current_package) == None {
                    continue;
                }
                match self.partial_solution.relation(&incompat) {
                    // If the partial solution satisfies the incompatibility
                    // we must perform conflict resolution.
                    Relation::Satisfied => {
                        let (root_id, root_cause) =
                            self.conflict_resolution(incompat_id, &incompat)?;
                        // root_cause is guaranted to be almost satisfied by the partial solution
                        // according to PubGrub documentation.
                        match self.partial_solution.relation(&root_cause) {
                            Relation::AlmostSatisfied(package_almost, term) => {
                                changed = vec![package_almost.clone()];
                                // Add (not term) to the partial solution with incompat as cause.
                                self.partial_solution.add_derivation(package_almost, term.negate(), root_cause, root_id);
                            }
                            _ => return Err(PubGrubError::Failure("This should never happen, root_cause is guaranted to be almost satisfied by the partial solution".into())),
                        }
                    }
                    Relation::AlmostSatisfied(package_almost, term) => {
                        changed.push(package_almost.clone());
                        // Add (not term) to the partial solution with incompat as cause.
                        self.partial_solution.add_derivation(
                            package_almost,
                            term.negate(),
                            incompat,
                            incompat_id,
                        );
                    }
                    _ => {}
                }
            }
            // If there are no more changed packages, unit propagation is done.
            match changed.pop() {
                None => break,
                Some(current) => current_package = current,
            }
        }
        Ok(())
    }

    /// Return the root cause and the backtracked model.
    /// CF https://github.com/dart-lang/pub/blob/master/doc/solver.md#unit-propagation
    fn conflict_resolution(
        &mut self,
        incompat_id: usize,
        incompatibility: &Incompatibility<P, V>,
    ) -> Result<(usize, Incompatibility<P, V>), PubGrubError<P, V>> {
        let mut current_incompat = incompatibility.clone();
        let mut current_incompat_changed = false;
        loop {
            if current_incompat.is_terminal(&self.root_package, &self.root_version) {
                return Err(PubGrubError::NoSolution(self.build_derivation_tree(
                    self.incompatibility_store.len(),
                    &current_incompat,
                )));
            } else {
                let (satisfier, satisfier_level, previous_satisfier_level) = self
                    .partial_solution
                    .find_satisfier_and_previous_satisfier_level(&current_incompat);
                match satisfier {
                    Decision { .. } => {
                        let root_id = self
                            .backtrack(
                                current_incompat.clone(),
                                current_incompat_changed,
                                previous_satisfier_level,
                            )
                            .unwrap_or(incompat_id);
                        return Ok((root_id, current_incompat));
                    }
                    Derivation {
                        cause, cause_id, ..
                    } => {
                        if previous_satisfier_level != satisfier_level {
                            let root_id = self
                                .backtrack(
                                    current_incompat.clone(),
                                    current_incompat_changed,
                                    previous_satisfier_level,
                                )
                                .unwrap_or(incompat_id);
                            return Ok((root_id, current_incompat));
                        } else {
                            let current_id = match current_incompat_changed {
                                true => self.store_incompatibility(current_incompat.clone()),
                                false => incompat_id,
                            };
                            current_incompat = Incompatibility::prior_cause(
                                current_id,
                                &current_incompat,
                                cause_id,
                                &cause,
                            );
                            current_incompat_changed = true;
                        }
                    }
                }
            }
        }
    }

    /// Backtracking.
    /// Return the id of the added incompatibility if incompat has changed.
    fn backtrack(
        &mut self,
        incompat: Incompatibility<P, V>,
        incompat_changed: bool,
        decision_level: usize,
    ) -> Option<usize> {
        self.partial_solution.backtrack(decision_level);
        match incompat_changed {
            true => Some(self.add_incompatibility(incompat)),
            false => None,
        }
    }

    // Error reporting #########################################################

    fn build_derivation_tree(
        &self,
        id: usize,
        incompat: &Incompatibility<P, V>,
    ) -> DerivationTree<P, V> {
        let shared_ids = self.find_shared_ids(id, incompat);
        incompat.build_derivation_tree(id, &shared_ids, self.incompatibility_store.as_slice())
    }

    fn find_shared_ids(&self, id: usize, incompat: &Incompatibility<P, V>) -> Set<usize> {
        let mut all_ids = Set::new();
        let mut shared_ids = Set::new();
        let mut stack = Vec::new();
        stack.push((id, incompat));
        while let Some((stack_id, stack_incompat)) = stack.pop() {
            if let Some((id1, id2)) = stack_incompat.causes() {
                if all_ids.contains(&stack_id) {
                    shared_ids.insert(stack_id);
                } else {
                    all_ids.insert(stack_id);
                    stack.push((id1, &self.incompatibility_store[id1]));
                    stack.push((id2, &self.incompatibility_store[id2]));
                }
            }
        }
        shared_ids
    }
}
