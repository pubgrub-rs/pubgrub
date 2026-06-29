// SPDX-License-Identifier: MPL-2.0

//! An incompatibility is a set of terms for different packages
//! that should never be satisfied all together.

use std::fmt::{Debug, Display};
use std::sync::Arc;

use crate::internal::{Arena, DecisionLevel, HashArena, Id, SmallMap};
use crate::{
    DependencyProvider, DerivationTree, Derived, External, Map, Package, Set, Term, VersionSet,
    term,
};

#[derive(Debug, Clone)]
struct ContradictionCache {
    /// The decision level where this incompatibility became contradicted.
    decision_level: DecisionLevel,
    /// The backtrack generation for that decision level.
    backtrack_generation: u32,
}

impl ContradictionCache {
    fn not_contradicted() -> Self {
        Self {
            decision_level: DecisionLevel::MAX,
            backtrack_generation: 0,
        }
    }

    fn is_contradicted(&self, last_valid_decision_levels: &[DecisionLevel]) -> bool {
        // The active generation has no backtrack target yet, so every contradiction recorded in
        // it remains valid. Completed generations store the highest decision level that survived
        // their first invalidating backtrack.
        last_valid_decision_levels
            .get(self.backtrack_generation as usize)
            .map(|&level| self.decision_level <= level)
            .unwrap_or(true)
    }
}

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
pub struct Incompatibility<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> {
    package_terms: SmallMap<Id<P>, Term<VS>>,
    /// The reason for the incompatibility.
    pub kind: Kind<P, VS, M>,
    contradiction_cache: ContradictionCache,
}

/// Type alias of unique identifiers for incompatibilities.
pub type IncompId<P, VS, M> = Id<Incompatibility<P, VS, M>>;

pub(crate) type IncompDpId<DP> = IncompId<
    <DP as DependencyProvider>::P,
    <DP as DependencyProvider>::VS,
    <DP as DependencyProvider>::M,
>;

/// The reason for the incompatibility.
#[derive(Debug, Clone)]
pub enum Kind<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> {
    /// Initial incompatibility aiming at picking the root package for the first decision.
    ///
    /// This incompatibility drives the resolution, it requires that we pick the (virtual) root
    /// packages.
    NotRoot(Id<P>, VS::V),
    /// There are no versions in the given range for this package.
    ///
    /// This incompatibility is used when we tried all versions in a range and no version
    /// worked, so we have to backtrack
    NoVersions(Id<P>, VS),
    /// Incompatibility coming from the dependencies of a given package.
    ///
    /// If a@1 depends on b>=1,<2, we create an incompatibility with terms `{a 1, b <1,>=2}` with
    /// kind `FromDependencyOf(a, b)`. The version sets are stored in the incompatibility terms.
    ///
    /// We can merge multiple dependents with the same version. For example, if a@1 depends on b and
    /// a@2 depends on b, we can say instead a@1||2 depends on b.
    FromDependencyOf(Id<P>, Id<P>),
    /// Derived from two causes. Stores cause ids.
    ///
    /// For example, if a -> b and b -> c, we can derive a -> c.
    DerivedFrom(IncompId<P, VS, M>, IncompId<P, VS, M>),
    /// The package is unavailable for reasons outside pubgrub.
    ///
    /// Examples:
    /// * The version would require building the package, but builds are disabled.
    /// * The package is not available in the cache, but internet access has been disabled.
    Custom(Id<P>, VS, M),
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
    Contradicted(Id<P>),
    /// If S satisfies all but one of I's terms and is inconclusive for the remaining term,
    /// we say S "almost satisfies" I and we call the remaining term the "unsatisfied term".
    AlmostSatisfied(Id<P>),
    /// Otherwise, we say that their relation is inconclusive.
    Inconclusive,
}

