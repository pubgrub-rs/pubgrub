use pubgrub::cache::{Cache, SimpleCache};
use pubgrub::error::PubGrubError;
use pubgrub::range::Range;
use pubgrub::report::{DefaultStringReporter, Reporter};
use pubgrub::solver::Solver;
use pubgrub::version::SemanticVersion;

// https://github.com/dart-lang/pub/blob/master/doc/solver.md#linear-error-reporting
fn main() {
    let mut solver = SimpleCache::new();
    #[rustfmt::skip]
    // root 1.0.0 depends on foo ^1.0.0 and baz ^1.0.0
    solver.add_dependencies(
        "root", version(1, 0, 0),
        vec![
            ("foo", Range::between(version(1, 0, 0), version(2, 0, 0))),
            ("baz", Range::between(version(1, 0, 0), version(2, 0, 0))),
        ].into_iter(),
    );
    #[rustfmt::skip]
    // foo 1.0.0 depends on bar ^2.0.0
    solver.add_dependencies(
        "foo", version(1, 0, 0),
        vec![("bar", Range::between(version(2, 0, 0), version(3, 0, 0)))].into_iter(),
    );
    #[rustfmt::skip]
    // bar 2.0.0 depends on baz ^3.0.0
    solver.add_dependencies(
        "bar", version(2, 0, 0),
        vec![("baz", Range::between(version(3, 0, 0), version(4, 0, 0)))].into_iter(),
    );
    // baz 1.0.0 and 3.0.0 have no dependencies
    solver.add_dependencies("baz", version(1, 0, 0), vec![].into_iter());
    solver.add_dependencies("baz", version(3, 0, 0), vec![].into_iter());

    // Run the solver.
    match solver.run(&"root", &version(1, 0, 0)) {
        Ok(sol) => println!("{:?}", sol),
        Err(PubGrubError::NoSolution(mut derivation_tree)) => {
            derivation_tree.collapse_noversion();
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
