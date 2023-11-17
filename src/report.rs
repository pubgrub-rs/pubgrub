// SPDX-License-Identifier: MPL-2.0

//! Build a report as clear as possible as to why
//! dependency solving failed.

use std::fmt;
use std::ops::{Deref, DerefMut};
use std::fmt::Write;

use crate::package::Package;
use crate::term::Term;
use crate::type_aliases::Map;
use crate::version_set::VersionSet;

/// Derivation tree resulting in the impossibility
/// to solve the dependencies of our root package.
#[derive(Debug, Clone)]
pub enum DerivationTree<P: Package, VS: VersionSet> {
    /// External incompatibility.
    External(External<P, VS>),
    /// Incompatibility derived from two others.
    Derived(Derived<P, VS>),
}

/// Incompatibilities that are not derived from others,
/// they have their own reason.
#[derive(Debug, Clone)]
pub enum External<P: Package, VS: VersionSet> {
    /// Initial incompatibility aiming at picking the root package for the first decision.
    NotRoot(P, VS::V),
    /// There are no versions in the given set for this package.
    NoVersions(P, VS),
    /// Dependencies of the package are unavailable for versions in that set.
    UnavailableDependencies(P, VS),
    /// Incompatibility coming from the dependencies of a given package.
    FromDependencyOf(P, VS, P, VS),
}

/// Incompatibility derived from two others.
#[derive(Debug, Clone)]
pub struct Derived<P: Package, VS: VersionSet> {
    /// Terms of the incompatibility.
    pub terms: Map<P, Term<VS>>,
    /// Indicate if that incompatibility is present multiple times
    /// in the derivation tree.
    /// If that is the case, it has a unique id, provided in that option.
    /// Then, we may want to only explain it once,
    /// and refer to the explanation for the other times.
    pub shared_id: Option<usize>,
    /// First cause.
    pub cause1: Box<DerivationTree<P, VS>>,
    /// Second cause.
    pub cause2: Box<DerivationTree<P, VS>>,
}

impl<P: Package, VS: VersionSet> DerivationTree<P, VS> {
    /// Merge the [NoVersions](External::NoVersions) external incompatibilities
    /// with the other one they are matched with
    /// in a derived incompatibility.
    /// This cleans up quite nicely the generated report.
    /// You might want to do this if you know that the
    /// [DependencyProvider](crate::solver::DependencyProvider)
    /// was not run in some kind of offline mode that may not
    /// have access to all versions existing.
    pub fn collapse_no_versions(&mut self) {
        match self {
            DerivationTree::External(_) => {}
            DerivationTree::Derived(derived) => {
                match (derived.cause1.deref_mut(), derived.cause2.deref_mut()) {
                    (DerivationTree::External(External::NoVersions(p, r)), ref mut cause2) => {
                        cause2.collapse_no_versions();
                        *self = cause2
                            .clone()
                            .merge_no_versions(p.to_owned(), r.to_owned())
                            .unwrap_or_else(|| self.to_owned());
                    }
                    (ref mut cause1, DerivationTree::External(External::NoVersions(p, r))) => {
                        cause1.collapse_no_versions();
                        *self = cause1
                            .clone()
                            .merge_no_versions(p.to_owned(), r.to_owned())
                            .unwrap_or_else(|| self.to_owned());
                    }
                    _ => {
                        derived.cause1.collapse_no_versions();
                        derived.cause2.collapse_no_versions();
                    }
                }
            }
        }
    }

    fn merge_no_versions(self, package: P, set: VS) -> Option<Self> {
        match self {
            // TODO: take care of the Derived case.
            // Once done, we can remove the Option.
            DerivationTree::Derived(_) => Some(self),
            DerivationTree::External(External::NotRoot(_, _)) => {
                panic!("How did we end up with a NoVersions merged with a NotRoot?")
            }
            DerivationTree::External(External::NoVersions(_, r)) => Some(DerivationTree::External(
                External::NoVersions(package, set.union(&r)),
            )),
            DerivationTree::External(External::UnavailableDependencies(_, r)) => Some(
                DerivationTree::External(External::UnavailableDependencies(package, set.union(&r))),
            ),
            DerivationTree::External(External::FromDependencyOf(p1, r1, p2, r2)) => {
                if p1 == package {
                    Some(DerivationTree::External(External::FromDependencyOf(
                        p1,
                        r1.union(&set),
                        p2,
                        r2,
                    )))
                } else {
                    Some(DerivationTree::External(External::FromDependencyOf(
                        p1,
                        r1,
                        p2,
                        r2.union(&set),
                    )))
                }
            }
        }
    }
}

