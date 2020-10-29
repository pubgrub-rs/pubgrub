# Internals of the PubGrub algorithm


For an alternative / complementary explanation of the PubGrub algorithm,
you can read the detailed description of the solver
provided by the original PubGrub author in the GitHub repository
of the dart package manager [pub][pub].

PubGrub is an algorithm inspired by conflict-driven nogood learning (CDNL-ASP),
an [approach presented by Gabser, Kaufmann and Schaub in 2012][ass].
The approach consists in iteratively taking decisions
(here picking a package and version) until reaching a conflict.
At that point it records a nogood (an "incompatibility" in PubGrub terminology)
describing the root cause of the conflict
and backtracks to a state previous to the decision leading to that conflict.
CDNL has many similarities with [CDCL][cdcl] (conflict-driven clause learning)
with the difference that nogoods are conjunctions
while clauses are disjunctions of literals.
More documentation of their approach is available on [their website][potassco].

At any moment, the PubGrub algorithm holds a state composed of two things,
(1) a partial solution and (2) a set of incompatibilities.
The partial solution (1) is a chronological list of "assignments",
which are either decisions taken or version constraints
for packages where no decision was made yet.
The set of incompatibilities (2) is an ever-growing collection of
incompatibilities.
We will describe them in more details later but simply put,
an incompatibility describes packages that are dependent or incompatible,
that is packages that must be present at the same time
or that cannot be present at the same time in the solution.

Incompatibilities express facts, and as such are always valid.
Therefore, the set of incompatibilities is never backtracked,
only growing and recording new knowledge along the way.
In contrast, the partial solution contains decisions and deductions
(called "derivations" in PubGrub terminology),
that are dependent on every decision made.
Therefore, PubGrub needs to be able to backtrack the partial solution
to an older state when there is a conflict.

[pub]: https://github.com/dart-lang/pub/blob/master/doc/solver.md
[ass]: https://www.sciencedirect.com/science/article/pii/S0004370212000409
[cdcl]: https://en.wikipedia.org/wiki/Conflict-driven_clause_learning
[potassco]: https://potassco.org/book/
