//! Format-version compatibility.
//!
//! The SDK declares a **contiguous supported range** of format versions and
//! classifies what an asset declares against it. Below the floor and above the
//! range both refuse, with distinct identifiers; in range but below the maximum
//! loads fully and adds an `info`.
//!
//! The floor does not rise during the beta. It may rise only in a release
//! *after* migration tooling ships, and never in the same release that
//! introduces the version it excludes — otherwise a user on an excluded version
//! is told to pin an older build, which is to say *not to upgrade*, negating
//! both halves of the promise this range exists to keep.
//!
//! [`classify`] takes the range as an argument rather than reading the constant
//! so that every classification is reachable from a test. The contract range now
//! holds two versions, so `Supported` — in range, below the maximum — is
//! reachable from a real version-1 vault for the first time; the installation
//! range still holds one, and injecting a range like `2..=4` is what reaches
//! that asset's `Supported` branch without fabricating an impossible file.

use core::ops::RangeInclusive;

/// The contract versions this SDK reads.
///
/// The ceiling rose to 2 in the change that landed the per-version key sets and
/// default tables the SDK now carries for each of them; widening it without
/// them is the regression the vault-contract record's first amendment named.
/// The floor stays at 1, so a version-1 vault keeps loading.
pub const SUPPORTED_CONTRACT_VERSIONS: RangeInclusive<u32> = 1..=2;

/// The installation-record versions this SDK reads.
///
/// Deliberately still one version: nothing in the contract's version 2 touches
/// the installation record's schema, and bumping it in sympathy would be
/// symmetry for its own sake.
pub const SUPPORTED_INSTALLATION_VERSIONS: RangeInclusive<u32> = 1..=1;

/// Where a declared version sits relative to a supported range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VersionClass {
    /// Below the floor: refuse, and name migration and the pinning recourse.
    BelowFloor,
    /// In range but below the maximum: load fully, and say a newer format
    /// exists.
    Supported,
    /// The maximum: load fully, say nothing.
    Current,
    /// Above the range: refuse. With unknown keys fatal, an older tool cannot
    /// best-effort a newer asset, so refusal is the only honest answer.
    TooNew,
}

impl VersionClass {
    /// The lowercase wire spelling, used by every structured format.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BelowFloor => "below-supported-floor",
            Self::Supported => "supported",
            Self::Current => "current",
            Self::TooNew => "too-new",
        }
    }

    /// Whether an asset at this classification is read any further.
    pub fn is_usable(self) -> bool {
        matches!(self, Self::Supported | Self::Current)
    }
}

/// Classifies `found` against `supported`.
pub fn classify(found: u32, supported: RangeInclusive<u32>) -> VersionClass {
    if found < *supported.start() {
        VersionClass::BelowFloor
    } else if found > *supported.end() {
        VersionClass::TooNew
    } else if found == *supported.end() {
        VersionClass::Current
    } else {
        VersionClass::Supported
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The widened contract range is what makes `Supported` — and with it
    /// `compat.newer-format-available` — reachable from a real vault for the
    /// first time; the record's range still holds one version.
    #[test]
    fn the_contract_range_holds_two_versions_and_the_record_range_holds_one() {
        assert_eq!(SUPPORTED_CONTRACT_VERSIONS, 1..=2);
        assert_eq!(SUPPORTED_INSTALLATION_VERSIONS, 1..=1);
        let real = classify(1, SUPPORTED_CONTRACT_VERSIONS);
        assert_eq!(real, VersionClass::Supported);
    }

    #[test]
    fn a_single_version_range_reaches_three_of_the_four_classes() {
        assert_eq!(classify(0, 1..=1), VersionClass::BelowFloor);
        assert_eq!(classify(1, 1..=1), VersionClass::Current);
        assert_eq!(classify(2, 1..=1), VersionClass::TooNew);
        assert_eq!(
            classify(0, SUPPORTED_CONTRACT_VERSIONS),
            VersionClass::BelowFloor
        );
        assert_eq!(
            classify(1, SUPPORTED_INSTALLATION_VERSIONS),
            VersionClass::Current
        );
    }

    #[test]
    fn a_multi_version_range_reaches_every_class() {
        assert_eq!(classify(1, 2..=4), VersionClass::BelowFloor);
        assert_eq!(classify(2, 2..=4), VersionClass::Supported);
        assert_eq!(classify(3, 2..=4), VersionClass::Supported);
        assert_eq!(classify(4, 2..=4), VersionClass::Current);
        assert_eq!(classify(5, 2..=4), VersionClass::TooNew);
    }

    #[test]
    fn the_floor_and_the_maximum_are_both_inclusive() {
        assert_eq!(classify(0, 0..=0), VersionClass::Current);
        assert_eq!(classify(u32::MAX, 0..=u32::MAX), VersionClass::Current);
        assert_eq!(
            classify(u32::MAX - 1, 0..=u32::MAX),
            VersionClass::Supported
        );
    }

    #[test]
    fn classifications_render_and_say_whether_the_asset_is_read_further() {
        assert_eq!(VersionClass::BelowFloor.as_str(), "below-supported-floor");
        assert_eq!(VersionClass::Supported.as_str(), "supported");
        assert_eq!(VersionClass::Current.as_str(), "current");
        assert_eq!(VersionClass::TooNew.as_str(), "too-new");
        assert!(!VersionClass::BelowFloor.is_usable());
        assert!(VersionClass::Supported.is_usable());
        assert!(VersionClass::Current.is_usable());
        assert!(!VersionClass::TooNew.is_usable());
    }

    #[test]
    fn classifications_clone_and_format() {
        let classes = vec![VersionClass::Current, VersionClass::TooNew];
        assert_eq!(classes.clone(), classes);
        assert!(format!("{:?}", classes[0]).contains("Current"));
        assert!(format!("{:?}", VersionClass::BelowFloor).contains("BelowFloor"));
        assert!(format!("{:?}", VersionClass::Supported).contains("Supported"));
        assert!(format!("{:?}", VersionClass::TooNew).contains("TooNew"));
    }
}
