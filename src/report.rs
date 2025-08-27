// SPDX-License-Identifier: MPL-2.0

//! Build a report as clear as possible as to why
//! dependency solving failed.

use std::fmt::{self, Debug, Display};
use std::ops::Deref;
use std::sync::Arc;

use crate::{Map, Package, Set, Term, VersionSet};

/// Reporter trait.
pub trait Reporter<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> {
    /// Output type of the report.
    type Output;

    /// Generate a report from the derivation tree
    /// describing the resolution failure using the default formatter.
    fn report(derivation_tree: &DerivationTree<P, VS, M>) -> Self::Output;

    /// Generate a report from the derivation tree
    /// describing the resolution failure using a custom formatter.
    fn report_with_formatter(
        derivation_tree: &DerivationTree<P, VS, M>,
        formatter: &impl ReportFormatter<P, VS, M, Output = Self::Output>,
    ) -> Self::Output;
}

/// Derivation tree resulting in the impossibility to solve the dependencies of our root package.
#[derive(Debug, Clone)]
pub enum DerivationTree<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> {
    /// External incompatibility.
    External(External<P, VS, M>),
    /// Incompatibility derived from two others.
    Derived(Derived<P, VS, M>),
}

/// Incompatibility that is not derived from other incompatibilities.
#[derive(Debug, Clone)]
pub enum External<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> {
    /// Initial incompatibility aiming at picking the root package for the first decision.
    NotRoot(P, VS::V),
    /// There are no versions in the given set for this package.
    NoVersions(P, VS),
    /// Incompatibility coming from the dependencies of a given package.
    FromDependencyOf(P, VS, P, VS),
    /// The package is unusable for reasons outside pubgrub.
    Custom(P, VS, M),
}

/// Incompatibility derived from two others.
#[derive(Debug, Clone)]
pub struct Derived<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> {
    /// Terms of the incompatibility.
    pub terms: Map<P, Term<VS>>,
    /// Indicate if the incompatibility is present multiple times in the derivation tree.
    ///
    /// If that is the case, the number is a unique id. This can be used to only explain this
    /// incompatibility once, then refer to the explanation for the other times.
    pub shared_id: Option<usize>,
    /// First cause.
    pub cause1: Arc<DerivationTree<P, VS, M>>,
    /// Second cause.
    pub cause2: Arc<DerivationTree<P, VS, M>>,
}

impl<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> DerivationTree<P, VS, M> {
    /// Get all packages referred to in the derivation tree.
    pub fn packages(&self) -> Set<&P> {
        let mut packages = Set::default();
        match self {
            Self::External(external) => match external {
                External::FromDependencyOf(p, _, p2, _) => {
                    packages.insert(p);
                    packages.insert(p2);
                }
                External::NoVersions(p, _)
                | External::NotRoot(p, _)
                | External::Custom(p, _, _) => {
                    packages.insert(p);
                }
            },
            Self::Derived(derived) => {
                // Less efficient than recursing with a `&mut Set<&P>`, but it's sufficient for
                // small to medium-sized inputs such as a single `DerivationTree`.
                packages.extend(derived.terms.keys());
                packages.extend(derived.cause1.packages().iter());
                packages.extend(derived.cause2.packages().iter());
            }
        }
        packages
    }

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
                match (
                    Arc::make_mut(&mut derived.cause1),
                    Arc::make_mut(&mut derived.cause2),
                ) {
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
                        Arc::make_mut(&mut derived.cause1).collapse_no_versions();
                        Arc::make_mut(&mut derived.cause2).collapse_no_versions();
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
            //
            // Cannot be merged because the reason may not match
            DerivationTree::External(External::NoVersions(_, _)) => None,
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
            // Cannot be merged because the reason may not match
            DerivationTree::External(External::Custom(_, _, _)) => None,
        }
    }
}

impl<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> Display for External<P, VS, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotRoot(package, version) => {
                write!(f, "we are solving dependencies of {package} {version}")
            }
            Self::NoVersions(package, set) => {
                if set == &VS::full() {
                    write!(f, "there is no available version for {package}")
                } else {
                    write!(f, "there is no version of {package} in {set}")
                }
            }
            Self::Custom(package, set, metadata) => {
                if set == &VS::full() {
                    write!(f, "dependencies of {package} are unavailable {metadata}")
                } else {
                    write!(
                        f,
                        "dependencies of {package} at version {set} are unavailable {metadata}"
                    )
                }
            }
            Self::FromDependencyOf(p, set_p, dep, set_dep) => {
                if set_p == &VS::full() && set_dep == &VS::full() {
                    write!(f, "{p} depends on {dep}")
                } else if set_p == &VS::full() {
                    write!(f, "{p} depends on {dep} {set_dep}")
                } else if set_dep == &VS::full() {
                    write!(f, "{p} {set_p} depends on {dep}")
                } else {
                    write!(f, "{p} {set_p} depends on {dep} {set_dep}")
                }
            }
        }
    }
}

