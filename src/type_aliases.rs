// SPDX-License-Identifier: MPL-2.0

//! Publicly exported type aliases.

/// Map implementation used by the library.
pub type Map<K, V> = rustc_hash::FxHashMap<K, V>;
