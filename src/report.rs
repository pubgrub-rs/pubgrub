// SPDX-License-Identifier: MPL-2.0

//! Build a report as clear as possible as to why
//! dependency solving failed.

use std::fmt::{self, Debug, Display};
use std::ops::Deref;
use std::sync::Arc;

use crate::{
    DependencyProvider, Map, NoSolutionError, PackageArena, PackageId, Set, Term, VersionIndex,
    VersionSet,
};

/// Reporter trait.
pub trait Reporter<DP: DependencyProvider> {
    /// Output type of the report.
    type Output;

    /// Generate a report from the error describing the resolution failure using the default formatter.
    fn report(error: &NoSolutionError<DP>, dependency_provider: &DP) -> Self::Output;

    /// Generate a report from the error describing the resolution failure using a custom formatter.
    fn report_with_formatter(
        error: &NoSolutionError<DP>,
        formatter: &impl ReportFormatter<DP, Output = Self::Output>,
        dependency_provider: &DP,
    ) -> Self::Output;
}

/// Derivation tree resulting in the impossibility
/// to solve the dependencies of our root package.
#[derive(Debug, Clone)]
pub enum DerivationTree<M: Eq + Clone + Debug + Display> {
    /// External incompatibility.
    External(External<M>),
    /// Incompatibility derived from two others.
    Derived(Derived<M>),
}

/// Incompatibilities that are not derived from others,
/// they have their own reason.
#[derive(Debug, Clone)]
pub enum External<M: Eq + Clone + Debug + Display> {
    /// Initial incompatibility aiming at picking the root package for the first decision.
    NotRoot(PackageId, VersionIndex),
    /// There are no versions in the given set for this package.
    NoVersions(PackageId, VersionSet),
    /// Incompatibility coming from the dependencies of a given package.
    FromDependencyOf(PackageId, VersionSet, PackageId, VersionSet),
    /// The package is unusable for reasons outside pubgrub.
    Custom(PackageId, VersionSet, M),
}

/// Incompatibility derived from two others.
#[derive(Debug, Clone)]
pub struct Derived<M: Eq + Clone + Debug + Display> {
    /// Terms of the incompatibility.
    pub terms: Map<PackageId, Term>,
    /// Indicate if that incompatibility is present multiple times
    /// in the derivation tree.
    /// If that is the case, it has a unique id, provided in that option.
    /// Then, we may want to only explain it once,
    /// and refer to the explanation for the other times.
    pub shared_id: Option<usize>,
    /// First cause.
    pub cause1: Arc<DerivationTree<M>>,
    /// Second cause.
    pub cause2: Arc<DerivationTree<M>>,
}

