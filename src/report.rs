// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Build a report as clear as possible as to why
//! dependency solving failed.

use std::collections::HashMap as Map;
use std::rc::Rc;

use crate::internal::term::Term;
use crate::package::Package;
use crate::range::Range;
use crate::version::Version;

/// Reporter trait.
pub trait Reporter<P: Package, V: Version> {}

/// Derivation tree resulting in the impossibility
/// to solve the dependencies of our root package.
pub enum DerivationTree<P: Package, V: Version> {
    /// External incompatibility.
    External(ExternalKind<P, V>),
    /// Incompatibility derived from two others.
    Derived {
        /// Terms of the incompatibility.
        terms: Map<P, Term<V>>,
        /// Indicate if that incompatibility is present multiple times
        /// in the derivation tree.
        /// If that is the case, it has a unique id, provided in that option.
        /// Then, we may want to only explain it once,
        /// and refer to the explanation for the other times.
        shared_id: Option<usize>,
        /// First cause.
        cause1: Rc<DerivationTree<P, V>>,
        /// Second cause.
        cause2: Rc<DerivationTree<P, V>>,
    },
}

/// Incompatibilities that are not derived from others,
/// they have their own reason.
pub enum ExternalKind<P: Package, V: Version> {
    /// No version exist in that range.
    NoVersion(P, Range<V>),
    /// Dependencies of the package are unavailable for versions in that range.
    UnavailableDependencies(P, Range<V>),
    /// Incompatibility coming from the dependencies of a given package.
    FromDependencyOf(P, Range<V>, P, Range<V>),
}
