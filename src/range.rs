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
//!
//! Ranges can be created from any type that implements [`Ord`] + [`Clone`].
//!
//! In order to advance the solver front, comparisons of versions sets are necessary in the algorithm.
//! To do those comparisons between two sets S1 and S2 we use the mathematical property that S1 ⊂ S2 if and only if S1 ∩ S2 == S1.
//! We can thus compute an intersection and evaluate an equality to answer if S1 is a subset of S2.
//! But this means that the implementation of equality must be correct semantically.
//! In practice, if equality is derived automatically, this means sets must have unique representations.
//!
//! By migrating from a custom representation for discrete sets in v0.2
//! to a generic bounded representation for continuous sets in v0.3
//! we are potentially breaking that assumption in two ways:
//!
//!  1. Minimal and maximal `Unbounded` values can be replaced by their equivalent if it exists.
//!  2. Simplifying adjacent bounds of discrete sets cannot be detected and automated in the generic intersection code.
//!
//! An example for each can be given when `T` is `u32`.
//! First, we can have both segments `S1 = (Unbounded, Included(42u32))` and `S2 = (Included(0), Included(42u32))`
//! that represent the same segment but are structurally different.
//! Thus, a derived equality check would answer `false` to `S1 == S2` while it's true.
//!
//! Second both segments `S1 = (Included(1), Included(5))` and `S2 = (Included(1), Included(3)) + (Included(4), Included(5))` are equal.
//! But without asking the user to provide a `bump` function for discrete sets,
//! the algorithm is not able tell that the space between the right `Included(3)` bound and the left `Included(4)` bound is empty.
//! Thus the algorithm is not able to reduce S2 to its canonical S1 form while computing sets operations like intersections in the generic code.
//!
//! This is likely to lead to user facing theoretically correct but practically nonsensical ranges,
//! like (Unbounded, Excluded(0)) or (Excluded(6), Excluded(7)).
//! In general nonsensical inputs often lead to hard to track bugs.
//! But as far as we can tell this should work in practice.
//! So for now this crate only provides an implementation for continuous ranges.
//! With the v0.3 api the user could choose to bring back the discrete implementation from v0.2, as documented in the guide.
//! If doing so regularly fixes bugs seen by users, we will bring it back into the core library.
//! If we do not see practical bugs, or we get a formal proof that the code cannot lead to error states, then we may remove this warning.

use crate::{internal::small_vec::SmallVec, version_set::VersionSet};
use std::cmp::Ordering;
use std::ops::RangeBounds;
use std::{
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
            (start.as_ref(), end.1.as_ref())
        })
    }

    /// Returns true if the this Range contains the specified value.
    pub fn contains(&self, v: &V) -> bool {
        for segment in self.segments.iter() {
            match within_bounds(v, segment) {
                Ordering::Less => return false,
                Ordering::Equal => return true,
                Ordering::Greater => (),
            }
        }
        false
    }

    /// Returns true if the this Range contains the specified values.
    ///
    /// The `versions` iterator must be sorted.
    /// Functionally equivalent to `versions.map(|v| self.contains(v))`.
    /// Except it runs in `O(size_of_range + len_of_versions)` not `O(size_of_range * len_of_versions)`
    pub fn contains_many<'s, I>(&'s self, versions: I) -> impl Iterator<Item = bool> + 's
    where
        I: Iterator<Item = &'s V> + 's,
        V: 's,
    {
        versions.scan(0, move |i, v| {
            while let Some(segment) = self.segments.get(*i) {
                match within_bounds(v, segment) {
                    Ordering::Less => return Some(false),
                    Ordering::Equal => return Some(true),
                    Ordering::Greater => *i += 1,
                }
            }
            Some(false)
        })
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
        if valid_segment(&start, &end) {
            Self {
                segments: SmallVec::one((start, end)),
            }
        } else {
            Self::empty()
        }
    }

    fn check_invariants(self) -> Self {
        if cfg!(debug_assertions) {
            for p in self.segments.as_slice().windows(2) {
                match (&p[0].1, &p[1].0) {
                    (Included(l_end), Included(r_start)) => assert!(l_end < r_start),
                    (Included(l_end), Excluded(r_start)) => assert!(l_end < r_start),
                    (Excluded(l_end), Included(r_start)) => assert!(l_end < r_start),
                    (Excluded(l_end), Excluded(r_start)) => assert!(l_end <= r_start),
                    (_, Unbounded) => panic!(),
                    (Unbounded, _) => panic!(),
                }
            }
            for (s, e) in self.segments.iter() {
                assert!(valid_segment(s, e));
            }
        }
        self
    }
}

