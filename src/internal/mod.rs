// SPDX-License-Identifier: MPL-2.0

//! Non exposed modules.

mod core;
mod incompatibility;
mod partial_solution;
mod small_map;

pub(crate) use core::State;
pub(crate) use incompatibility::{IncompatArena, IncompatId, Incompatibility, Relation};
pub(crate) use partial_solution::{DecisionLevel, PartialSolution, SatisfierSearch};
pub(crate) use small_map::SmallMap;
