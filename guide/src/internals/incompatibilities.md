# Incompatibilities


## Definition

Incompatibilities are called "nogoods" in [CDNL-ASP][ass] terminology.
**An incompatibility is a [conjunction][conjunction] of package terms that must
be evaluated false**, meaning at least one package term must be evaluated false.
Otherwise we say that the incompatibility has been "satisfied".
Satisfied incompatibilities represent conflicts and thus
the goal of the PubGrub algorithm is to build a solution
such that none of the produced incompatibilities are ever satisfied.
If one incompatibility becomes satisfied at some point,
the algorithm finds the root cause of it and backtracks the partial solution
before the decision at the origin of that root cause.

> Remark: incompatibilities (nogoods) are the opposite of clauses
> in traditional conflict-driven clause learning ([CDCL][cdcl])
> which are disjunctions of literals that must be evaluated true,
> so have at least one literal evaluated true.
>
> The gist of CDCL is that it builds a solution to satisfy a
> [conjunctive normal form][cnf] (conjunction of clauses) while
> CDNL builds a solution to unsatisfy a [disjunctive normal form][dnf]
> (disjunction of nogoods).
>
> In addition, PubGrub is a lazy CDNL algorithm since the disjunction of nogoods
> (incompatibilities) is built on the fly with the solution.

[ass]: https://www.sciencedirect.com/science/article/pii/S0004370212000409
[cdcl]: https://en.wikipedia.org/wiki/Conflict-driven_clause_learning
[conjunction]: https://en.wikipedia.org/wiki/Logical_conjunction
[cnf]: https://en.wikipedia.org/wiki/Conjunctive_normal_form
[dnf]: https://en.wikipedia.org/wiki/Disjunctive_normal_form

In this guide, we will note incompatibilities with curly braces.
An incompatibility containing one term \\(T_a\\) for package \\(a\\)
and one term \\(T_b\\) for package \\(b\\) will be noted

\\[ \\{ a: T_a, b: T_b \\}. \\]

> Remark: in a more "mathematical" setting, we would probably have noted
> \\( T_a \land T_b \\), but the chosen notation maps well
> with the representation of incompatibilities as hash maps.


## Properties

**Packages only appear once in an incompatibility**.
Since an incompatibility is a conjunction,
multiple terms for the same package are merged with the intersection of those terms.

**Terms that are always satisfied can be removed from an incompatibility**.
We previously explained that the term \\( \neg [\varnothing] \\) is always evaluated true.
As a consequence, it can safely be removed from the conjunction of terms that is the incompatibility.

\\[ \\{ a: T_a, b: T_b, c: \neg [\varnothing] \\} = \\{ a: T_a, b: T_b \\} \\]

**Dependencies can be expressed as incompatibilities**.
Saying that versions in range \\( r_a \\) of package \\( a \\)
depend on versions in range \\( r_b \\) of package \\( b \\)
can be expressed by the incompatibility

\\[ \\{ a: [r_a], b: \neg [r_b] \\}. \\]


## Unit propagation

If all terms but one of an incompatibility are satisfied by a partial solution,
we can deduce that the remaining unsatisfied term must be evaluated false.
We can thus derive a new unit term for the partial solution
which is the negation of the remaining unsatisfied term of the incompatibility.
For example, if we have the incompatibility
\\( \\{ a: T_a, b: T_b, c: T_c \\} \\)
and if \\( T_a \\) and \\( T_b \\) are satisfied by terms in the partial solution
then we can derive that the term \\( \overline{T_c} \\) can be added for package \\( c \\)
in the partial solution.


## Rule of resolution

Intuitively, we are able to deduce things such as if package \\( a \\)
depends and package \\( b \\) which depends on package \\( c \\),
then \\( a \\) depends on \\( c \\).
With incompatibilities, we would note

\\[               \\{ a: T_a, b: \overline{T_b} \\}, \quad
                  \\{ b: T_b, c: \overline{T_c} \\}  \quad
\Rightarrow \quad \\{ a: T_a, c: \overline{T_c} \\}. \\]

This is the simplified version of the rule of resolution.
For the generalization, let's reuse the "more mathematical" notation of conjunctions
for incompatibilities \\( T_a \land T_b \\) and the above rule would be

\\[               T_a \land \overline{T_b}, \quad
                  T_b \land \overline{T_c}  \quad
\Rightarrow \quad T_a \land \overline{T_c}. \\]

In fact, the above rule can also be expressed as follows

\\[               T_a \land \overline{T_b}, \quad
                  T_b \land \overline{T_c}  \quad
\Rightarrow \quad T_a \land (\overline{T_b} \lor T_b) \land \overline{T_c} \\]

since for any term \\( T \\), the disjunction \\( T \lor \overline{T} \\) is always true.
In general, for any two incompatibilities \\( T_a^1 \land T_b^1 \land \cdots \land T_z^1 \\)
and \\( T_a^2 \land T_b^2 \land \cdots \land T_z^2 \\) we can deduce a third,
called the resolvent whose expression is

\\[ (T_a^1 \lor T_a^2) \land (T_b^1 \land T_b^2) \land \cdots \land (T_z^1 \land T_z^2). \\]

In that expression, only one pair of package terms is regrouped as a union (a disjunction),
the others are all intersected (conjunction).
If a term for a package does not exist in one incompatibility,
it can safely be replaced by the term \\( \neg [\varnothing] \\) in the expression above
as we have already explained before.