fn within_bounds<V: PartialOrd>(v: &V, segment: &Interval<V>) -> Ordering {
    let below_lower_bound = match segment {
        (Excluded(start), _) => v <= start,
        (Included(start), _) => v < start,
        (Unbounded, _) => false,
    };
    if below_lower_bound {
        return Ordering::Less;
    }
    let below_upper_bound = match segment {
        (_, Unbounded) => true,
        (_, Included(end)) => v <= end,
        (_, Excluded(end)) => v < end,
    };
    if below_upper_bound {
        return Ordering::Equal;
    }
    Ordering::Greater
}

fn valid_segment<T: PartialOrd>(start: &Bound<T>, end: &Bound<T>) -> bool {
    match (start, end) {
        (Included(s), Included(e)) => s <= e,
        (Included(s), Excluded(e)) => s < e,
        (Excluded(s), Included(e)) => s < e,
        (Excluded(s), Excluded(e)) => s < e,
        (Unbounded, _) | (_, Unbounded) => true,
    }
}

/// Group adjacent versions locations.
///
/// ```text
/// [None, 3, 6, 7, None] -> [(3, 7)]
/// [3, 6, 7, None] -> [(None, 7)]
/// [3, 6, 7] -> [(None, None)]
/// [None, 1, 4, 7, None, None, None, 8, None, 9] -> [(1, 7), (8, 8), (9, None)]
/// ```
fn group_adjacent_locations(
    mut locations: impl Iterator<Item = Option<usize>>,
) -> impl Iterator<Item = (Option<usize>, Option<usize>)> {
    // If the first version matched, then the lower bound of that segment is not needed
    let mut seg = locations.next().flatten().map(|ver| (None, Some(ver)));
    std::iter::from_fn(move || {
        for ver in locations.by_ref() {
            if let Some(ver) = ver {
                // As long as were still matching versions, we keep merging into the currently matching segment
                seg = Some((seg.map_or(Some(ver), |(s, _)| s), Some(ver)));
            } else {
                // If we have found a version that doesn't match, then right the merge segment and prepare for a new one.
                if seg.is_some() {
                    return seg.take();
                }
            }
        }
        // If the last version matched, then write out the merged segment but the upper bound is not needed.
        seg.take().map(|(s, _)| (s, None))
    })
}

impl<V: Ord + Clone> Range<V> {
    /// Computes the union of this `Range` and another.
    pub fn union(&self, other: &Self) -> Self {
        self.complement()
            .intersection(&other.complement())
            .complement()
            .check_invariants()
    }

