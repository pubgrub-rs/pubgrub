// SPDX-License-Identifier: MPL-2.0

//! Publicly exported type aliases.

#![allow(warnings)]

use std::{
    borrow::Borrow,
    hash::{BuildHasher, BuildHasherDefault, Hash},
};

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
pub struct Map<K, V, S = BuildHasherDefault<rustc_hash::FxHasher>> {
    map: std::collections::HashMap<K, V, S>,
}

impl<K, V, S> Map<K, V, S> {
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Map<K, V, S> {
        Map {
            map: std::collections::HashMap::with_capacity_and_hasher(capacity, hasher),
        }
    }

    pub fn iter(&self) -> MapIter<'_, K, V, S> {
        let keys: Vec<&K> = self.map.keys().collect();
        MapIter {
            map: self,
            order: keys.into_iter(),
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        let keys: Vec<&K> = self.map.keys().collect();
        MapKeys {
            order: keys.into_iter(),
            _v: std::marker::PhantomData::<&V>,
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn retain(&mut self, f: impl FnMut(&K, &mut V) -> bool) {
        self.map.retain(f)
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> Map<K, V, S> {
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

    pub fn get_key_value(&self, k: &K) -> Option<(&K, &V)> {
        self.map.get_key_value(k)
    }

    pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        self.map.get_mut(k)
    }

    pub fn entry(&mut self, k: K) -> std::collections::hash_map::Entry<'_, K, V> {
        self.map.entry(k)
    }
}

impl<K, V, S: Default> Default for Map<K, V, S> {
    fn default() -> Map<K, V, S> {
        Map {
            map: std::collections::HashMap::default(),
        }
    }
}

impl<K: Eq + Hash, V: PartialEq, S: BuildHasher> PartialEq for Map<K, V, S> {
    fn eq(&self, other: &Map<K, V, S>) -> bool {
        self.map.eq(&other.map)
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> Extend<(K, V)> for Map<K, V, S> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, it: T) {
        self.map.extend(it)
    }
}

impl<'a, K: 'a + Hash + Eq + Clone, V: 'a + Clone, S: BuildHasher> Extend<(&'a K, &'a V)>
    for Map<K, V, S>
{
    fn extend<T: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, it: T) {
        self.map
            .extend(it.into_iter().map(|(k, v)| (k.clone(), v.clone())))
    }
}

impl<K: Hash + Eq, V, S: BuildHasher + Default> FromIterator<(K, V)> for Map<K, V, S> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(it: I) -> Map<K, V, S> {
        Map {
            map: FromIterator::from_iter(it),
        }
    }
}

impl<K: Eq + Hash + Borrow<Q>, Q: Eq + Hash + ?Sized, V, S: BuildHasher> std::ops::Index<&Q>
    for Map<K, V, S>
{
    type Output = V;

    fn index(&self, key: &Q) -> &V {
        self.map.index(key)
    }
}

impl<K: Clone + Eq + Hash, V, S: BuildHasher> IntoIterator for Map<K, V, S> {
    type Item = (K, V);
    type IntoIter = MapIntoIter<K, V, S>;

    fn into_iter(self) -> Self::IntoIter {
        let keys: Vec<K> = self.map.keys().map(|k| k.clone()).collect();
        MapIntoIter {
            map: self,
            order: keys.into_iter(),
        }
    }
}

impl<'a, K: Eq + Hash, V, S: BuildHasher> IntoIterator for &'a Map<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = MapIter<'a, K, V, S>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct MapIter<'a, K, V, S = BuildHasherDefault<rustc_hash::FxHasher>> {
    map: &'a Map<K, V, S>,
    order: std::vec::IntoIter<&'a K>,
}

impl<'a, K: Eq + Hash, V, S: BuildHasher> Iterator for MapIter<'a, K, V, S> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        self.order
            .next()
            .map(|k| self.map.get_key_value(k).unwrap())
    }
}

pub struct MapIntoIter<K, V, S = BuildHasherDefault<rustc_hash::FxHasher>> {
    map: Map<K, V, S>,
    order: std::vec::IntoIter<K>,
}

impl<K: Clone + Eq + Hash, V, S: BuildHasher> Iterator for MapIntoIter<K, V, S> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        self.order.next().map(|k| {
            let v = self.map.remove(&k).unwrap();
            (k, v)
        })
    }
}

pub struct MapKeys<'a, K, V> {
    order: std::vec::IntoIter<&'a K>,
    _v: std::marker::PhantomData<&'a V>,
}

impl<'a, K, V> Iterator for MapKeys<'a, K, V> {
    type Item = &'a K;
    fn next(&mut self) -> Option<Self::Item> {
        self.order.next()
    }
}

pub struct Set<V> {
    set: rustc_hash::FxHashSet<V>,
}

impl<V> Set<V> {
    pub fn iter(&self) -> SetIter<V> {
        let keys: Vec<&V> = self.set.iter().collect();
        SetIter {
            order: keys.into_iter(),
        }
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

impl<V> IntoIterator for Set<V> {
    type Item = V;
    type IntoIter = SetIntoIter<V>;

    fn into_iter(self) -> Self::IntoIter {
        let keys: Vec<V> = self.set.into_iter().collect();
        SetIntoIter {
            order: keys.into_iter(),
        }
    }
}

pub struct SetIter<'a, V> {
    order: std::vec::IntoIter<&'a V>,
}

impl<'a, V> Iterator for SetIter<'a, V> {
    type Item = &'a V;
    fn next(&mut self) -> Option<Self::Item> {
        self.order.next()
    }
}

pub struct SetIntoIter<V> {
    order: std::vec::IntoIter<V>,
}

impl<'a, V> Iterator for SetIntoIter<V> {
    type Item = V;
    fn next(&mut self) -> Option<Self::Item> {
        self.order.next()
    }
}
