// SPDX-License-Identifier: MPL-2.0

//! An incompatibility is a set of terms for different packages
//! that should never be satisfied all together.

use std::fmt::{self, Debug, Display};
use std::sync::Arc;

use crate::internal::arena::{Arena, Id};
use crate::internal::small_map::SmallMap;
use crate::{
    term, DefaultStringReportFormatter, DependencyProvider, DerivationTree, Derived, External, Map,
    Package, ReportFormatter, Set, Term, VersionSet,
};

/// An incompatibility is a set of terms for different packages
/// that should never be satisfied all together.
/// An incompatibility usually originates from a package dependency.
/// For example, if package A at version 1 depends on package B
/// at version 2, you can never have both terms `A = 1`
/// and `not B = 2` satisfied at the same time in a partial solution.
/// This would mean that we found a solution with package A at version 1
/// but not with package B at version 2.
/// Yet A at version 1 depends on B at version 2 so this is not possible.
/// Therefore, the set `{ A = 1, not B = 2 }` is an incompatibility,
/// defined from dependencies of A at version 1.
///
/// Incompatibilities can also be derived from two other incompatibilities
/// during conflict resolution. More about all this in
/// [PubGrub documentation](https://github.com/dart-lang/pub/blob/master/doc/solver.md#incompatibility).
#[derive(Debug, Clone)]
pub(crate) struct Incompatibility<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> {
    package_terms: SmallMap<P, Term<VS>>,
    kind: Kind<P, VS, M>,
}

/// Type alias of unique identifiers for incompatibilities.
pub(crate) type IncompId<P, VS, M> = Id<Incompatibility<P, VS, M>>;

pub(crate) type IncompDpId<DP> = IncompId<
    <DP as DependencyProvider>::P,
    <DP as DependencyProvider>::VS,
    <DP as DependencyProvider>::M,
>;

#[derive(Debug, Clone)]
enum Kind<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> {
    /// Initial incompatibility aiming at picking the root package for the first decision.
    ///
    /// This incompatibility drives the resolution, it requires that we pick the (virtual) root
    /// packages.
    NotRoot(P, VS::V),
    /// There are no versions in the given range for this package.
    ///
    /// This incompatibility is used when we tried all versions in a range and no version
    /// worked, so we have to backtrack
    NoVersions(P, VS),
    /// Incompatibility coming from the dependencies of a given package.
    ///
    /// If a@1 depends on b>=1,<2, we create an incompatibility with terms `{a 1, b <1,>=2}` with
    /// kind `FromDependencyOf(a, 1, b, >=1,<2)`.
    ///
    /// We can merge multiple dependents with the same version. For example, if a@1 depends on b and
    /// a@2 depends on b, we can say instead a@1||2 depends on b.
    FromDependencyOf(P, VS, P, VS),
    /// Derived from two causes. Stores cause ids.
    ///
    /// For example, if a -> b and b -> c, we can derive a -> c.
    DerivedFrom(IncompId<P, VS, M>, IncompId<P, VS, M>),
    /// The package is unavailable for reasons outside pubgrub.
    ///
    /// Examples:
    /// * The version would require building the package, but builds are disabled.
    /// * The package is not available in the cache, but internet access has been disabled.
    Custom(P, VS, M),
}

/// A Relation describes how a set of terms can be compared to an incompatibility.
/// Typically, the set of terms comes from the partial solution.
#[derive(Eq, PartialEq, Debug)]
pub(crate) enum Relation<P: Package> {
    /// We say that a set of terms S satisfies an incompatibility I
    /// if S satisfies every term in I.
    Satisfied,
    /// We say that S contradicts I
    /// if S contradicts at least one term in I.
    Contradicted(P),
    /// If S satisfies all but one of I's terms and is inconclusive for the remaining term,
    /// we say S "almost satisfies" I and we call the remaining term the "unsatisfied term".
    AlmostSatisfied(P),
    /// Otherwise, we say that their relation is inconclusive.
    Inconclusive,
}

impl<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> Incompatibility<P, VS, M> {
    /// Create the initial "not Root" incompatibility.
    pub(crate) fn not_root(package: P, version: VS::V) -> Self {
        Self {
            package_terms: SmallMap::One([(
                package.clone(),
                Term::Negative(VS::singleton(version.clone())),
            )]),
            kind: Kind::NotRoot(package, version),
        }
    }

    /// Create an incompatibility to remember that a given set does not contain any version.
    pub(crate) fn no_versions(package: P, term: Term<VS>) -> Self {
        let set = match &term {
            Term::Positive(r) => r.clone(),
            Term::Negative(_) => panic!("No version should have a positive term"),
        };
        Self {
            package_terms: SmallMap::One([(package.clone(), term)]),
            kind: Kind::NoVersions(package, set),
        }
    }

