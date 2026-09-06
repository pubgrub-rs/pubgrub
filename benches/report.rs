// SPDX-License-Identifier: MPL-2.0

//! Measure error-tree operations separately from dependency-provider setup and resolution.

use std::hint::black_box;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use pubgrub::{DefaultStringReporter, OfflineDependencyProvider, PubGrubError, Ranges, Reporter};

fn report(c: &mut Criterion) {
    for depth in [100u32, 2_000] {
        let mut provider = OfflineDependencyProvider::<u32, Ranges<u32>>::new();
        for package in 0..depth {
            provider.add_dependencies(package, 1u32, [(package + 1, Ranges::singleton(1u32))]);
        }
        let Err(PubGrubError::NoSolution(tree)) = pubgrub::resolve(&provider, 0, 1u32) else {
            panic!("the dependency chain must be unsatisfiable");
        };

        let mut group = c.benchmark_group(format!("report/{depth}"));
        group.bench_function("resolve", |b| {
            b.iter(|| black_box(pubgrub::resolve(&provider, 0, 1u32)))
        });
        group.bench_function("format", |b| {
            b.iter(|| black_box(DefaultStringReporter::report(&tree)))
        });
        group.bench_function("packages", |b| b.iter(|| black_box(tree.packages())));
        group.bench_function("clone", |b| b.iter(|| black_box(tree.clone())));
        group.bench_function("collapse", |b| {
            b.iter_batched(
                || tree.clone(),
                |mut tree| {
                    tree.collapse_no_versions();
                    black_box(tree)
                },
                BatchSize::SmallInput,
            )
        });
        group.finish();
    }
}

criterion_group!(benches, report);
criterion_main!(benches);
