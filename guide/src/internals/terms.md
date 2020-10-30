# Terms


## Definition

A term is an atomic variable, called ["literal"][literal] in mathematical logic,
that is evaluated either true or false depending on the evaluation context.
In PubGrub, a term is either a positive or a negative range of versions defined as follows.

```rust
pub enum Term<V> {
    Positive(Range<V>),
    Negative(Range<V>),
}
```

A positive term is evaluated true if and only if
a version contained in its range was selected.
A negative term is evaluated true if and only if
a version not contained in its range was selected,
or no version was selected.
The `negate()` method transforms a positive term into a negative one and vice versa.
In this guide, for any given range \\(r\\),
we will note \\([r]\\) its associated positive term,
and \\(\neg [r]\\) its associated negative term.
And for any term \\(T\\), we will note \\(\overline{T}\\) the negation of that term.
Therefore we have the following rules,

\\[\begin{eqnarray}
\overline{[r]} &=& \neg [r], \nonumber \\\\
\overline{\neg [r]} &=& [r]. \nonumber \\\\
\end{eqnarray}\\]

[literal]: https://en.wikipedia.org/wiki/Literal_(mathematical_logic)


## Special terms

Provided that we have defined an empty range of versions \\(\varnothing\\),
there are two terms with special behavior which are \\([\varnothing]\\) and \\(\neg [\varnothing]\\).
By definition, \\([\varnothing]\\) is evaluated true if and only if
a version contained in the empty range is selected.
This is impossible and as such the term \\([\varnothing]\\) is always evaluated false.
And by negation, the term \\(\neg [\varnothing]\\) is always evaluated true.


## Intersection of terms

We define the "intersection" of two terms
as the conjunction of those two terms (a logical AND).
Therefore, if any of the terms is positive, the intersection also is a positive term.
Given any two ranges of versions \\(r_1\\) and \\(r_2\\), the intersection of terms
based on those ranges is defined as follows,

\\[\begin{eqnarray}
[r_1] \cap [r_2] &=& [r_1 \cap r_2],                 \nonumber \\\\
[r_1] \cap \neg [r_2] &=& [r_1 \cap \overline{r_2}], \nonumber \\\\
\neg [r_1] \cap \neg [r_2] &=& \neg [r_1 \cup r_2].  \nonumber \\\\
\end{eqnarray}\\]

And for any two terms \\(T_1\\) and \\(T_2\\), their union and intersection are related by

\\[ \overline{T_1 \cup T_2} = \overline{T_1} \cap \overline{T_1}. \\]


## Relation between terms

We say that a term \\(T_1\\) is satisfied by another term \\(T_2\\)
if \\(T_2\\) implies \\(T_1\\), i.e.
when \\(T_2\\) is evaluated true then \\(T_1\\) must also be true.
This is equivalent to saying that \\(T_2\\) is a subset of \\(T_1\\),
which is verified if \\( T_2 \cap T_1 = T_2 \\).

> **Note on comparability of terms:**
>
> Checking if a term is satisfied by another term is accomplished
> in the code by verifying if the intersection of the two terms
> equals the second term.
> It is thus very important that terms have unique representations,
> and by consequence also that **ranges have a unique representation**.
>
> In the provided `Range` type, ranges are implemented
> as an ordered vec of non-intersecting semi-open intervals.
> It is thus important that they are always reduced to their
> canonical form.
> For example, the range `2 <= v < 2` is actually empty
> and thus should not be represented by `vec![(2, Some(2))]`
> but by the empty `vec![]`.
> **This requires special care when implementing range intersection**.
