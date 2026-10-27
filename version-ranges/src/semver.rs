use semver::{Version, VersionReq};

use crate::Ranges;

/// The first release after `major.*` or `major.minor.*`, if one exists.
fn next_prefix(major: u64, minor: Option<u64>) -> Option<Version> {
    if let Some(next_minor) = minor.and_then(|m| m.checked_add(1)) {
        Some(Version::new(major, next_minor, 0))
    } else {
        Some(Version::new(major.checked_add(1)?, 0, 0))
    }
}

/// Follows the specifications in [`semver::Op`].
fn ranges_exact(major: u64, minor: Option<u64>, patch: Option<u64>) -> Ranges<Version> {
    match (minor, patch) {
        (minor, None) => {
            ranges_greater_eq(major, minor, None).intersection(&ranges_less_eq(major, minor, None))
        }
        (None, Some(_)) => panic!("invalid version requirement"),
        (Some(minor), Some(patch)) => Ranges::singleton(Version::new(major, minor, patch)),
    }
}

/// Follows the specifications in [`semver::Op`].
fn ranges_greater(major: u64, minor: Option<u64>, patch: Option<u64>) -> Ranges<Version> {
    match (minor, patch) {
        (minor, None) => next_prefix(major, minor).map_or_else(Ranges::empty, Ranges::higher_than),
        (None, Some(_)) => panic!("invalid version requirement"),
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
        (minor, None) => {
            next_prefix(major, minor).map_or_else(Ranges::full, Ranges::strictly_lower_than)
        }
        (None, Some(_)) => panic!("invalid version requirement"),
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
            .intersection(&ranges_less_eq(major, Some(minor), None)),
    }
}

/// Follows the specifications in [`semver::Op`].
fn ranges_caret(major: u64, minor: Option<u64>, patch: Option<u64>) -> Ranges<Version> {
    match (major, minor, patch) {
        (major, Some(minor), Some(patch)) if major > 0 => {
            ranges_greater_eq(major, Some(minor), Some(patch))
                .intersection(&ranges_less_eq(major, None, None))
        }

        (0, Some(minor), Some(patch)) if minor > 0 => ranges_greater_eq(
            0,
            Some(minor),
            Some(patch),
        )
        .intersection(&ranges_less_eq(0, Some(minor), None)),

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

#[test]
fn requirements_at_numeric_limits() {
    let max = u64::MAX;
    for requirement in [
        format!("={max}"),
        format!(">{max}"),
        format!("<={max}"),
        format!("1.{max}.*"),
        format!(">1.{max}"),
        format!(">{max}.{max}"),
        format!("~1.{max}.1"),
        format!("^0.{max}.1"),
        format!("^{max}.1.0"),
    ] {
        let req = VersionReq::parse(&requirement).unwrap();
        let ranges = Ranges::from_req(req.clone());
        for (major, minor, patch) in [
            (0, max, 1),
            (1, max, 0),
            (1, max, 1),
            (2, 0, 0),
            (max - 1, max, max),
            (max, 0, 0),
            (max, 1, 0),
            (max, max, max),
        ] {
            let version = Version::new(major, minor, patch);
            assert_eq!(
                ranges.contains(&version),
                req.matches(&version),
                "{requirement} with {version}"
            );
        }
    }
}
