// SPDX-License-Identifier: MPL-2.0

use crates_index::Index;
use crates_index::Version;
use pubgrub::error::PubGrubError;
use pubgrub::type_aliases::Map;
use pubgrub::version::SemanticVersion as V;
use semver;
use std::collections::BTreeMap;
use std::convert::TryFrom;

mod index;
use index::{ConvertCrateError, CrateDeps};

fn main() {
    let index = Index::new("temp/_index");
    if !index.exists() {
        index.retrieve().expect("Could not fetch crates.io index");
    }

    // output_all_crates(&index);
    // output_all_requirements(&index);
    // convert_index(&index).unwrap();

    // for crate_ in index.crates() {
    //     for v in crate_.versions().iter() {
    //         // println!("{} @ {}", v.name(), v.version());
    //         // print_if_not_yanked(v);
    //         // print_dep_target(v);
    //         // print_dep_requirements(v);
    //         print_dep_default(v);
    //     }
    // }

    solve_all_index().unwrap();
}

fn output_all_crates(index: &Index) {
    for crate_ in index.crates() {
        println!("{}", crate_.name());
    }
}

fn output_all_requirements(index: &Index) {
    let mut reqs = std::collections::BTreeSet::new();
    for crate_ in index.crates() {
        for v in crate_.versions().iter() {
            for dep in v.dependencies().iter() {
                reqs.insert(dep.requirement().to_string());
            }
        }
    }
    reqs.iter().for_each(|r| println!("{}", r));
}

fn convert_index(index: &Index) -> Result<index::Index, ConvertCrateError> {
    let mut crates: Map<String, BTreeMap<V, CrateDeps>> = Map::default();
    for crate_ in index.crates() {
        for v in crate_.versions().iter() {
            // Convert semver into SemanticVersion (V).
            let semver_v = semver::Version::parse(v.version())
                .map_err(ConvertCrateError::VersionParseError)?;
            // Skip pre-release versions.
            if semver_v.is_prerelease() {
                eprintln!("pre-release: {}@{}", v.name(), v.version());
                continue;
            }
            let sem_ver = V::new(
                semver_v.major as u32,
                semver_v.minor as u32,
                semver_v.patch as u32,
            );

            let crate_deps = match CrateDeps::try_from(v) {
                Ok(cd) => cd,
                // Just skip versions that are not valid.
                Err(err) => {
                    eprintln!("{:?}", err);
                    continue;
                }
            };
            let v_entry = crates
                .entry(v.name().to_string())
                .or_insert(BTreeMap::new());
            v_entry.insert(sem_ver, crate_deps);
        }
    }
    println!(
        "Valid versions: {}",
        crates.values().map(|v| v.len()).sum::<usize>()
    );

    let registry = index::Index { crates };
    let pretty_config = ron::ser::PrettyConfig::new()
        .with_depth_limit(6)
        .with_indentor("  ".to_string());
    let index_str = ron::ser::to_string_pretty(&registry, pretty_config).expect("woops ron");
    std::fs::write("temp/index.ron", &index_str).expect("woops ron write");

    Ok(registry)
}

fn solve_all_index() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Loading ron file");
    let index_str = std::fs::read_to_string("temp/index.ron")?;
    let deps_index: index::Index = ron::de::from_str(&index_str)?;
    let mut deps_provider = index::CrateProvider::new(&deps_index);
    eprintln!("Solving dependencies");
    let version_count: usize = deps_provider.index.crates.values().map(|p| p.len()).sum();
    let mut solved_count = 0;
    let pb = indicatif::ProgressBar::new(version_count as u64);
    let to_root_id = |p| format!("{}:", p);
    for (package, versions) in deps_provider.index.crates.iter() {
        for v in versions.keys() {
            let mut start_time = deps_provider.start_time.borrow_mut();
            *start_time = std::time::Instant::now();
            drop(start_time);
            match pubgrub::solver::resolve(&deps_provider, to_root_id(package), v.clone()) {
                Ok(_) => solved_count += 1,
                // Err(_) => eprintln!("{} @ {}", package, v),
                Err(PubGrubError::ErrorInShouldCancel(_)) => {
                    eprintln!("\nshould cancel {} @ {}\n", package, v);
                }
                Err(_) => {}
            }
            pb.inc(1);
        }
    }
    pb.finish();
    println!("Found solutions for {} / {}", solved_count, version_count);
    Ok(())
}

fn print_if_not_yanked(v: &Version) {
    if !v.is_yanked() {
        println!("{} @ {}", v.name(), v.version());
    }
}

fn print_dep_target(v: &Version) {
    v.dependencies().first().map(|dep| {
        println!(
            "{} @ {} dep {} target {:?}",
            v.name(),
            v.version(),
            dep.crate_name(),
            dep.target()
        );
    });
}

fn print_dep_requirements(v: &Version) {
    v.dependencies().first().map(|dep| {
        println!(
            "{} @ {} dep {} requirement {}",
            v.name(),
            v.version(),
            dep.crate_name(),
            dep.requirement(),
        );
    });
}

fn print_dep_default(v: &Version) {
    v.dependencies().first().map(|dep| {
        println!(
            "{} @ {} dep {} has_default: {}",
            v.name(),
            v.version(),
            dep.crate_name(),
            dep.has_default_features(),
        );
    });
}
