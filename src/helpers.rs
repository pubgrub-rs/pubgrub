//! Helpers structs.

use std::fmt::{self, Display};
use std::iter::repeat_n;
use std::num::NonZeroU64;
use std::rc::Rc;

use crate::{VersionIndex, VersionSet};

/// Package allowing more than 63 versions.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Pkg<P> {
    pkg: P,
    quotient: u64,
    count: u64,
}

impl<P> Pkg<P> {
    /// Get the inner package.
    pub fn pkg(&self) -> &P {
        &self.pkg
    }
}

impl<P: Display> Display for Pkg<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (q={})", self.pkg, self.quotient)
    }
}

/// Virtual package ensuring package unicity.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct VirtualPkg<P> {
    pkg: P,
    quotient: u64,
    count: u64,
}

impl<P> VirtualPkg<P> {
    /// Get the inner package.
    pub fn pkg(&self) -> &P {
        &self.pkg
    }
}

impl<P: Display> Display for VirtualPkg<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VirtualPkg({}, q={}, c={})",
            self.pkg, self.quotient, self.count
        )
    }
}

/// Virtual package dependency allowing more than 63 versions.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct VirtualDep<P> {
    pkg: P,
    version_indices: Rc<[VersionSet]>,
    offset: u64,
    quotient: u64,
}

impl<P> VirtualDep<P> {
    /// Get the inner package.
    pub fn pkg(&self) -> &P {
        &self.pkg
    }
}

impl<P: Display> Display for VirtualDep<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let vc = self
            .version_indices
            .iter()
            .map(|vs| vs.count())
            .sum::<usize>();

        write!(
            f,
            "VirtualDep({}, vc={vc}, o={}, q={})",
            self.pkg, self.offset, self.quotient
        )
    }
}

/// Package wrapper used to allow more than 63 versions per package.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum PackageVersionWrapper<P: Clone + Display> {
    /// Package allowing more than 63 versions
    Pkg(Pkg<P>),
    /// Virtual package ensuring package unicity
    VirtualPkg(VirtualPkg<P>),
    /// Virtual package dependency allowing more than 63 versions
    VirtualDep(VirtualDep<P>),
}

impl<P: Clone + Display> Display for PackageVersionWrapper<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pkg(p) => p.fmt(f),
            Self::VirtualPkg(vp) => vp.fmt(f),
            Self::VirtualDep(vd) => vd.fmt(f),
        }
    }
}

impl<P: Clone + Display> PackageVersionWrapper<P> {
    /// Create a new package.
    pub fn new_pkg(
        pkg: P,
        true_version_index: u64,
        version_count: NonZeroU64,
    ) -> (Self, VersionIndex) {
        (
            Self::Pkg(Pkg {
                pkg,
                quotient: true_version_index / VersionIndex::MAX,
                count: (version_count.get() - 1) / VersionIndex::MAX,
            }),
            VersionIndex::new((true_version_index % VersionIndex::MAX) as u8).unwrap(),
        )
    }

    /// Create a new package dependency with no versions.
    pub fn new_empty_dep(pkg: P) -> (Self, VersionSet) {
        (
            Self::Pkg(Pkg {
                pkg,
                quotient: 0,
                count: 0,
            }),
            VersionSet::empty(),
        )
    }

    /// Create a new package dependency at the specified version.
    pub fn new_singleton_dep(
        pkg: P,
        true_version_index: u64,
        version_count: u64,
    ) -> (Self, VersionSet) {
        match NonZeroU64::new(version_count) {
            Some(version_count) => {
                assert!(true_version_index < version_count.get());
                let (this, v) = Self::new_pkg(pkg, true_version_index, version_count);
                (this, VersionSet::singleton(v))
            }
            None => Self::new_empty_dep(pkg),
        }
    }

    /// Create a new package dependency at the specified versions.
    pub fn new_dep(
        pkg: P,
        true_version_indices: impl IntoIterator<Item = u64>,
        version_count: u64,
    ) -> (Self, VersionSet) {
        let Some(nz_version_count) = NonZeroU64::new(version_count) else {
            return Self::new_empty_dep(pkg);
        };
        if version_count <= VersionIndex::MAX {
            let mut set = VersionSet::empty();
            for true_version_index in true_version_indices {
                assert!(true_version_index < version_count);
                let v = VersionIndex::new(true_version_index as u8).unwrap();
                set = set.union(VersionSet::singleton(v));
            }
            return (
                Self::Pkg(Pkg {
                    pkg,
                    quotient: 0,
                    count: (version_count - 1) / VersionIndex::MAX,
                }),
                set,
            );
        }

        let mut true_version_indices = true_version_indices.into_iter();

        let Some(first) = true_version_indices.next() else {
            return Self::new_empty_dep(pkg);
        };
        assert!(first < version_count);

        let Some(second) = true_version_indices.next() else {
            let (d, vs) = Self::new_pkg(pkg, first, nz_version_count);
            return (d, VersionSet::singleton(vs));
        };
        assert!(second < version_count);

        let mut version_indices = Rc::from_iter(repeat_n(
            VersionSet::empty(),
            (1 + (version_count - 1) / VersionIndex::MAX) as usize,
        ));
        let versions_slice = Rc::make_mut(&mut version_indices);

        for true_version_index in [first, second].into_iter().chain(true_version_indices) {
            assert!(true_version_index < version_count);
            let index = (true_version_index / VersionIndex::MAX) as usize;
            let v = VersionIndex::new((true_version_index % VersionIndex::MAX) as u8).unwrap();
            let set = versions_slice.get_mut(index).unwrap();
            *set = set.union(VersionSet::singleton(v));
        }

        let offset = 0;
        let quotient = VersionIndex::MAX.pow(version_count.ilog(VersionIndex::MAX) - 1);
        let version_set = Self::dep_version_set(&version_indices, offset, quotient);

        let this = Self::VirtualDep(VirtualDep {
            pkg,
            version_indices,
            offset,
            quotient,
        });

        (this, version_set)
    }

