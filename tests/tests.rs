use pubgrub::range::Range;
use pubgrub::solver::{OfflineSolver, Solver};
use pubgrub::version::NumberVersion;

#[test]
fn same_result_on_repeated_runs() {
    let mut solver = OfflineSolver::<_, NumberVersion>::new();

    solver.add_dependencies("c", 0, vec![]);
    solver.add_dependencies("c", 2, vec![]);
    solver.add_dependencies("b", 0, vec![]);
    solver.add_dependencies("b", 1, vec![("c", Range::between(0, 1))]);

    solver.add_dependencies("a", 0, vec![("b", Range::any()), ("c", Range::any())]);

    let name = "a";
    let ver = NumberVersion(0);
    let one = solver.run(name, ver);
    for _ in 0..10 {
        match (&one, &solver.run(name, ver)) {
            (Ok(l), Ok(r)) => assert_eq!(l, r),
            _ => panic!("not the same result"),
        }
    }
}
