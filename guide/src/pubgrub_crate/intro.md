# Using the pubgrub crate

The PubGrub algorithm works for any package type
implementing `Clone + Eq + Hash + Debug + Display`,
and any version type implementing our `Version` trait, defined as follows.

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

Now that we know the `Package` and `Version` trait requirements,
let's explain how to actually use `pubgrub` with a simple example.
