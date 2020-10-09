// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use pubgrub::{
    error::PubGrubError, package::Package, report::DefaultStringReporter, report::Reporter,
    version::NumberVersion,
};
use pubgrub::{range::Range, solver::resolve, solver::OfflineDependencyProvider};

use proptest::collection::{btree_map, vec};
use proptest::prelude::*;
use proptest::sample::Index;
use proptest::string::string_regex;

fn string_names() -> impl Strategy<Value = String> {
    string_regex("[A-Za-z][A-Za-z0-9_-]{0,5}")
        .unwrap()
        .prop_filter("reseved names", |n| {
            // root is the name of the thing being compiled
            // so it would be confusing to have it in the index
            // bad is a name reserved for a dep that won't work
            n != "root" && n != "bad"
        })
}

/// This generates a random registry index.
/// Unlike vec((Name, Ver, vec((Name, VerRq), ..), ..)
/// This strategy has a high probability of having valid dependencies
pub fn registry_strategy<N: Package + Ord>(
    name: impl Strategy<Value = N>,
    bad_name: N,
) -> impl Strategy<
    Value = (
        OfflineDependencyProvider<N, NumberVersion>,
        Vec<(N, NumberVersion)>,
    ),
> {
    let max_crates = 40;
    let max_versions = 15;
    let shrinkage = 50;
    let complicated_len = 10usize;

    // If this is false than the crate will depend on the nonexistent "bad"
    // instead of the complex set we generated for it.
    let allow_deps = prop::bool::weighted(0.99);

    let a_version = ..(max_versions as u32);

    let list_of_versions = btree_map(a_version, allow_deps, 1..=max_versions)
        .prop_map(move |ver| ver.into_iter().collect::<Vec<_>>());

    let list_of_crates_with_versions = btree_map(name, list_of_versions, 1..=max_crates);

    // each version of each crate can depend on each crate smaller then it.
    // In theory shrinkage should be 2, but in practice we get better trees with a larger value.
    let max_deps = max_versions * (max_crates * (max_crates - 1)) / shrinkage;

    let raw_version_range = (any::<Index>(), any::<Index>());
    let raw_dependency = (any::<Index>(), any::<Index>(), raw_version_range);

    fn order_index(a: Index, b: Index, size: usize) -> (usize, usize) {
        use std::cmp::{max, min};
        let (a, b) = (a.index(size), b.index(size));
        (min(a, b), max(a, b))
    }

    let list_of_raw_dependency = vec(raw_dependency, ..=max_deps);

    // By default a package depends only on other packages that have a smaller name,
    // this helps make sure that all things in the resulting index are DAGs.
    // If this is true then the DAG is maintained with grater instead.
    let reverse_alphabetical = any::<bool>().no_shrink();

    (
        list_of_crates_with_versions,
        list_of_raw_dependency,
        reverse_alphabetical,
        1..(complicated_len + 1),
    )
        .prop_map(
            move |(crate_vers_by_name, raw_dependencies, reverse_alphabetical, complicated_len)| {
                let mut list_of_pkgid: Vec<(
                    (N, NumberVersion),
                    Option<Vec<(N, Range<NumberVersion>)>>,
                )> = crate_vers_by_name
                    .iter()
                    .flat_map(|(name, vers)| {
                        vers.iter().map(move |x| {
                            (
                                (name.clone(), NumberVersion::from(x.0)),
                                if x.1 { Some(vec![]) } else { None },
                            )
                        })
                    })
                    .collect();
                let len_all_pkgid = list_of_pkgid.len();
                for (a, b, (c, d)) in raw_dependencies {
                    let (a, b) = order_index(a, b, len_all_pkgid);
                    let (a, b) = if reverse_alphabetical { (b, a) } else { (a, b) };
                    let ((dep_name, _), _) = list_of_pkgid[a].to_owned();
                    if &(list_of_pkgid[b].0).0 == &dep_name {
                        continue;
                    }
                    let s = &crate_vers_by_name[&dep_name];
                    let s_last_index = s.len() - 1;
                    let (c, d) = order_index(c, d, s.len());

                    if let (_, Some(deps)) = &mut list_of_pkgid[b] {
                        deps.push((
                            dep_name,
                            if c == 0 && d == s_last_index {
                                Range::any()
                            } else if c == 0 {
                                Range::strictly_lower_than(s[d].0 + 1)
                            } else if d == s_last_index {
                                Range::higher_than(s[c].0)
                            } else if c == d {
                                Range::exact(s[c].0)
                            } else {
                                Range::between(s[c].0, s[d].0 + 1)
                            },
                        ))
                    }
                }

                let mut solver = OfflineDependencyProvider::<N, NumberVersion>::new();

                let complicated_len = std::cmp::min(complicated_len, list_of_pkgid.len());
                let complicated: Vec<_> = if reverse_alphabetical {
                    &list_of_pkgid[..complicated_len]
                } else {
                    &list_of_pkgid[(list_of_pkgid.len() - complicated_len)..]
                }
                .iter()
                .map(|(x, _)| (x.0.clone(), x.1))
                .collect();

                for ((name, ver), deps) in list_of_pkgid {
                    solver.add_dependencies(
                        name,
                        ver,
                        deps.unwrap_or_else(|| vec![(bad_name.clone(), Range::any())]),
                    );
                }

                (solver, complicated)
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig {
        result_cache: prop::test_runner::basic_result_cache,
        .. ProptestConfig::default()
    })]

    #[test]
    /// This test is mostly for profiling
    fn prop_passes_string(
        (dependency_provider, cases) in registry_strategy(string_names(), "bad".to_owned())
    )  {
        for (name, ver) in cases {
            let _ = resolve(&dependency_provider, name, ver);
        }
    }

    #[test]
    /// This test is mostly for profiling
    fn prop_passes_int(
        (dependency_provider, cases) in registry_strategy(0u16..665, 666)
    )  {
        for (name, ver) in cases {
            let _ = resolve(&dependency_provider, name, ver);
        }
    }

    #[test]
    /// This tests wheter the allgorithm is still deterministic 
    fn prop_same_on_repeated_runs(
        (dependency_provider, cases) in registry_strategy(0u16..665, 666)
    )  {
        for (name, ver) in cases {
            let one = resolve(&dependency_provider, name, ver);
            for _ in 0..3 {
                match (&one, &resolve(&dependency_provider, name, ver)) {
                    (Ok(l), Ok(r)) => assert_eq!(l, r),
                    (Err(PubGrubError::NoSolution(derivation_l)), Err(PubGrubError::NoSolution(derivation_r))) => {
                        assert_eq!(
                            format!("{}", DefaultStringReporter::report(&derivation_l)),
                            format!("{}", DefaultStringReporter::report(&derivation_r)),
                        )},
                    _ => panic!("not the same result")
                }
            }
        }
    }
}
