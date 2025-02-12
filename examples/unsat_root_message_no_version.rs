// SPDX-License-Identifier: MPL-2.0

use std::fmt::{self, Display};

use pubgrub::{
    resolve, DefaultStringReporter, Derived, External, Map, OfflineDependencyProvider,
    PubGrubError, Ranges, ReportFormatter, Reporter, SemanticVersion, Term,
};

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

#[derive(Debug, Default)]
struct CustomReportFormatter;

impl ReportFormatter<Package, Ranges<SemanticVersion>, String> for CustomReportFormatter {
    type Output = String;

    fn format_terms(&self, terms: &Map<Package, Term<Ranges<SemanticVersion>>>) -> String {
        let terms_vec: Vec<_> = terms.iter().collect();
        match terms_vec.as_slice() {
            [] => "version solving failed".into(),
            [(package @ Package::Root, Term::Positive(_))] => {
                format!("{package} is forbidden")
            }
            [(package @ Package::Root, Term::Negative(_))] => {
                format!("{package} is mandatory")
            }
            [(package @ Package::Package(_), Term::Positive(ranges))] => {
                format!("{package} {ranges} is forbidden")
            }
            [(package @ Package::Package(_), Term::Negative(ranges))] => {
                format!("{package} {ranges} is mandatory")
            }
            [(p1, Term::Positive(r1)), (p2, Term::Negative(r2))] => {
                External::<_, _, String>::FromDependencyOf(p1, r1.clone(), p2, r2.clone())
                    .to_string()
            }
            [(p1, Term::Negative(r1)), (p2, Term::Positive(r2))] => {
                External::<_, _, String>::FromDependencyOf(p2, r2.clone(), p1, r1.clone())
                    .to_string()
            }
            slice => {
                let str_terms: Vec<_> = slice.iter().map(|(p, t)| format!("{p} {t}")).collect();
                str_terms.join(", ") + " are incompatible"
            }
        }
    }

    fn format_external(
        &self,
        external: &External<Package, Ranges<SemanticVersion>, String>,
    ) -> String {
        match external {
            External::NotRoot(package, version) => {
                format!("we are solving dependencies of {package} {version}")
            }
            External::NoVersions(package, set) => {
                if set == &Ranges::full() {
                    format!("there is no available version for {package}")
                } else {
                    format!("there is no version of {package} in {set}")
                }
            }
            External::Custom(package, set, reason) => {
                if set == &Ranges::full() {
                    format!("dependencies of {package} are unavailable because {reason}")
                } else {
                    format!("dependencies of {package} at version {set} are unavailable because {reason}")
                }
            }
            External::FromDependencyOf(package, package_set, dependency, dependency_set) => {
                if package_set == &Ranges::full() && dependency_set == &Ranges::full() {
                    format!("{package} depends on {dependency}")
                } else if package_set == &Ranges::full() {
                    format!("{package} depends on {dependency} {dependency_set}")
                } else if dependency_set == &Ranges::full() {
                    if matches!(package, Package::Root) {
                        // Exclude the dummy version for root packages
                        format!("{package} depends on {dependency}")
                    } else {
                        format!("{package} {package_set} depends on {dependency}")
                    }
                } else if matches!(package, Package::Root) {
                    // Exclude the dummy version for root packages
                    format!("{package} depends on {dependency} {dependency_set}")
                } else {
                    format!("{package} {package_set} depends on {dependency} {dependency_set}")
                }
            }
        }
    }

    /// Simplest case, we just combine two external incompatibilities.
    fn explain_both_external(
        &self,
        external1: &External<Package, Ranges<SemanticVersion>, String>,
        external2: &External<Package, Ranges<SemanticVersion>, String>,
        current_terms: &Map<Package, Term<Ranges<SemanticVersion>>>,
    ) -> String {
        // TODO: order should be chosen to make it more logical.
        format!(
            "Because {} and {}, {}.",
            self.format_external(external1),
            self.format_external(external2),
            self.format_terms(current_terms)
        )
    }

