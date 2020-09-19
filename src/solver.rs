// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! PubGrub version solving algorithm.
//!
//! It consists in efficiently finding a set of packages and versions
//! that satisfy all the constraints of a given project dependencies.
//! In addition, when that is not possible,
//! PubGrub tries to provide a very human-readable and clear
//! explanation as to why that failed.
//! Below is an example of explanation present in
//! the introductory blog post about PubGrub
//!
//! ```txt
//! Because dropdown >=2.0.0 depends on icons >=2.0.0 and
//!   root depends on icons <2.0.0, dropdown >=2.0.0 is forbidden.
//!
//! And because menu >=1.1.0 depends on dropdown >=2.0.0,
//!   menu >=1.1.0 is forbidden.
//!
//! And because menu <1.1.0 depends on dropdown >=1.0.0 <2.0.0
//!   which depends on intl <4.0.0, every version of menu
//!   requires intl <4.0.0.
//!
//! So, because root depends on both menu >=1.0.0 and intl >=5.0.0,
//!   version solving failed.
//! ```
//!
//! The algorithm is generic and works for any type of dependency system
//! as long as packages (P) and versions (V) implement the following traits:
//!
//! ```ignore
//! P: Clone + Eq + Hash,
//! V: Clone + Ord + Version,
//! ```
//!
//! Where the `Version` trait simply states that there should be
//! a minimal `lowest` version (like 0.0.0 in semantic versions),
//! and that for any version, it is possible to compute
//! what the next version closest to this one is.
//! For semantic versions, `bump` corresponds to an increment of the patch number.
//!
//!
//! ## API
//!
//! ```ignore
//! solution = solver.run(package, version)?;
//! ```
//!
//! Where `solver` provides the list of available packages and versions,
//! as well as the dependencies of every available package
//! by implementing the `Solver` trait.
//! The call to `run` for a given package at a given version
//! will compute the set of packages and versions needed
//! to satisfy the dependencies of that package and version pair.
//! If there is no solution, the reason will be provided as clear as possible.

use std::collections::HashMap as Map;
use std::error::Error;
use std::hash::Hash;

use crate::cache::Cache;
use crate::range::Range;
use crate::version::Version;

/// Solver trait.
/// Given functions to retrieve the list of available versions of a package,
/// and their dependencies, this provides a `run` method,
/// able to compute a complete set of direct and indirect dependencies
/// satisfying the chosen package constraints.
///
/// Remark: for ease of use, the `Solver` trait is automatically implemented
/// for any type that implements `Cache` such as `SimpleCache`.
pub trait Solver<P, V>
where
    P: Clone + Eq + Hash,
    V: Clone + Ord + Version,
{
    /// List available versions for a given package.
    /// The strategy of which version should be preferably picked in the list of available versions
    /// is implied by the order of the list: first version in the list will be tried first.
    fn list_available_versions(&self, package: &P) -> Result<Vec<V>, Box<dyn Error>>;

    /// Retrieve the package dependencies.
    /// Return None if it's dependencies are unknown.
    fn get_dependencies(
        &self,
        package: &P,
        version: &V,
    ) -> Result<Option<Map<P, Range<V>>>, Box<dyn Error>>;

    /// Solve dependencies of a given package.
    fn run(&self, package: &P, version: &V) -> Result<Map<P, V>, Box<dyn Error>> {
        todo!()
    }
}

/// Automatically implement Config if your type implements Cache.
/// Versions are listed with newest versions first.
impl<P, V, C> Solver<P, V> for C
where
    P: Clone + Eq + Hash,
    V: Clone + Ord + Version,
    C: Cache<P, V>,
{
    fn list_available_versions(&self, package: &P) -> Result<Vec<V>, Box<dyn Error>> {
        Ok(self
            .versions(package)
            .map(|v| v.into_iter().rev().collect())
            .unwrap_or(Vec::new()))
    }

    fn get_dependencies(
        &self,
        package: &P,
        version: &V,
    ) -> Result<Option<Map<P, Range<V>>>, Box<dyn Error>> {
        Ok(self.dependencies(package, version))
    }
}
