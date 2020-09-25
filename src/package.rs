// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Trait for identifying packages.
//! Automatically implemented for traits implementing Clone + Eq + Hash.

use std::hash::Hash;

/// Trait for identifying packages.
/// Automatically implemented for types already implementing Clone + Eq + Hash.
pub trait Package: Clone + Eq + Hash {}

/// Automatically implement the Package trait for any type
/// that already implement Clone + Eq + Hash.
impl<T: Clone + Eq + Hash> Package for T {}
