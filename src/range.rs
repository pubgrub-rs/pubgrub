// SPDX-License-Identifier: MPL-2.0

//! Ranges are constraints defining sets of versions.
//!
//! Concretely, those constraints correspond to any set of versions
//! representable as the concatenation, union, and complement
//! of the ranges building blocks.
//!
//! Those building blocks are:
//!  - [empty()](Range::empty): the empty set
//!  - [full()](Range::full): the set of all possible versions
//!  - [singleton(v)](Range::singleton): the set containing only the version v
//!  - [higher_than(v)](Range::higher_than): the set defined by `v <= versions`
//!  - [strictly_higher_than(v)](Range::strictly_higher_than): the set defined by `v < versions`
//!  - [lower_than(v)](Range::lower_than): the set defined by `versions <= v`
//!  - [strictly_lower_than(v)](Range::strictly_lower_than): the set defined by `versions < v`
//!  - [between(v1, v2)](Range::between): the set defined by `v1 <= versions < v2`

use crate::{internal::small_vec::SmallVec, version_set::VersionSet};
use std::ops::RangeBounds;
use std::{
    cmp::Ordering,
    fmt::{Debug, Display, Formatter},
    ops::Bound::{self, Excluded, Included, Unbounded},
};

/// A Range represents multiple intervals of a continuous range of monotone increasing
/// values.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Range<V> {
    segments: SmallVec<Interval<V>>,
}

type Interval<V> = (Bound<V>, Bound<V>);

impl<V> Range<V> {
    /// Empty set of versions.
    pub fn empty() -> Self {
        Self {
            segments: SmallVec::empty(),
        }
    }

    /// Set of all possible versions
    pub fn full() -> Self {
        Self {
            segments: SmallVec::one((Unbounded, Unbounded)),
        }
    }

    /// Set of all versions higher or equal to some version
    pub fn higher_than(v: impl Into<V>) -> Self {
        Self {
            segments: SmallVec::one((Included(v.into()), Unbounded)),
        }
    }

    /// Set of all versions higher to some version
    pub fn strictly_higher_than(v: impl Into<V>) -> Self {
        Self {
            segments: SmallVec::one((Excluded(v.into()), Unbounded)),
        }
    }

    /// Set of all versions lower to some version
    pub fn strictly_lower_than(v: impl Into<V>) -> Self {
        Self {
            segments: SmallVec::one((Unbounded, Excluded(v.into()))),
        }
    }

    /// Set of all versions lower or equal to some version
    pub fn lower_than(v: impl Into<V>) -> Self {
        Self {
            segments: SmallVec::one((Unbounded, Included(v.into()))),
        }
    }

    /// Set of versions greater or equal to `v1` but less than `v2`.
    pub fn between(v1: impl Into<V>, v2: impl Into<V>) -> Self {
        Self {
            segments: SmallVec::one((Included(v1.into()), Excluded(v2.into()))),
        }
    }
}

impl<V: Clone> Range<V> {
    /// Set containing exactly one version
    pub fn singleton(v: impl Into<V>) -> Self {
        let v = v.into();
        Self {
            segments: SmallVec::one((Included(v.clone()), Included(v))),
        }
    }

    /// Set containing all versions expect one
    pub fn not_equal(v: impl Into<V>) -> Self {
        let v = v.into();
        Self {
            segments: SmallVec::Two([(Unbounded, Excluded(v.clone())), (Excluded(v), Unbounded)]),
        }
    }

    /// Returns the complement of this Range.
    pub fn complement(&self) -> Self {
        match self.segments.first() {
            // Complement of ∅ is ∞
            None => Self::full(),

            // Complement of ∞ is ∅
            Some((Unbounded, Unbounded)) => Self::empty(),

            // First high bound is +∞
            Some((Included(v), Unbounded)) => Self::strictly_lower_than(v.clone()),
            Some((Excluded(v), Unbounded)) => Self::lower_than(v.clone()),

            Some((Unbounded, Included(v))) => {
                Self::negate_segments(Excluded(v.clone()), &self.segments[1..])
            }
            Some((Unbounded, Excluded(v))) => {
                Self::negate_segments(Included(v.clone()), &self.segments[1..])
            }
            Some((Included(_), Included(_)))
            | Some((Included(_), Excluded(_)))
            | Some((Excluded(_), Included(_)))
            | Some((Excluded(_), Excluded(_))) => Self::negate_segments(Unbounded, &self.segments),
        }
    }

