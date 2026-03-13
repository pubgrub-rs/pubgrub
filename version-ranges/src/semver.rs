use semver::{Version, VersionReq};

use crate::Ranges;

/// Follows the specifications in [`semver::Op`].
fn ranges_exact(major: u64, minor: Option<u64>, patch: Option<u64>) -> Ranges<Version> {
    match (minor, patch) {
        (None, None) => ranges_greater_eq(major, Some(0), Some(0)).intersection(&ranges_less(
            major + 1,
            Some(0),
            Some(0),
        )),
        (None, Some(_)) => panic!("invalid version requirement"),
        (Some(minor), None) => ranges_greater_eq(major, Some(minor), Some(0))
            .intersection(&ranges_less(major, Some(minor + 1), Some(0))),
        (Some(minor), Some(patch)) => Ranges::singleton(Version::new(major, minor, patch)),
    }
}

/// Follows the specifications in [`semver::Op`].
fn ranges_greater(major: u64, minor: Option<u64>, patch: Option<u64>) -> Ranges<Version> {
    match (minor, patch) {
        (None, None) => ranges_greater_eq(major + 1, Some(0), Some(0)),
        (None, Some(_)) => panic!("invalid version requirement"),
        (Some(minor), None) => ranges_greater_eq(major, Some(minor + 1), Some(0)),
        (Some(minor), Some(patch)) => {
            Ranges::strictly_higher_than(Version::new(major, minor, patch))
        }
    }
}

/// Follows the specifications in [`semver::Op`].
fn ranges_greater_eq(major: u64, minor: Option<u64>, patch: Option<u64>) -> Ranges<Version> {
    Ranges::higher_than(Version::new(major, minor.unwrap_or(0), patch.unwrap_or(0)))
}

/// Follows the specifications in [`semver::Op`].
fn ranges_less(major: u64, minor: Option<u64>, patch: Option<u64>) -> Ranges<Version> {
    Ranges::strictly_lower_than(Version::new(major, minor.unwrap_or(0), patch.unwrap_or(0)))
}

/// Follows the specifications in [`semver::Op`].
fn ranges_less_eq(major: u64, minor: Option<u64>, patch: Option<u64>) -> Ranges<Version> {
    match (minor, patch) {
        (None, None) => ranges_less(major + 1, Some(0), Some(0)),
        (None, Some(_)) => panic!("invalid version requirement"),
        (Some(minor), None) => ranges_less(major, Some(minor + 1), Some(0)),
        (Some(minor), Some(patch)) => Ranges::lower_than(Version::new(major, minor, patch)),
    }
}

/// Follows the specifications in [`semver::Op`].
fn ranges_tilde(major: u64, minor: Option<u64>, patch: Option<u64>) -> Ranges<Version> {
    match (minor, patch) {
        (None, None) => ranges_exact(major, None, None),
        (None, Some(_)) => panic!("invalid version requirement"),
        (Some(minor), None) => ranges_exact(major, Some(minor), None),
        (Some(minor), Some(patch)) => ranges_greater_eq(major, Some(minor), Some(patch))
            .intersection(&ranges_less(major, Some(minor + 1), Some(0))),
    }
}

/// Follows the specifications in [`semver::Op`].
fn ranges_caret(major: u64, minor: Option<u64>, patch: Option<u64>) -> Ranges<Version> {
    match (major, minor, patch) {
        (major, Some(minor), Some(patch)) if major > 0 => ranges_greater_eq(
            major,
            Some(minor),
            Some(patch),
        )
        .intersection(&ranges_less(major + 1, Some(0), Some(0))),

        (0, Some(minor), Some(patch)) if minor > 0 => ranges_greater_eq(
            0,
            Some(minor),
            Some(patch),
        )
        .intersection(&ranges_less(0, Some(minor + 1), Some(0))),

        (0, Some(0), Some(patch)) => ranges_exact(0, Some(0), Some(patch)),

        (major, Some(minor), None) if major > 0 || minor > 0 => {
            ranges_caret(major, Some(minor), Some(0))
        }

        (0, Some(0), None) => ranges_exact(0, Some(0), None),

        (major, None, None) => ranges_exact(major, None, None),

        (_, _, _) => unreachable!(),
    }
}

/// Follows the specifications in [`semver::Op`].
fn ranges_wildcard(major: u64, minor: Option<u64>, _patch: Option<u64>) -> Ranges<Version> {
    match minor {
        Some(minor) => ranges_exact(major, Some(minor), None),
        None => ranges_exact(major, None, None),
    }
}

impl Ranges<Version> {
    /// Convert from a semver version requirement.
    ///
    /// Note that this is not a lossless conversion.
    /// Semver additionally require the `VersionReq` to contain prereleases for versions with prereleases to be matched.
    /// Therefore the `Ranges` constraint after conversion will be **looser** than original as it does only interval arithmetics based on ordering of versions.
    ///
    /// The behavior is undefined unless all major, minor and patch numbers in `VersionReq` are less than `u64::MAX` due to integer overflow.
    pub fn from_req(req: VersionReq) -> Self {
        let mut ranges = Ranges::full();
        for cmp in req.comparators {
            let new = match cmp.op {
                semver::Op::Exact => ranges_exact(cmp.major, cmp.minor, cmp.patch),
                semver::Op::Greater => ranges_greater(cmp.major, cmp.minor, cmp.patch),
                semver::Op::GreaterEq => ranges_greater_eq(cmp.major, cmp.minor, cmp.patch),
                semver::Op::Less => ranges_less(cmp.major, cmp.minor, cmp.patch),
                semver::Op::LessEq => ranges_less_eq(cmp.major, cmp.minor, cmp.patch),
                semver::Op::Tilde => ranges_tilde(cmp.major, cmp.minor, cmp.patch),
                semver::Op::Caret => ranges_caret(cmp.major, cmp.minor, cmp.patch),
                semver::Op::Wildcard => ranges_wildcard(cmp.major, cmp.minor, cmp.patch),
                _ => unimplemented!(),
            };
            ranges = ranges.intersection(&new);
        }
        ranges
    }
}
