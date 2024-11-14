// SPDX-License-Identifier: MPL-2.0

//! This crate contains a performance-optimized type for generic version ranges and operations on
//! them.
//!
//! [`Ranges`] can represent version selectors such as `(>=1, <2) OR (==3) OR (>4)`. Internally,
//! it is an ordered list of contiguous intervals (segments) with inclusive, exclusive or open-ended
//! ends, similar to a `Vec<(Bound<T>, Bound<T>)>`.
//!
//! You can construct a basic range from one of the following build blocks. All other ranges are
//! concatenation, union, and complement of these basic ranges.
//!  - [empty()](Ranges::empty): No version
//!  - [full()](Ranges::full): All versions
//!  - [singleton(v)](Ranges::singleton): Only the version v exactly
//!  - [higher_than(v)](Ranges::higher_than): All versions `v <= versions`
//!  - [strictly_higher_than(v)](Ranges::strictly_higher_than): All versions `v < versions`
//!  - [lower_than(v)](Ranges::lower_than): All versions `versions <= v`
//!  - [strictly_lower_than(v)](Ranges::strictly_lower_than): All versions `versions < v`
//!  - [between(v1, v2)](Ranges::between): All versions `v1 <= versions < v2`
//!
//! [`Ranges`] is generic over any type that implements [`Ord`] + [`Clone`] and can represent all
//! kinds of slices with ordered coordinates, not just version ranges. While built as a
//! performance-critical piece of [pubgrub](https://github.com/pubgrub-rs/pubgrub), it can be
//! adopted for other domains, too.
//!
//! Note that there are limitations to the equality implementation: Given a `Ranges<u32>`,
//! the segments `(Unbounded, Included(42u32))` and `(Included(0), Included(42u32))` as well as
//! `(Included(1), Included(5))` and  `(Included(1), Included(3)) + (Included(4), Included(5))`
//! are reported as unequal, even though the match the same versions: We can't tell that there isn't
//! a version between `0` and `-inf` or `3` and `4` respectively.
//!
//! ## Optional features
//!
//! * `serde`: serialization and deserialization for the version range, given that the version type
//!   also supports it.
//! * `proptest`: Exports are proptest strategy for [`Ranges<u32>`].

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Bound::{self, Excluded, Included, Unbounded};
use std::ops::RangeBounds;

#[cfg(any(feature = "proptest", test))]
use proptest::prelude::*;
use smallvec::{smallvec, SmallVec};

/// Ranges represents multiple intervals of a continuous range of monotone increasing values.
///
/// Internally, [`Ranges`] are an ordered list of segments, where segment is a bounds pair.
///
/// Invariants:
/// 1. The segments are sorted, from lowest to highest (through `Ord`).
/// 2. Each segment contains at least one version (start < end).
/// 3. There is at least one version between two segments.
///
/// These ensure that equivalent instances have an identical representation, which is important
/// for `Eq` and `Hash`. Note that this representation cannot strictly guaranty equality of
/// [`Ranges`] with equality of its representation without also knowing the nature of the underlying
/// versions. In particular, if the version space is discrete, different representations, using
/// different types of bounds (exclusive/inclusive) may correspond to the same set of existing
/// versions. It is a tradeoff we acknowledge, but which makes representations of continuous version
/// sets more accessible, to better handle features like pre-releases and other types of version
/// modifiers. For example, `[(Included(3u32), Excluded(7u32))]` and
/// `[(Included(3u32), Included(6u32))]` refer to the same version set, since there is no version
/// between 6 and 7, which this crate doesn't know about.

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Ranges<V> {
    /// Profiling in <https://github.com/pubgrub-rs/pubgrub/pull/262#discussion_r1804276278> showed
    /// that a single stack entry is the most efficient. This is most likely due to `Interval<V>`
    /// being large.
    segments: SmallVec<[Interval<V>; 1]>,
}

// TODO: Replace the tuple type with a custom enum inlining the bounds to reduce the type's size.
type Interval<V> = (Bound<V>, Bound<V>);

impl<V> Ranges<V> {
    /// Empty set of versions.
    pub fn empty() -> Self {
        Self {
            segments: SmallVec::new(),
        }
    }

    /// Set of all possible versions
    pub fn full() -> Self {
        Self {
            segments: smallvec![(Unbounded, Unbounded)],
        }
    }

    /// Set of all versions higher or equal to some version
    pub fn higher_than(v: impl Into<V>) -> Self {
        Self {
            segments: smallvec![(Included(v.into()), Unbounded)],
        }
    }

    /// Set of all versions higher to some version
    pub fn strictly_higher_than(v: impl Into<V>) -> Self {
        Self {
            segments: smallvec![(Excluded(v.into()), Unbounded)],
        }
    }

    /// Set of all versions lower to some version
    pub fn strictly_lower_than(v: impl Into<V>) -> Self {
        Self {
            segments: smallvec![(Unbounded, Excluded(v.into()))],
        }
    }

    /// Set of all versions lower or equal to some version
    pub fn lower_than(v: impl Into<V>) -> Self {
        Self {
            segments: smallvec![(Unbounded, Included(v.into()))],
        }
    }

