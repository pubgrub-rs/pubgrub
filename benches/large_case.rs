// SPDX-License-Identifier: MPL-2.0
use std::time::Duration;

extern crate criterion;
use self::criterion::*;

use pubgrub::bounded_range::BoundedRange;
use pubgrub::discrete_range::DiscreteRange;
use pubgrub::package::Package;
use pubgrub::solver::{resolve, OfflineDependencyProvider};
use pubgrub::version::{NumberVersion, SemanticVersion, Version};
use pubgrub::version_set::VersionSet;
use serde::de::Deserialize;
use std::hash::Hash;

fn bench<'a, P: Package + Deserialize<'a>, V: VersionSet + Deserialize<'a>>(
    b: &mut Bencher,
    case: &'a str,
) where
    <V as VersionSet>::V: Deserialize<'a>,
{
    let dependency_provider: OfflineDependencyProvider<P, V> = ron::de::from_str(&case).unwrap();

    b.iter(|| {
        for p in dependency_provider.packages() {
            for n in dependency_provider.versions(p).unwrap() {
                let _ = resolve(&dependency_provider, p.clone(), n.clone());
            }
        }
    });
}

fn bench_nested(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_cases");
    group.measurement_time(Duration::from_secs(20));

    for case in std::fs::read_dir("test-examples").unwrap() {
        let case = case.unwrap().path();
        let name = case.file_name().unwrap().to_string_lossy();
        let data = std::fs::read_to_string(&case).unwrap();
        if name.ends_with("u16_discrete_NumberVersion.ron") {
            group.bench_function(name, |b| {
                bench::<u16, DiscreteRange<NumberVersion>>(b, &data);
            });
        } else if name.ends_with("u16_bounded_NumberVersion.ron") {
            group.bench_function(name, |b| {
                bench::<u16, BoundedRange<NumberVersion>>(b, &data);
            });
        } else if name.ends_with("str_SemanticVersion.ron") {
            group.bench_function(name, |b| {
                bench::<&str, DiscreteRange<SemanticVersion>>(b, &data);
            });
        }
    }

    group.finish();
}

criterion_group!(benches, bench_nested);
criterion_main!(benches);
