// SPDX-License-Identifier: MPL-2.0

//! Handling pubgrub errors.

use thiserror::Error;

use crate::package::Package;
use crate::report::DerivationTree;
use crate::version::Version;

/// Errors that may occur while solving dependencies.
#[derive(Error, Debug)]
pub enum PubGrubError<P: Package, V: Version> {
    /// There is no solution for this set of dependencies.
    #[error("No solution")]
    NoSolution(DerivationTree<P, V>),

    /// Error arising when the implementer of
    /// [DependencyProvider](crate::solver::DependencyProvider)
    /// returned an error in the method
    /// [list_available_versions](crate::solver::DependencyProvider::list_available_versions).
    #[error("Retrieving available versions of package {package} failed")]
    ErrorRetrievingVersions {
        /// Package for which we want the list of versions.
        package: P,
        /// Error raised by the implementer of
        /// [DependencyProvider](crate::solver::DependencyProvider).
        source: Box<dyn std::error::Error>,
    },

    /// Error arising when the implementer of
    /// [DependencyProvider](crate::solver::DependencyProvider)
    /// returned an error in the method
    /// [get_dependencies](crate::solver::DependencyProvider::get_dependencies).
    #[error("Retrieving dependencies of {package} {version} failed")]
    ErrorRetrievingDependencies {
        /// Package whose dependencies we want.
        package: P,
        /// Version of the package for which we want the dependencies.
        version: V,
        /// Error raised by the implementer of
        /// [DependencyProvider](crate::solver::DependencyProvider).
        source: Box<dyn std::error::Error>,
    },

    /// Error arising when the implementer of
    /// [DependencyProvider](crate::solver::DependencyProvider)
    /// returned a dependency on an empty range.
    /// This technically means that the package can not be selected,
    /// but is clearly some kind of mistake.
    #[error("Retrieving dependency {dependent} of {package} {version} is the empty set")]
    ForbiddenEmptyDependency {
        /// Package whose dependencies we want.
        package: P,
        /// Version of the package for which we want the dependencies.
        version: V,
        /// The dependent package that requires us to pick from the empty set.
        dependent: P,
    },

    /// Error arising when the implementer of [DependencyProvider](crate::solver::DependencyProvider)
    /// returned an error in the method [should_cancel](crate::solver::DependencyProvider::should_cancel).
    #[error("We should cancel")]
    ErrorShouldCancel(Box<dyn std::error::Error>),

    /// Something unexpected happened.
    #[error("{0}")]
    Failure(String),
}
