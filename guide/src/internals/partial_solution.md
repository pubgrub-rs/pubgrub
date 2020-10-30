# Partial solution

The partial solution is the part of PubGrub state holding all the decisions taken
and the derivations computed from unit propagation on almost satisfied incompatibilities.
We regroup decisions and derivations under the term "assignment".

The partial solution must be backtrackable when conflicts are detected.
For this reason all assignments are recorded with their associated decision level.
The decision level of an assignment is a counter for the number of decisions
that have already been taken (including that one if it is a decision).
If we represent all assignments as a chronological vec, they would look like follows:

```txt
[ (0, root_derivation),
  (1, root_decision),
  (1, derivation_1a),
  (1, derivation_1b),
  ...,
  (2, decision_2),
  (2, derivation_2a),
  ...,
]
```

The partial solution must also enable efficient evaluation of incompatibilities
in the unit propagation loop.
For this, we need to have efficient access to all assignments
referring to the packages present in an incompatibility.

To enable both efficient backtracking and efficient access to specific
package assignments, the current implementation holds a dual representation
of the the partial solution.
One is called `history` and keeps dated (with decision levels) assignments
in an ordered growing vec.
The other is called `memory` and organizes assignments in a hashmap
where they are regrouped by packages which are the hashmap keys.
It would be interresting to see how the partial solution is stored
in other implementations of PubGrub such as the one in [dart pub][pub].

[pub]: https://github.com/dart-lang/pub
