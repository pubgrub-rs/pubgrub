// SPDX-License-Identifier: MPL-2.0

//! An incompatibility is a set of terms for different packages
//! that should never be satisfied all together.

use std::collections::HashSet as Set;
use std::fmt;

use crate::package::Package;
use crate::range::Range;
use crate::report::{DefaultStringReporter, DerivationTree, Derived, External};
use crate::solver::DependencyConstraints;
use crate::term::{self, Term};
use crate::type_aliases::Map;
use crate::version::Version;

/// An incompatibility is a set of terms for different packages
/// that should never be satisfied all together.
/// An incompatibility usually originates from a package dependency.
/// For example, if package A at version 1 depends on package B
/// at version 2, you can never have both terms `A = 1`
/// and `not B = 2` satisfied at the same time in a partial solution.
/// This would mean that we found a solution with package A at version 1
/// but not with package B at version 2.
/// Yet A at version 1 depends on B at version 2 so this is not possible.
/// Therefore, the set `{ A = 1, not B = 2 }` is an incompatibility,
/// defined from dependencies of A at version 1.
///
/// Incompatibilities can also be derived from two other incompatibilities
/// during conflict resolution. More about all this in
/// [PubGrub documentation](https://github.com/dart-lang/pub/blob/master/doc/solver.md#incompatibility).
#[derive(Debug, Clone)]
pub struct Incompatibility<P: Package, V: Version> {
    /// TODO: remove pub.
    pub id: usize,
    package_terms: Map<P, Term<V>>,
    kind: Kind<P, V>,
}

#[derive(Debug, Clone)]
enum Kind<P: Package, V: Version> {
    NotRoot(P, V),
    NoVersion(P, Range<V>),
    UnavailableDependencies(P, Range<V>),
    FromDependencyOf(P, Range<V>, P, Range<V>),
    DerivedFrom(usize, usize),
}

/// A type alias for a pair of [Package] and a corresponding [Term].
pub type PackageTerm<P, V> = (P, Term<V>);

/// A Relation describes how a set of terms can be compared to an incompatibility.
/// Typically, the set of terms comes from the partial solution.
#[derive(Eq, PartialEq)]
pub enum Relation<P: Package, V: Version> {
    /// We say that a set of terms S satisfies an incompatibility I
    /// if S satisfies every term in I.
    Satisfied,
    /// We say that S contradicts I
    /// if S contradicts at least one term in I.
    Contradicted(PackageTerm<P, V>),
    /// If S satisfies all but one of I's terms and is inconclusive for the remaining term,
    /// we say S "almost satisfies" I and we call the remaining term the "unsatisfied term".
    AlmostSatisfied(PackageTerm<P, V>),
    /// Otherwise, we say that their relation is inconclusive.
    Inconclusive,
}

impl<P: Package, V: Version> Incompatibility<P, V> {
    /// Create the initial "not Root" incompatibility.
    pub fn not_root(id: usize, package: P, version: V) -> Self {
        let mut package_terms = Map::with_capacity_and_hasher(1, Default::default());
        package_terms.insert(
            package.clone(),
            Term::Negative(Range::exact(version.clone())),
        );
        Self {
            id,
            package_terms,
            kind: Kind::NotRoot(package, version),
        }
    }

    /// Create an incompatibility to remember
    /// that a given range does not contain any version.
    pub fn no_version(id: usize, package: P, term: Term<V>) -> Self {
        let range = match &term {
            Term::Positive(r) => r.clone(),
            Term::Negative(_) => panic!("No version should have a positive term"),
        };
        let mut package_terms = Map::with_capacity_and_hasher(1, Default::default());
        package_terms.insert(package.clone(), term);
        Self {
            id,
            package_terms,
            kind: Kind::NoVersion(package, range),
        }
    }

    /// Create an incompatibility to remember
    /// that a package version is not selectable
    /// because its list of dependencies is unavailable.
    pub fn unavailable_dependencies(id: usize, package: P, version: V) -> Self {
        let range = Range::exact(version);
        let mut package_terms = Map::with_capacity_and_hasher(1, Default::default());
        package_terms.insert(package.clone(), Term::Positive(range.clone()));
        Self {
            id,
            package_terms,
            kind: Kind::UnavailableDependencies(package, range),
        }
    }

