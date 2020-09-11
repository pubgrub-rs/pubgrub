// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Ranges are constraints defining sets of versions.
//!
//! Concretely, those constraints correspond to any set of versions
//! representable as the concatenation, union, and complement
//! of the base version ranges building blocks.
//!
//! Those building blocks are:
//!  - `none()`: the empty set
//!  - `any()`: the set of all possible versions
//!  - `exact(v)`: the set containing only the version v
//!  - `higherThan(v)`: `v <= versions`
//!  - `lowerThan(v)`: `versions < v` (note the "strictly" lower)
//!  - `between(v1, v2)`: `v1 <= versions < v2`

use crate::version::Version;

/// A Range is a set of versions.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Range<T: Clone + Eq + Version> {
    segments: Vec<Interval<T>>,
}

type Interval<T> = (T, Option<T>);

// Range building blocks.
impl<T: Clone + Ord + Version> Range<T> {
    /// Empty set of versions.
    pub fn none() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Set of all possible versions.
    pub fn any() -> Self {
        Self::higher_than(T::lowest())
    }

    /// Set containing exactly one version.
    pub fn exact(v: T) -> Self {
        Self {
            segments: vec![(v.clone(), Some(v.bump()))],
        }
    }

    /// Set of all versions higher or equal to some version.
    pub fn higher_than(v: T) -> Self {
        Self {
            segments: vec![(v, None)],
        }
    }

    /// Set of all versions strictly lower than some version.
    pub fn strictly_lower_than(v: T) -> Self {
        if v == T::lowest() {
            Self::none()
        } else {
            Self {
                segments: vec![(T::lowest(), Some(v))],
            }
        }
    }

    /// Set of all versions comprised between two given versions.
    /// The lower bound is included and the higher bound excluded.
    /// `v1 <= v < v2`.
    pub fn between(v1: T, v2: T) -> Self {
        if v1 < v2 {
            Self {
                segments: vec![(v1, Some(v2))],
            }
        } else {
            Self::none()
        }
    }
}

// Set operations.
impl<T: Clone + Ord + Version> Range<T> {
    // Negate ##################################################################

    /// Compute the complement set of versions.
    pub fn negate(&self) -> Self {
        match self.segments.as_slice().first() {
            None => Self::any(), // Complement of ∅  is *

            // First high bound is +∞
            Some((v, None)) => {
                // Complement of * is ∅
                if v == &T::lowest() {
                    Self::none()
                // Complement of "v <= _" is "_ < v"
                } else {
                    Self::strictly_lower_than(v.clone())
                }
            }

            // First high bound is not +∞
            Some((v1, Some(v2))) => {
                if v1 == &T::lowest() {
                    Self {
                        segments: Self::negate_segments(v2.clone(), &self.segments[1..]),
                    }
                } else {
                    Self {
                        segments: Self::negate_segments(T::lowest(), &self.segments),
                    }
                }
            }
        }
    }

    /// Helper function performing the negation of intervals in segments.
    /// For example:
    ///    [ (v1, None) ] => [ (start, Some(v1)) ]
    ///    [ (v1, Some(v2)) ] => [ (start, Some(v1)), (v2, None) ]
    fn negate_segments(start: T, segments: &[Interval<T>]) -> Vec<Interval<T>> {
        let mut complement_segments = Vec::with_capacity(1 + segments.len());
        let mut start = Some(start);
        for (v1, some_v2) in segments.iter() {
            // start.unwrap() is fine because `segments` is not exposed,
            // and our usage guaranties that only the last segment may contain a None.
            complement_segments.push((start.unwrap(), Some(v1.to_owned())));
            start = some_v2.to_owned();
        }
        if let Some(last) = start {
            complement_segments.push((last, None));
        }
        complement_segments
    }

    // Union and intersection ##################################################

    /// Compute the union of two sets of versions.
    pub fn union(&self, other: &Self) -> Self {
        (self.negate().intersection(&other.negate())).negate()
    }

    /// Compute the intersection of two sets of versions.
    pub fn intersection(&self, other: &Self) -> Self {
        Self {
            segments: Self::intersection_segments(&self.segments, &other.segments),
        }
    }

    /// Helper function performing intersection of two interval segments.
    fn intersection_segments(s1: &[Interval<T>], s2: &[Interval<T>]) -> Vec<Interval<T>> {
        let mut segments = Vec::with_capacity(s1.len().min(s2.len()));
        let mut left_iter = s1.iter();
        let mut right_iter = s2.iter();
        let mut left = left_iter.next();
        let mut right = right_iter.next();
        loop {
            match (left, right) {
                // Both left and right still contain a finite interval:
                (Some((l1, Some(l2))), Some((r1, Some(r2)))) => {
                    if l2 <= r1 {
                        // Intervals are disjoint, progress on the left.
                        left = left_iter.next();
                    } else if r2 <= l1 {
                        // Intervals are disjoint, progress on the right.
                        right = right_iter.next();
                    } else {
                        // Intervals are not disjoint.
                        let start = l1.max(r1).to_owned();
                        if l2 < r2 {
                            segments.push((start, Some(l2.to_owned())));
                            left = left_iter.next();
                        } else {
                            segments.push((start, Some(r2.to_owned())));
                            right = right_iter.next();
                        }
                    }
                }

                // Right contains an infinite interval:
                (Some((l1, Some(l2))), Some((r1, None))) => {
                    if l2 < r1 {
                        left = left_iter.next();
                    } else if l2 == r1 {
                        segments.extend(left_iter.cloned());
                        break;
                    } else {
                        let start = l1.max(r1).to_owned();
                        segments.push((start, Some(l2.to_owned())));
                        segments.extend(left_iter.cloned());
                        break;
                    }
                }

                // Left contains an infinite interval:
                (Some((l1, None)), Some((r1, Some(r2)))) => {
                    if r2 < l1 {
                        right = right_iter.next();
                    } else if r2 == l1 {
                        segments.extend(right_iter.cloned());
                        break;
                    } else {
                        let start = l1.max(r1).to_owned();
                        segments.push((start, Some(r2.to_owned())));
                        segments.extend(right_iter.cloned());
                        break;
                    }
                }

                // Left or right has ended.
                _ => {
                    break;
                }
            }
        }
        segments
    }
}

// Other useful functions.
impl<T: Clone + Ord + Version> Range<T> {
    /// Check if a range contains a given version.
    pub fn contains(&self, version: &T) -> bool {
        for (v1, some_v2) in self.segments.iter() {
            match some_v2 {
                None => return v1 <= version,
                Some(v2) => {
                    if version < v1 {
                        return false;
                    } else if version < v2 {
                        return true;
                    }
                }
            }
        }
        false
    }
}
