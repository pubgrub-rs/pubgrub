# Overview of the algorithm


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
When there is no more decision to be made,
the algorithm returns the decisions from the partial solution.


## Unit propagation overview

Unit propagation is the core mechanism of the algorithm.
For the currently selected package,
unit propagation aims at deriving new constraints
(called "terms" in PubGrub and "literals" in CDNL terminology),
from all incompatibilities referring to that package.
For example, if an incompatibility specifies that packages a and b
are incompatible, and if we just made a decision for package a,
then we can derive a term specifying that package b should not appear in the solution.

While browsing incompatibilities, we may stumble upon one that is already "satisfied"
by the current state of the partial solution.
In our previous example, that would be the case if
we had previously already made a decision for package b
(in practice that exact situation could not happen but let's leave that subtlety for later).
If an incompatibility is satisfied, we call that a conflict and must perform conflict resolution,
and backtrack the partial solution to a state previous to the root cause of the conflict.


## Conflict resolution overview

Conflict resolution aims at finding the root cause of a conflict,
recording it in an incompatibility,
and backtracking the partial solution to a state
previous to the decision at the root of the conflict.
This is performed by a loop composed of two steps:

1. Find the earliest assignment in the partial solution such that
   the conflictual incompatibility is satisfied by
   the partial solution until there.
   That assignment is called the "satisfier".
2. If the satisfier is the root cause of the conflict,
   end the loop and backtrack the partial solution.
   Otherwise, compute the "prior cause" of the satisfier,
   which is a new incompatibility and continue the loop
   with that one as the conflictual incompatibility.
