#[cfg(feature = "serde")]
use std::time::Instant;

#[cfg(feature = "serde")]
use pubgrub::{
    solver::{resolve, OfflineDependencyProvider},
    version::NumberVersion,
};

#[cfg(feature = "serde")]
use dhat::{Dhat, DhatAlloc};

#[cfg(feature = "serde")]
#[global_allocator]
static ALLOCATOR: DhatAlloc = DhatAlloc;

#[cfg(not(feature = "serde"))]
fn main() {}

#[cfg(feature = "serde")]
fn main() {
    let mut cases = vec![];

    // Do all the prep work befor we start the profiling
    for case in std::fs::read_dir("test-examples").unwrap() {
        let case = case.unwrap().path();
        let name = case.file_name().unwrap().to_string_lossy().to_string();
        let data = std::fs::read_to_string(&case).unwrap();
        if name.ends_with("u16_NumberVersion.ron") {
            let dependency_provider: OfflineDependencyProvider<u16, NumberVersion> =
                ron::de::from_str(&data).unwrap();

            cases.push(Box::new(move || {
                eprintln!("{}", name);
                for p in dependency_provider.packages() {
                    for n in dependency_provider.versions(p).unwrap() {
                        let _ = resolve(&dependency_provider, p.clone(), n.clone());
                    }
                }
            }) as Box<dyn Fn() -> ()>);
        } else if name.ends_with("str_SemanticVersion.ron") {
            let data: &str = Box::leak(Box::new(data));
            let dependency_provider: OfflineDependencyProvider<
                &str,
                pubgrub::version::SemanticVersion,
            > = ron::de::from_str(&data).unwrap();
            cases.push(Box::new(move || {
                eprintln!("{}", name);
                for p in dependency_provider.packages() {
                    for n in dependency_provider.versions(p).unwrap() {
                        let _ = resolve(&dependency_provider, p.clone(), n.clone());
                    }
                }
            }) as Box<dyn Fn() -> ()>);
        }
    }

    // Now start the profiler and run the examples.
    let _dhat = Dhat::start_heap_profiling();
    for case in cases {
        let now = Instant::now();
        case();
        println!("{}", now.elapsed().as_secs_f32());
    }
}
