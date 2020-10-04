// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! PubGrub version solving algorithm.
//!
//! Version solving consists in efficiently finding a set of packages and versions
//! that satisfy all the constraints of a given project dependencies.
//! In addition, when that is not possible,
//! we should try to provide a very human-readable and clear
//! explanation as to why that failed.
//!
//! # Package and Version traits
//!
//! All the code in this crate is manipulating packages and versions,
//! and for this to work, we defined a `Package` and `Version` traits,
//! that are used as bounds on most of the exposed types and functions.
//!
//! Package identifiers needs to implement our `Package` trait,
//! which is automatic if the type already implements
//! `Clone + Eq + Hash + Debug + Display`.
//! So things like `String` will work out of the box.
//!
//! Our `Version` trait requires `Clone + Ord + Debug + Display`
//! and also the definition of two methods,
//! `lowest() -> Self` which returns the lowest version existing,
//! and `bump(&self) -> Self` which returns the next smallest version
//! strictly higher than the current one.
//! For convenience, this library already provides two implementations of `Version`.
//! The first one is `NumberVersion`, basically a newtype for `usize`.
//! The second one is `SemanticVersion` that implements semantic versioning rules.
//!
//! # Basic example
//!
//! Let's imagine that we are building a user interface
//! with a menu containing dropdowns with some icons,
//! icons that we are also directly using in other parts of the interface.
//! For this scenario our direct dependencies are `menu` and `icons`,
//! but the complete set of dependencies looks like follows:
//!
//! - `root` depends on `menu` and `icons`
//! - `menu` depends on `dropdown`
//! - `dropdown` depends on `icons`
//! - `icons` has no dependency
//!
//! We can model that scenario with this library as follows
//! ```ignore
//! let mut solver = OfflineSolver::<&str, NumberVersion>::new();
//! solver.add_dependencies(
//!     "root", 1, vec![("menu", Range::any()), ("icons", Range::any())],
//! );
//! solver.add_dependencies("menu", 1, vec![("dropdown", Range::any())]);
//! solver.add_dependencies("dropdown", 1, vec![("icons", Range::any())]);
//! solver.add_dependencies("icons", 1, vec![]);
//!
//! // Run the solver.
//! let _solution = solver.run("root", 1).unwrap();
//! ```
//!
//! # Solver trait
//!
//! In our previous example we used the `OfflineSolver`,
//! which is a basic implementation of the `Solver` trait.
//!
//! But we might want to implement the `Solver` trait for our own type.
//! Let's say that we will use `String` for packages,
//! and `SemanticVersion` for versions.
//! This may be done quite easily by implementing the two following functions.
//! ```ignore
//! impl Solver<String, SemanticVersion> for MySolver {
//!     fn list_available_versions(
//!         &mut self,
//!         package: &String
//!     ) -> Result<Vec<SemanticVersion>, Box<dyn Error>> {
//!         ...
//!     }
//!
//!     fn get_dependencies(
//!         &mut self,
//!         package: &String,
//!         version: &SemanticVersion,
//!     ) -> Result<Option<Map<String, Range<SemanticVersion>>>, Box<dyn Error>> {
//!         ...
//!     }
//! }
//! ```
//! The first method `list_available_versions` should return all available
//! versions of a package.
//! The second method `get_dependencies` aims at retrieving the dependencies
//! of a given package at a given version.
//! Return `None` if dependencies are unknown.
//!
//! On a real scenario, these two methods may involve reading the file system
//! or doing network request, so you may want to hold a cache in your `MySolver` type.
//! You could use the `OfflineSolver` type provided by the crate as guidance,
//! but you are free to use whatever approach
//! makes sense in your situation.
//!
//! # Solution and error reporting
//!
//! When everything goes well, the solver finds and returns the complete
//! set of direct and indirect dependencies satisfying all the constraints.
//! The packages and versions selected are returned in a `IndexMap<P, V>`.
//! But sometimes there is no solution because dependencies are incompatible.
//! In such cases, `solver.run(...)` returns a
//! `PubGrubError::NoSolution(derivation_tree)`,
//! where the provided derivation tree is a custom binary tree
//! containing the full chain of reasons why there is no solution.
//!
//! All the items in the tree are called incompatibilities
//! and may be of two types, either "external" or "derived".
//! Leaves of the tree are external incompatibilities,
//! and nodes are derived.
//! External incompatibilities have reasons that are independent
//! of the way this solver is implemented such as
//!  - dependencies: "package_a" at version 1 depends on "package_b" at version 4
//!  - missing dependencies: dependencies of "package_a" are unknown
//!  - absence of version: there is no version of "package_a" in the range [3.1.0  4.0.0[
//!
//! Derived incompatibilities are obtained by the solver by deduction,
//! such as if "a" depends on "b" and "b" depends on "c", "a" depends on "c".
//!
//! This crate defines a `Reporter` trait, with an associated `Output` type
//! and a single method
//! ```ignore
//! report(derivation_tree: &DerivationTree<P, V>) -> Output
//! ```
//! Implementing a `Reporter` may involve a lot of heuristics
//! to make the output human-readable and natural.
//! For convenience, we provide a default implementation
//! `DefaultStringReporter`, that output the report as a String.
//! You may use it as follows:
//! ```ignore
//! match solver.run(root_package, root_version) {
//!     Ok(solution) => println!("{:?}", solution),
//!     Err(PubGrubError::NoSolution(mut derivation_tree)) => {
//!         derivation_tree.collapse_noversion();
//!         eprintln!("{}", DefaultStringReporter::report(&derivation_tree));
//!     }
//!     Err(err) => panic!("{:?}", err),
//! };
//! ```
//! Notice that we also used `collapse_noversion()` above.
//! This method simplifies the derivation tree to get rid
//! of the `NoVersion` external incompatibilities in the derivation tree.
//! So instead of seeing things like this in the report:
//! ```txt
//! Because there is no version of foo in 1.0.1 <= v < 2.0.0
//! and foo 1.0.0 depends on bar 2.0.0 <= v < 3.0.0,
//! foo 1.0.0 <= v < 2.0.0 depends on bar 2.0.0 <= v < 3.0.0.
//! ```
//! you may have directly:
//! ```txt
//! foo 1.0.0 <= v < 2.0.0 depends on bar 2.0.0 <= v < 3.0.0.
//! ```
//! Beware though that if you are using some kind of offline mode
//! with a cache, you may want to know that some versions
//! do not exist in your cache.

#![warn(missing_docs)]

pub mod error;
pub mod package;
pub mod range;
pub mod report;
pub mod solver;
pub mod term;
pub mod version;

mod internal;
