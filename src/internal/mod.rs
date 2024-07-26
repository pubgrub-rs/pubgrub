// SPDX-License-Identifier: MPL-2.0

//! Non exposed modules.

pub(crate) use core::State;
pub(crate) use incompatibility::{IncompDpId, Incompatibility};
pub(crate) use partial_solution::SatisfierSearch;
pub(crate) use small_vec::SmallVec;

mod arena;
mod core;
mod incompatibility;
mod partial_solution;
mod small_map;
mod small_vec;