    /// Helper function performing the negation of intervals in segments.
    fn negate_segments(start: Bound<V>, segments: &[Interval<V>]) -> Self {
        let mut complement_segments: SmallVec<Interval<V>> = SmallVec::empty();
        let mut start = start;
        for (v1, v2) in segments {
            complement_segments.push((
                start,
                match v1 {
                    Included(v) => Excluded(v.clone()),
                    Excluded(v) => Included(v.clone()),
                    Unbounded => unreachable!(),
                },
            ));
            start = match v2 {
                Included(v) => Excluded(v.clone()),
                Excluded(v) => Included(v.clone()),
                Unbounded => Unbounded,
            }
        }
        if !matches!(start, Unbounded) {
            complement_segments.push((start, Unbounded));
        }

        Self {
            segments: complement_segments,
        }
    }
}

impl<V: Ord> Range<V> {
    /// Convert to something that can be used with
    /// [BTreeMap::range](std::collections::BTreeMap::range).
    /// All versions contained in self, will be in the output,
    /// but there may be versions in the output that are not contained in self.
    /// Returns None if the range is empty.
    pub fn bounding_range(&self) -> Option<(Bound<&V>, Bound<&V>)> {
        self.segments.first().map(|(start, _)| {
            let end = self
                .segments
                .last()
                .expect("if there is a first element, there must be a last element");
            (bound_as_ref(start), bound_as_ref(&end.1))
        })
    }

    /// Returns true if the this Range contains the specified value.
    pub fn contains(&self, v: &V) -> bool {
        if let Some(bounding_range) = self.bounding_range() {
            if !bounding_range.contains(v) {
                return false;
            }
        }

        for segment in self.segments.iter() {
            if match segment {
                (Unbounded, Unbounded) => true,
                (Unbounded, Included(end)) => v <= end,
                (Unbounded, Excluded(end)) => v < end,
                (Included(start), Unbounded) => v >= start,
                (Included(start), Included(end)) => v >= start && v <= end,
                (Included(start), Excluded(end)) => v >= start && v < end,
                (Excluded(start), Unbounded) => v > start,
                (Excluded(start), Included(end)) => v > start && v <= end,
                (Excluded(start), Excluded(end)) => v > start && v < end,
            } {
                return true;
            }
        }
        false
    }

    /// Construct a simple range from anything that impls [RangeBounds] like `v1..v2`.
    pub fn from_range_bounds<R, IV>(bounds: R) -> Self
    where
        R: RangeBounds<IV>,
        IV: Clone + Into<V>,
    {
        let start = match bounds.start_bound() {
            Included(v) => Included(v.clone().into()),
            Excluded(v) => Excluded(v.clone().into()),
            Unbounded => Unbounded,
        };
        let end = match bounds.end_bound() {
            Included(v) => Included(v.clone().into()),
            Excluded(v) => Excluded(v.clone().into()),
            Unbounded => Unbounded,
        };
        match (start, end) {
            (Included(a), Included(b)) if b < a => Self::empty(),
            (Excluded(a), Excluded(b)) if b < a => Self::empty(),
            (Included(a), Excluded(b)) if b <= a => Self::empty(),
            (Excluded(a), Included(b)) if b <= a => Self::empty(),
            (a, b) => Self {
                segments: SmallVec::one((a, b)),
            },
        }
    }
}

/// Implementation of [`Bound::as_ref`] which is currently marked as unstable.
fn bound_as_ref<V>(bound: &Bound<V>) -> Bound<&V> {
    match bound {
        Included(v) => Included(v),
        Excluded(v) => Excluded(v),
        Unbounded => Unbounded,
    }
}

