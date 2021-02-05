// SPDX-License-Identifier: MPL-2.0

//! Assignments are the building blocks of a PubGrub partial solution.
//! (partial solution = the current state of the solution we are building in the algorithm).

use crate::internal::arena::Arena;
use crate::internal::incompatibility::IncompId;
use crate::internal::incompatibility::Incompatibility;
use crate::package::Package;
use crate::term::Term;
use crate::version::Version;

/// An assignment is either a decision: a chosen version for a package,
/// or a derivation : a term specifying compatible versions for a package.
/// We also record the incompatibility at the origin of a derivation, called its cause.
#[derive(Clone)]
pub enum Assignment<P: Package, V: Version> {
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
        /// Incompatibility cause of the derivation.
        cause: IncompId<P, V>,
    },
}

impl<P: Package, V: Version> Assignment<P, V> {
    /// Return the package for this assignment
    pub fn package(&self) -> &P {
        match self {
            Self::Decision { package, .. } => package,
            Self::Derivation { package, .. } => package,
        }
    }

    /// Retrieve the current assignment as a [Term].
    /// If this is decision, it returns a positive term with that exact version.
    /// Otherwise, if this is a derivation, just returns its term.
    pub fn as_term(&self, store: &Arena<Incompatibility<P, V>>) -> Term<V> {
        match &self {
            Self::Decision { version, .. } => Term::exact(version.clone()),
            Self::Derivation { package, cause } => store[*cause].get(&package).unwrap().negate(),
        }
    }
}
