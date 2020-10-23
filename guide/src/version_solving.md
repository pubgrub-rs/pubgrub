# Version solving

Version solving consists in finding a set of packages and versions
that satisfy all the constraints of a given project dependencies.
In Rust, it is the package manager _Cargo_ that takes the dependencies specified
in the `Cargo.toml` file and deduces the complete list of exact versions
of packages needed for your code to run.
That includes direct dependencies but also indirect ones,
which are dependencies of your dependencies.
Packages and versions are not restricted to code libraries though.
In fact they could refer to anything where "packages" act as a general
name for things, and "versions" describe the evolution of those things.
Such things could be office documents, laws, cooking recipes etc.
Though if you version cooking recipes, well you are a very organized person.


## Semantic versioning

The most common form of versioning scheme for dependencies is called
[semantic versioning][semver].
The base idea of semantic versioning is to provide versions as triplets
of the form `Major.Minor.Patch` such as `2.4.1`.
The "semantic" term comes from the meaning of each part of the versioning system.
Publishing a new patch version, for example from `2.4.1` to `2.4.2` means
no interface has changed in the provided API of the library.
That may be the case for documentation changes,
or internal performance improvements for example.
Increasing the minor number, for example from `2.4.1` to `2.5.0`,
means that things have been added to the API,
but nothing was changed in the pre-existing API provided by the library.
Finally, increasing the major number, such as from `2.4.1` to `3.0.0`,
means that some parts of the API have changed
and thus may be incompatible with how we are currently using the previous version.

In Rust, packages are called crates and use semantic versioning.
In fact, if you specify a dependency of the form `package = "2.4.1"`,
cargo will interpret that as the version constraint `2.4.1 <= v < 3.0.0` for that package.
It does so based on the fact that any version in that range should not break
our code according to the rules of semantic versioning.
For more information on dependencies specifications in `Cargo.toml`
you can read the [Cargo reference book][cargo-ref].

[semver]: https://semver.org/
[cargo-ref]: https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html


## Side note on semantic versioning

Some people think that the granularity of semantic versioning
is too broad in the case of major version changes.
Instead, versions should never be breaking, but use new namespaces
for things that change.
It brings the same benefits in the large,
that what choosing immutability as default brings in the small.
For more information on this point of view,
I highly recommend ["Spec-ulation" by Rich Hickey][speculation],
creator of the Clojure programming language.

[speculation]: https://youtu.be/oyLBGkS5ICk


## Version solving with PubGrub

The algorithm provided by this crate, called PubGrub does not care
if versions follow the semantic versioning scheme or not.
We simply define a `Version` trait as follows, based on an ordered type.

```rust
pub trait Version: Clone + Ord + Debug + Display {
    fn lowest() -> Self;
    fn bump(&self) -> Self;
}
```

The `lowest()` trait method should return the lowest version existing,
and `bump(&self)` should return the smallest version stricly
higher than the current one.

For convenience, we already provide the `SemanticVersion` type,
which implements `Version` for versions expressed as `Major.Minor.Patch`.
We also provide the `NumberVersion` implementation of `Version`,
which is basically just a newtype for non-negative integers, 0, 1, 2, etc.
