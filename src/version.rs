// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Traits and implementations to create and compare versions.

use std::fmt::{self, Debug, Display};

/// Versions have a minimal version (a "0" version)
/// and are ordered such that every version has a next one.
pub trait Version: Clone + Ord + Debug + Display {
    /// Returns the lowest version.
    fn lowest() -> Self;
    /// Returns the next version, the smallest strictly higher version.
    fn bump(&self) -> Self;
}

/// Type for semantic versions: major.minor.patch.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SemanticVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

// Constructors
impl SemanticVersion {
    /// Create a version with "major", "minor" and "patch" values.
    /// `version = major.minor.patch`
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
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
impl From<(u32, u32, u32)> for SemanticVersion {
    fn from(tuple: (u32, u32, u32)) -> Self {
        let (major, minor, patch) = tuple;
        Self::new(major, minor, patch)
    }
}

// Convert a version into a tuple (major, minor, patch).
impl Into<(u32, u32, u32)> for SemanticVersion {
    fn into(self) -> (u32, u32, u32) {
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

impl Display for SemanticVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// Implement Version for SemanticVersion.
impl Version for SemanticVersion {
    fn lowest() -> Self {
        Self::zero()
    }
    fn bump(&self) -> Self {
        self.bump_patch()
    }
}

/// Simplest versions possible, just a positive number.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct NumberVersion(pub usize);

// Convert an usize into a version.
impl From<usize> for NumberVersion {
    fn from(v: usize) -> Self {
        Self(v)
    }
}

// Convert a version into an usize.
impl Into<usize> for NumberVersion {
    fn into(self) -> usize {
        self.0
    }
}

impl Display for NumberVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Version for NumberVersion {
    fn lowest() -> Self {
        Self(0)
    }
    fn bump(&self) -> Self {
        Self(self.0 + 1)
    }
}