    /// Set of versions greater or equal to `v1` but less than `v2`.
    pub fn between(v1: impl Into<V>, v2: impl Into<V>) -> Self {
        Self {
            segments: smallvec![(Included(v1.into()), Excluded(v2.into()))],
        }
    }

    /// Whether the set is empty, i.e. it has not ranges
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

impl<V: Clone> Ranges<V> {
    /// Set containing exactly one version
    pub fn singleton(v: impl Into<V>) -> Self {
        let v = v.into();
        Self {
            segments: smallvec![(Included(v.clone()), Included(v))],
        }
    }

    /// Returns the complement, which contains everything not included in `self`.
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
        let mut complement_segments = SmallVec::new();
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

impl<V: Ord> Ranges<V> {
    /// If self contains exactly a single version, return it, otherwise, return [None].
    pub fn as_singleton(&self) -> Option<&V> {
        match self.segments.as_slice() {
            [(Included(v1), Included(v2))] => {
                if v1 == v2 {
                    Some(v1)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

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

    /// Returns true if self contains the specified value.
    pub fn contains(&self, version: &V) -> bool {
        self.segments
            .binary_search_by(|segment| {
                // We have to reverse because we need the segment wrt to the version, while
                // within bounds tells us the version wrt to the segment.
                within_bounds(version, segment).reverse()
            })
            // An equal interval is one that contains the version
            .is_ok()
    }

    /// Returns true if self contains the specified values.
    ///
    /// The `versions` iterator must be sorted.
    /// Functionally equivalent to `versions.map(|v| self.contains(v))`.
    /// Except it runs in `O(size_of_range + len_of_versions)` not `O(size_of_range * len_of_versions)`
    pub fn contains_many<'s, I, BV>(&'s self, versions: I) -> impl Iterator<Item = bool> + 's
    where
        I: Iterator<Item = BV> + 's,
        BV: Borrow<V> + 's,
    {
        #[cfg(debug_assertions)]
        let mut last: Option<BV> = None;
        versions.scan(0, move |i, v| {
            #[cfg(debug_assertions)]
            {
                if let Some(l) = last.as_ref() {
                    assert!(
                        l.borrow() <= v.borrow(),
                        "`contains_many` `versions` argument incorrectly sorted"
                    );
                }
            }
            while let Some(segment) = self.segments.get(*i) {
                match within_bounds(v.borrow(), segment) {
                    Ordering::Less => return Some(false),
                    Ordering::Equal => return Some(true),
                    Ordering::Greater => *i += 1,
                }
            }
            #[cfg(debug_assertions)]
            {
                last = Some(v);
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
                segments: smallvec![(start, end)],
            }
        } else {
            Self::empty()
        }
    }

    /// See [`Ranges`] for the invariants checked.
    fn check_invariants(self) -> Self {
        if cfg!(debug_assertions) {
            for p in self.segments.as_slice().windows(2) {
                assert!(end_before_start_with_gap(&p[0].1, &p[1].0));
            }
            for (s, e) in self.segments.iter() {
                assert!(valid_segment(s, e));
            }
        }
        self
    }
}

/// Implementing `PartialOrd` for start `Bound` of an interval.
///
/// Legend: `∞` is unbounded, `[1,2]` is `>=1,<=2`, `]1,2[` is `>1,<2`.
///
/// ```text
/// left:   ∞-------]
/// right:    [-----]
/// left is smaller, since it starts earlier.
///
/// left:   [-----]
/// right:  ]-----]
/// left is smaller, since it starts earlier.
/// ```
fn cmp_bounds_start<V: PartialOrd>(left: Bound<&V>, right: Bound<&V>) -> Option<Ordering> {
    Some(match (left, right) {
        // left:   ∞-----
        // right:  ∞-----
        (Unbounded, Unbounded) => Ordering::Equal,
        // left:     [---
        // right:  ∞-----
        (Included(_left), Unbounded) => Ordering::Greater,
        // left:     ]---
        // right:  ∞-----
        (Excluded(_left), Unbounded) => Ordering::Greater,
        // left:   ∞-----
        // right:    [---
        (Unbounded, Included(_right)) => Ordering::Less,
        // left:   [----- OR [----- OR   [-----
        // right:    [--- OR [----- OR [---
        (Included(left), Included(right)) => left.partial_cmp(right)?,
        (Excluded(left), Included(right)) => match left.partial_cmp(right)? {
            // left:   ]-----
            // right:    [---
            Ordering::Less => Ordering::Less,
            // left:   ]-----
            // right:  [---
            Ordering::Equal => Ordering::Greater,
            // left:     ]---
            // right:  [-----
            Ordering::Greater => Ordering::Greater,
        },
        // left:   ∞-----
        // right:    ]---
        (Unbounded, Excluded(_right)) => Ordering::Less,
        (Included(left), Excluded(right)) => match left.partial_cmp(right)? {
            // left:   [-----
            // right:    ]---
            Ordering::Less => Ordering::Less,
            // left:   [-----
            // right:  ]---
            Ordering::Equal => Ordering::Less,
            // left:     [---
            // right:  ]-----
            Ordering::Greater => Ordering::Greater,
        },
        // left:   ]----- OR ]----- OR   ]---
        // right:    ]--- OR ]----- OR ]-----
        (Excluded(left), Excluded(right)) => left.partial_cmp(right)?,
    })
}

/// Implementing `PartialOrd` for end `Bound` of an interval.
///
/// We flip the unbounded ranges from `-∞` to `∞`, while `V`-valued bounds checks remain the same.
///
/// Legend: `∞` is unbounded, `[1,2]` is `>=1,<=2`, `]1,2[` is `>1,<2`.
///
/// ```text
/// left:   [--------∞
/// right:  [-----]
/// left is greater, since it starts earlier.
///
/// left:   [-----[
/// right:  [-----]
/// left is smaller, since it ends earlier.
/// ```
fn cmp_bounds_end<V: PartialOrd>(left: Bound<&V>, right: Bound<&V>) -> Option<Ordering> {
    Some(match (left, right) {
        // left:   -----∞
        // right:  -----∞
        (Unbounded, Unbounded) => Ordering::Equal,
        // left:   ---]
        // right:  -----∞
        (Included(_left), Unbounded) => Ordering::Less,
        // left:   ---[
        // right:  -----∞
        (Excluded(_left), Unbounded) => Ordering::Less,
        // left:  -----∞
        // right: ---]
        (Unbounded, Included(_right)) => Ordering::Greater,
        // left:   -----] OR -----] OR ---]
        // right:    ---] OR -----] OR -----]
        (Included(left), Included(right)) => left.partial_cmp(right)?,
        (Excluded(left), Included(right)) => match left.partial_cmp(right)? {
            // left:   ---[
            // right:  -----]
            Ordering::Less => Ordering::Less,
            // left:   -----[
            // right:  -----]
            Ordering::Equal => Ordering::Less,
            // left:   -----[
            // right:  ---]
            Ordering::Greater => Ordering::Greater,
        },
        (Unbounded, Excluded(_right)) => Ordering::Greater,
        (Included(left), Excluded(right)) => match left.partial_cmp(right)? {
            // left:   ---]
            // right:  -----[
            Ordering::Less => Ordering::Less,
            // left:   -----]
            // right:  -----[
            Ordering::Equal => Ordering::Greater,
            // left:   -----]
            // right:  ---[
            Ordering::Greater => Ordering::Greater,
        },
        // left:   -----[ OR -----[ OR ---[
        // right:  ---[   OR -----[ OR -----[
        (Excluded(left), Excluded(right)) => left.partial_cmp(right)?,
    })
}

impl<V: PartialOrd> PartialOrd for Ranges<V> {
    /// A simple ordering scheme where we zip the segments and compare all bounds in order. If all
    /// bounds are equal, the longer range is considered greater. (And if all zipped bounds are
    /// equal and we have the same number of segments, the ranges are equal).
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        for (left, right) in self.segments.iter().zip(other.segments.iter()) {
            let start_cmp = cmp_bounds_start(left.start_bound(), right.start_bound())?;
            if start_cmp != Ordering::Equal {
                return Some(start_cmp);
            }
            let end_cmp = cmp_bounds_end(left.end_bound(), right.end_bound())?;
            if end_cmp != Ordering::Equal {
                return Some(end_cmp);
            }
        }
        Some(self.segments.len().cmp(&other.segments.len()))
    }
}

impl<V: Ord> Ord for Ranges<V> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other)
            .expect("PartialOrd must be `Some(Ordering)` for types that implement `Ord`")
    }
}

/// The ordering of the version wrt to the interval.
/// ```text
///      |-------|
///   ^      ^      ^
///   less   equal  greater
/// ```
fn within_bounds<V: PartialOrd>(version: &V, segment: &Interval<V>) -> Ordering {
    let below_lower_bound = match segment {
        (Excluded(start), _) => version <= start,
        (Included(start), _) => version < start,
        (Unbounded, _) => false,
    };
    if below_lower_bound {
        return Ordering::Less;
    }
    let below_upper_bound = match segment {
        (_, Unbounded) => true,
        (_, Included(end)) => version <= end,
        (_, Excluded(end)) => version < end,
    };
    if below_upper_bound {
        return Ordering::Equal;
    }
    Ordering::Greater
}

/// A valid segment is one where at least one version fits between start and end
fn valid_segment<T: PartialOrd>(start: &Bound<T>, end: &Bound<T>) -> bool {
    match (start, end) {
        // Singleton interval are allowed
        (Included(s), Included(e)) => s <= e,
        (Included(s), Excluded(e)) => s < e,
        (Excluded(s), Included(e)) => s < e,
        (Excluded(s), Excluded(e)) => s < e,
        (Unbounded, _) | (_, Unbounded) => true,
    }
}

/// The end of one interval is before the start of the next one, so they can't be concatenated
/// into a single interval. The `union` method calling with both intervals and then the intervals
/// switched. If either is true, the intervals are separate in the union and if both are false, they
/// are merged.
/// ```text
/// True for these two:
///  |----|
///                |-----|
///       ^ end    ^ start
/// False for these two:
///  |----|
///     |-----|
/// Here it depends: If they both exclude the position they share, there is a version in between
/// them that blocks concatenation
///  |----|
///       |-----|
/// ```
fn end_before_start_with_gap<V: PartialOrd>(end: &Bound<V>, start: &Bound<V>) -> bool {
    match (end, start) {
        (_, Unbounded) => false,
        (Unbounded, _) => false,
        (Included(left), Included(right)) => left < right,
        (Included(left), Excluded(right)) => left < right,
        (Excluded(left), Included(right)) => left < right,
        (Excluded(left), Excluded(right)) => left <= right,
    }
}

fn left_start_is_smaller<V: PartialOrd>(left: Bound<V>, right: Bound<V>) -> bool {
    match (left, right) {
        (Unbounded, _) => true,
        (_, Unbounded) => false,
        (Included(l), Included(r)) => l <= r,
        (Excluded(l), Excluded(r)) => l <= r,
        (Included(l), Excluded(r)) => l <= r,
        (Excluded(l), Included(r)) => l < r,
    }
}

fn left_end_is_smaller<V: PartialOrd>(left: Bound<V>, right: Bound<V>) -> bool {
    match (left, right) {
        (_, Unbounded) => true,
        (Unbounded, _) => false,
        (Included(l), Included(r)) => l <= r,
        (Excluded(l), Excluded(r)) => l <= r,
        (Excluded(l), Included(r)) => l <= r,
        (Included(l), Excluded(r)) => l < r,
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

impl<V: Ord + Clone> Ranges<V> {
    /// Computes the union of this `Ranges` and another.
    pub fn union(&self, other: &Self) -> Self {
        let mut output = SmallVec::new();
        let mut accumulator: Option<(&Bound<_>, &Bound<_>)> = None;
        let mut left_iter = self.segments.iter().peekable();
        let mut right_iter = other.segments.iter().peekable();
        loop {
            let smaller_interval = match (left_iter.peek(), right_iter.peek()) {
                (Some((left_start, left_end)), Some((right_start, right_end))) => {
                    if left_start_is_smaller(left_start.as_ref(), right_start.as_ref()) {
                        left_iter.next();
                        (left_start, left_end)
                    } else {
                        right_iter.next();
                        (right_start, right_end)
                    }
                }
                (Some((left_start, left_end)), None) => {
                    left_iter.next();
                    (left_start, left_end)
                }
                (None, Some((right_start, right_end))) => {
                    right_iter.next();
                    (right_start, right_end)
                }
                (None, None) => break,
            };

            if let Some(accumulator_) = accumulator {
                if end_before_start_with_gap(accumulator_.1, smaller_interval.0) {
                    output.push((accumulator_.0.clone(), accumulator_.1.clone()));
                    accumulator = Some(smaller_interval);
                } else {
                    let accumulator_end = match (accumulator_.1, smaller_interval.1) {
                        (_, Unbounded) | (Unbounded, _) => &Unbounded,
                        (Included(l), Excluded(r) | Included(r)) if l == r => accumulator_.1,
                        (Included(l) | Excluded(l), Included(r) | Excluded(r)) => {
                            if l > r {
                                accumulator_.1
                            } else {
                                smaller_interval.1
                            }
                        }
                    };
                    accumulator = Some((accumulator_.0, accumulator_end));
                }
            } else {
                accumulator = Some(smaller_interval)
            }
        }

        if let Some(accumulator) = accumulator {
            output.push((accumulator.0.clone(), accumulator.1.clone()));
        }

        Self { segments: output }.check_invariants()
    }

    /// Computes the intersection of two sets of versions.
    pub fn intersection(&self, other: &Self) -> Self {
        let mut output = SmallVec::new();
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
            let left_end_is_smaller = left_end_is_smaller(left_end.as_ref(), right_end.as_ref());
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
            // So we do not need to check if the `start` from the input `end` came from is smaller than `end`.
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

    /// Return true if there can be no `V` so that `V` is contained in both `self` and `other`.
    ///
    /// Note that we don't know that set of all existing `V`s here, so we only check if the segments
    /// are disjoint, not if no version is contained in both.
    pub fn is_disjoint(&self, other: &Self) -> bool {
        // The operation is symmetric
        let mut left_iter = self.segments.iter().peekable();
        let mut right_iter = other.segments.iter().peekable();

        while let Some((left, right)) = left_iter.peek().zip(right_iter.peek()) {
            if !valid_segment(&right.start_bound(), &left.end_bound()) {
                left_iter.next();
            } else if !valid_segment(&left.start_bound(), &right.end_bound()) {
                right_iter.next();
            } else {
                return false;
            }
        }

        // The remaining element(s) can't intersect anymore
        true
    }

    /// Return true if any `V` that is contained in `self` is also contained in `other`.
    ///
    /// Note that we don't know that set of all existing `V`s here, so we only check if all
    /// segments `self` are contained in a segment of `other`.
    pub fn subset_of(&self, other: &Self) -> bool {
        let mut containing_iter = other.segments.iter();
        let mut subset_iter = self.segments.iter();
        let Some(mut containing_elem) = containing_iter.next() else {
            // As long as we have subset elements, we need containing elements
            return subset_iter.next().is_none();
        };

        for subset_elem in subset_iter {
            // Check if the current containing element ends before the subset element.
            // There needs to be another containing element for our subset element in this case.
            while !valid_segment(&subset_elem.start_bound(), &containing_elem.end_bound()) {
                if let Some(containing_elem_) = containing_iter.next() {
                    containing_elem = containing_elem_;
                } else {
                    return false;
                };
            }

            let start_contained =
                left_start_is_smaller(containing_elem.start_bound(), subset_elem.start_bound());

            if !start_contained {
                // The start element is not contained
                return false;
            }

            let end_contained =
                left_end_is_smaller(subset_elem.end_bound(), containing_elem.end_bound());

            if !end_contained {
                // The end element is not contained
                return false;
            }
        }

        true
    }

    /// Returns a simpler representation that contains the same versions.
    ///
    /// For every one of the Versions provided in versions the existing range and the simplified range will agree on whether it is contained.
    /// The simplified version may include or exclude versions that are not in versions as the implementation wishes.
    ///
    /// If none of the versions are contained in the original than the range will be returned unmodified.
    /// If the range includes a single version, it will be returned unmodified.
    /// If all the versions are contained in the original than the range will be simplified to `full`.
    ///
    /// If the given versions are not sorted the correctness of this function is not guaranteed.
    pub fn simplify<'s, I, BV>(&self, versions: I) -> Self
    where
        I: Iterator<Item = BV> + 's,
        BV: Borrow<V> + 's,
    {
        // Do not simplify singletons
        if self.as_singleton().is_some() {
            return self.clone();
        }

        #[cfg(debug_assertions)]
        let mut last: Option<BV> = None;
        // Return the segment index in the range for each version in the range, None otherwise
        let version_locations = versions.scan(0, move |i, v| {
            #[cfg(debug_assertions)]
            {
                if let Some(l) = last.as_ref() {
                    assert!(
                        l.borrow() <= v.borrow(),
                        "`simplify` `versions` argument incorrectly sorted"
                    );
                }
            }
            while let Some(segment) = self.segments.get(*i) {
                match within_bounds(v.borrow(), segment) {
                    Ordering::Less => return Some(None),
                    Ordering::Equal => return Some(Some(*i)),
                    Ordering::Greater => *i += 1,
                }
            }
            #[cfg(debug_assertions)]
            {
                last = Some(v);
            }
            Some(None)
        });
        let mut kept_segments = group_adjacent_locations(version_locations).peekable();

        // Do not return null sets
        if kept_segments.peek().is_none() {
            return self.clone();
        }

        self.keep_segments(kept_segments)
    }

    /// Create a new range with a subset of segments at given location bounds.
    ///
    /// Each new segment is constructed from a pair of segments, taking the
    /// start of the first and the end of the second.
    fn keep_segments(
        &self,
        kept_segments: impl Iterator<Item = (Option<usize>, Option<usize>)>,
    ) -> Ranges<V> {
        let mut segments = SmallVec::new();
        for (s, e) in kept_segments {
            segments.push((
                s.map_or(Unbounded, |s| self.segments[s].0.clone()),
                e.map_or(Unbounded, |e| self.segments[e].1.clone()),
            ));
        }
        Self { segments }.check_invariants()
    }

    /// Iterate over the parts of the range.
    pub fn iter(&self) -> impl Iterator<Item = (&Bound<V>, &Bound<V>)> {
        self.segments.iter().map(|(start, end)| (start, end))
    }
}

// Newtype to avoid leaking our internal representation.
pub struct RangesIter<V>(smallvec::IntoIter<[Interval<V>; 1]>);

impl<V> Iterator for RangesIter<V> {
    type Item = Interval<V>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0.len(), Some(self.0.len()))
    }
}

impl<V> ExactSizeIterator for RangesIter<V> {}

impl<V> DoubleEndedIterator for RangesIter<V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back()
    }
}

impl<V> IntoIterator for Ranges<V> {
    type Item = (Bound<V>, Bound<V>);
    // Newtype to avoid leaking our internal representation.
    type IntoIter = RangesIter<V>;