impl<V: Ord + Clone> Range<V> {
    /// Computes the intersection of two sets of versions.
    pub fn intersection(&self, other: &Self) -> Self {
        let mut segments: SmallVec<Interval<V>> = SmallVec::empty();
        let mut left_iter = self.segments.iter();
        let mut right_iter = other.segments.iter();
        let mut left = left_iter.next();
        let mut right = right_iter.next();
        while let (Some((left_lower, left_upper)), Some((right_lower, right_upper))) = (left, right)
        {
            // Check if the left range completely smaller than the right range.
            if let (
                Included(left_upper_version) | Excluded(left_upper_version),
                Included(right_lower_version) | Excluded(right_lower_version),
            ) = (left_upper, right_lower)
            {
                match left_upper_version.cmp(right_lower_version) {
                    Ordering::Less => {
                        // Left range is disjoint from the right range.
                        left = left_iter.next();
                        continue;
                    }
                    Ordering::Equal => {
                        if !matches!((left_upper, right_lower), (Included(_), Included(_))) {
                            // Left and right are overlapping exactly, but one of the bounds is exclusive, therefor the ranges are disjoint
                            left = left_iter.next();
                            continue;
                        }
                    }
                    Ordering::Greater => {
                        // Left upper bound is greater than right lower bound, so the lower bound is the right lower bound
                    }
                }
            }
            // Check if the right range completely smaller than the left range.
            if let (
                Included(left_lower_version) | Excluded(left_lower_version),
                Included(right_upper_version) | Excluded(right_upper_version),
            ) = (left_lower, right_upper)
            {
                match right_upper_version.cmp(left_lower_version) {
                    Ordering::Less => {
                        // Right range is disjoint from the left range.
                        right = right_iter.next();
                        continue;
                    }
                    Ordering::Equal => {
                        if !matches!((right_upper, left_lower), (Included(_), Included(_))) {
                            // Left and right are overlapping exactly, but one of the bounds is exclusive, therefor the ranges are disjoint
                            right = right_iter.next();
                            continue;
                        }
                    }
                    Ordering::Greater => {
                        // Right upper bound is greater than left lower bound, so the lower bound is the left lower bound
                    }
                }
            }

            // At this point we know there is an overlap between the versions, find the lowest bound
            let lower = match (left_lower, right_lower) {
                (Unbounded, Included(_) | Excluded(_)) => right_lower.clone(),
                (Included(_) | Excluded(_), Unbounded) => left_lower.clone(),
                (Unbounded, Unbounded) => Unbounded,
                (Included(l) | Excluded(l), Included(r) | Excluded(r)) => match l.cmp(r) {
                    Ordering::Less => right_lower.clone(),
                    Ordering::Equal => match (left_lower, right_lower) {
                        (Included(_), Excluded(v)) => Excluded(v.clone()),
                        (Excluded(_), Excluded(v)) => Excluded(v.clone()),
                        (Excluded(v), Included(_)) => Excluded(v.clone()),
                        (Included(_), Included(v)) => Included(v.clone()),
                        _ => unreachable!(),
                    },
                    Ordering::Greater => left_lower.clone(),
                },
            };

            // At this point we know there is an overlap between the versions, find the lowest bound
            let upper = match (left_upper, right_upper) {
                (Unbounded, Included(_) | Excluded(_)) => {
                    right = right_iter.next();
                    right_upper.clone()
                }
                (Included(_) | Excluded(_), Unbounded) => {
                    left = left_iter.next();
                    left_upper.clone()
                }
                (Unbounded, Unbounded) => {
                    left = left_iter.next();
                    right = right_iter.next();
                    Unbounded
                }
                (Included(l) | Excluded(l), Included(r) | Excluded(r)) => match l.cmp(r) {
                    Ordering::Less => {
                        left = left_iter.next();
                        left_upper.clone()
                    }
                    Ordering::Equal => match (left_upper, right_upper) {
                        (Included(_), Excluded(v)) => {
                            right = right_iter.next();
                            Excluded(v.clone())
                        }
                        (Excluded(_), Excluded(v)) => {
                            left = left_iter.next();
                            right = right_iter.next();
                            Excluded(v.clone())
                        }
                        (Excluded(v), Included(_)) => {
                            left = left_iter.next();
                            Excluded(v.clone())
                        }
                        (Included(_), Included(v)) => {
                            left = left_iter.next();
                            right = right_iter.next();
                            Included(v.clone())
                        }
                        _ => unreachable!(),
                    },
                    Ordering::Greater => {
                        right = right_iter.next();
                        right_upper.clone()
                    }
                },
            };

            segments.push((lower, upper));
        }

        Self { segments }
    }
}

