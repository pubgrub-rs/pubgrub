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

/// helper functions to create versions.
fn version(major: usize, minor: usize, patch: usize) -> SemanticVersion {
    SemanticVersion::new(major, minor, patch)
}
