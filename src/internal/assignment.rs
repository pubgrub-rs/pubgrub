// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Assignments are the building blocks of a PubGrub partial solution.
//! (partial solution = the current state of the solution we are building in the algorithm).

use std::hash::Hash;

use crate::internal::incompatibility::Incompatibility;
use crate::internal::term::Term;
use crate::range::Range;
use crate::version::Version;

/// An assignment refers to a given package and can either be
/// (1) a decision, which is a chosen version,
/// or (2) a derivation, which is a `Term` specifying compatible versions.
#[derive(Clone)]
pub struct Assignment<'a, P, V>
where
    P: Clone + Eq + Hash,
    V: Clone + Ord + Version,
{
    /// A `decisionLevel` records how many decisions have already been taken,
    /// including this one if it is a decision.
    pub decision_level: usize,
    /// Package that this assignment refers to.
    pub package: P,
    /// Type of the assignement, either decision or derivation.
    pub kind: Kind<'a, P, V>,
}

/// An assignment is either a decision, with the chosen version,
/// or a derivation term, specifying compatible versions
/// according to previous assignments and all incompatibilities.
/// We also record the incompatibility responsible for
/// that derivation term as its "cause".
#[derive(Clone)]
pub enum Kind<'a, P, V>
where
    P: Clone + Eq + Hash,
    V: Clone + Ord + Version,
{
    /// A decision is the choice of a version.
    Decision(V),
    /// A derivation aggregates previous constraints into a term
    /// specifying compatible versions.
    /// It also records the incompatibility which caused that derivated term.
    Derivation {
        /// Term of the derivation.
        term: Term<V>,
        /// Incompatibility cause of the derivation.
        cause: &'a Incompatibility<'a, P, V>,
    },
}

impl<'a, P, V> Assignment<'a, P, V>
where
    P: Clone + Eq + Hash,
    V: Clone + Ord + Version,
{
    /// Retrieve the current assignment as a `Term`.
    /// If this is decision, it returns a positive term with that exact version.
    /// Otherwise, if this is a derivation, just returns its term.
    pub fn as_term(&self) -> Term<V> {
        match &self.kind {
            Kind::Decision(version) => Term::Positive(Range::exact(version.clone())),
            Kind::Derivation { term, .. } => term.clone(),
        }
    }

    /// Constructor for a decision.
    pub fn new_decision(level: usize, package: P, version: V) -> Self {
        Self {
            decision_level: level,
            package,
            kind: Kind::Decision(version),
        }
    }

    /// Constructor for a derivation.
    pub fn new_derivation(
        level: usize,
        package: P,
        term: Term<V>,
        cause: &'a Incompatibility<'a, P, V>,
    ) -> Self {
        Self {
            decision_level: level,
            package,
            kind: Kind::Derivation { term, cause },
        }
    }
}