impl<M: Eq + Clone + Debug + Display> DerivationTree<M> {
    /// Get all packages referred to in the derivation tree.
    pub fn packages(&self) -> Set<PackageId> {
        let mut packages = Set::default();
        match self {
            Self::External(external) => match *external {
                External::FromDependencyOf(pid1, _, pid2, _) => {
                    packages.insert(pid1);
                    packages.insert(pid2);
                }
                External::NoVersions(pid, _)
                | External::NotRoot(pid, _)
                | External::Custom(pid, _, _) => {
                    packages.insert(pid);
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

    /// Merge the [`NoVersions`](External::NoVersions) external incompatibilities
    /// with the other one they are matched with
    /// in a derived incompatibility.
    /// This cleans up quite nicely the generated report.
    /// You might want to do this if you know that the [`DependencyProvider`]
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
                    (DerivationTree::External(External::NoVersions(pid, r)), cause2) => {
                        cause2.collapse_no_versions();
                        *self = cause2
                            .clone()
                            .merge_no_versions(pid.to_owned(), r.to_owned())
                            .unwrap_or_else(|| self.to_owned());
                    }
                    (cause1, DerivationTree::External(External::NoVersions(pid, r))) => {
                        cause1.collapse_no_versions();
                        *self = cause1
                            .clone()
                            .merge_no_versions(pid.to_owned(), r.to_owned())
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

    fn merge_no_versions(self, package_id: PackageId, set: VersionSet) -> Option<Self> {
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
            DerivationTree::External(External::FromDependencyOf(pid1, r1, pid2, r2)) => {
                if pid1 == package_id {
                    Some(DerivationTree::External(External::FromDependencyOf(
                        pid1,
                        r1.union(set),
                        pid2,
                        r2,
                    )))
                } else {
                    Some(DerivationTree::External(External::FromDependencyOf(
                        pid1,
                        r1,
                        pid2,
                        r2.union(set),
                    )))
                }
            }
            // Cannot be merged because the reason may not match
            DerivationTree::External(External::Custom(_, _, _)) => None,
        }
    }
}

impl<M: Eq + Clone + Debug + Display> External<M> {
    /// Returns an object implementing `Display` for this `External`.
    pub fn display<'a, DP: DependencyProvider<M = M>>(
        &'a self,
        package_store: &'a PackageArena<DP::P>,
        dependency_provider: &'a DP,
    ) -> ExternalDisplay<'a, DP> {
        ExternalDisplay {
            external: self,
            package_store,
            dependency_provider,
        }
    }
}

pub struct ExternalDisplay<'a, DP: DependencyProvider> {
    external: &'a External<DP::M>,
    package_store: &'a PackageArena<DP::P>,
    dependency_provider: &'a DP,
}

impl<DP: DependencyProvider> Display for ExternalDisplay<'_, DP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self.external {
            External::NotRoot(package_id, version_index) => {
                let p = self.package_store.pkg(package_id).unwrap();
                write!(
                    f,
                    "we are solving dependencies of {}",
                    self.dependency_provider
                        .package_version_display(p, version_index),
                )
            }
            External::NoVersions(package_id, set) => {
                let p = self.package_store.pkg(package_id).unwrap();
                if set == VersionSet::full() {
                    write!(f, "there is no available version for {p}")
                } else {
                    write!(
                        f,
                        "there is no version of {}",
                        self.dependency_provider.package_version_set_display(p, set),
                    )
                }
            }
            External::Custom(package_id, set, ref metadata) => {
                let p = self.package_store.pkg(package_id).unwrap();
                if set == VersionSet::full() {
                    write!(f, "dependencies of {p} are unavailable ({metadata})")
                } else {
                    write!(
                        f,
                        "dependencies of {} are unavailable ({metadata})",
                        self.dependency_provider.package_version_set_display(p, set),
                    )
                }
            }
            External::FromDependencyOf(pid, set_p, did, set_dep) => {
                let p = self.package_store.pkg(pid).unwrap();
                let d = self.package_store.pkg(did).unwrap();
                if set_p == VersionSet::full() && set_dep == VersionSet::full() {
                    write!(f, "{p} depends on {d}")
                } else if set_p == VersionSet::full() {
                    write!(
                        f,
                        "{p} depends on {}",
                        self.dependency_provider
                            .package_version_set_display(d, set_dep),
                    )
                } else if set_dep == VersionSet::full() {
                    write!(
                        f,
                        "{} depends on {d}",
                        self.dependency_provider
                            .package_version_set_display(p, set_p),
                    )
                } else {
                    write!(
                        f,
                        "{} depends on {}",
                        self.dependency_provider
                            .package_version_set_display(p, set_p),
                        self.dependency_provider
                            .package_version_set_display(d, set_dep),
                    )
                }
            }
        }
    }
}

/// Trait for formatting outputs in the reporter.
pub trait ReportFormatter<DP: DependencyProvider> {
    /// Output type of the report.
    type Output;

    /// Format an [External] incompatibility.
    fn format_external(
        &self,
        external: &External<DP::M>,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) -> Self::Output;

    /// Format terms of an incompatibility.
    fn format_terms(
        &self,
        terms: &Map<PackageId, Term>,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) -> Self::Output;

    /// Simplest case, we just combine two external incompatibilities.
    fn explain_both_external(
        &self,
        external1: &External<DP::M>,
        external2: &External<DP::M>,
        current_terms: &Map<PackageId, Term>,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) -> Self::Output;

    /// Both causes have already been explained so we use their refs.
    #[allow(clippy::too_many_arguments)]
    fn explain_both_ref(
        &self,
        ref_id1: usize,
        derived1: &Derived<DP::M>,
        ref_id2: usize,
        derived2: &Derived<DP::M>,
        current_terms: &Map<PackageId, Term>,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) -> Self::Output;

    /// One cause is derived (already explained so one-line),
    /// the other is a one-line external cause,
    /// and finally we conclude with the current incompatibility.
    fn explain_ref_and_external(
        &self,
        ref_id: usize,
        derived: &Derived<DP::M>,
        external: &External<DP::M>,
        current_terms: &Map<PackageId, Term>,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) -> Self::Output;

    /// Add an external cause to the chain of explanations.
    fn and_explain_external(
        &self,
        external: &External<DP::M>,
        current_terms: &Map<PackageId, Term>,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) -> Self::Output;

    /// Add an already explained incompat to the chain of explanations.
    fn and_explain_ref(
        &self,
        ref_id: usize,
        derived: &Derived<DP::M>,
        current_terms: &Map<PackageId, Term>,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) -> Self::Output;

