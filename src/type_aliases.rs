// SPDX-License-Identifier: MPL-2.0

//! Publicly exported type aliases.

use crate::{internal::incompatibility::IncompId, solver::DependencyProvider};

/// Map implementation used by the library.
pub type Map<K, V> = rustc_hash::FxHashMap<K, V>;

/// Set implementation used by the library.
pub type Set<V> = rustc_hash::FxHashSet<V>;

/// Concrete dependencies picked by the library during [resolve](crate::solver::resolve).
pub type SelectedDependencies<DP> =
    Map<<DP as DependencyProvider>::P, <DP as DependencyProvider>::V>;

pub(crate) type IncompDpId<DP> = IncompId<
    <DP as DependencyProvider>::P,
    <DP as DependencyProvider>::VS,
    <DP as DependencyProvider>::M,
>;
