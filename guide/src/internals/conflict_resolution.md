# Conflict resolution

As stated before, a conflict is a satisfied incompatibility
that we detected in the unit propagation loop.
The goal of conflict resolution is to backtrack the partial solution
such that we have the following guarantees:

1. The root cause incompatibility of the conflict is almost satisfied
   (such that we can continue unit propagation).
2. The following derivations will be different than before conflict resolution.

Let the "satisfier" be the earliest assignment in the partial solution
making the incompatibility fully satisfied by the partial solution up to that point.
We know that we want to backtrack the partial solution at least previous to that assignment.
Backtracking only makes sense if done at decision levels frontiers.
As such the conflictual incompatibility can only become "almost satisfied"
if there is only one package term related to incompatibility satisfaction
at the decision level of that satisfier.
When the satisfier is a decision this is trivial since all previous assignments
are of lower decision levels.
When the satisfier is a derivation however we need to check that property.
We do that by computing the "previous satisfier" decision level.
The previous satisfier is (if it exists) the earliest assignment
previous to the satisfier such that the partial solution up to that point,
plus the satisfier, makes the incompatibility satisfied.
Once we found it, we know that property (1) is guaranteed as long as
we backtrack to a decision level between the one of the previous satisfier
and the one of the satisfier, as long as these are different.

If the satisfier and previous satisfier decisions levels are the same,
we cannot guarantee (1) for that incompatibility after backtracking.
Therefore, the key of conflict resolution is to derive a new incompatibility
for which we will be able to guarantee (1).
And we have seen how to do that with the
[rule of resolution](incompatibilities.md#rule-of-resolution).
We will derive a new incompatibility called the "prior cause"
as the resolvent of the current incompatibility and
the incompatibility which is the cause of the satisfier.
If necessary, we repeat that procedure until finding an incompatibility,
called the "root cause" for which we can guarantee that it will
be almost satisfied after backtracking (1).

Now the question is where do we cut?
Is there a reason we cut at the previous satisfier decision level?
Is it to guarantee (2)? Would that not be guaranteed if we picked
another decision level? Is it because backtracking further back
will reduce the number of potential conflicts?