    /// Generate a list of incompatibilities from direct dependencies of a package.
    pub fn from_dependencies(
        start_id: usize,
        package: P,
        version: V,
        deps: &DependencyConstraints<P, V>,
    ) -> Vec<Self> {
        deps.iter()
            .enumerate()
            .map(|(i, dep)| {
                Self::from_dependency(start_id + i, package.clone(), version.clone(), dep)
            })
            .collect()
    }

    /// Build an incompatibility from a given dependency.
    fn from_dependency(id: usize, package: P, version: V, dep: (&P, &Range<V>)) -> Self {
        let mut package_terms = Map::with_capacity_and_hasher(2, Default::default());
        let range1 = Range::exact(version);
        package_terms.insert(package.clone(), Term::Positive(range1.clone()));
        let (p2, range2) = dep;
        package_terms.insert(p2.clone(), Term::Negative(range2.clone()));
        Self {
            id,
            package_terms,
            kind: Kind::FromDependencyOf(package, range1, p2.clone(), range2.clone()),
        }
    }

    /// Perform the intersection of terms in two incompatibilities.
    fn intersection(i1: &Map<P, Term<V>>, i2: &Map<P, Term<V>>) -> Map<P, Term<V>> {
        Self::merge(i1, i2, |t1, t2| Some(t1.intersection(t2)))
    }

    /// Merge two hash maps.
    ///
    /// When a key is common to both,
    /// apply the provided function to both values.
    /// If the result is None, remove that key from the merged map,
    /// otherwise add the content of the Some(_).
    fn merge<T: Clone, F: Fn(&T, &T) -> Option<T>>(
        map_1: &Map<P, T>,
        map_2: &Map<P, T>,
        f: F,
    ) -> Map<P, T> {
        let mut merged_map = map_1.clone();
        merged_map.reserve(map_2.len());
        let mut to_delete = Vec::new();
        for (key, val_2) in map_2.iter() {
            match merged_map.get_mut(key) {
                None => {
                    merged_map.insert(key.clone(), val_2.clone());
                }
                Some(val_1) => match f(val_1, val_2) {
                    None => to_delete.push(key),
                    Some(merged_value) => *val_1 = merged_value,
                },
            }
        }
        for key in to_delete.iter() {
            merged_map.remove(key);
        }
        merged_map
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
    /// And it would even keep the same [Kind]: [FromDependencyOf](Kind::FromDependencyOf) foo.
    ///
    /// Here we do the simple stupid thing of just growing the Vec.
    /// TODO: improve this.
    /// It may not be trivial since those incompatibilities
    /// may already have derived others.
    /// Maybe this should not be pursued.
    pub fn merge_into(self, incompatibilities: &mut Vec<Self>) {
        incompatibilities.push(self);
    }

    /// Prior cause of two incompatibilities using the rule of resolution.
    pub fn prior_cause(
        id: usize,
        incompat: &Self,
        satisfier_cause: &Self,
        package: &P,
        satisfier: &Term<V>,
    ) -> Self {
        let kind = Kind::DerivedFrom(incompat.id, satisfier_cause.id);
        let mut t1 = incompat.package_terms.clone();
        let mut t2 = satisfier_cause.package_terms.clone();
        t1.remove(package);
        t2.remove(package);
        let mut prior_cause_terms = Self::intersection(&t1, &t2);
        let term = incompat.package_terms.get(package).unwrap();
        let p_term = term.union(&satisfier.negate());
        if p_term != Term::any() {
            prior_cause_terms.insert(package.clone(), p_term);
        }
        Self {
            id,
            package_terms: prior_cause_terms,
            kind,
        }
    }

    /// CF definition of Relation enum.
    pub fn relation(&self, mut terms: impl FnMut(&P) -> Option<Term<V>>) -> Relation<P, V> {
        let mut relation = Relation::Satisfied;
        for (package, incompat_term) in self.package_terms.iter() {
            match terms(package).map(|term| incompat_term.relation_with(&term)) {
                Some(term::Relation::Satisfied) => {}
                Some(term::Relation::Contradicted) => {
                    return Relation::Contradicted((package.clone(), incompat_term.clone()));
                }
                None | Some(term::Relation::Inconclusive) => {
                    // If a package is not present, the intersection is the same as any.
                    // And according to the rules of satisfactions, the relation would be inconclusive.
                    // It could also be satisfied if the incompatibility term was also any.
                    // But we systematically remove those from incompatibilities.
                    // So we're safe on that front.
                    if relation == Relation::Satisfied {
                        relation =
                            Relation::AlmostSatisfied((package.clone(), incompat_term.clone()));
                    } else {
                        relation = Relation::Inconclusive;
                    }
                }
            }
        }
        relation
    }

    /// Check if an incompatibility should mark the end of the algorithm
    /// because it satisfies the root package.
    pub fn is_terminal(&self, root_package: &P, root_version: &V) -> bool {
        if self.package_terms.is_empty() {
            true
        } else if self.package_terms.len() > 1 {
            false
        } else {
            let (package, term) = self.package_terms.iter().next().unwrap();
            (package == root_package) && term.contains(&root_version)
        }
    }

    /// Get the term related to a given package (if it exists).
    pub fn get(&self, package: &P) -> Option<&Term<V>> {
        self.package_terms.get(package)
    }

    /// Iterate over packages.
    pub fn iter(&self) -> impl Iterator<Item = (&P, &Term<V>)> {
        self.package_terms.iter()
    }

    // Reporting ###############################################################

    /// Retrieve parent causes if of type DerivedFrom.
    pub fn causes(&self) -> Option<(usize, usize)> {
        match self.kind {
            Kind::DerivedFrom(id1, id2) => Some((id1, id2)),
            _ => None,
        }
    }

    /// Build a derivation tree for error reporting.
    pub fn build_derivation_tree(
        &self,
        shared_ids: &Set<usize>,
        store: &[Self],
    ) -> DerivationTree<P, V> {
        match &self.kind {
            Kind::DerivedFrom(id1, id2) => {
                let cause1 = store[*id1].build_derivation_tree(shared_ids, store);
                let cause2 = store[*id2].build_derivation_tree(shared_ids, store);
                let derived = Derived {
                    terms: self.package_terms.clone(),
                    shared_id: shared_ids.get(&self.id).cloned(),
                    cause1: Box::new(cause1),
                    cause2: Box::new(cause2),
                };
                DerivationTree::Derived(derived)
            }
            Kind::NotRoot(package, version) => {
                DerivationTree::External(External::NotRoot(package.clone(), version.clone()))
            }
            Kind::NoVersion(package, range) => {
                DerivationTree::External(External::NoVersion(package.clone(), range.clone()))
            }
            Kind::UnavailableDependencies(package, range) => DerivationTree::External(
                External::UnavailableDependencies(package.clone(), range.clone()),
            ),
            Kind::FromDependencyOf(package, range, dep_package, dep_range) => {
                DerivationTree::External(External::FromDependencyOf(
                    package.clone(),
                    range.clone(),
                    dep_package.clone(),
                    dep_range.clone(),
                ))
            }
        }
    }
}

impl<P: Package, V: Version> fmt::Display for Incompatibility<P, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            DefaultStringReporter::string_terms(&self.package_terms)
        )
    }
}

