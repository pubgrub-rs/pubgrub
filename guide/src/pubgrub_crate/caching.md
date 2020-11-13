# Caching dependencies in a DependencyProvider

Due to potential backtracking, the resolver may ask dependencies of a package multiple times.
A dependency provider can also survive one resolution and be reused for another one,
usually of the same package and thus asking for the same dependencies.
Since dependencies are generally immutable, caching them is a valid strategy
to avoid slow operations that may be needed such as fetching remote data.
But caching data in a dependency provider while computing the result of `get_dependencies`
would require `&mut self` instead of `&self`.
We wanted to encode in the type that the `resolve` function cannot mutate
on its own a dependency provider so that's why we chose `&self`.
But if the dependency provider wants to mutate itself there is
a pattern called [interior mutability](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html)
enabling exactly that.
We will now setup a simple example of how to do that.
If by the end you think the trade-off is not worth it and we should
use `&mut self` in the method signature, please let us know with an issue in pubgrub repository.

Let `DependencyCache` be our cache for dependencies,
with existing methods to fetch them over the network,

```rust
struct DependencyCache<P: Package, V: Version> {
    cache: Map<P, BTreeMap<V, DependencyConstraints<P, V>>>,
}

impl<P: Package, V: Version> DependencyCache<P, V> {
    /// Initialize cached dependencies.
    pub fn new() -> Self { ... }
    /// Fetch dependencies of a given package on the network and store them in the cache.
    pub fn fetch_dependencies(&mut self, package: &P, version: &V) -> Result<(), Box<dyn Error>> { ... }
    /// Extract dependencies of a given package from the cache.
    pub fn get(&self, package: &P, version: &V) -> Dependencies { ... }
}
```

We can implement the `DependencyProvider` trait in the following way,
using `RefCell` for interior mutability in order to cache dependencies.

```rust
pub struct CachingDependencyProvider<P: Package, V: Version> {
    cached_dependencies: RefCell<DependencyCache>,
}

impl<P: Package, V: Version> DependencyProvider<P, V> for CachingDependencyProvider<P, V> {
    fn make_decision<...>(...) -> ... { ... }
    fn get_dependencies(
        &self,
        package: &P,
        version: &V,
    ) -> Result<Dependencies<P, V>, Box<dyn Error>> {
        match self.cached_dependencies.get(package, version) {
            deps @ Dependencies::Kown(_) => Ok(deps),
            Dependencies::Unknown => {
                self.cached_dependencies.borrow_mut().fetch_dependencies(package, version)?;
                self.cached_dependencies.get(package, version)
            }
        }
    }
}
```

A complete caching example based on the `OfflineDependencyProvider`
is available in `examples/caching_dependency_provider.rs`.