    /// Computes the intersection of two sets of versions.
    pub fn intersection(&self, other: &Self) -> Self {
        let mut output: SmallVec<Interval<V>> = SmallVec::empty();
        let mut left_iter = self.segments.iter().peekable();
        let mut right_iter = other.segments.iter().peekable();
        // By the definition of intersection any point that is matched by the output
        // must have a segment in each of the inputs that it matches.
        // Therefore, every segment in the output must be the intersection of a segment from each of the inputs.
        // It would be correct to do the "O(n^2)" thing, by computing the intersection of every segment from one input
        // with every segment of the other input, and sorting the result.
        // We can avoid the sorting by generating our candidate segments with an increasing `end` value.
        while let Some(((left_start, left_end), (right_start, right_end))) =
            left_iter.peek().zip(right_iter.peek())
        {
            // The next smallest `end` value is going to come from one of the inputs.
            let left_end_is_smaller = match (left_end, right_end) {
                (Included(l), Included(r))
                | (Excluded(l), Excluded(r))
                | (Excluded(l), Included(r)) => l <= r,

                (Included(l), Excluded(r)) => l < r,
                (_, Unbounded) => true,
                (Unbounded, _) => false,
            };
            // Now that we are processing `end` we will never have to process any segment smaller than that.
            // We can ensure that the input that `end` came from is larger than `end` by advancing it one step.
            // `end` is the smaller available input, so we know the other input is already larger than `end`.
            // Note: We can call `other_iter.next_if( == end)`, but the ends lining up is rare enough that
            // it does not end up being faster in practice.
            let (other_start, end) = if left_end_is_smaller {
                left_iter.next();
                (right_start, left_end)
            } else {
                right_iter.next();
                (left_start, right_end)
            };
            // `start` will either come from the input `end` came from or the other input, whichever one is larger.
            // The intersection is invalid if `start` > `end`.
            // But, we already know that the segments in our input are valid.
            // So we do not need to check if the `start`  from the input `end` came from is smaller then `end`.
            // If the `other_start` is larger than end, then the intersection will be invalid.
            if !valid_segment(other_start, end) {
                // Note: We can call `this_iter.next_if(!valid_segment(other_start, this_end))` in a loop.
                // But the checks make it slower for the benchmarked inputs.
                continue;
            }
            let start = match (left_start, right_start) {
                (Included(l), Included(r)) => Included(std::cmp::max(l, r)),
                (Excluded(l), Excluded(r)) => Excluded(std::cmp::max(l, r)),

                (Included(i), Excluded(e)) | (Excluded(e), Included(i)) => {
                    if i <= e {
                        Excluded(e)
                    } else {
                        Included(i)
                    }
                }
                (s, Unbounded) | (Unbounded, s) => s.as_ref(),
            };
            // Now we clone and push a new segment.
            // By dealing with references until now we ensure that NO cloning happens when we reject the segment.
            output.push((start.cloned(), end.clone()))
        }

        Self { segments: output }.check_invariants()
    }

    /// Returns a simpler Range that contains the same versions
    ///
    /// For every one of the Versions provided in versions the existing range and
    /// the simplified range will agree on whether it is contained.
    /// The simplified version may include or exclude versions that are not in versions as the implementation wishes.
    /// For example:
    ///  - If all the versions are contained in the original than the range will be simplified to `full`.
    ///  - If none of the versions are contained in the original than the range will be simplified to `empty`.
    ///
    /// If versions are not sorted the correctness of this function is not guaranteed.
    pub fn simplify<'v, I>(&self, versions: I) -> Self
    where
        I: Iterator<Item = &'v V> + 'v,
        V: 'v,
    {
        // Return the segment index in the range for each version in the range, None otherwise
        let version_locations = versions.scan(0, move |i, v| {
            while let Some(segment) = self.segments.get(*i) {
                match within_bounds(v, segment) {
                    Ordering::Less => return Some(None),
                    Ordering::Equal => return Some(Some(*i)),
                    Ordering::Greater => *i += 1,
                }
            }
            Some(None)
        });
        let kept_segments = group_adjacent_locations(version_locations);
        self.keep_segments(kept_segments)
    }

