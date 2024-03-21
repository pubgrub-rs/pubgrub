// SPDX-License-Identifier: MPL-2.0

//! Publicly exported type aliases.

use crate::{
    internal::incompatibility::IncompId, solver::DependencyProvider, version_set::VersionSet,
};

/// Map implementation used by the library.
pub type Map<K, V> = rustc_hash::FxHashMap<K, V>;

/// Set implementation used by the library.
pub type Set<V> = rustc_hash::FxHashSet<V>;

/// The version type associated with the version set associated with the dependency provider.
pub type V<DP> = <<DP as DependencyProvider>::VS as VersionSet>::V;

/// Concrete dependencies picked by the library during [resolve](crate::solver::resolve)
/// from [DependencyConstraints].
pub type SelectedDependencies<DP> = Map<<DP as DependencyProvider>::P, V<DP>>;

/// Holds information about all possible versions a given package can accept.
/// There is a difference in semantics between an empty map
/// inside [DependencyConstraints] and [Dependencies::Unknown](crate::solver::Dependencies::Unknown):
/// the former means the package has no dependency and it is a known fact,
/// while the latter means they could not be fetched by the [DependencyProvider].
pub type DependencyConstraints<P, VS> = Map<P, VS>;

pub(crate) type IncompDpId<DP> =
    IncompId<<DP as DependencyProvider>::P, <DP as DependencyProvider>::VS>;
