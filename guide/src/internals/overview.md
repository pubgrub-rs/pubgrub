# Overview of the algorithm

PubGrub is a conflict driven clause learning ([CDCL][cdcl]) algorithm.
This means that it takes decisions (chooses package versions)
until reaching a conflict.
At that moment it records a clause,
also called "incompatibility" in this documentation,
describing the root cause of the conflict
and backtracks to a state previous to the decision leading to that conflict.

At any moment, the algorithm holds a state composed of two things,
(1) a partial solution and (2) a set of incompatibilities.
The partial solution is composed of decisions taken,
and version constraints for packages where no decision was made yet.
The set of incompatibilities contains an ever-growing list of
incompatibilities (clauses).
We will describe them in more details later,
but simply put, an incompatibility describes packages that are incompatible,
that is packages that cannot be present at the same time in the solution.

Incompatibilities express facts, and as such are always valid.
Therefore, the set of incompatibilities is never backtracked,
only growing and recording new knowledge along the way.
In contrast, the partial solution contains decisions and deductions ("derivations"),
that are dependent on every decision made.
Therefore, PubGrub needs to be able to backtrack the partial solution
to an older state when there are conflicts.

[cdcl]: todo


## Solver main loop

The solver runs in a loop with the following steps:

1. Perform unit propagation on the currently selected package.
2. Make a decision: pick a new package and version
   compatible with the current state of the partial solution.
3. Retrieve dependencies for the newly selected package
   and transform those into incompatibilities.

At any point within the loop, the algorithm may fail
due to an impossibility to solve a conflict or
an error occuring while trying to retrieve dependencies.


## Unit propagation

Unit propagation is the core mechanism of the algorithm.
For the currently selected package,
unit propagation aims at deriving new constraints, called "terms",
from all incompatibilities referring to that package.
For example, if an incompatibility specifies that packages a and b
are incompatible, and if we just made a decision for package a,
then we can derive a term specifying that package b should not appear in the solution.

While browsing incompatibilities, we may stumble upon one that is already "satisfied"
by the current state of the partial solution.
In our previous example, that would be the case if
we had previously already made a decision for package b
(in practice that exact situation could not happen but let's leave that subtlety for later).
If an incompatibility is satisfied, we then must perform conflict resolution,
and backtrack the partial solution to a state previous to the root cause of the conflict.


## Conflict resolution

Conflict resolution aims at finding the root cause of a conflict,
recording it in an incompatibility,
and backtracking the partial solution to a state
previous to the decision at the root of the conflict.
This is performed by a loop composed of two steps:

1. Find the earliest term in the partial solution such that
   the conflictual incompatibility is satisfied by all terms
   in the partial solution until this one, called the "satisfier".
2. If the satisfier is the root cause of the conflict,
   end the loop and backtrack the partial solution.
   Otherwise, compute the "prior cause" of the satisfier,
   which is a new incompatibility and continue the loop
   with that one as the conflictual incompatibility.