    /// Create a new range with a subset of segments at given location bounds.
    ///
    /// Each new segment is constructed from a pair of segments, taking the
    /// start of the first and the end of the second.
    fn keep_segments(
        &self,
        kept_segments: impl Iterator<Item = (Option<usize>, Option<usize>)>,
    ) -> Range<V> {
        let mut segments = SmallVec::Empty;
        for (s, e) in kept_segments {
            segments.push((
                s.map_or(Unbounded, |s| self.segments[s].0.clone()),
                e.map_or(Unbounded, |e| self.segments[e].1.clone()),
            ));
        }
        Self { segments }.check_invariants()
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

    fn union(&self, other: &Self) -> Self {
        Range::union(self, other)
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
                    write!(f, " | ")?;
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
                            write!(f, ">={v}, <={b}")?
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

    use super::*;

    /// Generate version sets from a random vector of deltas between bounds.
    /// Each bound is randomly inclusive or exclusive.
    pub fn strategy() -> impl Strategy<Value = Range<u32>> {
        (
            any::<bool>(),
            prop::collection::vec(any::<(u32, bool)>(), 1..10),
        )
            .prop_map(|(start_unbounded, deltas)| {
                let mut start = if start_unbounded {
                    Some(Unbounded)
                } else {
                    None
                };
                let mut largest: u32 = 0;
                let mut last_bound_was_inclusive = false;
                let mut segments = SmallVec::Empty;
                for (delta, inclusive) in deltas {
                    // Add the offset to the current bound
                    largest = match largest.checked_add(delta) {
                        Some(s) => s,
                        None => {
                            // Skip this offset, if it would result in a too large bound.
                            continue;
                        }
                    };

                    let current_bound = if inclusive {
                        Included(largest)
                    } else {
                        Excluded(largest)
                    };

                    // If we already have a start bound, the next offset defines the complete range.
                    // If we don't have a start bound, we have to generate one.
                    if let Some(start_bound) = start.take() {
                        // If the delta from the start bound is 0, the only authorized configuration is
                        // Included(x), Included(x)
                        if delta == 0 && !(matches!(start_bound, Included(_)) && inclusive) {
                            start = Some(start_bound);
                            continue;
                        }
                        last_bound_was_inclusive = inclusive;
                        segments.push((start_bound, current_bound));
                    } else {
                        // If the delta from the end bound of the last range is 0 and
                        // any of the last ending or current starting bound is inclusive,
                        // we skip the delta because they basically overlap.
                        if delta == 0 && (last_bound_was_inclusive || inclusive) {
                            continue;
                        }
                        start = Some(current_bound);
                    }
                }

                // If we still have a start bound, but didn't have enough deltas to complete another
                // segment, we add an unbounded upperbound.
                if let Some(start_bound) = start {
                    segments.push((start_bound, Unbounded));
                }

                Range { segments }.check_invariants()
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
            let rv: Range<_> = Range::from_range_bounds(range);
            assert_eq!(range.contains(&version), rv.contains(&version));
        }

        #[test]
        fn from_range_bounds_round_trip(range in any::<(Bound<u32>, Bound<u32>)>()) {
            let rv: Range<u32> = Range::from_range_bounds(range);
            let rv2: Range<u32> = rv.bounding_range().map(Range::from_range_bounds::<_, u32>).unwrap_or_else(Range::empty);
            assert_eq!(rv, rv2);
        }

        #[test]
        fn contains(range in strategy(), versions in proptest::collection::vec(version_strat(), ..30)) {
            for v in versions {
                assert_eq!(range.contains(&v), range.segments.iter().any(|s| RangeBounds::contains(s, &v)));
            }
        }

        #[test]
        fn contains_many(range in strategy(), mut versions in proptest::collection::vec(version_strat(), ..30)) {
            versions.sort();
            assert_eq!(versions.len(), range.contains_many(versions.iter()).count());
            for (a, b) in versions.iter().zip(range.contains_many(versions.iter())) {
                assert_eq!(range.contains(a), b);
            }
        }

        #[test]
        fn simplify(range in strategy(), mut versions in proptest::collection::vec(version_strat(), ..30)) {
            versions.sort();
            let simp = range.simplify(versions.iter());

            for v in versions {
                assert_eq!(range.contains(&v), simp.contains(&v));
            }
            assert!(simp.segments.len() <= range.segments.len())
        }
    }
}
