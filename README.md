# PubGrub version solving algorithm.

It consists in efficiently finding a set of packages and versions
that satisfy all the constraints of a given project dependencies.
In addition, when that is not possible,
PubGrub tries to provide a very human-readable and clear
explanation as to why that failed.
Below is an example of an explanation present in
the introductory blog post about PubGrub.

```txt
Because dropdown >=2.0.0 depends on icons >=2.0.0 and
  root depends on icons <2.0.0, dropdown >=2.0.0 is forbidden.

And because menu >=1.1.0 depends on dropdown >=2.0.0,
  menu >=1.1.0 is forbidden.

And because menu <1.1.0 depends on dropdown >=1.0.0 <2.0.0
  which depends on intl <4.0.0, every version of menu
  requires intl <4.0.0.

So, because root depends on both menu >=1.0.0 and intl >=5.0.0,
  version solving failed.
```

The algorithm is generic and works for any type of dependency system
as long as packages (P) and versions (V) implement
the `Package` and `Version` traits.
`Package` is strictly equivalent and automatically generated
for any type that implement `Clone + Eq + Hash + Debug + Display`.
`Version` simply states that versions are ordered,
that there should be
a minimal `lowest` version (like 0.0.0 in semantic versions),
and that for any version, it is possible to compute
what the next version closest to this one is (`bump`).
For semantic versions, `bump` corresponds to an increment of the patch number.


## API

```rust
let solution = resolve(&dependency_provider, package, version)?;
```

Where `dependency_provider` supplies the list of available packages and versions,
as well as the dependencies of every available package
by implementing the `DependencyProvider` trait.
The call to `resolve` for a given package at a given version
will compute the set of packages and versions needed
to satisfy the dependencies of that package and version pair.
If there is no solution, the reason will be provided as clear as possible.


## Contributing

Discussion and development happens here on github and on our [Zulip stream](https://rust-lang.zulipchat.com/#narrow/stream/260232-t-cargo.2FPubGrub). Please join in!

## PubGrub

PubGrub is a version solving algorithm,
written in 2018 by Natalie Weizenbaum
for the Dart package manager.
It is supposed to be very fast and to explain errors
more clearly than the alternatives.
An introductory blog post was
[published on Medium][medium-pubgrub] by its author.

The detailed explanation of the algorithm is
[provided on GitHub][github-pubgrub].
The foundation of the algorithm is based on ASP (Answer Set Programming),
and a book called
"[Answer Set Solving in Practice][potassco-book]"
by Martin Gebser, Roland Kaminski, Benjamin Kaufmann and Torsten Schaub.

[medium-pubgrub]: https://medium.com/@nex3/pubgrub-2fb6470504f
[github-pubgrub]: https://github.com/dart-lang/pub/blob/master/doc/solver.md
[potassco-book]: https://potassco.org/book/
