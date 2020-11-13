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
The documentation of Pub
(PubGrub implementation for the dart programming language)
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
    potential_packages: impl Iterator<Item = (T, U)>,
) -> Result<(T, Option<V>), Box<dyn Error>>;
```


## Picking a package

Potential packages basically are packages that appeared in the dependencies
of a package we've already added to the solution.
So once a package appear in potential packages,
it will continue to be proposed as such until we pick it,
or a conflict shows up and the solution is backtracked before needing it.

Imagine that one such potential package is limited to a range
containing no existing version, we are heading directly to a conflict!
So we have better dealing with that conflict as soon as possible,
instead of delaying it for later since we will have to backtrack anyway.
Consequently, we always want to pick first a conflictual potential package
with no valid version.
Similarly, potential packages with only one valid version give us no choice
and limit options going forward, so we might want to pick such potential packages
before others with more version choices.
Generalizing this strategy to picking the potential package with the lowest
number of valid versions is a rather good heuristic performance-wise.

This strategy is the one employed by the `OfflineDependencyProvider`.
For convenience, we also provide a helper function `make_fewest_versions_decision_helper`
directly embedding this strategy.
It can be used directly in `make_decision` if provided
a helper function to retrieve existing versions of a package
`list_available_versions: Fn(&P) -> Iterator<Item = V>`.


## Picking a version

By default, the version returned by the helper function
`make_fewest_versions_decision_helper` is the first compatible one
in the iterator returned by `list_available_versions` for the chosen package.
So you can order the iterator with preferred versions first
and they will be picked by the solver.
This is very convenient to easily switch between a dependency provider
that picks the most recent compatible packages and one that chooses instead
the oldest compatible versions.
Such behavior may be desirable for checking that dependencies lower bounds
still pass the code tests for example.

In general, letting the dependency provider choose a version in
`make_decision` provides a great deal of flexibility and enables things like

- choosing the newest versions,
- choosing the oldest versions,
- choosing already downloaded versions,
- choosing versions specified in a lock file,

and many other desirable behavior for the resolver,
controlled directly by the dependency provider.
