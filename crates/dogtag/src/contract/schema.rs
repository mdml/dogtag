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

use super::kinds::ScalarKind;
use super::model::Capability;

/// Which keys a `[[type.property]]` may carry, which depends on the kind it
/// declares: `values` belongs to an `enum` and `of` to a `list`.
pub(crate) struct PropertyKeys {
    /// A property declaring `kind = "enum"`.
    pub(crate) enumeration: &'static [&'static str],
    /// A property declaring `kind = "list"` whose elements are scalars.
    pub(crate) list: &'static [&'static str],
    /// A property declaring one of the scalar kinds.
    pub(crate) scalar: &'static [&'static str],
    /// A property whose kind did not resolve, which carries every kind's keys
    /// so an unknown kind produces one diagnostic rather than three.
    pub(crate) unresolved: &'static [&'static str],
}

/// What a property spells about its own shape, which is what decides the keys
/// it may carry.
///
/// Two spellings rather than one, because `field` is legal on a `list` exactly
/// when that list names `record` in `of` — the same rule that makes `values`
/// legal on an `enum` and nowhere else, one level further down.
pub(crate) struct Shape<'a> {
    /// The `kind` the property declares, when it declares a string.
    pub(crate) kind: Option<&'a str>,
    /// The `of` the property declares, when it declares a string.
    pub(crate) of: Option<&'a str>,
}

/// Which keys a `[[type.property.field]]` may carry, which depends on the kind
/// the field declares: `values` belongs to an `enum`.
///
/// No arm carries `of` or `field`, at any kind. A field may be neither a `list`
/// nor a `record`, so the keys those two constructs are declared with are not
/// field keys at all — and a field spelling one is told so twice, once for the
/// kind it may not hold and once for the key that does not exist here, because
/// both are true and each has its own repair.
pub(crate) struct FieldKeys {
    /// A field declaring `kind = "enum"`.
    pub(crate) enumeration: &'static [&'static str],
    /// A field declaring one of the scalar kinds.
    pub(crate) scalar: &'static [&'static str],
    /// A field whose kind did not resolve, which carries every field kind's
    /// keys so an unknown kind produces one diagnostic rather than two.
    pub(crate) unresolved: &'static [&'static str],
}

/// What the record kind defines at a version that defines it: the two property
/// shapes that carry a field list, the field's own key sets, and the value its
/// one optional leaf takes when a contract omits it.
///
/// Keys and default travel together for the reason [`TagVocabulary`]'s do: a
/// version that does not define the record kind has *no row* rather than an
/// inert one, so `kind = "record"` and `of = "record"` are kinds that version
/// does not define rather than kinds it defines and refuses.
pub(crate) struct RecordKind {
    /// `[[type.property]]` declaring `kind = "record"`.
    pub(crate) property: &'static [&'static str],
    /// `[[type.property]]` declaring `kind = "list"` over records — and over an
    /// element kind that did not resolve, so a misspelled `of` beside a field
    /// list produces one diagnostic rather than two.
    pub(crate) property_list: &'static [&'static str],
    /// `[[type.property.field]]`.
    pub(crate) field: FieldKeys,
    /// Whether a `[[type.property.field]]` declaring no `required` is required.
    pub(crate) field_required: bool,
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

/// What the write seats define at a version that defines them: the key set of
/// `[capture]`, the directory an undeclared `[capture]` resolves to, and the
/// birth state a type declaring none resolves to.
///
/// Keys and defaults travel together for the reason [`TagVocabulary`]'s do: the
/// two seats are version-scoped as one construct. There is no version defining
/// `[capture]` without the birth state, and none defining a default for a leaf
/// it never reads, so a version that does not define the write seats has *no
/// row* rather than an inert one in two separate tables.
pub(crate) struct WriteSeats {
    /// `[capture]`.
    pub(crate) capture: &'static [&'static str],
    /// The vault-relative directory a contract declaring no `[capture]`
    /// captures into.
    ///
    /// A value rather than an absence, because the seat's whole purpose is that
    /// two agents on one vault agree about the landing spot: a contract that
    /// declares nothing still has an answer, and it is this one.
    pub(crate) directory: &'static str,
    /// Which flags a `[[type]]` declaring no birth state stamps at birth.
    ///
    /// Empty: an undeclared birth state stamps nothing, which is the polarity
    /// the lifecycle model's absence-is-ordinary rule requires of a flag.
    pub(crate) born_flagged: &'static [&'static str],
}

