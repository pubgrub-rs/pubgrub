// SPDX-License-Identifier: MPL-2.0

use std::cell::RefCell;
use std::fmt::{Debug, Display};
use std::hash::Hash;

use pubgrub::{
    helpers::PackageVersionWrapper, resolve, Dependencies, DependencyConstraints,
    DependencyProvider, Map, OfflineDependencyProvider, PackageArena, PackageId, PubGrubError,
    Ranges, SelectedDependencies, VersionIndex, VersionRanges, VersionSet,
};

type NumVS = Ranges<u32>;

trait RemoteProvider: DependencyProvider<P = PackageVersionWrapper<Self::Pkg>> {
    type Pkg: Debug + Display + Clone + Eq + Hash;
    type R: VersionRanges;

    fn resolve_parameters(
        &self,
        p: Self::Pkg,
        v: impl Into<<Self::R as VersionRanges>::V>,
    ) -> Option<(PackageVersionWrapper<Self::Pkg>, VersionIndex)>;
}

impl<P: Debug + Display + Clone + Eq + Hash, R: VersionRanges> RemoteProvider
    for OfflineDependencyProvider<P, R>
{
    type Pkg = P;
    type R = R;

    fn resolve_parameters(
        &self,
        p: P,
        v: impl Into<<Self::R as VersionRanges>::V>,
    ) -> Option<(PackageVersionWrapper<P>, VersionIndex)> {
        self.resolve_parameters(p, v)
    }
}

// An example implementing caching dependency provider that will
// store queried dependencies in memory and check them before querying more from remote.
struct CachingDependencyProvider<DP: RemoteProvider<R = R>, R: VersionRanges>
where
    DP::P: Debug + Display + Clone + Eq + Hash,
{
    remote_dependencies: DP,
    cached_dependencies: RefCell<Map<PackageId, Map<VersionIndex, DependencyConstraints>>>,
}

impl<DP: RemoteProvider<R = R>, R: VersionRanges> CachingDependencyProvider<DP, R>
where
    DP::P: Debug + Display + Clone + Eq + Hash,
{
    fn new(remote_dependencies_provider: DP) -> Self {
        CachingDependencyProvider {
            remote_dependencies: remote_dependencies_provider,
            cached_dependencies: Default::default(),
        }
    }

    fn resolve(
        &mut self,
        p: <DP as RemoteProvider>::Pkg,
        v: impl Into<R::V>,
    ) -> Result<SelectedDependencies<Self>, PubGrubError<Self>> {
        let Some((p, v)) = self.remote_dependencies.resolve_parameters(p, v) else {
            return Err(PubGrubError::NoRoot);
        };
        resolve(self, p, v)
    }
}

impl<DP: RemoteProvider<R = R>, R: VersionRanges> DependencyProvider
    for CachingDependencyProvider<DP, R>
where
    DP::P: Debug + Display + Clone + Eq + Hash,
    R::V: Clone,
{
    // Cache dependencies if they were already queried
    fn get_dependencies(
        &mut self,
        package_id: PackageId,
        version_index: VersionIndex,
        package_store: &mut PackageArena<Self::P>,
    ) -> Result<Dependencies<DP::M>, DP::Err> {
        let mut cache = self.cached_dependencies.borrow_mut();
        if let Some(deps) = cache
            .get(&package_id)
            .and_then(|vmap| vmap.get(&version_index))
        {
            return Ok(Dependencies::Available(deps.clone()));
        }

        match self
            .remote_dependencies
            .get_dependencies(package_id, version_index, package_store)
        {
            Ok(Dependencies::Available(deps)) => {
                cache
                    .entry(package_id)
                    .or_default()
                    .insert(version_index, deps.clone());
                Ok(Dependencies::Available(deps))
            }

            Ok(Dependencies::Unavailable(reason)) => Ok(Dependencies::Unavailable(reason)),
            error @ Err(_) => error,
        }
    }

    fn choose_version(
        &mut self,
        package_id: PackageId,
        set: VersionSet,
        package_store: &PackageArena<Self::P>,
    ) -> Result<Option<VersionIndex>, DP::Err> {
        self.remote_dependencies
            .choose_version(package_id, set, package_store)
    }

    type Priority = DP::Priority;

    fn prioritize(
        &mut self,
        package_id: PackageId,
        set: VersionSet,
        package_store: &PackageArena<Self::P>,
    ) -> Self::Priority {
        self.remote_dependencies
            .prioritize(package_id, set, package_store)
    }

    type Err = DP::Err;

    type P = DP::P;
    type M = DP::M;

    fn package_version_display<'a>(
        &'a self,
        package: &'a Self::P,
        version_index: VersionIndex,
    ) -> impl Display + 'a {
        self.remote_dependencies
            .package_version_display(package, version_index)
    }

    fn package_version_set_display<'a>(
        &'a self,
        package: &'a Self::P,
        version_set: VersionSet,
    ) -> impl Display + 'a {
        self.remote_dependencies
            .package_version_set_display(package, version_set)
    }
}

fn main() {
    // Simulating remote provider locally.
    let mut remote_dependencies_provider = OfflineDependencyProvider::<&str, NumVS>::new();

    // Add dependencies as needed. Here only root package is added.
    remote_dependencies_provider.add_dependencies("root", 1u32, Vec::new());

    let mut caching_dependencies_provider =
        CachingDependencyProvider::new(remote_dependencies_provider);

    let solution = caching_dependencies_provider.resolve("root", 1u32);
    println!("Solution: {:?}", solution);
}
