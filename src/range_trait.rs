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

use std::cmp::Ordering;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Bound;

use crate::internal::small_vec::SmallVec;
use crate::version_trait::{flip_bound, owned_bound, ref_bound};
use crate::version_trait::{Interval, NumberInterval, NumberVersion, Version};

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Range<I, V> {
    segments: SmallVec<I>,
    #[cfg_attr(feature = "serde", serde(skip_serializing))]
    phantom: PhantomData<V>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum SidedBound<V> {
    Left(Bound<V>),
    Right(Bound<V>),
}

fn ref_sided_bound<V>(sb: &SidedBound<V>) -> SidedBound<&V> {
    match sb {
        SidedBound::Left(bound) => SidedBound::Left(ref_bound(bound)),
        SidedBound::Right(bound) => SidedBound::Right(ref_bound(bound)),
    }
}

impl<V: Ord> PartialOrd for SidedBound<V> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<V: Ord> Ord for SidedBound<V> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.compare(other)
    }
}

impl<V: Ord> SidedBound<V> {
    fn unwrap(self) -> Bound<V> {
        match self {
            Self::Left(bound) => bound,
            Self::Right(bound) => bound,
        }
    }
    fn compare(&self, other: &Self) -> Ordering {
        match (&self, other) {
            // Handling of both left bounds.
            (SidedBound::Left(Bound::Unbounded), SidedBound::Left(Bound::Unbounded)) => {
                Ordering::Equal
            }
            (SidedBound::Left(Bound::Unbounded), SidedBound::Left(_)) => Ordering::Less,
            (SidedBound::Left(Bound::Excluded(l1)), SidedBound::Left(Bound::Excluded(l2))) => {
                l1.cmp(l2)
            }
            (SidedBound::Left(Bound::Excluded(l1)), SidedBound::Left(Bound::Included(l2))) => {
                // An open left bound is greater than a closed left bound.
                l1.cmp(l2).then(Ordering::Greater)
            }
            (SidedBound::Left(Bound::Included(l1)), SidedBound::Left(Bound::Included(l2))) => {
                l1.cmp(l2)
            }
            (SidedBound::Left(_), SidedBound::Left(_)) => other.compare(&self).reverse(),

            // Handling of both right bounds.
            (SidedBound::Right(Bound::Unbounded), SidedBound::Right(Bound::Unbounded)) => {
                Ordering::Equal
            }
            (SidedBound::Right(Bound::Unbounded), SidedBound::Right(_)) => Ordering::Greater,
            (SidedBound::Right(Bound::Excluded(r1)), SidedBound::Right(Bound::Excluded(r2))) => {
                r1.cmp(r2)
            }
            (SidedBound::Right(Bound::Excluded(r1)), SidedBound::Right(Bound::Included(r2))) => {
                // An open Right bound is smaller than a closed Right bound.
                r1.cmp(r2).then(Ordering::Less)
            }
            (SidedBound::Right(Bound::Included(r1)), SidedBound::Right(Bound::Included(r2))) => {
                r1.cmp(r2)
            }
            (SidedBound::Right(_), SidedBound::Right(_)) => other.compare(&self).reverse(),

            // Handling of left and right bounds.
            (SidedBound::Left(Bound::Unbounded), SidedBound::Right(_)) => Ordering::Less,
            (SidedBound::Left(_), SidedBound::Right(Bound::Unbounded)) => Ordering::Less,
            (SidedBound::Left(Bound::Excluded(l)), SidedBound::Right(Bound::Excluded(r))) => {
                // An open left bound is after an open right bound.
                l.cmp(r).then(Ordering::Greater)
            }
            (SidedBound::Left(Bound::Excluded(l)), SidedBound::Right(Bound::Included(r))) => {
                l.cmp(r)
            }
            (SidedBound::Left(Bound::Included(l)), SidedBound::Right(Bound::Excluded(r))) => {
                l.cmp(r)
            }
            (SidedBound::Left(Bound::Included(l)), SidedBound::Right(Bound::Included(r))) => {
                l.cmp(r).then(Ordering::Less)
            }

            // Handling of right and left bounds.
            (SidedBound::Right(_), SidedBound::Left(_)) => other.compare(&self).reverse(),
        }
    }
}

