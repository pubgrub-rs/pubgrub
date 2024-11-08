// SPDX-License-Identifier: MPL-2.0

//! This bench monitors the performance of backtracking and term intersection.
//!
//! Dependencies are constructed in a way that all versions need to be tested before finding a solution.

use criterion::*;
use pubgrub::OfflineDependencyProvider;
use version_ranges::Ranges;

/// This benchmark is a simplified reproduction of one of the patterns found in the `solana-*` crates from Cargo:
/// * `solana-archiver-lib v1.1.12` depends on many layers of other solana crates with req `>= 1.1.12`.
/// * each version `1.x.y` higher than `1.5.15` of a solana crate depends on other solana crates with req `= 1.x.y`.
/// * `solana-crate-features` depends on `cc` with the `num_cpus` feature, which doesn't exist in recent versions of `cc`.
fn backtracking_singletons(c: &mut Criterion, package_count: u32, version_count: u32) {
    let mut dependency_provider = OfflineDependencyProvider::<u32, Ranges<u32>>::new();

    dependency_provider.add_dependencies(0u32, 0u32, [(1u32, Ranges::full())]);
    dependency_provider.add_dependencies(1u32, 0u32, []);

    for n in 1..package_count {
        for v in 1..version_count {
            dependency_provider.add_dependencies(n, v, [(n + 1, Ranges::singleton(v))]);
        }
    }

    c.bench_function("backtracking_singletons", |b| {
        b.iter(|| {
            let _ = pubgrub::resolve(&dependency_provider, 0u32, 0u32);
        })
    });
}

/// This benchmark is a simplified reproduction of one of the patterns found in the `solana-*` crates from Cargo:
/// * `solana-archiver-lib v1.1.12` depends on many layers of other solana crates with req `>= 1.1.12`.
/// * `solana-archiver-lib v1.1.12` also depends on `ed25519-dalek v1.0.0-pre.3`.
/// * each version `1.x.y` higher than `1.5.15` of a solana crate depends on other solana crates with req `= 1.x.y`.
/// * `solana-crate-features >= 1.2.17` depends on `ed25519-dalek v1.0.0-pre.4` or a higher incompatible version.
fn backtracking_disjoint_versions(c: &mut Criterion, package_count: u32, version_count: u32) {
    let mut dependency_provider = OfflineDependencyProvider::<u32, Ranges<u32>>::new();

    let root_deps = [(1u32, Ranges::full()), (u32::MAX, Ranges::singleton(0u32))];
    dependency_provider.add_dependencies(0u32, 0u32, root_deps);

    dependency_provider.add_dependencies(1u32, 0u32, []);

    for n in 1..package_count {
        for v in 1..version_count {
            dependency_provider.add_dependencies(n, v, [(n + 1, Ranges::singleton(v))]);
        }
    }
    for v in 1..version_count {
        dependency_provider.add_dependencies(package_count, v, [(u32::MAX, Ranges::singleton(v))]);
    }

    for v in 0..version_count {
        dependency_provider.add_dependencies(u32::MAX, v, []);
    }

    c.bench_function("backtracking_disjoint_versions", |b| {
        b.iter(|| {
            let _ = pubgrub::resolve(&dependency_provider, 0u32, 0u32);
        })
    });
}

/// This benchmark is a simplified reproduction of one of the patterns found in the `solana-*` crates from Cargo:
/// * `solana-archiver-lib v1.1.12` depends on many layers of other solana crates with req `>= 1.1.12`.
/// * each version `1.x.y` lower than `1.5.14` of a solana crate depends on other solana crates with req `>= 1.x.y`.
/// * `solana-crate-features` depends on `cc` with the `num_cpus` feature, which doesn't exist in recent versions of `cc`.
fn backtracking_ranges(c: &mut Criterion, package_count: u32, version_count: u32) {
    let mut dependency_provider = OfflineDependencyProvider::<u32, Ranges<u32>>::new();

    dependency_provider.add_dependencies(0u32, 0u32, [(1u32, Ranges::full())]);
    dependency_provider.add_dependencies(1u32, 0u32, []);

    for n in 1..package_count {
        for v in 1..version_count {
            let r = Ranges::higher_than(version_count - v);
            dependency_provider.add_dependencies(n, v, [(n + 1, r)]);
        }
    }

    c.bench_function("backtracking_ranges", |b| {
        b.iter(|| {
            let _ = pubgrub::resolve(&dependency_provider, 0u32, 0u32);
        })
    });
}

fn bench_group(c: &mut Criterion) {
    backtracking_singletons(c, 100, 500);
    backtracking_disjoint_versions(c, 300, 200);
    backtracking_ranges(c, 5, 200);
}

criterion_group!(benches, bench_group);
criterion_main!(benches);
