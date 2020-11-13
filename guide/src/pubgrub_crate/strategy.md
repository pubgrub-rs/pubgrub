# Strategical decision making in a DependencyProvider

In PubGrub, decision making is the process of
choosing the next package and version that will be appended
to the solution being built.
Every time such a decision must be made,
potential valid packages are preselected
with corresponding valid ranges of versions.
Then, there is some freedom regarding which of those
package versions to choose.

The strategy employed to choose such package and version
cannot change the existence of a solution or not,
but can drastically change the performances of the solver,
or the properties of the solution.
The documentation of Pub, the implementation for the dart programming language
states the following.

> Pub chooses the latest matching version
> of the package with the fewest versions
> that match the outstanding constraint.
> This tends to find conflicts earlier if any exist,
> since these packages will run out of versions to try more quickly.
> But there's likely room for improvement in these heuristics.

In pubgrub, decision making responsability is split in two places.
The resolver takes care of making a preselection for potential packages
and corresponding ranges of versions.
Then it's the dependency provider that has the freedom of employing
the strategy it wants to pick one package version within
the `make_decision` method.

```rust
fn make_decision<T: Borrow<P>, U: Borrow<Range<V>>>(
    &self,
    packages: impl Iterator<Item = (T, U)>,
) -> Result<(T, Option<V>), Box<dyn Error>>;
```


## Picking a package


## Picking a version