    /// Both causes have already been explained so we use their refs.
    fn explain_both_ref(
        &self,
        ref_id1: usize,
        derived1: &Derived<Package, Ranges<SemanticVersion>, String>,
        ref_id2: usize,
        derived2: &Derived<Package, Ranges<SemanticVersion>, String>,
        current_terms: &Map<Package, Term<Ranges<SemanticVersion>>>,
    ) -> String {
        // TODO: order should be chosen to make it more logical.
        format!(
            "Because {} ({}) and {} ({}), {}.",
            self.format_terms(&derived1.terms),
            ref_id1,
            self.format_terms(&derived2.terms),
            ref_id2,
            self.format_terms(current_terms)
        )
    }

    /// One cause is derived (already explained so one-line),
    /// the other is a one-line external cause,
    /// and finally we conclude with the current incompatibility.
    fn explain_ref_and_external(
        &self,
        ref_id: usize,
        derived: &Derived<Package, Ranges<SemanticVersion>, String>,
        external: &External<Package, Ranges<SemanticVersion>, String>,
        current_terms: &Map<Package, Term<Ranges<SemanticVersion>>>,
    ) -> String {
        // TODO: order should be chosen to make it more logical.
        format!(
            "Because {} ({}) and {}, {}.",
            self.format_terms(&derived.terms),
            ref_id,
            self.format_external(external),
            self.format_terms(current_terms)
        )
    }

    /// Add an external cause to the chain of explanations.
    fn and_explain_external(
        &self,
        external: &External<Package, Ranges<SemanticVersion>, String>,
        current_terms: &Map<Package, Term<Ranges<SemanticVersion>>>,
    ) -> String {
        format!(
            "And because {}, {}.",
            self.format_external(external),
            self.format_terms(current_terms)
        )
    }

    /// Add an already explained incompat to the chain of explanations.
    fn and_explain_ref(
        &self,
        ref_id: usize,
        derived: &Derived<Package, Ranges<SemanticVersion>, String>,
        current_terms: &Map<Package, Term<Ranges<SemanticVersion>>>,
    ) -> String {
        format!(
            "And because {} ({}), {}.",
            self.format_terms(&derived.terms),
            ref_id,
            self.format_terms(current_terms)
        )
    }

    /// Add an already explained incompat to the chain of explanations.
    fn and_explain_prior_and_external(
        &self,
        prior_external: &External<Package, Ranges<SemanticVersion>, String>,
        external: &External<Package, Ranges<SemanticVersion>, String>,
        current_terms: &Map<Package, Term<Ranges<SemanticVersion>>>,
    ) -> String {
        format!(
            "And because {} and {}, {}.",
            self.format_external(prior_external),
            self.format_external(external),
            self.format_terms(current_terms)
        )
    }
}

fn main() {
    let mut dependency_provider =
        OfflineDependencyProvider::<Package, Ranges<SemanticVersion>>::new();
    // Define the root package with a dependency on a package we do not provide
    dependency_provider.add_dependencies(
        Package::Root,
        (0, 0, 0),
        vec![(
            Package::Package("foo".to_string()),
            Ranges::singleton((1, 0, 0)),
        )],
    );

    // Run the algorithm
    match resolve(&dependency_provider, Package::Root, (0, 0, 0)) {
        Ok(sol) => println!("{:?}", sol),
        Err(PubGrubError::NoSolution(derivation_tree)) => {
            eprintln!("No solution.\n");

            eprintln!("### Default report:");
            eprintln!("```");
            eprintln!("{}", DefaultStringReporter::report(&derivation_tree));
            eprintln!("```\n");

            eprintln!("### Report with custom formatter:");
            eprintln!("```");
            eprintln!(
                "{}",
                DefaultStringReporter::report_with_formatter(
                    &derivation_tree,
                    &CustomReportFormatter
                )
            );
            eprintln!("```");
            std::process::exit(1);
        }
        Err(err) => panic!("{:?}", err),
    };
}
