# Conflict resolution

As stated before, a conflict is a satisfied incompatibility.
We may detect such conflict while generating new derivations
in the unit propagation loop.
Given a satisfied incompatibility, conflict resolution aims at
finding the root cause of the conflict
and backtracking the partial solution just before the decision at its origin.

TODO: explain how finding the earliest satisfier fits into this.

TODO: explain that prior cause is just the previously explained
rule of resolution on two incompatibilities, the current one,
and the one being the cause of the earliest satisfier.
