use std::fmt::Debug;
use std::hash::Hash;

use crate::FxIndexSet;

/// Type for identifying packages.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct PackageId(pub(crate) u32);

impl PackageId {
    /// Get the inner value.
    #[inline]
    pub fn get(self) -> u32 {
        self.0
    }
}

/// Package arena
#[derive(Debug, Clone)]
pub struct PackageArena<P>(FxIndexSet<P>);

impl<P: Eq + Hash> PackageArena<P> {
    /// Create an empty package arena.
    pub(crate) fn new() -> Self {
        Self(Default::default())
    }

    /// Iterate over packages.
    pub(crate) fn into_iter(self) -> indexmap::set::IntoIter<P> {
        self.0.into_iter()
    }

    /// Get the package identifier of a package.
    pub fn get(&self, package: &P) -> Option<PackageId> {
        Some(PackageId(self.0.get_index_of(package)? as u32))
    }

    /// Insert a package in the arena and get its identifier.
    pub fn insert(&mut self, package: P) -> PackageId {
        PackageId(self.0.insert_full(package).0 as u32)
    }

    /// Get a package from its identifier.
    pub fn pkg(&self, package_id: PackageId) -> Option<&P> {
        self.0.get_index(package_id.get() as usize)
    }
}