impl<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> Incompatibility<P, VS, M> {
    /// Create the initial "not Root" incompatibility.
    pub(crate) fn not_root(package: Id<P>, version: VS::V) -> Self {
        Self {
            package_terms: SmallMap::One([(
                package,
                Term::Negative(VS::singleton(version.clone())),
            )]),
            kind: Kind::NotRoot(package, version),
            contradiction_cache: ContradictionCache::not_contradicted(),
        }
    }

    /// Create an incompatibility to remember that a given set does not contain any version.
    pub fn no_versions(package: Id<P>, term: Term<VS>) -> Self {
        let set = match &term {
            Term::Positive(r) => r.clone(),
            Term::Negative(_) => panic!("No version should have a positive term"),
        };
        Self {
            package_terms: SmallMap::One([(package, term)]),
            kind: Kind::NoVersions(package, set),
            contradiction_cache: ContradictionCache::not_contradicted(),
        }
    }

    /// Create an incompatibility for a reason outside pubgrub.
    #[allow(dead_code)] // Used by uv
    pub fn custom_term(package: Id<P>, term: Term<VS>, metadata: M) -> Self {
        let set = match &term {
            Term::Positive(r) => r.clone(),
            Term::Negative(_) => panic!("No version should have a positive term"),
        };
        Self {
            package_terms: SmallMap::One([(package, term)]),
            kind: Kind::Custom(package, set, metadata),
            contradiction_cache: ContradictionCache::not_contradicted(),
        }
    }

    /// Create an incompatibility for a reason outside pubgrub.
    pub fn custom_version(package: Id<P>, version: VS::V, metadata: M) -> Self {
        let set = VS::singleton(version);
        let term = Term::Positive(set.clone());
        Self {
            package_terms: SmallMap::One([(package, term)]),
            kind: Kind::Custom(package, set, metadata),
            contradiction_cache: ContradictionCache::not_contradicted(),
        }
    }

    /// Build an incompatibility from a given dependency.
    pub fn from_dependency(package: Id<P>, versions: VS, dep: (Id<P>, VS)) -> Self {
        let (p2, set2) = dep;
        Self {
            package_terms: if set2 == VS::empty() {
                SmallMap::One([(package, Term::Positive(versions))])
            } else {
                SmallMap::Two([
                    (package, Term::Positive(versions)),
                    (p2, Term::Negative(set2)),
                ])
            },
            kind: Kind::FromDependencyOf(package, p2),
            contradiction_cache: ContradictionCache::not_contradicted(),
        }
    }

    fn dependency_terms(&self, p1: Id<P>, p2: Id<P>) -> (&VS, Option<&VS>) {
        let mut terms = self.package_terms.iter();
        let versions = match terms.next() {
            Some((term_package, Term::Positive(versions))) if *term_package == p1 => versions,
            _ => panic!("dependency incompatibility must start with its positive term"),
        };
        let dependency_versions = match terms.next() {
            None => None,
            Some((term_package, Term::Negative(versions))) if *term_package == p2 => Some(versions),
            _ => panic!("dependency incompatibility must end with its negative term"),
        };
        assert!(
            terms.next().is_none(),
            "dependency incompatibility must contain at most two terms"
        );
        (versions, dependency_versions)
    }

