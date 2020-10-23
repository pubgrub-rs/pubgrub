# Basic example with OfflineDependencyProvider

Let's imagine that we are building a user interface
with a menu containing dropdowns with some icons,
icons that we are also directly using in other parts of the interface.
For this scenario our direct dependencies are `menu` and `icons`,
but the complete set of dependencies looks like follows.

- `user_interface` depends on `menu` and `icons`
- `menu` depends on `dropdown`
- `dropdown` depends on `icons`
- `icons` has no dependency

We can model that scenario as follows.

```rust
use pubgrub::solver::{OfflineDependencyProvider, resolve};
use pubgrub::version::NumberVersion;
use pubgrub::range::Range;

// Initialize a dependency provider.
let mut dependency_provider = OfflineDependencyProvider::<&str, NumberVersion>::new();

// Add all known dependencies.
dependency_provider.add_dependencies(
    "user_interface", 1, vec![("menu", Range::any()), ("icons", Range::any())],
);
dependency_provider.add_dependencies("menu", 1, vec![("dropdown", Range::any())]);
dependency_provider.add_dependencies("dropdown", 1, vec![("icons", Range::any())]);
dependency_provider.add_dependencies("icons", 1, vec![]);

// Run the algorithm.
let solution = resolve(&dependency_provider, "user_interface", 1).unwrap();
```

As we can see in the previous code example,
the key function of PubGrub version solver is `resolve`.
It takes as arguments a dependency provider,
as well as the package and version for which we want to solve
dependencies, here package `"user_interface"` at version 1.

The dependency provider must be an instance of a type implementing
the `DependencyProvider` trait defined in this crate.
That trait defines methods that the resolver can call
when looking for packages and versions to try in the solver loop.
For convenience and for testing purposes, we already provide
an implementation of a dependency provider called `OfflineDependencyProvider`.
As the names suggest, it doesn't do anything fancy
and you have to pre-register all known dependencies with calls to
`add_dependencies(package, version, vec_of_dependencies)`
before being able to use it in the `resolve` function.

Dependencies are specified with a `Range`,
ranges being versions constraints defining sets of versions.
In most cases, you would use `Range::between(v1, v2)`
which means any version higher or equal to `v1` and strictly lower than `v2`.
In the previous example, we just used `Range::any()`
which basically means any version.
