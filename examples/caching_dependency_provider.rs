use pubgrub::package::Package;
use pubgrub::range::Range;
use pubgrub::solver::{resolve, DependencyProvider, OfflineDependencyProvider};
use pubgrub::version::{NumberVersion, Version};
use pubgrub::Map;
use std::cell::RefCell;
use std::error::Error;
use std::hash::Hash;

// An example implementing caching dependency provider that will
// store queried dependencies in memory and check them before querying more from remote.
struct CachingDependencyProvider<P: Package, V: Version + Hash, DP: DependencyProvider<P, V>> {
    remote_dependencies: DP,
    cached_dependencies: RefCell<OfflineDependencyProvider<P, V>>,
}

impl<P: Package, V: Version + Hash, DP: DependencyProvider<P, V>>
    CachingDependencyProvider<P, V, DP>
{
    pub fn new(remote_dependencies_provider: DP) -> Self {
        CachingDependencyProvider {
            remote_dependencies: remote_dependencies_provider,
            cached_dependencies: RefCell::new(OfflineDependencyProvider::new()),
        }
    }
}

impl<P: Package, V: Version + Hash, DP: DependencyProvider<P, V>> DependencyProvider<P, V>
    for CachingDependencyProvider<P, V, DP>
{
    // Lists only from remote for simplicity
    fn list_available_versions(&self, package: &P) -> Result<Vec<V>, Box<dyn Error>> {
        self.remote_dependencies.list_available_versions(package)
    }

    // Caches dependencies if they were already queried
    fn get_dependencies(
        &self,
        package: &P,
        version: &V,
    ) -> Result<Option<Map<P, Range<V>>>, Box<dyn Error>> {
        let mut cache = self.cached_dependencies.borrow_mut();
        match cache.get_dependencies(package, version) {
            Ok(None) => {
                let dependencies = self.remote_dependencies.get_dependencies(package, version);
                match dependencies {
                    Ok(dependencies) => {
                        cache.add_dependencies(
                            package.clone(),
                            version.clone(),
                            dependencies.clone().unwrap_or_default(),
                        );
                        Ok(dependencies)
                    }
                    error @ Err(_) => error,
                }
            }
            dependencies @ Ok(_) => dependencies,
            error @ Err(_) => error,
        }
    }
}

fn main() {
    // Simulating remote provider locally.
    let mut remote_dependencies_provider = OfflineDependencyProvider::<&str, NumberVersion>::new();

    // Add dependencies as needed. Here only root package is added.
    remote_dependencies_provider.add_dependencies("root", 1, Vec::new());

    let caching_dependencies_provider =
        CachingDependencyProvider::new(remote_dependencies_provider);

    let solution = resolve(&caching_dependencies_provider, "root", 1);
    println!("Solution: {:?}", solution);
}
