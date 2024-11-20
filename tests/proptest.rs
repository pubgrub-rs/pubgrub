// SPDX-License-Identifier: MPL-2.0

#![allow(clippy::type_complexity)]

mod sat_dependency_provider;

use std::convert::Infallible;
use std::fmt::{Debug, Display};
use std::hash::Hash;

use proptest::{
    collection::{btree_map, btree_set, vec},
    prelude::*,
    sample::Index,
    string::string_regex,
};
use pubgrub::{
    helpers::PackageVersionWrapper, resolve, DefaultStringReporter, Dependencies,
    DependencyProvider, DerivationTree, External, Map, NoSolutionError, OfflineDependencyProvider,
    PackageArena, PackageId, PubGrubError, Ranges, Reporter, SelectedDependencies, Set,
    VersionIndex, VersionRanges, VersionSet,
};

use crate::sat_dependency_provider::SatResolve;

/// The same as [OfflineDependencyProvider] but takes versions from the opposite end:
/// if [OfflineDependencyProvider] returns versions from newest to oldest, this returns them from oldest to newest.
#[derive(Clone)]
struct OldestVersionsDependencyProvider<P: Debug + Display + Clone + Eq + Hash, R: VersionRanges>(
    OfflineDependencyProvider<P, R>,
);

impl<P: Debug + Display + Clone + Eq + Hash, R: VersionRanges> DependencyProvider
    for OldestVersionsDependencyProvider<P, R>
{
    fn get_dependencies(
        &mut self,
        pid: PackageId,
        v: VersionIndex,
        package_store: &mut PackageArena<Self::P>,
    ) -> Result<Dependencies<Self::M>, Infallible> {
        self.0.get_dependencies(pid, v, package_store)
    }

    fn choose_version(
        &mut self,
        _: PackageId,
        set: VersionSet,
        _: &PackageArena<Self::P>,
    ) -> Result<Option<VersionIndex>, Infallible> {
        Ok(set.first())
    }

    type Priority = <OfflineDependencyProvider<P, R> as DependencyProvider>::Priority;

    fn prioritize(
        &mut self,
        package_id: PackageId,
        set: VersionSet,
        package_store: &PackageArena<Self::P>,
    ) -> Self::Priority {
        self.0.prioritize(package_id, set, package_store)
    }

    type Err = Infallible;

    type P = PackageVersionWrapper<P>;
    type M = &'static str;

    fn package_version_display<'a>(
        &'a self,
        package: &'a Self::P,
        version_index: VersionIndex,
    ) -> impl Display + 'a {
        self.0.package_version_display(package, version_index)
    }

    fn package_version_set_display<'a>(
        &'a self,
        package: &'a Self::P,
        version_set: VersionSet,
    ) -> impl Display + 'a {
        self.0.package_version_set_display(package, version_set)
    }
}

/// The same as DP but it has a timeout.
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

impl<DP: DependencyProvider> DependencyProvider for TimeoutDependencyProvider<DP> {
    fn get_dependencies(
        &mut self,
        pid: PackageId,
        version_index: VersionIndex,
        package_store: &mut PackageArena<Self::P>,
    ) -> Result<Dependencies<DP::M>, DP::Err> {
        self.dp.get_dependencies(pid, version_index, package_store)
    }

    fn should_cancel(&mut self) -> Result<(), DP::Err> {
        assert!(self.start_time.elapsed().as_secs() < 60);
        let calls = self.call_count.get();
        assert!(calls < self.max_calls);
        self.call_count.set(calls + 1);
        Ok(())
    }

    fn choose_version(
        &mut self,
        package_id: PackageId,
        set: VersionSet,
        package_store: &PackageArena<Self::P>,
    ) -> Result<Option<VersionIndex>, DP::Err> {
        self.dp.choose_version(package_id, set, package_store)
    }

    type Priority = DP::Priority;

    fn prioritize(
        &mut self,
        package_id: PackageId,
        set: VersionSet,
        package_store: &PackageArena<Self::P>,
    ) -> Self::Priority {
        self.dp.prioritize(package_id, set, package_store)
    }

