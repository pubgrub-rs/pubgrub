// SPDX-License-Identifier: MPL-2.0
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::time::Duration;

use criterion::*;
use pubgrub::{Map, OfflineDependencyProvider, Range, VersionRanges};
use serde::de::Deserialize;

fn bench<
    'a,
    P: Debug + Display + Clone + Eq + Hash + Deserialize<'a>,
    R: VersionRanges + Deserialize<'a>,
>(
    b: &mut Bencher,
    case: &'a str,
) where
    R::V: Deserialize<'a>,
{
    let mut dependency_provider: OfflineDependencyProvider<P, R> = ron::de::from_str(case).unwrap();

    let dependencies = dependency_provider
        .packages()
        .map(|p| {
            (
                p.clone(),
                dependency_provider.versions(p).unwrap().cloned().collect(),
            )
        })
        .collect::<Map<_, Vec<_>>>();

    b.iter(|| {
        for (p, versions) in &dependencies {
            for v in versions {
                let _ = dependency_provider.resolve(p.clone(), v.clone());
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
        if name.ends_with("u16_NumberVersion.ron") {
            group.bench_function(name, |b| {
                bench::<u16, Range<u32>>(b, &data);
            });
        }
    }

    group.finish();
}

criterion_group!(benches, bench_nested);
criterion_main!(benches);
