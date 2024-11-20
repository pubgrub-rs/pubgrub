// SPDX-License-Identifier: MPL-2.0

//! An incompatibility is a set of terms for different packages
//! that should never be satisfied all together.

use std::fmt::{self, Debug, Display};
use std::ops::{Index, IndexMut, Range};
use std::sync::Arc;

use crate::{
    internal::{DecisionLevel, SmallMap},
    term, DefaultStringReportFormatter, DependencyProvider, DerivationTree, Derived, External, Map,
    PackageArena, PackageId, ReportFormatter, Set, Term, VersionIndex, VersionSet,
};

/// Type for identifying incompatibilities.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub(crate) struct IncompatId(pub(crate) u32);

impl IncompatId {
    /// Convert `Range<Self>` to an iterator.
    pub(crate) fn range_to_iter(range: Range<Self>) -> impl Iterator<Item = Self> {
        (range.start.0..range.end.0).map(IncompatId)
    }
}

/// Incompatibility arena
#[derive(Debug, Clone)]
pub(crate) struct IncompatArena<M: Eq + Clone + Debug + Display>(Vec<Incompatibility<M>>);

impl<M: Eq + Clone + Debug + Display> IncompatArena<M> {
    /// Create an empty incompatibility arena.
    pub(crate) fn new() -> Self {
        Self(Default::default())
    }

    /// Push an incompatibility in the arena and get its identifier.
    pub(crate) fn push(&mut self, incompatibility: Incompatibility<M>) -> IncompatId {
        let len = self.0.len();
        self.0.push(incompatibility);
        IncompatId(len as u32)
    }

    /// Appends incompatibilities to the arena and get their identifier.
    pub(crate) fn extend(
        &mut self,
        incompatibilities: impl Iterator<Item = Incompatibility<M>>,
    ) -> Range<IncompatId> {
        let start = self.0.len();
        self.0.extend(incompatibilities);
        let end = self.0.len();
        IncompatId(start as u32)..IncompatId(end as u32)
    }
}

impl<M: Eq + Clone + Debug + Display> Index<IncompatId> for IncompatArena<M> {
    type Output = Incompatibility<M>;

    fn index(&self, index: IncompatId) -> &Self::Output {
        &self.0[index.0 as usize]
    }
}

impl<M: Eq + Clone + Debug + Display> IndexMut<IncompatId> for IncompatArena<M> {
    fn index_mut(&mut self, index: IncompatId) -> &mut Self::Output {
        &mut self.0[index.0 as usize]
    }
}

impl<M: Eq + Clone + Debug + Display> Index<Range<IncompatId>> for IncompatArena<M> {
    type Output = [Incompatibility<M>];

    fn index(&self, range: Range<IncompatId>) -> &Self::Output {
        &self.0[range.start.0 as usize..range.end.0 as usize]
    }
}

#[derive(Debug, Clone)]
struct ContradicationInfo {
    /// Store the decision level when the incompatibility was found to be contradicted.
    /// These will stay contradicted until we have backtracked beyond its associated decision level.
    decision_level: DecisionLevel,
    /// Store the backtrack generation for the decision level.
    backtrack_generation: u32,
}

impl ContradicationInfo {
    /// Construct a new value.
    fn new(decision_level: DecisionLevel, backtrack_generation: u32) -> Self {
        Self {
            decision_level,
            backtrack_generation,
        }
    }

    /// Construct a value interpreted as not contradicted.
    fn not_contradicted() -> Self {
        Self {
            decision_level: DecisionLevel::MAX,
            backtrack_generation: 0,
        }
    }