/// What the tag vocabulary defines at a version that defines it: the key sets
/// of its two tables, and the value its one optional leaf takes when a contract
/// omits it.
///
/// Keys and default travel together because the construct is version-scoped as
/// a whole. There is no version defining `[tags]` without
/// `[[type.tag-namespace]]`, and none defining a default for a leaf it never
/// reads, so a version that does not define the vocabulary has *no row* rather
/// than an inert one in three separate tables.
pub(crate) struct TagVocabulary {
    /// `[tags]`.
    pub(crate) tags: &'static [&'static str],
    /// `[[type.tag-namespace]]`.
    pub(crate) namespace: &'static [&'static str],
    /// Whether a `[[type.tag-namespace]]` declaring no `required` is required.
    pub(crate) namespace_required: bool,
}

/// The validity rules one contract version imposes on what a contract may
/// *say*, as distinct from which keys it may spell and what an omission
/// resolves to.
///
/// Validity is part of a version's schema. A contract that loaded clean at the
/// version it declares must keep loading forever, so a rule added by a later
/// version is scoped to that version and above rather than applied to every
/// contract this release reads — otherwise the upgrade promise the floor policy
/// exists to keep would break on the first bump that tightened anything.
pub(crate) struct Rules {
    /// Whether the catch-all type may declare something a note must carry.
    ///
    /// Version 1 allows it and version 2 refuses it: every untyped note binds
    /// to the catch-all, so a requiring catch-all renders "accepts anything"
    /// beside requirements every untyped note instantly fails. A version-1
    /// corpus in that shape simply collects missing-required findings on its
    /// untyped notes instead.
    pub(crate) catch_all_may_require: bool,
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
    /// What a contract at this version may say.
    pub(crate) rules: Rules,
    /// The tag vocabulary, at a version that defines it.
    ///
    /// `None` is the whole of "this version has no tag vocabulary", and it
    /// gates the *walk* rather than only the key sweep. A version-1 contract
    /// writing `[tags]` is refused as an unknown key **and** resolves to a
    /// model with no tag vocabulary in it — the "absent from, never defaulted
    /// into" line the vault-contract record draws, which no key set on its own
    /// can hold, because a key set decides legality and this decides existence.
    pub(crate) tags: Option<TagVocabulary>,
    /// The record kind, at a version that defines it.
    ///
    /// `None` the same way [`Schema::tags`] is: it gates the *value* vocabulary
    /// as well as the key sets, so at a version without this row `record` is
    /// simply not a kind — which is what stops a version-1 contract acquiring a
    /// construct only version 2 defines the moment the lattice widens.
    pub(crate) records: Option<RecordKind>,
    /// The write seats, at a version that defines them.
    ///
    /// `None` the same way [`Schema::tags`] is, and with the same consequence:
    /// a version-1 or version-2 model carries no capture declaration and no
    /// birth state at all, rather than one resolved from a table its format
    /// never had. What that does *not* do is disable the verb — `capture` reads
    /// [`WriteSeats::directory`] and [`WriteSeats::born_flagged`] from
    /// [`CURRENT_WRITE_SEATS`] where a model carries no seat, so a version-2
    /// vault captures into the default directory with no birth flag, exactly as
    /// a version-3 vault declaring neither does. The seats configure the verb;
    /// they do not enable it.
    pub(crate) write: Option<WriteSeats>,
}