// Ranges building blocks.
impl<I: Interval<V>, V: Version> Range<I, V> {
    /// Empty set of versions.
    pub fn empty() -> Self {
        Self {
            segments: SmallVec::empty(),
            phantom: PhantomData,
        }
    }

    /// Set of all possible versions.
    pub fn full() -> Self {
        Self {
            segments: SmallVec::one(I::new(V::minimum(), V::maximum())),
            phantom: PhantomData,
        }
    }

    /// Set containing exactly one version.
    pub fn singleton(v: impl Into<V>) -> Self {
        let v = v.into();
        let start_bound = Bound::Included(v.clone());
        let end_bound = Bound::Included(v);
        Self {
            segments: SmallVec::one(I::new(start_bound, end_bound)),
            phantom: PhantomData,
        }
    }

    /// Set of all versions higher or equal to some version.
    pub fn higher_than(v: impl Into<V>) -> Self {
        Self {
            segments: SmallVec::one(I::new(Bound::Included(v.into()), V::maximum())),
            phantom: PhantomData,
        }
    }

    /// Set of all versions strictly lower than some version.
    pub fn strictly_lower_than(v: impl Into<V>) -> Self {
        Self {
            segments: SmallVec::one(I::new(V::minimum(), Bound::Excluded(v.into()))),
            phantom: PhantomData,
        }
    }

    /// Set of all versions strictly lower than some version.
    pub fn strictly_higher_than(v: impl Into<V>) -> Self {
        Self {
            segments: SmallVec::one(I::new(Bound::Excluded(v.into()), V::maximum())),
            phantom: PhantomData,
        }
    }

    /// Set of all versions comprised between two given versions.
    /// The lower bound is included and the higher bound excluded.
    /// `v1 <= v < v2`.
    pub fn between(v1: impl Into<V>, v2: impl Into<V>) -> Self {
        let start_bound = Bound::Included(v1.into());
        let end_bound = Bound::Excluded(v2.into());
        Self {
            segments: SmallVec::one(I::new(start_bound, end_bound)),
            phantom: PhantomData,
        }
    }
}

// Set operations.
impl<I: Interval<V>, V: Version> Range<I, V> {
    // Negate ##################################################################

    /// Compute the complement set of versions.
    pub fn complement(&self) -> Self {
        match self.segments.first() {
            None => Self::full(), // Complement of ∅  is *
            Some(seg) => {
                if seg.start_bound() == ref_bound(&V::minimum()) {
                    Self::complement_segments(
                        owned_bound(flip_bound(seg.end_bound())),
                        &self.segments[1..],
                    )
                } else {
                    Self::complement_segments(V::minimum(), &self.segments)
                }
            }
        }
    }

    /// Helper function performing the negation of intervals in segments.
    /// For example:
    ///    [ (v1, None) ] => [ (start, Some(v1)) ]
    ///    [ (v1, Some(v2)) ] => [ (start, Some(v1)), (v2, None) ]
    fn complement_segments(start: Bound<V>, segments: &[I]) -> Self {
        let mut complemented_segments = SmallVec::empty();
        let mut start = start;
        for seg in segments {
            complemented_segments.push(I::new(start, owned_bound(flip_bound(seg.start_bound()))));
            start = owned_bound(flip_bound(seg.end_bound()));
        }
        if start != V::maximum() {
            complemented_segments.push(I::new(start, V::maximum()));
        }

        Self {
            segments: complemented_segments,
            phantom: PhantomData,
        }
    }

    // Union and intersection ##################################################

    /// Compute the union of two sets of versions.
    pub fn union(&self, other: &Self) -> Self {
        self.complement()
            .intersection(&other.complement())
            .complement()
    }

