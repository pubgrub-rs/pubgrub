// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Versions following the semantic versioning scheme of major.minor.patch.
//!
//! This module provides traits and types to create and compare versions.

/// Versions have a minimal version (a "0" version)
/// and are ordered such that every version has a next one.
pub trait Version {
    /// Returns the lowest version.
    fn lowest() -> Self;
    /// Returns the next version, the smallest strictly higher version.
    fn bump(self) -> Self;
}

/// Type for semantic versions: major.minor.patch.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SemanticVersion {
    major: usize,
    minor: usize,
    patch: usize,
}

// Constructors
impl SemanticVersion {
    /// Create a version with "major", "minor" and "patch" values.
    /// `version = major.minor.patch`
    pub fn new(major: usize, minor: usize, patch: usize) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Version 0.0.0.
    pub fn zero() -> Self {
        Self::new(0, 0, 0)
    }

    /// Version 1.0.0.
    pub fn one() -> Self {
        Self::new(1, 0, 0)
    }

    /// Version 2.0.0.
    pub fn two() -> Self {
        Self::new(2, 0, 0)
    }
}

// Convert a tuple (major, minor, patch) into a version.
impl From<(usize, usize, usize)> for SemanticVersion {
    fn from(tuple: (usize, usize, usize)) -> Self {
        let (major, minor, patch) = tuple;
        Self::new(major, minor, patch)
    }
}

// Convert a version into a tuple (major, minor, patch).
impl Into<(usize, usize, usize)> for SemanticVersion {
    fn into(self) -> (usize, usize, usize) {
        (self.major, self.minor, self.patch)
    }
}

// Bump versions.
impl SemanticVersion {
    /// Bump the patch number of a version.
    pub fn bump_patch(self) -> Self {
        Self::new(self.major, self.minor, self.patch + 1)
    }

    /// Bump the minor number of a version.
    pub fn bump_minor(self) -> Self {
        Self::new(self.major, self.minor + 1, self.patch)
    }

    /// Bump the major number of a version.
    pub fn bump_major(self) -> Self {
        Self::new(self.major + 1, self.minor, self.patch)
    }
}

// Implement Version for SemanticVersion.
impl Version for SemanticVersion {
    fn lowest() -> Self {
        Self::zero()
    }
    fn bump(self) -> Self {
        self.bump_patch()
    }
}

/// Simplest versions possible, just a positive number.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct NumberVersion(pub usize);

impl Version for NumberVersion {
    fn lowest() -> Self {
        Self(0)
    }
    fn bump(self) -> Self {
        Self(self.0 + 1)
    }
}
