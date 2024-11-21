// SPDX-License-Identifier: MPL-2.0

/// Type for identifying a version index.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct VersionIndex(u8);

impl VersionIndex {
    /// Maximum possible version index.
    pub const MAX: u64 = (u64::BITS - 1) as u64;

    /// Constructor for a version index.
    #[inline]
    pub fn new(v: u8) -> Option<Self> {
        if v < Self::MAX as u8 {
            Some(Self(v))
        } else {
            None
        }
    }

    /// Get the inner version index.
    #[inline]
    pub fn get(self) -> u8 {
        self.0
    }
}

/// Type for identifying a set of version indices.
///
/// This is implemented as a `u64` bitset which can represent up to 63 versions.
/// The first bit is kept unset to leave space for the positive/negative bit of [`Term`](super::Term).
///
/// See the [helpers](super::helpers) module to support more than 63 versions by using a wrapper package.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct VersionSet(pub(crate) u64);

impl VersionSet {
    /// Constructor for an empty set containing no version index.
    #[inline]
    pub fn empty() -> Self {
        Self(0)
    }

    /// Constructor for the set containing all version indices.
    #[inline]
    pub fn full() -> Self {
        Self(u64::MAX & (!1))
    }

    /// Constructor for a set containing exactly one version index.
    #[inline]
    pub fn singleton(v: VersionIndex) -> Self {
        Self(2 << v.0)
    }

    /// Compute the complement of this set.
    #[inline]
    pub fn complement(self) -> Self {
        Self((!self.0) & (!1))
    }

    /// Compute the intersection with another set.
    #[inline]
    pub fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Compute the union with another set.
    #[inline]
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Evaluate membership of a version index in this set.
    #[inline]
    pub fn contains(self, v: VersionIndex) -> bool {
        self.intersection(Self::singleton(v)) != Self::empty()
    }

    /// Whether the set has no overlapping version indices.
    #[inline]
    pub fn is_disjoint(self, other: Self) -> bool {
        self.intersection(other) == Self::empty()
    }

    /// Whether all version indices of `self` are contained in `other`.
    #[inline]
    pub fn subset_of(self, other: Self) -> bool {
        self == self.intersection(other)
    }

    /// Get an iterator over the version indices contained in the set.
    #[inline]
    pub fn iter(self) -> impl Iterator<Item = VersionIndex> {
        (0..VersionIndex::MAX)
            .filter(move |v| self.0 & (2 << v) != 0)
            .map(|v| VersionIndex(v as u8))
    }

    /// Get the first version index of the set.
    #[inline]
    pub fn first(self) -> Option<VersionIndex> {
        if self != Self::empty() {
            Some(VersionIndex((self.0 >> 1).trailing_zeros() as u8))
        } else {
            None
        }
    }

    /// Get the last version index of the set.
    #[inline]
    pub fn last(self) -> Option<VersionIndex> {
        if self != Self::empty() {
            let v = VersionIndex::MAX - (self.0 >> 1).leading_zeros() as u64;
            Some(VersionIndex(v as u8))
        } else {
            None
        }
    }

    /// Count the number of version indices contained in the set.
    #[inline]
    pub fn count(self) -> usize {
        self.0.count_ones() as usize
    }
}
