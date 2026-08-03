//! What each contract version defines: its key sets and its default table.
//!
//! Key legality and resolution are both scoped to the version a contract
//! *declares*. A version-1 contract is judged against version 1's key set, its
//! omissions resolve against version 1's default table, and a construct only a
//! later version defines is **absent from** — never defaulted into — a
//! version-1 model. Without the second half, a build reading two versions would
//! resolve a version-1 vault's omissions against version 2's table, change that
//! vault's semantics on upgrade, and record provenance asserting it had not:
//! provenance that lies, which the vault-contract record names as worse than
//! plain inheritance.
//!
//! There is deliberately **no upgrade-on-read**. A value produced by an upgrade
//! comes from neither the file nor a format default, so provenance would need a
//! fourth source or would lie; a version-1 model simply lacks whatever version 1
//! does not define.
//!
//! One [`Schema`] per version, selected by [`of`] from the version the contract
//! declares. [`of`] is also the gate: a contract reaches the body walk only by
//! having a schema, so *this release reads that version* and *this release
//! carries that version's parse rules* are one fact rather than two that can
//! drift apart. That is the shape the first amendment to the vault-contract
//! record made due before the supported range could widen — widening it without
//! these two dimensions is the regression it named.

use super::model::{Capability, ScalarKind};

/// Which keys a `[[type.property]]` may carry, which depends on the kind it
/// declares: `values` belongs to an `enum` and `of` to a `list`.
pub(crate) struct PropertyKeys {
    /// A property declaring `kind = "enum"`.
    pub(crate) enumeration: &'static [&'static str],
    /// A property declaring `kind = "list"`.
    pub(crate) list: &'static [&'static str],
    /// A property declaring one of the scalar kinds.
    pub(crate) scalar: &'static [&'static str],
    /// A property whose kind did not resolve, which carries every kind's keys
    /// so an unknown kind produces one diagnostic rather than three.
    pub(crate) unresolved: &'static [&'static str],
}

/// Every key each of the contract's tables defines, at one version.
pub(crate) struct Keys {
    /// The contract root.
    pub(crate) root: &'static [&'static str],
    /// `[dialect]`.
    pub(crate) dialect: &'static [&'static str],
    /// `[[flag]]`.
    pub(crate) flag: &'static [&'static str],
    /// `[lifecycle]`.
    pub(crate) lifecycle: &'static [&'static str],
    /// `[lifecycle.ordinary]`.
    pub(crate) ordinary: &'static [&'static str],
    /// `[[type]]`.
    pub(crate) declared_type: &'static [&'static str],
    /// `[[type.relationship]]`.
    pub(crate) relationship: &'static [&'static str],
    /// `[[type.property]]`.
    pub(crate) property: PropertyKeys,
}

/// What one contract version supplies for each leaf a contract may omit.
///
/// A default is a property of the *version*, never of the SDK: changing one
/// takes a version bump that an unchanged vault does not have, which is what
/// separates a format default from the live inheritance the settings PDR ruled
/// out.
///
/// The vault-contract record's 2026-08-03 amendment ratifies version 1's table
/// as "exactly the two implemented literals: `required = false` on a property,
/// `capabilities = []` on a type" — a count of two that the implementation has
/// carried as three since M2, because a relationship's `required` is defaulted
/// on the same rule and its `default` provenance entry is pinned by test. The
/// reading taken here is that `required = false` names one rule that binds at
/// every leaf spelled `required`, so version 1's table has three entries and
/// the record's count is short by one.
pub(crate) struct Defaults {
    /// The capabilities a `[[type]]` declaring no `capabilities` carries.
    pub(crate) type_capabilities: &'static [Capability],
    /// Whether a `[[type.property]]` declaring no `required` is required.
    pub(crate) property_required: bool,
    /// Whether a `[[type.relationship]]` declaring no `required` is required.
    pub(crate) relationship_required: bool,
}

/// One contract version's schema.
pub(crate) struct Schema {
    /// The version this is the schema of, which every message about a key or a
    /// default quotes so a reader knows which format refused them.
    pub(crate) version: u32,
    /// What each table may declare.
    pub(crate) keys: Keys,
    /// What an omission resolves to.
    pub(crate) defaults: Defaults,
}

impl Schema {
    /// Which keys a property declaring `spelled` may carry.
    pub(crate) fn property_keys(&self, spelled: Option<&str>) -> &'static [&'static str] {
        match spelled {
            Some("enum") => self.keys.property.enumeration,
            Some("list") => self.keys.property.list,
            Some(other) if ScalarKind::named(other).is_some() => self.keys.property.scalar,
            _ => self.keys.property.unresolved,
        }
    }
}

/// The key sets contract version 1 defines.
const VERSION_1_KEYS: Keys = Keys {
    root: &["contract_version", "dialect", "flag", "lifecycle", "type"],
    dialect: &["links"],
    flag: &["property"],
    lifecycle: &["axis", "none", "ordinary"],
    ordinary: &["absent", "value"],
    declared_type: &["capabilities", "name", "property", "relationship"],
    relationship: &["predicate", "required"],
    property: PropertyKeys {
        enumeration: &["kind", "name", "required", "values"],
        list: &["kind", "name", "of", "required"],
        scalar: &["kind", "name", "required"],
        unresolved: &["kind", "name", "of", "required", "values"],
    },
};

