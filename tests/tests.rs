// SPDX-License-Identifier: MPL-2.0

use pubgrub::error::PubGrubError;
use pubgrub::range::Range;
use pubgrub::solver::{resolve, OfflineDependencyProvider};
use pubgrub::type_aliases::Map;
use pubgrub::version::NumberVersion;

type NumVS = Range<NumberVersion>;

#[test]
fn constrains_are_not_in_solution() {
    let mut dependency_provider = OfflineDependencyProvider::<_, NumVS>::new();

    dependency_provider.add_dependencies("a", 0, []);
    dependency_provider.add_constraints("a", 0, [("c", Range::singleton(1))]);

    // Run the algorithm
    let computed_solution =
        resolve(&dependency_provider, "a", NumberVersion(0)).expect("a solution was not found");

    // Solution.
    let mut expected_solution = Map::default();
    expected_solution.insert("a", NumberVersion(0));

    assert_eq!(computed_solution, expected_solution);
}

#[test]
fn constrains_affect_the_solution() {
    let mut dependency_provider = OfflineDependencyProvider::<_, NumVS>::new();

    dependency_provider.add_dependencies("a", 0, [("b", Range::full())]);
    dependency_provider.add_constraints("a", 0, [("c", Range::singleton(0))]);
    dependency_provider.add_dependencies("b", 0, [("c", Range::full())]);
    dependency_provider.add_dependencies("c", 0, []);
    dependency_provider.add_dependencies("c", 1, []);

    // Run the algorithm
    let computed_solution =
        resolve(&dependency_provider, "a", NumberVersion(0)).expect("a solution was not found");

    // Solution.
    let mut expected_solution = Map::default();
    expected_solution.insert("a", NumberVersion(0));
    expected_solution.insert("b", NumberVersion(0));
    expected_solution.insert("c", NumberVersion(0));

    assert_eq!(computed_solution, expected_solution);
}

#[test]
fn constrains_can_exclude_dependency_from_the_solution() {
    let mut dependency_provider = OfflineDependencyProvider::<_, NumVS>::new();

    // "a" depends on "b" but requires that "c" is not included through an empty range constraint.
    // version 0 of "b" depends on nothing
    // version 1 of "b" depends on "c"
    dependency_provider.add_dependencies("a", 0, [("b", Range::full())]);
    dependency_provider.add_constraints("a", 0, [("c", Range::empty())]);
    dependency_provider.add_dependencies("b", 0, []);
    dependency_provider.add_dependencies("b", 1, [("c", Range::full())]);
    dependency_provider.add_dependencies("c", 0, []);

    // Run the algorithm
    let computed_solution =
        resolve(&dependency_provider, "a", NumberVersion(0)).expect("a solution was not found");

    // Solution.
    //
    // - The expected result is that "b" version 0 is selected over version 1 because that latest
    //   version (1) depends on the 'illegal' package "c".
    // - Package "c" is not included.
    let mut expected_solution = Map::default();
    expected_solution.insert("a", NumberVersion(0));
    expected_solution.insert("b", NumberVersion(0));

    assert_eq!(computed_solution, expected_solution);
}

#[test]
fn same_result_on_repeated_runs() {
    let mut dependency_provider = OfflineDependencyProvider::<_, NumVS>::new();

    dependency_provider.add_dependencies("c", 0, []);
    dependency_provider.add_dependencies("c", 2, []);
    dependency_provider.add_dependencies("b", 0, []);
    dependency_provider.add_dependencies("b", 1, [("c", Range::between(0, 1))]);

    dependency_provider.add_dependencies("a", 0, [("b", Range::full()), ("c", Range::full())]);

    let name = "a";
    let ver = NumberVersion(0);
    let one = resolve(&dependency_provider, name, ver);
    for _ in 0..10 {
        match (&one, &resolve(&dependency_provider, name, ver)) {
            (Ok(l), Ok(r)) => assert_eq!(l, r),
            _ => panic!("not the same result"),
        }
    }
}

#[test]
fn should_always_find_a_satisfier() {
    let mut dependency_provider = OfflineDependencyProvider::<_, NumVS>::new();
    dependency_provider.add_dependencies("a", 0, [("b", Range::empty())]);
    assert!(matches!(
        resolve(&dependency_provider, "a", 0),
        Err(PubGrubError::DependencyOnTheEmptySet { .. })
    ));

    dependency_provider.add_dependencies("c", 0, [("a", Range::full())]);
    assert!(matches!(
        resolve(&dependency_provider, "c", 0),
        Err(PubGrubError::DependencyOnTheEmptySet { .. })
    ));
}

#[test]
fn cannot_depend_on_self() {
    let mut dependency_provider = OfflineDependencyProvider::<_, NumVS>::new();
    dependency_provider.add_dependencies("a", 0, [("a", Range::full())]);
    assert!(matches!(
        resolve(&dependency_provider, "a", 0),
        Err(PubGrubError::SelfDependency { .. })
    ));
}
