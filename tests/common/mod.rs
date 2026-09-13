// SPDX-License-Identifier: MPL-2.0

use pubgrub::{OfflineDependencyProvider, Ranges};

/// The constrained-cousins backtracking case from issue #293, using integer package/version ids.
pub fn constrained_cousins(
    depth: u32,
    branching: u32,
) -> OfflineDependencyProvider<u32, Ranges<u32>> {
    let mut provider = OfflineDependencyProvider::new();
    let cloaking = u32::MAX - 1;
    let constrained = u32::MAX;
    let incompatible = branching + 10;
    provider.add_dependencies(0, 0u32, [(1, Ranges::full()), (cloaking, Ranges::full())]);
    provider.add_dependencies(
        cloaking,
        0u32,
        [(constrained, Ranges::strictly_lower_than(incompatible))],
    );
    for version in 1..=incompatible {
        provider.add_dependencies(constrained, version, []);
    }
    for n in 0..depth {
        for version in 1..branching {
            provider.add_dependencies(
                3 * n + 1,
                version,
                [(3 * n + 2, Ranges::singleton(version))],
            );
            provider.add_dependencies(
                3 * n + 2,
                version,
                [(3 * n + 3, Ranges::singleton(version))],
            );
            provider.add_dependencies(3 * n + 3, version, [(3 * n + 4, Ranges::full())]);
        }
    }
    for version in 1..branching {
        provider.add_dependencies(
            3 * depth + 1,
            version,
            [(constrained, Ranges::higher_than(incompatible))],
        );
    }
    provider
}
