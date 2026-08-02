//! The kernel's diagnostic identifiers, and the opaque identifier type.
//!
//! A kernel identifier is `<area>.<slug>`, lowercase kebab-case, drawn from
//! the exhaustive [`KernelDiagnostic`] enum. The enum is the single source of
//! the set: there is no registry document beside it to drift from it.
//!
//! **Identifiers are permanent public API.** Renaming one is a breaking change
//! to every consumer of every binding, and there is no deprecation mechanism.
//!
//! Consumer identifiers must begin `ext.` — `ext.<namespace>.<slug>` — which
//! [`DiagnosticId::external`] enforces. That leaves the kernel the whole
//! remaining namespace permanently, so a consumer's identifier can never
//! become indistinguishable from one the kernel later mints.

use core::fmt;

use super::Severity;

/// Every diagnostic the kernel can raise.
///
/// The set is exhaustive and flat. [`KernelDiagnostic::ALL`] enumerates it, and
/// [`KernelDiagnostic::id`] gives each variant its stable identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KernelDiagnostic {
    /// The upward walk reached the filesystem root without finding a vault sentinel.
    DiscoveryNoVaultFound,
    /// A directory holds `.dogtag/` but not `.dogtag/contract.toml`, which halts the walk
    /// there.
    DiscoveryIncompleteVaultRoot,
    /// An exactly specified path is not a vault root; no upward search is performed.
    DiscoveryNotAVaultRoot,
    /// Canonicalization or a directory probe failed.
    DiscoveryPathUnreadable,
    /// An ancestor of the resolved root also holds the sentinel.
    DiscoveryNestedVault,
    /// The canonical root differs from the path that was requested.
    DiscoveryRootResolvedThroughSymlink,
    /// The resolved root is not under the supplied home directory.
    DiscoveryRootOutsideHome,
    /// The root directory's mode grants write to the group or to others.
    DiscoveryRootGroupOrWorldWritable,
    /// The contract file could not be read.
    ContractUnreadable,
    /// The contract file is not valid UTF-8.
    ContractInvalidUtf8,
    /// The contract file begins with a byte order mark.
    ContractByteOrderMark,
    /// The contract file uses carriage-return line endings.
    ContractCarriageReturnLineEnding,
    /// The contract file is not well-formed TOML.
    ContractMalformedToml,
    /// The contract declares no `contract_version`.
    ContractVersionMissing,
    /// `contract_version` is not an integer, is negative, or is beyond a `u32`.
    ContractVersionInvalid,
    /// The contract declares a key the version it declares does not define.
    ContractUnknownKey,
    /// The contract omits a key the version it declares requires.
    ContractMissingKey,
    /// A contract value has a TOML type the declared version does not allow.
    ContractValueWrongType,
    /// A declaration's name is empty, or holds a `.`.
    ///
    /// Provenance is addressed by a dotted key path built by joining declaration
    /// names, so a type named `t.property.p` addresses the same key as type `t`'s
    /// property `p` — and the second silently replaces the first in the map whose
    /// whole job is saying where a value came from. An empty name addresses
    /// nothing at all.
    ContractDeclarationNameInvalid,
    /// Two types share a name.
    ContractDuplicateType,
    /// Two properties on one type share a name.
    ContractDuplicateProperty,
    /// Two relationships on one type share a predicate.
    ContractDuplicatePredicate,
    /// One property name declares different kinds on different types.
    ContractPropertyKindConflict,
    /// A type declares a capability the format does not define.
    ContractUnknownCapability,
    /// No type declares the catch-all capability.
    ContractMissingCatchAll,
    /// More than one type declares the catch-all capability.
    ContractMultipleCatchAll,
    /// A property declares a kind outside the closed lattice of eight.
    ContractUnknownPropertyKind,
    /// An `enum` property's `values` are missing, empty, non-string, or duplicated.
    ContractInvalidEnumValues,
    /// A `list` property's `of` is missing, names an unknown kind, or names `list`.
    ContractInvalidListOf,
    /// The contract declares no `[lifecycle]` table at all.
    ContractMissingLifecycle,
    /// `[lifecycle]` declares neither an axis with an ordinary state nor `none = true`.
    ContractLifecycleIncomplete,
    /// `[lifecycle]` declares `none` alongside `axis` or `ordinary`.
    ContractLifecycleNoneWithAxis,
    /// The lifecycle axis names no declared property.
    ContractLifecycleAxisUndeclared,
    /// The lifecycle axis names a property whose kind is not `enum`.
    ContractLifecycleAxisNotEnum,
    /// `[lifecycle.ordinary]` declares neither `value` nor `absent`, both, or a non-boolean
    /// `absent`.
    ContractLifecycleOrdinaryInvalid,
    /// `ordinary.value` is not a member of the axis property's declared values.
    ContractLifecycleOrdinaryValueUndeclared,
    /// `ordinary.value` is declared, but the axis property is not required on every type that
    /// declares it.
    ContractLifecycleOrdinaryValueOptional,
    /// `ordinary.absent` is declared, but the axis property is required on some type.
    ContractLifecycleOrdinaryAbsentRequired,
    /// A flag names no declared property.
    ContractFlagPropertyUndeclared,
    /// A flag names a property whose kind is not `boolean`.
    ContractFlagPropertyNotBoolean,
    /// Two flags name the same property.
    ContractDuplicateFlag,
    /// The contract declares no `[dialect]` table.
    ContractMissingDialect,
    /// `dialect.links` names a link dialect the format does not define.
    ContractUnknownLinkDialect,
    /// The contract declares no type at all.
    ContractNoTypes,
    /// The installation record could not be read.
    InstallationUnreadable,
    /// The installation record is not valid UTF-8.
    InstallationInvalidUtf8,
    /// The installation record begins with a byte order mark.
    InstallationByteOrderMark,
    /// The installation record uses carriage-return line endings.
    InstallationCarriageReturnLineEnding,
    /// The installation record is not well-formed TOML.
    InstallationMalformedToml,
    /// The record declares no `installation_version`.
    InstallationVersionMissing,
    /// `installation_version` is not an integer, is negative, or is beyond a `u32`.
    InstallationVersionInvalid,
    /// The record declares a key the version it declares does not define. This is what refuses
    /// a local record that tries to supply a contract-owned setting: the authority partition is
    /// structural rather than policed.
    InstallationUnknownKey,
    /// The record omits a key the version it declares requires.
    InstallationMissingKey,
    /// A record value has a TOML type the declared version does not allow.
    InstallationValueWrongType,
    /// Two registry entries share a name.
    InstallationDuplicateVaultName,
    /// A registry entry's name is not kebab-case, or holds a path separator.
    InstallationVaultNameInvalid,
    /// A registry entry's path is not absolute. `~` and `$VAR` are never expanded, so both are
    /// caught here.
    InstallationVaultPathNotAbsolute,
    /// No registry entry carries the requested name, or no record exists at all.
    InstallationUnknownVaultName,
    /// A registry entry's path is absent, or is not a vault root.
    InstallationVaultPathNotARoot,
    /// The contract declares a version below the supported floor.
    CompatContractBelowSupportedFloor,
    /// The contract declares a version above the supported range.
    CompatContractTooNew,
    /// The contract declares a supported version below the newest this SDK reads.
    CompatNewerFormatAvailable,
    /// The installation record declares a version below the supported floor.
    CompatInstallationBelowSupportedFloor,
    /// The installation record declares a version above the supported range.
    CompatInstallationTooNew,
    /// The record declares a supported version below the newest this SDK reads.
    CompatNewerInstallationFormatAvailable,
}

