// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use pubgrub::range::Range;
use pubgrub::solver::{resolve, OfflineDependencyProvider};
use pubgrub::version::NumberVersion;

#[test]
fn same_result_on_repeated_runs() {
    let mut dependency_provider = OfflineDependencyProvider::<_, NumberVersion>::new();

    dependency_provider.add_dependencies("c", 0, vec![]);
    dependency_provider.add_dependencies("c", 2, vec![]);
    dependency_provider.add_dependencies("b", 0, vec![]);
    dependency_provider.add_dependencies("b", 1, vec![("c", Range::between(0, 1))]);

    dependency_provider.add_dependencies("a", 0, vec![("b", Range::any()), ("c", Range::any())]);

    let name = "a";
    let ver = NumberVersion(0);
    let one = resolve(&dependency_provider, name, ver);
    for _ in 0..10 {
        match (&one, &resolve(&dependency_provider, name, ver)) {
            (Ok(l), Ok(r)) => assert_eq!(l, r),
            _ => panic!("not the same result"),
        }
    }
}
