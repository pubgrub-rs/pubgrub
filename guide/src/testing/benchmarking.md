# Benchmarking

TODO: This section has to be improved.
For now I'm just linking to issue #27 where
I described a bit how to start with benchmarking.

## Performance analysis

## Profiling

## Effect of code layout on performance

Changing your username has an impact on the performances of your code.
This is not clickbait I promess, but simply an effect of layout changes.
Se before making any assumption on performance improvement or regression
try making sure that measures are actually reflecting the intent
of the code changes and not something else.
It was shown that code layout changes can produce ±40% in performance changes.

I highly recommend watching the talk ["Performance Matters" by Emery Berger][perf-talk]
presented at strangeloop 2019 for more information on
"sound performance analysis" and "effective performance profiling".
There is an [issue in criterion][criterion-stabilizer] about the "stabilizer" tool.
And for causal profiling, it seems possible to directly use [coz][coz] for rust programs.

<iframe width="560" height="315" src="https://www.youtube-nocookie.com/embed/r-TLSBdHe1A?start=656" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture" allowfullscreen></iframe>

[perf-talk]: https://youtu.be/r-TLSBdHe1A?t=656
[criterion-stabilizer]: https://github.com/bheisler/criterion.rs/issues/334
[coz]: https://github.com/plasma-umass/coz/tree/master/rust