/// Trait for formatting outputs in the reporter.
pub trait ReportFormatter<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> {
    /// Output type of the report.
    type Output;

    /// Format an [External] incompatibility.
    fn format_external(&self, external: &External<P, VS, M>) -> Self::Output;

    /// Format terms of an incompatibility.
    fn format_terms(&self, terms: &Map<P, Term<VS>>) -> Self::Output;

    /// Simplest case, we just combine two external incompatibilities.
    fn explain_both_external(
        &self,
        external1: &External<P, VS, M>,
        external2: &External<P, VS, M>,
        current_terms: &Map<P, Term<VS>>,
    ) -> Self::Output;

    /// Both causes have already been explained so we use their refs.
    fn explain_both_ref(
        &self,
        ref_id1: usize,
        derived1: &Derived<P, VS, M>,
        ref_id2: usize,
        derived2: &Derived<P, VS, M>,
        current_terms: &Map<P, Term<VS>>,
    ) -> Self::Output;

    /// One cause is derived (already explained so one-line),
    /// the other is a one-line external cause,
    /// and finally we conclude with the current incompatibility.
    fn explain_ref_and_external(
        &self,
        ref_id: usize,
        derived: &Derived<P, VS, M>,
        external: &External<P, VS, M>,
        current_terms: &Map<P, Term<VS>>,
    ) -> Self::Output;

    /// Add an external cause to the chain of explanations.
    fn and_explain_external(
        &self,
        external: &External<P, VS, M>,
        current_terms: &Map<P, Term<VS>>,
    ) -> Self::Output;

    /// Add an already explained incompat to the chain of explanations.
    fn and_explain_ref(
        &self,
        ref_id: usize,
        derived: &Derived<P, VS, M>,
        current_terms: &Map<P, Term<VS>>,
    ) -> Self::Output;

    /// Add an already explained incompat to the chain of explanations.
    fn and_explain_prior_and_external(
        &self,
        prior_external: &External<P, VS, M>,
        external: &External<P, VS, M>,
        current_terms: &Map<P, Term<VS>>,
    ) -> Self::Output;
}

/// Default formatter for the default reporter.
#[derive(Default, Debug)]
pub struct DefaultStringReportFormatter;