/// The single source of identifiers and severities.
///
/// Both [`KernelDiagnostic::id`] and [`KernelDiagnostic::severity`] are table
/// lookups over this slice rather than matches, so neither grows a branch per
/// variant.
const REGISTRY: &[(KernelDiagnostic, &str, Severity)] = &[
    (
        KernelDiagnostic::DiscoveryNoVaultFound,
        "discovery.no-vault-found",
        Severity::Error,
    ),
    (
        KernelDiagnostic::DiscoveryIncompleteVaultRoot,
        "discovery.incomplete-vault-root",
        Severity::Error,
    ),
    (
        KernelDiagnostic::DiscoveryNotAVaultRoot,
        "discovery.not-a-vault-root",
        Severity::Error,
    ),
    (
        KernelDiagnostic::DiscoveryPathUnreadable,
        "discovery.path-unreadable",
        Severity::Error,
    ),
    (
        KernelDiagnostic::DiscoveryNestedVault,
        "discovery.nested-vault",
        Severity::Warning,
    ),
    (
        KernelDiagnostic::DiscoveryRootResolvedThroughSymlink,
        "discovery.root-resolved-through-symlink",
        Severity::Info,
    ),
    (
        KernelDiagnostic::DiscoveryRootOutsideHome,
        "discovery.root-outside-home",
        Severity::Warning,
    ),
    (
        KernelDiagnostic::DiscoveryRootGroupOrWorldWritable,
        "discovery.root-group-or-world-writable",
        Severity::Warning,
    ),
    (
        KernelDiagnostic::ContractUnreadable,
        "contract.unreadable",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractInvalidUtf8,
        "contract.invalid-utf8",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractByteOrderMark,
        "contract.byte-order-mark",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractCarriageReturnLineEnding,
        "contract.carriage-return-line-ending",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractMalformedToml,
        "contract.malformed-toml",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractVersionMissing,
        "contract.version-missing",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractVersionInvalid,
        "contract.version-invalid",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractUnknownKey,
        "contract.unknown-key",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractMissingKey,
        "contract.missing-key",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractValueWrongType,
        "contract.value-wrong-type",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractDeclarationNameInvalid,
        "contract.declaration-name-invalid",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractDuplicateType,
        "contract.duplicate-type",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractDuplicateProperty,
        "contract.duplicate-property",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractDuplicatePredicate,
        "contract.duplicate-predicate",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractPropertyKindConflict,
        "contract.property-kind-conflict",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractUnknownCapability,
        "contract.unknown-capability",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractMissingCatchAll,
        "contract.missing-catch-all",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractMultipleCatchAll,
        "contract.multiple-catch-all",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractUnknownPropertyKind,
        "contract.unknown-property-kind",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractInvalidEnumValues,
        "contract.invalid-enum-values",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractInvalidListOf,
        "contract.invalid-list-of",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractMissingLifecycle,
        "contract.missing-lifecycle",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractLifecycleIncomplete,
        "contract.lifecycle-incomplete",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractLifecycleNoneWithAxis,
        "contract.lifecycle-none-with-axis",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractLifecycleAxisUndeclared,
        "contract.lifecycle-axis-undeclared",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractLifecycleAxisNotEnum,
        "contract.lifecycle-axis-not-enum",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractLifecycleOrdinaryInvalid,
        "contract.lifecycle-ordinary-invalid",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractLifecycleOrdinaryValueUndeclared,
        "contract.lifecycle-ordinary-value-undeclared",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractLifecycleOrdinaryValueOptional,
        "contract.lifecycle-ordinary-value-optional",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractLifecycleOrdinaryAbsentRequired,
        "contract.lifecycle-ordinary-absent-required",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractFlagPropertyUndeclared,
        "contract.flag-property-undeclared",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractFlagPropertyNotBoolean,
        "contract.flag-property-not-boolean",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractDuplicateFlag,
        "contract.duplicate-flag",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractMissingDialect,
        "contract.missing-dialect",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractUnknownLinkDialect,
        "contract.unknown-link-dialect",
        Severity::Error,
    ),
    (
        KernelDiagnostic::ContractNoTypes,
        "contract.no-types",
        Severity::Error,
    ),
    (
        KernelDiagnostic::InstallationUnreadable,
        "installation.unreadable",
        Severity::Error,
    ),
    (
        KernelDiagnostic::InstallationInvalidUtf8,
        "installation.invalid-utf8",
        Severity::Error,
    ),
    (
        KernelDiagnostic::InstallationByteOrderMark,
        "installation.byte-order-mark",
        Severity::Error,
    ),
    (
        KernelDiagnostic::InstallationCarriageReturnLineEnding,
        "installation.carriage-return-line-ending",
        Severity::Error,
    ),
    (
        KernelDiagnostic::InstallationMalformedToml,
        "installation.malformed-toml",
        Severity::Error,
    ),
    (
        KernelDiagnostic::InstallationVersionMissing,
        "installation.version-missing",
        Severity::Error,
    ),
    (
        KernelDiagnostic::InstallationVersionInvalid,
        "installation.version-invalid",
        Severity::Error,
    ),
    (
        KernelDiagnostic::InstallationUnknownKey,
        "installation.unknown-key",
        Severity::Error,
    ),
    (
        KernelDiagnostic::InstallationMissingKey,
        "installation.missing-key",
        Severity::Error,
    ),
    (
        KernelDiagnostic::InstallationValueWrongType,
        "installation.value-wrong-type",
        Severity::Error,
    ),
    (
        KernelDiagnostic::InstallationDuplicateVaultName,
        "installation.duplicate-vault-name",
        Severity::Error,
    ),
    (
        KernelDiagnostic::InstallationVaultNameInvalid,
        "installation.vault-name-invalid",
        Severity::Error,
    ),
    (
        KernelDiagnostic::InstallationVaultPathNotAbsolute,
        "installation.vault-path-not-absolute",
        Severity::Error,
    ),
    (
        KernelDiagnostic::InstallationUnknownVaultName,
        "installation.unknown-vault-name",
        Severity::Error,
    ),
    (
        KernelDiagnostic::InstallationVaultPathNotARoot,
        "installation.vault-path-not-a-root",
        Severity::Error,
    ),
    (
        KernelDiagnostic::CompatContractBelowSupportedFloor,
        "compat.contract-below-supported-floor",
        Severity::Error,
    ),
    (
        KernelDiagnostic::CompatContractTooNew,
        "compat.contract-too-new",
        Severity::Error,
    ),
    (
        KernelDiagnostic::CompatNewerFormatAvailable,
        "compat.newer-format-available",
        Severity::Info,
    ),
    (
        KernelDiagnostic::CompatInstallationBelowSupportedFloor,
        "compat.installation-below-supported-floor",
        Severity::Error,
    ),
    (
        KernelDiagnostic::CompatInstallationTooNew,
        "compat.installation-too-new",
        Severity::Error,
    ),
    (
        KernelDiagnostic::CompatNewerInstallationFormatAvailable,
        "compat.newer-installation-format-available",
        Severity::Info,
    ),
];