impl Schema {
    /// Which keys a property of this `shape` may carry.
    pub(crate) fn property_keys(&self, shape: &Shape<'_>) -> &'static [&'static str] {
        match shape.kind {
            Some("enum") => self.keys.property.enumeration,
            Some("list") => self.list_keys(shape.of),
            Some("record") => self.record_keys(),
            Some(other) if ScalarKind::named(other).is_some() => self.keys.property.scalar,
            _ => self.keys.property.unresolved,
        }
    }

    /// Which keys a `list` naming `of` may carry: a field list exactly where
    /// the elements are records, or where the element kind did not resolve.
    fn list_keys(&self, of: Option<&str>) -> &'static [&'static str] {
        let Some(records) = self.records.as_ref() else {
            return self.keys.property.list;
        };
        match of {
            Some(spelled) if ScalarKind::named(spelled).is_some() => self.keys.property.list,
            _ => records.property_list,
        }
    }

    /// Which keys a property declaring `kind = "record"` may carry — which at a
    /// version defining no record kind is the set for a kind it does not
    /// define.
    fn record_keys(&self) -> &'static [&'static str] {
        self.records
            .as_ref()
            .map_or(self.keys.property.unresolved, |records| records.property)
    }
}

impl RecordKind {
    /// Which keys a record field declaring `spelled` may carry.
    pub(crate) fn field_keys(&self, spelled: Option<&str>) -> &'static [&'static str] {
        match spelled {
            Some("enum") => self.field.enumeration,
            Some(other) if ScalarKind::named(other).is_some() => self.field.scalar,
            _ => self.field.unresolved,
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
/// Stated in full rather than as version 1's row plus a delta: the two rows are
/// read side by side when a reader asks what a version changed, and a delta
/// would hide `tags` and `tag-namespace` behind a syntax rather than showing
/// them in the sets they joined. Version 1's row is pinned as whole equalities
/// by test, so a key added to both at once fails there.
///
/// The two shapes that carry a field list are not here but in
/// [`VERSION_2_RECORDS`], for the reason that row exists: a version defining no
/// record kind has no set for them at all. What *is* here is `field` in the
/// unresolved arm, so a property whose `kind` is a typo is told about the typo
/// rather than about the field list it wrote underneath.
const VERSION_2_KEYS: Keys = Keys {
    root: &[
        "contract_version",
        "dialect",
        "flag",
        "lifecycle",
        "tags",
        "type",
    ],
    dialect: &["links"],
    flag: &["property"],
    lifecycle: &["axis", "none", "ordinary"],
    ordinary: &["absent", "value"],
    declared_type: &[
        "capabilities",
        "name",
        "property",
        "relationship",
        "tag-namespace",
    ],
    relationship: &["predicate", "required"],
    property: PropertyKeys {
        enumeration: &["kind", "name", "required", "values"],
        list: &["kind", "name", "of", "required"],
        scalar: &["kind", "name", "required"],
        unresolved: &["field", "kind", "name", "of", "required", "values"],
    },
};

/// The record kind contract version 2 defines.
///
/// `field` is legal on exactly two property shapes — `kind = "record"`, and
/// `kind = "list"` with `of = "record"` — because those are the two shapes the
/// adopted sketch declares fields under. The sketch's only worked example is
/// the list, whose fields sit in `[[type.property.field]]` on the property
/// itself; the prose that adopts it attaches the field list to `kind =
/// "record"` and never says where a `list` of `record` declares one, so the
/// sketch is what settles it. One place a field is declared, for both shapes.
const VERSION_2_RECORDS: RecordKind = RecordKind {
    property: &["field", "kind", "name", "required"],
    property_list: &["field", "kind", "name", "of", "required"],
    field: FieldKeys {
        enumeration: &["kind", "name", "required", "values"],
        scalar: &["kind", "name", "required"],
        unresolved: &["kind", "name", "required", "values"],
    },
    field_required: false,
};

/// The tag vocabulary contract version 2 defines.
///
/// `open` and `values` are both legal keys and exactly one of them may be
/// written; that is a validity rule rather than a key rule, so the sweep admits
/// both and the `tags` module refuses the pair.
const VERSION_2_TAGS: TagVocabulary = TagVocabulary {
    tags: &["property"],
    namespace: &["open", "prefix", "required", "values"],
    namespace_required: false,
};

/// The key sets contract version 3 defines.
///
/// Stated in full for the reason [`VERSION_2_KEYS`] is: the rows are read side
/// by side when a reader asks what a version changed, and version 2's row is
/// pinned as whole equalities by test, so a key added to both at once fails
/// there.
///
/// Two keys join, and they are the whole of version 3's motion: `capture` at
/// the root, and `born-flagged` on a `[[type]]`. `[capture]`'s own key set is
/// not here but in [`VERSION_3_WRITE`], for the reason that row exists — a
/// version defining no write seats has no set for it at all.
const VERSION_3_KEYS: Keys = Keys {
    root: &[
        "capture",
        "contract_version",
        "dialect",
        "flag",
        "lifecycle",
        "tags",
        "type",
    ],
    dialect: &["links"],
    flag: &["property"],
    lifecycle: &["axis", "none", "ordinary"],
    ordinary: &["absent", "value"],
    declared_type: &[
        "born-flagged",
        "capabilities",
        "name",
        "property",
        "relationship",
        "tag-namespace",
    ],
    relationship: &["predicate", "required"],
    property: PropertyKeys {
        enumeration: &["kind", "name", "required", "values"],
        list: &["kind", "name", "of", "required"],
        scalar: &["kind", "name", "required"],
        unresolved: &["field", "kind", "name", "of", "required", "values"],
    },
};

/// The write seats contract version 3 defines.
///
/// `[capture]` carries one key, and the birth state carries none of its own: it
/// is a leaf on `[[type]]`, declared in [`VERSION_3_KEYS`] beside the other
/// keys a type may spell. What is here is the pair of defaults, which is the
/// half a key set cannot state — where a contract declaring no `[capture]`
/// captures, and what a type declaring no birth state stamps.
const VERSION_3_WRITE: WriteSeats = WriteSeats {
    capture: &["directory"],
    directory: super::capture::DEFAULT_CAPTURE_DIRECTORY,
    born_flagged: &[],
};

/// The values contract version 1 supplies for an omitted leaf.
const VERSION_1_DEFAULTS: Defaults = Defaults {
    type_capabilities: &[],
    property_required: false,
    relationship_required: false,
};

/// The values contract version 2 supplies for an omitted leaf.
///
/// Version 2's table is version 1's plus what version 2's own constructs
/// declare. A tag namespace's `required` and a record field's live in
/// [`VERSION_2_TAGS`] and [`VERSION_2_RECORDS`] beside the keys they belong to
/// rather than here, because version 1 has no row for a leaf it never reads.
const VERSION_2_DEFAULTS: Defaults = VERSION_1_DEFAULTS;

/// The values contract version 3 supplies for an omitted leaf.
///
/// Version 2's table unchanged: version 3's own two defaults are the write
/// seats', and they live in [`VERSION_3_WRITE`] beside the keys they belong to
/// for the same reason the tag vocabulary's and the record kind's do.
const VERSION_3_DEFAULTS: Defaults = VERSION_2_DEFAULTS;

/// What contract version 1 lets a contract say.
///
/// Version 1's validity is frozen: a contract that loaded clean at
/// `0.1.0-beta.1` keeps loading. Every rule version 2 adds is off here.
const VERSION_1_RULES: Rules = Rules {
    catch_all_may_require: true,
};

/// What contract version 2 lets a contract say.
const VERSION_2_RULES: Rules = Rules {
    catch_all_may_require: false,
};

/// What contract version 3 lets a contract say.
///
/// Version 2's validity unchanged, and the catch-all rule is load-bearing here
/// rather than inherited by habit: a capture binds to the catch-all, and a
/// catch-all that may require nothing is exactly why a capture cannot fail
/// contract rules by construction. Version 3 is the version that writes, so it
/// is the version that most needs the rule.
const VERSION_3_RULES: Rules = VERSION_2_RULES;

/// Contract version 1.
pub(crate) static VERSION_1: Schema = Schema {
    version: 1,
    keys: VERSION_1_KEYS,
    defaults: VERSION_1_DEFAULTS,
    rules: VERSION_1_RULES,
    tags: None,
    records: None,
    write: None,
};

/// Contract version 2.
pub(crate) static VERSION_2: Schema = Schema {
    version: 2,
    keys: VERSION_2_KEYS,
    defaults: VERSION_2_DEFAULTS,
    rules: VERSION_2_RULES,
    tags: Some(VERSION_2_TAGS),
    records: Some(VERSION_2_RECORDS),
    write: None,
};

/// Contract version 3.
///
/// The tag vocabulary and the record kind carry over unchanged: version 3 adds
/// the write seats and subtracts nothing, so a version-2 contract renamed to
/// version 3 means what it meant.
pub(crate) static VERSION_3: Schema = Schema {
    version: 3,
    keys: VERSION_3_KEYS,
    defaults: VERSION_3_DEFAULTS,
    rules: VERSION_3_RULES,
    tags: Some(VERSION_2_TAGS),
    records: Some(VERSION_2_RECORDS),
    write: Some(VERSION_3_WRITE),
};

/// Every contract version this release reads, in ascending order.
///
/// The floor does not rise during the beta, so this list only ever grows: the
/// SDK carries every historical version's parse rules and default tables, and
/// that cost is the price of a newer tool loading an older vault.
static SCHEMAS: &[&Schema] = &[&VERSION_1, &VERSION_2, &VERSION_3];

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
        assert_eq!(of(3).map(|schema| schema.version), Some(3));
    }

    #[test]
    fn version_3_adds_the_two_write_seats_to_version_2s_sets() {
        // Pinned as whole equalities, the way version 2's row is: a key added
        // to version 3 and to an older version in one edit fails on the older
        // version's own pinning test, and one added here alone fails on this.
        let joined: [&[&str]; 2] = [VERSION_3.keys.root, VERSION_3.keys.declared_type];
        assert_eq!(
            joined,
            [
                &[
                    "capture",
                    "contract_version",
                    "dialect",
                    "flag",
                    "lifecycle",
                    "tags",
                    "type"
                ][..],
                &[
                    "born-flagged",
                    "capabilities",
                    "name",
                    "property",
                    "relationship",
                    "tag-namespace"
                ][..],
            ]
        );
    }

    #[test]
    fn version_3_carries_version_2s_constructs_unchanged() {
        // Version 3 adds and subtracts nothing else, so a version-2 contract
        // renamed to version 3 means what it meant. Held as equalities over the
        // rows that could silently drift.
        let unchanged: [&[&str]; 4] = [
            VERSION_3.keys.dialect,
            VERSION_3.keys.lifecycle,
            VERSION_3.property_keys(&of_kind("enum")),
            VERSION_3.property_keys(&of_kind("record")),
        ];
        assert_eq!(
            unchanged,
            [
                VERSION_2.keys.dialect,
                VERSION_2.keys.lifecycle,
                VERSION_2.property_keys(&of_kind("enum")),
                VERSION_2.property_keys(&of_kind("record")),
            ]
        );
        let vocabulary = VERSION_3.tags.as_ref().expect("version 3 defines it");
        assert_eq!(vocabulary.namespace, VERSION_2_TAGS.namespace);
        let records = VERSION_3.records.as_ref().expect("version 3 defines it");
        assert_eq!(records.property, VERSION_2_RECORDS.property);
        assert!(!VERSION_3.rules.catch_all_may_require);
        assert_eq!(
            VERSION_3.defaults.type_capabilities,
            VERSION_2.defaults.type_capabilities
        );
    }

    #[test]
    fn only_version_3_defines_the_write_seats() {
        // Existence rather than legality, the way the tag vocabulary is: a
        // version with no row has no capture declaration and no birth state in
        // its model at all, which is what stops an older vault acquiring a
        // construct only version 3 defines.
        let seats = VERSION_3.write.as_ref().expect("version 3 defines them");
        assert_eq!(seats.capture, ["directory"]);
        assert_eq!(
            seats.directory,
            super::super::capture::DEFAULT_CAPTURE_DIRECTORY
        );
        assert_eq!(seats.born_flagged, [""; 0]);
        let absent = (VERSION_1.write.is_some(), VERSION_2.write.is_some());
        assert_eq!(absent, (false, false));
    }

    #[test]
    fn version_1_declares_exactly_the_keys_it_declared_at_the_first_release() {
        // Pinned as whole sets rather than by absence, so a construct added to
        // version 2 cannot be added to version 1 in the same edit without this
        // failing.
        let keys = &VERSION_1.keys;
        let tables: [&[&str]; 7] = [
            keys.root,
            keys.dialect,
            keys.flag,
            keys.lifecycle,
            keys.ordinary,
            keys.declared_type,
            keys.relationship,
        ];
        assert_eq!(
            tables,
            [
                &["contract_version", "dialect", "flag", "lifecycle", "type"][..],
                &["links"][..],
                &["property"][..],
                &["axis", "none", "ordinary"][..],
                &["absent", "value"][..],
                &["capabilities", "name", "property", "relationship"][..],
                &["predicate", "required"][..],
            ]
        );
        let properties: [&[&str]; 4] = [
            keys.property.enumeration,
            keys.property.list,
            keys.property.scalar,
            keys.property.unresolved,
        ];
        assert_eq!(
            properties,
            [
                &["kind", "name", "required", "values"][..],
                &["kind", "name", "of", "required"][..],
                &["kind", "name", "required"][..],
                &["kind", "name", "of", "required", "values"][..],
            ]
        );
    }

    #[test]
    fn version_1_defaults_exactly_the_three_leaves_it_defaulted_at_the_first_release() {
        let defaults = &VERSION_1.defaults;
        assert_eq!(defaults.type_capabilities, []);
        assert!(!defaults.property_required);
        assert!(!defaults.relationship_required);
    }

    #[test]
    fn version_1_keeps_the_validity_it_had_at_the_first_release() {
        // Validity is part of a version's schema. Version 2 refuses a
        // requiring catch-all; version 1 must not, or a vault that loaded
        // clean at `0.1.0-beta.1` stops loading on upgrade.
        assert!(VERSION_1.rules.catch_all_may_require);
        assert!(!VERSION_2.rules.catch_all_may_require);
    }

    /// A property spelling `kind` and no `of`.
    fn of_kind(kind: &str) -> Shape<'_> {
        Shape {
            kind: Some(kind),
            of: None,
        }
    }

    #[test]
    fn a_propertys_key_set_follows_the_kind_it_declares() {
        assert_eq!(
            VERSION_1.property_keys(&of_kind("enum")),
            ["kind", "name", "required", "values"]
        );
        assert_eq!(
            VERSION_1.property_keys(&of_kind("list")),
            ["kind", "name", "of", "required"]
        );
        assert_eq!(
            VERSION_1.property_keys(&of_kind("string")),
            ["kind", "name", "required"]
        );
    }

    #[test]
    fn a_property_whose_kind_did_not_resolve_carries_every_kinds_keys() {
        let every = ["kind", "name", "of", "required", "values"];
        assert_eq!(VERSION_1.property_keys(&of_kind("url")), every);
        let nameless = Shape {
            kind: None,
            of: None,
        };
        assert_eq!(VERSION_1.property_keys(&nameless), every);
    }

    #[test]
    fn a_version_that_defines_no_record_kind_defines_neither_spelling_of_it() {
        // `record` reaches the arm for a kind version 1 does not define, and a
        // `list` at version 1 carries the same keys whatever it names in `of`,
        // so `field` is an unknown key wherever a version-1 contract writes it.
        let every = ["kind", "name", "of", "required", "values"];
        assert_eq!(VERSION_1.property_keys(&of_kind("record")), every);
        let list_of_record = Shape {
            kind: Some("list"),
            of: Some("record"),
        };
        assert_eq!(
            VERSION_1.property_keys(&list_of_record),
            ["kind", "name", "of", "required"]
        );
        assert!(VERSION_1.records.is_none());
    }

    #[test]
    fn only_the_two_record_shapes_carry_a_field_list() {
        let with_of = |of| {
            VERSION_2.property_keys(&Shape {
                kind: Some("list"),
                of: Some(of),
            })
        };
        // An element kind that did not resolve carries the field list too, and
        // so does a `list` with no `of` at all, so a misspelled `of` beside a
        // field list is one diagnostic rather than two.
        let shapes: [&[&str]; 5] = [
            VERSION_2.property_keys(&of_kind("record")),
            with_of("record"),
            with_of("recrod"),
            VERSION_2.property_keys(&of_kind("list")),
            with_of("string"),
        ];
        let carrying = &["field", "kind", "name", "of", "required"][..];
        assert_eq!(
            shapes,
            [
                &["field", "kind", "name", "required"][..],
                carrying,
                carrying,
                carrying,
                &["kind", "name", "of", "required"][..],
            ]
        );
    }

    #[test]
    fn a_fields_key_set_follows_the_kind_it_declares() {
        let records = VERSION_2.records.as_ref().expect("version 2 defines it");
        // Never `of` and never `field`: a field may be neither a `list` nor a
        // `record`, so neither key exists at this level at any kind.
        let every = &["kind", "name", "required", "values"][..];
        let sets: [&[&str]; 5] = [
            records.field_keys(Some("enum")),
            records.field_keys(Some("date")),
            records.field_keys(Some("url")),
            records.field_keys(Some("list")),
            records.field_keys(None),
        ];
        let scalar = &["kind", "name", "required"][..];
        assert_eq!(sets, [every, scalar, every, every, every]);
        assert!(!records.field_required);
    }

    #[test]
    fn version_2_adds_the_tag_vocabularys_two_tables_to_version_1s_sets() {
        let joined: [&[&str]; 2] = [VERSION_2.keys.root, VERSION_2.keys.declared_type];
        assert_eq!(
            joined,
            [
                &[
                    "contract_version",
                    "dialect",
                    "flag",
                    "lifecycle",
                    "tags",
                    "type"
                ][..],
                &[
                    "capabilities",
                    "name",
                    "property",
                    "relationship",
                    "tag-namespace"
                ][..],
            ]
        );
        let unchanged = (
            VERSION_2.defaults.property_required,
            VERSION_2.property_keys(&Shape {
                kind: Some("list"),
                of: Some("date"),
            }),
        );
        assert_eq!(
            unchanged,
            (
                VERSION_1.defaults.property_required,
                VERSION_1.property_keys(&of_kind("list"))
            )
        );
    }

    #[test]
    fn only_version_2_defines_the_tag_vocabulary() {
        // Existence rather than legality: version 1 has no row at all, so the
        // walk that would resolve a tag vocabulary never runs for it and a
        // version-1 model cannot carry one.
        let vocabulary = VERSION_2.tags.as_ref().expect("version 2 defines it");
        let declared: [&[&str]; 2] = [vocabulary.tags, vocabulary.namespace];
        assert_eq!(
            declared,
            [
                &["property"][..],
                &["open", "prefix", "required", "values"][..]
            ]
        );
        let absent = (vocabulary.namespace_required, VERSION_1.tags.is_some());
        assert_eq!(absent, (false, false));
    }
}
