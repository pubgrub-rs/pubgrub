// SPDX-License-Identifier: MPL-2.0

//! Non exposed modules.

mod arena;
mod core;
mod incompatibility;
mod partial_solution;
mod small_map;
mod small_vec;

pub(crate) use arena::{Arena, HashArena};
pub(crate) use incompatibility::{IncompDpId, IncompId, Incompatibility, Relation};
pub(crate) use partial_solution::{DecisionLevel, PartialSolution, SatisfierSearch};
pub(crate) use small_map::SmallMap;
pub(crate) use small_vec::SmallVec;

// Unstable API for driving solver state directly.
pub use arena::Id;
#[cfg(feature = "unstable-state-api")]
pub use core::ConflictId;
pub use core::{Dependency, State};
