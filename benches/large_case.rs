// SPDX-License-Identifier: MPL-2.0
use std::time::Duration;

extern crate criterion;
use self::criterion::*;

use pubgrub::package::Package;
use pubgrub::solver::{resolve, DependencyProvider, OfflineDependencyProvider};
use pubgrub::version::{NumberVersion, SemanticVersion, Version};
use serde::de::Deserialize;
use std::hash::Hash;

fn bench<'a, P: Package + Deserialize<'a>, V: Version + Hash + Deserialize<'a>>(
    b: &mut Bencher,
    case: &'a str,
    p: P,
) {
    let dependency_provider: OfflineDependencyProvider<P, V> = ron::de::from_str(&case).unwrap();
    let all_versions = dependency_provider.list_available_versions(&p).unwrap();

    b.iter(|| {
        for n in &all_versions {
            let _ = resolve(&dependency_provider, p.clone(), n.clone());
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
        if name.ends_with("u16_NumberVersion.ron") {
            group.bench_function(name, |b| {
                bench::<u16, NumberVersion>(b, &data, 0);
            });
        } else if name.ends_with("str_SemanticVersion.ron") {
            group.bench_function(name, |b| {
                bench::<&str, SemanticVersion>(b, &data, "root");
            });
        }
    }

    group.finish();
}

criterion_group!(benches, bench_nested);
criterion_main!(benches);
