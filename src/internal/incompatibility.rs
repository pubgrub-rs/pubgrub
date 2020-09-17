// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! An incompatibility is a set of terms for different packages
//! that should never be satisfied all together.

use std::collections::HashMap as Map;
use std::hash::Hash;

use crate::internal::term::{self, Term};
use crate::range::Range;
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
pub struct Incompatibility<'a, P, V>
where
    P: Clone + Eq + Hash,
    V: Clone + Ord + Version,
{
    package_terms: Map<P, Term<V>>,
    kind: Kind<'a, P, V>,
}

#[derive(Debug, Clone)]
enum Kind<'a, P, V>
where
    P: Clone + Eq + Hash,
    V: Clone + Ord + Version,
{
    NotRoot,
    NoVersion,
    UnavailableDependencies(P, V),
    FromDependencyOf(P),
    DerivedFrom(&'a Incompatibility<'a, P, V>, &'a Incompatibility<'a, P, V>),
}

/// A Relation describes how a set of terms can be compared to an incompatibility.
#[derive(Eq, PartialEq)]
pub enum Relation<P, V>
where
    P: Clone + Eq + Hash,
    V: Clone + Ord + Version,
{
    /// We say that a set of terms S satisfies an incompatibility I
    /// if S satisfies every term in I.
    Satisfied,
    /// We say that S contradicts I
    /// if S contradicts at least one term in I.
    Contradicted(P, Term<V>),
    /// If S satisfies all but one of I's terms and is inconclusive for the remaining term,
    /// we say S "almost satisfies" I and we call the remaining term the "unsatisfied term".
    AlmostSatisfied(P, Term<V>),
    /// Otherwise, we say that their relation is inconclusive.
    Inconclusive,
}

