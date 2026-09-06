// SPDX-License-Identifier: MPL-2.0

mod common;

use std::io::Write as _;

use pubgrub::{
    DefaultStringReporter, DerivationTree, DerivationTreeNode, Derived, External, Map,
    PubGrubError, Ranges, Reporter,
};

#[test]
fn constrained_cousins_can_be_reported_and_collapsed() {
    let provider = common::constrained_cousins(100, 50);
    let Err(PubGrubError::NoSolution(mut tree)) = pubgrub::resolve(&provider, 0, 0u32) else {
        panic!("the constrained cousins must be unsatisfiable");
    };
    // Exercise debug formatting without retaining its roughly 250 MB of output.
    write!(std::io::sink(), "{tree:?}").unwrap();
    assert!(!DefaultStringReporter::report(&tree).is_empty());
    assert!(tree.packages().contains(&0));
    tree.collapse_no_versions();
    assert!(!DefaultStringReporter::report(&tree).is_empty());
}

#[test]
fn rebuild_tree_with_public_api() {
    let provider = common::constrained_cousins(3, 4);
    let Err(PubGrubError::NoSolution(tree)) = pubgrub::resolve(&provider, 0, 0u32) else {
        panic!("the constrained cousins must be unsatisfiable");
    };
    let root = tree.root_id();
    let report = DefaultStringReporter::report(&tree);
    let mut nodes = tree.into_nodes();
    let (first_id, DerivationTreeNode::External(first)) = nodes.next().unwrap() else {
        panic!("the first node must be external");
    };
    let mut rebuilt = DerivationTree::new(first);
    let mut ids = Map::from_iter([(first_id, rebuilt.root_id())]);
    for (old_id, mut node) in nodes {
        if let DerivationTreeNode::Derived(derived) = &mut node {
            derived.cause1 = ids[&derived.cause1];
            derived.cause2 = ids[&derived.cause2];
        }
        ids.insert(old_id, rebuilt.push(node));
    }
    rebuilt.set_root(ids[&root]);
    assert_eq!(DefaultStringReporter::report(&rebuilt), report);
}

#[test]
fn construct_tree_with_public_api() {
    let mut tree = DerivationTree::<&str, Ranges<u32>, String>::new(External::NoVersions(
        "bar",
        Ranges::full(),
    ));
    let missing = tree.root_id();
    let dependency = tree.push(DerivationTreeNode::External(External::FromDependencyOf(
        "foo",
        Ranges::full(),
        "bar",
        Ranges::full(),
    )));
    let root = tree.push(DerivationTreeNode::Derived(Derived::new(
        Map::default(),
        None,
        dependency,
        missing,
    )));
    tree.set_root(root);
    tree.collapse_no_versions();
    assert_eq!(DefaultStringReporter::report(&tree), "foo depends on bar");
}

#[test]
fn editing_a_clone_leaves_the_original_unchanged() {
    let tree = DerivationTree::<&str, Ranges<u32>, String>::new(External::NoVersions(
        "bar",
        Ranges::full(),
    ));
    let report = DefaultStringReporter::report(&tree);
    let mut cloned = tree.clone();
    let dependency = cloned.push(DerivationTreeNode::External(External::FromDependencyOf(
        "foo",
        Ranges::full(),
        "bar",
        Ranges::full(),
    )));
    let root = cloned.push(DerivationTreeNode::Derived(Derived::new(
        Map::default(),
        None,
        dependency,
        cloned.root_id(),
    )));
    cloned.set_root(root);
    let mut collapsed = cloned.clone();
    let uncollapsed = DefaultStringReporter::report(&cloned);
    collapsed.collapse_no_versions();
    assert_eq!(DefaultStringReporter::report(&tree), report);
    assert_eq!(DefaultStringReporter::report(&cloned), uncollapsed);
    assert_eq!(
        DefaultStringReporter::report(&collapsed),
        "foo depends on bar"
    );
}