    /// Add an already explained incompat to the chain of explanations.
    fn and_explain_prior_and_external(
        &self,
        prior_external: &External<DP::M>,
        external: &External<DP::M>,
        current_terms: &Map<PackageId, Term>,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) -> Self::Output;
}

/// Default formatter for the default reporter.
#[derive(Default, Debug)]
pub struct DefaultStringReportFormatter;

impl<DP: DependencyProvider> ReportFormatter<DP> for DefaultStringReportFormatter {
    type Output = String;

    fn format_external(
        &self,
        external: &External<DP::M>,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) -> String {
        external
            .display(package_store, dependency_provider)
            .to_string()
    }

    fn format_terms(
        &self,
        terms: &Map<PackageId, Term>,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) -> Self::Output {
        let terms_vec: Vec<_> = terms.iter().map(|(&pid, &t)| (pid, t)).collect();
        match *terms_vec.as_slice() {
            [] => "version solving failed".into(),
            // TODO: special case when that unique package is root.
            [(pid, term)] => {
                let pkg = package_store.pkg(pid).unwrap();
                if term.is_positive() {
                    format!(
                        "{} is forbidden",
                        dependency_provider.package_version_set_display(pkg, term.version_set()),
                    )
                } else {
                    format!(
                        "{} is mandatory",
                        dependency_provider.package_version_set_display(pkg, term.version_set()),
                    )
                }
            }
            [(pid1, t1), (pid2, t2)] if t1.is_positive() && t2.is_negative() => {
                let r1 = t1.version_set();
                let r2 = t2.version_set();
                self.format_external(
                    &External::FromDependencyOf(pid1, r1, pid2, r2),
                    package_store,
                    dependency_provider,
                )
            }
            [(pid1, t1), (pid2, t2)] if t1.is_negative() && t2.is_positive() => {
                let r1 = t1.version_set();
                let r2 = t2.version_set();
                self.format_external(
                    &External::FromDependencyOf(pid2, r2, pid1, r1),
                    package_store,
                    dependency_provider,
                )
            }
            ref slice => {
                let str_terms: Vec<_> = slice
                    .iter()
                    .map(|&(pid, t)| {
                        let pvs = dependency_provider.package_version_set_display(
                            package_store.pkg(pid).unwrap(),
                            t.version_set(),
                        );
                        if t.is_positive() {
                            format!("{pvs}")
                        } else {
                            format!("Not ( {pvs} )")
                        }
                    })
                    .collect();
                str_terms.join(", ") + " are incompatible"
            }
        }
    }

    /// Simplest case, we just combine two external incompatibilities.
    fn explain_both_external(
        &self,
        external1: &External<DP::M>,
        external2: &External<DP::M>,
        current_terms: &Map<PackageId, Term>,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) -> String {
        // TODO: order should be chosen to make it more logical.
        format!(
            "Because {} and {}, {}.",
            self.format_external(external1, package_store, dependency_provider),
            self.format_external(external2, package_store, dependency_provider),
            ReportFormatter::<DP>::format_terms(
                self,
                current_terms,
                package_store,
                dependency_provider
            )
        )
    }

    /// Both causes have already been explained so we use their refs.
    fn explain_both_ref(
        &self,
        ref_id1: usize,
        derived1: &Derived<DP::M>,
        ref_id2: usize,
        derived2: &Derived<DP::M>,
        current_terms: &Map<PackageId, Term>,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) -> String {
        // TODO: order should be chosen to make it more logical.
        format!(
            "Because {} ({}) and {} ({}), {}.",
            ReportFormatter::<DP>::format_terms(
                self,
                &derived1.terms,
                package_store,
                dependency_provider
            ),
            ref_id1,
            ReportFormatter::<DP>::format_terms(
                self,
                &derived2.terms,
                package_store,
                dependency_provider
            ),
            ref_id2,
            ReportFormatter::<DP>::format_terms(
                self,
                current_terms,
                package_store,
                dependency_provider
            )
        )
    }

    /// One cause is derived (already explained so one-line),
    /// the other is a one-line external cause,
    /// and finally we conclude with the current incompatibility.
    fn explain_ref_and_external(
        &self,
        ref_id: usize,
        derived: &Derived<DP::M>,
        external: &External<DP::M>,
        current_terms: &Map<PackageId, Term>,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) -> String {
        // TODO: order should be chosen to make it more logical.
        format!(
            "Because {} ({}) and {}, {}.",
            ReportFormatter::<DP>::format_terms(
                self,
                &derived.terms,
                package_store,
                dependency_provider
            ),
            ref_id,
            self.format_external(external, package_store, dependency_provider),
            ReportFormatter::<DP>::format_terms(
                self,
                current_terms,
                package_store,
                dependency_provider
            )
        )
    }

