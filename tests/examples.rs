use std::collections::HashMap;

use pubgrub::range::Range;
use pubgrub::solver::{OfflineSolver, Solver};
use pubgrub::version::SemanticVersion;

#[test]
/// https://github.com/dart-lang/pub/blob/master/doc/solver.md#no-conflicts
fn no_conflict() {
    let mut solver = OfflineSolver::<&str, SemanticVersion>::new();
    #[rustfmt::skip]
    solver.add_dependencies(
        "root", (1, 0, 0),
        vec![("foo", Range::between((1, 0, 0), (2, 0, 0)))],
    );
    #[rustfmt::skip]
    solver.add_dependencies(
        "foo", (1, 0, 0),
        vec![("bar", Range::between((1, 0, 0), (2, 0, 0)))],
    );
    solver.add_dependencies("bar", (1, 0, 0), vec![]);
    solver.add_dependencies("bar", (2, 0, 0), vec![]);

    // Run the solver.
    let solver_solution = solver.run("root", (1, 0, 0)).unwrap();

    // Solution.
    let mut solution = HashMap::new();
    solution.insert("root", (1, 0, 0).into());
    solution.insert("foo", (1, 0, 0).into());
    solution.insert("bar", (1, 0, 0).into());

    // Comparing the true solution with the one computed by the solver.
    assert_eq!(solution, solver_solution);
}

#[test]
/// https://github.com/dart-lang/pub/blob/master/doc/solver.md#avoiding-conflict-during-decision-making
fn avoiding_conflict_during_decision_making() {
    let mut solver = OfflineSolver::<&str, SemanticVersion>::new();
    #[rustfmt::skip]
    solver.add_dependencies(
        "root", (1, 0, 0),
        vec![
            ("foo", Range::between((1, 0, 0), (2, 0, 0))),
            ("bar", Range::between((1, 0, 0), (2, 0, 0))),
        ],
    );
    #[rustfmt::skip]
    solver.add_dependencies(
        "foo", (1, 1, 0),
        vec![("bar", Range::between((2, 0, 0), (3, 0, 0)))],
    );
    solver.add_dependencies("foo", (1, 0, 0), vec![]);
    solver.add_dependencies("bar", (1, 0, 0), vec![]);
    solver.add_dependencies("bar", (1, 1, 0), vec![]);
    solver.add_dependencies("bar", (2, 0, 0), vec![]);

    // Run the solver.
    let solver_solution = solver.run("root", (1, 0, 0)).unwrap();

    // Solution.
    let mut solution = HashMap::new();
    solution.insert("root", (1, 0, 0).into());
    solution.insert("foo", (1, 0, 0).into());
    solution.insert("bar", (1, 1, 0).into());

    // Comparing the true solution with the one computed by the solver.
    assert_eq!(solution, solver_solution);
}

#[test]
/// https://github.com/dart-lang/pub/blob/master/doc/solver.md#performing-conflict-resolution
fn conflict_resolution() {
    let mut solver = OfflineSolver::<&str, SemanticVersion>::new();
    #[rustfmt::skip]
    solver.add_dependencies(
        "root", (1, 0, 0),
        vec![("foo", Range::higher_than((1, 0, 0)))],
    );
    #[rustfmt::skip]
    solver.add_dependencies(
        "foo", (2, 0, 0),
        vec![("bar", Range::between((1, 0, 0), (2, 0, 0)))],
    );
    solver.add_dependencies("foo", (1, 0, 0), vec![]);
    #[rustfmt::skip]
    solver.add_dependencies(
        "bar", (1, 0, 0),
        vec![("foo", Range::between((1, 0, 0), (2, 0, 0)))],
    );

    // Run the solver.
    let solver_solution = solver.run("root", (1, 0, 0)).unwrap();

    // Solution.
    let mut solution = HashMap::new();
    solution.insert("root", (1, 0, 0).into());
    solution.insert("foo", (1, 0, 0).into());

    // Comparing the true solution with the one computed by the solver.
    assert_eq!(solution, solver_solution);
}

#[test]
/// https://github.com/dart-lang/pub/blob/master/doc/solver.md#conflict-resolution-with-a-partial-satisfier
fn conflict_with_partial_satisfier() {
    let mut solver = OfflineSolver::<&str, SemanticVersion>::new();
    #[rustfmt::skip]
    // root 1.0.0 depends on foo ^1.0.0 and target ^2.0.0
    solver.add_dependencies(
        "root", (1, 0, 0),
        vec![
            ("foo", Range::between((1, 0, 0), (2, 0, 0))),
            ("target", Range::between((2, 0, 0), (3, 0, 0))),
        ],
    );
    #[rustfmt::skip]
    // foo 1.1.0 depends on left ^1.0.0 and right ^1.0.0
    solver.add_dependencies(
        "foo", (1, 1, 0),
        vec![
            ("left", Range::between((1, 0, 0), (2, 0, 0))),
            ("right", Range::between((1, 0, 0), (2, 0, 0))),
        ],
    );
    solver.add_dependencies("foo", (1, 0, 0), vec![]);
    #[rustfmt::skip]
    // left 1.0.0 depends on shared >=1.0.0
    solver.add_dependencies(
        "left", (1, 0, 0),
        vec![("shared", Range::higher_than((1, 0, 0)))],
    );
    #[rustfmt::skip]
    // right 1.0.0 depends on shared <2.0.0
    solver.add_dependencies(
        "right", (1, 0, 0),
        vec![("shared", Range::strictly_lower_than((2, 0, 0)))],
    );
    solver.add_dependencies("shared", (2, 0, 0), vec![]);
    #[rustfmt::skip]
    // shared 1.0.0 depends on target ^1.0.0
    solver.add_dependencies(
        "shared", (1, 0, 0),
        vec![("target", Range::between((1, 0, 0), (2, 0, 0)))],
    );
    solver.add_dependencies("target", (2, 0, 0), vec![]);
    solver.add_dependencies("target", (1, 0, 0), vec![]);

    // Run the solver.
    let solver_solution = solver.run("root", (1, 0, 0)).unwrap();

    // Solution.
    let mut solution = HashMap::new();
    solution.insert("root", (1, 0, 0).into());
    solution.insert("foo", (1, 0, 0).into());
    solution.insert("target", (2, 0, 0).into());

    // Comparing the true solution with the one computed by the solver.
    assert_eq!(solution, solver_solution);
}
