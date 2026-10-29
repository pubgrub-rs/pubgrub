use semver::{Version, VersionReq};

use crate::Ranges;

/// Computing a version requirement bound would overflow a version component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionReqOverflow;

impl std::fmt::Display for VersionReqOverflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("version requirement bound would overflow a u64")
    }
}

impl std::error::Error for VersionReqOverflow {}

/// The first release after `major.*` or `major.minor.*`.
fn next_prefix(major: u64, minor: Option<u64>) -> Result<Version, VersionReqOverflow> {
    Ok(match minor {
        Some(minor) => Version::new(major, minor.checked_add(1).ok_or(VersionReqOverflow)?, 0),
        None => Version::new(major.checked_add(1).ok_or(VersionReqOverflow)?, 0, 0),
    })
}

/// Follows the specifications in [`semver::Op`].
fn ranges_exact(
    major: u64,
    minor: Option<u64>,
    patch: Option<u64>,
) -> Result<Ranges<Version>, VersionReqOverflow> {
    Ok(match (minor, patch) {
        (minor, None) => Ranges::between(
            Version::new(major, minor.unwrap_or(0), 0),
            next_prefix(major, minor)?,
        ),
        (None, Some(_)) => panic!("invalid version requirement"),
        (Some(minor), Some(patch)) => Ranges::singleton(Version::new(major, minor, patch)),
    })
}

/// Follows the specifications in [`semver::Op`].
fn ranges_greater(
    major: u64,
    minor: Option<u64>,
    patch: Option<u64>,
) -> Result<Ranges<Version>, VersionReqOverflow> {
    Ok(match (minor, patch) {
        (minor, None) => Ranges::higher_than(next_prefix(major, minor)?),
        (None, Some(_)) => panic!("invalid version requirement"),
        (Some(minor), Some(patch)) => {
            Ranges::strictly_higher_than(Version::new(major, minor, patch))
        }
    })
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
fn ranges_less_eq(
    major: u64,
    minor: Option<u64>,
    patch: Option<u64>,
) -> Result<Ranges<Version>, VersionReqOverflow> {
    Ok(match (minor, patch) {
        (minor, None) => Ranges::strictly_lower_than(next_prefix(major, minor)?),
        (None, Some(_)) => panic!("invalid version requirement"),
        (Some(minor), Some(patch)) => Ranges::lower_than(Version::new(major, minor, patch)),
    })
}

/// Follows the specifications in [`semver::Op`].
fn ranges_tilde(
    major: u64,
    minor: Option<u64>,
    patch: Option<u64>,
) -> Result<Ranges<Version>, VersionReqOverflow> {
    match (minor, patch) {
        (_, None) => ranges_exact(major, minor, None),
        (None, Some(_)) => panic!("invalid version requirement"),
        (Some(minor), Some(patch)) => Ok(Ranges::between(
            Version::new(major, minor, patch),
            next_prefix(major, Some(minor))?,
        )),
    }
}

/// Follows the specifications in [`semver::Op`].
fn ranges_caret(
    major: u64,
    minor: Option<u64>,
    patch: Option<u64>,
) -> Result<Ranges<Version>, VersionReqOverflow> {
    match (major, minor, patch) {
        (major, Some(minor), Some(patch)) if major > 0 => Ok(Ranges::between(
            Version::new(major, minor, patch),
            next_prefix(major, None)?,
        )),
        (0, Some(minor), Some(patch)) if minor > 0 => Ok(Ranges::between(
            Version::new(0, minor, patch),
            next_prefix(0, Some(minor))?,
        )),
        (0, Some(0), Some(patch)) => ranges_exact(0, Some(0), Some(patch)),
        (major, Some(minor), None) if major > 0 || minor > 0 => {
            ranges_caret(major, Some(minor), Some(0))
        }
        (0, Some(0), None) => ranges_exact(0, Some(0), None),
        (major, None, None) => ranges_exact(major, None, None),
        (_, _, _) => unreachable!(),
    }
}

impl Ranges<Version> {
    /// Convert from a semver version requirement.
    ///
    /// Conversion can include prereleases that the original requirement excludes.
    /// `Ranges` compares versions by their ordering, while [`VersionReq`] also requires a
    /// prerelease comparator with the same major, minor, and patch.
    ///
    /// # Errors
    ///
    /// Returns [`VersionReqOverflow`] if computing a range bound requires incrementing a major
    /// or minor component that is already `u64::MAX`.
    pub fn from_req(req: VersionReq) -> Result<Self, VersionReqOverflow> {
        let mut ranges = Ranges::full();
        for cmp in req.comparators {
            let new = match cmp.op {
                semver::Op::Exact => ranges_exact(cmp.major, cmp.minor, cmp.patch)?,
                semver::Op::Greater => ranges_greater(cmp.major, cmp.minor, cmp.patch)?,
                semver::Op::GreaterEq => ranges_greater_eq(cmp.major, cmp.minor, cmp.patch),
                semver::Op::Less => ranges_less(cmp.major, cmp.minor, cmp.patch),
                semver::Op::LessEq => ranges_less_eq(cmp.major, cmp.minor, cmp.patch)?,
                semver::Op::Tilde => ranges_tilde(cmp.major, cmp.minor, cmp.patch)?,
                semver::Op::Caret => ranges_caret(cmp.major, cmp.minor, cmp.patch)?,
                semver::Op::Wildcard => ranges_exact(cmp.major, cmp.minor, None)?,
                _ => unimplemented!(),
            };
            ranges = ranges.intersection(&new);
        }
        Ok(ranges)
    }
}

#[test]
fn requirements_with_overflowing_bounds_return_errors() {
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
        assert_eq!(
            Ranges::from_req(req),
            Err(VersionReqOverflow),
            "{requirement}"
        );
    }
}

#[test]
fn requirements_with_representable_bounds() {
    let max = u64::MAX;
    for requirement in [
        format!("={max}.{max}.{max}"),
        format!(">={max}"),
        format!("<{max}"),
        format!("={}", max - 1),
        format!("1.{}.*", max - 1),
    ] {
        let req = VersionReq::parse(&requirement).unwrap();
        let ranges = Ranges::from_req(req.clone()).unwrap();
        for version in [
            Version::new(1, max - 1, 0),
            Version::new(1, max, 0),
            Version::new(max - 1, max, max),
            Version::new(max, 0, 0),
            Version::new(max, max, max),
        ] {
            assert_eq!(
                ranges.contains(&version),
                req.matches(&version),
                "{requirement} with {version}"
            );
        }
    }
}
