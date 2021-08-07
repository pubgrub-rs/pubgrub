// SPDX-License-Identifier: MPL-2.0

//! Ranges are constraints defining sets of versions.
//!
//! Concretely, those constraints correspond to any set of versions
//! representable as the concatenation, union, and complement
//! of the ranges building blocks.
//!
//! Those building blocks are:
//!  - [none()](Range::none): the empty set
//!  - [any()](Range::any): the set of all possible versions
//!  - [exact(v)](Range::exact): the set containing only the version v
//!  - [higher_than(v)](Range::higher_than): the set defined by `v <= versions`
//!  - [strictly_lower_than(v)](Range::strictly_lower_than): the set defined by `versions < v`
//!  - [between(v1, v2)](Range::between): the set defined by `v1 <= versions < v2`

use std::fmt::{Debug, Display};

/// Trait describing sets of versions.
pub trait VersionSet: Debug + Display + Clone + Eq {
    /// Version type associated with the sets manipulated.
    type V: Clone + Debug + Display;

    // Constructors
    /// Constructor for an empty set containing no version.
    fn empty() -> Self;
    /// Constructor for a set containing exactly one version.
    fn singleton(v: Self::V) -> Self;

    // Operations
    /// Compute the complement of this set.
    fn complement(&self) -> Self;
    /// Compute the intersection with another set.
    fn intersection(&self, other: &Self) -> Self;

    // Membership
    /// Evaluate membership of a version in this set.
    fn contains(&self, v: &Self::V) -> bool;

    // Automatically implemented functions ###########################

    /// Constructor for the set containing all versions.
    /// Automatically implemented as `Self::empty().complement()`.
    fn full() -> Self {
        Self::empty().complement()
    }

    /// Compute the union with another set.
    /// Thanks to set properties, this is automatically implemented as:
    /// `self.complement().intersection(&other.complement()).complement()`
    fn union(&self, other: &Self) -> Self {
        self.complement()
            .intersection(&other.complement())
            .complement()
    }
}