/// The key sets contract version 2 defines.
///
/// Equal to version 1's at this release, and stated as its own constant rather
/// than shared: version 2's constructs — the tag vocabulary and the record kind
/// — arrive in the changes that carry them, and *this* is the row they extend.
/// Sharing one constant would make adding a key to version 2 add it to version
/// 1 as well, which is the whole failure this mechanism exists to prevent.
const VERSION_2_KEYS: Keys = VERSION_1_KEYS;

/// The values contract version 1 supplies for an omitted leaf.
const VERSION_1_DEFAULTS: Defaults = Defaults {
    type_capabilities: &[],
    property_required: false,
    relationship_required: false,
};

/// The values contract version 2 supplies for an omitted leaf.
///
/// Version 2's table is version 1's plus whatever version 2's own constructs
/// declare, and at this release they declare nothing: the same reasoning as
/// [`VERSION_2_KEYS`], for the second of the two dimensions.
const VERSION_2_DEFAULTS: Defaults = VERSION_1_DEFAULTS;

/// Contract version 1.
pub(crate) static VERSION_1: Schema = Schema {
    version: 1,
    keys: VERSION_1_KEYS,
    defaults: VERSION_1_DEFAULTS,
};

/// Contract version 2.
pub(crate) static VERSION_2: Schema = Schema {
    version: 2,
    keys: VERSION_2_KEYS,
    defaults: VERSION_2_DEFAULTS,
};

/// Every contract version this release reads, in ascending order.
///
/// The floor does not rise during the beta, so this list only ever grows: the
/// SDK carries every historical version's parse rules and default tables, and
/// that cost is the price of a newer tool loading an older vault.
static SCHEMAS: &[&Schema] = &[&VERSION_1, &VERSION_2];

/// The schema of `version`, when this release carries one.
///
/// `None` is the whole of "this release does not read that version", which is
/// why the caller classifies rather than guessing: whether the version is below
/// the floor or above the ceiling decides what a reader is told to do about it.
pub(crate) fn of(version: u32) -> Option<&'static Schema> {
    SCHEMAS
        .iter()
        .copied()
        .find(|schema| schema.version == version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compat::SUPPORTED_CONTRACT_VERSIONS;

    #[test]
    fn a_schema_exists_for_exactly_the_versions_this_release_reads() {
        // The supported range and the carried schemas are one fact. A range
        // that widened without a schema would resolve the new version against
        // some other version's table, which is the regression the vault
        // contract's first amendment named; this is that sentence as a test.
        for version in 0..=4 {
            assert_eq!(
                of(version).is_some(),
                SUPPORTED_CONTRACT_VERSIONS.contains(&version),
                "version {version}"
            );
        }
    }

    #[test]
    fn each_schema_answers_for_the_version_it_was_asked_about() {
        assert_eq!(of(1).map(|schema| schema.version), Some(1));
        assert_eq!(of(2).map(|schema| schema.version), Some(2));
    }

    #[test]
    fn version_1_declares_exactly_the_keys_it_declared_at_the_first_release() {
        // Pinned as whole sets rather than by absence, so a construct added to
        // version 2 cannot be added to version 1 in the same edit without this
        // failing.
        let keys = &VERSION_1.keys;
        assert_eq!(
            keys.root,
            ["contract_version", "dialect", "flag", "lifecycle", "type"]
        );
        assert_eq!(keys.dialect, ["links"]);
        assert_eq!(keys.flag, ["property"]);
        assert_eq!(keys.lifecycle, ["axis", "none", "ordinary"]);
        assert_eq!(keys.ordinary, ["absent", "value"]);
        assert_eq!(
            keys.declared_type,
            ["capabilities", "name", "property", "relationship"]
        );
        assert_eq!(keys.relationship, ["predicate", "required"]);
    }

    #[test]
    fn version_1_defaults_exactly_the_three_leaves_it_defaulted_at_the_first_release() {
        let defaults = &VERSION_1.defaults;
        assert_eq!(defaults.type_capabilities, []);
        assert!(!defaults.property_required);
        assert!(!defaults.relationship_required);
    }

    #[test]
    fn a_propertys_key_set_follows_the_kind_it_declares() {
        assert_eq!(
            VERSION_1.property_keys(Some("enum")),
            ["kind", "name", "required", "values"]
        );
        assert_eq!(
            VERSION_1.property_keys(Some("list")),
            ["kind", "name", "of", "required"]
        );
        assert_eq!(
            VERSION_1.property_keys(Some("string")),
            ["kind", "name", "required"]
        );
    }

    #[test]
    fn a_property_whose_kind_did_not_resolve_carries_every_kinds_keys() {
        let every = ["kind", "name", "of", "required", "values"];
        assert_eq!(VERSION_1.property_keys(Some("url")), every);
        assert_eq!(VERSION_1.property_keys(None), every);
    }

    #[test]
    fn version_2_carries_exactly_version_1s_constructs_at_this_release() {
        // The version exists and the range reads it before either construct it
        // will carry lands, which is the slice ordering the packet fixes: the
        // mechanism ships in the change that widens the range, and the
        // constructs ship in the changes that add them.
        assert_eq!(VERSION_2.keys.root, VERSION_1.keys.root);
        assert_eq!(
            VERSION_2.defaults.property_required,
            VERSION_1.defaults.property_required
        );
        assert_eq!(
            VERSION_2.property_keys(Some("list")),
            VERSION_1.property_keys(Some("list"))
        );
    }
}