impl<'a, P, V> Incompatibility<'a, P, V>
where
    P: Clone + Eq + Hash,
    V: Clone + Ord + Version,
{
    /// Create the initial "not Root" incompatibility.
    pub fn not_root(package: P, version: V) -> Self {
        let mut package_terms = Map::with_capacity(1);
        package_terms.insert(package, Term::Negative(Range::exact(version)));
        Self {
            package_terms,
            kind: Kind::NotRoot,
        }
    }

    /// Create an incompatibility to remember
    /// that a given range does not contain any version.
    pub fn no_version(package: P, term: Term<V>) -> Self {
        let mut package_terms = Map::with_capacity(1);
        package_terms.insert(package, term);
        Self {
            package_terms,
            kind: Kind::NoVersion,
        }
    }

    /// Create an incompatibility to remember
    /// that a package version is not selectable
    /// because its list of dependencies is unavailable.
    pub fn unavailable_dependencies(package: P, version: V) -> Self {
        let mut package_terms = Map::with_capacity(1);
        package_terms.insert(
            package.clone(),
            Term::Positive(Range::exact(version.clone())),
        );
        Self {
            package_terms,
            kind: Kind::UnavailableDependencies(package, version),
        }
    }

    /// Generate a list of incompatibilities from direct dependencies of a package.
    pub fn from_dependencies(package: P, version: V, deps: &Map<P, Range<V>>) -> Vec<Self> {
        deps.iter()
            .map(|dep| Self::from_dependency(package.clone(), version.clone(), dep))
            .collect()
    }

    /// Build an incompatibility from a given dependency.
    fn from_dependency(package: P, version: V, dep: (&P, &Range<V>)) -> Self {
        let mut i1 = Map::with_capacity(1);
        let mut i2 = Map::with_capacity(1);
        i1.insert(
            package.clone(),
            Term::Positive(Range::exact(version.clone())),
        );
        let (p, range) = dep;
        i2.insert(p.clone(), Term::Negative(range.clone()));
        Self::union(&i1, &i2, Kind::FromDependencyOf(package))
    }

    /// Perform the union of two incompatibilities.
    /// Terms that are always satisfied are removed from the union.
    fn union(i1: &Map<P, Term<V>>, i2: &Map<P, Term<V>>, kind: Kind<'a, P, V>) -> Self {
        let package_terms = Self::merge(i1, i2, |t1, t2| {
            let term_union = t1.union(t2);
            if term_union == Term::Negative(Range::none()) {
                // When the union of two terms is "not none",
                // remove that term from the incompatibility
                // since it will always be satisfied.
                None
            } else {
                Some(term_union)
            }
        });
        Self {
            package_terms,
            kind,
        }
    }

    /// Merge two hash maps.
    ///
    /// When a key is common to both,
    /// apply the provided function to both values.
    /// If the result is None, remove that key from the merged map,
    /// otherwise add the content of the Some(_).
    fn merge<T: Clone, F: Fn(&T, &T) -> Option<T>>(
        t1: &Map<P, T>,
        t2: &Map<P, T>,
        f: F,
    ) -> Map<P, T> {
        let mut merged_map: Map<_, _> = t1.clone();
        merged_map.reserve(t2.len());
        let mut to_delete = Vec::new();
        for (package, term_2) in t2.iter() {
            match merged_map.get_mut(package) {
                None => {
                    merged_map.insert(package.clone(), term_2.clone());
                }
                Some(term_1) => match f(term_1, term_2) {
                    None => to_delete.push(package),
                    Some(term_union) => *term_1 = term_union,
                },
            }
        }
        for package in to_delete.iter() {
            merged_map.remove(package);
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
    /// without having to check existance of other versions though.
    /// And it would even keep the same `Kind`: `FromDependencyOf foo`.
    ///
    /// Here we do the simple stupid thing of just growing the Vec.
    /// TODO: improve this.
    /// It may not be trivial since those incompatibilities
    /// may already have derived others.
    /// Maybe this should not be persued.
    pub fn merge_into(self, incompatibilities: Vec<Self>) -> Vec<Self> {
        let mut incompats = incompatibilities;
        incompats.push(self);
        incompats
    }

    /// A prior cause is computed as the union of the terms in two incompatibilities.
    /// Terms that are always satisfied are removed from the union.
    pub fn prior_cause(i1: &'a Self, i2: &'a Self) -> Self {
        let kind = Kind::DerivedFrom(i1, i2);
        Self::union(&i1.package_terms, &i2.package_terms, kind)
    }

    /// CF definition of Relation enum.
    pub fn relation<T: AsRef<Term<V>>>(
        &self,
        terms_set: &mut Map<P, impl Iterator<Item = T>>,
    ) -> Relation<P, V> {
        let mut relation = Relation::Satisfied;
        for (package, incompat_term) in self.package_terms.iter() {
            let terms_in_set = terms_set.get_mut(package);
            match incompat_term.relation_with(terms_in_set) {
                term::Relation::Satisfied => {}
                term::Relation::Contradicted => {
                    relation = Relation::Contradicted(package.clone(), incompat_term.clone());
                    break;
                }
                term::Relation::Inconclusive => {
                    if relation == Relation::Satisfied {
                        relation =
                            Relation::AlmostSatisfied(package.clone(), incompat_term.clone());
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
            (package == root_package) && term.accept_version(&root_version)
        }
    }

    /// Get the term related to a given package (if it exists).
    pub fn get(&self, package: &P) -> Option<&Term<V>> {
        self.package_terms.get(package)
    }

    /// Iterate over packages.
    pub fn iter(&self) -> std::collections::hash_map::Iter<P, Term<V>> {
        self.package_terms.iter()
    }
}

impl<'a, P, V> IntoIterator for Incompatibility<'a, P, V>
where
    P: Clone + Eq + Hash,
    V: Clone + Ord + Version,
{
    type Item = (P, Term<V>);
    type IntoIter = std::collections::hash_map::IntoIter<P, Term<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.package_terms.into_iter()
    }
}
