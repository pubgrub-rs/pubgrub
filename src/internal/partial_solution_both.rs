// SPDX-License-Identifier: MPL-2.0

use crate::term::Term;
use crate::type_aliases::SelectedDependencies;
use crate::version::Version;
use crate::{package::Package, range::Range};

use super::incompatibility::IncompId;
use super::incompatibility::Incompatibility;
use super::partial_solution;
use super::partial_solution_bis;
pub use super::partial_solution_bis::Assignment;
pub use super::partial_solution_bis::DecisionLevel;
use super::{arena::Arena, incompatibility::Relation};

#[derive(Clone)]
pub struct PartialSolution<P: Package, V: Version> {
    old: partial_solution::PartialSolution<P, V>,
    bis: partial_solution_bis::PartialSolution<P, V>,
}

impl<P: Package, V: Version> PartialSolution<P, V> {
    /// Initialize an empty partial solution.
    pub fn empty() -> Self {
        Self {
            old: partial_solution::PartialSolution::empty(),
            bis: partial_solution_bis::PartialSolution::empty(),
        }
    }

    /// Add a decision to the partial solution.
    pub fn add_decision(&mut self, package: P, version: V) {
        // println!("add_decision(p: {}, v: {})", &package, &version);
        self.old.add_decision(package.clone(), version.clone());
        self.bis.add_decision(package.clone(), version);
        self.term_intersection_for_package(&package); // for the asserts
    }

    /// Add a derivation to the partial solution.
    pub fn add_derivation(
        &mut self,
        package: P,
        cause: IncompId<P, V>,
        store: &Arena<Incompatibility<P, V>>,
    ) {
        // println!("add_derivation(p: {}, cause: {:?})", &package, &cause);
        self.old.add_derivation(package.clone(), cause, store);
        self.bis.add_derivation(package.clone(), cause, store);
        self.term_intersection_for_package(&package); // for the asserts
    }

    /// If a partial solution has, for every positive derivation,
    /// a corresponding decision that satisfies that assignment,
    /// it's a total solution and version solving has succeeded.
    pub fn extract_solution(&self) -> Option<SelectedDependencies<P, V>> {
        let old = self.old.extract_solution();
        let bis = self.bis.extract_solution();
        assert_eq!(old, bis);
        bis
    }

    /// Compute, cache and retrieve the intersection of all terms for this package.
    pub fn term_intersection_for_package(&self, package: &P) -> Option<&Term<V>> {
        let old = self.old.term_intersection_for_package(package);
        let bis = self.bis.term_intersection_for_package(package);
        assert_eq!(old, bis);
        bis
    }

    /// Backtrack the partial solution to a given decision level.
    pub fn backtrack(
        &mut self,
        decision_level: DecisionLevel,
        store: &Arena<Incompatibility<P, V>>,
    ) {
        // println!("backtrack({:?})", &decision_level);
        self.old.backtrack(decision_level, store);
        self.bis.backtrack(decision_level, store);
        // println!("old: {:#?}", &self.old);
        // println!("bis: {:#?}", &self.bis);
    }

    /// Extract potential packages for the next iteration of unit propagation.
    /// Return `None` if there is no suitable package anymore, which stops the algorithm.
    pub fn potential_packages(&self) -> Option<impl Iterator<Item = (&P, &Range<V>)>> {
        let old = self.old.potential_packages().map(|i| i.collect::<Vec<_>>());
        let bis = self.bis.potential_packages().map(|i| i.collect::<Vec<_>>());
        assert_eq!(old, bis);
        self.old.potential_packages()
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
        // println!(
        //     "add_version(p: {}, version: {}, new_incompatibilities: {:?})",
        //     &package, &version, &new_incompatibilities
        // );
        self.old.add_version(
            package.clone(),
            version.clone(),
            new_incompatibilities.clone(),
            store,
        );
        self.bis
            .add_version(package.clone(), version, new_incompatibilities, store);
        self.term_intersection_for_package(&package); // for the asserts
    }

    /// Check if the terms in the partial solution satisfy the incompatibility.
    pub fn relation(&self, incompat: &Incompatibility<P, V>) -> Relation<P> {
        let old = self.old.relation(incompat);
        let bis = self.bis.relation(incompat);
        assert_eq!(old, bis);
        bis
    }

    /// Find satisfier and previous satisfier decision level.
    pub fn find_satisfier_and_previous_satisfier_level(
        &self,
        incompat: &Incompatibility<P, V>,
        store: &Arena<Incompatibility<P, V>>,
    ) -> (Assignment<P, V>, DecisionLevel, DecisionLevel) {
        let old = self
            .old
            .find_satisfier_and_previous_satisfier_level(incompat, store);
        let bis = self
            .bis
            .find_satisfier_and_previous_satisfier_level(incompat, store);
        assert_eq!(old.0, bis.0);
        assert_eq!(old.1, bis.1);
        assert_eq!(old.2, bis.2);
        bis
    }
}