    /// Create an incompatibility for a reason outside pubgrub.
    #[allow(dead_code)] // Used by uv
    pub(crate) fn custom_term(package: P, term: Term<VS>, metadata: M) -> Self {
        let set = match &term {
            Term::Positive(r) => r.clone(),
            Term::Negative(_) => panic!("No version should have a positive term"),
        };
        Self {
            package_terms: SmallMap::One([(package.clone(), term)]),
            kind: Kind::Custom(package, set, metadata),
        }
    }

    /// Create an incompatibility for a reason outside pubgrub.
    pub(crate) fn custom_version(package: P, version: VS::V, metadata: M) -> Self {
        let set = VS::singleton(version);
        let term = Term::Positive(set.clone());
        Self {
            package_terms: SmallMap::One([(package.clone(), term)]),
            kind: Kind::Custom(package, set, metadata),
        }
    }

    /// Build an incompatibility from a given dependency.
    pub(crate) fn from_dependency(package: P, versions: VS, dep: (P, VS)) -> Self {
        let (p2, set2) = dep;
        Self {
            package_terms: if set2 == VS::empty() {
                SmallMap::One([(package.clone(), Term::Positive(versions.clone()))])
            } else {
                SmallMap::Two([
                    (package.clone(), Term::Positive(versions.clone())),
                    (p2.clone(), Term::Negative(set2.clone())),
                ])
            },
            kind: Kind::FromDependencyOf(package, versions, p2, set2),
        }
    }

    pub(crate) fn as_dependency(&self) -> Option<(&P, &P)> {
        match &self.kind {
            Kind::FromDependencyOf(p1, _, p2, _) => Some((p1, p2)),
            _ => None,
        }
    }

    /// Merge dependant versions with the same dependency.
    ///
    /// When multiple versions of a package depend on the same range of another package,
    /// we can merge the two into a single incompatibility.
    /// For example, if a@1 depends on b and a@2 depends on b, we can say instead
    /// a@1||2 depends on b.
    ///
    /// It is a special case of prior cause computation where the unified package
    /// is the common dependant in the two incompatibilities expressing dependencies.
    pub(crate) fn merge_dependents(&self, other: &Self) -> Option<Self> {
        // It is almost certainly a bug to call this method without checking that self is a dependency
        debug_assert!(self.as_dependency().is_some());
        // Check that both incompatibilities are of the shape p1 depends on p2,
        // with the same p1 and p2.
        let self_pkgs = self.as_dependency()?;
        if self_pkgs != other.as_dependency()? {
            return None;
        }
        let (p1, p2) = self_pkgs;
        let dep_term = self.get(p2);
        // The dependency range for p2 must be the same in both case
        // to be able to merge multiple p1 ranges.
        if dep_term != other.get(p2) {
            return None;
        }
        return Some(Self::from_dependency(
            p1.clone(),
            self.get(p1)
                .unwrap()
                .unwrap_positive()
                .union(other.get(p1).unwrap().unwrap_positive()), // It is safe to `simplify` here
            (
                p2.clone(),
                dep_term.map_or(VS::empty(), |v| v.unwrap_negative().clone()),
            ),
        ));
    }

    /// Prior cause of two incompatibilities using the rule of resolution.
    pub(crate) fn prior_cause(
        incompat: Id<Self>,
        satisfier_cause: Id<Self>,
        package: &P,
        incompatibility_store: &Arena<Self>,
    ) -> Self {
        let kind = Kind::DerivedFrom(incompat, satisfier_cause);
        // Optimization to avoid cloning and dropping t1
        let (t1, mut package_terms) = incompatibility_store[incompat]
            .package_terms
            .split_one(package)
            .unwrap();
        let satisfier_cause_terms = &incompatibility_store[satisfier_cause].package_terms;
        package_terms.merge(
            satisfier_cause_terms.iter().filter(|(p, _)| p != &package),
            |t1, t2| Some(t1.intersection(t2)),
        );
        let term = t1.union(satisfier_cause_terms.get(package).unwrap());
        if term != Term::any() {
            package_terms.insert(package.clone(), term);
        }
        Self {
            package_terms,
            kind,
        }
    }

    /// Check if an incompatibility should mark the end of the algorithm
    /// because it satisfies the root package.
    pub(crate) fn is_terminal(&self, root_package: &P, root_version: &VS::V) -> bool {
        if self.package_terms.len() == 0 {
            true
        } else if self.package_terms.len() > 1 {
            false
        } else {
            let (package, term) = self.package_terms.iter().next().unwrap();
            (package == root_package) && term.contains(root_version)
        }
    }

    /// Get the term related to a given package (if it exists).
    pub(crate) fn get(&self, package: &P) -> Option<&Term<VS>> {
        self.package_terms.get(package)
    }

