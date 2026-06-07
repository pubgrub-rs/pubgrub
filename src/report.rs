// SPDX-License-Identifier: MPL-2.0

//! Build a report as clear as possible as to why
//! dependency solving failed.

use std::fmt::{self, Debug, Display};
use std::marker::PhantomData;

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
///
/// The tree is stored as an arena of nodes. Causes of derived incompatibilities are represented as
/// node ids instead of recursive ownership, so very deep derivation trees can be traversed and
/// dropped without recursively consuming stack.
#[derive(Debug, Clone)]
pub struct DerivationTree<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> {
    arena: Vec<DerivationTreeNode<P, VS, M>>,
    root: DerivationTreeId,
}

/// Identifier of a node in a [DerivationTree].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DerivationTreeId(usize);

/// A node in a [DerivationTree].
#[derive(Debug, Clone)]
pub enum DerivationTreeNode<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> {
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
    pub cause1: DerivationTreeId,
    /// Second cause.
    pub cause2: DerivationTreeId,
    _metadata: PhantomData<M>,
}

impl<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> Derived<P, VS, M> {
    pub(crate) fn new(
        terms: Map<P, Term<VS>>,
        shared_id: Option<usize>,
        cause1: DerivationTreeId,
        cause2: DerivationTreeId,
    ) -> Self {
        Self {
            terms,
            shared_id,
            cause1,
            cause2,
            _metadata: PhantomData,
        }
    }
}

impl DerivationTreeId {
    pub(crate) fn from_raw(raw: usize) -> Self {
        Self(raw)
    }
}

impl<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> DerivationTree<P, VS, M> {
    pub(crate) fn from_arena(
        arena: Vec<DerivationTreeNode<P, VS, M>>,
        root: DerivationTreeId,
    ) -> Self {
        debug_assert!(root.0 < arena.len());
        Self { arena, root }
    }

    pub(crate) fn alloc_node(
        arena: &mut Vec<DerivationTreeNode<P, VS, M>>,
        node: DerivationTreeNode<P, VS, M>,
    ) -> DerivationTreeId {
        let id = DerivationTreeId::from_raw(arena.len());
        arena.push(node);
        id
    }

    /// Return the id of the root incompatibility.
    pub fn root_id(&self) -> DerivationTreeId {
        self.root
    }

    /// Return the root incompatibility.
    pub fn root(&self) -> &DerivationTreeNode<P, VS, M> {
        self.node(self.root)
    }

    /// Return a node from the derivation tree.
    pub fn node(&self, id: DerivationTreeId) -> &DerivationTreeNode<P, VS, M> {
        &self.arena[id.0]
    }

    fn derived(&self, id: DerivationTreeId) -> &Derived<P, VS, M> {
        match self.node(id) {
            DerivationTreeNode::Derived(derived) => derived,
            DerivationTreeNode::External(_) => panic!("expected derived incompatibility"),
        }
    }

    fn external(&self, id: DerivationTreeId) -> &External<P, VS, M> {
        match self.node(id) {
            DerivationTreeNode::External(external) => external,
            DerivationTreeNode::Derived(_) => panic!("expected external incompatibility"),
        }
    }

