// SPDX-License-Identifier: MPL-2.0

//! A term is the fundamental unit of operation of the PubGrub algorithm.
//! It is a positive or negative expression regarding a set of versions.

use std::fmt::{self, Display};

use crate::VersionSet;

/// A positive or negative expression regarding a set of versions.
///
/// `Positive(r)` and `Negative(r.complement())` are not equivalent:
/// * the term `Positive(r)` is satisfied if the package is selected AND the selected version is in `r`.
/// * the term `Negative(r.complement())` is satisfied if the package is not selected OR the selected version is in `r`.
///
/// A `Positive` term in the partial solution requires a version to be selected, but a `Negative` term
/// allows for a solution that does not have that package selected.
/// Specifically, `Positive(VS::empty())` means that there was a conflict (we need to select a version for the package
/// but can't pick any), while `Negative(VS::full())` would mean it is fine as long as we don't select the package.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Term<VS: VersionSet> {
    /// For example, `1.0.0 <= v < 2.0.0` is a positive expression
    /// that is evaluated true if a version is selected
    /// and comprised between version 1.0.0 and version 2.0.0.
    Positive(VS),
    /// The term `not (v < 3.0.0)` is a negative expression
    /// that is evaluated true if a version >= 3.0.0 is selected
    /// or if no version is selected at all.
    Negative(VS),
}

/// Base methods.
impl<VS: VersionSet> Term<VS> {
    /// A term that is always true.
    pub(crate) fn any() -> Self {
        Self::Negative(VS::empty())
    }

    /// A term that is never true.
    pub(crate) fn empty() -> Self {
        Self::Positive(VS::empty())
    }

    /// A positive term containing exactly that version.
    pub(crate) fn exact(version: VS::V) -> Self {
        Self::Positive(VS::singleton(version))
    }

    /// Simply check if a term is positive.
    pub(crate) fn is_positive(&self) -> bool {
        match self {
            Self::Positive(_) => true,
            Self::Negative(_) => false,
        }
    }

    /// Negate a term.
    /// Evaluation of a negated term always returns
    /// the opposite of the evaluation of the original one.
    pub(crate) fn negate(&self) -> Self {
        match self {
            Self::Positive(set) => Self::Negative(set.clone()),
            Self::Negative(set) => Self::Positive(set.clone()),
        }
    }

    /// Evaluate a term regarding a given choice of version.
    pub(crate) fn contains(&self, v: &VS::V) -> bool {
        match self {
            Self::Positive(set) => set.contains(v),
            Self::Negative(set) => !set.contains(v),
        }
    }

    /// Unwrap the set contained in a positive term.
    /// Will panic if used on a negative set.
    pub(crate) fn unwrap_positive(&self) -> &VS {
        match self {
            Self::Positive(set) => set,
            _ => panic!("Negative term cannot unwrap positive set"),
        }
    }

    /// Unwrap the set contained in a negative term.
    /// Will panic if used on a positive set.
    pub(crate) fn unwrap_negative(&self) -> &VS {
        match self {
            Self::Negative(set) => set,
            _ => panic!("Positive term cannot unwrap negative set"),
        }
    }
}

/// Set operations with terms.
impl<VS: VersionSet> Term<VS> {
    /// Compute the intersection of two terms.
    ///
    /// The intersection is negative (unselected package is allowed)
    /// if all terms are negative.
    pub(crate) fn intersection(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Positive(r1), Self::Positive(r2)) => Self::Positive(r1.intersection(r2)),
            (Self::Positive(p), Self::Negative(n)) | (Self::Negative(n), Self::Positive(p)) => {
                Self::Positive(n.complement().intersection(p))
            }
            (Self::Negative(r1), Self::Negative(r2)) => Self::Negative(r1.union(r2)),
        }
    }

    /// Check whether two terms are mutually exclusive.
    ///
    /// An optimization for the native implementation of checking whether the intersection of two sets is empty.
    pub(crate) fn is_disjoint(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Positive(r1), Self::Positive(r2)) => r1.is_disjoint(r2),
            // Unselected package is allowed in both terms, so they are never disjoint.
            (Self::Negative(_), Self::Negative(_)) => false,
            // If the positive term is a subset of the negative term, it lies fully in the region that the negative
            // term excludes.
            (Self::Positive(p), Self::Negative(n)) | (Self::Negative(n), Self::Positive(p)) => {
                p.subset_of(n)
            }
        }
    }

    /// Compute the union of two terms.
    /// If at least one term is negative, the union is also negative (unselected package is allowed).
    pub(crate) fn union(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Positive(r1), Self::Positive(r2)) => Self::Positive(r1.union(r2)),
            (Self::Positive(p), Self::Negative(n)) | (Self::Negative(n), Self::Positive(p)) => {
                Self::Negative(p.complement().intersection(n))
            }
            (Self::Negative(r1), Self::Negative(r2)) => Self::Negative(r1.intersection(r2)),
        }
    }

    /// Indicate if this term is a subset of another term.
    /// Just like for sets, we say that t1 is a subset of t2
    /// if and only if t1 ∩ t2 = t1.
    pub(crate) fn subset_of(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Positive(r1), Self::Positive(r2)) => r1.subset_of(r2),
            (Self::Positive(r1), Self::Negative(r2)) => r1.is_disjoint(r2),
            // Only a negative term allows the unselected package,
            // so it can never be a subset of a positive term.
            (Self::Negative(_), Self::Positive(_)) => false,
            (Self::Negative(r1), Self::Negative(r2)) => r2.subset_of(r1),
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