    type Err = DP::Err;

    type P = DP::P;
    type M = DP::M;

    fn package_version_display<'a>(
        &'a self,
        package: &'a Self::P,
        version_index: VersionIndex,
    ) -> impl Display + 'a {
        self.dp.package_version_display(package, version_index)
    }

    fn package_version_set_display<'a>(
        &'a self,
        package: &'a Self::P,
        version_set: VersionSet,
    ) -> impl Display + 'a {
        self.dp.package_version_set_display(package, version_set)
    }
}

fn timeout_resolve<DP: DependencyProvider>(
    dependency_provider: DP,
    pkg: DP::P,
    version_index: VersionIndex,
) -> Result<
    SelectedDependencies<TimeoutDependencyProvider<DP>>,
    PubGrubError<TimeoutDependencyProvider<DP>>,
> {
    let mut dp = TimeoutDependencyProvider::new(dependency_provider, 50_000);
    resolve(&mut dp, pkg, version_index)
}

type NumVS = Ranges<u32>;

#[test]
#[should_panic]
fn should_cancel_can_panic() {
    let mut dependency_provider = OfflineDependencyProvider::<_, NumVS>::new();
    dependency_provider.add_dependencies(0, 0u32, [(666, Ranges::full())]);

    // Run the algorithm.
    let (p, v) = dependency_provider.resolve_parameters(0, 0u32).unwrap();
    let mut dp = TimeoutDependencyProvider::new(dependency_provider, 1);
    let _ = resolve(&mut dp, p, v);
}

fn string_names() -> impl Strategy<Value = String> {
    string_regex("[A-Za-z][A-Za-z0-9_-]{0,5}")
        .unwrap()
        .prop_filter("reserved names", |n| {
            // root is the name of the thing being compiled
            // so it would be confusing to have it in the index
            // bad is a name reserved for a dep that won't work
            n != "root" && n != "bad"
        })
}

/// This generates a random registry index.
/// Unlike vec((Name, Ver, vec((Name, VerRq), ..), ..)
/// This strategy has a high probability of having valid dependencies
pub fn registry_strategy<N: Debug + Display + Clone + Hash + Ord>(
    name: impl Strategy<Value = N>,
) -> impl Strategy<Value = (OfflineDependencyProvider<N, NumVS>, Vec<(N, u32)>)> {
    let max_crates = 40;
    let max_versions = 15;
    let shrinkage = 40;
    let complicated_len = 10usize;

    let a_version = ..(max_versions as u32);

    let list_of_versions = btree_set(a_version, 1..=max_versions)
        .prop_map(move |ver| ver.into_iter().collect::<Vec<_>>());

    let list_of_crates_with_versions = btree_map(name, list_of_versions, 1..=max_crates);

    // each version of each crate can depend on each crate smaller then it.
    // In theory shrinkage should be 2, but in practice we get better trees with a larger value.
    let max_deps = max_versions * (max_crates * (max_crates - 1)) / shrinkage;

    let raw_version_range = (any::<Index>(), any::<Index>());
    let raw_dependency = (any::<Index>(), any::<Index>(), raw_version_range);

    fn order_index(a: Index, b: Index, size: usize) -> (usize, usize) {
        let (a, b) = (a.index(size), b.index(size));
        (a.min(b), a.max(b))
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
                let mut list_of_pkgid: Vec<((N, u32), Vec<(N, NumVS)>)> = crate_vers_by_name
                    .iter()
                    .flat_map(|(name, vers)| vers.iter().map(move |&x| ((name.clone(), x), vec![])))
                    .collect();
                let len_all_pkgid = list_of_pkgid.len();
                for (a, b, (c, d)) in raw_dependencies {
                    let (a, b) = order_index(a, b, len_all_pkgid);
                    let (a, b) = if reverse_alphabetical { (b, a) } else { (a, b) };
                    let ((dep_name, _), _) = list_of_pkgid[a].to_owned();
                    if list_of_pkgid[b].0 .0 == dep_name {
                        continue;
                    }
                    let s = &crate_vers_by_name[&dep_name];
                    let s_last_index = s.len() - 1;
                    let (c, d) = order_index(c, d, s.len() + 1);

                    list_of_pkgid[b].1.push((
                        dep_name,
                        if c > s_last_index {
                            Ranges::empty()
                        } else if c == 0 && d >= s_last_index {
                            Ranges::full()
                        } else if c == 0 {
                            Ranges::strictly_lower_than(s[d] + 1)
                        } else if d >= s_last_index {
                            Ranges::higher_than(s[c])
                        } else if c == d {
                            Ranges::singleton(s[c])
                        } else {
                            Ranges::between(s[c], s[d] + 1)
                        },
                    ));
                }

                let mut dependency_provider = OfflineDependencyProvider::<N, NumVS>::new();

                let complicated_len = complicated_len.min(list_of_pkgid.len());
                let complicated: Vec<_> = if reverse_alphabetical {
                    &list_of_pkgid[..complicated_len]
                } else {
                    &list_of_pkgid[(list_of_pkgid.len() - complicated_len)..]
                }
                .iter()
                .map(|(x, _)| (x.0.clone(), x.1))
                .collect();

                for ((name, ver), deps) in list_of_pkgid {
                    dependency_provider.add_dependencies(name, ver, deps);
                }

                (dependency_provider, complicated)
            },
        )
}

