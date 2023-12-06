use std::time::Instant;

use pubgrub::{
    range::Range,
    solver::{resolve, OfflineDependencyProvider},
    version::NumberVersion,
};

fn main() {
    let mut dependency_provider = OfflineDependencyProvider::<&str, Range<NumberVersion>>::new();

    // root depends on foo...
    dependency_provider.add_dependencies("root", 1, vec![("foo", Range::full())]);

    for i in 1..500 {
        // foo depends on bar...
        dependency_provider.add_dependencies("foo", i, vec![("bad", Range::full())]);
    }

    let start = Instant::now();
    _ = resolve(&dependency_provider, "root", 1);
    let time = start.elapsed().as_secs_f32();
    let len = dependency_provider
        .versions(&"foo")
        .into_iter()
        .flatten()
        .count();
    println!("{len}, {time}");
}
