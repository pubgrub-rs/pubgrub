// SPDX-License-Identifier: MPL-2.0

use pubgrub::error::PubGrubError;
use pubgrub::range::Range;
use pubgrub::report::Reporter;
use pubgrub::solver::{resolve, OfflineDependencyProvider};
use pubgrub::version::SemanticVersion;

use std::fmt::{self, Display};

use pubgrub::report::{DerivationTree, Derived, External};
use pubgrub::term::Term;
use pubgrub::type_aliases::Map;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Package {
    Root,
    Package(String),
}

impl Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Package::Root => write!(f, "root"),
            Package::Package(name) => write!(f, "{}", name),
        }
    }
}

/// Derivative of [`pubgrub::report::DefaultStringReporter`] for customized display
/// of package resolution errors.
pub struct CustomStringReporter {
    /// Number of explanations already with a line reference.
    ref_count: usize,
    /// Shared nodes that have already been marked with a line reference.
    /// The incompatibility ids are the keys, and the line references are the values.
    shared_with_ref: Map<usize, usize>,
    /// Accumulated lines of the report already generated.
    lines: Vec<String>,
}

impl CustomStringReporter {
    /// Initialize the reporter.
    fn new() -> Self {
        Self {
            ref_count: 0,
            shared_with_ref: Map::default(),
            lines: Vec::new(),
        }
    }

    fn build_recursive(&mut self, derived: &Derived<Package, Range<SemanticVersion>>) {
        self.build_recursive_helper(derived);
        if let Some(id) = derived.shared_id {
            if self.shared_with_ref.get(&id).is_none() {
                self.add_line_ref();
                self.shared_with_ref.insert(id, self.ref_count);
            }
        };
    }

