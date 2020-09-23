use std::collections::HashMap;

use pubgrub::cache::{Cache, SimpleCache};
use pubgrub::range::Range;
use pubgrub::solver::Solver;
use pubgrub::version::SemanticVersion;

#[test]
/// https://github.com/dart-lang/pub/blob/master/doc/solver.md#no-conflicts
fn no_conflict() {
    let mut solver = SimpleCache::new();
    #[rustfmt::skip]
    solver.add_dependencies(
        "root", version(1, 0, 0),
        vec![("foo", Range::between(version(1, 0, 0), version(2, 0, 0)))].into_iter(),
    );
    #[rustfmt::skip]
    solver.add_dependencies(
        "foo", version(1, 0, 0),
        vec![("bar", Range::between(version(1, 0, 0), version(2, 0, 0)))].into_iter(),
    );
    solver.add_dependencies("bar", version(1, 0, 0), vec![].into_iter());
    solver.add_dependencies("bar", version(2, 0, 0), vec![].into_iter());

    // Run the solver.
    let solver_solution = solver.run(&"root", &version(1, 0, 0)).unwrap();

    // Solution.
    let mut solution = HashMap::new();
    solution.insert("root", version(1, 0, 0));
    solution.insert("foo", version(1, 0, 0));
    solution.insert("bar", version(1, 0, 0));

    // Comparing the true solution with the one computed by the solver.
    assert_eq!(solution, solver_solution);
}

#[test]
/// https://github.com/dart-lang/pub/blob/master/doc/solver.md#avoiding-conflict-during-decision-making
fn avoiding_conflict_during_decision_making() {
    let mut solver = SimpleCache::new();
    #[rustfmt::skip]
    solver.add_dependencies(
        "root", version(1, 0, 0),
        vec![
            ("foo", Range::between(version(1, 0, 0), version(2, 0, 0))),
            ("bar", Range::between(version(1, 0, 0), version(2, 0, 0))),
        ].into_iter(),
    );
    #[rustfmt::skip]
    solver.add_dependencies(
        "foo", version(1, 1, 0),
        vec![("bar", Range::between(version(2, 0, 0), version(3, 0, 0)))].into_iter(),
    );
    solver.add_dependencies("foo", version(1, 0, 0), vec![].into_iter());
    solver.add_dependencies("bar", version(1, 0, 0), vec![].into_iter());
    solver.add_dependencies("bar", version(1, 1, 0), vec![].into_iter());
    solver.add_dependencies("bar", version(2, 0, 0), vec![].into_iter());

    // Run the solver.
    let solver_solution = solver.run(&"root", &version(1, 0, 0)).unwrap();

    // Solution.
    let mut solution = HashMap::new();
    solution.insert("root", version(1, 0, 0));
    solution.insert("foo", version(1, 0, 0));
    solution.insert("bar", version(1, 1, 0));

    // Comparing the true solution with the one computed by the solver.
    assert_eq!(solution, solver_solution);
}

#[test]
/// https://github.com/dart-lang/pub/blob/master/doc/solver.md#performing-conflict-resolution
fn conflict_resolution() {
    let mut solver = SimpleCache::new();
    #[rustfmt::skip]
    solver.add_dependencies(
        "root", version(1, 0, 0),
        vec![("foo", Range::higher_than(version(1, 0, 0)))].into_iter(),
    );
    #[rustfmt::skip]
    solver.add_dependencies(
        "foo", version(2, 0, 0),
        vec![("bar", Range::between(version(1, 0, 0), version(2, 0, 0)))].into_iter(),
    );
    solver.add_dependencies("foo", version(1, 0, 0), vec![].into_iter());
    #[rustfmt::skip]
    solver.add_dependencies(
        "bar", version(1, 0, 0),
        vec![("foo", Range::between(version(1, 0, 0), version(2, 0, 0)))].into_iter(),
    );

    // Run the solver.
    let solver_solution = solver.run(&"root", &version(1, 0, 0)).unwrap();

    // Solution.
    let mut solution = HashMap::new();
    solution.insert("root", version(1, 0, 0));
    solution.insert("foo", version(1, 0, 0));

    // Comparing the true solution with the one computed by the solver.
    assert_eq!(solution, solver_solution);
}

#[test]
/// https://github.com/dart-lang/pub/blob/master/doc/solver.md#conflict-resolution-with-a-partial-satisfier
fn conflict_with_partial_satisfier() {
    let mut solver = SimpleCache::new();
    #[rustfmt::skip]
    // root 1.0.0 depends on foo ^1.0.0 and target ^2.0.0
    solver.add_dependencies(
        "root", version(1, 0, 0),
        vec![
            ("foo", Range::between(version(1, 0, 0), version(2, 0, 0))),
            ("target", Range::between(version(2, 0, 0), version(3, 0, 0))),
        ]
        .into_iter(),
    );
    #[rustfmt::skip]
    // foo 1.1.0 depends on left ^1.0.0 and right ^1.0.0
    solver.add_dependencies(
        "foo", version(1, 1, 0),
        vec![
            ("left", Range::between(version(1, 0, 0), version(2, 0, 0))),
            ("right", Range::between(version(1, 0, 0), version(2, 0, 0))),
        ]
        .into_iter(),
    );
    solver.add_dependencies("foo", version(1, 0, 0), vec![].into_iter());
    #[rustfmt::skip]
    // left 1.0.0 depends on shared >=1.0.0
    solver.add_dependencies(
        "left", version(1, 0, 0),
        vec![("shared", Range::higher_than(version(1, 0, 0)))].into_iter(),
    );
    #[rustfmt::skip]
    // right 1.0.0 depends on shared <2.0.0
    solver.add_dependencies(
        "right", version(1, 0, 0),
        vec![("shared", Range::strictly_lower_than(version(2, 0, 0)))].into_iter(),
    );
    solver.add_dependencies("shared", version(2, 0, 0), vec![].into_iter());
    #[rustfmt::skip]
    // shared 1.0.0 depends on target ^1.0.0
    solver.add_dependencies(
        "shared", version(1, 0, 0),
        vec![("target", Range::between(version(1, 0, 0), version(2, 0, 0)))].into_iter(),
    );
    solver.add_dependencies("target", version(2, 0, 0), vec![].into_iter());
    solver.add_dependencies("target", version(1, 0, 0), vec![].into_iter());

    // Run the solver.
    let solver_solution = solver.run(&"root", &version(1, 0, 0)).unwrap();

    // Solution.
    let mut solution = HashMap::new();
    solution.insert("root", version(1, 0, 0));
    solution.insert("foo", version(1, 0, 0));
    solution.insert("target", version(2, 0, 0));

    // Comparing the true solution with the one computed by the solver.
    assert_eq!(solution, solver_solution);
}

/// helper functions to create versions.
fn version(major: usize, minor: usize, patch: usize) -> SemanticVersion {
    SemanticVersion::new(major, minor, patch)
}