    pub(crate) fn as_dependency(&self) -> Option<(Id<P>, Id<P>, Option<&VS>)> {
        match &self.kind {
            Kind::FromDependencyOf(p1, p2) => {
                let (_, dependency_range) = self.dependency_terms(*p1, *p2);
                Some((*p1, *p2, dependency_range))
            }
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
        let (p1, p2, _) = self.as_dependency()?;
        let (other_p1, other_p2, _) = other.as_dependency()?;
        if (p1, p2) != (other_p1, other_p2) {
            return None;
        }
        // We ignore self-dependencies. They are always either trivially true or trivially false,
        // as the package version implies whether the constraint will always be fulfilled or always
        // violated.
        // At time of writing, the public crate API only allowed a map of dependencies,
        // meaning it can't hit this branch, which requires two self-dependencies.
        if p1 == p2 {
            return None;
        }
        let dep_term = self.get(p2);
        // The dependency range for p2 must be the same in both case
        // to be able to merge multiple p1 ranges.
        if dep_term != other.get(p2) {
            return None;
        }
        Some(Self::from_dependency(
            p1,
            self.get(p1)
                .unwrap()
                .unwrap_positive()
                .union(other.get(p1).unwrap().unwrap_positive()), // It is safe to `simplify` here
            (
                p2,
                dep_term.map_or(VS::empty(), |v| v.unwrap_negative().clone()),
            ),
        ))
    }

    /// Prior cause of two incompatibilities using the rule of resolution.
    pub(crate) fn prior_cause(
        incompat: Id<Self>,
        satisfier_cause: Id<Self>,
        package: Id<P>,
        incompatibility_store: &Arena<Self>,
    ) -> Self {
        let kind = Kind::DerivedFrom(incompat, satisfier_cause);
        // Optimization to avoid cloning and dropping t1
        let (t1, mut package_terms) = incompatibility_store[incompat]
            .package_terms
            .split_one(&package)
            .unwrap();
        let satisfier_cause_terms = &incompatibility_store[satisfier_cause].package_terms;
        package_terms.merge(
            satisfier_cause_terms.iter().filter(|(p, _)| p != &&package),
            |t1, t2| Some(t1.intersection(t2)),
        );
        let term = t1.union(satisfier_cause_terms.get(&package).unwrap());
        if term != Term::any() {
            package_terms.insert(package, term);
        }
        Self {
            package_terms,
            kind,
            contradiction_cache: ContradictionCache::not_contradicted(),
        }
    }

    pub(crate) fn is_contradicted(&self, last_valid_decision_levels: &[DecisionLevel]) -> bool {
        self.contradiction_cache
            .is_contradicted(last_valid_decision_levels)
    }

    pub(crate) fn mark_contradicted(
        &mut self,
        decision_level: DecisionLevel,
        backtrack_generation: u32,
    ) {
        self.contradiction_cache = ContradictionCache {
            decision_level,
            backtrack_generation,
        };
    }

    pub(crate) fn reset_contradiction_cache(&mut self) {
        self.contradiction_cache = ContradictionCache::not_contradicted();
    }

    /// Check if an incompatibility should mark the end of the algorithm
    /// because it satisfies the root package.
    pub(crate) fn is_terminal(&self, root_package: Id<P>, root_version: &VS::V) -> bool {
        if self.package_terms.len() == 0 {
            true
        } else if self.package_terms.len() > 1 {
            false
        } else {
            let (package, term) = self.package_terms.iter().next().unwrap();
            (package == &root_package) && term.contains(root_version)
        }
    }

    /// Get the term related to a given package (if it exists).
    pub(crate) fn get(&self, package: Id<P>) -> Option<&Term<VS>> {
        self.package_terms.get(&package)
    }

    /// Iterate over packages.
    pub fn iter(&self) -> impl Iterator<Item = (Id<P>, &Term<VS>)> {
        self.package_terms
            .iter()
            .map(|(package, term)| (*package, term))
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
        package_store: &HashArena<P>,
        precomputed: &Map<Id<Self>, Arc<DerivationTree<P, VS, M>>>,
    ) -> DerivationTree<P, VS, M> {
        match store[self_id].kind.clone() {
            Kind::DerivedFrom(id1, id2) => {
                let derived: Derived<P, VS, M> = Derived {
                    terms: store[self_id]
                        .package_terms
                        .iter()
                        .map(|(&a, b)| (package_store[a].clone(), b.clone()))
                        .collect(),
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
                DerivationTree::External(External::NotRoot(package_store[package].clone(), version))
            }
            Kind::NoVersions(package, set) => DerivationTree::External(External::NoVersions(
                package_store[package].clone(),
                set.clone(),
            )),
            Kind::FromDependencyOf(package, dep_package) => {
                let (package_versions, dependency_versions) =
                    store[self_id].dependency_terms(package, dep_package);
                DerivationTree::External(External::FromDependencyOf(
                    package_store[package].clone(),
                    package_versions.clone(),
                    package_store[dep_package].clone(),
                    dependency_versions.cloned().unwrap_or_else(VS::empty),
                ))
            }
            Kind::Custom(package, set, metadata) => DerivationTree::External(External::Custom(
                package_store[package].clone(),
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
    pub(crate) fn relation(&self, terms: impl Fn(Id<P>) -> Option<&'a Term<VS>>) -> Relation<P> {
        let mut relation = Relation::Satisfied;
        for (&package, incompat_term) in self.package_terms.iter() {
            match terms(package).map(|term| incompat_term.relation_with(term)) {
                Some(term::Relation::Satisfied) => {}
                Some(term::Relation::Contradicted) => {
                    return Relation::Contradicted(package);
                }
                None | Some(term::Relation::Inconclusive) => {
                    // If a package is not present, the intersection is the same as [Term::any].
                    // According to the rules of satisfactions, the relation would be inconclusive.
                    // It could also be satisfied if the incompatibility term was also [Term::any],
                    // but we systematically remove those from incompatibilities
                    // so we're safe on that front.
                    if relation == Relation::Satisfied {
                        relation = Relation::AlmostSatisfied(package);
                    } else {
                        return Relation::Inconclusive;
                    }
                }
            }
        }
        relation
    }
}

impl<P: Package, VS: VersionSet, M: Eq + Clone + Debug + Display> Incompatibility<P, VS, M> {
    /// Display the incompatibility.
    pub fn display<'a>(&'a self, package_store: &'a HashArena<P>) -> impl Display + 'a {
        match self.iter().collect::<Vec<_>>().as_slice() {
            [] => "version solving failed".into(),
            // TODO: special case when that unique package is root.
            [(package, Term::Positive(range))] => {
                format!("{} {} is forbidden", package_store[*package], range)
            }
            [(package, Term::Negative(range))] => {
                format!("{} {} is mandatory", package_store[*package], range)
            }
            [
                (p_pos, Term::Positive(r_pos)),
                (p_neg, Term::Negative(r_neg)),
            ]
            | [
                (p_neg, Term::Negative(r_neg)),
                (p_pos, Term::Positive(r_pos)),
            ] => External::<_, _, M>::FromDependencyOf(
                &package_store[*p_pos],
                r_pos.clone(),
                &package_store[*p_neg],
                r_neg.clone(),
            )
            .to_string(),
            slice => {
                let str_terms: Vec<_> = slice
                    .iter()
                    .map(|(p, t)| format!("{} {}", package_store[*p], t))
                    .collect();
                str_terms.join(", ") + " are incompatible"
            }
        }
    }
}

// TESTS #######################################################################

#[cfg(test)]
pub(crate) mod tests {
    use proptest::prelude::*;
    use std::cmp::Reverse;
    use std::collections::BTreeMap;
    use std::fmt::{self, Formatter};

    use super::*;
    use crate::internal::State;
    use crate::term::tests::strategy as term_strat;
    use crate::{OfflineDependencyProvider, Ranges};

    #[test]
    fn contradiction_cache_tracks_backtrack_generations() {
        let current_generation = ContradictionCache {
            decision_level: DecisionLevel::new(3),
            backtrack_generation: 1,
        };

        // A generation without a recorded backtrack target is still active.
        assert!(current_generation.is_contradicted(&[DecisionLevel::ZERO]));
        // Backtracking below the contradiction's decision level invalidates it.
        assert!(!current_generation.is_contradicted(&[DecisionLevel::ZERO, DecisionLevel::new(2)]));
        // Backtracking to or above that decision level preserves it.
        assert!(current_generation.is_contradicted(&[DecisionLevel::ZERO, DecisionLevel::new(3)]));

        let later_generation = ContradictionCache {
            decision_level: DecisionLevel::new(5),
            backtrack_generation: 2,
        };
        assert!(later_generation.is_contradicted(&[DecisionLevel::ZERO, DecisionLevel::new(3)]));
        assert!(!later_generation.is_contradicted(&[
            DecisionLevel::ZERO,
            DecisionLevel::new(3),
            DecisionLevel::new(4),
        ]));
    }

    #[derive(Debug, Eq, PartialEq, Hash)]
    struct PanicOnCloneRanges(Ranges<usize>);

    impl Clone for PanicOnCloneRanges {
        fn clone(&self) -> Self {
            panic!("version set was cloned")
        }
    }

    impl Display for PanicOnCloneRanges {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            Display::fmt(&self.0, f)
        }
    }

    impl VersionSet for PanicOnCloneRanges {
        type V = usize;

        fn empty() -> Self {
            Self(Ranges::empty())
        }

        fn singleton(v: Self::V) -> Self {
            Self(Ranges::singleton(v))
        }

        fn complement(&self) -> Self {
            Self(self.0.complement())
        }

        fn intersection(&self, other: &Self) -> Self {
            Self(self.0.intersection(&other.0))
        }

        fn contains(&self, v: &Self::V) -> bool {
            self.0.contains(v)
        }
    }

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
            let mut package_store = HashArena::new();
            let p1 = package_store.alloc("p1");
            let p2 = package_store.alloc("p2");
            let p3 = package_store.alloc("p3");
            let i1 = store.alloc(Incompatibility {
                package_terms: SmallMap::Two([(p1, t1.clone()), (p2, t2.negate())]),
                kind: Kind::<_, _, String>::FromDependencyOf(p1, p2),
                contradiction_cache: ContradictionCache::not_contradicted(),
            });

            let i2 = store.alloc(Incompatibility {
                package_terms: SmallMap::Two([(p2, t2), (p3, t3.clone())]),
                kind: Kind::<_, _, String>::FromDependencyOf(p2, p3),
                contradiction_cache: ContradictionCache::not_contradicted(),
            });

            let mut i3 = Map::default();
            i3.insert(p1, t1);
            i3.insert(p3, t3);

            let i_resolution = Incompatibility::prior_cause(i1, i2, p2, &store);
            assert_eq!(i_resolution.package_terms.iter().map(|(&k, v)|(k, v.clone())).collect::<Map<_, _>>(), i3);
        }

    }

    #[test]
    fn from_dependency_does_not_clone_version_sets() {
        let mut package_store = HashArena::new();
        let package = package_store.alloc("package".to_string());
        let dependency = package_store.alloc("dependency".to_string());

        for dependency_versions in [
            PanicOnCloneRanges(Ranges::singleton(2usize)),
            PanicOnCloneRanges(Ranges::empty()),
        ] {
            let _: Incompatibility<String, PanicOnCloneRanges, String> =
                Incompatibility::from_dependency(
                    package,
                    PanicOnCloneRanges(Ranges::singleton(1usize)),
                    (dependency, dependency_versions),
                );
        }
    }

    fn assert_dependency_tree(
        package_store: &HashArena<String>,
        incompatibility: Incompatibility<String, Ranges<usize>, String>,
        expected_versions: &Ranges<usize>,
        expected_dependency: &str,
        expected_dependency_versions: &Ranges<usize>,
    ) {
        let mut store = Arena::new();
        let id = store.alloc(incompatibility);
        let tree = Incompatibility::build_derivation_tree(
            id,
            &Set::default(),
            &store,
            package_store,
            &Map::default(),
        );
        let DerivationTree::External(External::FromDependencyOf(
            actual_package,
            actual_versions,
            actual_dependency,
            actual_dependency_versions,
        )) = tree
        else {
            panic!("expected a dependency external")
        };
        assert_eq!(actual_package, "package");
        assert_eq!(&actual_versions, expected_versions);
        assert_eq!(actual_dependency, expected_dependency);
        assert_eq!(&actual_dependency_versions, expected_dependency_versions);
    }

    #[test]
    fn dependency_derivation_trees_reconstruct_ranges() {
        let mut package_store = HashArena::new();
        let package = package_store.alloc("package".to_string());
        let dependency = package_store.alloc("dependency".to_string());
        let versions = Ranges::between(1usize, 4usize);
        let other_versions = Ranges::singleton(4usize);
        let dependency_versions = Ranges::between(7usize, 10usize);

        assert_dependency_tree(
            &package_store,
            Incompatibility::from_dependency(
                package,
                versions.clone(),
                (dependency, dependency_versions.clone()),
            ),
            &versions,
            "dependency",
            &dependency_versions,
        );

        let empty = Ranges::empty();
        assert_dependency_tree(
            &package_store,
            Incompatibility::from_dependency(
                package,
                versions.clone(),
                (dependency, empty.clone()),
            ),
            &versions,
            "dependency",
            &empty,
        );

        assert_dependency_tree(
            &package_store,
            Incompatibility::from_dependency(
                package,
                versions.clone(),
                (package, dependency_versions.clone()),
            ),
            &versions,
            "package",
            &dependency_versions,
        );

        let first: Incompatibility<String, Ranges<usize>, String> =
            Incompatibility::from_dependency(
                package,
                versions.clone(),
                (dependency, dependency_versions.clone()),
            );
        let second = Incompatibility::from_dependency(
            package,
            other_versions.clone(),
            (dependency, dependency_versions.clone()),
        );
        let merged = first.merge_dependents(&second).unwrap();
        assert_dependency_tree(
            &package_store,
            merged,
            &versions.union(&other_versions),
            "dependency",
            &dependency_versions,
        );
    }

    /// Check that multiple self-dependencies are supported.
    ///
    /// The current public API deduplicates dependencies through a map, so we test them here
    /// manually.
    ///
    /// https://github.com/astral-sh/uv/issues/13344
    #[test]
    fn package_depend_on_self() {
        let cases: &[Vec<(String, Ranges<usize>)>] = &[
            vec![("foo".to_string(), Ranges::full())],
            vec![
                ("foo".to_string(), Ranges::full()),
                ("foo".to_string(), Ranges::full()),
            ],
            vec![
                ("foo".to_string(), Ranges::full()),
                ("foo".to_string(), Ranges::singleton(1usize)),
            ],
            vec![
                ("foo".to_string(), Ranges::singleton(1usize)),
                ("foo".to_string(), Ranges::from_range_bounds(1usize..2)),
                ("foo".to_string(), Ranges::from_range_bounds(1usize..3)),
            ],
        ];

        for case in cases {
            let mut state: State<OfflineDependencyProvider<String, Ranges<usize>>> =
                State::init("root".to_string(), 0);
            state.unit_propagation(state.root_package).unwrap();

            // Add the root package
            state.add_package_version_dependencies(
                state.root_package,
                0,
                Ranges::singleton(0usize),
                [("foo".to_string(), Ranges::singleton(1usize))],
            );
            state.unit_propagation(state.root_package).unwrap();

            // Add a package that depends on itself twice
            let (next, _) = state
                .partial_solution
                .pick_highest_priority_pkg(|_p, _r| (0, Reverse(0)))
                .unwrap();
            state.add_package_version_dependencies(
                next,
                1,
                Ranges::singleton(1usize),
                case.clone(),
            );
            state.unit_propagation(next).unwrap();

            assert!(
                state
                    .partial_solution
                    .pick_highest_priority_pkg(|_p, _r| (0, Reverse(0)))
                    .is_none()
            );

            let solution: BTreeMap<String, usize> = state
                .partial_solution
                .extract_solution()
                .map(|(p, v)| (state.package_store[p].clone(), v))
                .collect();
            let expected = BTreeMap::from([("root".to_string(), 0), ("foo".to_string(), 1)]);

            assert_eq!(solution, expected, "{case:?}");
        }
    }
}
