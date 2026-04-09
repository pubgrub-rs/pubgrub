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
#[derive(Clone)]
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
#[derive(Clone)]
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

// Manual iterative `Debug` implementations for `DerivationTree` and `Derived`. The auto-derived
// `Debug` walks `cause1`/`cause2: Arc<DerivationTree>` recursively and treats the DAG as a tree,
// which blows both the stack *and* the output-size budget on deep derivation trees: a single
// root-to-leaf path has ~5k frames, and unfolding the DAG without deduping causes exponential
// output (see <https://github.com/pubgrub-rs/pubgrub/issues/293>).
//
// Instead, walk iteratively and dedup shared subtrees by pointer, emitting a `#N` back-reference
// on the second and later visits.

/// One step of the iterative debug walk.
enum DebugStep<'a, P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> {
    Tree(&'a DerivationTree<P, VS, M>),
    Derived(&'a Derived<P, VS, M>),
    Str(&'static str),
}

fn debug_walk<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display>(
    start: DebugStep<'_, P, VS, M>,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    let mut stack = vec![start];
    // Maps a `Derived`'s address to the index at which it was first emitted.
    // Subsequent visits emit `#N` instead of expanding the subtree again.
    let mut seen: Map<*const Derived<P, VS, M>, usize> = Map::default();
    while let Some(step) = stack.pop() {
        match step {
            DebugStep::Str(s) => f.write_str(s)?,
            DebugStep::Tree(DerivationTree::External(external)) => {
                write!(f, "External({external:?})")?;
            }
            DebugStep::Tree(DerivationTree::Derived(d)) => {
                let key = d as *const Derived<P, VS, M>;
                if let Some(&id) = seen.get(&key) {
                    write!(f, "Derived(#{id})")?;
                    continue;
                }
                let id = seen.len();
                seen.insert(key, id);
                write!(f, "Derived(#{id} ")?;
                stack.push(DebugStep::Str(")"));
                stack.push(DebugStep::Derived(d));
            }
            DebugStep::Derived(d) => {
                // Direct `Debug::fmt` on a `Derived` (not via `DerivationTree`) does not go
                // through the `seen` registration path, so it may emit a full subtree on its
                // own. That's fine: subsequent repeated subtrees are still deduped.
                write!(
                    f,
                    "Derived {{ terms: {:?}, shared_id: {:?}, cause1: ",
                    d.terms, d.shared_id
                )?;
                stack.push(DebugStep::Str(" }"));
                stack.push(DebugStep::Tree(&d.cause2));
                stack.push(DebugStep::Str(", cause2: "));
                stack.push(DebugStep::Tree(&d.cause1));
            }
        }
    }
    Ok(())
}

impl<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> Debug
    for DerivationTree<P, VS, M>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_walk(DebugStep::Tree(self), f)
    }
}

impl<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> Debug for Derived<P, VS, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_walk(DebugStep::Derived(self), f)
    }
}

