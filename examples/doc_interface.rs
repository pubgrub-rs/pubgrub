// SPDX-License-Identifier: MPL-2.0

use pubgrub::bounded_range::BoundedRange;
use pubgrub::solver::{resolve, OfflineDependencyProvider};
use pubgrub::version::NumberVersion;

type NumVS = BoundedRange<NumberVersion>;

// `root` depends on `menu` and `icons`
// `menu` depends on `dropdown`
// `dropdown` depends on `icons`
// `icons` has no dependency
#[rustfmt::skip]
fn main() {
    let mut dependency_provider = OfflineDependencyProvider::<&str, NumVS>::new();
    dependency_provider.add_dependencies(
        "root", 1, [("menu", BoundedRange::any()), ("icons", BoundedRange::any())],
    );
    dependency_provider.add_dependencies("menu", 1, [("dropdown", BoundedRange::any())]);
    dependency_provider.add_dependencies("dropdown", 1, [("icons", BoundedRange::any())]);
    dependency_provider.add_dependencies("icons", 1, []);

    // Run the algorithm.
    let solution = resolve(&dependency_provider, "root", 1);
    println!("Solution: {:?}", solution);
}
