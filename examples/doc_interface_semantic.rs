use pubgrub::error::PubGrubError;
use pubgrub::range::Range;
use pubgrub::report::{DefaultStringReporter, Reporter};
use pubgrub::solver::{OfflineSolver, Solver};
use pubgrub::version::SemanticVersion;

// `root` depends on `menu` and `icons 1.0.0`
// `menu 1.0.0` depends on `dropdown < 2.0.0`
// `menu >= 1.1.0` depends on `dropdown >= 2.0.0`
// `dropdown 1.8.0` has no dependency
// `dropdown >= 2.0.0` depends on `icons 2.0.0`
// `icons` has no dependency
#[rustfmt::skip]
fn main() {
    let mut solver = OfflineSolver::<&str, SemanticVersion>::new();
    // Direct dependencies: menu and icons.
    solver.add_dependencies("root", (1, 0, 0), vec![
        ("menu", Range::any()),
        ("icons", Range::exact((1, 0, 0))),
    ]);

    // Dependencies of the menu lib.
    solver.add_dependencies("menu", (1, 0, 0), vec![
        ("dropdown", Range::strictly_lower_than((2, 0, 0))),
    ]);
    solver.add_dependencies("menu", (1, 1, 0), vec![
        ("dropdown", Range::higher_than((2, 0, 0))),
    ]);
    solver.add_dependencies("menu", (1, 2, 0), vec![
        ("dropdown", Range::higher_than((2, 0, 0))),
    ]);
    solver.add_dependencies("menu", (1, 3, 0), vec![
        ("dropdown", Range::higher_than((2, 0, 0))),
    ]);
    solver.add_dependencies("menu", (1, 4, 0), vec![
        ("dropdown", Range::higher_than((2, 0, 0))),
    ]);
    solver.add_dependencies("menu", (1, 5, 0), vec![
        ("dropdown", Range::higher_than((2, 0, 0))),
    ]);

    // Dependencies of the dropdown lib.
    solver.add_dependencies("dropdown", (1, 8, 0), vec![]);
    solver.add_dependencies("dropdown", (2, 0, 0), vec![
        ("icons", Range::exact((2, 0, 0))),
    ]);
    solver.add_dependencies("dropdown", (2, 1, 0), vec![
        ("icons", Range::exact((2, 0, 0))),
    ]);
    solver.add_dependencies("dropdown", (2, 2, 0), vec![
        ("icons", Range::exact((2, 0, 0))),
    ]);
    solver.add_dependencies("dropdown", (2, 3, 0), vec![
        ("icons", Range::exact((2, 0, 0))),
    ]);

    // Icons has no dependency.
    solver.add_dependencies("icons", (1, 0, 0), vec![]);
    solver.add_dependencies("icons", (2, 0, 0), vec![]);

    // Run the solver.
    match solver.run("root", (1, 0, 0)) {
        Ok(sol) => println!("{:?}", sol),
        Err(PubGrubError::NoSolution(mut derivation_tree)) => {
            derivation_tree.collapse_noversion();
            eprintln!("{}", DefaultStringReporter::report(&derivation_tree));
        }
        Err(err) => panic!("{:?}", err),
    };
}