impl<T: Debug + Display + Clone + Eq + Ord> VersionSet for Range<T> {
    type V = T;

    fn empty() -> Self {
        Range::empty()
    }

    fn singleton(v: Self::V) -> Self {
        Range::singleton(v)
    }

    fn complement(&self) -> Self {
        Range::complement(self)
    }

    fn intersection(&self, other: &Self) -> Self {
        Range::intersection(self, other)
    }

    fn contains(&self, v: &Self::V) -> bool {
        Range::contains(self, v)
    }

    fn full() -> Self {
        Range::full()
    }
}

// REPORT ######################################################################

impl<V: Display + Eq> Display for Range<V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.segments.is_empty() {
            write!(f, "∅")?;
        } else {
            for (idx, segment) in self.segments.iter().enumerate() {
                if idx > 0 {
                    write!(f, ", ")?;
                }
                match segment {
                    (Unbounded, Unbounded) => write!(f, "*")?,
                    (Unbounded, Included(v)) => write!(f, "<={v}")?,
                    (Unbounded, Excluded(v)) => write!(f, "<{v}")?,
                    (Included(v), Unbounded) => write!(f, ">={v}")?,
                    (Included(v), Included(b)) => {
                        if v == b {
                            write!(f, "{v}")?
                        } else {
                            write!(f, ">={v},<={b}")?
                        }
                    }
                    (Included(v), Excluded(b)) => write!(f, ">={v}, <{b}")?,
                    (Excluded(v), Unbounded) => write!(f, ">{v}")?,
                    (Excluded(v), Included(b)) => write!(f, ">{v}, <={b}")?,
                    (Excluded(v), Excluded(b)) => write!(f, ">{v}, <{b}")?,
                };
            }
        }
        Ok(())
    }
}

// SERIALIZATION ###############################################################

#[cfg(feature = "serde")]
impl<'de, V: serde::Deserialize<'de>> serde::Deserialize<'de> for Range<V> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // This enables conversion from the "old" discrete implementation of `Range` to the new
        // bounded one.
        //
        // Serialization is always performed in the new format.
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum EitherInterval<V> {
            B(Bound<V>, Bound<V>),
            D(V, Option<V>),
        }

        let bounds: SmallVec<EitherInterval<V>> = serde::Deserialize::deserialize(deserializer)?;

        let mut segments = SmallVec::Empty;
        for i in bounds {
            match i {
                EitherInterval::B(l, r) => segments.push((l, r)),
                EitherInterval::D(l, Some(r)) => segments.push((Included(l), Excluded(r))),
                EitherInterval::D(l, None) => segments.push((Included(l), Unbounded)),
            }
        }

        Ok(Range { segments })
    }
}

// TESTS #######################################################################

#[cfg(test)]
pub mod tests {
    use proptest::prelude::*;
    use proptest::test_runner::TestRng;

    use super::*;

    pub fn strategy() -> impl Strategy<Value = Range<u32>> {
        prop::collection::vec(any::<u32>(), 0..10)
            .prop_map(|mut vec| {
                vec.sort_unstable();
                vec.dedup();
                vec
            })
            .prop_perturb(|vec, mut rng| {
                let mut segments = SmallVec::empty();
                let mut iter = vec.into_iter().peekable();
                if let Some(first) = iter.next() {
                    fn next_bound<I: Iterator<Item = u32>>(
                        iter: &mut I,
                        rng: &mut TestRng,
                    ) -> Bound<u32> {
                        if let Some(next) = iter.next() {
                            if rng.gen_bool(0.5) {
                                Included(next)
                            } else {
                                Excluded(next)
                            }
                        } else {
                            Unbounded
                        }
                    }

                    let start = if rng.gen_bool(0.3) {
                        Unbounded
                    } else {
                        if rng.gen_bool(0.5) {
                            Included(first)
                        } else {
                            Excluded(first)
                        }
                    };

                    let end = next_bound(&mut iter, &mut rng);
                    segments.push((start, end));

                    while iter.peek().is_some() {
                        let start = next_bound(&mut iter, &mut rng);
                        let end = next_bound(&mut iter, &mut rng);
                        segments.push((start, end));
                    }
                }
                return Range { segments };
            })
    }

