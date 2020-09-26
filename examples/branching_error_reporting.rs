use pubgrub::cache::{Cache, SimpleCache};
use pubgrub::error::PubGrubError;
use pubgrub::range::Range;
use pubgrub::report::{DefaultStringReporter, Reporter};
use pubgrub::solver::Solver;
use pubgrub::version::SemanticVersion;

// https://github.com/dart-lang/pub/blob/master/doc/solver.md#branching-error-reporting
fn main() {
    let mut solver = SimpleCache::new();
    #[rustfmt::skip]
    // root 1.0.0 depends on foo ^1.0.0
    solver.add_dependencies(
        "root", version(1, 0, 0),
        vec![("foo", Range::between(version(1, 0, 0), version(2, 0, 0)))].into_iter(),
    );
    #[rustfmt::skip]
    // foo 1.0.0 depends on a ^1.0.0 and b ^1.0.0
    solver.add_dependencies(
        "foo", version(1, 0, 0),
        vec![
            ("a", Range::between(version(1, 0, 0), version(2, 0, 0))),
            ("b", Range::between(version(1, 0, 0), version(2, 0, 0))),
        ]
        .into_iter(),
    );
    #[rustfmt::skip]
    // foo 1.1.0 depends on x ^1.0.0 and y ^1.0.0
    solver.add_dependencies(
        "foo", version(1, 1, 0),
        vec![
            ("x", Range::between(version(1, 0, 0), version(2, 0, 0))),
            ("y", Range::between(version(1, 0, 0), version(2, 0, 0))),
        ]
        .into_iter(),
    );
    #[rustfmt::skip]
    // a 1.0.0 depends on b ^2.0.0
    solver.add_dependencies(
        "a", version(1, 0, 0),
        vec![("b", Range::between(version(2, 0, 0), version(3, 0, 0)))].into_iter(),
    );
    // b 1.0.0 and 2.0.0 have no dependencies.
    solver.add_dependencies("b", version(1, 0, 0), vec![].into_iter());
    solver.add_dependencies("b", version(2, 0, 0), vec![].into_iter());
    #[rustfmt::skip]
    // x 1.0.0 depends on y ^2.0.0.
    solver.add_dependencies(
        "x", version(1, 0, 0),
        vec![("y", Range::between(version(2, 0, 0), version(3, 0, 0)))].into_iter(),
    );
    // y 1.0.0 and 2.0.0 have no dependencies.
    solver.add_dependencies("y", version(1, 0, 0), vec![].into_iter());
    solver.add_dependencies("y", version(2, 0, 0), vec![].into_iter());

    // Run the solver.
    match solver.run(&"root", &version(1, 0, 0)) {
        Ok(sol) => println!("{:?}", sol),
        Err(PubGrubError::NoSolution(derivation_tree)) => {
            eprintln!("{}", DefaultStringReporter::report(&derivation_tree));
            std::process::exit(1);
        }
        Err(err) => panic!("{:?}", err),
    };
}

/// helper functions to create versions.
fn version(major: usize, minor: usize, patch: usize) -> SemanticVersion {
    SemanticVersion::new(major, minor, patch)
}
