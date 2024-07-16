// SPDX-License-Identifier: MPL-2.0

use pubgrub::{resolve, OfflineDependencyProvider, PubGrubError, Range};

type NumVS = Range<u32>;

#[test]
fn same_result_on_repeated_runs() {
    let mut dependency_provider = OfflineDependencyProvider::<_, NumVS>::new();

    dependency_provider.add_dependencies("c", 0u32, []);
    dependency_provider.add_dependencies("c", 2u32, []);
    dependency_provider.add_dependencies("b", 0u32, []);
    dependency_provider.add_dependencies("b", 1u32, [("c", Range::between(0u32, 1u32))]);

    dependency_provider.add_dependencies("a", 0u32, [("b", Range::full()), ("c", Range::full())]);

    let name = "a";
    let ver: u32 = 0;
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
    dependency_provider.add_dependencies("a", 0u32, [("b", Range::empty())]);
    assert!(matches!(
        resolve(&dependency_provider, "a", 0u32),
        Err(PubGrubError::NoSolution { .. })
    ));

    dependency_provider.add_dependencies("c", 0u32, [("a", Range::full())]);
    assert!(matches!(
        resolve(&dependency_provider, "c", 0u32),
        Err(PubGrubError::NoSolution { .. })
    ));
}

#[test]
fn cannot_depend_on_self() {
    let mut dependency_provider = OfflineDependencyProvider::<_, NumVS>::new();
    dependency_provider.add_dependencies("a", 0u32, [("a", Range::full())]);
    assert!(matches!(
        resolve(&dependency_provider, "a", 0u32),
        Err(PubGrubError::SelfDependency { .. })
    ));
}
