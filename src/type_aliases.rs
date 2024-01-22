// SPDX-License-Identifier: MPL-2.0

//! Publicly exported type aliases.

#![allow(warnings)]

use std::{
    borrow::Borrow,
    hash::{BuildHasher, BuildHasherDefault, Hash},
};

// /// Map implementation used by the library.
// pub type Map<K, V> = rustc_hash::FxHashMap<K, V>;

// /// Set implementation used by the library.
// pub type Set<V> = rustc_hash::FxHashSet<V>;

pub type Map<K, V> = MapI<K, V, BuildHasherDefault<rustc_hash::FxHasher>>;

/// Concrete dependencies picked by the library during [resolve](crate::solver::resolve)
/// from [DependencyConstraints].
pub type SelectedDependencies<P, V> = Map<P, V>;

/// Holds information about all possible versions a given package can accept.
/// There is a difference in semantics between an empty map
/// inside [DependencyConstraints] and [Dependencies::Unknown](crate::solver::Dependencies::Unknown):
/// the former means the package has no dependency and it is a known fact,
/// while the latter means they could not be fetched by the [DependencyProvider](crate::solver::DependencyProvider).
pub type DependencyConstraints<P, VS> = Map<P, VS>;

#[derive(Debug, Clone)]
pub struct MapI<K, V, S> {
    map: std::collections::HashMap<K, V, S>,
}

impl<K, V, S> MapI<K, V, S> {
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> MapI<K, V, S> {
        MapI {
            map: std::collections::HashMap::with_capacity_and_hasher(capacity, hasher),
        }
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, K, V> {
        self.map.iter()
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.map.keys()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn retain(&mut self, f: impl FnMut(&K, &mut V) -> bool) {
        self.map.retain(f)
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> MapI<K, V, S> {
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        self.map.insert(k, v)
    }

    pub fn remove(&mut self, k: &K) -> Option<V> {
        self.map.remove(k)
    }

    pub fn contains_key(&self, k: &K) -> bool {
        self.map.contains_key(k)
    }

    pub fn get(&self, k: &K) -> Option<&V> {
        self.map.get(k)
    }

    pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        self.map.get_mut(k)
    }

    pub fn entry(&mut self, k: K) -> std::collections::hash_map::Entry<'_, K, V> {
        self.map.entry(k)
    }
}

impl<K, V, S: Default> Default for MapI<K, V, S> {
    fn default() -> MapI<K, V, S> {
        MapI {
            map: std::collections::HashMap::default(),
        }
    }
}

impl<K: Eq + Hash, V: PartialEq, S: BuildHasher> PartialEq for MapI<K, V, S> {
    fn eq(&self, other: &MapI<K, V, S>) -> bool {
        self.map.eq(&other.map)
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> Extend<(K, V)> for MapI<K, V, S> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, it: T) {
        self.map.extend(it)
    }
}

impl<'a, K: 'a + Hash + Eq + Clone, V: 'a + Clone, S: BuildHasher> Extend<(&'a K, &'a V)>
    for MapI<K, V, S>
{
    fn extend<T: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, it: T) {
        self.map
            .extend(it.into_iter().map(|(k, v)| (k.clone(), v.clone())))
    }
}

pub struct Set<V> {
    set: rustc_hash::FxHashSet<V>,
}

impl<V> Set<V> {
    pub fn iter(&self) -> impl Iterator<Item = &V> {
        self.set.iter()
    }
}

impl<V: Hash + Eq> Set<V> {
    pub fn insert(&mut self, v: V) -> bool {
        self.set.insert(v)
    }

    pub fn contains(&self, v: &V) -> bool {
        self.set.contains(v)
    }

    pub fn get(&self, v: &V) -> Option<&V> {
        self.set.get(v)
    }
}

impl<V> Default for Set<V> {
    fn default() -> Set<V> {
        Set {
            set: rustc_hash::FxHashSet::default(),
        }
    }
}

impl<V: Hash + Eq> Extend<V> for Set<V> {
    fn extend<T: IntoIterator<Item = V>>(&mut self, it: T) {
        self.set.extend(it)
    }
}

impl<'a, V: 'a + Hash + Eq + Clone> Extend<&'a V> for Set<V> {
    fn extend<T: IntoIterator<Item = &'a V>>(&mut self, it: T) {
        self.set.extend(it.into_iter().cloned())
    }
}

impl<K: Hash + Eq, V, S: BuildHasher + Default> FromIterator<(K, V)> for MapI<K, V, S> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(it: I) -> MapI<K, V, S> {
        MapI {
            map: FromIterator::from_iter(it),
        }
    }
}

impl<K: Eq + Hash + Borrow<Q>, Q: Eq + Hash + ?Sized, V, S: BuildHasher> std::ops::Index<&Q>
    for MapI<K, V, S>
{
    type Output = V;

    fn index(&self, key: &Q) -> &V {
        self.map.index(key)
    }
}

impl<K, V, S> IntoIterator for MapI<K, V, S> {
    type Item = (K, V);
    type IntoIter = std::collections::hash_map::IntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl<'a, K, V, S> IntoIterator for &'a MapI<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = std::collections::hash_map::Iter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        (&self.map).into_iter()
    }
}
