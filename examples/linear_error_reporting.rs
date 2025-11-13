// SPDX-License-Identifier: MPL-2.0

use astral_pubgrub::{
    DefaultStringReporter, OfflineDependencyProvider, PubGrubError, Ranges, Reporter,
    SemanticVersion, resolve,
};

type SemVS = Ranges<SemanticVersion>;

// https://github.com/dart-lang/pub/blob/master/doc/solver.md#linear-error-reporting
fn main() {
    let mut dependency_provider = OfflineDependencyProvider::<&str, SemVS>::new();
    #[rustfmt::skip]
    // root 1.0.0 depends on foo ^1.0.0 and baz ^1.0.0
        dependency_provider.add_dependencies(
        "root", (1, 0, 0),
        [
            ("foo", Ranges::from_range_bounds((1, 0, 0)..(2, 0, 0))),
            ("baz", Ranges::from_range_bounds((1, 0, 0)..(2, 0, 0))),
        ],
    );
    #[rustfmt::skip]
    // foo 1.0.0 depends on bar ^2.0.0
        dependency_provider.add_dependencies(
        "foo", (1, 0, 0),
        [("bar", Ranges::from_range_bounds((2, 0, 0)..(3, 0, 0)))],
    );
    #[rustfmt::skip]
    // bar 2.0.0 depends on baz ^3.0.0
        dependency_provider.add_dependencies(
        "bar", (2, 0, 0),
        [("baz", Ranges::from_range_bounds((3, 0, 0)..(4, 0, 0)))],
    );
    // baz 1.0.0 and 3.0.0 have no dependencies
    dependency_provider.add_dependencies("baz", (1, 0, 0), []);
    dependency_provider.add_dependencies("baz", (3, 0, 0), []);

    // Run the algorithm.
    match resolve(&dependency_provider, "root", (1, 0, 0)) {
        Ok(sol) => println!("{sol:?}"),
        Err(PubGrubError::NoSolution(mut derivation_tree)) => {
            derivation_tree.collapse_no_versions();
            eprintln!("{}", DefaultStringReporter::report(&derivation_tree));
            std::process::exit(1);
        }
        Err(err) => panic!("{err:?}"),
    };
}
