// SPDX-License-Identifier: MPL-2.0

//! Publicly exported type aliases.

use crate::{DependencyProvider, PackageId, VersionIndex, VersionSet};

/// Map implementation used by the library.
pub type Map<K, V> = rustc_hash::FxHashMap<K, V>;

/// Set implementation used by the library.
pub type Set<V> = rustc_hash::FxHashSet<V>;

/// IndexMap implementation used by the library.
pub type FxIndexMap<K, V> = indexmap::IndexMap<K, V, rustc_hash::FxBuildHasher>;

/// IndexSet implementation used by the library.
pub type FxIndexSet<V> = indexmap::IndexSet<V, rustc_hash::FxBuildHasher>;

/// Concrete dependencies picked by the library during [resolve](crate::solver::resolve)
/// from [DependencyConstraints].
pub type SelectedDependencies<DP> = Map<<DP as DependencyProvider>::P, VersionIndex>;

/// Holds information about all possible versions a given package can accept.
/// There is a difference in semantics between an empty map
/// inside [DependencyConstraints] and [Dependencies::Unavailable](crate::solver::Dependencies::Unavailable):
/// the former means the package has no dependency and it is a known fact,
/// while the latter means they could not be fetched by the [DependencyProvider].
pub type DependencyConstraints = Map<PackageId, VersionSet>;
