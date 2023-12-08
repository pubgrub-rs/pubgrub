// SPDX-License-Identifier: MPL-2.0

use pubgrub::error::PubGrubError;
use pubgrub::range::Range;
use pubgrub::report::Reporter;
use pubgrub::solver::{resolve, OfflineDependencyProvider};
use pubgrub::version::SemanticVersion;

use pubgrub::report::{DefaultStringReporter, External, ReportFormatter};
use pubgrub::term::Term;
use pubgrub::type_aliases::Map;
use std::fmt::{self, Display};

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

impl ReportFormatter<Package, Range<SemanticVersion>> for CustomReportFormatter {
    type Output = String;

    fn format_terms(&self, terms: &Map<Package, Term<Range<SemanticVersion>>>) -> String {
        let terms_vec: Vec<_> = terms.iter().collect();
        match terms_vec.as_slice() {
            [] => "version solving failed".into(),
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

    fn format_external(&self, external: &External<Package, Range<SemanticVersion>>) -> String {
        match external {
            External::NotRoot(package, version) => {
                format!("we are solving dependencies of {package} {version}")
            }
            External::NoVersions(package, set) => {
                if set == &Range::full() {
                    format!("there is no available version for {package}")
                } else {
                    format!("there is no version of {package} in {set}")
                }
            }
            External::UnavailableDependencies(package, set) => {
                if set == &Range::full() {
                    format!("dependencies of {package} are unavailable")
                } else {
                    format!("dependencies of {package} at version {set} are unavailable")
                }
            }
            External::UnusableDependencies(package, set, ..) => {
                if set == &Range::full() {
                    format!("dependencies of {package} are unusable")
                } else {
                    format!("dependencies of {package} at version {set} are unusable")
                }
            }
            External::FromDependencyOf(package, package_set, dependency, dependency_set) => {
                if package_set == &Range::full() && dependency_set == &Range::full() {
                    format!("{package} depends on {dependency}")
                } else if package_set == &Range::full() {
                    format!("{package} depends on {dependency} {dependency_set}")
                } else if dependency_set == &Range::full() {
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
