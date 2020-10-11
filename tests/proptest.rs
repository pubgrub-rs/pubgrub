// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::{collections::BTreeSet as Set, error::Error};

use pubgrub::type_aliases::Map;
use pubgrub::{
    error::PubGrubError, package::Package, report::DefaultStringReporter, report::Reporter,
    solver::DependencyProvider, version::NumberVersion, version::Version,
};
use pubgrub::{range::Range, solver::resolve, solver::OfflineDependencyProvider};

use proptest::collection::{btree_map, vec};
use proptest::prelude::*;
use proptest::sample::Index;
use proptest::string::string_regex;

/// The same as DP but it prefers the opposite versions.
/// If DP returns versions from newest to oldest, this returns them from oldest to newest.
#[derive(Clone)]
struct MinimalDependencyProvider<DP>(DP);

impl<P: Package, V: Version, DP: DependencyProvider<P, V>> DependencyProvider<P, V>
    for MinimalDependencyProvider<DP>
{
    // Lists only from remote for simplicity
    fn list_available_versions(&self, p: &P) -> Result<Vec<V>, Box<dyn Error>> {
        self.0.list_available_versions(p).map(|mut v| {
            v.reverse();
            v
        })
    }

    // Caches dependencies if they were already queried
    fn get_dependencies(&self, p: &P, v: &V) -> Result<Option<Map<P, Range<V>>>, Box<dyn Error>> {
        self.0.get_dependencies(p, v)
    }
}

/// The same as DP but it has a time out.
#[derive(Clone)]
struct TimeoutDependencyProvider<DP> {
    dp: DP,
    start_time: std::time::Instant,
    call_count: std::cell::Cell<u64>,
    max_calls: u64,
}

impl<DP> TimeoutDependencyProvider<DP> {
    fn new(dp: DP, max_calls: u64) -> Self {
        Self {
            dp,
            start_time: std::time::Instant::now(),
            call_count: std::cell::Cell::new(0),
            max_calls,
        }
    }
}

impl<P: Package, V: Version, DP: DependencyProvider<P, V>> DependencyProvider<P, V>
    for TimeoutDependencyProvider<DP>
{
    // Lists only from remote for simplicity
    fn list_available_versions(&self, p: &P) -> Result<Vec<V>, Box<dyn Error>> {
        self.dp.list_available_versions(p)
    }

    // Caches dependencies if they were already queried
    fn get_dependencies(&self, p: &P, v: &V) -> Result<Option<Map<P, Range<V>>>, Box<dyn Error>> {
        self.dp.get_dependencies(p, v)
    }

    fn callback(&self) -> Result<(), Box<dyn Error>> {
        assert!(self.start_time.elapsed().as_secs() < 60);
        let calls = self.call_count.get();
        assert!(calls < self.max_calls);
        self.call_count.set(calls + 1);
        Ok(())
    }
}

#[test]
#[should_panic]
fn callback_can_panic() {
    let mut dependency_provider = OfflineDependencyProvider::<_, NumberVersion>::new();
    dependency_provider.add_dependencies(0, 0, vec![(666, Range::any())]);

    // Run the algorithm.
    let _ = resolve(
        &TimeoutDependencyProvider::new(dependency_provider, 1),
        0,
        0,
    );
}

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
    let shrinkage = 40;
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

/// This test is to test the generator to ensure
/// that it makes registries with large dependency trees
#[test]
fn meta_test_deep_trees_from_strategy() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;

    let mut dis = [0; 21];

    let strategy = registry_strategy(0u16..665, 666);
    let mut test_runner = TestRunner::deterministic();
    for _ in 0..128 {
        let (dependency_provider, cases) = strategy
            .new_tree(&mut TestRunner::new_with_rng(
                Default::default(),
                test_runner.new_rng(),
            ))
            .unwrap()
            .current();

        for (name, ver) in cases {
            let res = resolve(&dependency_provider, name, ver);
            dis[res
                .as_ref()
                .map(|x| std::cmp::min(x.len(), dis.len()) - 1)
                .unwrap_or(0)] += 1;
            if dis.iter().all(|&x| x > 0) {
                return;
            }
        }
    }

    panic!(
        "In {} tries we did not see a wide enough distribution of dependency trees! dis: {:?}",
        dis.iter().sum::<i32>(),
        dis
    );
}