    fn build_recursive_helper(&mut self, current: &Derived<Package, Range<SemanticVersion>>) {
        match (&*current.cause1, &*current.cause2) {
            (DerivationTree::External(external1), DerivationTree::External(external2)) => {
                // Simplest case, we just combine two external incompatibilities.
                self.lines.push(Self::explain_both_external(
                    external1,
                    external2,
                    &current.terms,
                ));
            }
            (DerivationTree::Derived(derived), DerivationTree::External(external)) => {
                // One cause is derived, so we explain this first
                // then we add the one-line external part
                // and finally conclude with the current incompatibility.
                self.report_one_each(derived, external, &current.terms);
            }
            (DerivationTree::External(external), DerivationTree::Derived(derived)) => {
                self.report_one_each(derived, external, &current.terms);
            }
            (DerivationTree::Derived(derived1), DerivationTree::Derived(derived2)) => {
                // This is the most complex case since both causes are also derived.
                match (
                    self.line_ref_of(derived1.shared_id),
                    self.line_ref_of(derived2.shared_id),
                ) {
                    // If both causes already have been referenced (shared_id),
                    // the explanation simply uses those references.
                    (Some(ref1), Some(ref2)) => self.lines.push(Self::explain_both_ref(
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
                        self.build_recursive(derived2);
                        self.lines
                            .push(Self::and_explain_ref(ref1, derived1, &current.terms));
                    }
                    (None, Some(ref2)) => {
                        self.build_recursive(derived1);
                        self.lines
                            .push(Self::and_explain_ref(ref2, derived2, &current.terms));
                    }
                    // Finally, if no line reference exists yet,
                    // we call recursively the first one and then,
                    //   - if this was a shared node, it will get a line ref
                    //     and we can simply recall this with the current node.
                    //   - otherwise, we add a line reference to it,
                    //     recursively call on the second node,
                    //     and finally conclude.
                    (None, None) => {
                        self.build_recursive(derived1);
                        if derived1.shared_id.is_some() {
                            self.lines.push(String::new());
                            self.build_recursive(current);
                        } else {
                            self.add_line_ref();
                            let ref1 = self.ref_count;
                            self.lines.push(String::new());
                            self.build_recursive(derived2);
                            self.lines
                                .push(Self::and_explain_ref(ref1, derived1, &current.terms));
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
    fn report_one_each(
        &mut self,
        derived: &Derived<Package, Range<SemanticVersion>>,
        external: &External<Package, Range<SemanticVersion>>,
        current_terms: &Map<Package, Term<Range<SemanticVersion>>>,
    ) {
        match self.line_ref_of(derived.shared_id) {
            Some(ref_id) => self.lines.push(Self::explain_ref_and_external(
                ref_id,
                derived,
                external,
                current_terms,
            )),
            None => self.report_recurse_one_each(derived, external, current_terms),
        }
    }

    /// Report one derived (without a line ref yet) and one external.
    fn report_recurse_one_each(
        &mut self,
        derived: &Derived<Package, Range<SemanticVersion>>,
        external: &External<Package, Range<SemanticVersion>>,
        current_terms: &Map<Package, Term<Range<SemanticVersion>>>,
    ) {
        match (&*derived.cause1, &*derived.cause2) {
            // If the derived cause has itself one external prior cause,
            // we can chain the external explanations.
            (DerivationTree::Derived(prior_derived), DerivationTree::External(prior_external)) => {
                self.build_recursive(prior_derived);
                self.lines.push(Self::and_explain_prior_and_external(
                    prior_external,
                    external,
                    current_terms,
                ));
            }
            // If the derived cause has itself one external prior cause,
            // we can chain the external explanations.
            (DerivationTree::External(prior_external), DerivationTree::Derived(prior_derived)) => {
                self.build_recursive(prior_derived);
                self.lines.push(Self::and_explain_prior_and_external(
                    prior_external,
                    external,
                    current_terms,
                ));
            }
            _ => {
                self.build_recursive(derived);
                self.lines
                    .push(Self::and_explain_external(external, current_terms));
            }
        }
    }

    // String explanations #####################################################

    /// Simplest case, we just combine two external incompatibilities.
    fn explain_both_external(
        external1: &External<Package, Range<SemanticVersion>>,
        external2: &External<Package, Range<SemanticVersion>>,
        current_terms: &Map<Package, Term<Range<SemanticVersion>>>,
    ) -> String {
        // TODO: order should be chosen to make it more logical.
        format!(
            "Because {} and {}, {}.",
            CustomExternal::from_pubgrub(external1.clone()),
            CustomExternal::from_pubgrub(external2.clone()),
            Self::string_terms(current_terms)
        )
    }

    /// Both causes have already been explained so we use their refs.
    fn explain_both_ref(
        ref_id1: usize,
        derived1: &Derived<Package, Range<SemanticVersion>>,
        ref_id2: usize,
        derived2: &Derived<Package, Range<SemanticVersion>>,
        current_terms: &Map<Package, Term<Range<SemanticVersion>>>,
    ) -> String {
        // TODO: order should be chosen to make it more logical.
        format!(
            "Because {} ({}) and {} ({}), {}.",
            Self::string_terms(&derived1.terms),
            ref_id1,
            Self::string_terms(&derived2.terms),
            ref_id2,
            Self::string_terms(current_terms)
        )
    }

    /// One cause is derived (already explained so one-line),
    /// the other is a one-line external cause,
    /// and finally we conclude with the current incompatibility.
    fn explain_ref_and_external(
        ref_id: usize,
        derived: &Derived<Package, Range<SemanticVersion>>,
        external: &External<Package, Range<SemanticVersion>>,
        current_terms: &Map<Package, Term<Range<SemanticVersion>>>,
    ) -> String {
        // TODO: order should be chosen to make it more logical.
        format!(
            "Because {} ({}) and {}, {}.",
            Self::string_terms(&derived.terms),
            ref_id,
            CustomExternal::from_pubgrub(external.clone()),
            Self::string_terms(current_terms)
        )
    }

    /// Add an external cause to the chain of explanations.
    fn and_explain_external(
        external: &External<Package, Range<SemanticVersion>>,
        current_terms: &Map<Package, Term<Range<SemanticVersion>>>,
    ) -> String {
        format!(
            "And because {}, {}.",
            CustomExternal::from_pubgrub(external.clone()),
            Self::string_terms(current_terms)
        )
    }

    /// Add an already explained incompat to the chain of explanations.
    fn and_explain_ref(
        ref_id: usize,
        derived: &Derived<Package, Range<SemanticVersion>>,
        current_terms: &Map<Package, Term<Range<SemanticVersion>>>,
    ) -> String {
        format!(
            "And because {} ({}), {}.",
            Self::string_terms(&derived.terms),
            ref_id,
            Self::string_terms(current_terms)
        )
    }

    /// Add an already explained incompat to the chain of explanations.
    fn and_explain_prior_and_external(
        prior_external: &External<Package, Range<SemanticVersion>>,
        external: &External<Package, Range<SemanticVersion>>,
        current_terms: &Map<Package, Term<Range<SemanticVersion>>>,
    ) -> String {
        format!(
            "And because {} and {}, {}.",
            CustomExternal::from_pubgrub(prior_external.clone()),
            CustomExternal::from_pubgrub(external.clone()),
            Self::string_terms(current_terms)
        )
    }

    /// Try to print terms of an incompatibility in a human-readable way.
    pub fn string_terms(terms: &Map<Package, Term<Range<SemanticVersion>>>) -> String {
        let terms_vec: Vec<_> = terms.iter().collect();
        match terms_vec.as_slice() {
            [] => "version solving failed".into(),
            // TODO: special case when that unique package is root.
            [(package @ Package::Root, Term::Positive(_))] => {
                format!("{package} is forbidden")
            }
            [(package @ Package::Root, Term::Negative(_))] => {
                format!("{package} is mandatory")
            }
            [(package @ Package::Package(_), Term::Positive(range))] => {
                format!("{package} {range} is forbidden")
            }
            [(package @ Package::Package(_), Term::Negative(range))] => {
                format!("{package} {range} is mandatory")
            }
            [(p1, Term::Positive(r1)), (p2, Term::Negative(r2))] => {
                External::FromDependencyOf(p1, r1.clone(), p2, r2.clone()).to_string()
            }
            [(p1, Term::Negative(r1)), (p2, Term::Positive(r2))] => {
                External::FromDependencyOf(p2, r2.clone(), p1, r1.clone()).to_string()
            }
            slice => {
                let str_terms: Vec<_> = slice.iter().map(|(p, t)| format!("{p} {t}")).collect();
                str_terms.join(", ") + " are incompatible"
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
        shared_id.and_then(|id| self.shared_with_ref.get(&id).copied())
    }
}

impl Reporter<Package, Range<SemanticVersion>> for CustomStringReporter {
    type Output = String;

    fn report(derivation_tree: &DerivationTree<Package, Range<SemanticVersion>>) -> Self::Output {
        match derivation_tree {
            DerivationTree::External(external) => {
                CustomExternal::from_pubgrub(external.clone()).to_string()
            }
            DerivationTree::Derived(derived) => {
                let mut reporter = Self::new();
                reporter.build_recursive(derived);
                reporter.lines.join("\n")
            }
        }
    }
}

/// Derivative of [`pubgrub::report::External`] for customized display
/// for internal [`Package`].
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
enum CustomExternal {
    /// Initial incompatibility aiming at picking the root package for the first decision.
    NotRoot(Package, SemanticVersion),
    /// There are no versions in the given set for this package.
    NoVersions(Package, Range<SemanticVersion>),
    /// Dependencies of the package are unavailable for versions in that set.
    UnavailableDependencies(Package, Range<SemanticVersion>),
    /// Incompatibility coming from the dependencies of a given package.
    FromDependencyOf(
        Package,
        Range<SemanticVersion>,
        Package,
        Range<SemanticVersion>,
    ),
}

impl CustomExternal {
    fn from_pubgrub(external: External<Package, Range<SemanticVersion>>) -> Self {
        match external {
            External::NotRoot(p, v) => CustomExternal::NotRoot(p, v),
            External::NoVersions(p, vs) => CustomExternal::NoVersions(p, vs),
            External::UnavailableDependencies(p, vs) => {
                CustomExternal::UnavailableDependencies(p, vs)
            }
            External::FromDependencyOf(p, vs, p_dep, vs_dep) => {
                CustomExternal::FromDependencyOf(p, vs, p_dep, vs_dep)
            }
        }
    }
}

impl fmt::Display for CustomExternal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotRoot(package, version) => {
                write!(f, "we are solving dependencies of {package} {version}")
            }
            Self::NoVersions(package, set) => {
                if set == &Range::full() {
                    write!(f, "there is no available version for {package}")
                } else {
                    write!(f, "there is no version of {package} in {set}")
                }
            }
            Self::UnavailableDependencies(package, set) => {
                if set == &Range::full() {
                    write!(f, "dependencies of {package} are unavailable")
                } else {
                    write!(
                        f,
                        "dependencies of {package} at version {set} are unavailable"
                    )
                }
            }
            Self::FromDependencyOf(package, package_set, dependency, dependency_set) => {
                if package_set == &Range::full() && dependency_set == &Range::full() {
                    write!(f, "{package} depends on {dependency}")
                } else if package_set == &Range::full() {
                    write!(f, "{package} depends on {dependency} {dependency_set}")
                } else if dependency_set == &Range::full() {
                    if matches!(package, Package::Root) {
                        // Exclude the dummy version for root packages
                        write!(f, "{package} depends on {dependency}")
                    } else {
                        write!(f, "{package} {package_set} depends on {dependency}")
                    }
                } else {
                    if matches!(package, Package::Root) {
                        // Exclude the dummy version for root packages
                        write!(f, "{package} depends on {dependency} {dependency_set}")
                    } else {
                        write!(
                            f,
                            "{package} {package_set} depends on {dependency} {dependency_set}"
                        )
                    }
                }
            }
        }
    }
}

fn main() {
    let mut dependency_provider =
        OfflineDependencyProvider::<Package, Range<SemanticVersion>>::new();
    // Define the root package with a dependency on a package we do not provide
    dependency_provider.add_dependencies(
        Package::Root,
        (0, 0, 0),
        vec![(
            Package::Package("foo".to_string()),
            Range::singleton((1, 0, 0)),
        )],
    );

    // Run the algorithm
    match resolve(&dependency_provider, Package::Root, (0, 0, 0)) {
        Ok(sol) => println!("{:?}", sol),
        Err(PubGrubError::NoSolution(mut derivation_tree)) => {
            eprintln!("No solution.\n");

            eprintln!("### Default report:");
            eprintln!("```");
            eprintln!("{}", CustomStringReporter::report(&derivation_tree));
            eprintln!("```\n");

            derivation_tree.collapse_no_versions();
            eprintln!("### Report with `collapse_no_versions`:");
            eprintln!("```");
            eprintln!("{}", CustomStringReporter::report(&derivation_tree));
            eprintln!("```");
            std::process::exit(1);
        }
        Err(err) => panic!("{:?}", err),
    };
}
