// SPDX-License-Identifier: MPL-2.0

//! A term is the fundamental unit of operation of the PubGrub algorithm.
//! It is a positive or negative expression regarding a set of versions.

use std::fmt::{self, Display};

use crate::experimental::{VersionIndex, VersionSet};

/// A positive or negative expression regarding a set of versions.
///
/// `Term::positive(vs)` and `Term::negative(vs.complement())` are not equivalent:
/// * `Term::positive(vs)` is satisfied if the package is selected AND the selected version is in `vs`.
/// * `Term::negative(vs.complement())` is satisfied if the package is not selected OR the selected version is in `vs`.
///
/// A positive term in the partial solution requires a version to be selected, but a negative term
/// allows for a solution that does not have that package selected.
/// Specifically, `Term::positive(VersionSet::empty())` means that there was a conflict
/// (we need to select a version for the package but can't pick any),
/// while `Term::negative(VersionSet::full())` would mean it is fine as long as we don't select the package.
///
/// Like [`VersionSet`], this is implemented as a `u64` bitset,
/// where the first bit is set if the unselected package is allowed (for a negative term),
/// and the others bits are set if the package selected at the corresponding version index is allowed.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct Term(u64);

impl Term {
    /// Construct a positive `Term`.
    /// For example, `[1, 2]` is a positive expression
    /// that is evaluated true if a version with index 1 or 2 is selected.
    #[inline]
    pub(crate) fn positive(vs: VersionSet) -> Self {
        Self(vs.0)
    }

    /// Construct a negative `Term`.
    /// For example, `not [3, 5]` is a negative expression
    /// that is evaluated true if a version with index 3 or 5 is selected
    /// or if no version is selected at all.
    #[inline]
    pub(crate) fn negative(vs: VersionSet) -> Self {
        Self(!vs.0)
    }

    /// A term that is always true.
    #[inline]
    pub(crate) fn any() -> Self {
        Self(!0)
    }

    /// A term that is never true.
    #[inline]
    pub(crate) fn empty() -> Self {
        Self(0)
    }

    /// A positive term containing exactly that version.
    #[inline]
    pub(crate) fn exact(version_index: VersionIndex) -> Self {
        Self::positive(VersionSet::singleton(version_index))
    }

    /// Simply check if a term is positive.
    #[inline]
    pub fn is_positive(self) -> bool {
        self.0 & 1 == 0
    }

    /// Simply check if a term is negative.
    #[inline]
    pub fn is_negative(self) -> bool {
        self.0 & 1 != 0
    }

    /// Negate a term.
    /// Evaluation of a negated term always returns
    /// the opposite of the evaluation of the original one.
    #[inline]
    pub(crate) fn negate(self) -> Self {
        Self(!self.0)
    }

    /// Get the inner version set.
    #[inline]
    pub fn version_set(self) -> VersionSet {
        if self.is_positive() {
            VersionSet(self.0)
        } else {
            VersionSet(!self.0)
        }
    }

    /// Evaluate a term regarding a given choice of version.
    #[inline]
    pub(crate) fn contains(self, v: VersionIndex) -> bool {
        self.0 & VersionSet::singleton(v).0 != 0
    }

    /// Unwrap the set contained in a positive term.
    /// Will panic if used on a negative set.
    #[inline]
    pub(crate) fn unwrap_positive(self) -> VersionSet {
        if self.is_positive() {
            VersionSet(self.0)
        } else {
            panic!("Negative term cannot unwrap positive set")
        }
    }

    /// Unwrap the set contained in a negative term.
    /// Will panic if used on a positive set.
    #[inline]
    pub(crate) fn unwrap_negative(self) -> VersionSet {
        if self.is_negative() {
            VersionSet(!self.0)
        } else {
            panic!("Positive term cannot unwrap negative set")
        }
    }

    /// Compute the intersection of two terms.
    /// The intersection is negative (unselected package is allowed)
    /// if all terms are negative.
    #[inline]
    pub(crate) fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Compute the union of two terms.
    /// If at least one term is negative, the union is also negative (unselected package is allowed).
    #[inline]
    pub(crate) fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Check whether two terms are mutually exclusive.
    #[inline]
    pub(crate) fn is_disjoint(self, other: Self) -> bool {
        self.0 & other.0 == 0
    }

