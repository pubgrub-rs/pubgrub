#![feature(test)]
extern crate test;
use test::Bencher;

use pubgrub::solver::{OfflineSolver, Solver};
use pubgrub::version::NumberVersion;

#[cfg(feature = "serde")]
#[bench]
/// This is an entirely synthetic benchmark. It may not be realistic.
/// It is too slow to be useful in the long term. But hopefully that can be fixed by making `run` faster.
/// It has not bean minimized. There are meny `add_dependencies` that have no impact on the runtime or output.
fn large_case(b: &mut Bencher) {
    let s = std::fs::read_to_string("test-examples/large_case_u16_NumberVersion.ron").unwrap();
    let mut solver: OfflineSolver<u16, NumberVersion> = ron::de::from_str(&s).unwrap();

    // bench
    b.iter(|| {
        let _ = solver.run(0, 0);
    });
}