    /// Check for contradiction.
    fn is_contradicted(&self, last_valid_decision_levels: &[DecisionLevel]) -> bool {
        last_valid_decision_levels
            .get(self.backtrack_generation as usize)
            .map(|&l| self.decision_level <= l)
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
pub(crate) struct Incompatibility<M: Eq + Clone + Debug + Display> {
    package_terms: SmallMap<PackageId, Term>,
    kind: Kind<M>,
    contradiction_info: ContradicationInfo,
}

#[derive(Debug, Clone)]
enum Kind<M: Eq + Clone + Debug + Display> {
    /// Initial incompatibility aiming at picking the root package for the first decision.
    ///
    /// This incompatibility drives the resolution, it requires that we pick the (virtual) root
    /// packages.
    NotRoot(PackageId, VersionIndex),
    /// There are no versions in the given range for this package.
    ///
    /// This incompatibility is used when we tried all versions in a range and no version
    /// worked, so we have to backtrack
    NoVersions(PackageId, VersionSet),
    /// Incompatibility coming from the dependencies of a given package.
    ///
    /// If a@1 depends on b>=1,<2, we create an incompatibility with terms `{a 1, b <1,>=2}` with
    /// kind `FromDependencyOf(a, 1, b, >=1,<2)`.
    ///
    /// We can merge multiple dependents with the same version. For example, if a@1 depends on b and
    /// a@2 depends on b, we can say instead a@1||2 depends on b.
    FromDependencyOf(PackageId, VersionSet, PackageId, VersionSet),
    /// Derived from two causes. Stores cause ids.
    ///
    /// For example, if a -> b and b -> c, we can derive a -> c.
    DerivedFrom(IncompatId, IncompatId),
    /// The package is unavailable for reasons outside pubgrub.
    ///
    /// Examples:
    /// * The version would require building the package, but builds are disabled.
    /// * The package is not available in the cache, but internet access has been disabled.
    Custom(PackageId, VersionSet, M),
}

/// A Relation describes how a set of terms can be compared to an incompatibility.
/// Typically, the set of terms comes from the partial solution.
#[derive(Eq, PartialEq, Debug)]
pub(crate) enum Relation {
    /// We say that a set of terms S satisfies an incompatibility I
    /// if S satisfies every term in I.
    Satisfied,
    /// We say that S contradicts I
    /// if S contradicts at least one term in I.
    Contradicted(PackageId),
    /// If S satisfies all but one of I's terms and is inconclusive for the remaining term,
    /// we say S "almost satisfies" I and we call the remaining term the "unsatisfied term".
    AlmostSatisfied(PackageId),
    /// Otherwise, we say that their relation is inconclusive.
    Inconclusive,
}

impl<M: Eq + Clone + Debug + Display> Incompatibility<M> {
    /// Create the initial "not Root" incompatibility.
    pub(crate) fn not_root(package_id: PackageId, version_index: VersionIndex) -> Self {
        Self {
            package_terms: SmallMap::One([(
                package_id,
                Term::negative(VersionSet::singleton(version_index)),
            )]),
            kind: Kind::NotRoot(package_id, version_index),
            contradiction_info: ContradicationInfo::not_contradicted(),
        }
    }

    /// Create an incompatibility to remember that a given set does not contain any version.
    pub(crate) fn no_versions(package_id: PackageId, term: Term) -> Self {
        let set = if term.is_positive() {
            term.version_set()
        } else {
            panic!("No version should have a positive term")
        };
        Self {
            package_terms: SmallMap::One([(package_id, term)]),
            kind: Kind::NoVersions(package_id, set),
            contradiction_info: ContradicationInfo::not_contradicted(),
        }
    }

    /// Create an incompatibility for a reason outside pubgrub.
    #[allow(dead_code)] // Used by uv
    pub(crate) fn custom_term(package_id: PackageId, term: Term, metadata: M) -> Self {
        let set = if term.is_positive() {
            term.version_set()
        } else {
            panic!("No version should have a positive term")
        };
        Self {
            package_terms: SmallMap::One([(package_id, term)]),
            kind: Kind::Custom(package_id, set, metadata),
            contradiction_info: ContradicationInfo::not_contradicted(),
        }
    }

    /// Create an incompatibility for a reason outside pubgrub.
    pub(crate) fn custom_version(
        package_id: PackageId,
        version_index: VersionIndex,
        metadata: M,
    ) -> Self {
        let set = VersionSet::singleton(version_index);
        let term = Term::positive(set);
        Self {
            package_terms: SmallMap::One([(package_id, term)]),
            kind: Kind::Custom(package_id, set, metadata),
            contradiction_info: ContradicationInfo::not_contradicted(),
        }
    }

    /// Build an incompatibility from a given dependency.
    pub(crate) fn from_dependency(
        package_id: PackageId,
        vs: VersionSet,
        dep: (PackageId, VersionSet),
    ) -> Self {
        let (pid2, set2) = dep;
        Self {
            package_terms: if set2 == VersionSet::empty() {
                SmallMap::One([(package_id, Term::positive(vs))])
            } else {
                SmallMap::Two([
                    (package_id, Term::positive(vs)),
                    (pid2, Term::negative(set2)),
                ])
            },
            kind: Kind::FromDependencyOf(package_id, vs, pid2, set2),
            contradiction_info: ContradicationInfo::not_contradicted(),
        }
    }

    pub(crate) fn as_dependency(&self) -> Option<(PackageId, PackageId)> {
        match self.kind {
            Kind::FromDependencyOf(pid1, _, pid2, _) => Some((pid1, pid2)),
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
        let self_package_ids = self.as_dependency()?;
        if self_package_ids != other.as_dependency()? {
            return None;
        }
        let (pid1, pid2) = self_package_ids;
        let dep_term = self.get(pid2);
        // The dependency range for p2 must be the same in both case
        // to be able to merge multiple p1 ranges.
        if dep_term != other.get(pid2) {
            return None;
        }
        Some(Self::from_dependency(
            pid1,
            self.get(pid1)
                .unwrap()
                .unwrap_positive()
                .union(other.get(pid1).unwrap().unwrap_positive()),
            (
                pid2,
                dep_term.map_or(VersionSet::empty(), |v| v.unwrap_negative()),
            ),
        ))
    }

    /// Prior cause of two incompatibilities using the rule of resolution.
    pub(crate) fn prior_cause(
        incompat: IncompatId,
        satisfier_cause: IncompatId,
        package_id: PackageId,
        incompatibility_store: &IncompatArena<M>,
    ) -> Self {
        let kind = Kind::DerivedFrom(incompat, satisfier_cause);
        // Optimization to avoid cloning and dropping t1
        let (t1, mut package_terms) = incompatibility_store[incompat]
            .package_terms
            .split_one(&package_id)
            .unwrap();
        let satisfier_cause_terms = &incompatibility_store[satisfier_cause].package_terms;
        package_terms.merge(
            satisfier_cause_terms
                .iter()
                .filter(|(&pid, _)| pid != package_id),
            |&t1, &t2| Some(t1.intersection(t2)),
        );
        let term = t1.union(*satisfier_cause_terms.get(&package_id).unwrap());
        if term != Term::any() {
            package_terms.insert(package_id, term);
        }
        Self {
            package_terms,
            kind,
            contradiction_info: ContradicationInfo::not_contradicted(),
        }
    }

    /// Check if an incompatibility should mark the end of the algorithm
    /// because it satisfies the root package.
    pub(crate) fn is_terminal(
        &self,
        root_package_id: PackageId,
        root_version_index: VersionIndex,
    ) -> bool {
        if self.package_terms.len() == 0 {
            true
        } else if self.package_terms.len() > 1 {
            false
        } else {
            let (&package_id, term) = self.package_terms.iter().next().unwrap();
            (package_id == root_package_id) && term.contains(root_version_index)
        }
    }

    /// Check if an incompatibility is contradicted.
    pub(crate) fn is_contradicted(&self, last_valid_decision_levels: &[DecisionLevel]) -> bool {
        self.contradiction_info
            .is_contradicted(last_valid_decision_levels)
    }

    /// Set incompatibility contradication info.
    pub(crate) fn set_contradication_info(
        &mut self,
        decision_level: DecisionLevel,
        backtrack_generation: u32,
    ) {
        self.contradiction_info = ContradicationInfo::new(decision_level, backtrack_generation);
    }

    /// Get the term related to a given package (if it exists).
    pub(crate) fn get(&self, package_id: PackageId) -> Option<Term> {
        self.package_terms.get(&package_id).copied()
    }

    /// Iterate over packages.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (PackageId, Term)> + use<'_, M> {
        self.package_terms.iter().map(|(&k, &v)| (k, v))
    }

    /// Retrieve parent causes if of type DerivedFrom.
    pub(crate) fn causes(&self) -> Option<(IncompatId, IncompatId)> {
        match self.kind {
            Kind::DerivedFrom(id1, id2) => Some((id1, id2)),
            _ => None,
        }
    }

    /// Build a derivation tree for error reporting.
    pub(crate) fn build_derivation_tree(
        self_id: IncompatId,
        shared_ids: &Set<IncompatId>,
        store: &IncompatArena<M>,
        precomputed: &Map<IncompatId, Arc<DerivationTree<M>>>,
    ) -> DerivationTree<M> {
        match store[self_id].kind.clone() {
            Kind::DerivedFrom(id1, id2) => {
                let derived = Derived {
                    terms: store[self_id].package_terms.as_map(),
                    shared_id: shared_ids.get(&self_id).map(|id| id.0 as usize),
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
            Kind::NotRoot(package_id, version_index) => {
                DerivationTree::External(External::NotRoot(package_id, version_index))
            }
            Kind::NoVersions(package_id, set) => {
                DerivationTree::External(External::NoVersions(package_id, set))
            }
            Kind::FromDependencyOf(package_id, set, dep_package_id, dep_set) => {
                DerivationTree::External(External::FromDependencyOf(
                    package_id,
                    set,
                    dep_package_id,
                    dep_set,
                ))
            }
            Kind::Custom(package_id, set, metadata) => {
                DerivationTree::External(External::Custom(package_id, set, metadata.clone()))
            }
        }
    }

    /// CF definition of Relation enum.
    pub(crate) fn relation(&self, terms: impl Fn(PackageId) -> Option<Term>) -> Relation {
        let mut relation = Relation::Satisfied;
        for (&package_id, &incompat_term) in self.package_terms.iter() {
            match terms(package_id).map(|term| incompat_term.relation_with(term)) {
                Some(term::Relation::Satisfied) => {}
                Some(term::Relation::Contradicted) => {
                    return Relation::Contradicted(package_id);
                }
                None | Some(term::Relation::Inconclusive) => {
                    // If a package is not present, the intersection is the same as [Term::any].
                    // According to the rules of satisfactions, the relation would be inconclusive.
                    // It could also be satisfied if the incompatibility term was also [Term::any],
                    // but we systematically remove those from incompatibilities
                    // so we're safe on that front.
                    if relation == Relation::Satisfied {
                        relation = Relation::AlmostSatisfied(package_id);
                    } else {
                        return Relation::Inconclusive;
                    }
                }
            }
        }
        relation
    }

    pub(crate) fn display<'a, DP: DependencyProvider>(
        &'a self,
        package_store: &'a PackageArena<DP::P>,
        dependency_provider: &'a DP,
    ) -> IncompatibilityDisplay<'a, DP, M> {
        IncompatibilityDisplay {
            incompatibility: self,
            package_store,
            dependency_provider,
        }
    }
}

pub(crate) struct IncompatibilityDisplay<
    'a,
    DP: DependencyProvider,
    M: Eq + Clone + Debug + Display,
> {
    incompatibility: &'a Incompatibility<M>,
    package_store: &'a PackageArena<DP::P>,
    dependency_provider: &'a DP,
}

impl<DP: DependencyProvider, M: Eq + Clone + Debug + Display> Display
    for IncompatibilityDisplay<'_, DP, M>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            ReportFormatter::<DP>::format_terms(
                &DefaultStringReportFormatter,
                &self.incompatibility.package_terms.as_map(),
                self.package_store,
                self.dependency_provider
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
            let mut store = IncompatArena::new();
            let i1 = store.push(Incompatibility {
                package_terms: SmallMap::Two([(PackageId(1), t1), (PackageId(2), t2.negate())]),
                kind: Kind::<String>::FromDependencyOf(PackageId(1), VersionSet::full(), PackageId(2), VersionSet::full()),
                contradiction_info: ContradicationInfo::not_contradicted(),
            });

            let i2 = store.push(Incompatibility {
                package_terms: SmallMap::Two([(PackageId(2), t2), (PackageId(3), t3)]),
                kind: Kind::<String>::FromDependencyOf(PackageId(2), VersionSet::full(), PackageId(3), VersionSet::full()),
                contradiction_info: ContradicationInfo::not_contradicted(),
            });

            let mut i3 = Map::default();
            i3.insert(PackageId(1), t1);
            i3.insert(PackageId(3), t3);

            let i_resolution = Incompatibility::prior_cause(i1, i2, PackageId(2), &store);
            assert_eq!(i_resolution.package_terms.as_map(), i3);
        }

    }
}
