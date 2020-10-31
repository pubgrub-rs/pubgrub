# Testing and benchmarking

Any non-trivial software is flawed and it is a programmer's goal
to make it correct, readable and fast.
Testing helps getting correct programs,
and benchmarking helps making them faster.
In this section we present the approach and tools used
to test and benchmark pubgrub.

**Note on reproducibility:**

"Insanity is doing the same thing over and over again, but expecting different results".
Einstein [probably didn't came up with that one][einstein-quote] but this is still
very much the definition of non-reproducibility,
and it can drive us mad when chasing [heisenbugs][heisenbug].
For this reason we try to avoid everything that would make pubgrub
non reproducible, such that every failed test can be reproduced and fixed.

[einstein-quote]: http://www.news.hypercrit.net/2012/11/13/einstein-on-insanity/
[heisenbug]: https://en.wikipedia.org/wiki/Heisenbug
