// SPDX-License-Identifier: MPL-2.0

use std::cmp::Reverse;
use std::convert::Infallible;
use std::fmt::Display;
use std::num::NonZeroU64;

use pubgrub::{
    helpers::PackageVersionWrapper, resolve, DefaultStringReporter, Dependencies,
    DependencyProvider, Map, PackageArena, PackageId, PubGrubError, Reporter, VersionIndex,
    VersionSet,
};

struct Provider {
    version_counts: Map<&'static str, u64>,
}

impl DependencyProvider for Provider {
    type P = PackageVersionWrapper<&'static str>;
    type M = &'static str;
    type Priority = Reverse<u64>;
    type Err = Infallible;

    fn prioritize(
        &mut self,
        _: PackageId,
        set: VersionSet,
        _: &PackageArena<Self::P>,
    ) -> Self::Priority {
        Reverse(set.count() as u64)
    }

    fn choose_version(
        &mut self,
        _: PackageId,
        set: VersionSet,
        _: &PackageArena<Self::P>,
    ) -> Result<Option<VersionIndex>, Self::Err> {
        Ok(set.last())
    }

    fn get_dependencies(
        &mut self,
        package: PackageId,
        version_index: VersionIndex,
        package_store: &mut PackageArena<Self::P>,
    ) -> Result<Dependencies<Self::M>, Self::Err> {
        let mut dep_map = Map::default();

        let add_dep = |store: &mut PackageArena<Self::P>, dep_map: &mut Map<_, _>, (d, vs)| {
            dep_map.insert(store.insert(d), vs);
        };

        let add_range_dep =
            |store: &mut PackageArena<Self::P>, dep_map: &mut Map<_, _>, (dn, r, vc)| {
                add_dep(store, dep_map, PackageVersionWrapper::new_dep(dn, r, vc));
            };

        let add_singleton_dep =
            |store: &mut PackageArena<Self::P>, dep_map: &mut Map<_, _>, (dn, v, vc)| {
                let (d, vs) = PackageVersionWrapper::new_singleton_dep(dn, v, vc);
                add_dep(store, dep_map, (d, vs));
            };

        let wrapper = package_store.pkg(package).unwrap();
        let inner = wrapper.inner(version_index).map(|(&p, v)| (p, v));

        if let Some((d, vs)) = wrapper.dependency(version_index) {
            add_dep(package_store, &mut dep_map, (d, vs));
        }

        if let Some((name, true_version_index)) = inner {
            match (name, true_version_index) {
                ("root", 0) => {
                    {
                        let dn = "dep1";
                        match self.version_counts.get(dn) {
                            Some(&vc) => {
                                add_singleton_dep(package_store, &mut dep_map, (dn, 1, vc))
                            }
                            None => return Ok(Dependencies::Unavailable("unavailable")),
                        }
                    }
                    {
                        let dn = "dep2";
                        match self.version_counts.get(dn) {
                            Some(&vc) => {
                                add_range_dep(package_store, &mut dep_map, (dn, 0..64, vc))
                            }
                            None => return Ok(Dependencies::Unavailable("unavailable")),
                        }
                    }
                }
                ("dep1", 1) => {
                    let dn = "many";
                    match self.version_counts.get(dn) {
                        Some(&vc) => add_range_dep(package_store, &mut dep_map, (dn, 0..10000, vc)),
                        None => return Ok(Dependencies::Unavailable("unavailable")),
                    }
                }
                ("dep2", 1) | ("many", _) => (),
                _ => return Ok(Dependencies::Unavailable("unavailable")),
            }
        };

        Ok(Dependencies::Available(dep_map))
    }

    fn package_version_display<'a>(
        &'a self,
        package: &'a Self::P,
        version_index: VersionIndex,
    ) -> impl Display + 'a {
        match package.inner(version_index) {
            Some((&pkg, true_version_index)) => format!("{pkg} @ {true_version_index}"),
            None => format!("{package} @ {}", version_index.get()),
        }
    }

    fn package_version_set_display<'a>(
        &'a self,
        package: &'a Self::P,
        version_set: VersionSet,
    ) -> impl Display + 'a {
        let version_indices = version_set
            .iter()
            .map(|version_index| match package.inner(version_index) {
                Some((_, version_index)) => version_index,
                None => version_index.get() as u64,
            })
            .collect::<Vec<_>>();

        match package.inner_pkg() {
            Some(p) => format!("{p} @ {version_indices:?}"),
            _ => format!("{package} @ {version_indices:?}"),
        }
    }
}

fn main() {
    let (root_pkg, root_version_index) =
        PackageVersionWrapper::new_pkg("root", 0, NonZeroU64::new(1).unwrap());

    let mut provider = Provider {
        version_counts: Map::from_iter([("root", 1), ("dep1", 63), ("dep2", 64), ("many", 10000)]),
    };

    match resolve(&mut provider, root_pkg, root_version_index) {
        Ok(sol) => {
            for (p, &v) in &sol {
                let pv = provider.package_version_display(p, v);
                println!("{pv}");
            }
        }
        Err(PubGrubError::NoSolution(mut error)) => {
            error.derivation_tree.collapse_no_versions();
            eprintln!("{}", DefaultStringReporter::report(&error, &provider));
            std::process::exit(1);
        }
        Err(err) => panic!("{:?}", err),
    }
}