impl<P: Package, V: Version> IntoIterator for Incompatibility<P, V> {
    type Item = (P, Term<V>);
    type IntoIter = std::collections::hash_map::IntoIter<P, Term<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.package_terms.into_iter()
    }
}

// TESTS #######################################################################

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::term::tests::strategy as term_strat;
    use proptest::prelude::*;

    proptest! {

        /// For any three different packages p1, p2 and p3,
        /// for any three terms t1, t2 and t3,
        /// if we have the two following incompatibilities:
        ///    { p1: t1, p2: not t2 }
        ///    { p2: t2, p3: t3 }
        /// the rule of resolution says that we can deduce the following incompatibility:
        ///    { p1: t1, p3: t3 }
        #[test]
        fn rule_of_resolution(t1 in term_strat(), t2 in term_strat(), t3 in term_strat()) {
            let mut i1 = Map::default();
            i1.insert("p1", t1.clone());
            i1.insert("p2", t2.negate());
            let i1 = Incompatibility { id: 0, package_terms: i1, kind: Kind::DerivedFrom(0,0) };

            let mut i2 = Map::default();
            i2.insert("p2", t2.clone());
            i2.insert("p3", t3.clone());
            let i2 = Incompatibility { id: 0, package_terms: i2, kind: Kind::DerivedFrom(0,0) };

            let mut i3 = Map::default();
            i3.insert("p1", t1);
            i3.insert("p3", t3);

            let i_resolution = Incompatibility::prior_cause(0, &i1, &i2, &"p2", &t2.negate());
            assert_eq!(i_resolution.package_terms, i3);
        }

    }
}
