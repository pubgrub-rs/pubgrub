// SPDX-License-Identifier: MPL-2.0

//! Handling pubgrub errors.

use thiserror::Error;

use crate::{DependencyProvider, DerivationTree};

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
    #[error("There is no solution")]
    NoSolution(NoSolutionError<DP>),

    /// Error arising when the implementer of [DependencyProvider] returned an error in the method
    /// [`get_dependencies`](DependencyProvider::get_dependencies).
    #[error("Retrieving dependencies of {package} {version} failed")]
    ErrorRetrievingDependencies {
        /// Package whose dependencies we want.
        package: DP::P,
        /// Version of the package for which we want the dependencies.
        version: DP::V,
        /// Error raised by the implementer of [DependencyProvider].
        source: DP::Err,
    },

    /// Error arising when the implementer of [DependencyProvider] returned an error in the method
    /// [`choose_version`](DependencyProvider::choose_version).
    #[error("Choosing a version for {package} failed")]
    ErrorChoosingVersion {
        /// Package to choose a version for.
        package: DP::P,
        /// Error raised by the implementer of [DependencyProvider].
        source: DP::Err,
    },

    /// Error arising when the implementer of [DependencyProvider]
    /// returned an error in the method [`should_cancel`](DependencyProvider::should_cancel).
    #[error("The solver was cancelled")]
    ErrorInShouldCancel(#[source] DP::Err),
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
            Self::ErrorChoosingVersion { package, source } => f
                .debug_struct("ErrorChoosingVersion")
                .field("package", package)
                .field("source", source)
                .finish(),
            Self::ErrorInShouldCancel(arg0) => {
                f.debug_tuple("ErrorInShouldCancel").field(arg0).finish()
            }
        }
    }
}
