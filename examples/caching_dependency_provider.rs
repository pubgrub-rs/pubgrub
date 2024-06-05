// SPDX-License-Identifier: MPL-2.0

use std::cell::RefCell;

use pubgrub::range::Range;
use pubgrub::solver::{resolve, Dependencies, DependencyProvider, OfflineDependencyProvider};

type NumVS = Range<u32>;

// An example implementing caching dependency provider that will
// store queried dependencies in memory and check them before querying more from remote.
struct CachingDependencyProvider<DP: DependencyProvider> {
    remote_dependencies: DP,
    cached_dependencies: RefCell<OfflineDependencyProvider<DP::P, DP::VS>>,
}

impl<DP: DependencyProvider> CachingDependencyProvider<DP> {
    pub fn new(remote_dependencies_provider: DP) -> Self {
        CachingDependencyProvider {
            remote_dependencies: remote_dependencies_provider,
            cached_dependencies: RefCell::new(OfflineDependencyProvider::new()),
        }
    }
}

impl<DP: DependencyProvider<M = String>> DependencyProvider for CachingDependencyProvider<DP> {
    // Caches dependencies if they were already queried
    fn get_dependencies(
        &self,
        package: &DP::P,
        version: &DP::V,
    ) -> Result<Dependencies<DP::P, DP::VS, DP::M>, DP::Err> {
        let mut cache = self.cached_dependencies.borrow_mut();
        match cache.get_dependencies(package, version) {
            Ok(Dependencies::Unavailable(_)) => {
                let dependencies = self.remote_dependencies.get_dependencies(package, version);
                match dependencies {
                    Ok(Dependencies::Available(dependencies)) => {
                        cache.add_dependencies(
                            package.clone(),
                            version.clone(),
                            dependencies.clone(),
                        );
                        Ok(Dependencies::Available(dependencies))
                    }
                    Ok(Dependencies::Unavailable(reason)) => Ok(Dependencies::Unavailable(reason)),
                    error @ Err(_) => error,
                }
            }
            Ok(dependencies) => Ok(dependencies),
            Err(_) => unreachable!(),
        }
    }

    fn choose_version(&self, package: &DP::P, range: &DP::VS) -> Result<Option<DP::V>, DP::Err> {
        self.remote_dependencies.choose_version(package, range)
    }

    type Priority = DP::Priority;

    fn prioritize(&self, package: &DP::P, range: &DP::VS) -> Self::Priority {
        self.remote_dependencies.prioritize(package, range)
    }

    type Err = DP::Err;

    type P = DP::P;
    type V = DP::V;
    type VS = DP::VS;
    type M = DP::M;
}

fn main() {
    // Simulating remote provider locally.
    let mut remote_dependencies_provider = OfflineDependencyProvider::<&str, NumVS>::new();

    // Add dependencies as needed. Here only root package is added.
    remote_dependencies_provider.add_dependencies("root", 1u32, Vec::new());

    let caching_dependencies_provider =
        CachingDependencyProvider::new(remote_dependencies_provider);

    let solution = resolve(&caching_dependencies_provider, "root", 1u32);
    println!("Solution: {:?}", solution);
}