impl<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> ReportFormatter<P, VS, M>
    for DefaultStringReportFormatter
{
    type Output = String;

    fn format_external(&self, external: &External<P, VS, M>) -> String {
        external.to_string()
    }

    fn format_terms(&self, terms: &Map<P, Term<VS>>) -> Self::Output {
        let terms_vec: Vec<_> = terms.iter().collect();
        match terms_vec.as_slice() {
            [] => "version solving failed".into(),
            // TODO: special case when that unique package is root.
            [(package, Term::Positive(range))] => format!("{package} {range} is forbidden"),
            [(package, Term::Negative(range))] => format!("{package} {range} is mandatory"),
            [(p1, Term::Positive(r1)), (p2, Term::Negative(r2))] => self.format_external(
                &External::<_, _, M>::FromDependencyOf(p1, r1.clone(), p2, r2.clone()),
            ),
            [(p1, Term::Negative(r1)), (p2, Term::Positive(r2))] => self.format_external(
                &External::<_, _, M>::FromDependencyOf(p2, r2.clone(), p1, r1.clone()),
            ),
            slice => {
                let str_terms: Vec<_> = slice.iter().map(|(p, t)| format!("{p} {t}")).collect();
                str_terms.join(", ") + " are incompatible"
            }
        }
    }

    /// Simplest case, we just combine two external incompatibilities.
    fn explain_both_external(
        &self,
        external1: &External<P, VS, M>,
        external2: &External<P, VS, M>,
        current_terms: &Map<P, Term<VS>>,
    ) -> String {
        // TODO: order should be chosen to make it more logical.
        format!(
            "Because {} and {}, {}.",
            self.format_external(external1),
            self.format_external(external2),
            ReportFormatter::<P, VS, M>::format_terms(self, current_terms)
        )
    }

    /// Both causes have already been explained so we use their refs.
    fn explain_both_ref(
        &self,
        ref_id1: usize,
        derived1: &Derived<P, VS, M>,
        ref_id2: usize,
        derived2: &Derived<P, VS, M>,
        current_terms: &Map<P, Term<VS>>,
    ) -> String {
        // TODO: order should be chosen to make it more logical.
        format!(
            "Because {} ({}) and {} ({}), {}.",
            ReportFormatter::<P, VS, M>::format_terms(self, &derived1.terms),
            ref_id1,
            ReportFormatter::<P, VS, M>::format_terms(self, &derived2.terms),
            ref_id2,
            ReportFormatter::<P, VS, M>::format_terms(self, current_terms)
        )
    }

    /// One cause is derived (already explained so one-line),
    /// the other is a one-line external cause,
    /// and finally we conclude with the current incompatibility.
    fn explain_ref_and_external(
        &self,
        ref_id: usize,
        derived: &Derived<P, VS, M>,
        external: &External<P, VS, M>,
        current_terms: &Map<P, Term<VS>>,
    ) -> String {
        // TODO: order should be chosen to make it more logical.
        format!(
            "Because {} ({}) and {}, {}.",
            ReportFormatter::<P, VS, M>::format_terms(self, &derived.terms),
            ref_id,
            self.format_external(external),
            ReportFormatter::<P, VS, M>::format_terms(self, current_terms)
        )
    }

    /// Add an external cause to the chain of explanations.
    fn and_explain_external(
        &self,
        external: &External<P, VS, M>,
        current_terms: &Map<P, Term<VS>>,
    ) -> String {
        format!(
            "And because {}, {}.",
            self.format_external(external),
            ReportFormatter::<P, VS, M>::format_terms(self, current_terms)
        )
    }

    /// Add an already explained incompat to the chain of explanations.
    fn and_explain_ref(
        &self,
        ref_id: usize,
        derived: &Derived<P, VS, M>,
        current_terms: &Map<P, Term<VS>>,
    ) -> String {
        format!(
            "And because {} ({}), {}.",
            ReportFormatter::<P, VS, M>::format_terms(self, &derived.terms),
            ref_id,
            ReportFormatter::<P, VS, M>::format_terms(self, current_terms)
        )
    }

    /// Add an already explained incompat to the chain of explanations.
    fn and_explain_prior_and_external(
        &self,
        prior_external: &External<P, VS, M>,
        external: &External<P, VS, M>,
        current_terms: &Map<P, Term<VS>>,
    ) -> String {
        format!(
            "And because {} and {}, {}.",
            self.format_external(prior_external),
            self.format_external(external),
            ReportFormatter::<P, VS, M>::format_terms(self, current_terms)
        )
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
    fn new() -> Self {
        Self {
            ref_count: 0,
            shared_with_ref: Map::default(),
            lines: Vec::new(),
        }
    }

    fn build_recursive<
        P: Package,
        VS: VersionSet,
        M: Eq + Clone + Debug + Display,
        F: ReportFormatter<P, VS, M, Output = String>,
    >(
        &mut self,
        derived: &Derived<P, VS, M>,
        formatter: &F,
    ) {
        self.build_recursive_helper(derived, formatter);
        if let Some(id) = derived.shared_id {
            #[allow(clippy::map_entry)] // `add_line_ref` not compatible with proposed fix.
            if !self.shared_with_ref.contains_key(&id) {
                self.add_line_ref();
                self.shared_with_ref.insert(id, self.ref_count);
            }
        };
    }

    fn build_recursive_helper<
        P: Package,
        VS: VersionSet,
        M: Eq + Clone + Debug + Display,
        F: ReportFormatter<P, VS, M, Output = String>,
    >(
        &mut self,
        current: &Derived<P, VS, M>,
        formatter: &F,
    ) {
        match (current.cause1.deref(), current.cause2.deref()) {
            (DerivationTree::External(external1), DerivationTree::External(external2)) => {
                // Simplest case, we just combine two external incompatibilities.
                self.lines.push(formatter.explain_both_external(
                    external1,
                    external2,
                    &current.terms,
                ));
            }
            (DerivationTree::Derived(derived), DerivationTree::External(external)) => {
                // One cause is derived, so we explain this first
                // then we add the one-line external part
                // and finally conclude with the current incompatibility.
                self.report_one_each(derived, external, &current.terms, formatter);
            }
            (DerivationTree::External(external), DerivationTree::Derived(derived)) => {
                self.report_one_each(derived, external, &current.terms, formatter);
            }
            (DerivationTree::Derived(derived1), DerivationTree::Derived(derived2)) => {
                // This is the most complex case since both causes are also derived.
                match (
                    self.line_ref_of(derived1.shared_id),
                    self.line_ref_of(derived2.shared_id),
                ) {
                    // If both causes already have been referenced (shared_id),
                    // the explanation simply uses those references.
                    (Some(ref1), Some(ref2)) => self.lines.push(formatter.explain_both_ref(
                        ref1,
                        derived1,
                        ref2,
                        derived2,
                        &current.terms,
                    )),
                    // Otherwise, if one only has a line number reference,
                    // we recursively call the one without reference and then
                    // add the one with reference to conclude.
                    (Some(ref1), None) => {
                        self.build_recursive(derived2, formatter);
                        self.lines
                            .push(formatter.and_explain_ref(ref1, derived1, &current.terms));
                    }
                    (None, Some(ref2)) => {
                        self.build_recursive(derived1, formatter);
                        self.lines
                            .push(formatter.and_explain_ref(ref2, derived2, &current.terms));
                    }
                    // Finally, if no line reference exists yet,
                    // we call recursively the first one and then,
                    //   - if this was a shared node, it will get a line ref
                    //     and we can simply recall this with the current node.
                    //   - otherwise, we add a line reference to it,
                    //     recursively call on the second node,
                    //     and finally conclude.
                    (None, None) => {
                        self.build_recursive(derived1, formatter);
                        if derived1.shared_id.is_some() {
                            self.lines.push("".into());
                            self.build_recursive(current, formatter);
                        } else {
                            self.add_line_ref();
                            let ref1 = self.ref_count;
                            self.lines.push("".into());
                            self.build_recursive(derived2, formatter);
                            self.lines.push(formatter.and_explain_ref(
                                ref1,
                                derived1,
                                &current.terms,
                            ));
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
    fn report_one_each<
        P: Package,
        VS: VersionSet,
        M: Eq + Clone + Debug + Display,
        F: ReportFormatter<P, VS, M, Output = String>,
    >(
        &mut self,
        derived: &Derived<P, VS, M>,
        external: &External<P, VS, M>,
        current_terms: &Map<P, Term<VS>>,
        formatter: &F,
    ) {
        match self.line_ref_of(derived.shared_id) {
            Some(ref_id) => self.lines.push(formatter.explain_ref_and_external(
                ref_id,
                derived,
                external,
                current_terms,
            )),
            None => self.report_recurse_one_each(derived, external, current_terms, formatter),
        }
    }

    /// Report one derived (without a line ref yet) and one external.
    fn report_recurse_one_each<
        P: Package,
        VS: VersionSet,
        M: Eq + Clone + Debug + Display,
        F: ReportFormatter<P, VS, M, Output = String>,
    >(
        &mut self,
        derived: &Derived<P, VS, M>,
        external: &External<P, VS, M>,
        current_terms: &Map<P, Term<VS>>,
        formatter: &F,
    ) {
        match (derived.cause1.deref(), derived.cause2.deref()) {
            // If the derived cause has itself one external prior cause,
            // we can chain the external explanations.
            (DerivationTree::Derived(prior_derived), DerivationTree::External(prior_external)) => {
                self.build_recursive(prior_derived, formatter);
                self.lines.push(formatter.and_explain_prior_and_external(
                    prior_external,
                    external,
                    current_terms,
                ));
            }
            // If the derived cause has itself one external prior cause,
            // we can chain the external explanations.
            (DerivationTree::External(prior_external), DerivationTree::Derived(prior_derived)) => {
                self.build_recursive(prior_derived, formatter);
                self.lines.push(formatter.and_explain_prior_and_external(
                    prior_external,
                    external,
                    current_terms,
                ));
            }
            _ => {
                self.build_recursive(derived, formatter);
                self.lines
                    .push(formatter.and_explain_external(external, current_terms));
            }
        }
    }

    // Helper functions ########################################################

    fn add_line_ref(&mut self) {
        let new_count = self.ref_count + 1;
        self.ref_count = new_count;
        if let Some(line) = self.lines.last_mut() {
            *line = format!("{line} ({new_count})");
        }
    }

    fn line_ref_of(&self, shared_id: Option<usize>) -> Option<usize> {
        shared_id.and_then(|id| self.shared_with_ref.get(&id).cloned())
    }
}

impl<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> Reporter<P, VS, M>
    for DefaultStringReporter
{
    type Output = String;

    fn report(derivation_tree: &DerivationTree<P, VS, M>) -> Self::Output {
        let formatter = DefaultStringReportFormatter;
        match derivation_tree {
            DerivationTree::External(external) => formatter.format_external(external),
            DerivationTree::Derived(derived) => {
                let mut reporter = Self::new();
                reporter.build_recursive(derived, &formatter);
                reporter.lines.join("\n")
            }
        }
    }

    fn report_with_formatter(
        derivation_tree: &DerivationTree<P, VS, M>,
        formatter: &impl ReportFormatter<P, VS, M, Output = Self::Output>,
    ) -> Self::Output {
        match derivation_tree {
            DerivationTree::External(external) => formatter.format_external(external),
            DerivationTree::Derived(derived) => {
                let mut reporter = Self::new();
                reporter.build_recursive(derived, formatter);
                reporter.lines.join("\n")
            }
        }
    }
}