    /// Compute the intersection of two sets of versions.
    pub fn intersection(&self, other: &Self) -> Self {
        let mut segments = SmallVec::empty();
        let mut left_iter = self.segments.iter();
        let mut right_iter = other.segments.iter();
        let mut left = left_iter.next();
        let mut right = right_iter.next();
        loop {
            match (left, right) {
                (Some(seg_left), Some(seg_right)) => {
                    let l1 = seg_left.start_bound();
                    let l2 = seg_left.end_bound();
                    let r1 = seg_right.start_bound();
                    let r2 = seg_right.end_bound();
                    match SidedBound::Right(l2).compare(&SidedBound::Left(r1)) {
                        // Disjoint intervals with left < right.
                        Ordering::Less => left = left_iter.next(),
                        Ordering::Equal => left = left_iter.next(),
                        // Possible intersection with left >= right.
                        Ordering::Greater => {
                            match SidedBound::Right(r2).compare(&SidedBound::Left(l1)) {
                                // Disjoint intervals with left < right.
                                Ordering::Less => right = right_iter.next(),
                                Ordering::Equal => right = right_iter.next(),
                                // Intersection for sure.
                                Ordering::Greater => {
                                    let start = match SidedBound::Left(l1)
                                        .compare(&SidedBound::Right(r1))
                                    {
                                        Ordering::Less => r1,
                                        _ => l1,
                                    };
                                    match SidedBound::Right(l2).compare(&SidedBound::Right(r2)) {
                                        Ordering::Less => {
                                            segments
                                                .push(I::new(owned_bound(start), owned_bound(l2)));
                                            left = left_iter.next();
                                        }
                                        Ordering::Equal => {
                                            segments
                                                .push(I::new(owned_bound(start), owned_bound(l2)));
                                            left = left_iter.next();
                                            right = right_iter.next();
                                        }
                                        Ordering::Greater => {
                                            segments
                                                .push(I::new(owned_bound(start), owned_bound(r2)));
                                            right = right_iter.next();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // Left or right has ended.
                _ => {
                    break;
                }
            }
        }

        Self {
            segments,
            phantom: PhantomData,
        }
    }

    // Contains ################################################################

    /// Check if ranges contain a given version.
    pub fn contains(&self, version: &V) -> bool {
        for seg in &self.segments {
            match (seg.start_bound(), seg.end_bound()) {
                (Bound::Unbounded, Bound::Unbounded) => return seg.contains(version),
                (Bound::Unbounded, Bound::Excluded(r)) => match version.cmp(r) {
                    Ordering::Less => return seg.contains(version),
                    Ordering::Equal => return false,
                    Ordering::Greater => {}
                },
                (Bound::Unbounded, Bound::Included(r)) => match version.cmp(r) {
                    Ordering::Greater => {}
                    _ => return seg.contains(version),
                },
                (Bound::Excluded(l), Bound::Unbounded) => match version.cmp(l) {
                    Ordering::Greater => return seg.contains(version),
                    _ => return false,
                },
                (Bound::Excluded(l), Bound::Excluded(r)) => match version.cmp(l) {
                    Ordering::Less => return false,
                    Ordering::Equal => return false,
                    Ordering::Greater => match version.cmp(r) {
                        Ordering::Less => return seg.contains(version),
                        Ordering::Equal => return false,
                        Ordering::Greater => {}
                    },
                },
                (Bound::Excluded(l), Bound::Included(r)) => match version.cmp(l) {
                    Ordering::Less => return false,
                    Ordering::Equal => return false,
                    Ordering::Greater => match version.cmp(r) {
                        Ordering::Greater => {}
                        _ => return seg.contains(version),
                    },
                },
                (Bound::Included(l), Bound::Unbounded) => match version.cmp(l) {
                    Ordering::Less => return false,
                    _ => return seg.contains(version),
                },
                (Bound::Included(l), Bound::Excluded(r)) => match version.cmp(l) {
                    Ordering::Less => return false,
                    Ordering::Equal => return seg.contains(version),
                    Ordering::Greater => match version.cmp(r) {
                        Ordering::Less => return seg.contains(version),
                        Ordering::Equal => return false,
                        Ordering::Greater => {}
                    },
                },
                (Bound::Included(l), Bound::Included(r)) => match version.cmp(l) {
                    Ordering::Less => return false,
                    Ordering::Equal => return seg.contains(version),
                    Ordering::Greater => match version.cmp(r) {
                        Ordering::Greater => {}
                        _ => return seg.contains(version),
                    },
                },
            }
        }
        false
    }
}

// REPORT ######################################################################

impl<I: Interval<V>, V: Version> fmt::Display for Range<I, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.segments.as_slice() {
            [] => write!(f, "∅"),
            [seg] => {
                write!(f, "{}", interval_to_string(seg))
            }
            more_than_one_interval => {
                let string_intervals: Vec<_> = more_than_one_interval
                    .iter()
                    .map(interval_to_string)
                    .collect();
                write!(f, "{}", string_intervals.join(" OR "))
            }
        }
    }
}

fn interval_to_string<I: Interval<V>, V: Version>(seg: &I) -> String {
    let start = seg.start_bound();
    let end = seg.end_bound();
    if start == ref_bound(&V::minimum()) {
        display_end_bound(end)
    } else if end == ref_bound(&V::maximum()) {
        display_start_bound(start)
    } else {
        format!("{}, {}", display_start_bound(start), display_end_bound(end))
    }
}

fn display_start_bound<V: Version>(start: Bound<&V>) -> String {
    match start {
        Bound::Unbounded => "∗".to_string(),
        Bound::Excluded(v) => format!("> {}", v),
        Bound::Included(v) => format!(">= {}", v),
    }
}

fn display_end_bound<V: Version>(end: Bound<&V>) -> String {
    match end {
        Bound::Unbounded => "∗".to_string(),
        Bound::Excluded(v) => format!("< {}", v),
        Bound::Included(v) => format!("<= {}", v),
    }
}

// TESTS #######################################################################

#[cfg(test)]
pub mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::version_trait::{SemanticInterval, SemanticVersion};

    // SidedBound tests.

    use Bound::{Excluded, Included, Unbounded};
    use SidedBound::{Left, Right};

    fn sided_bound_strategy() -> impl Strategy<Value = SidedBound<NumberVersion>> {
        prop_oneof![
            bound_strategy().prop_map(Left),
            bound_strategy().prop_map(Right),
        ]
    }

    fn bound_strategy() -> impl Strategy<Value = Bound<NumberVersion>> {
        prop_oneof![
            Just(Unbounded),
            any::<u32>().prop_map(|n| Excluded(n.into())),
            any::<u32>().prop_map(|n| Included(n.into())),
        ]
    }

    fn no_unbound_strategy() -> impl Strategy<Value = Bound<NumberVersion>> {
        prop_oneof![
            any::<u32>().prop_map(|n| Excluded(n.into())),
            any::<u32>().prop_map(|n| Included(n.into())),
        ]
    }

    proptest! {

        #[test]
        fn reverse_bounds_reverse_order(sb1 in sided_bound_strategy(), sb2 in sided_bound_strategy()) {
            let s1 = ref_sided_bound(&sb1);
            let s2 = ref_sided_bound(&sb2);
            assert_eq!(s1.compare(&s2), s2.compare(&s1).reverse());
        }

    }

    // Ranges tests.

    #[test]
    fn failing_symmetric() {
        let s1: Range<NumberInterval, NumberVersion> = Range::strictly_higher_than(0);
        let s2 = Range::higher_than(0);
        eprintln!("first:  {}", s1);
        eprintln!("second: {}", s2);
        assert_eq!(s1.intersection(&s2), s2.intersection(&s1));
    }

    pub fn strategy() -> impl Strategy<Value = Range<NumberInterval, NumberVersion>> {
        // TODO: find a better strategy so that the "intersection_is_symmetric" test
        // triggers the same failure than the negate_singleton above.
        prop::collection::vec(no_unbound_strategy(), 0..20).prop_map(|mut vec| {
            let extract = |b: &Bound<NumberVersion>| match b {
                Unbounded => panic!("cannot happen"),
                Excluded(v) => v.0,
                Included(v) => v.0,
            };
            vec.dedup();
            vec.sort_unstable_by_key(extract);
            let mut segments: SmallVec<NumberInterval> = SmallVec::empty();
            let mut last_right: i64 = -1;
            let mut pair_iter = vec.chunks_exact(2);
            while let Some([b1, b2]) = pair_iter.next() {
                let v1 = extract(b1);
                let v2 = extract(b2);
                if v1 as i64 == last_right {
                    segments.push(NumberInterval::new(Excluded(v1.into()), b2.clone()));
                } else if v1 == v2 {
                    // segments.push((v1..=v1).into());
                    segments.push(NumberInterval::new(
                        Included(v1.into()),
                        Included(v2.into()),
                    ));
                } else {
                    segments.push(NumberInterval::new(b1.clone(), b2.clone()));
                }
                last_right = v2 as i64;
            }
            if let [b1] = pair_iter.remainder() {
                let v1 = extract(b1);
                if v1 as i64 == last_right {
                    segments.push(NumberInterval::new(Excluded(v1.into()), Unbounded));
                } else {
                    segments.push((v1..).into());
                }
            }
            eprintln!("generated segments length: {}", segments.len());
            Range {
                // segments: merge_juxtaposed_segments(segments),
                segments,
                phantom: PhantomData,
            }
        })
    }

    fn version_strat() -> impl Strategy<Value = NumberVersion> {
        any::<u32>().prop_map(NumberVersion)
    }

    proptest! {

        // #[test]
        // fn long_ranges(ranges in strategy()) {
        //     if ranges.segments.len() > 2 {
        //         assert_eq!(ranges, Range::empty());
        //     }
        // }

        // Testing negate ----------------------------------

        #[test]
        fn negate_is_different(ranges in strategy()) {
            assert_ne!(ranges.complement(), ranges);
        }

        #[test]
        fn double_negate_is_identity(ranges in strategy()) {
            assert_eq!(ranges.complement().complement(), ranges);
        }

        #[test]
        fn negate_contains_opposite(ranges in strategy(), version in version_strat()) {
            assert_ne!(ranges.contains(&version), ranges.complement().contains(&version));
        }

        // Testing intersection ----------------------------

        #[test]
        fn intersection_is_symmetric(r1 in strategy(), r2 in strategy()) {
            assert_eq!(r1.intersection(&r2), r2.intersection(&r1));
        }

        #[test]
        fn intersection_with_any_is_identity(ranges in strategy()) {
            assert_eq!(Range::full().intersection(&ranges), ranges);
        }

        #[test]
        fn intersection_with_none_is_none(ranges in strategy()) {
            assert_eq!(Range::empty().intersection(&ranges), Range::empty());
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
        fn intesection_of_complements_is_none(ranges in strategy()) {
            assert_eq!(ranges.complement().intersection(&ranges), Range::empty());
        }

        #[test]
        fn intesection_contains_both(r1 in strategy(), r2 in strategy(), version in version_strat()) {
            assert_eq!(r1.intersection(&r2).contains(&version), r1.contains(&version) && r2.contains(&version));
        }

        // Testing union -----------------------------------

        #[test]
        fn union_of_complements_is_any(ranges in strategy()) {
            assert_eq!(ranges.complement().union(&ranges), Range::full());
        }

        #[test]
        fn union_contains_either(r1 in strategy(), r2 in strategy(), version in version_strat()) {
            assert_eq!(r1.union(&r2).contains(&version), r1.contains(&version) || r2.contains(&version));
        }

        // Testing contains --------------------------------

        #[test]
        fn always_contains_exact(version in version_strat()) {
            assert!(Range::<NumberInterval, _>::singleton(version).contains(&version));
        }

        #[test]
        fn contains_negation(ranges in strategy(), version in version_strat()) {
            assert_ne!(ranges.contains(&version), ranges.complement().contains(&version));
        }

        #[test]
        fn contains_intersection(ranges in strategy(), version in version_strat()) {
            assert_eq!(ranges.contains(&version), ranges.intersection(&Range::singleton(version)) != Range::empty());
        }
    }

    // #[test]
    // fn contains_negation_0() {
    //     let full: Range<NumberInterval, NumberVersion> = Range::full();
    //     println!("full: {:?}", full);
    //     let full_comp = full.complement();
    //     println!("full.complement(): {:?}", full_comp);
    //     let v0 = NumberVersion::from(0);
    //     assert_eq!(&full_comp, &Range::empty());
    //     assert_ne!(full.contains(&v0), full_comp.contains(&v0));
    //     panic!("force fail");
    // }
}
