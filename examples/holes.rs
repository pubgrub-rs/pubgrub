use pubgrub::error::PubGrubError;
use pubgrub::range::Range;
use pubgrub::report::{DefaultStringReporter, Reporter};
use pubgrub::solver::{resolve, OfflineDependencyProvider};
use pubgrub::version::SemanticVersion;
use rustc_hash::FxHashMap;

fn main() {
    let mut dependency_provider = OfflineDependencyProvider::<&str, Range<SemanticVersion>>::new();

    // root depends on foo...
    dependency_provider.add_dependencies("root", (1, 0, 0), vec![("foo", Range::full())]);

    for i in 1..5 {
        // foo depends on bar...
        dependency_provider.add_dependencies("foo", (i, 0, 0), vec![("bar", Range::full())]);
    }

    let mut versions = FxHashMap::default();
    for package in dependency_provider.packages() {
        versions.insert(
            *package,
            dependency_provider
                .versions(package)
                .expect("Package must have versions")
                .cloned()
                .collect::<Vec<_>>(),
        );
    }

    match resolve(&dependency_provider, "root", (1, 0, 0)) {
        Ok(sol) => println!("{:?}", sol),
        Err(PubGrubError::NoSolution(derivation_tree)) => {
            let simple_deriviation_tree = derivation_tree.simplify_versions(&versions);
            eprintln!("{}", DefaultStringReporter::report(&derivation_tree));
            eprintln!("\n----------\n");
            eprintln!(
                "{}",
                DefaultStringReporter::report(&simple_deriviation_tree)
            );
        }
        Err(err) => panic!("{:?}", err),
    };
}