impl<P: Package, VS: VersionSet> fmt::Display for External<P, VS> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotRoot(package, version) => {
                write!(f, "we are solving dependencies of {} {}", package, version)
            }
            Self::NoVersions(package, set) => {
                if set == &VS::full() {
                    write!(f, "there is no available version for {}", package)
                } else {
                    write!(f, "there is no version of {} in {}", package, set)
                }
            }
            Self::UnavailableDependencies(package, set) => {
                if set == &VS::full() {
                    write!(f, "dependencies of {} are unavailable", package)
                } else {
                    write!(
                        f,
                        "dependencies of {} at version {} are unavailable",
                        package, set
                    )
                }
            }
            Self::FromDependencyOf(p, set_p, dep, set_dep) => {
                if set_p == &VS::full() && set_dep == &VS::full() {
                    write!(f, "{} depends on {}", p, dep)
                } else if set_p == &VS::full() {
                    write!(f, "{} depends on {} {}", p, dep, set_dep)
                } else if set_dep == &VS::full() {
                    write!(f, "{} {} depends on {}", p, set_p, dep)
                } else {
                    write!(f, "{} {} depends on {} {}", p, set_p, dep, set_dep)
                }
            }
        }
    }
}

/// Default reporter able to generate an explanation as a [String].
pub struct DefaultStringReporter {
    /// Number of explanations already with a line reference.
    ref_count: usize,
    /// Shared nodes that have already been marked with a line reference.
    /// The incompatibility ids are the keys, and the line references are the values.
    shared_with_ref: Map<usize, usize>,
    /// Accumulated lines of the report already generated.
    lines: Vec<String>,
}

impl DefaultStringReporter {
    /// Initialize the reporter.
    pub fn new() -> Self {
        Self {
            ref_count: 0,
            shared_with_ref: Map::default(),
            lines: Vec::new(),
        }
    }
}

pub trait Reporter<P: Package, VS: VersionSet> {
    fn register_shared_id(&mut self, id: usize);
    fn print_ln(&mut self, line: String);
    fn add_line_ref(&mut self) -> usize;
    fn line_ref_of(&mut self, shared_id: Option<usize>) -> Option<usize>;
    fn string_terms(&mut self, terms: &Map<P, Term<VS>>) -> String;
    fn string_external(&mut self, terms: &External<P, VS>) -> String;
}

impl<P: Package, VS: VersionSet> Reporter<P, VS> for DefaultStringReporter {
    fn register_shared_id(&mut self, id: usize) {
        if self.shared_with_ref.get(&id).is_none() {
            let ref_count = Reporter::<P, VS>::add_line_ref(self);
            self.shared_with_ref.insert(id, ref_count);
        }
    }

    fn print_ln(&mut self, line: String) {
        self.lines.push(line)
    }

    fn add_line_ref(&mut self) -> usize {
        self.ref_count += 1;
        if let Some(line) = self.lines.last_mut() {
            write!(line, " ({})", self.ref_count).unwrap();
        }
        self.ref_count
    }

    fn line_ref_of(&mut self, shared_id: Option<usize>) -> Option<usize> {
        shared_id.and_then(|id| self.shared_with_ref.get(&id).cloned())
    }

    fn string_terms(&mut self, terms: &Map<P, Term<VS>>) -> String {
        let terms_vec: Vec<_> = terms.iter().collect();
        match terms_vec.as_slice() {
            [] => "version solving failed".into(),
            // TODO: special case when that unique package is root.
            [(package, Term::Positive(range))] => format!("{} {} is forbidden", package, range),
            [(package, Term::Negative(range))] => format!("{} {} is mandatory", package, range),
            [(p1, Term::Positive(r1)), (p2, Term::Negative(r2))] => {
                External::FromDependencyOf(p1, r1.clone(), p2, r2.clone()).to_string()
            }
            [(p1, Term::Negative(r1)), (p2, Term::Positive(r2))] => {
                External::FromDependencyOf(p2, r2.clone(), p1, r1.clone()).to_string()
            }
            slice => {
                let str_terms: Vec<_> = slice.iter().map(|(p, t)| format!("{} {}", p, t)).collect();
                str_terms.join(", ") + " are incompatible"
            }
        }
    }