/// How many diagnostics the registry holds.
const COUNT: usize = REGISTRY.len();

/// The registry's variants, in registry order.
///
/// Derived from `REGISTRY` rather than written a second time, so the two can
/// never disagree about which variants exist.
const fn registry_variants() -> [KernelDiagnostic; COUNT] {
    let mut variants = [KernelDiagnostic::DiscoveryNoVaultFound; COUNT];
    let mut index = 0;
    while index < COUNT {
        variants[index] = REGISTRY[index].0;
        index += 1;
    }
    variants
}

static VARIANTS: [KernelDiagnostic; COUNT] = registry_variants();

/// The registry row for `kind`.
///
/// A lookup rather than a match: cyclomatic complexity 1, and the `expect` is
/// unreachable because this module's tests prove `REGISTRY` complete.
fn entry(kind: KernelDiagnostic) -> &'static (KernelDiagnostic, &'static str, Severity) {
    REGISTRY
        .iter()
        .find(|candidate| candidate.0 == kind)
        .expect("every KernelDiagnostic variant is registered")
}

impl KernelDiagnostic {
    /// Every variant, exactly once.
    pub const ALL: &'static [KernelDiagnostic] = &VARIANTS;

    /// This diagnostic's stable identifier, `<area>.<slug>`.
    pub fn id(self) -> &'static str {
        entry(self).1
    }

    /// The severity this diagnostic is always raised at.
    pub fn severity(self) -> Severity {
        entry(self).2
    }
}