    fn version_strat() -> impl Strategy<Value = u32> {
        any::<u32>()
    }

    proptest! {

        // Testing negate ----------------------------------

        #[test]
        fn negate_is_different(range in strategy()) {
            assert_ne!(range.complement(), range);
        }

        #[test]
        fn double_negate_is_identity(range in strategy()) {
            assert_eq!(range.complement().complement(), range);
        }

        #[test]
        fn negate_contains_opposite(range in strategy(), version in version_strat()) {
            assert_ne!(range.contains(&version), range.complement().contains(&version));
        }

        // Testing intersection ----------------------------

        #[test]
        fn intersection_is_symmetric(r1 in strategy(), r2 in strategy()) {
            assert_eq!(r1.intersection(&r2), r2.intersection(&r1));
        }

        #[test]
        fn intersection_with_any_is_identity(range in strategy()) {
            assert_eq!(Range::full().intersection(&range), range);
        }

        #[test]
        fn intersection_with_none_is_none(range in strategy()) {
            assert_eq!(Range::empty().intersection(&range), Range::empty());
        }

        #[test]
        fn intersection_is_idempotent(r1 in strategy(), r2 in strategy()) {
            assert_eq!(r1.intersection(&r2).intersection(&r2), r1.intersection(&r2));
        }

        #[test]
        fn intersection_is_associative(r1 in strategy(), r2 in strategy(), r3 in strategy()) {
            assert_eq!(r1.intersection(&r2).intersection(&r3), r1.intersection(&r2.intersection(&r3)));
        }

        #[test]
        fn intesection_of_complements_is_none(range in strategy()) {
            assert_eq!(range.complement().intersection(&range), Range::empty());
        }

        #[test]
        fn intesection_contains_both(r1 in strategy(), r2 in strategy(), version in version_strat()) {
            assert_eq!(r1.intersection(&r2).contains(&version), r1.contains(&version) && r2.contains(&version));
        }

        // Testing union -----------------------------------

        #[test]
        fn union_of_complements_is_any(range in strategy()) {
            assert_eq!(range.complement().union(&range), Range::full());
        }

        #[test]
        fn union_contains_either(r1 in strategy(), r2 in strategy(), version in version_strat()) {
            assert_eq!(r1.union(&r2).contains(&version), r1.contains(&version) || r2.contains(&version));
        }

        // Testing contains --------------------------------

        #[test]
        fn always_contains_exact(version in version_strat()) {
            assert!(Range::singleton(version).contains(&version));
        }

        #[test]
        fn contains_negation(range in strategy(), version in version_strat()) {
            assert_ne!(range.contains(&version), range.complement().contains(&version));
        }

        #[test]
        fn contains_intersection(range in strategy(), version in version_strat()) {
            assert_eq!(range.contains(&version), range.intersection(&Range::singleton(version)) != Range::empty());
        }

        #[test]
        fn contains_bounding_range(range in strategy(), version in version_strat()) {
            if range.contains(&version) {
                assert!(range.bounding_range().map(|b| b.contains(&version)).unwrap_or(false));
            }
        }

        #[test]
        fn from_range_bounds(range in any::<(Bound<u32>, Bound<u32>)>(), version in version_strat()) {
            let rv: Range<u32> = Range::from_range_bounds(range);
            assert_eq!(range.contains(&version), rv.contains(&version));
        }

        #[test]
        fn from_range_bounds_round_trip(range in any::<(Bound<u32>, Bound<u32>)>()) {
            let rv: Range<u32> = Range::from_range_bounds(range);
            let rv2: Range<u32> = rv.bounding_range().map(Range::from_range_bounds::<_, u32>).unwrap_or_else(Range::empty);
            assert_eq!(rv, rv2);
        }
    }
}