    /// Get all packages referred to in the derivation tree.
    pub fn packages(&self) -> Set<&P> {
        let mut packages = Set::default();
        let mut seen = Set::default();
        let mut stack = vec![self.root];
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            match self.node(id) {
                DerivationTreeNode::External(external) => match external {
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
                DerivationTreeNode::Derived(derived) => {
                    packages.extend(derived.terms.keys());
                    stack.push(derived.cause1);
                    stack.push(derived.cause2);
                }
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
        let old_arena = std::mem::take(&mut self.arena);
        let old_root = self.root;
        let mut order = Vec::new();
        let mut seen = Set::default();
        let mut stack = vec![old_root];
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            order.push(id);
            if let DerivationTreeNode::Derived(derived) = &old_arena[id.0] {
                stack.push(derived.cause1);
                stack.push(derived.cause2);
            }
        }

        let mut new_arena = Vec::with_capacity(old_arena.len());
        let mut remapped = Map::default();
        for old_id in order.into_iter().rev() {
            let new_id = match &old_arena[old_id.0] {
                DerivationTreeNode::External(external) => Self::alloc_node(
                    &mut new_arena,
                    DerivationTreeNode::External(external.clone()),
                ),
                DerivationTreeNode::Derived(derived) => {
                    let cause1 = remapped[&derived.cause1];
                    let cause2 = remapped[&derived.cause2];
                    match (&old_arena[derived.cause1.0], &old_arena[derived.cause2.0]) {
                        (DerivationTreeNode::External(External::NoVersions(package, set)), _) => {
                            self.merge_no_versions_id(
                                &mut new_arena,
                                cause2,
                                package.clone(),
                                set.clone(),
                            )
                            .unwrap_or_else(|| {
                                Self::alloc_node(
                                    &mut new_arena,
                                    DerivationTreeNode::Derived(Derived::new(
                                        derived.terms.clone(),
                                        derived.shared_id,
                                        cause1,
                                        cause2,
                                    )),
                                )
                            })
                        }
                        (_, DerivationTreeNode::External(External::NoVersions(package, set))) => {
                            self.merge_no_versions_id(
                                &mut new_arena,
                                cause1,
                                package.clone(),
                                set.clone(),
                            )
                            .unwrap_or_else(|| {
                                Self::alloc_node(
                                    &mut new_arena,
                                    DerivationTreeNode::Derived(Derived::new(
                                        derived.terms.clone(),
                                        derived.shared_id,
                                        cause1,
                                        cause2,
                                    )),
                                )
                            })
                        }
                        _ => Self::alloc_node(
                            &mut new_arena,
                            DerivationTreeNode::Derived(Derived::new(
                                derived.terms.clone(),
                                derived.shared_id,
                                cause1,
                                cause2,
                            )),
                        ),
                    }
                }
            };
            remapped.insert(old_id, new_id);
        }

        self.root = remapped[&old_root];
        self.arena = new_arena;
    }

    fn merge_no_versions_id(
        &self,
        arena: &mut Vec<DerivationTreeNode<P, VS, M>>,
        id: DerivationTreeId,
        package: P,
        set: VS,
    ) -> Option<DerivationTreeId> {
        match &arena[id.0] {
            // TODO: take care of the Derived case.
            // Once done, we can remove the Option.
            DerivationTreeNode::Derived(_) => Some(id),
            DerivationTreeNode::External(External::NotRoot(_, _)) => {
                panic!("How did we end up with a NoVersions merged with a NotRoot?")
            }
            //
            DerivationTreeNode::External(External::NoVersions(_, _)) => None,
            DerivationTreeNode::External(External::FromDependencyOf(p1, r1, p2, r2)) => {
                if p1 == &package {
                    Some(Self::alloc_node(
                        arena,
                        DerivationTreeNode::External(External::FromDependencyOf(
                            p1.clone(),
                            r1.union(&set),
                            p2.clone(),
                            r2.clone(),
                        )),
                    ))
                } else {
                    Some(Self::alloc_node(
                        arena,
                        DerivationTreeNode::External(External::FromDependencyOf(
                            p1.clone(),
                            r1.clone(),
                            p2.clone(),
                            r2.union(&set),
                        )),
                    ))
                }
            }
            // Cannot be merged because the reason may not match
            DerivationTreeNode::External(External::Custom(_, _, _)) => None,
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

#[derive(Clone, Copy)]
enum ReportStep {
    Build(DerivationTreeId),
    BuildHelper(DerivationTreeId),
    AfterBuild(DerivationTreeId),
    AfterFirstNoRefs {
        current: DerivationTreeId,
        derived1: DerivationTreeId,
        derived2: DerivationTreeId,
    },
    AndExplainRef {
        ref_id: usize,
        derived: DerivationTreeId,
        current: DerivationTreeId,
    },
    AndExplainExternal {
        external: DerivationTreeId,
        current: DerivationTreeId,
    },
    AndExplainPriorAndExternal {
        prior_external: DerivationTreeId,
        external: DerivationTreeId,
        current: DerivationTreeId,
    },
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

    fn build<
        P: Package,
        VS: VersionSet,
        M: Eq + Clone + Debug + Display,
        F: ReportFormatter<P, VS, M, Output = String>,
    >(
        &mut self,
        tree: &DerivationTree<P, VS, M>,
        root: DerivationTreeId,
        formatter: &F,
    ) {
        let mut stack = vec![ReportStep::Build(root)];
        while let Some(step) = stack.pop() {
            match step {
                ReportStep::Build(id) => {
                    stack.push(ReportStep::AfterBuild(id));
                    stack.push(ReportStep::BuildHelper(id));
                }
                ReportStep::AfterBuild(id) => {
                    let derived = tree.derived(id);
                    if let Some(shared_id) = derived.shared_id {
                        #[allow(clippy::map_entry)]
                        // `add_line_ref` not compatible with proposed fix.
                        if !self.shared_with_ref.contains_key(&shared_id) {
                            self.add_line_ref();
                            self.shared_with_ref.insert(shared_id, self.ref_count);
                        }
                    }
                }
                ReportStep::BuildHelper(current_id) => {
                    let current = tree.derived(current_id);
                    match (tree.node(current.cause1), tree.node(current.cause2)) {
                        (
                            DerivationTreeNode::External(external1),
                            DerivationTreeNode::External(external2),
                        ) => {
                            self.lines.push(formatter.explain_both_external(
                                external1,
                                external2,
                                &current.terms,
                            ));
                        }
                        (DerivationTreeNode::Derived(_), DerivationTreeNode::External(_)) => self
                            .push_report_one_each(
                                &mut stack,
                                tree,
                                current.cause1,
                                current.cause2,
                                current_id,
                                formatter,
                            ),
                        (DerivationTreeNode::External(_), DerivationTreeNode::Derived(_)) => self
                            .push_report_one_each(
                                &mut stack,
                                tree,
                                current.cause2,
                                current.cause1,
                                current_id,
                                formatter,
                            ),
                        (
                            DerivationTreeNode::Derived(derived1),
                            DerivationTreeNode::Derived(derived2),
                        ) => {
                            match (
                                self.line_ref_of(derived1.shared_id),
                                self.line_ref_of(derived2.shared_id),
                            ) {
                                (Some(ref1), Some(ref2)) => {
                                    self.lines.push(formatter.explain_both_ref(
                                        ref1,
                                        derived1,
                                        ref2,
                                        derived2,
                                        &current.terms,
                                    ));
                                }
                                (Some(ref1), None) => {
                                    stack.push(ReportStep::AndExplainRef {
                                        ref_id: ref1,
                                        derived: current.cause1,
                                        current: current_id,
                                    });
                                    stack.push(ReportStep::Build(current.cause2));
                                }
                                (None, Some(ref2)) => {
                                    stack.push(ReportStep::AndExplainRef {
                                        ref_id: ref2,
                                        derived: current.cause2,
                                        current: current_id,
                                    });
                                    stack.push(ReportStep::Build(current.cause1));
                                }
                                (None, None) => {
                                    stack.push(ReportStep::AfterFirstNoRefs {
                                        current: current_id,
                                        derived1: current.cause1,
                                        derived2: current.cause2,
                                    });
                                    stack.push(ReportStep::Build(current.cause1));
                                }
                            }
                        }
                    }
                }
                ReportStep::AfterFirstNoRefs {
                    current,
                    derived1,
                    derived2,
                } => {
                    if tree.derived(derived1).shared_id.is_some() {
                        self.lines.push(String::new());
                        stack.push(ReportStep::Build(current));
                    } else {
                        self.add_line_ref();
                        let ref_id = self.ref_count;
                        self.lines.push(String::new());
                        stack.push(ReportStep::AndExplainRef {
                            ref_id,
                            derived: derived1,
                            current,
                        });
                        stack.push(ReportStep::Build(derived2));
                    }
                }
                ReportStep::AndExplainRef {
                    ref_id,
                    derived,
                    current,
                } => {
                    self.lines.push(formatter.and_explain_ref(
                        ref_id,
                        tree.derived(derived),
                        &tree.derived(current).terms,
                    ));
                }
                ReportStep::AndExplainExternal { external, current } => {
                    self.lines.push(formatter.and_explain_external(
                        tree.external(external),
                        &tree.derived(current).terms,
                    ));
                }
                ReportStep::AndExplainPriorAndExternal {
                    prior_external,
                    external,
                    current,
                } => {
                    self.lines.push(formatter.and_explain_prior_and_external(
                        tree.external(prior_external),
                        tree.external(external),
                        &tree.derived(current).terms,
                    ));
                }
            }
        }
    }

    fn push_report_one_each<
        P: Package,
        VS: VersionSet,
        M: Eq + Clone + Debug + Display,
        F: ReportFormatter<P, VS, M, Output = String>,
    >(
        &mut self,
        stack: &mut Vec<ReportStep>,
        tree: &DerivationTree<P, VS, M>,
        derived_id: DerivationTreeId,
        external_id: DerivationTreeId,
        current_id: DerivationTreeId,
        formatter: &F,
    ) {
        let derived = tree.derived(derived_id);
        match self.line_ref_of(derived.shared_id) {
            Some(ref_id) => self.lines.push(formatter.explain_ref_and_external(
                ref_id,
                derived,
                tree.external(external_id),
                &tree.derived(current_id).terms,
            )),
            None => match (tree.node(derived.cause1), tree.node(derived.cause2)) {
                (DerivationTreeNode::Derived(_), DerivationTreeNode::External(_)) => {
                    stack.push(ReportStep::AndExplainPriorAndExternal {
                        prior_external: derived.cause2,
                        external: external_id,
                        current: current_id,
                    });
                    stack.push(ReportStep::Build(derived.cause1));
                }
                (DerivationTreeNode::External(_), DerivationTreeNode::Derived(_)) => {
                    stack.push(ReportStep::AndExplainPriorAndExternal {
                        prior_external: derived.cause1,
                        external: external_id,
                        current: current_id,
                    });
                    stack.push(ReportStep::Build(derived.cause2));
                }
                _ => {
                    stack.push(ReportStep::AndExplainExternal {
                        external: external_id,
                        current: current_id,
                    });
                    stack.push(ReportStep::Build(derived_id));
                }
            },
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
        match derivation_tree.root() {
            DerivationTreeNode::External(external) => formatter.format_external(external),
            DerivationTreeNode::Derived(_) => {
                let mut reporter = Self::new();
                reporter.build(derivation_tree, derivation_tree.root_id(), &formatter);
                reporter.lines.join("\n")
            }
        }
    }

    fn report_with_formatter(
        derivation_tree: &DerivationTree<P, VS, M>,
        formatter: &impl ReportFormatter<P, VS, M, Output = Self::Output>,
    ) -> Self::Output {
        match derivation_tree.root() {
            DerivationTreeNode::External(external) => formatter.format_external(external),
            DerivationTreeNode::Derived(_) => {
                let mut reporter = Self::new();
                reporter.build(derivation_tree, derivation_tree.root_id(), formatter);
                reporter.lines.join("\n")
            }
        }
    }
}
