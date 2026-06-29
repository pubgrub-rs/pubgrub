# Changelog

Changelog for the version-ranges crate.

## Unreleased

### Added

- Add classification of subset, disjoint, and overlapping ranges.

- Add `Ranges::difference`, which computes the versions contained in `self` but not in `other` in a single pass, without materializing the complement ([#432](https://github.com/pubgrub-rs/pubgrub/pull/432)).

## v0.1.3 - 2026-04-09

- Add optional `semver` conversions ([#405](https://github.com/pubgrub-rs/pubgrub/pull/405))

## v0.1.2

* Allow `Ranges::contains` to accept borrows, e.g. `&str` for `Ranges<String>`

## v0.1.1

* Added `Ranges::from_iter`
* Implement `IntoIter` on `Ranges`

## v0.1.0

Initial release!
