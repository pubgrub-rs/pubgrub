// SPDX-License-Identifier: MPL-2.0

//! Handling pubgrub errors.

use thiserror::Error;

use crate::report::DerivationTree;
use crate::solver::DependencyProvider;

/// There is no solution for this set of dependencies.
pub type NoSolutionError<DP> = DerivationTree<
    <DP as DependencyProvider>::P,
    <DP as DependencyProvider>::VS,
    <DP as DependencyProvider>::M,
>;

/// Errors that may occur while solving dependencies.
#[derive(Error)]
pub enum PubGrubError<DP: DependencyProvider> {
    /// There is no solution for this set of dependencies.
    #[error("No solution")]
    NoSolution(NoSolutionError<DP>),

    /// Error arising when the implementer of
    /// [DependencyProvider]
    /// returned an error in the method
    /// [get_dependencies](crate::solver::DependencyProvider::get_dependencies).
    #[error("Retrieving dependencies of {package} {version} failed")]
    ErrorRetrievingDependencies {
        /// Package whose dependencies we want.
        package: DP::P,
        /// Version of the package for which we want the dependencies.
        version: DP::V,
        /// Error raised by the implementer of
        /// [DependencyProvider].
        source: DP::Err,
    },

    /// Error arising when the implementer of
    /// [DependencyProvider]
    /// returned a dependency on the requested package.
    /// This technically means that the package directly depends on itself,
    /// and is clearly some kind of mistake.
    #[error("{package} {version} depends on itself")]
    SelfDependency {
        /// Package whose dependencies we want.
        package: DP::P,
        /// Version of the package for which we want the dependencies.
        version: DP::V,
    },

    /// Error arising when the implementer of
    /// [DependencyProvider]
    /// returned an error in the method
    /// [choose_version](crate::solver::DependencyProvider::choose_version).
    #[error("Decision making failed")]
    ErrorChoosingPackageVersion(#[source] DP::Err),

    /// Error arising when the implementer of [DependencyProvider]
    /// returned an error in the method [should_cancel](crate::solver::DependencyProvider::should_cancel).
    #[error("We should cancel")]
    ErrorInShouldCancel(#[source] DP::Err),

    /// Something unexpected happened.
    #[error("{0}")]
    Failure(String),
}

impl<DP: DependencyProvider> From<NoSolutionError<DP>> for PubGrubError<DP> {
    fn from(err: NoSolutionError<DP>) -> Self {
        Self::NoSolution(err)
    }
}

impl<DP> std::fmt::Debug for PubGrubError<DP>
where
    DP: DependencyProvider,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSolution(err) => f.debug_tuple("NoSolution").field(&err).finish(),
            Self::ErrorRetrievingDependencies {
                package,
                version,
                source,
            } => f
                .debug_struct("ErrorRetrievingDependencies")
                .field("package", package)
                .field("version", version)
                .field("source", source)
                .finish(),
            Self::SelfDependency { package, version } => f
                .debug_struct("SelfDependency")
                .field("package", package)
                .field("version", version)
                .finish(),
            Self::ErrorChoosingPackageVersion(arg0) => f
                .debug_tuple("ErrorChoosingPackageVersion")
                .field(arg0)
                .finish(),
            Self::ErrorInShouldCancel(arg0) => {
                f.debug_tuple("ErrorInShouldCancel").field(arg0).finish()
            }
            Self::Failure(arg0) => f.debug_tuple("Failure").field(arg0).finish(),
        }
    }
}