    fn into_iter(self) -> Self::IntoIter {
        RangesIter(self.segments.into_iter())
    }
}

impl<V: Ord> FromIterator<(Bound<V>, Bound<V>)> for Ranges<V> {
    /// Constructor from arbitrary, unsorted and potentially overlapping ranges.
    ///
    /// This is equivalent, but faster, to computing the [`Ranges::union`] of the
    /// [`Ranges::from_range_bounds`] of each segment.
    fn from_iter<T: IntoIterator<Item = (Bound<V>, Bound<V>)>>(iter: T) -> Self {
        // We have three constraints we need to fulfil:
        // 1. The segments are sorted, from lowest to highest (through `Ord`): By sorting.
        // 2. Each segment contains at least one version (start < end): By skipping invalid
        //    segments.
        // 3. There is at least one version between two segments: By merging overlapping elements.
        //
        // Technically, the implementation has a O(n²) worst case complexity since we're inserting
        // and removing. This has two motivations: One is that we don't have any performance
        // critical usages of this method as of this writing, so we have no real world benchmark.
        // The other is that we get the elements from an iterator, so to avoid moving elements
        // around we would first need to build a different, sorted collection with extra
        // allocation(s), before we could build our real segments. --Konsti

        // For this implementation, we choose to only build a single smallvec and insert or remove
        // in it, instead of e.g. collecting the segments into a sorted datastructure first and then
        // construction the second smallvec without shifting.
        let mut segments: SmallVec<[Interval<V>; 1]> = SmallVec::new();

        for segment in iter {
            if !valid_segment(&segment.start_bound(), &segment.end_bound()) {
                continue;
            }
            // Find where to insert the new segment
            let insertion_point = segments.partition_point(|elem: &Interval<V>| {
                cmp_bounds_start(elem.start_bound(), segment.start_bound())
                    .unwrap()
                    .is_lt()
            });
            // Is it overlapping with the previous segment?
            let previous_overlapping = insertion_point > 0
                && !end_before_start_with_gap(
                    &segments[insertion_point - 1].end_bound(),
                    &segment.start_bound(),
                );

            // Is it overlapping with the following segment? We'll check if there's more than one
            // overlap later.
            let next_overlapping = insertion_point < segments.len()
                && !end_before_start_with_gap(
                    &segment.end_bound(),
                    &segments[insertion_point].start_bound(),
                );

            match (previous_overlapping, next_overlapping) {
                (true, true) => {
                    // previous:  |------|
                    // segment:       |------|
                    // following:          |------|
                    // final:     |---------------|
                    //
                    // OR
                    //
                    // previous:  |------|
                    // segment:       |-----------|
                    // following:          |----|
                    // final:     |---------------|
                    //
                    // OR
                    //
                    // previous:  |------|
                    // segment:       |----------------|
                    // following:          |----|   |------|
                    // final:     |------------------------|
                    // We merge all three segments into one, which is effectively removing one of
                    // two previously inserted and changing the bounds on the other.

                    // Remove all elements covered by the final element
                    let mut following = segments.remove(insertion_point);
                    while insertion_point < segments.len()
                        && !end_before_start_with_gap(
                            &segment.end_bound(),
                            &segments[insertion_point].start_bound(),
                        )
                    {
                        following = segments.remove(insertion_point);
                    }

                    // Set end to max(segment.end, <last overlapping segment>.end)
                    if cmp_bounds_end(segment.end_bound(), following.end_bound())
                        .unwrap()
                        .is_lt()
                    {
                        segments[insertion_point - 1].1 = following.1;
                    } else {
                        segments[insertion_point - 1].1 = segment.1;
                    }
                }
                (true, false) => {
                    // previous:  |------|
                    // segment:       |------|
                    // following:                |------|
                    //
                    // OR
                    //
                    // previous:  |----------|
                    // segment:       |---|
                    // following:                |------|
                    //
                    // final:     |----------|   |------|
                    // We can reuse the existing element by extending it.

                    // Set end to max(segment.end, <previous>.end)
                    if cmp_bounds_end(
                        segments[insertion_point - 1].end_bound(),
                        segment.end_bound(),
                    )
                    .unwrap()
                    .is_lt()
                    {
                        segments[insertion_point - 1].1 = segment.1;
                    }
                }
                (false, true) => {
                    // previous:  |------|
                    // segment:             |------|
                    // following:               |------|
                    // final:    |------|   |----------|
                    //
                    // OR
                    //
                    // previous:  |------|
                    // segment:             |----------|
                    // following:               |---|
                    // final:    |------|   |----------|
                    //
                    // OR
                    //
                    // previous:  |------|
                    // segment:             |------------|
                    // following:               |---|  |------|
                    //
                    // final:    |------|   |-----------------|
                    // We can reuse the existing element by extending it.

                    // Remove all fully covered segments so the next element is the last one that
                    // overlaps.
                    while insertion_point + 1 < segments.len()
                        && !end_before_start_with_gap(
                            &segment.end_bound(),
                            &segments[insertion_point + 1].start_bound(),
                        )
                    {
                        // We know that the one after also overlaps, so we can drop the current
                        // following.
                        segments.remove(insertion_point);
                    }

                    // Set end to max(segment.end, <last overlapping segment>.end)
                    if cmp_bounds_end(segments[insertion_point].end_bound(), segment.end_bound())
                        .unwrap()
                        .is_lt()
                    {
                        segments[insertion_point].1 = segment.1;
                    }
                    segments[insertion_point].0 = segment.0;
                }
                (false, false) => {
                    // previous:  |------|
                    // segment:             |------|
                    // following:                      |------|
                    //
                    // final:    |------|   |------|   |------|

                    // This line is O(n), which makes the algorithm O(n²), but it should be good
                    // enough for now.
                    segments.insert(insertion_point, segment);
                }
            }
        }

        Self { segments }.check_invariants()
    }
}

// REPORT ######################################################################

impl<V: Display + Eq> Display for Ranges<V> {
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
impl<'de, V: serde::Deserialize<'de>> serde::Deserialize<'de> for Ranges<V> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // This enables conversion from the "old" discrete implementation of `Ranges` to the new
        // bounded one.
        //
        // Serialization is always performed in the new format.
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum EitherInterval<V> {
            B(Bound<V>, Bound<V>),
            D(V, Option<V>),
        }

