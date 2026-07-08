// SPDX-License-Identifier: MPL-2.0

//! Resolution with dependent versions widened over the known versions must behave like
//! resolution with singleton dependent versions.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::internal::{Id, Incompatibility, State};
use crate::{
    Dependencies, DependencyProvider, Map, OfflineDependencyProvider, PackageResolutionStatistics,
    Ranges, Term,
};

type NumVS = Ranges<u32>;
type Provider = OfflineDependencyProvider<&'static str, NumVS>;

/// A `resolve` loop that keeps the version sets minimal by widening the version whose
/// dependencies are added over the known versions.
///
/// Returns `None` if there is no solution.
fn resolve_with_widening(
    provider: &Provider,
    root: &'static str,
    version: u32,
    widen: bool,
) -> Option<BTreeMap<&'static str, u32>> {
    // Sorted version lists per package, the input for the widening.
    let versions: Map<&'static str, Vec<u32>> = provider
        .packages()
        .map(|p| (*p, provider.versions(p).unwrap().copied().collect()))
        .collect();
    let mut widened = false;

    let mut state: State<Provider> = State::init(root, version);
    let mut added_dependencies: Map<Id<&'static str>, BTreeSet<u32>> = Map::default();
    let mut next = state.root_package;
    let solution = loop {
        state.unit_propagation(next).ok()?;

        let Some((package, term_intersection)) =
            state.partial_solution.pick_highest_priority_pkg(|p, r| {
                provider.prioritize(
                    &state.package_store[p],
                    r,
                    &PackageResolutionStatistics::default(),
                )
            })
        else {
            break state
                .partial_solution
                .extract_solution()
                .map(|(p, v)| (state.package_store[p], v))
                .collect();
        };
        next = package;

        let Some(decision) = provider
            .choose_version(&state.package_store[package], term_intersection)
            .unwrap()
        else {
            let inc =
                Incompatibility::no_versions(package, Term::Positive(term_intersection.clone()));
            state.add_incompatibility(inc);
            continue;
        };

        if added_dependencies
            .entry(package)
            .or_default()
            .insert(decision)
        {
            let dependencies = match provider
                .get_dependencies(&state.package_store[package], &decision)
                .unwrap()
            {
                Dependencies::Unavailable(reason) => {
                    state.add_incompatibility(Incompatibility::custom_version(
                        package, decision, reason,
                    ));
                    continue;
                }
                Dependencies::Available(dependencies) => dependencies,
            };
            let version_set = NumVS::singleton(decision);
            let versions = if widen {
                let widened_versions =
                    version_set.widen_versions(versions.get(state.package_store[package]).unwrap());
                widened |= widened_versions != version_set;
                widened_versions
            } else {
                version_set
            };
            state.add_package_version_dependencies(package, decision, versions, dependencies);
        } else {
            state.partial_solution.add_decision(package, decision);
        }
    };
    if widen {
        assert!(widened, "no version set was ever widened");
    }
    Some(solution)
}

/// A registry that requires rejecting versions of "a" and "b" one by one, similar to boto3 and
/// botocore: Without widening, the version sets of "a" accumulate one hole per rejected version,
/// and the incompatibilities of "a" versions depending on "j" merge into growing unions of
/// singletons.
fn backtracking_provider(conflict_below: u32) -> Provider {
    let mut provider = Provider::new();
    provider.add_dependencies(
        "root",
        1u32,
        [
            ("u", Ranges::strictly_lower_than(2u32)),
            ("a", Ranges::full()),
        ],
    );
    provider.add_dependencies("u", 1u32, []);
    provider.add_dependencies("u", 2u32, []);
    for k in 1..=30u32 {
        provider.add_dependencies("a", k, [("b", Ranges::singleton(k)), ("j", Ranges::full())]);
        // Versions of "b" above the threshold conflict with the "u" requirement of root.
        let u_range = if k >= conflict_below {
            Ranges::higher_than(2u32)
        } else {
            Ranges::strictly_lower_than(2u32)
        };
        provider.add_dependencies("b", k, [("u", u_range)]);
    }
    provider.add_dependencies("j", 1u32, []);
    provider
}

#[test]
fn widening_finds_same_solution_after_backtracking() {
    let provider = backtracking_provider(4);
    let expected = BTreeMap::from([("root", 1), ("a", 3), ("b", 3), ("j", 1), ("u", 1)]);

    let without = resolve_with_widening(&provider, "root", 1, false).unwrap();
    let with = resolve_with_widening(&provider, "root", 1, true).unwrap();

    assert_eq!(without, expected);
    assert_eq!(with, expected);
}

#[test]
fn widening_finds_same_conflict() {
    // All versions of "b" conflict, so there is no solution.
    let provider = backtracking_provider(0);

    assert_eq!(resolve_with_widening(&provider, "root", 1, false), None);
    assert_eq!(resolve_with_widening(&provider, "root", 1, true), None);
}
