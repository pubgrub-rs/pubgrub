// SPDX-License-Identifier: MPL-2.0

use std::fmt::{self, Debug, Display};
use std::hash::Hash;

use pubgrub::{
    helpers::PackageVersionWrapper, DefaultStringReporter, DependencyProvider, Derived, External,
    Map, OfflineDependencyProvider, PackageArena, PackageId, PubGrubError, Ranges, ReportFormatter,
    Reporter, SemanticVersion, Term, VersionSet,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CustomPackage {
    Root,
    Package(String),
}

impl Display for CustomPackage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CustomPackage::Root => write!(f, "root"),
            CustomPackage::Package(name) => write!(f, "{name}"),
        }
    }
}

type Store = PackageArena<PackageVersionWrapper<CustomPackage>>;
type Dp = OfflineDependencyProvider<CustomPackage, Ranges<SemanticVersion>>;

#[derive(Debug, Default)]
struct CustomReportFormatter;

impl ReportFormatter<Dp> for CustomReportFormatter {
    type Output = String;

    fn format_terms(&self, terms: &Map<PackageId, Term>, package_store: &Store, dp: &Dp) -> String {
        let terms_vec: Vec<_> = terms
            .iter()
            .map(|(&pid, &v)| (pid, package_store.pkg(pid).unwrap(), v))
            .collect();
        match *terms_vec.as_slice() {
            [] => "version solving failed".into(),
            [(pid, package, t)] => match package.inner_pkg() {
                Some(&CustomPackage::Root) if t.is_positive() => format!("{package} is forbidden"),
                Some(&CustomPackage::Root) => format!("{package} is mandatory"),
                _ if t.is_positive() => format!(
                    "{} is forbidden",
                    dp.package_version_set_display(
                        package_store.pkg(pid).unwrap(),
                        t.version_set()
                    ),
                ),
                _ => format!(
                    "{} is mandatory",
                    dp.package_version_set_display(
                        package_store.pkg(pid).unwrap(),
                        t.version_set()
                    ),
                ),
            },
            [(pid1, _, t1), (pid2, _, t2)] if t1.is_positive() && t2.is_negative() => {
                External::FromDependencyOf(pid1, t1.version_set(), pid2, t2.version_set())
                    .display(package_store, dp)
                    .to_string()
            }
            [(pid1, _, t1), (pid2, _, t2)] if t1.is_negative() && t2.is_positive() => {
                External::FromDependencyOf(pid2, t2.version_set(), pid1, t1.version_set())
                    .display(package_store, dp)
                    .to_string()
            }
            ref slice => {
                let str_terms: Vec<_> = slice.iter().map(|(_, p, t)| format!("{p} {t}")).collect();
                str_terms.join(", ") + " are incompatible"
            }
        }
    }

    fn format_external(
        &self,
        external: &External<&'static str>,
        package_store: &Store,
        dp: &Dp,
    ) -> String {
        match *external {
            External::NotRoot(package_id, version_index) => {
                let pkg = package_store.pkg(package_id).unwrap();
                format!(
                    "we are solving dependencies of {}",
                    dp.package_version_display(pkg, version_index),
                )
            }
            External::NoVersions(package_id, set) => {
                let pkg = package_store.pkg(package_id).unwrap();
                if set == VersionSet::full() {
                    format!("there is no available version for {pkg}")
                } else {
                    format!(
                        "there is no version of {}",
                        dp.package_version_set_display(pkg, set),
                    )
                }
            }
            External::Custom(package_id, set, ref reason) => {
                let pkg = package_store.pkg(package_id).unwrap();
                if set == VersionSet::full() {
                    format!("dependencies of {pkg} are unavailable because {reason}")
                } else {
                    format!(
                        "dependencies of {} are unavailable because {reason}",
                        dp.package_version_set_display(pkg, set),
                    )
                }
            }
            External::FromDependencyOf(package_id, package_set, dep_id, dep_set) => {
                let pkg = package_store.pkg(package_id).unwrap();
                let dep_pkg = package_store.pkg(dep_id).unwrap();
                if package_set == VersionSet::full() && dep_set == VersionSet::full() {
                    format!("{pkg} depends on {dep_pkg}")
                } else if package_set == VersionSet::full() {
                    format!(
                        "{pkg} depends on {}",
                        dp.package_version_set_display(dep_pkg, dep_set),
                    )
                } else if dep_set == VersionSet::full() {
                    if pkg.inner_pkg() == Some(&CustomPackage::Root) {
                        // Exclude the dummy version for root packages
                        format!("{pkg} depends on {dep_pkg}")
                    } else {
                        format!(
                            "{} depends on {dep_pkg}",
                            dp.package_version_set_display(pkg, package_set),
                        )
                    }
                } else if pkg.inner_pkg() == Some(&CustomPackage::Root) {
                    // Exclude the dummy version for root packages
                    format!(
                        "{pkg} depends on {}",
                        dp.package_version_set_display(dep_pkg, dep_set),
                    )
                } else {
                    format!(
                        "{} depends on {}",
                        dp.package_version_set_display(pkg, package_set),
                        dp.package_version_set_display(dep_pkg, dep_set),
                    )
                }
            }
        }
    }