        let bounds: SmallVec<[EitherInterval<V>; 2]> =
            serde::Deserialize::deserialize(deserializer)?;

        let mut segments = SmallVec::new();
        for i in bounds {
            match i {
                EitherInterval::B(l, r) => segments.push((l, r)),
                EitherInterval::D(l, Some(r)) => segments.push((Included(l), Excluded(r))),
                EitherInterval::D(l, None) => segments.push((Included(l), Unbounded)),
            }
        }

        Ok(Ranges { segments })
    }
}

/// Generate version sets from a random vector of deltas between randomly inclusive or exclusive
/// bounds.
#[cfg(any(feature = "proptest", test))]
pub fn proptest_strategy() -> impl Strategy<Value = Ranges<u32>> {
    (
        any::<bool>(),
        prop::collection::vec(any::<(u32, bool)>(), 0..10),
    )
        .prop_map(|(start_unbounded, deltas)| {
            let mut start = if start_unbounded {
                Some(Unbounded)
            } else {
                None
            };
            let mut largest: u32 = 0;
            let mut last_bound_was_inclusive = false;
            let mut segments = SmallVec::new();
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

            Ranges { segments }.check_invariants()
        })
}

#[cfg(test)]
pub mod tests {
    use proptest::prelude::*;

    use super::*;

