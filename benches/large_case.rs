// SPDX-License-Identifier: MPL-2.0

use criterion::*;
use pubgrub::package::Package;
use pubgrub::range::Range;
use pubgrub::solver::{OfflineSolver, Solver};
use pubgrub::version::{SemanticVersion, Version};
use pubv2::type_aliases::Map;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::hash::Hash;
use std::time::Duration;

type DependencyConstraints<P, V> = Map<P, Range<V>>;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(transparent)]
pub struct OfflineDependencyProvider<P: Package, V: Version> {
    dependencies: Map<P, BTreeMap<V, DependencyConstraints<P, V>>>,
}

fn bench<'a, P: Package + Deserialize<'a>, V: Version + Hash + Deserialize<'a>>(
    b: &mut Bencher,
    case: &'a str,
) {
    let dependency_provider: OfflineDependencyProvider<P, V> = ron::de::from_str(&case).unwrap();

    // Conversion from pubgrub v0.2 to pubgrub v0.1
    let mut offline_solver: OfflineSolver<P, V> = OfflineSolver::new();
    for (p, v_map) in dependency_provider.dependencies.iter() {
        for (v, deps) in v_map.iter() {
            offline_solver.add_dependencies(
                p.clone(),
                v.clone(),
                deps.iter().map(|(dp, dr)| (dp.clone(), dr.clone())),
            )
        }
    }

    b.iter(|| {
        for (p, v_map) in dependency_provider.dependencies.iter() {
            for (v, _) in v_map.iter() {
                let _ = offline_solver.run(p.clone(), v.clone());
            }
        }
    });
}

fn bench_nested(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_cases");
    group.measurement_time(Duration::from_secs(20));

    println!("Bench started!");
    for case in std::fs::read_dir("test-examples").unwrap() {
        let case = case.unwrap().path();
        let name = case.file_name().unwrap().to_string_lossy();
        let data = std::fs::read_to_string(&case).unwrap();
        if name.ends_with("str_SemanticVersion.ron") {
            group.bench_function(name, |b| {
                bench::<&str, SemanticVersion>(b, &data);
            });
        }
    }

    group.finish();
}

criterion_group!(benches, bench_nested);
criterion_main!(benches);