    /// Clone and replace the package of this wrapper.
    pub fn replace_pkg<T: Clone + Display>(&self, new_pkg: T) -> PackageVersionWrapper<T> {
        match *self {
            Self::Pkg(Pkg {
                pkg: _,
                quotient,
                count,
            }) => PackageVersionWrapper::Pkg(Pkg {
                pkg: new_pkg,
                quotient,
                count,
            }),
            Self::VirtualPkg(VirtualPkg {
                pkg: _,
                quotient,
                count,
            }) => PackageVersionWrapper::VirtualPkg(VirtualPkg {
                pkg: new_pkg,
                quotient,
                count,
            }),
            Self::VirtualDep(VirtualDep {
                pkg: _,
                ref version_indices,
                offset,
                quotient,
            }) => PackageVersionWrapper::VirtualDep(VirtualDep {
                pkg: new_pkg,
                version_indices: version_indices.clone(),
                offset,
                quotient,
            }),
        }
    }

    /// Get the inner package if existing.
    pub fn inner_pkg(&self) -> Option<&P> {
        match self {
            Self::Pkg(Pkg { pkg, .. }) => Some(pkg),
            _ => None,
        }
    }

    /// Get the inner package if existing.
    pub fn inner(&self, version_index: VersionIndex) -> Option<(&P, u64)> {
        match self {
            Self::Pkg(Pkg { pkg, quotient, .. }) => Some((
                pkg,
                quotient * VersionIndex::MAX + version_index.get() as u64,
            )),
            _ => None,
        }
    }

    /// Get the inner package if existing.
    pub fn into_inner(self, version_index: VersionIndex) -> Option<(P, u64)> {
        match self {
            Self::Pkg(Pkg { pkg, quotient, .. }) => Some((
                pkg,
                quotient * VersionIndex::MAX + version_index.get() as u64,
            )),
            _ => None,
        }
    }

    /// Get the wrapper virtual dependency if existing.
    pub fn dependency(&self, version_index: VersionIndex) -> Option<(Self, VersionSet)> {
        match *self {
            Self::Pkg(Pkg {
                ref pkg,
                quotient,
                count,
            })
            | Self::VirtualPkg(VirtualPkg {
                ref pkg,
                quotient,
                count,
            }) => {
                if count == 0 {
                    None
                } else {
                    Some((
                        Self::VirtualPkg(VirtualPkg {
                            pkg: pkg.clone(),
                            quotient: quotient / VersionIndex::MAX,
                            count: count / VersionIndex::MAX,
                        }),
                        VersionSet::singleton(
                            VersionIndex::new((quotient % VersionIndex::MAX) as u8).unwrap(),
                        ),
                    ))
                }
            }
            Self::VirtualDep(VirtualDep {
                ref pkg,
                ref version_indices,
                offset,
                quotient,
            }) => {
                let offset = offset + version_index.get() as u64 * quotient;
                if quotient == 1 {
                    return Some((
                        Self::Pkg(Pkg {
                            pkg: pkg.clone(),
                            quotient: offset,
                            count: (version_indices.len() - 1) as u64,
                        }),
                        version_indices[offset as usize],
                    ));
                }
                let quotient = quotient / VersionIndex::MAX;
                let version_set = Self::dep_version_set(version_indices, offset, quotient);

                let this = Self::VirtualDep(VirtualDep {
                    pkg: pkg.clone(),
                    version_indices: version_indices.clone(),
                    offset,
                    quotient,
                });

                Some((this, version_set))
            }
        }
    }

    fn dep_version_set(sets: &[VersionSet], offset: u64, quotient: u64) -> VersionSet {
        sets[offset as usize..]
            .chunks(quotient as usize)
            .take(VersionIndex::MAX as usize)
            .enumerate()
            .filter(|&(_, sets)| sets.iter().any(|&vs| vs != VersionSet::empty()))
            .map(|(i, _)| VersionSet::singleton(VersionIndex::new(i as u8).unwrap()))
            .fold(VersionSet::empty(), |acc, vs| acc.union(vs))
    }
}