/// Relation between terms.
impl<VS: VersionSet> Term<VS> {
    /// Check if a set of terms satisfies this term.
    ///
    /// We say that a set of terms S "satisfies" a term t
    /// if t must be true whenever every term in S is true.
    ///
    /// It turns out that this can also be expressed with set operations:
    ///    S satisfies t if and only if  ⋂ S ⊆ t
    #[cfg(test)]
    fn satisfied_by(&self, terms_intersection: &Self) -> bool {
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
    #[cfg(test)]
    fn contradicted_by(&self, terms_intersection: &Self) -> bool {
        terms_intersection.intersection(self) == Self::empty()
    }

    /// Check if a set of terms satisfies or contradicts a given term.
    /// Otherwise the relation is inconclusive.
    pub(crate) fn relation_with(&self, other_terms_intersection: &Self) -> Relation {
        if other_terms_intersection.subset_of(self) {
            Relation::Satisfied
        } else if self.is_disjoint(other_terms_intersection) {
            Relation::Contradicted
        } else {
            Relation::Inconclusive
        }
    }
}

impl<VS: VersionSet> AsRef<Self> for Term<VS> {
    fn as_ref(&self) -> &Self {
        self
    }
}

// REPORT ######################################################################

impl<VS: VersionSet + Display> Display for Term<VS> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Positive(set) => write!(f, "{}", set),
            Self::Negative(set) => write!(f, "Not ( {} )", set),
        }
    }
}

// TESTS #######################################################################

#[cfg(test)]
pub mod tests {
    use super::*;
    use proptest::prelude::*;
    use version_ranges::Ranges;

    pub fn strategy() -> impl Strategy<Value = Term<Ranges<u32>>> {
        prop_oneof![
            version_ranges::proptest_strategy().prop_map(Term::Negative),
            version_ranges::proptest_strategy().prop_map(Term::Positive),
        ]
    }
    proptest! {

        // Testing relation --------------------------------

        #[test]
        fn relation_with(term1 in strategy(), term2 in strategy()) {
            match term1.relation_with(&term2) {
                Relation::Satisfied => assert!(term1.satisfied_by(&term2)),
                Relation::Contradicted => assert!(term1.contradicted_by(&term2)),
                Relation::Inconclusive => {
                    assert!(!term1.satisfied_by(&term2));
                    assert!(!term1.contradicted_by(&term2));
                }
            }
        }

        /// Ensure that we don't wrongly convert between positive and negative ranges
        #[test]
        fn positive_negative(term1 in strategy(), term2 in strategy()) {
            let intersection_positive = term1.is_positive() || term2.is_positive();
            let union_positive = term1.is_positive() && term2.is_positive();
            assert_eq!(term1.intersection(&term2).is_positive(), intersection_positive);
            assert_eq!(term1.union(&term2).is_positive(), union_positive);
        }

        #[test]
        fn is_disjoint_through_intersection(r1 in strategy(), r2 in strategy()) {
            let disjoint_def = r1.intersection(&r2) == Term::empty();
            assert_eq!(r1.is_disjoint(&r2), disjoint_def);
        }

        #[test]
        fn subset_of_through_intersection(r1 in strategy(), r2 in strategy()) {
            let disjoint_def = r1.intersection(&r2) == r1;
            assert_eq!(r1.subset_of(&r2), disjoint_def);
        }

        #[test]
        fn union_through_intersection(r1 in strategy(), r2 in strategy()) {
            let union_def = r1
                .negate()
                .intersection(&r2.negate())
                .negate();
            assert_eq!(r1.union(&r2), union_def);
        }
    }
}
