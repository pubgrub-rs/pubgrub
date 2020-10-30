# Building a report tree


## Terminal incompatibility

Whenever we build an incompatibility that will always be satisfied,
version solving has failed.
The empty incompatibility, for example is an incompatibility
for which terms of all packages are \\( \neg [\varnothing] \\)
and thus all terms are satisfied.
Another terminal incompatibility is an incompatibility
containing a single package term, satisfied by picking
the package and version at the root of the solution,
the one for which we want to resolve dependencies.


## Derivation tree to report tree

Incompatibilities are either external or derived.
External incompatibilities are the one expressing facts
independent of the solver deductions capabilities,
such as dependencies, unavailable dependencies, missing versions etc.
In contrast, derived incompatibilities are the one obtained
through the rule of resolution when computing prior causes.
Every derived incompatibility keeps a reference to the two
incompatibilities that were used to derive it.
As a consequence, a chain of derived incompatibilities defines
a binary tree, that we call the derivation tree.

When version solving failed, it is thus possible to take
the derivation tree of the terminal incompatibility to build
a complete explanation of why it failed.
That derivation tree however is using incompatibilities
whose shape are dependent on internal implementation details.
The purpose of the report tree is then to transform the derivation tree
into a data structure easily usable for reporting.


## Report tree type

In order to provide information in the most detailed way,
the report tree uses enum types that try to be as self-explanatory as possible.
I'm not sure the current design, based on a recursive (boxed) enum is the best one
but it has the advantage of presenting the binary report tree
in a very straightforward manner, easy to process.


## Duplicate nodes

Though it has the shape of a binary tree, and can be represented as a binary tree,
the derivation tree is actually a derivation graph.
Indeed, even if every derived incompatibility was built from two others,
some incompatibilities may have been used to derive multiple new incompatibilities.

TODO: ascii representations similar to the ones in
the [error reporting section of pub solver.md][error-reporting].

[error-reporting]: https://github.com/dart-lang/pub/blob/master/doc/solver.md#error-reporting

There is still much API exploration work to be done on error reporting.
