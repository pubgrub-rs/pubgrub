// SPDX-License-Identifier: MPL-2.0
use std::time::Duration;

extern crate criterion;
use self::criterion::*;

use pubgrub::Package;
use pubgrub::Range;
use pubgrub::SemanticVersion;
use pubgrub::VersionSet;
use pubgrub::{resolve, OfflineDependencyProvider};
use serde::de::Deserialize;

fn bench<'a, P: Package + Deserialize<'a>, VS: VersionSet + Deserialize<'a>>(
    b: &mut Bencher,
    case: &'a str,
) where
    <VS as VersionSet>::V: Deserialize<'a>,
{
    let dependency_provider: OfflineDependencyProvider<P, VS> = ron::de::from_str(case).unwrap();

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
        if name.ends_with("u16_NumberVersion.ron") || name.ends_with("u16_u32.ron") {
            group.bench_function(name, |b| {
                bench::<u16, Range<u32>>(b, &data);
            });
        } else if name.ends_with("str_SemanticVersion.ron") {
            group.bench_function(name, |b| {
                bench::<&str, Range<SemanticVersion>>(b, &data);
            });
        }
    }

    group.finish();
}

criterion_group!(benches, bench_nested);
criterion_main!(benches);
