// SPDX-License-Identifier: MPL-2.0

use crates_index::Index;
use crates_index::Version;
use pubgrub::type_aliases::Map;
use pubgrub::version::SemanticVersion as V;
use semver;
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
    convert_index(&index).unwrap();

    // for crate_ in index.crates() {
    //     for v in crate_.versions().iter() {
    //         // println!("{} @ {}", v.name(), v.version());
    //         // print_if_not_yanked(v);
    //         // print_dep_target(v);
    //         // print_dep_requirements(v);
    //         print_dep_default(v);
    //     }
    // }
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
    let mut crates: Map<String, Map<V, CrateDeps>> = Map::default();
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
            let v_entry = crates.entry(v.name().to_string()).or_insert(Map::default());
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
