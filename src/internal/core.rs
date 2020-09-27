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

    /// TODO: remove pub.
    pub incompatibilities: Vec<Incompatibility<P, V>>,

    /// Partial solution.
    /// TODO: remove pub.
    pub partial_solution: PartialSolution<P, V>,

    /// The store is the reference storage for all incompatibilities.
    /// The id field in one incompatibility refers
    /// to the position in the `incompatibility_store` vec,
    /// NOT the position in the `incompatibilities` vec.
    /// TODO: remove pub.
    pub incompatibility_store: Vec<Incompatibility<P, V>>,
}

impl<P: Package, V: Version> State<P, V> {
    /// Initialization of PubGrub state.
    pub fn init(root_package: P, root_version: V) -> Self {
        let not_root_incompat =
            Incompatibility::not_root(0, root_package.clone(), root_version.clone());
        Self {
            root_package,
            root_version,
            incompatibilities: vec![not_root_incompat.clone()],
            partial_solution: PartialSolution::empty(),
            incompatibility_store: vec![not_root_incompat],
        }
    }

    /// Add an incompatibility to the state.
    pub fn add_incompatibility<F: Fn(usize) -> Incompatibility<P, V>>(&mut self, gen_incompat: F) {
        let incompat = gen_incompat(self.incompatibility_store.len());
        self.incompatibility_store.push(incompat.clone());
        incompat.merge_into(&mut self.incompatibilities);
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
            let mut loop_incompatibilities = self.incompatibilities.clone();
            while let Some(incompat) = loop_incompatibilities.pop() {
                // We only care about that incompatibility if it contains the current package.
                if incompat.get(&current_package) == None {
                    continue;
                }
                match self.partial_solution.relation(&incompat) {
                    // If the partial solution satisfies the incompatibility
                    // we must perform conflict resolution.
                    Relation::Satisfied => {
                        let root_cause = self.conflict_resolution(&incompat)?;
                        // root_cause is guaranted to be almost satisfied by the partial solution
                        // according to PubGrub documentation.
                        match self.partial_solution.relation(&root_cause) {
                            Relation::AlmostSatisfied(package_almost, term) => {
                                changed = vec![package_almost.clone()];
                                // Add (not term) to the partial solution with incompat as cause.
                                self.partial_solution.add_derivation(package_almost, term.negate(), root_cause);
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
        incompatibility: &Incompatibility<P, V>,
    ) -> Result<Incompatibility<P, V>, PubGrubError<P, V>> {
        let mut current_incompat = incompatibility.clone();
        let mut current_incompat_changed = false;
        loop {
            if current_incompat.is_terminal(&self.root_package, &self.root_version) {
                return Err(PubGrubError::NoSolution(
                    self.build_derivation_tree(&current_incompat),
                ));
            } else {
                let (satisfier, satisfier_level, previous_satisfier_level) = self
                    .partial_solution
                    .find_satisfier_and_previous_satisfier_level(&current_incompat);
                match satisfier {
                    Decision { .. } => {
                        self.backtrack(
                            current_incompat.clone(),
                            current_incompat_changed,
                            previous_satisfier_level,
                        );
                        return Ok(current_incompat);
                    }
                    Derivation { cause, .. } => {
                        if previous_satisfier_level != satisfier_level {
                            self.backtrack(
                                current_incompat.clone(),
                                current_incompat_changed,
                                previous_satisfier_level,
                            );
                            return Ok(current_incompat);
                        } else {
                            let id = self.incompatibility_store.len();
                            let prior_cause =
                                Incompatibility::prior_cause(id, &current_incompat, &cause);
                            // eprintln!("\ncause 1: {}", &current_incompat);
                            // eprintln!("cause 2: {}", &cause);
                            // eprintln!("prior cause: {}\n", &prior_cause);
                            self.incompatibility_store.push(prior_cause.clone());
                            current_incompat = prior_cause;
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
        incompat: Incompatibility<P, V>,
        incompat_changed: bool,
        decision_level: usize,
    ) {
        self.partial_solution.backtrack(decision_level);
        if incompat_changed {
            incompat.merge_into(&mut self.incompatibilities);
        }
    }

    // Error reporting #########################################################

    fn build_derivation_tree(&self, incompat: &Incompatibility<P, V>) -> DerivationTree<P, V> {
        let shared_ids = self.find_shared_ids(incompat);
        incompat.build_derivation_tree(&shared_ids, self.incompatibility_store.as_slice())
    }

    fn find_shared_ids(&self, incompat: &Incompatibility<P, V>) -> Set<usize> {
        let mut all_ids = Set::new();
        let mut shared_ids = Set::new();
        let mut stack = Vec::new();
        stack.push(incompat);
        while let Some(i) = stack.pop() {
            if let Some((id1, id2)) = i.causes() {
                if all_ids.contains(&i.id) {
                    shared_ids.insert(i.id);
                } else {
                    all_ids.insert(i.id);
                    stack.push(&self.incompatibility_store[id1]);
                    stack.push(&self.incompatibility_store[id2]);
                }
            }
        }
        shared_ids
    }
}