proptest! {
    #![proptest_config(ProptestConfig {
    max_shrink_iters:
        if std::env::var("CI").is_ok() {
            // This attempts to make sure that CI will fail fast,
            0
        } else {
            // but that local builds will give a small clear test case.
            2048 // u32::MAX
        },
        result_cache: prop::test_runner::basic_result_cache,
        .. ProptestConfig::default()
    })]

    #[test]
    /// This test is mostly for profiling
    fn prop_passes_string(
        (dependency_provider, cases) in registry_strategy(string_names(), "bad".to_owned())
    )  {
        for (name, ver) in cases {
            let _ = resolve(&TimeoutDependencyProvider::new(dependency_provider.clone(), 50_000), name, ver);
        }
    }

    #[test]
    /// This test is mostly for profiling
    fn prop_passes_int(
        (dependency_provider, cases) in registry_strategy(0u16..665, 666)
    )  {
        for (name, ver) in cases {
            let _ = resolve(&TimeoutDependencyProvider::new(dependency_provider.clone(), 50_000), name, ver);
        }
    }

    #[test]
    /// This tests wheter the allgorithm is still deterministic
    fn prop_same_on_repeated_runs(
        (dependency_provider, cases) in registry_strategy(0u16..665, 666)
    )  {
        for (name, ver) in cases {
            let one = resolve(&TimeoutDependencyProvider::new(dependency_provider.clone(), 50_000), name, ver);
            for _ in 0..3 {
                match (&one, &resolve(&TimeoutDependencyProvider::new(dependency_provider.clone(), 50_000), name, ver)) {
                    (Ok(l), Ok(r)) => assert_eq!(l, r),
                    (Err(PubGrubError::NoSolution(derivation_l)), Err(PubGrubError::NoSolution(derivation_r))) => {
                        prop_assert_eq!(
                            format!("{}", DefaultStringReporter::report(&derivation_l)),
                            format!("{}", DefaultStringReporter::report(&derivation_r))
                        )},
                    _ => panic!("not the same result")
                }
            }
        }
    }

    #[test]
    /// MinimalDependencyProvider change what order the candidates
    /// are tried but not the existence of a solution
    fn prop_minimum_version_errors_the_same(
        (dependency_provider, cases) in registry_strategy(0u16..665, 666)
    )  {
        let minimal_provider = MinimalDependencyProvider(dependency_provider.clone());
        for (name, ver) in cases {
            let l = resolve(&TimeoutDependencyProvider::new(dependency_provider.clone(), 50_000), name, ver);
            let r = resolve(&TimeoutDependencyProvider::new(minimal_provider.clone(), 50_000), name, ver);
            match (&l, &r) {
                (Ok(_), Ok(_)) => (),
                (Err(_), Err(_)) => (),
                _ => panic!("not the same result")
            }
        }
    }

    #[test]
    fn prop_removing_a_dep_cant_break(
        (dependency_provider, cases) in registry_strategy(0u16..665, 666),
        indexes_to_remove in prop::collection::vec((any::<prop::sample::Index>(), any::<prop::sample::Index>(), any::<prop::sample::Index>()), 1..10)
    )  {
        let packages: Vec<_> = dependency_provider.packages().collect();
        let mut removed_provider = dependency_provider.clone();
        for (package_idx, version_idx, dep_idx) in indexes_to_remove {
            let package = package_idx.get(&packages);
            let versions = dependency_provider.list_available_versions(package).unwrap();
            let version = version_idx.get(&versions);
            let dependencys: Vec<_> = dependency_provider.get_dependencies(package, version).unwrap().unwrap().into_iter().collect();
            if !dependencys.is_empty() {
                let dependency = dep_idx.get(&dependencys).0.clone();
                removed_provider.add_dependencies(
                    **package,
                    *version,
                    dependencys.into_iter().filter(|x| x.0 != dependency)
                )
            }
        }
        for (name, ver) in cases {
            if resolve(&dependency_provider, name, ver).is_ok() {
                prop_assert!(
                    resolve(&TimeoutDependencyProvider::new(dependency_provider.clone(), 50_000), name, ver).is_ok(),
                    "full index worked for `{} = \"={}\"` but removing some deps broke it!",
                    name,
                    ver,
                )
            }
        }
    }

    #[test]
    fn prop_limited_independence_of_irrelevant_alternatives(
        (dependency_provider, cases) in registry_strategy(0u16..665, 666),
        indexes_to_remove in prop::collection::vec(any::<prop::sample::Index>(), 1..10)
    )  {
        let all_versions: Vec<(u16, NumberVersion)> = dependency_provider
        .packages()
        .flat_map(|&p| {
            dependency_provider
                .list_available_versions(&p)
                .unwrap()
                .into_iter()
                .map(move |v| (p, v))
        })
        .collect();
        let to_remove: Set<(_, _)> = indexes_to_remove.iter().map(|x| x.get(&all_versions)).cloned().collect();
        for (name, ver) in cases {
            match resolve(&TimeoutDependencyProvider::new(dependency_provider.clone(), 50_000), name, ver) {
                Ok(used) => {
                    // If resolution was successful, then unpublishing a version of a crate
                    // that was not selected should not change that.
                    let mut solver = OfflineDependencyProvider::<_, NumberVersion>::new();
                    for &(n, v) in &all_versions {
                        if used.get(&n) == Some(&v) // it was ues
                           || to_remove.get(&(n, v)).is_none() // or it is not one to be removed
                        {
                            solver.add_dependencies(n, v, dependency_provider.get_dependencies(&n, &v).unwrap().unwrap())
                        }
                    }
                    prop_assert!(
                        resolve(&TimeoutDependencyProvider::new(solver.clone(), 50_000), name, ver).is_ok(),
                        "unpublishing {:?} stopped `{} = \"={}\"` from working",
                        to_remove,
                        name,
                        ver
                    )
                }
                Err(_) => {
                    // If resolution was unsuccessful, then it should stay unsuccessful
                    // even if any version of a crate is unpublished.
                    let mut solver = OfflineDependencyProvider::<_, NumberVersion>::new();
                    for &(n, v) in &all_versions {
                        if to_remove.get(&(n, v)).is_none() // it is not one to be removed
                        {
                            solver.add_dependencies(n, v, dependency_provider.get_dependencies(&n, &v).unwrap().unwrap())
                        }
                    }
                    prop_assert!(
                        resolve(&TimeoutDependencyProvider::new(solver.clone(), 50_000), name, ver).is_err(),
                        "full index did not work for `{} = \"={}\"` but unpublishing {:?} fixed it!",
                        name,
                        ver,
                        to_remove,
                    )
                }
            }
        }
    }
}
