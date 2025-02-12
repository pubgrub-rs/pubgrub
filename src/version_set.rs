// SPDX-License-Identifier: MPL-2.0

use std::fmt::{Debug, Display};

use crate::Ranges;

/// A set of versions.
///
/// See [`Ranges`] for an implementation.
///
/// The methods with default implementations can be overwritten for better performance, but their
/// output must be equal to the default implementation.
///
/// # Equality
///
/// It is important that the `Eq` trait is implemented so that if two sets contain the same
/// versions, they are equal under `Eq`. In particular, you can only use `#[derive(Eq, PartialEq)]`
/// if `Eq` is strictly equivalent to the structural equality, i.e. if version sets are always
/// stored in a canonical representations. Such problems may arise if your implementations of
/// `complement()` and `intersection()` do not return canonical representations.
///
/// For example, `>=1,<4 || >=2,<5` and `>=1,<4 || >=3,<5` are equal, because they can both be
/// normalized to `>=1,<5`.
///
/// Note that pubgrub does not know which versions actually exist for a package, the contract
/// is about upholding the mathematical properties of set operations, assuming all versions are
/// possible. This is required for the solver to determine the relationship of version sets to each
/// other.
pub trait VersionSet: Debug + Display + Clone + Eq {
    /// Version type associated with the sets manipulated.
    type V: Debug + Display + Clone + Ord;

    // Constructors

    /// An empty set containing no version.
    fn empty() -> Self;

    /// A set containing only the given version.
    fn singleton(v: Self::V) -> Self;

    // Operations

    /// The set of all version that are not in this set.
    fn complement(&self) -> Self;

    /// The set of all versions that are in both sets.
    fn intersection(&self, other: &Self) -> Self;

    /// Whether the version is part of this set.
    fn contains(&self, v: &Self::V) -> bool;

    // Automatically implemented functions

    /// The set containing all versions.
    ///
    /// The default implementation is the complement of the empty set.
    fn full() -> Self {
        Self::empty().complement()
    }

    /// The set of all versions that are either (or both) of the sets.
    ///
    /// The default implementation is complement of the intersection of the complements of both sets
    /// (De Morgan's law).
    fn union(&self, other: &Self) -> Self {
        self.complement()
            .intersection(&other.complement())
            .complement()
    }

    /// Whether the ranges have no overlapping segments.
    fn is_disjoint(&self, other: &Self) -> bool {
        self.intersection(other) == Self::empty()
    }

    /// Whether all ranges of `self` are contained in `other`.
    fn subset_of(&self, other: &Self) -> bool {
        self == &self.intersection(other)
    }
}

/// [`Ranges`] contains optimized implementations of all operations.
impl<T: Debug + Display + Clone + Eq + Ord> VersionSet for Ranges<T> {
    type V = T;

    fn empty() -> Self {
        Ranges::empty()
    }

    fn singleton(v: Self::V) -> Self {
        Ranges::singleton(v)
    }

    fn complement(&self) -> Self {
        Ranges::complement(self)
    }

    fn intersection(&self, other: &Self) -> Self {
        Ranges::intersection(self, other)
    }

    fn contains(&self, v: &Self::V) -> bool {
        Ranges::contains(self, v)
    }

    fn full() -> Self {
        Ranges::full()
    }

    fn union(&self, other: &Self) -> Self {
        Ranges::union(self, other)
    }

    fn is_disjoint(&self, other: &Self) -> bool {
        Ranges::is_disjoint(self, other)
    }

    fn subset_of(&self, other: &Self) -> bool {
        Ranges::subset_of(self, other)
    }
}