    /// Simplest case, we just combine two external incompatibilities.
    fn explain_both_external(
        &self,
        external1: &External<&'static str>,
        external2: &External<&'static str>,
        current_terms: &Map<PackageId, Term>,
        package_store: &Store,
        dp: &Dp,
    ) -> String {
        // TODO: order should be chosen to make it more logical.
        format!(
            "Because {} and {}, {}.",
            self.format_external(external1, package_store, dp),
            self.format_external(external2, package_store, dp),
            self.format_terms(current_terms, package_store, dp)
        )
    }

    /// Both causes have already been explained so we use their refs.
    fn explain_both_ref(
        &self,
        ref_id1: usize,
        derived1: &Derived<&'static str>,
        ref_id2: usize,
        derived2: &Derived<&'static str>,
        current_terms: &Map<PackageId, Term>,
        package_store: &Store,
        dp: &Dp,
    ) -> String {
        // TODO: order should be chosen to make it more logical.
        format!(
            "Because {} ({}) and {} ({}), {}.",
            self.format_terms(&derived1.terms, package_store, dp),
            ref_id1,
            self.format_terms(&derived2.terms, package_store, dp),
            ref_id2,
            self.format_terms(current_terms, package_store, dp)
        )
    }

    /// One cause is derived (already explained so one-line),
    /// the other is a one-line external cause,
    /// and finally we conclude with the current incompatibility.
    fn explain_ref_and_external(
        &self,
        ref_id: usize,
        derived: &Derived<&'static str>,
        external: &External<&'static str>,
        current_terms: &Map<PackageId, Term>,
        package_store: &Store,
        dp: &Dp,
    ) -> String {
        // TODO: order should be chosen to make it more logical.
        format!(
            "Because {} ({}) and {}, {}.",
            self.format_terms(&derived.terms, package_store, dp),
            ref_id,
            self.format_external(external, package_store, dp),
            self.format_terms(current_terms, package_store, dp)
        )
    }

    /// Add an external cause to the chain of explanations.
    fn and_explain_external(
        &self,
        external: &External<&'static str>,
        current_terms: &Map<PackageId, Term>,
        package_store: &Store,
        dp: &Dp,
    ) -> String {
        format!(
            "And because {}, {}.",
            self.format_external(external, package_store, dp),
            self.format_terms(current_terms, package_store, dp)
        )
    }

    /// Add an already explained incompat to the chain of explanations.
    fn and_explain_ref(
        &self,
        ref_id: usize,
        derived: &Derived<&'static str>,
        current_terms: &Map<PackageId, Term>,
        package_store: &Store,
        dp: &Dp,
    ) -> String {
        format!(
            "And because {} ({}), {}.",
            self.format_terms(&derived.terms, package_store, dp),
            ref_id,
            self.format_terms(current_terms, package_store, dp)
        )
    }

    /// Add an already explained incompat to the chain of explanations.
    fn and_explain_prior_and_external(
        &self,
        prior_external: &External<&'static str>,
        external: &External<&'static str>,
        current_terms: &Map<PackageId, Term>,
        package_store: &Store,
        dp: &Dp,
    ) -> String {
        format!(
            "And because {} and {}, {}.",
            self.format_external(prior_external, package_store, dp),
            self.format_external(external, package_store, dp),
            self.format_terms(current_terms, package_store, dp)
        )
    }
}

fn main() {
    let mut dependency_provider =
        OfflineDependencyProvider::<CustomPackage, Ranges<SemanticVersion>>::new();

    // Define the root package with a dependency on a package we do not provide
    dependency_provider.add_dependencies(
        CustomPackage::Root,
        (0, 0, 0),
        vec![(
            CustomPackage::Package("foo".to_string()),
            Ranges::singleton((1, 0, 0)),
        )],
    );

    // Run the algorithm
    match dependency_provider.resolve(CustomPackage::Root, (0, 0, 0)) {
        Ok(sol) => println!("{:?}", sol),
        Err(PubGrubError::NoSolution(error)) => {
            eprintln!("No solution.\n");

            eprintln!("### Default report:");
            eprintln!("```");
            eprintln!(
                "{}",
                DefaultStringReporter::report(&error, &dependency_provider)
            );
            eprintln!("```\n");

            eprintln!("### Report with custom formatter:");
            eprintln!("```");
            eprintln!(
                "{}",
                DefaultStringReporter::report_with_formatter(
                    &error,
                    &CustomReportFormatter,
                    &dependency_provider
                )
            );
            eprintln!("```");
            std::process::exit(1);
        }
        Err(err) => panic!("{:?}", err),
    };
}