    fn string_external(&mut self, terms: &External<P, VS>) -> String {
        terms.to_string()
    }
}

fn build_recursive<P: Package, VS: VersionSet>(
    reporter: &mut impl Reporter<P, VS>,
    derived: &Derived<P, VS>,
) {
    build_recursive_helper(reporter, derived);
    if let Some(id) = derived.shared_id {
        reporter.register_shared_id(id);
    };
}

fn build_recursive_helper<P: Package, VS: VersionSet>(
    reporter: &mut impl Reporter<P, VS>,
    current: &Derived<P, VS>,
) {
    match (current.cause1.deref(), current.cause2.deref()) {
        (DerivationTree::External(external1), DerivationTree::External(external2)) => {
            // Simplest case, we just combine two external incompatibilities.
            let line = explain_both_external(reporter, external1, external2, &current.terms);
            reporter.print_ln(line);
        }
        (DerivationTree::Derived(derived), DerivationTree::External(external)) => {
            // One cause is derived, so we explain this first
            // then we add the one-line external part
            // and finally conclude with the current incompatibility.
            report_one_each(reporter, derived, external, &current.terms);
        }
        (DerivationTree::External(external), DerivationTree::Derived(derived)) => {
            report_one_each(reporter, derived, external, &current.terms);
        }
        (DerivationTree::Derived(derived1), DerivationTree::Derived(derived2)) => {
            // This is the most complex case since both causes are also derived.
            match (
                reporter.line_ref_of(derived1.shared_id),
                reporter.line_ref_of(derived2.shared_id),
            ) {
                // If both causes already have been referenced (shared_id),
                // the explanation simply uses those references.
                (Some(ref1), Some(ref2)) => {
                    let line =
                        explain_both_ref(reporter, ref1, derived1, ref2, derived2, &current.terms);
                    reporter.print_ln(line);
                }
                // Otherwise, if one only has a line number reference,
                // we recursively call the one without reference and then
                // add the one with reference to conclude.
                (Some(ref1), None) => {
                    build_recursive(reporter, derived2);
                    let line = and_explain_ref(reporter, ref1, derived1, &current.terms);
                    reporter.print_ln(line);
                }
                (None, Some(ref2)) => {
                    build_recursive(reporter, derived1);
                    let line = and_explain_ref(reporter, ref2, derived2, &current.terms);
                    reporter.print_ln(line);
                }
                // Finally, if no line reference exists yet,
                // we call recursively the first one and then,
                //   - if this was a shared node, it will get a line ref
                //     and we can simply recall this with the current node.
                //   - otherwise, we add a line reference to it,
                //     recursively call on the second node,
                //     and finally conclude.
                (None, None) => {
                    build_recursive(reporter, derived1);
                    if derived1.shared_id.is_some() {
                        reporter.print_ln("".into());
                        build_recursive(reporter, current);
                    } else {
                        let ref1 = reporter.add_line_ref();
                        reporter.print_ln("".into());
                        build_recursive(reporter, derived2);
                        let line = and_explain_ref(reporter, ref1, derived1, &current.terms);
                        reporter.print_ln(line);
                    }
                }
            }
        }
    }
}

/// Report a derived and an external incompatibility.
///
/// The result will depend on the fact that the derived incompatibility
/// has already been explained or not.
fn report_one_each<P: Package, VS: VersionSet>(
    reporter: &mut impl Reporter<P, VS>,
    derived: &Derived<P, VS>,
    external: &External<P, VS>,
    current_terms: &Map<P, Term<VS>>,
) {
    match reporter.line_ref_of(derived.shared_id) {
        Some(ref_id) => {
            let line = explain_ref_and_external(reporter, ref_id, derived, external, current_terms);
            reporter.print_ln(line)
        }
        None => report_recurse_one_each(reporter, derived, external, current_terms),
    }
}

