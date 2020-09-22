// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Assignments are the building blocks of a PubGrub partial solution.
//! (partial solution = the current state of the solution we are building in the algorithm).

use std::hash::Hash;

use crate::internal::incompatibility::Incompatibility;
use crate::internal::term::Term;
use crate::version::Version;

/// An assignment is either a decision: a chosen version for a package,
/// or a derivation : a term specifying compatible versions for a package.
/// We also record the incompatibility at the origin of a derivation, called its cause.
#[derive(Clone)]
pub enum Assignment<P, V>
where
    P: Clone + Eq + Hash,
    V: Clone + Ord + Version,
{
    /// The decision.
    Decision {
        /// The package corresponding to the decision.
        package: P,
        /// The decided version.
        version: V,
    },
    /// The derivation.
    Derivation {
        /// The package corresponding to the derivation.
        package: P,
        /// Term of the derivation.
        term: Term<V>,
        /// Incompatibility cause of the derivation.
        cause: Incompatibility<P, V>,
    },
}

impl<P, V> Assignment<P, V>
where
    P: Clone + Eq + Hash,
    V: Clone + Ord + Version,
{
    /// Return the package for this assignment
    pub fn package(&self) -> &P {
        match self {
            Self::Decision { package, .. } => package,
            Self::Derivation { package, .. } => package,
        }
    }

    /// Retrieve the current assignment as a `Term`.
    /// If this is decision, it returns a positive term with that exact version.
    /// Otherwise, if this is a derivation, just returns its term.
    pub fn as_term(&self) -> Term<V> {
        match &self {
            Self::Decision { version, .. } => Term::exact(version.clone()),
            Self::Derivation { term, .. } => term.clone(),
        }
    }

    /// Constructor for a decision.
    pub fn decision(package: P, version: V) -> Self {
        Self::Decision { package, version }
    }

    /// Constructor for a derivation.
    pub fn derivation(package: P, term: Term<V>, cause: Incompatibility<P, V>) -> Self {
        Self::Derivation {
            package,
            term,
            cause,
        }
    }
}
