# Changelog

All notable changes to this project will be documented in this file.

## Unreleased [(diff)][diff-unreleased]

## [0.2.0] - 2020-11-17 - [(diff with 0.1.0)][diff-0.2.0]

This release brings many important improvements to PubGrub.
The gist of it is:

- A bug in the algorithm was [fixed](https://github.com/pubgrub-rs/pubgrub/pull/23).
- The solver is now implemented in a `resolve` function taking as argument
  an implementer of the `DependencyProvider` trait,
  which has more control over the decision making process.
- End-to-end property testing of large synthetic registries was added.
- Huge performance improvements (more than 10x better performances).

### Changes affecting the public API

#### Added

- Links to code items in the code documenation.
- New `"serde"` feature that serialize and deserialize few types in the API.
- New variants for `error::PubGrubError` which are `DependencyOnTheEmptySet`,
  `SelfDependency`, `ErrorChoosingPackageVersion` and `ErrorShouldCancel`.
- New `type_alias::Map` defined as `rustc_hash::FxHashMap`.
- New `type_alias::SelectedDependencies<P, V>` defined as `Map<P, V>`.
- The types `Dependencies` and `DependencyConstraints` were introduced to clarify intent.
- New function `choose_package_with_fewest_versions` to help implement
  the `choose_package_version` method of a `DependencyProvider`.
- Implement `FromStr` for `SemanticVersion`.
- Add the `VersionParseError` type for parsing of semantic versions.

#### Changed

- Using SPDX license identifiers instead of MPL-2.0 classic file headers.
- `./github/workflows/` CI workflow was improved, including check for conventional commits.
- The `Solver` trait was replaced by a `DependencyProvider` trait
  which now must implement a `choose_package_version` method
  instead of `list_available_versions`.
  So it now has the ability to choose a package in addition to a version.
  The `DependencyProvider` also has a new optional method `should_cancel`
  that may be used to stop the solver if needed.
- The `choose_package_version` and `get_dependencies` methods of the
  `DependencyProvider` trait now take an immutable reference to `self`.
  Interior mutability can be used by implementor if mutability is needed.
- The `Solver.run` method was thus replaced by a free function `solver::resolve`
  taking a dependency provider as first argument.
- The `OfflineSolver` is thus replaced by an `OfflineDependencyProvider`.
- `SemanticVersion` now takes `u32` instead of `usize` for its 3 parts.
- `NumberVersion` now uses `u32` instead of `usize`.

#### Removed

- `ErrorRetrievingVersions` variant of `error::PubGrubError`.

### Changes in the internal parts of the API

#### Added

- `benches/large_case.rs` enables benchmarking of serialized registries of packages.
- `examples/caching_dependency_provider.rs` an example dependency provider caching dependencies.
- `PackageTerm<P, V> = (P, Term<V>)` new type alias for readability.
- `Memory.term_intersection_for_package(&mut self, package: &P) -> Option<&Term<V>>`
- New types were introduces for conflict resolution in `internal::partial_solution`
  to clarify the intent and return values of some functions.
  Those types are `DatedAssignment` and `SatisfierAndPreviousHistory`.
- `PartialSolution.term_intersection_for_package` calling the same function
  from its `memory`.
- New property tests for ranges: `negate_contains_opposite`, `intesection_contains_both`
  and `union_contains_either`.
- A large synthetic test case was added in `test-examples/`.
- A new test example `double_choices` was added
  for the detection of a bug (fixed) in the implementation.
- Property testing of big synthetic datasets was added in `tests/proptest.rs`.
- Comparison of PubGrub solver and a SAT solver
  was added with `tests/sat_dependency_provider.rs`.
- Other regression and unit tests were added to `tests/tests.rs`.

#### Changed

- `State.incompatibilities` is now wrapped inside a `Rc`.
- `DecisionLevel(u32)` is used in place of `usize` for partial solution decision levels.
- `State.conflict_resolution` now also returns the almost satisfied package
  to avoid an unnecessary call to `self.partial_solution.relation(...)` after conflict resolution.
- `Kind::NoVersion` renamed to `Kind::NoVersions` and all other usage of `noversion`
  has been changed to `no_versions`.
- Variants of the `incompatibility::Relation` enum have changed.
- Incompatibility now uses a deterministic hasher to store packages in its hash map.
- `incompatibility.relation(...)` now takes a function as argument to avoid computations
  of unnecessary terms intersections.
- `Memory` now uses a deterministic hasher instead of the default one.
- `memory::PackageAssignments` is now an enum instead of a struct.
- Derivations in a `PackageAssignments` keep a precomputed intersection of derivation terms.
- `potential_packages` method now returns a `Range`
  instead of a `Term` for the versions constraint of each package.
- `PartialSolution.relation` now takes `&mut self` instead of `&self`
  to be able to store computation of terms intersection.
- `Term.accept_version` was renamed `Term.contains`.
- The `satisfied_by` and `contradicted_by` methods of a `Term`
  now directly takes a reference to the intersection of other terms.
  Same for `relation_with`.

#### Removed

- `term` field of an `Assignment::Derivation` variant.
- `Memory.all_terms` method was removed.
- `Memory.remove_decision` method was removed in favor of a check before using `Memory.add_decision`.
- `PartialSolution` methods `pick_package` and `pick_version` have been removed
  since control was given back to the dependency provider to choose a package version.
- `PartialSolution` methods `remove_last_decision` and `satisfies_any_of` were removed
  in favor of a preventive check before calling `add_decision`.
- `Term.is_negative`.

#### Fixed

- Prior cause computation (`incompatibility::prior_cause`) now uses the intersection of package terms
  instead of their union, which was an implementation error.

## [0.1.0] - 2020-10-01

### Added

- `README.md` as the home page of this repository.
- `LICENSE`, code is provided under the MPL 2.0 license.
- `Cargo.toml` configuration of this Rust project.
- `src/` containing all the source code for this first implementation of PubGrub in Rust.
- `tests/` containing test end-to-end examples.
- `examples/` other examples, not in the form of tests.
- `.gitignore` configured for a Rust project.
- `.github/workflows/` CI to automatically build, test and document on push and pull requests.

[0.2.0]: https://github.com/pubgrub-rs/pubgrub/releases/tag/v0.2.0
[0.1.0]: https://github.com/pubgrub-rs/pubgrub/releases/tag/v0.1.0
[diff-unreleased]: https://github.com/pubgrub-rs/pubgrub/compare/release...dev
[diff-0.2.0]: https://github.com/mpizenberg/elm-pointer-events/compare/v0.1.0...v0.2.0