    fn version_strat() -> impl Strategy<Value = u32> {
        any::<u32>()
    }

    proptest! {

        // Testing serde ----------------------------------

        #[cfg(feature = "serde")]
        #[test]
        fn serde_round_trip(range in proptest_strategy()) {
            let s = ron::ser::to_string(&range).unwrap();
            let r = ron::de::from_str(&s).unwrap();
            assert_eq!(range, r);
        }

        // Testing negate ----------------------------------

        #[test]
        fn negate_is_different(range in proptest_strategy()) {
            assert_ne!(range.complement(), range);
        }

        #[test]
        fn double_negate_is_identity(range in proptest_strategy()) {
            assert_eq!(range.complement().complement(), range);
        }

        #[test]
        fn negate_contains_opposite(range in proptest_strategy(), version in version_strat()) {
            assert_ne!(range.contains(&version), range.complement().contains(&version));
        }

        // Testing intersection ----------------------------

        #[test]
        fn intersection_is_symmetric(r1 in proptest_strategy(), r2 in proptest_strategy()) {
            assert_eq!(r1.intersection(&r2), r2.intersection(&r1));
        }

        #[test]
        fn intersection_with_any_is_identity(range in proptest_strategy()) {
            assert_eq!(Ranges::full().intersection(&range), range);
        }

        #[test]
        fn intersection_with_none_is_none(range in proptest_strategy()) {
            assert_eq!(Ranges::empty().intersection(&range), Ranges::empty());
        }

        #[test]
        fn intersection_is_idempotent(r1 in proptest_strategy(), r2 in proptest_strategy()) {
            assert_eq!(r1.intersection(&r2).intersection(&r2), r1.intersection(&r2));
        }

        #[test]
        fn intersection_is_associative(r1 in proptest_strategy(), r2 in proptest_strategy(), r3 in proptest_strategy()) {
            assert_eq!(r1.intersection(&r2).intersection(&r3), r1.intersection(&r2.intersection(&r3)));
        }

        #[test]
        fn intesection_of_complements_is_none(range in proptest_strategy()) {
            assert_eq!(range.complement().intersection(&range), Ranges::empty());
        }

        #[test]
        fn intesection_contains_both(r1 in proptest_strategy(), r2 in proptest_strategy(), version in version_strat()) {
            assert_eq!(r1.intersection(&r2).contains(&version), r1.contains(&version) && r2.contains(&version));
        }

        // Testing union -----------------------------------

        #[test]
        fn union_of_complements_is_any(range in proptest_strategy()) {
            assert_eq!(range.complement().union(&range), Ranges::full());
        }

        #[test]
        fn union_contains_either(r1 in proptest_strategy(), r2 in proptest_strategy(), version in version_strat()) {
            assert_eq!(r1.union(&r2).contains(&version), r1.contains(&version) || r2.contains(&version));
        }

        #[test]
        fn is_disjoint_through_intersection(r1 in proptest_strategy(), r2 in proptest_strategy()) {
            let disjoint_def = r1.intersection(&r2) == Ranges::empty();
            assert_eq!(r1.is_disjoint(&r2), disjoint_def);
        }

        #[test]
        fn subset_of_through_intersection(r1 in proptest_strategy(), r2 in proptest_strategy()) {
            let disjoint_def = r1.intersection(&r2) == r1;
            assert_eq!(r1.subset_of(&r2), disjoint_def);
        }

        #[test]
        fn union_through_intersection(r1 in proptest_strategy(), r2 in proptest_strategy()) {
            let union_def = r1
                .complement()
                .intersection(&r2.complement())
                .complement()
                .check_invariants();
            assert_eq!(r1.union(&r2), union_def);
        }

        // Testing contains --------------------------------

        #[test]
        fn always_contains_exact(version in version_strat()) {
            assert!(Ranges::singleton(version).contains(&version));
        }

        #[test]
        fn contains_negation(range in proptest_strategy(), version in version_strat()) {
            assert_ne!(range.contains(&version), range.complement().contains(&version));
        }

        #[test]
        fn contains_intersection(range in proptest_strategy(), version in version_strat()) {
            assert_eq!(range.contains(&version), range.intersection(&Ranges::singleton(version)) != Ranges::empty());
        }

        #[test]
        fn contains_bounding_range(range in proptest_strategy(), version in version_strat()) {
            if range.contains(&version) {
                assert!(range.bounding_range().map(|b| b.contains(&version)).unwrap_or(false));
            }
        }

        #[test]
        fn from_range_bounds(range in any::<(Bound<u32>, Bound<u32>)>(), version in version_strat()) {
            let rv: Ranges<_> = Ranges::from_range_bounds(range);
            assert_eq!(range.contains(&version), rv.contains(&version));
        }

        #[test]
        fn from_range_bounds_round_trip(range in any::<(Bound<u32>, Bound<u32>)>()) {
            let rv: Ranges<u32> = Ranges::from_range_bounds(range);
            let rv2: Ranges<u32> = rv.bounding_range().map(Ranges::from_range_bounds::<_, u32>).unwrap_or_else(Ranges::empty);
            assert_eq!(rv, rv2);
        }

        #[test]
        fn contains(range in proptest_strategy(), versions in proptest::collection::vec(version_strat(), ..30)) {
            for v in versions {
                assert_eq!(range.contains(&v), range.segments.iter().any(|s| RangeBounds::contains(s, &v)));
            }
        }

        #[test]
        fn contains_many(range in proptest_strategy(), mut versions in proptest::collection::vec(version_strat(), ..30)) {
            versions.sort();
            assert_eq!(versions.len(), range.contains_many(versions.iter()).count());
            for (a, b) in versions.iter().zip(range.contains_many(versions.iter())) {
                assert_eq!(range.contains(a), b);
            }
        }

        #[test]
        fn simplify(range in proptest_strategy(), mut versions in proptest::collection::vec(version_strat(), ..30)) {
            versions.sort();
            let simp = range.simplify(versions.iter());

            for v in versions {
                assert_eq!(range.contains(&v), simp.contains(&v));
            }
            assert!(simp.segments.len() <= range.segments.len())
        }

        #[test]
        fn from_iter_valid(segments in proptest::collection::vec(any::<(Bound<u32>, Bound<u32>)>(), ..30)) {
            let mut expected = Ranges::empty();
            for segment in &segments {
                expected = expected.union(&Ranges::from_range_bounds(*segment));
            }
            let actual =  Ranges::from_iter(segments.clone());
            assert_eq!(expected, actual, "{segments:?}");
        }
    }