    /// Indicate if this term is a subset of another term.
    /// Just like for sets, we say that t1 is a subset of t2
    /// if and only if t1 ∩ t2 = t1.
    #[inline]
    pub(crate) fn subset_of(self, other: Self) -> bool {
        self.0 & other.0 == self.0
    }

    /// Check if a set of terms satisfies or contradicts a given term.
    /// Otherwise the relation is inconclusive.
    #[inline]
    pub(crate) fn relation_with(self, other_terms_intersection: Self) -> Relation {
        if other_terms_intersection.subset_of(self) {
            Relation::Satisfied
        } else if other_terms_intersection.is_disjoint(self) {
            Relation::Contradicted
        } else {
            Relation::Inconclusive
        }
    }
}

/// Describe a relation between a set of terms S and another term t.
///
/// As a shorthand, we say that a term v
/// satisfies or contradicts a term t if {v} satisfies or contradicts it.
pub(crate) enum Relation {
    /// We say that a set of terms S "satisfies" a term t
    /// if t must be true whenever every term in S is true.
    Satisfied,
    /// Conversely, S "contradicts" t if t must be false
    /// whenever every term in S is true.
    Contradicted,
    /// If neither of these is true we say that S is "inconclusive" for t.
    Inconclusive,
}

impl Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_negative() {
            write!(f, "Not ( ")?;
        }

        let mut list = f.debug_list();
        for v in self.version_set().iter() {
            list.entry(&v.get());
        }
        list.finish()?;

        if self.is_negative() {
            write!(f, " )")?;
        }

        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use proptest::prelude::*;

    use super::*;

    impl Term {
        /// Check if a set of terms satisfies this term.
        ///
        /// We say that a set of terms S "satisfies" a term t
        /// if t must be true whenever every term in S is true.
        ///
        /// It turns out that this can also be expressed with set operations:
        ///    S satisfies t if and only if  ⋂ S ⊆ t
        fn satisfied_by(self, terms_intersection: Self) -> bool {
            terms_intersection.subset_of(self)
        }

        /// Check if a set of terms contradicts this term.
        ///
        /// We say that a set of terms S "contradicts" a term t
        /// if t must be false whenever every term in S is true.
        ///
        /// It turns out that this can also be expressed with set operations:
        ///    S contradicts t if and only if ⋂ S is disjoint with t
        ///    S contradicts t if and only if  (⋂ S) ⋂ t = ∅
        fn contradicted_by(self, terms_intersection: Self) -> bool {
            terms_intersection.intersection(self) == Self::empty()
        }
    }

    pub fn strategy() -> impl Strategy<Value = Term> {
        any::<u64>().prop_map(Term)
    }

    proptest! {
        /// Testing relation
        #[test]
        fn relation_with(term1 in strategy(), term2 in strategy()) {
            match term1.relation_with(term2) {
                Relation::Satisfied => assert!(term1.satisfied_by(term2)),
                Relation::Contradicted => assert!(term1.contradicted_by(term2)),
                Relation::Inconclusive => {
                    assert!(!term1.satisfied_by(term2));
                    assert!(!term1.contradicted_by(term2));
                }
            }
        }

        /// Ensure that we don't wrongly convert between positive and negative ranges
        #[test]
        fn positive_negative(term1 in strategy(), term2 in strategy()) {
            let intersection_positive = term1.is_positive() || term2.is_positive();
            let union_positive = term1.is_positive() & term2.is_positive();
            assert_eq!(term1.intersection(term2).is_positive(), intersection_positive);
            assert_eq!(term1.union(term2).is_positive(), union_positive);
        }

        #[test]
        fn is_disjoint_through_intersection(r1 in strategy(), r2 in strategy()) {
            let disjoint_def = r1.intersection(r2) == Term::empty();
            assert_eq!(r1.is_disjoint(r2), disjoint_def);
        }

        #[test]
        fn subset_of_through_intersection(r1 in strategy(), r2 in strategy()) {
            let disjoint_def = r1.intersection(r2) == r1;
            assert_eq!(r1.subset_of(r2), disjoint_def);
        }

        #[test]
        fn union_through_intersection(r1 in strategy(), r2 in strategy()) {
            let union_def = r1
                .negate()
                .intersection(r2.negate())
                .negate();
            assert_eq!(r1.union(r2), union_def);
        }
    }
}
