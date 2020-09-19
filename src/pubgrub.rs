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
//! ```rust
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
//! The algorithm is provided in two forms, synchronous and asynchronous.
//! The synchronous API is quite straightforward.
//!
//!
//! ### Direct synchronous call
//!
//! ```rust
//! solution = pubgrub_config.solve(package, version)?;
//! ```
//!
//! Where `pubgrub_config` provides the list of available packages and versions,
//! as well as the dependencies of every available package.
//! The call to `solve` for a given package at a given version
//! will compute the set of packages and versions needed
//! to satisfy the dependencies of that package and version pair.
//! If there is no solution, the reason will be provided as clear as possible.
//!
//!
//! ### Asynchronous API
//!
//! Sometimes, it is too expensive to provide upfront
//! the list of all packages and versions,
//! as well as all dependencies for every one of those.
//! This may very well require some network, file or other asynchronous code.
//! For this reason, it is possible to run the PubGrub algorithm step by step.
//! Every time an effect may be required, it stops and informs the caller,
//! which may resume the algorithm once necessary data is retrieved.
//!
//! ```rust
//! let effect = pubgrub_state.update(&cache, msg)?;
//! ```
//!
//! The `Effect` type is public to enable the caller
//! to identify and perform the required task before resuming.
//! The `Msg` type is also public to drive the algorithm according
//! to what was expected in the last effect when resuming.
//!
//! At any point between two `update` calls,
//! the caller can update the `Cache` of already loaded data.
//!
//! The algorithm informs the caller that all is done
//! when the `End` effect is returned.

use std::collections::HashMap as Map;
use std::error::Error;
use std::hash::Hash;

use crate::cache::Cache;
use crate::internal::assignment::Assignment;
use crate::internal::assignment::Kind;
use crate::internal::incompatibility::Incompatibility;
use crate::internal::incompatibility::Relation;
use crate::internal::partial_solution::PartialSolution;
use crate::range::Range;
use crate::version::Version;

// -----------------------------------------------------------------------------
// # Sync
// -----------------------------------------------------------------------------

/// Configuration of available packages to solve dependencies.
/// Remark: for ease of use, the Config trait is automatically implemented
/// for any type that implements Cache.
pub trait Config<P, V>
where
    P: Clone + Eq + Hash,
    V: Clone + Ord + Version,
{
    /// List available versions for a given package.
    /// The strategy of which version should be preferably picked in the list of available versions
    /// is implied by the order of the list: first version in the list will be tried first.
    fn list_available_versions(&self, package: &P) -> Vec<V>;

    /// Retrieve the package dependencies.
    /// Return None if it's dependencies are unknown.
    fn get_dependencies(&self, package: &P, version: &V) -> Option<Map<P, Range<V>>>;

    /// Solve dependencies of a given package.
    fn solve(&self, package: &P, version: &V) -> Result<Map<P, V>, Box<dyn Error>> {
        todo!()
    }
}

/// Automatically implement Config if your type implements Cache.
/// Versions are listed with newest versions first.
impl<P, V, C> Config<P, V> for C
where
    P: Clone + Eq + Hash,
    V: Clone + Ord + Version,
    C: Cache<P, V>,
{
    fn list_available_versions(&self, package: &P) -> Vec<V> {
        self.versions(package)
            .map(|v| v.into_iter().rev().collect())
            .unwrap_or(Vec::new())
    }

    fn get_dependencies(&self, package: &P, version: &V) -> Option<Map<P, Range<V>>> {
        self.dependencies(package, version)
    }
}

// -----------------------------------------------------------------------------
// # Async
// @docs State, stateToString, Effect, effectToString, Msg
// @docs init, update
// -----------------------------------------------------------------------------