impl fmt::Display for KernelDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// A diagnostic identifier: either one of the kernel's, or a consumer's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticId(DiagnosticIdRepr);

#[derive(Clone, Debug, PartialEq, Eq)]
enum DiagnosticIdRepr {
    Kernel(KernelDiagnostic),
    External(String),
}

impl DiagnosticId {
    /// The identifier of a kernel diagnostic.
    pub fn kernel(kind: KernelDiagnostic) -> Self {
        Self(DiagnosticIdRepr::Kernel(kind))
    }

    /// A consumer's identifier.
    ///
    /// Accepts `ext.<namespace>.<slug>`: at least three dot-separated segments,
    /// the first exactly `ext`, every segment lowercase kebab-case. Everything
    /// else is rejected, so a consumer cannot mint an identifier the kernel
    /// might later claim.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidExternalId`] when `id` is outside that namespace.
    pub fn external(id: &str) -> Result<Self, InvalidExternalId> {
        if is_external_id(id) {
            Ok(Self(DiagnosticIdRepr::External(id.to_owned())))
        } else {
            Err(InvalidExternalId { id: id.to_owned() })
        }
    }

    /// The identifier as it is written in output.
    pub fn as_str(&self) -> &str {
        match &self.0 {
            DiagnosticIdRepr::Kernel(kind) => kind.id(),
            DiagnosticIdRepr::External(id) => id,
        }
    }
}

impl fmt::Display for DiagnosticId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The rejection [`DiagnosticId::external`] returns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidExternalId {
    id: String,
}

