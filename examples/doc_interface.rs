use pubgrub::cache::{Cache, SimpleCache};
use pubgrub::range::Range;
use pubgrub::solver::Solver;
use pubgrub::version::NumberVersion;

// `root` depends on `menu` and `icons`
// `menu` depends on `dropdown`
// `dropdown` depends on `icons`
// `icons` has no dependency
#[rustfmt::skip]
fn main() {
    let mut solver = SimpleCache::<&str, NumberVersion>::new();
    solver.add_dependencies(
        "root", 1, vec![("menu", Range::any()), ("icons", Range::any())],
    );
    solver.add_dependencies("menu", 1, vec![("dropdown", Range::any())]);
    solver.add_dependencies("dropdown", 1, vec![("icons", Range::any())]);
    solver.add_dependencies("icons", 1, vec![]);

    // Run the solver.
    let _solution = solver.run("root", 1).unwrap();
}