impl<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> DerivationTree<P, VS, M> {
    /// Get all packages referred to in the derivation tree.
    pub fn packages(&self) -> Set<&P> {
        let mut packages = Set::default();
        let mut stack: Vec<&Self> = vec![self];
        // Dedup shared subtrees by their address. The derivation tree is a DAG, not a tree:
        // many `Derived` nodes can be reached via multiple `Arc` parents.
        let mut seen: Set<*const Self> = Set::default();
        while let Some(node) = stack.pop() {
            if !seen.insert(node as *const _) {
                continue;
            }
            match node {
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
                    packages.extend(derived.terms.keys());
                    stack.push(&derived.cause1);
                    stack.push(&derived.cause2);
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
        // Bottom-up rewrite of the DAG. We walk the tree in post-order, computing the
        // collapsed version of each unique node and memoizing the result by the source
        // node's address. This avoids both the recursive walk (stack overflow on deep
        // trees) and the implicit cloning of shared subtrees that the previous
        // `Arc::make_mut`-based recursive impl performed at every level.
        type NodePtr<P, VS, M> = *const DerivationTree<P, VS, M>;

        // Step 1: collect each unique node in post-order using an explicit stack.
        let mut topo: Vec<&Self> = Vec::new();
        let mut visited: Set<NodePtr<P, VS, M>> = Set::default();
        let mut work: Vec<(&Self, bool)> = vec![(self, false)];
        while let Some((node, processed)) = work.pop() {
            if processed {
                topo.push(node);
                continue;
            }
            let key = node as NodePtr<P, VS, M>;
            if !visited.insert(key) {
                continue;
            }
            work.push((node, true));
            if let DerivationTree::Derived(d) = node {
                work.push((&d.cause1, false));
                work.push((&d.cause2, false));
            }
        }

        // Step 2: compute the collapsed result for each unique node, in topological order.
        let mut memo: Map<NodePtr<P, VS, M>, Arc<DerivationTree<P, VS, M>>> = Map::default();
        for node in &topo {
            let key = *node as NodePtr<P, VS, M>;
            let result: Arc<DerivationTree<P, VS, M>> = match node {
                DerivationTree::External(_) => Arc::new((*node).clone()),
                DerivationTree::Derived(d) => {
                    let cause1 = memo[&(Arc::as_ptr(&d.cause1) as NodePtr<P, VS, M>)].clone();
                    let cause2 = memo[&(Arc::as_ptr(&d.cause2) as NodePtr<P, VS, M>)].clone();
                    let collapsed_derived = || {
                        Arc::new(DerivationTree::Derived(Derived {
                            terms: d.terms.clone(),
                            shared_id: d.shared_id,
                            cause1: cause1.clone(),
                            cause2: cause2.clone(),
                        }))
                    };
                    match (cause1.deref(), cause2.deref()) {
                        (DerivationTree::External(External::NoVersions(p, r)), _) => {
                            match (*cause2).clone().merge_no_versions(p.clone(), r.clone()) {
                                Some(merged) => Arc::new(merged),
                                None => collapsed_derived(),
                            }
                        }
                        (_, DerivationTree::External(External::NoVersions(p, r))) => {
                            match (*cause1).clone().merge_no_versions(p.clone(), r.clone()) {
                                Some(merged) => Arc::new(merged),
                                None => collapsed_derived(),
                            }
                        }
                        _ => collapsed_derived(),
                    }
                }
            };
            memo.insert(key, result);
        }

        // Step 3: replace `*self` with the root's collapsed version.
        let root_key = self as NodePtr<P, VS, M>;
        let root = memo.remove(&root_key).expect("root was visited");
        // The root may still be referenced by `memo` entries for other nodes (if shared),
        // in which case `try_unwrap` returns `Err` and we fall back to cloning the inner.
        *self = match Arc::try_unwrap(root) {
            Ok(inner) => inner,
            Err(arc) => (*arc).clone(),
        };
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

/// One step of the iterative report builder. Reifies the call stack of the
/// previously-recursive `build_recursive` / `build_recursive_helper` functions.
enum BuildFrame<'a, P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> {
    /// Equivalent to one call to the original `build_recursive(derived)`:
    /// dispatches on the cause types and pushes follow-up frames.
    Process(&'a Derived<P, VS, M>),
    /// Equivalent to the post-step of the original `build_recursive`: if
    /// `shared_id` is set and not yet registered, register it and add a line
    /// reference to the most recently emitted line.
    PostShared(Option<usize>),
    /// Emit `and_explain_prior_and_external(prior_external, external, current_terms)`.
    EmitPriorAndExternal {
        prior_external: &'a External<P, VS, M>,
        external: &'a External<P, VS, M>,
        current_terms: &'a Map<P, Term<VS>>,
    },
    /// Emit `and_explain_external(external, current_terms)`.
    EmitExternal {
        external: &'a External<P, VS, M>,
        current_terms: &'a Map<P, Term<VS>>,
    },
    /// Emit `and_explain_ref(ref_id, derived, current_terms)`.
    EmitRef {
        ref_id: usize,
        derived: &'a Derived<P, VS, M>,
        current_terms: &'a Map<P, Term<VS>>,
    },
    /// Continuation of the `(None, None)` sub-branch in the (Derived, Derived)
    /// case, executed after `derived1` has been fully processed.
    PostD1OfNoneNone {
        current: &'a Derived<P, VS, M>,
        derived1: &'a Derived<P, VS, M>,
        derived2: &'a Derived<P, VS, M>,
    },
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

    /// Iteratively walk the derivation tree to build the report.
    ///
    /// This used to be implemented as mutually recursive functions
    /// (`build_recursive` / `build_recursive_helper` / `report_one_each` /
    /// `report_recurse_one_each`), which blew the stack on deep derivation trees
    /// (see <https://github.com/pubgrub-rs/pubgrub/issues/293>). It is now driven
    /// by an explicit work stack of [`BuildFrame`]s. The control flow exactly
    /// mirrors the original recursive version.
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
        let mut stack: Vec<BuildFrame<'_, P, VS, M>> = vec![BuildFrame::Process(derived)];
        while let Some(frame) = stack.pop() {
            match frame {
                // Equivalent to one call to the original `build_recursive(d)`.
                BuildFrame::Process(d) => {
                    // The shared-id post-step runs after everything `d` produces, so
                    // it must be pushed first (LIFO order).
                    stack.push(BuildFrame::PostShared(d.shared_id));
                    // Inlined `build_recursive_helper(d)`.
                    match (d.cause1.deref(), d.cause2.deref()) {
                        (
                            DerivationTree::External(external1),
                            DerivationTree::External(external2),
                        ) => {
                            // Simplest case, we just combine two external incompatibilities.
                            self.lines.push(formatter.explain_both_external(
                                external1, external2, &d.terms,
                            ));
                        }
                        (DerivationTree::Derived(child), DerivationTree::External(external))
                        | (DerivationTree::External(external), DerivationTree::Derived(child)) => {
                            // Inlined `report_one_each(child, external, &d.terms, formatter)`.
                            match self.line_ref_of(child.shared_id) {
                                Some(ref_id) => {
                                    self.lines.push(formatter.explain_ref_and_external(
                                        ref_id, child, external, &d.terms,
                                    ));
                                }
                                None => {
                                    // Inlined `report_recurse_one_each(child, external, ...)`.
                                    match (child.cause1.deref(), child.cause2.deref()) {
                                        (
                                            DerivationTree::Derived(prior_derived),
                                            DerivationTree::External(prior_external),
                                        )
                                        | (
                                            DerivationTree::External(prior_external),
                                            DerivationTree::Derived(prior_derived),
                                        ) => {
                                            stack.push(BuildFrame::EmitPriorAndExternal {
                                                prior_external,
                                                external,
                                                current_terms: &d.terms,
                                            });
                                            stack.push(BuildFrame::Process(prior_derived));
                                        }
                                        _ => {
                                            stack.push(BuildFrame::EmitExternal {
                                                external,
                                                current_terms: &d.terms,
                                            });
                                            stack.push(BuildFrame::Process(child));
                                        }
                                    }
                                }
                            }
                        }
                        (
                            DerivationTree::Derived(derived1),
                            DerivationTree::Derived(derived2),
                        ) => {
                            // The most complex case: both causes are derived.
                            match (
                                self.line_ref_of(derived1.shared_id),
                                self.line_ref_of(derived2.shared_id),
                            ) {
                                // If both causes already have been referenced (shared_id),
                                // the explanation simply uses those references.
                                (Some(ref1), Some(ref2)) => {
                                    self.lines.push(formatter.explain_both_ref(
                                        ref1, derived1, ref2, derived2, &d.terms,
                                    ));
                                }
                                // Otherwise, if one only has a line number reference,
                                // we recursively call the one without reference and then
                                // add the one with reference to conclude.
                                (Some(ref1), None) => {
                                    stack.push(BuildFrame::EmitRef {
                                        ref_id: ref1,
                                        derived: derived1,
                                        current_terms: &d.terms,
                                    });
                                    stack.push(BuildFrame::Process(derived2));
                                }
                                (None, Some(ref2)) => {
                                    stack.push(BuildFrame::EmitRef {
                                        ref_id: ref2,
                                        derived: derived2,
                                        current_terms: &d.terms,
                                    });
                                    stack.push(BuildFrame::Process(derived1));
                                }
                                // No line reference exists yet: process derived1, then
                                // decide based on whether processing derived1 created a
                                // line ref for it.
                                (None, None) => {
                                    stack.push(BuildFrame::PostD1OfNoneNone {
                                        current: d,
                                        derived1,
                                        derived2,
                                    });
                                    stack.push(BuildFrame::Process(derived1));
                                }
                            }
                        }
                    }
                }
                BuildFrame::PostShared(shared_id) => {
                    if let Some(id) = shared_id {
                        #[allow(clippy::map_entry)] // `add_line_ref` mutates `self.lines`.
                        if !self.shared_with_ref.contains_key(&id) {
                            self.add_line_ref();
                            self.shared_with_ref.insert(id, self.ref_count);
                        }
                    }
                }
                BuildFrame::EmitPriorAndExternal {
                    prior_external,
                    external,
                    current_terms,
                } => {
                    self.lines.push(formatter.and_explain_prior_and_external(
                        prior_external,
                        external,
                        current_terms,
                    ));
                }
                BuildFrame::EmitExternal {
                    external,
                    current_terms,
                } => {
                    self.lines
                        .push(formatter.and_explain_external(external, current_terms));
                }
                BuildFrame::EmitRef {
                    ref_id,
                    derived,
                    current_terms,
                } => {
                    self.lines
                        .push(formatter.and_explain_ref(ref_id, derived, current_terms));
                }
                BuildFrame::PostD1OfNoneNone {
                    current,
                    derived1,
                    derived2,
                } => {
                    // Mirrors the post-`build_recursive(derived1)` logic in the original
                    // (None, None) branch. By the time we get here, `derived1` has been
                    // fully processed (including its `PostShared` frame), so the
                    // `shared_with_ref` map already reflects whether it gained a line ref.
                    if derived1.shared_id.is_some() {
                        self.lines.push("".into());
                        // Re-process `current`. This time, the `(None, None)` branch
                        // will see that `derived1` has a line ref and dispatch into the
                        // `(None, Some(ref))` sub-branch instead.
                        stack.push(BuildFrame::Process(current));
                    } else {
                        self.add_line_ref();
                        let ref1 = self.ref_count;
                        self.lines.push("".into());
                        stack.push(BuildFrame::EmitRef {
                            ref_id: ref1,
                            derived: derived1,
                            current_terms: &current.terms,
                        });
                        stack.push(BuildFrame::Process(derived2));
                    }
                }
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