impl InvalidExternalId {
    /// The identifier that was rejected.
    pub fn id(&self) -> &str {
        &self.id
    }
}

impl fmt::Display for InvalidExternalId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{}` is not a consumer diagnostic identifier: consumer identifiers are \
             `ext.<namespace>.<slug>`, lowercase kebab-case",
            self.id
        )
    }
}

impl core::error::Error for InvalidExternalId {}

/// Whether `id` is a well-formed consumer identifier.
fn is_external_id(id: &str) -> bool {
    let segments: Vec<&str> = id.split('.').collect();
    segments.len() >= 3 && segments[0] == "ext" && segments.iter().all(|s| is_kebab(s))
}

/// Whether `segment` is lowercase kebab-case: hyphen-separated words of ASCII
/// lowercase letters and digits, with no empty word.
fn is_kebab(segment: &str) -> bool {
    segment.split('-').all(is_kebab_word)
}

fn is_kebab_word(word: &str) -> bool {
    !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// The identifier each variant must carry, written independently of `REGISTRY`.
///
/// Each area's table states its identifiers as literals; [`expected_area`] is what
/// makes the set complete, and the tables answer `""` outside their own area.
#[cfg(test)]
fn expected_id(kind: KernelDiagnostic) -> &'static str {
    match expected_area(kind) {
        "discovery" => expected_discovery_id(kind),
        "contract" => expected_contract_id(kind),
        "installation" => expected_installation_id(kind),
        _ => expected_compat_id(kind),
    }
}

/// Every `discovery.*` variant, as one pattern.
///
/// The four group macros are macros rather than functions because it is the
/// *patterns* an exhaustiveness check reads. A function could only answer
/// whether a variant is in its area, and an answer is not a pattern — which is
/// exactly why a match guard over such a function compiles while checking
/// nothing. Each group is its own item, so no one of them grows toward a size
/// that would have to be split again.
#[cfg(test)]
macro_rules! discovery_variants {
    () => {
        KernelDiagnostic::DiscoveryNoVaultFound
            | KernelDiagnostic::DiscoveryIncompleteVaultRoot
            | KernelDiagnostic::DiscoveryNotAVaultRoot
            | KernelDiagnostic::DiscoveryPathUnreadable
            | KernelDiagnostic::DiscoveryNestedVault
            | KernelDiagnostic::DiscoveryRootResolvedThroughSymlink
            | KernelDiagnostic::DiscoveryRootOutsideHome
            | KernelDiagnostic::DiscoveryRootGroupOrWorldWritable
    };
}

/// Every `contract.*` variant, as one pattern.
#[cfg(test)]
macro_rules! contract_variants {
    () => {
        KernelDiagnostic::ContractUnreadable
            | KernelDiagnostic::ContractInvalidUtf8
            | KernelDiagnostic::ContractByteOrderMark
            | KernelDiagnostic::ContractCarriageReturnLineEnding
            | KernelDiagnostic::ContractMalformedToml
            | KernelDiagnostic::ContractVersionMissing
            | KernelDiagnostic::ContractVersionInvalid
            | KernelDiagnostic::ContractUnknownKey
            | KernelDiagnostic::ContractMissingKey
            | KernelDiagnostic::ContractValueWrongType
            | KernelDiagnostic::ContractDeclarationNameInvalid
            | KernelDiagnostic::ContractDuplicateType
            | KernelDiagnostic::ContractDuplicateProperty
            | KernelDiagnostic::ContractDuplicatePredicate
            | KernelDiagnostic::ContractPropertyKindConflict
            | KernelDiagnostic::ContractUnknownCapability
            | KernelDiagnostic::ContractMissingCatchAll
            | KernelDiagnostic::ContractMultipleCatchAll
            | KernelDiagnostic::ContractUnknownPropertyKind
            | KernelDiagnostic::ContractInvalidEnumValues
            | KernelDiagnostic::ContractInvalidListOf
            | KernelDiagnostic::ContractMissingLifecycle
            | KernelDiagnostic::ContractLifecycleIncomplete
            | KernelDiagnostic::ContractLifecycleNoneWithAxis
            | KernelDiagnostic::ContractLifecycleAxisUndeclared
            | KernelDiagnostic::ContractLifecycleAxisNotEnum
            | KernelDiagnostic::ContractLifecycleOrdinaryInvalid
            | KernelDiagnostic::ContractLifecycleOrdinaryValueUndeclared
            | KernelDiagnostic::ContractLifecycleOrdinaryValueOptional
            | KernelDiagnostic::ContractLifecycleOrdinaryAbsentRequired
            | KernelDiagnostic::ContractFlagPropertyUndeclared
            | KernelDiagnostic::ContractFlagPropertyNotBoolean
            | KernelDiagnostic::ContractDuplicateFlag
            | KernelDiagnostic::ContractMissingDialect
            | KernelDiagnostic::ContractUnknownLinkDialect
            | KernelDiagnostic::ContractNoTypes
    };
}

/// Every `installation.*` variant, as one pattern.
#[cfg(test)]
macro_rules! installation_variants {
    () => {
        KernelDiagnostic::InstallationUnreadable
            | KernelDiagnostic::InstallationInvalidUtf8
            | KernelDiagnostic::InstallationByteOrderMark
            | KernelDiagnostic::InstallationCarriageReturnLineEnding
            | KernelDiagnostic::InstallationMalformedToml
            | KernelDiagnostic::InstallationVersionMissing
            | KernelDiagnostic::InstallationVersionInvalid
            | KernelDiagnostic::InstallationUnknownKey
            | KernelDiagnostic::InstallationMissingKey
            | KernelDiagnostic::InstallationValueWrongType
            | KernelDiagnostic::InstallationDuplicateVaultName
            | KernelDiagnostic::InstallationVaultNameInvalid
            | KernelDiagnostic::InstallationVaultPathNotAbsolute
            | KernelDiagnostic::InstallationUnknownVaultName
            | KernelDiagnostic::InstallationVaultPathNotARoot
    };
}

/// Every `compat.*` variant, as one pattern.
#[cfg(test)]
macro_rules! compat_variants {
    () => {
        KernelDiagnostic::CompatContractBelowSupportedFloor
            | KernelDiagnostic::CompatContractTooNew
            | KernelDiagnostic::CompatNewerFormatAvailable
            | KernelDiagnostic::CompatInstallationBelowSupportedFloor
            | KernelDiagnostic::CompatInstallationTooNew
            | KernelDiagnostic::CompatNewerInstallationFormatAvailable
    };
}

/// The area each variant belongs to.
///
/// The four groups above name every variant exactly once, and this is an
/// **exhaustive match** over them, so adding a variant without placing it in an
/// area fails to compile — naming the group that must grow. That is what makes
/// keeping the registry complete a compiler obligation rather than a review one.
#[cfg(test)]
fn expected_area(kind: KernelDiagnostic) -> &'static str {
    match kind {
        discovery_variants!() => "discovery",
        contract_variants!() => "contract",
        installation_variants!() => "installation",
        compat_variants!() => "compat",
    }
}

/// The identifier each `discovery.*` variant carries, and `""` for any other area.
#[cfg(test)]
fn expected_discovery_id(kind: KernelDiagnostic) -> &'static str {
    use KernelDiagnostic::*;
    match kind {
        DiscoveryNoVaultFound => "discovery.no-vault-found",
        DiscoveryIncompleteVaultRoot => "discovery.incomplete-vault-root",
        DiscoveryNotAVaultRoot => "discovery.not-a-vault-root",
        DiscoveryPathUnreadable => "discovery.path-unreadable",
        DiscoveryNestedVault => "discovery.nested-vault",
        DiscoveryRootResolvedThroughSymlink => "discovery.root-resolved-through-symlink",
        DiscoveryRootOutsideHome => "discovery.root-outside-home",
        DiscoveryRootGroupOrWorldWritable => "discovery.root-group-or-world-writable",
        _ => "",
    }
}

/// The identifier each `contract.*` variant carries, and `""` for any other area.
#[cfg(test)]
fn expected_contract_id(kind: KernelDiagnostic) -> &'static str {
    use KernelDiagnostic::*;
    match kind {
        ContractUnreadable => "contract.unreadable",
        ContractInvalidUtf8 => "contract.invalid-utf8",
        ContractByteOrderMark => "contract.byte-order-mark",
        ContractCarriageReturnLineEnding => "contract.carriage-return-line-ending",
        ContractMalformedToml => "contract.malformed-toml",
        ContractVersionMissing => "contract.version-missing",
        ContractVersionInvalid => "contract.version-invalid",
        ContractUnknownKey => "contract.unknown-key",
        ContractMissingKey => "contract.missing-key",
        ContractValueWrongType => "contract.value-wrong-type",
        ContractDeclarationNameInvalid => "contract.declaration-name-invalid",
        ContractDuplicateType => "contract.duplicate-type",
        ContractDuplicateProperty => "contract.duplicate-property",
        ContractDuplicatePredicate => "contract.duplicate-predicate",
        ContractPropertyKindConflict => "contract.property-kind-conflict",
        ContractUnknownCapability => "contract.unknown-capability",
        ContractMissingCatchAll => "contract.missing-catch-all",
        ContractMultipleCatchAll => "contract.multiple-catch-all",
        ContractUnknownPropertyKind => "contract.unknown-property-kind",
        ContractInvalidEnumValues => "contract.invalid-enum-values",
        ContractInvalidListOf => "contract.invalid-list-of",
        ContractMissingLifecycle => "contract.missing-lifecycle",
        ContractLifecycleIncomplete => "contract.lifecycle-incomplete",
        ContractLifecycleNoneWithAxis => "contract.lifecycle-none-with-axis",
        ContractLifecycleAxisUndeclared => "contract.lifecycle-axis-undeclared",
        ContractLifecycleAxisNotEnum => "contract.lifecycle-axis-not-enum",
        ContractLifecycleOrdinaryInvalid => "contract.lifecycle-ordinary-invalid",
        ContractLifecycleOrdinaryValueUndeclared => "contract.lifecycle-ordinary-value-undeclared",
        ContractLifecycleOrdinaryValueOptional => "contract.lifecycle-ordinary-value-optional",
        ContractLifecycleOrdinaryAbsentRequired => "contract.lifecycle-ordinary-absent-required",
        ContractFlagPropertyUndeclared => "contract.flag-property-undeclared",
        ContractFlagPropertyNotBoolean => "contract.flag-property-not-boolean",
        ContractDuplicateFlag => "contract.duplicate-flag",
        ContractMissingDialect => "contract.missing-dialect",
        ContractUnknownLinkDialect => "contract.unknown-link-dialect",
        ContractNoTypes => "contract.no-types",
        _ => "",
    }
}

/// The identifier each `installation.*` variant carries, and `""` for any other area.
#[cfg(test)]
fn expected_installation_id(kind: KernelDiagnostic) -> &'static str {
    use KernelDiagnostic::*;
    match kind {
        InstallationUnreadable => "installation.unreadable",
        InstallationInvalidUtf8 => "installation.invalid-utf8",
        InstallationByteOrderMark => "installation.byte-order-mark",
        InstallationCarriageReturnLineEnding => "installation.carriage-return-line-ending",
        InstallationMalformedToml => "installation.malformed-toml",
        InstallationVersionMissing => "installation.version-missing",
        InstallationVersionInvalid => "installation.version-invalid",
        InstallationUnknownKey => "installation.unknown-key",
        InstallationMissingKey => "installation.missing-key",
        InstallationValueWrongType => "installation.value-wrong-type",
        InstallationDuplicateVaultName => "installation.duplicate-vault-name",
        InstallationVaultNameInvalid => "installation.vault-name-invalid",
        InstallationVaultPathNotAbsolute => "installation.vault-path-not-absolute",
        InstallationUnknownVaultName => "installation.unknown-vault-name",
        InstallationVaultPathNotARoot => "installation.vault-path-not-a-root",
        _ => "",
    }
}

/// The identifier each `compat.*` variant carries, and `""` for any other area.
#[cfg(test)]
fn expected_compat_id(kind: KernelDiagnostic) -> &'static str {
    use KernelDiagnostic::*;
    match kind {
        CompatContractBelowSupportedFloor => "compat.contract-below-supported-floor",
        CompatContractTooNew => "compat.contract-too-new",
        CompatNewerFormatAvailable => "compat.newer-format-available",
        CompatInstallationBelowSupportedFloor => "compat.installation-below-supported-floor",
        CompatInstallationTooNew => "compat.installation-too-new",
        CompatNewerInstallationFormatAvailable => "compat.newer-installation-format-available",
        _ => "",
    }
}

/// The severity each identifier must carry.
///
/// Stated as the exceptions it is: every kernel diagnostic is an error unless
/// it is one of the six named here.
#[cfg(test)]
fn expected_severity(id: &str) -> Severity {
    match id {
        "discovery.nested-vault"
        | "discovery.root-outside-home"
        | "discovery.root-group-or-world-writable" => Severity::Warning,
        "discovery.root-resolved-through-symlink"
        | "compat.newer-format-available"
        | "compat.newer-installation-format-available" => Severity::Info,
        _ => Severity::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// The areas the kernel claims at M2.
    const AREAS: &[&str] = &["discovery", "contract", "installation", "compat"];

    #[test]
    fn every_variant_carries_the_identifier_it_declares() {
        for kind in KernelDiagnostic::ALL {
            assert_eq!(kind.id(), expected_id(*kind));
        }
    }

    #[test]
    fn every_variant_carries_the_severity_it_declares() {
        for kind in KernelDiagnostic::ALL {
            assert_eq!(kind.severity(), expected_severity(kind.id()));
        }
    }

    #[test]
    fn each_areas_table_answers_only_for_its_own_area() {
        let contract = KernelDiagnostic::ContractNoTypes;
        assert_eq!(expected_discovery_id(contract), "");
        assert_eq!(expected_installation_id(contract), "");
        assert_eq!(expected_compat_id(contract), "");
        let discovery = KernelDiagnostic::DiscoveryNestedVault;
        assert_eq!(expected_contract_id(discovery), "");
    }

    #[test]
    fn every_variant_renders_as_its_identifier() {
        for kind in KernelDiagnostic::ALL {
            assert_eq!(kind.to_string(), kind.id());
        }
    }

    #[test]
    fn every_variant_is_debuggable_and_comparable() {
        for kind in KernelDiagnostic::ALL {
            let copy = *kind;
            assert_eq!(copy, *kind);
            assert!(!format!("{kind:?}").is_empty());
        }
    }

    #[test]
    fn all_is_the_registry_and_nothing_else() {
        assert_eq!(KernelDiagnostic::ALL.len(), REGISTRY.len());
        assert_eq!(registry_variants().as_slice(), KernelDiagnostic::ALL);
    }

    #[test]
    fn identifiers_are_unique() {
        let mut seen = BTreeSet::new();
        for kind in KernelDiagnostic::ALL {
            assert!(seen.insert(kind.id()), "duplicate identifier");
        }
        assert_eq!(seen.len(), KernelDiagnostic::ALL.len());
    }

    #[test]
    fn identifiers_are_a_known_area_and_a_kebab_slug() {
        for kind in KernelDiagnostic::ALL {
            let (area, slug) = kind.id().split_once('.').expect("`<area>.<slug>`");
            assert!(AREAS.contains(&area), "unknown area");
            assert!(is_kebab(slug), "slug is not kebab-case");
        }
    }

    #[test]
    fn a_kernel_identifier_renders_as_its_slug() {
        let id = DiagnosticId::kernel(KernelDiagnostic::ContractNoTypes);
        assert_eq!(id.as_str(), "contract.no-types");
        assert_eq!(id.to_string(), "contract.no-types");
        let copy = id.clone();
        assert_eq!(copy, id);
        assert!(format!("{id:?}").contains("ContractNoTypes"));
    }

    #[test]
    fn external_identifiers_accept_the_ext_namespace() {
        let short = DiagnosticId::external("ext.acme.dangling-link").expect("well-formed");
        assert_eq!(short.as_str(), "ext.acme.dangling-link");
        let long = DiagnosticId::external("ext.acme.house-style.line-length").expect("well-formed");
        assert_eq!(long.as_str(), "ext.acme.house-style.line-length");
    }

    #[test]
    fn external_identifiers_reject_everything_outside_the_ext_namespace() {
        let rejected = [
            "contract.unknown-key",
            "extended.foo.bar",
            "ext.",
            "ext.ns",
            "ext.NS.slug",
            "",
        ];
        for id in rejected {
            let error = DiagnosticId::external(id).expect_err("must be rejected");
            assert_eq!(error.id(), id);
        }
    }

    #[test]
    fn the_rejection_names_the_identifier_and_the_shape_required() {
        let error = DiagnosticId::external("ext.ns").expect_err("must be rejected");
        let rendered = error.to_string();
        assert!(rendered.contains("ext.ns"));
        assert!(rendered.contains("ext.<namespace>.<slug>"));
        let copy = error.clone();
        assert_eq!(copy, error);
        assert!(format!("{error:?}").contains("InvalidExternalId"));
    }

    #[test]
    fn kebab_words_reject_empty_and_uppercase_segments() {
        assert!(is_kebab("no-vault-found"));
        assert!(is_kebab("utf8"));
        assert!(!is_kebab("Not-Kebab"));
        assert!(!is_kebab("trailing-"));
        let empty = "";
        assert!(!is_kebab(empty));
    }
}
