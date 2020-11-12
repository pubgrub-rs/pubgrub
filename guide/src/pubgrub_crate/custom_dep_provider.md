# Writing your own dependency provider

The `OfflineDependencyProvider` is very useful for testing
and playing with the API, but would not be usable in more complex
settings like Cargo for example.
In such cases, a dependency provider may need to retrieve
package information from caches, from the disk or from network requests.
Then, you might want to implement `DependencyProvider` for your own type.
The `DependencyProvider` trait is defined as follows.

```rust
/// Trait that allows the algorithm to retrieve available packages and their dependencies.
/// An implementor needs to be supplied to the [resolve] function.
pub trait DependencyProvider<P: Package, V: Version> {
    /// Chooses which package the [resolve] should decide on next.
    /// If there are available versions for that package,
    /// return `Some(v)` where `v` is one such version,
    /// otherwise return `None`.
    ///
    /// Note: the type `T`, used both in the method arguments and the return type,
    /// ensures that this method returns an item from the `packages` argument.
    fn make_decision<T: Borrow<P>, U: Borrow<Range<V>>>(
        &self,
        potential_packages: impl Iterator<Item = (T, U)>,
    ) -> Result<(T, Option<V>), Box<dyn Error>>;

    /// Retrieves the package dependencies.
    /// Return [Dependencies::Unknown] if its dependencies are unknown.
    fn get_dependencies(
        &self,
        package: &P,
        version: &V,
    ) -> Result<Dependencies<P, V>, Box<dyn Error>>;

    /// This is called fairly regularly during the resolution,
    /// if it returns an Err then resolution will be terminated.
    /// This is helpful if you want to add some form of early termination like a timeout,
    /// or you want to add some form of user feedback if things are taking a while.
    /// If not provided the resolver will run as long as needed.
    fn should_cancel(&self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}
```

As you can see, implementing the `DependencyProvider` trait requires you
to implement two functions, `make_decision` and `get_dependencies`.
The first one, `make_decision` is called by the resolver when a new
package has to be tried.
At that point, the resolver call `make_decision` with a list
of potential packages and their associated acceptable version ranges.
It's then the role of the dependency retriever to pick a package
and a suitable version in that range.
The simplest decision strategy would be to just pick the first package,
and first compatible version. Provided there exists a method
`fn available_versions(package: &P) -> impl Iterator<Item = &V>` for your type,
it could be implemented as follows.
We discuss advanced [decision making strategies later](./strategy.md).

```rust
fn make_decision<T: Borrow<P>, U: Borrow<Range<V>>>(
    &self,
    potential_packages: impl Iterator<Item = (T, U)>,
) -> Result<(T, Option<V>), Box<dyn Error>> {
    let (package, range) = potential_packages.next().unwrap();
    let version = self
        .available_versions(package.borrow())
        .filter(|v| range.borrow().contains(v))
        .next();
    Ok((package, version.cloned()))
}
```

The second required method is the `get_dependencies` method.
For a given package version, this method should return
the corresponding dependencies.
Retrieving those dependencies may fail due to IO or other issues,
and in such cases the function should return an error.
Even if it succeeds, we want to distinguish the cases
where the answer is that we don't know the package dependencies
and the cases where the package has no dependencies.
For this reason, the return type in case of a success is the
`Dependencies` enum, defined as follows.

```rust
pub enum Dependencies<P: Package, V: Version> {
    Unknown,
    Known(DependencyConstraints<P, V>),
}

pub type DependencyConstraints<P, V> = Map<P, Range<V>>;
```

Finally, there is an optional `should_cancel` method.
As described in its documentation,
this method is regularly called in the solver loop,
and defaults to doing nothing.
But if needed, you can override it to provide custom behavior,
such as giving some feedback, or stopping early to prevent ddos.
Any useful behavior would require mutability of `self`,
and that is possible thanks to interior mutability.
Read on the [next section](./caching.md) for more info on that!
