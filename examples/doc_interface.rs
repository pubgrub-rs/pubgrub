// SPDX-License-Identifier: MPL-2.0

use pubgrub::{resolve, OfflineDependencyProvider, Ranges};

type NumVS = Ranges<u32>;

// `root` depends on `menu` and `icons`
// `menu` depends on `dropdown`
// `dropdown` depends on `icons`
// `icons` has no dependency
#[rustfmt::skip]
fn main() {
    let mut dependency_provider = OfflineDependencyProvider::<&str, NumVS>::new();
    dependency_provider.add_dependencies(
        "root", 1u32, [("menu", Ranges::full()), ("icons", Ranges::full())],
    );
    dependency_provider.add_dependencies("menu", 1u32, [("dropdown", Ranges::full())]);
    dependency_provider.add_dependencies("dropdown", 1u32, [("icons", Ranges::full())]);
    dependency_provider.add_dependencies("icons", 1u32, []);

    // Run the algorithm.
    let solution = resolve(&dependency_provider, "root", 1u32);
    println!("Solution: {:?}", solution);
}
