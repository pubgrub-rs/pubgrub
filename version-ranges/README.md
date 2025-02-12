# Ranges

[![crates.io](https://img.shields.io/crates/v/version-ranges.svg?logo=rust)](https://crates.io/crates/version-ranges)
[![docs.rs](https://img.shields.io/badge/docs.rs-version-ranges)](https://docs.rs/version-ranges)

This crate contains a performance-optimized type for generic version ranges and operations on them.

`Ranges` can represent version selectors such as `(>=1.5.1, <2) OR (==3.1) OR (>4)`. Internally, it is an ordered list
of contiguous intervals (segments) with inclusive, exclusive or open-ended ends, similar to a
`Vec<(Bound<T>, Bound<T>)>`.

You can construct a basic range from one of the following build blocks. All other ranges are concatenation, union, and
complement of these basic ranges.

- `Ranges::empty()`: No version
- `Ranges::full()`: All versions
- `Ranges::singleton(v)`: Only the version v exactly
- `Ranges::higher_than(v)`: All versions `v <= versions`
- `Ranges::strictly_higher_than(v)`: All versions `v < versions`
- `Ranges::lower_than(v)`: All versions `versions <= v`
- `Ranges::strictly_lower_than(v)`: All versions `versions < v`
- `Ranges::between(v1, v2)`: All versions `v1 <= versions < v2`

The optimized operations include `complement`, `contains`, `contains_many`, `intersection`, `is_disjoint`,
`subset_of` and `union`.

`Ranges` is generic over any type that implements `Ord` + `Clone` and can represent all kinds of slices with ordered
coordinates, not just version ranges. While built as a performance-critical piece
of [pubgrub](https://github.com/pubgrub-rs/pubgrub), it can be adopted for other domains, too.

![A number line and a sample range on it](number-line-ranges.svg)

You can imagine a `Ranges` as slices over a number line.

Note that there are limitations to the equality implementation: Given a `Ranges<u32>`, the segments
`(Unbounded, Included(42u32))` and `(Included(0), Included(42u32))` as well as
`(Included(1), Included(5))` and  `(Included(1), Included(3)) + (Included(4), Included(5))`
are reported as unequal, even though the match the same versions: We can't tell that there isn't a version between `0`
and `-inf` or `3` and `4` respectively.

## Optional features

* `serde`: serialization and deserialization for the version range, given that the version type also supports it.
* `proptest`: Exports are proptest strategy for `Ranges<u32>`.