    /// Add an external cause to the chain of explanations.
    fn and_explain_external(
        &self,
        external: &External<DP::M>,
        current_terms: &Map<PackageId, Term>,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) -> String {
        format!(
            "And because {}, {}.",
            self.format_external(external, package_store, dependency_provider),
            ReportFormatter::<DP>::format_terms(
                self,
                current_terms,
                package_store,
                dependency_provider
            )
        )
    }

    /// Add an already explained incompat to the chain of explanations.
    fn and_explain_ref(
        &self,
        ref_id: usize,
        derived: &Derived<DP::M>,
        current_terms: &Map<PackageId, Term>,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) -> String {
        format!(
            "And because {} ({}), {}.",
            ReportFormatter::<DP>::format_terms(
                self,
                &derived.terms,
                package_store,
                dependency_provider
            ),
            ref_id,
            ReportFormatter::<DP>::format_terms(
                self,
                current_terms,
                package_store,
                dependency_provider
            )
        )
    }

    /// Add an already explained incompat to the chain of explanations.
    fn and_explain_prior_and_external(
        &self,
        prior_external: &External<DP::M>,
        external: &External<DP::M>,
        current_terms: &Map<PackageId, Term>,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) -> String {
        format!(
            "And because {} and {}, {}.",
            self.format_external(prior_external, package_store, dependency_provider),
            self.format_external(external, package_store, dependency_provider),
            ReportFormatter::<DP>::format_terms(
                self,
                current_terms,
                package_store,
                dependency_provider
            )
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

    fn build_recursive<DP: DependencyProvider, F: ReportFormatter<DP, Output = String>>(
        &mut self,
        derived: &Derived<DP::M>,
        formatter: &F,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) {
        self.build_recursive_helper(derived, formatter, package_store, dependency_provider);
        if let Some(id) = derived.shared_id {
            #[allow(clippy::map_entry)] // `add_line_ref` not compatible with proposed fix.
            if !self.shared_with_ref.contains_key(&id) {
                self.add_line_ref();
                self.shared_with_ref.insert(id, self.ref_count);
            }
        };
    }

    fn build_recursive_helper<DP: DependencyProvider, F: ReportFormatter<DP, Output = String>>(
        &mut self,
        current: &Derived<DP::M>,
        formatter: &F,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) {
        match (current.cause1.deref(), current.cause2.deref()) {
            (DerivationTree::External(external1), DerivationTree::External(external2)) => {
                // Simplest case, we just combine two external incompatibilities.
                self.lines.push(formatter.explain_both_external(
                    external1,
                    external2,
                    &current.terms,
                    package_store,
                    dependency_provider,
                ));
            }
            (DerivationTree::Derived(derived), DerivationTree::External(external)) => {
                // One cause is derived, so we explain this first
                // then we add the one-line external part
                // and finally conclude with the current incompatibility.
                self.report_one_each(
                    derived,
                    external,
                    &current.terms,
                    formatter,
                    package_store,
                    dependency_provider,
                );
            }
            (DerivationTree::External(external), DerivationTree::Derived(derived)) => {
                self.report_one_each(
                    derived,
                    external,
                    &current.terms,
                    formatter,
                    package_store,
                    dependency_provider,
                );
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
                        package_store,
                        dependency_provider,
                    )),
                    // Otherwise, if one only has a line number reference,
                    // we recursively call the one without reference and then
                    // add the one with reference to conclude.
                    (Some(ref1), None) => {
                        self.build_recursive(
                            derived2,
                            formatter,
                            package_store,
                            dependency_provider,
                        );
                        self.lines.push(formatter.and_explain_ref(
                            ref1,
                            derived1,
                            &current.terms,
                            package_store,
                            dependency_provider,
                        ));
                    }
                    (None, Some(ref2)) => {
                        self.build_recursive(
                            derived1,
                            formatter,
                            package_store,
                            dependency_provider,
                        );
                        self.lines.push(formatter.and_explain_ref(
                            ref2,
                            derived2,
                            &current.terms,
                            package_store,
                            dependency_provider,
                        ));
                    }
                    // Finally, if no line reference exists yet,
                    // we call recursively the first one and then,
                    //   - if this was a shared node, it will get a line ref
                    //     and we can simply recall this with the current node.
                    //   - otherwise, we add a line reference to it,
                    //     recursively call on the second node,
                    //     and finally conclude.
                    (None, None) => {
                        self.build_recursive(
                            derived1,
                            formatter,
                            package_store,
                            dependency_provider,
                        );
                        if derived1.shared_id.is_some() {
                            self.lines.push("".into());
                            self.build_recursive(
                                current,
                                formatter,
                                package_store,
                                dependency_provider,
                            );
                        } else {
                            self.add_line_ref();
                            let ref1 = self.ref_count;
                            self.lines.push("".into());
                            self.build_recursive(
                                derived2,
                                formatter,
                                package_store,
                                dependency_provider,
                            );
                            self.lines.push(formatter.and_explain_ref(
                                ref1,
                                derived1,
                                &current.terms,
                                package_store,
                                dependency_provider,
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
    fn report_one_each<DP: DependencyProvider, F: ReportFormatter<DP, Output = String>>(
        &mut self,
        derived: &Derived<DP::M>,
        external: &External<DP::M>,
        current_terms: &Map<PackageId, Term>,
        formatter: &F,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) {
        match self.line_ref_of(derived.shared_id) {
            Some(ref_id) => self.lines.push(formatter.explain_ref_and_external(
                ref_id,
                derived,
                external,
                current_terms,
                package_store,
                dependency_provider,
            )),
            None => self.report_recurse_one_each(
                derived,
                external,
                current_terms,
                formatter,
                package_store,
                dependency_provider,
            ),
        }
    }

    /// Report one derived (without a line ref yet) and one external.
    fn report_recurse_one_each<DP: DependencyProvider, F: ReportFormatter<DP, Output = String>>(
        &mut self,
        derived: &Derived<DP::M>,
        external: &External<DP::M>,
        current_terms: &Map<PackageId, Term>,
        formatter: &F,
        package_store: &PackageArena<DP::P>,
        dependency_provider: &DP,
    ) {
        match (derived.cause1.deref(), derived.cause2.deref()) {
            // If the derived cause has itself one external prior cause,
            // we can chain the external explanations.
            (DerivationTree::Derived(prior_derived), DerivationTree::External(prior_external)) => {
                self.build_recursive(prior_derived, formatter, package_store, dependency_provider);
                self.lines.push(formatter.and_explain_prior_and_external(
                    prior_external,
                    external,
                    current_terms,
                    package_store,
                    dependency_provider,
                ));
            }
            // If the derived cause has itself one external prior cause,
            // we can chain the external explanations.
            (DerivationTree::External(prior_external), DerivationTree::Derived(prior_derived)) => {
                self.build_recursive(prior_derived, formatter, package_store, dependency_provider);
                self.lines.push(formatter.and_explain_prior_and_external(
                    prior_external,
                    external,
                    current_terms,
                    package_store,
                    dependency_provider,
                ));
            }
            _ => {
                self.build_recursive(derived, formatter, package_store, dependency_provider);
                self.lines.push(formatter.and_explain_external(
                    external,
                    current_terms,
                    package_store,
                    dependency_provider,
                ));
            }
        }
    }

    // Helper functions ########################################################

    fn add_line_ref(&mut self) {
        let new_count = self.ref_count + 1;
        self.ref_count = new_count;
        if let Some(line) = self.lines.last_mut() {
            *line = format!("{} ({})", line, new_count);
        }
    }

    fn line_ref_of(&self, shared_id: Option<usize>) -> Option<usize> {
        shared_id.and_then(|id| self.shared_with_ref.get(&id).cloned())
    }
}

impl<DP: DependencyProvider> Reporter<DP> for DefaultStringReporter {
    type Output = String;

    fn report(error: &NoSolutionError<DP>, dependency_provider: &DP) -> Self::Output {
        let formatter = DefaultStringReportFormatter;
        match &error.derivation_tree {
            DerivationTree::External(external) => {
                formatter.format_external(external, &error.package_store, dependency_provider)
            }
            DerivationTree::Derived(derived) => {
                let mut reporter = Self::new();
                reporter.build_recursive(
                    derived,
                    &formatter,
                    &error.package_store,
                    dependency_provider,
                );
                reporter.lines.join("\n")
            }
        }
    }

    fn report_with_formatter(
        error: &NoSolutionError<DP>,
        formatter: &impl ReportFormatter<DP, Output = Self::Output>,
        dependency_provider: &DP,
    ) -> Self::Output {
        match &error.derivation_tree {
            DerivationTree::External(external) => {
                formatter.format_external(external, &error.package_store, dependency_provider)
            }
            DerivationTree::Derived(derived) => {
                let mut reporter = Self::new();
                reporter.build_recursive(
                    derived,
                    formatter,
                    &error.package_store,
                    dependency_provider,
                );
                reporter.lines.join("\n")
            }
        }
    }
}