/// Report one derived (without a line ref yet) and one external.
fn report_recurse_one_each<P: Package, VS: VersionSet>(
    reporter: &mut impl Reporter<P, VS>,
    derived: &Derived<P, VS>,
    external: &External<P, VS>,
    current_terms: &Map<P, Term<VS>>,
) {
    match (derived.cause1.deref(), derived.cause2.deref()) {
        // If the derived cause has itself one external prior cause,
        // we can chain the external explanations.
        (DerivationTree::Derived(prior_derived), DerivationTree::External(prior_external)) => {
            build_recursive(reporter, prior_derived);
            let line =
                and_explain_prior_and_external(reporter, prior_external, external, current_terms);
            reporter.print_ln(line);
        }
        // If the derived cause has itself one external prior cause,
        // we can chain the external explanations.
        (DerivationTree::External(prior_external), DerivationTree::Derived(prior_derived)) => {
            build_recursive(reporter, prior_derived);
            let line =
                and_explain_prior_and_external(reporter, prior_external, external, current_terms);
            reporter.print_ln(line);
        }
        _ => {
            build_recursive(reporter, derived);
            let line = and_explain_external(reporter, external, current_terms);
            reporter.print_ln(line);
        }
    }
}

// String explanations #####################################################

/// Simplest case, we just combine two external incompatibilities.
fn explain_both_external<P: Package, VS: VersionSet>(
    reporter: &mut impl Reporter<P, VS>,
    external1: &External<P, VS>,
    external2: &External<P, VS>,
    current_terms: &Map<P, Term<VS>>,
) -> String {
    // TODO: order should be chosen to make it more logical.
    format!(
        "Because {} and {}, {}.",
        reporter.string_external(external1),
        reporter.string_external(external2),
        reporter.string_terms(current_terms)
    )
}

/// Both causes have already been explained so we use their refs.
fn explain_both_ref<P: Package, VS: VersionSet>(
    reporter: &mut impl Reporter<P, VS>,
    ref_id1: usize,
    derived1: &Derived<P, VS>,
    ref_id2: usize,
    derived2: &Derived<P, VS>,
    current_terms: &Map<P, Term<VS>>,
) -> String {
    // TODO: order should be chosen to make it more logical.
    format!(
        "Because {} ({}) and {} ({}), {}.",
        reporter.string_terms(&derived1.terms),
        ref_id1,
        reporter.string_terms(&derived2.terms),
        ref_id2,
        reporter.string_terms(current_terms)
    )
}

/// One cause is derived (already explained so one-line),
/// the other is a one-line external cause,
/// and finally we conclude with the current incompatibility.
fn explain_ref_and_external<P: Package, VS: VersionSet>(
    reporter: &mut impl Reporter<P, VS>,
    ref_id: usize,
    derived: &Derived<P, VS>,
    external: &External<P, VS>,
    current_terms: &Map<P, Term<VS>>,
) -> String {
    // TODO: order should be chosen to make it more logical.
    format!(
        "Because {} ({}) and {}, {}.",
        reporter.string_terms(&derived.terms),
        ref_id,
        reporter.string_external(external),
        reporter.string_terms(current_terms)
    )
}

/// Add an external cause to the chain of explanations.
fn and_explain_external<P: Package, VS: VersionSet>(
    reporter: &mut impl Reporter<P, VS>,
    external: &External<P, VS>,
    current_terms: &Map<P, Term<VS>>,
) -> String {
    format!(
        "And because {}, {}.",
        reporter.string_external(external),
        reporter.string_terms(current_terms)
    )
}

/// Add an already explained incompat to the chain of explanations.
fn and_explain_ref<P: Package, VS: VersionSet>(
    reporter: &mut impl Reporter<P, VS>,
    ref_id: usize,
    derived: &Derived<P, VS>,
    current_terms: &Map<P, Term<VS>>,
) -> String {
    format!(
        "And because {} ({}), {}.",
        reporter.string_terms(&derived.terms),
        ref_id,
        reporter.string_terms(current_terms)
    )
}

/// Add an already explained incompat to the chain of explanations.
fn and_explain_prior_and_external<P: Package, VS: VersionSet>(
    reporter: &mut impl Reporter<P, VS>,
    prior_external: &External<P, VS>,
    external: &External<P, VS>,
    current_terms: &Map<P, Term<VS>>,
) -> String {
    format!(
        "And because {} and {}, {}.",
        reporter.string_external(prior_external),
        reporter.string_external(external),
        reporter.string_terms(current_terms)
    )
}

/// A straightforward way to convert a [`DerivationTree`] into a string for presenting to a user.
pub fn basic_string_reporter<P: Package, VS: VersionSet>(
    derivation_tree: &DerivationTree<P, VS>,
) -> String {
    let mut reporter = DefaultStringReporter::new();
    match derivation_tree {
        DerivationTree::External(external) => reporter.string_external(external),
        DerivationTree::Derived(derived) => {
            build_recursive(&mut reporter, derived);
            reporter.lines.join("\n")
        }
    }
}
