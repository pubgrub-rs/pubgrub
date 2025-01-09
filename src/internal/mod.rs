// SPDX-License-Identifier: MPL-2.0

//! Non exposed modules.

mod arena;
mod core;
mod incompatibility;
mod partial_solution;
mod small_map;
mod small_vec;

pub(crate) use arena::{Arena, HashArena};
pub(crate) use incompatibility::{IncompDpId, Relation};
pub(crate) use partial_solution::{DecisionLevel, PartialSolution, SatisfierSearch};
pub(crate) use small_map::SmallMap;
pub(crate) use small_vec::SmallVec;

// uv-specific additions
pub use arena::Id;
pub use core::State;
pub use incompatibility::{IncompId, Incompatibility, Kind};
