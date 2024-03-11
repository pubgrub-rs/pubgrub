// SPDX-License-Identifier: MPL-2.0

use pubgrub::error::PubGrubError;
use pubgrub::range::Range;
use pubgrub::solver::{resolve, OfflineDependencyProvider};
use pubgrub::version::NumberVersion;

type NumVS = Range<NumberVersion>;

#[test]
fn same_result_on_repeated_runs() {
    let mut dependency_provider = OfflineDependencyProvider::<_, NumVS>::new();

    dependency_provider.add_dependencies("c", 0, []);
    dependency_provider.add_dependencies("c", 2, []);
    dependency_provider.add_dependencies("b", 0, []);
    dependency_provider.add_dependencies("b", 1, [("c", Range::between(0, 1))]);

    dependency_provider.add_dependencies("a", 0, [("b", Range::full()), ("c", Range::full())]);

    let name = "a";
    let ver = NumberVersion(0);
    let one = resolve(&dependency_provider, name, ver);
    for _ in 0..10 {
        match (&one, &resolve(&dependency_provider, name, ver)) {
            (Ok(l), Ok(r)) => assert_eq!(l, r),
            _ => panic!("not the same result"),
        }
    }
}

#[test]
fn should_always_find_a_satisfier() {
    let mut dependency_provider = OfflineDependencyProvider::<_, NumVS>::new();
    dependency_provider.add_dependencies("a", 0, [("b", Range::empty())]);
    assert!(matches!(
        resolve(&dependency_provider, "a", 0),
        Err(PubGrubError::NoSolution { .. })
    ));

    dependency_provider.add_dependencies("c", 0, [("a", Range::full())]);
    assert!(matches!(
        resolve(&dependency_provider, "c", 0),
        Err(PubGrubError::NoSolution { .. })
    ));
}

#[test]
fn cannot_depend_on_self() {
    let mut dependency_provider = OfflineDependencyProvider::<_, NumVS>::new();
    dependency_provider.add_dependencies("a", 0, [("a", Range::full())]);
    assert!(matches!(
        resolve(&dependency_provider, "a", 0),
        Err(PubGrubError::SelfDependency { .. })
    ));
}

#[test]
fn redundant_scanning() {
    let mut dependency_provider = OfflineDependencyProvider::<u32, NumVS>::new();
    dependency_provider.add_dependencies(
        0,
        0,
        [
            (247, Range::higher_than(1)),
            (71, Range::strictly_lower_than(7)),
            (69, Range::higher_than(4)),
            (1, Range::full()),
        ],
    );

    dependency_provider.add_dependencies(1, 0, [(646, Range::singleton(0))]);
    dependency_provider.add_dependencies(1, 1, [(247, Range::strictly_lower_than(3))]);
    dependency_provider.add_dependencies(1, 2, [(71, Range::higher_than(7))]);
    dependency_provider.add_dependencies(1, 3, [(66, Range::full())]);
    dependency_provider.add_dependencies(1, 4, [(2, Range::empty())]);
    dependency_provider.add_dependencies(1, 5, [(3, Range::empty())]);
    dependency_provider.add_dependencies(1, 6, [(248, Range::from_range_bounds(1..3))]);
    dependency_provider.add_dependencies(1, 7, [(247, Range::singleton(0))]);
    dependency_provider.add_dependencies(1, 8, [(247, Range::singleton(1))]);
    dependency_provider.add_dependencies(1, 9, [(248, Range::singleton(1))]);
    dependency_provider.add_dependencies(1, 10, [(71, Range::singleton(7))]);
    dependency_provider.add_dependencies(1, 11, [(99, Range::full())]);
    dependency_provider.add_dependencies(1, 12, [(69, Range::strictly_lower_than(6))]);

    dependency_provider.add_dependencies(66, 0, [(69, Range::strictly_lower_than(5))]);

    dependency_provider.add_dependencies(
        69,
        7,
        [(176, Range::singleton(3)), (248, Range::singleton(0))],
    );
    dependency_provider.add_dependencies(
        69,
        10,
        [(248, Range::singleton(0)), (646, Range::singleton(3))],
    );

    dependency_provider.add_dependencies(99, 0, [(248, Range::singleton(1))]);

    dependency_provider.add_dependencies(71, 0, []);
    dependency_provider.add_dependencies(71, 2, []);
    dependency_provider.add_dependencies(71, 3, []);
    dependency_provider.add_dependencies(71, 4, []);
    dependency_provider.add_dependencies(71, 5, []);

    dependency_provider.add_dependencies(247, 3, []);
    dependency_provider.add_dependencies(247, 6, [(249, Range::empty())]);

    dependency_provider.add_dependencies(248, 0, []);
    dependency_provider.add_dependencies(248, 1, []);

    dependency_provider.add_dependencies(646, 0, []);
    dependency_provider.add_dependencies(646, 3, []);

    _ = resolve(&dependency_provider, 0, 0);
}
