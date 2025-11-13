// SPDX-License-Identifier: MPL-2.0

use pubgrub::{
    Dependencies, DependencyProvider, OfflineDependencyProvider, Package,
    PackageResolutionStatistics, PubGrubError, Ranges, VersionSet, resolve,
};
use std::convert::Infallible;

type NumVS = Ranges<u32>;

#[test]
fn same_result_on_repeated_runs() {
    let mut dependency_provider = OfflineDependencyProvider::<_, NumVS>::new();

    dependency_provider.add_dependencies("c", 0u32, []);
    dependency_provider.add_dependencies("c", 2u32, []);
    dependency_provider.add_dependencies("b", 0u32, []);
    dependency_provider.add_dependencies("b", 1u32, [("c", Ranges::between(0u32, 1u32))]);

    dependency_provider.add_dependencies("a", 0u32, [("b", Ranges::full()), ("c", Ranges::full())]);

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
    dependency_provider.add_dependencies("a", 0u32, [("b", Ranges::empty())]);
    assert!(matches!(
        resolve(&dependency_provider, "a", 0u32),
        Err(PubGrubError::NoSolution { .. })
    ));

    dependency_provider.add_dependencies("c", 0u32, [("a", Ranges::full())]);
    assert!(matches!(
        resolve(&dependency_provider, "c", 0u32),
        Err(PubGrubError::NoSolution { .. })
    ));
}

#[test]
fn depend_on_self() {
    let mut dependency_provider = OfflineDependencyProvider::<_, NumVS>::new();
    dependency_provider.add_dependencies("a", 0u32, [("a", Ranges::full())]);
    assert!(resolve(&dependency_provider, "a", 0u32).is_ok());
    dependency_provider.add_dependencies("a", 66u32, [("a", Ranges::singleton(111u32))]);
    assert!(resolve(&dependency_provider, "a", 66u32).is_err());
}

/// Test the prioritization is stable across platforms.
///
/// https://github.com/pubgrub-rs/pubgrub/issues/373#issuecomment-3384608891
#[test]
fn same_result_across_platforms() {
    struct UnprioritizingDependencyProvider<P: Package, VS: VersionSet> {
        dependency_provider: OfflineDependencyProvider<P, VS>,
    }

    impl<P: Package, VS: VersionSet> UnprioritizingDependencyProvider<P, VS> {
        fn new() -> Self {
            Self {
                dependency_provider: OfflineDependencyProvider::new(),
            }
        }

        pub fn add_dependencies<I: IntoIterator<Item = (P, VS)>>(
            &mut self,
            package: P,
            version: impl Into<VS::V>,
            dependencies: I,
        ) {
            self.dependency_provider
                .add_dependencies(package, version, dependencies);
        }
    }

    impl<P: Package, VS: VersionSet> DependencyProvider for UnprioritizingDependencyProvider<P, VS> {
        type P = P;
        type V = VS::V;
        type VS = VS;
        type M = String;
        type Priority = u32;
        type Err = Infallible;

        fn choose_version(&self, package: &P, range: &VS) -> Result<Option<VS::V>, Infallible> {
            self.dependency_provider.choose_version(package, range)
        }

        fn prioritize(
            &self,
            _package: &Self::P,
            _range: &Self::VS,
            _package_statistics: &PackageResolutionStatistics,
        ) -> Self::Priority {
            0
        }

        fn get_dependencies(
            &self,
            package: &P,
            version: &VS::V,
        ) -> Result<Dependencies<P, VS, Self::M>, Infallible> {
            self.dependency_provider.get_dependencies(package, version)
        }
    }

    let mut dependency_provider = UnprioritizingDependencyProvider::<_, NumVS>::new();

    let x = (0..1000)
        .map(|i| (i.to_string(), Ranges::full()))
        .collect::<Vec<_>>();
    dependency_provider.add_dependencies("root".to_string(), 1u32, x);

    for i in 0..1000 {
        let x = (0..1000)
            .filter(|j| *j != i)
            .map(|i| (i.to_string(), Ranges::<u32>::singleton(1u32)))
            .collect::<Vec<_>>();
        dependency_provider.add_dependencies(i.to_string(), 2u32, x);
        dependency_provider.add_dependencies(i.to_string(), 1u32, []);
    }

    let name = "root".to_string();
    let ver: u32 = 1;
    let resolution = resolve(&dependency_provider, name, ver).unwrap();
    let (p, _v) = resolution.into_iter().find(|(_p, v)| *v == 2).unwrap();
    assert_eq!(p, "0".to_string());
}