/// Ensures that generator makes registries with large dependency trees.
#[test]
fn meta_test_deep_trees_from_strategy() {
    use proptest::{strategy::ValueTree, test_runner::TestRunner};

    let mut dis = [0; 21];

    let strategy = registry_strategy(0u16..665);
    let mut test_runner = TestRunner::deterministic();
    for _ in 0..128 {
        let (mut dependency_provider, cases) = strategy
            .new_tree(&mut TestRunner::new_with_rng(
                Default::default(),
                test_runner.new_rng(),
            ))
            .unwrap()
            .current();

        for (name, ver) in cases {
            let res = dependency_provider.resolve(name, ver);
            dis[res
                .as_ref()
                .map(|x| x.len().min(dis.len()) - 1)
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

/// Removes versions from the dependency provider where the retain function returns false.
/// Solutions are constructed as a set of versions.
/// If there are fewer versions available, there are fewer valid solutions available.
/// If there was no solution to a resolution in the original dependency provider,
/// then there must still be no solution with some options removed.
/// If there was a solution to a resolution in the original dependency provider,
/// there may not be a solution after versions are removes iif removed versions were critical for all valid solutions.
fn retain_versions<N: Debug + Display + Clone + Hash + Ord, R: VersionRanges>(
    dependency_provider: &OfflineDependencyProvider<N, R>,
    retain: impl Fn(&N, &R::V) -> bool,
) -> OfflineDependencyProvider<N, R> {
    let mut smaller_dependency_provider = OfflineDependencyProvider::new();
    for n in dependency_provider.packages() {
        for v in dependency_provider.versions(n).unwrap() {
            if !retain(n, v) {
                continue;
            }
            let deps = dependency_provider.dependencies(n, v).unwrap();
            smaller_dependency_provider.add_dependencies(
                n.clone(),
                v.clone(),
                deps.iter().map(|(p, r)| {
                    let r = R::from_ordered_versions(
                        dependency_provider
                            .versions(p)
                            .unwrap()
                            .map(|v| (v.clone(), r.contains(v))),
                    );
                    (p.clone(), r)
                }),
            )
        }
    }
    smaller_dependency_provider
}

/// Removes dependencies from the dependency provider where the retain function returns false.
/// Solutions are constrained by having to fulfill all the dependencies.
/// If there are fewer dependencies required, there are more valid solutions.
/// If there was a solution to a resolution in the original dependency provider,
/// then there must still be a solution after dependencies are removed.
/// If there was no solution to a resolution in the original dependency provider,
/// there may now be a solution after dependencies are removed.
fn retain_dependencies<N: Debug + Display + Clone + Hash + Ord, R: VersionRanges>(
    dependency_provider: &OfflineDependencyProvider<N, R>,
    retain: impl Fn(&N, &R::V, &N) -> bool,
) -> OfflineDependencyProvider<N, R> {
    let mut smaller_dependency_provider = OfflineDependencyProvider::new();
    for n in dependency_provider.packages() {
        for v in dependency_provider.versions(n).unwrap() {
            let deps = dependency_provider.dependencies(n, v).unwrap();
            smaller_dependency_provider.add_dependencies(
                n.clone(),
                v.clone(),
                deps.iter().filter_map(|(dep, r)| {
                    if !retain(n, v, dep) {
                        None
                    } else {
                        let r = R::from_ordered_versions(
                            dependency_provider
                                .versions(dep)
                                .unwrap()
                                .map(|v| (v.clone(), r.contains(v))),
                        );
                        Some((dep.clone(), r))
                    }
                }),
            );
        }
    }
    smaller_dependency_provider
}

fn errors_the_same_with_only_report_dependencies<N: Debug + Display + Clone + Hash + Ord>(
    dependency_provider: OfflineDependencyProvider<N, NumVS>,
    name: N,
    ver: u32,
) {
    let (p, v) = dependency_provider
        .resolve_parameters(name.clone(), ver)
        .unwrap();

    let Err(PubGrubError::NoSolution(error)) = timeout_resolve(dependency_provider.clone(), p, v)
    else {
        return;
    };

    fn recursive<N: Debug + Display + Clone + Hash + Ord, M: Eq + Clone + Debug + Display>(
        to_retain: &mut Map<N, Map<u32, Set<N>>>,
        tree: &DerivationTree<M>,
        package_store: &PackageArena<PackageVersionWrapper<N>>,
        dependency_provider: &OfflineDependencyProvider<N, NumVS>,
    ) {
        match tree {
            &DerivationTree::External(External::FromDependencyOf(n1, vs1, n2, _)) => {
                let pkg1 = package_store.pkg(n1).unwrap();
                let pkg2 = package_store.pkg(n2).unwrap();

                if let Some(n1) = pkg1.inner_pkg() {
                    let n2 = match pkg2 {
                        PackageVersionWrapper::Pkg(p) => p.pkg(),
                        PackageVersionWrapper::VirtualPkg(vp) => vp.pkg(),
                        PackageVersionWrapper::VirtualDep(vd) => vd.pkg(),
                    };

                    for v in vs1.iter() {
                        let v1 = *dependency_provider
                            .versions(n1)
                            .unwrap()
                            .nth(pkg1.inner(v).unwrap().1 as usize)
                            .unwrap();

                        to_retain
                            .entry(n1.clone())
                            .or_default()
                            .entry(v1)
                            .or_default()
                            .insert(n2.clone());
                    }
                }
            }
            DerivationTree::Derived(d) => {
                recursive(to_retain, &*d.cause1, package_store, dependency_provider);
                recursive(to_retain, &*d.cause2, package_store, dependency_provider);
            }
            _ => {}
        }
    }

    let mut to_retain = Map::default();
    recursive(
        &mut to_retain,
        &error.derivation_tree,
        &error.package_store,
        &dependency_provider,
    );

    let removed_provider = retain_dependencies(&dependency_provider, |p, v, d| {
        (|| to_retain.get(p)?.get(v)?.get(d))().is_some()
    });

    let (p, v) = removed_provider.resolve_parameters(name, ver).unwrap();

    assert!(
        timeout_resolve(removed_provider, p, v).is_err(),
        "The full index errored filtering to only dependencies in the derivation tree succeeded"
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
            2048
        },
        result_cache: prop::test_runner::basic_result_cache,
        .. ProptestConfig::default()
    })]

    #[test]
    /// This test is mostly for profiling.
    fn prop_passes_string(
        (dependency_provider, cases) in registry_strategy(string_names())
    )  {
        for (name, ver) in cases {
            let (p, v) = dependency_provider.resolve_parameters(name, ver).unwrap();
            _ = timeout_resolve(dependency_provider.clone(), p, v);
        }
    }

    #[test]
    /// This test is mostly for profiling.
    fn prop_passes_int(
        (dependency_provider, cases) in registry_strategy(0u16..665)
    )  {
        for (name, ver) in cases {
            let (p, v) = dependency_provider.resolve_parameters(name, ver).unwrap();
            _ = timeout_resolve(dependency_provider.clone(), p, v);
        }
    }

    #[test]
    fn prop_sat_errors_the_same(
        (dependency_provider, cases) in registry_strategy(0u16..665)
    )  {
        let mut sat = SatResolve::new(&dependency_provider);
        for (name, ver) in cases {
            let (p, v) = dependency_provider.resolve_parameters(name, ver).unwrap();
            let res = timeout_resolve(dependency_provider.clone(), p, v);
            sat.check_resolve(&res, &name, &ver);
        }
    }

    #[test]
    fn prop_errors_the_same_with_only_report_dependencies(
        (dependency_provider, cases) in registry_strategy(0u16..665)
    )  {
        for (name, ver) in cases {
            errors_the_same_with_only_report_dependencies(dependency_provider.clone(), name, ver);
        }
    }

    #[test]
    /// This tests whether the algorithm is still deterministic.
    fn prop_same_on_repeated_runs(
        (dependency_provider, cases) in registry_strategy(0u16..665)
    )  {
        for (name, ver) in cases {
            let (p, v) = dependency_provider.resolve_parameters(name, ver).unwrap();
            let one = timeout_resolve(dependency_provider.clone(), p.clone(), v);
            for _ in 0..3 {
                match (&one, &timeout_resolve(dependency_provider.clone(), p.clone(), v)) {
                    (Ok(l), Ok(r)) => assert_eq!(l, r),
                    (Err(PubGrubError::NoSolution(error_l)), Err(PubGrubError::NoSolution(error_r))) => {
                        let (error_l, error_r) = (error_l.clone(), error_r.clone());
                        let error_l = NoSolutionError { package_store: error_l.package_store, derivation_tree: error_l.derivation_tree };
                        let error_r = NoSolutionError { package_store: error_r.package_store, derivation_tree: error_r.derivation_tree };
                        prop_assert_eq!(
                            DefaultStringReporter::report(&error_l, &dependency_provider),
                            DefaultStringReporter::report(&error_r, &dependency_provider)
                        );
                    }
                    _ => panic!("not the same result")
                }
            }
        }
    }

    #[test]
    /// [ReverseDependencyProvider] changes what order the candidates
    /// are tried but not the existence of a solution.
    fn prop_reversed_version_errors_the_same(
        (dependency_provider, cases) in registry_strategy(0u16..665)
    )  {
        let reverse_provider = OldestVersionsDependencyProvider(dependency_provider.clone());
        for (name, ver) in cases {
            let (p, v) = dependency_provider.resolve_parameters(name, ver).unwrap();
            let l = timeout_resolve(dependency_provider.clone(), p.clone(), v);
            let r = timeout_resolve(reverse_provider.clone(), p, v);
            match (&l, &r) {
                (Ok(_), Ok(_)) => (),
                (Err(_), Err(_)) => (),
                _ => panic!("not the same result")
            }
        }
    }

    #[test]
    fn prop_removing_a_dep_cant_break(
        (dependency_provider, cases) in registry_strategy(0u16..665),
        indexes_to_remove in vec((any::<Index>(), any::<Index>(), any::<Index>()), 1..10)
    ) {
        let packages: Vec<_> = dependency_provider.packages().copied().collect();
        let mut to_remove = Set::default();
        for (package_idx, version_idx, dep_idx) in indexes_to_remove {
            let pkg = *package_idx.get(&packages);
            let versions: Vec<_> = dependency_provider
                .versions(&pkg)
                .unwrap()
                .copied()
                .collect();
            let version = *version_idx.get(&versions);
            let deps = dependency_provider.dependencies(&pkg, &version).unwrap().iter().collect::<Vec<_>>();
            if !deps.is_empty() {
                to_remove.insert((pkg, version, *dep_idx.get(&deps).0));
            }
        }
        let removed_provider = retain_dependencies(
            &dependency_provider,
            |&p, &v, &d| !to_remove.contains(&(p, v, d))
        );
        for (name, ver) in cases {
            let (p, v) = dependency_provider.resolve_parameters(name, ver).unwrap();
            if timeout_resolve(dependency_provider.clone(), p.clone(), v).is_ok() {
                prop_assert!(
                    timeout_resolve(removed_provider.clone(), p, v).is_ok(),
                    "full index worked for `{name} = \"={ver}\"` but removing some deps broke it!"
                )
            }
        }
    }

    #[test]
    fn prop_limited_independence_of_irrelevant_alternatives(
        (dependency_provider, cases) in registry_strategy(0u16..665),
        indexes_to_remove in vec(any::<Index>(), 1..10)
    )  {
        let all_versions: Vec<(u16, u32)> = dependency_provider
            .packages()
            .flat_map(|&p| dependency_provider.versions(&p).unwrap().map(move |&v| (p, v)))
            .collect();
        let to_remove: Set<(_, _)> = indexes_to_remove.iter().map(|x| x.get(&all_versions)).cloned().collect();
        for (name, ver) in cases {
            let (p, v) = dependency_provider.resolve_parameters(name, ver).unwrap();
            match timeout_resolve(dependency_provider.clone(), p.clone(), v) {
                Ok(used) => {
                    let used_packages = used
                        .iter()
                        .filter_map(|(wrapper, &version_index)| wrapper.inner(version_index))
                        .map(|(p, v)| (p, *dependency_provider.versions(p).unwrap().nth(v as usize).unwrap()))
                        .collect::<Map<_,_>>();
                    // If resolution was successful, then unpublishing a version of a crate
                    // that was not selected should not change that.
                    let smaller_dependency_provider = retain_versions(&dependency_provider, |&n, &v| {
                        used_packages.get(&n) == Some(&v) // it was used
                            || !to_remove.contains(&(n, v)) // or it is not one to be removed
                    });
                    let (p, v) = smaller_dependency_provider.resolve_parameters(name, ver).unwrap();
                    prop_assert!(
                        timeout_resolve(smaller_dependency_provider.clone(), p, v).is_ok(),
                        "unpublishing {to_remove:?} stopped `{name} = \"={ver}\"` from working"
                    )
                }
                Err(_) => {
                    // If resolution was unsuccessful, then it should stay unsuccessful
                    // even if any version of a crate is unpublished.
                    let smaller_dependency_provider = retain_versions(&dependency_provider, |&n, &v| {
                        to_remove.contains(&(n, v)) // it is one to be removed
                    });
                    if let Some((p, v)) = smaller_dependency_provider.resolve_parameters(name, ver) {
                        prop_assert!(
                            timeout_resolve(smaller_dependency_provider.clone(), p, v).is_err(),
                            "full index did not work for `{name} = \"={ver}\"` but unpublishing {to_remove:?} fixed it!"
                        )
                    }
                }
            }
        }
    }
}

#[cfg(feature = "serde")]
#[test]
fn large_case() {
    for case in std::fs::read_dir("test-examples").unwrap() {
        let case = case.unwrap().path();
        let name = case.file_name().unwrap().to_string_lossy();
        eprint!("{} ", name);
        let data = std::fs::read_to_string(&case).unwrap();
        let start_time = std::time::Instant::now();
        if name.ends_with("u16_NumberVersion.ron") {
            let mut dependency_provider: OfflineDependencyProvider<u16, NumVS> =
                ron::de::from_str(&data).unwrap();
            let mut sat = SatResolve::new(&dependency_provider);
            let packages = dependency_provider.packages().cloned().collect::<Vec<_>>();
            for name in packages {
                let versions = dependency_provider
                    .versions(&name)
                    .unwrap()
                    .copied()
                    .collect::<Vec<_>>();
                for ver in versions {
                    let (p, v) = dependency_provider.resolve_parameters(name, ver).unwrap();
                    let res = resolve(&mut dependency_provider, p, v);
                    sat.check_resolve(&res, &name, &ver);
                }
            }
        }
        eprintln!(" in {}s", start_time.elapsed().as_secs())
    }
}
