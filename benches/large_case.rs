// SPDX-License-Identifier: MPL-2.0
use std::time::Duration;

extern crate criterion;
use self::criterion::*;

use pubgrub::solver::{resolve, OfflineDependencyProvider};
use pubgrub::version::NumberVersion;

fn bench_nested(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_cases");
    group.measurement_time(Duration::from_secs(20));

    for case in std::fs::read_dir("test-examples").unwrap() {
        let case = case.unwrap().path();

        group.bench_function(
            format!("{}", case.file_name().unwrap().to_string_lossy()),
            |b| {
                let s = std::fs::read_to_string(&case).unwrap();
                let dependency_provider: OfflineDependencyProvider<u16, NumberVersion> =
                    ron::de::from_str(&s).unwrap();

                b.iter(|| {
                    for &n in dependency_provider.versions(&0).unwrap() {
                        let _ = resolve(&dependency_provider, 0, n);
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_nested);
criterion_main!(benches);