    #[test]
    fn contains_many_can_take_owned() {
        let range: Ranges<u8> = Ranges::singleton(1);
        let versions = vec![1, 2, 3];
        // Check that iter can be a Cow
        assert_eq!(
            range.contains_many(versions.iter()).count(),
            range
                .contains_many(versions.iter().map(std::borrow::Cow::Borrowed))
                .count()
        );
        // Check that iter can be a V
        assert_eq!(
            range.contains_many(versions.iter()).count(),
            range.contains_many(versions.into_iter()).count()
        );
    }

    #[test]
    fn simplify_can_take_owned() {
        let range: Ranges<u8> = Ranges::singleton(1);
        let versions = vec![1, 2, 3];
        // Check that iter can be a Cow
        assert_eq!(
            range.simplify(versions.iter()),
            range.simplify(versions.iter().map(std::borrow::Cow::Borrowed))
        );
        // Check that iter can be a V
        assert_eq!(
            range.simplify(versions.iter()),
            range.simplify(versions.into_iter())
        );
    }

    #[test]
    fn version_ord() {
        let versions: &[Ranges<u32>] = &[
            Ranges::strictly_lower_than(1u32),
            Ranges::lower_than(1u32),
            Ranges::singleton(1u32),
            Ranges::between(1u32, 3u32),
            Ranges::higher_than(1u32),
            Ranges::strictly_higher_than(1u32),
            Ranges::singleton(2u32),
            Ranges::singleton(2u32).union(&Ranges::singleton(3u32)),
            Ranges::singleton(2u32)
                .union(&Ranges::singleton(3u32))
                .union(&Ranges::singleton(4u32)),
            Ranges::singleton(2u32).union(&Ranges::singleton(4u32)),
            Ranges::singleton(3u32),
        ];

        let mut versions_sorted = versions.to_vec();
        versions_sorted.sort();
        assert_eq!(versions_sorted, versions);

        // Check that the sorting isn't just stable because we're returning equal.
        let mut version_reverse_sorted = versions.to_vec();
        version_reverse_sorted.reverse();
        version_reverse_sorted.sort();
        assert_eq!(version_reverse_sorted, versions);
    }
}