    /// Iterate over packages.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&P, &Term<VS>)> {
        self.package_terms.iter()
    }

    // Reporting ###############################################################

    /// Retrieve parent causes if of type DerivedFrom.
    pub(crate) fn causes(&self) -> Option<(Id<Self>, Id<Self>)> {
        match self.kind {
            Kind::DerivedFrom(id1, id2) => Some((id1, id2)),
            _ => None,
        }
    }

    /// Build a derivation tree for error reporting.
    pub(crate) fn build_derivation_tree(
        self_id: Id<Self>,
        shared_ids: &Set<Id<Self>>,
        store: &Arena<Self>,
        precomputed: &Map<Id<Self>, Arc<DerivationTree<P, VS, M>>>,
    ) -> DerivationTree<P, VS, M> {
        match store[self_id].kind.clone() {
            Kind::DerivedFrom(id1, id2) => {
                let derived = Derived {
                    terms: store[self_id].package_terms.as_map(),
                    shared_id: shared_ids.get(&self_id).map(|id| id.into_raw()),
                    cause1: precomputed
                        .get(&id1)
                        .expect("Non-topological calls building tree")
                        .clone(),
                    cause2: precomputed
                        .get(&id2)
                        .expect("Non-topological calls building tree")
                        .clone(),
                };
                DerivationTree::Derived(derived)
            }
            Kind::NotRoot(package, version) => {
                DerivationTree::External(External::NotRoot(package, version))
            }
            Kind::NoVersions(package, set) => {
                DerivationTree::External(External::NoVersions(package.clone(), set.clone()))
            }
            Kind::FromDependencyOf(package, set, dep_package, dep_set) => {
                DerivationTree::External(External::FromDependencyOf(
                    package.clone(),
                    set.clone(),
                    dep_package.clone(),
                    dep_set.clone(),
                ))
            }
            Kind::Custom(package, set, metadata) => DerivationTree::External(External::Custom(
                package.clone(),
                set.clone(),
                metadata.clone(),
            )),
        }
    }
}

impl<'a, P: Package, VS: VersionSet + 'a, M: Eq + Clone + Debug + Display + 'a>
    Incompatibility<P, VS, M>
{
    /// CF definition of Relation enum.
    pub(crate) fn relation(&self, terms: impl Fn(&P) -> Option<&'a Term<VS>>) -> Relation<P> {
        let mut relation = Relation::Satisfied;
        for (package, incompat_term) in self.package_terms.iter() {
            match terms(package).map(|term| incompat_term.relation_with(term)) {
                Some(term::Relation::Satisfied) => {}
                Some(term::Relation::Contradicted) => {
                    return Relation::Contradicted(package.clone());
                }
                None | Some(term::Relation::Inconclusive) => {
                    // If a package is not present, the intersection is the same as [Term::any].
                    // According to the rules of satisfactions, the relation would be inconclusive.
                    // It could also be satisfied if the incompatibility term was also [Term::any],
                    // but we systematically remove those from incompatibilities
                    // so we're safe on that front.
                    if relation == Relation::Satisfied {
                        relation = Relation::AlmostSatisfied(package.clone());
                    } else {
                        return Relation::Inconclusive;
                    }
                }
            }
        }
        relation
    }
}

impl<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> Display
    for Incompatibility<P, VS, M>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            ReportFormatter::<P, VS, M>::format_terms(
                &DefaultStringReportFormatter,
                &self.package_terms.as_map()
            )
        )
    }
}

// TESTS #######################################################################

#[cfg(test)]
pub(crate) mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::term::tests::strategy as term_strat;
    use crate::Range;

    proptest! {

        /// For any three different packages p1, p2 and p3,
        /// for any three terms t1, t2 and t3,
        /// if we have the two following incompatibilities:
        ///    { p1: t1, p2: not t2 }
        ///    { p2: t2, p3: t3 }
        /// the rule of resolution says that we can deduce the following incompatibility:
        ///    { p1: t1, p3: t3 }
        #[test]
        fn rule_of_resolution(t1 in term_strat(), t2 in term_strat(), t3 in term_strat()) {
            let mut store = Arena::new();
            let i1 = store.alloc(Incompatibility {
                package_terms: SmallMap::Two([("p1", t1.clone()), ("p2", t2.negate())]),
                kind: Kind::<_, _, String>::FromDependencyOf("p1", Range::full(), "p2", Range::full())
            });

            let i2 = store.alloc(Incompatibility {
                package_terms: SmallMap::Two([("p2", t2), ("p3", t3.clone())]),
                kind: Kind::<_, _, String>::FromDependencyOf("p2", Range::full(), "p3", Range::full())
            });

            let mut i3 = Map::default();
            i3.insert("p1", t1);
            i3.insert("p3", t3);

            let i_resolution = Incompatibility::prior_cause(i1, i2, &"p2", &store);
            assert_eq!(i_resolution.package_terms.as_map(), i3);
        }

    }
}
