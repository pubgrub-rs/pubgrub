# Property testing

To test pubgrub, we employ a mix of unit tests, integration tests and property tests.
Some unit tests are present for example in the `version` module to validate
that the parsing of semantic versions is correct.
Integration tests are located in the `tests/` directory.
Property tests are co-located both with unit tests and integration tests
depending on required access to some private implementation details.


## Examples

We have multiple example cases inside `tests/examples.rs`.
Those mainly come from [dart documentation of the solver][dart-solver]
and are simple end-to-end example for which we know the results.
The first example called `no_conflict` is a simple case where
the root package depends on one package which itself has a dependency
to another package.
The tests there compare the expected result with the solution of the solver
for each of those examples.
These were very useful when making progress on PubGrub implementation
and can now also be referred to as example usage of the API.

[dart-solver]: https://github.com/dart-lang/pub/blob/master/doc/solver.md


## Unit property testing

Property testing consists in defining invariants,
generating random valid inputs for the tested functions,
and verifying that the invariants hold for the results.
Here are some examples extracted from the `range` module.

```rust
proptest! {
    #[test]
    fn negate_is_different(range in strategy()) {
        assert_ne!(range.negate(), range);
    }

    #[test]
    fn double_negate_is_identity(range in strategy()) {
        assert_eq!(range.negate().negate(), range);
    }

    #[test]
    fn negate_contains_opposite(range in strategy(), version in version_strat()) {
        assert_ne!(range.contains(&version), range.negate().contains(&version));
    }
}
```

As you can see, the input of the testing function is specified
in an unusual manner, using a function called a "strategy".
This is the terminology used by the [proptest crate][proptest]
and it simply is a way to describe how are randomly generated the values
used as inputs to those property tests.
Don't hesitate to have a look at the corresponding `strategy()` function
defined just above the extracted code if you want to know more about that.

[proptest]: https://altsysrq.github.io/rustdoc/proptest/latest/proptest/index.html


## End-to-end property testing

Defining and testing properties for small units of code
like ranges implementation is rather easy.
Coming up with interesting properties for end-to-end testing
such as results of full resolution is different.
But the most difficult part is probably finding a "strategy"
to generate random but valid registries of packages and dependencies.
This is the work that has been done in `tests/proptest.rs`
for the `registry_strategy()` function.

The simplest implementation of `registry_strategy()` would be 
`any::<Map<P, Map<V, Map<P, Range<V>>>>()`, with some machinery to convert to a `OfflineDependencyProvider`.
While this works, almost all the cases will not be solvable, as with a high probability the 
depended on package will not exist. 

Let's defer picking the dependency's until we have a list of available names.
Luckily proptest has a [Index]
type for this use case of picking something out of a list that has not yet bean generated. This 
leaves us with `any::Map<P, Map<V, Map<Index, Range<V>>>()`, with more machinery to get the `P` for each index.
While this works, almost all the cases will not be solvable, as with a high probability the 
`Range<V>` will not match any synthesized versions.

[Index]: (https://docs.rs/proptest/0.10/proptest/sample/struct.Index.html) 

Let's use [Index] to ensure our `Range`s are relevant. But we have two many [Index]s to talk about,
so lets give them some names. Lets say `Ip` for the one to pick the package, `Ia` and `Ib` for
the ones that define the range. This leaves us with `any::Map<P, Map<V, Map<Ip, (Ia, Ib)>>()`.
We convert by picking a dependency `P` using `Ip` then look up all the versions synthesized for that
package. Then we can call `Range::between(versions[Ia], versions[Ib].bump())` to make a range. 
Ok, half the time this will not be a valid range but we can fix it. If `versions[Ib] < versions[Ia]`
 then we use `Range::between(versions[Ib], versions[Ia].bump())`.

Now we finally have something that makes interesting registries! But not particularly realistic ones,
almost all of them end up with packages that indirectly depend on themselves. That kind of circular 
dependency is not allowed by most uses of PubGrub. To fix this lets make sure that we have a DAG,
by having each package only depend on packages of a lower index. One problem solved only to make a
new one. The [pigeonhole principle](https://en.wikipedia.org/wiki/Pigeonhole_principle) strikes again. 
Low index packages depend on all previous packages, and high index packages depend on two few.
The solution is to represent dependencies in a side list `Vec<(Ip, Id, (Ia, Ib))>`. If `Ip < Id` then
we switch them to maintain the DAG. In each record we add a dependency to `packages[Ip]` on 
`packages[Id]` in the range `versions[Ia], versions[Ib].bump()` as described above. And this is the 
best we currently know how to do, suggestions are welcome.


Generating random indexes of packages may produce cases
where dependency resolution would take too long.
For this reason, we introduced in the `DependencyProvider` trait definition
a function called `should_cancel` which is called in the solver loop.
By default it does nothing but can be overwritten such as
in `TimeoutDependencyProvider` defined there,
where the solver is stopped after a certain amount of time.

Once all this is setup, we have to come up with good properties.
Here are some of these:

- **The solver should return the same result on multiple runs with the same input**.
  That may seem trivial, but thinks like hashmaps do add some randomness.
  So that test ensures that we configured properly everything
  that could prevent reproducibility of the solver.
- **Changing the order in which valid package versions are tried
  should not change the existence of a solution or not**.
  Indeed, some freedom is available for the dependency provider
  to pick which package and version to choose next.
  We must ensure that it cannot change the existence of solution for
  our implementation of the solver algorithm.
- **Removing a dependency cannot prevent existence of a solution**.
  If a solution was found in a given situation,
  removing a dependency cannot get us in a situation where
  the solver does not find a solution anymore.
  Only adding dependencies should impact that.
- **Removing a package that does not appear in the dependency tree
  of a solution cannot break a solution**.
  Just as before, it should not impact the existence of a solution.


## Comparison with a SAT solver

In addition to the previous properties,
we can also compare the result of pubgrub with the one of an SAT solver.
The [SAT problem](https://en.wikipedia.org/wiki/Boolean_satisfiability_problem) asks if there is a
set of assignments for some Boolean variables so that all the provided Boolean logic statements 
evaluate to true. So if we can describe version solving as Boolean logic statements then we can use
these well tested tools to compare with our output.

We start by making a variable for each version of each package that expresses whether that version 
is selected to be used in the output. Lets call it \\( B_p_v \\).

One of the main constraints of version solving is that we can only have one version selected
 for each package. We can encode that by adding a logic statement \\( \\neg B_p_a \\parallel \\neg B_p_b \\)
 where \\(_a\\) and \\(_b\\) are different versions of package \\(_p\\). We use a more efficient but
 harder to understand encoding in the code, but this makes the point that it can be done.

The next main constraint of version solving is that for a version to be selected then for each of
 its dependencies there must be a selected version satisfying that dependency. We can encode that by 
 adding a logic statement \\( \\neg B_p_v \\parallel B_d_a \\parallel  ...  \\parallel  B_d_k \\) for
 each dependency of each version, where \\( {_a,  ... ,  _k} \\) are the versions that match the dependency.
 
The last constraint of version solving is that the root package needs to be selected. So we add the
logic statement \\( \\neg B_p_v \\) for the root package and version.

What comparisons can we do?

- **If pubgrub cannot find a solution then neither can the SAT solver.**
- **If pubgrub finds a solution then adding statements that match it dose not lead to a contradiction.**
