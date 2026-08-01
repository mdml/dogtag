//! The resolved committed contract, as version 1 of the format declares it.
//!
//! Every accessor here answers a question about a **declaration**. Nothing in
//! this module knows a vocabulary word: a type is identity-bearing because the
//! contract says so, never because of what it is called, and the same holds for
//! the lifecycle axis, its ordinary state, and every flag.
//!
//! **Declaration order is preserved everywhere.** [`Contract::types`],
//! [`TypeDecl::properties`], and [`TypeDecl::relationships`] return their
//! declarations in the order the contract writes them, never sorted. Only
//! [`Contract::provenance`] is ordered by key, which is provenance's own
//! contract rather than this model's.

use core::fmt;

use crate::provenance::Provenance;

/// A committed vault contract that loaded.
///
/// A `Contract` exists only when the file resolved: it is what semantic
/// operations take, so an unresolved contract cannot be mistaken for a usable
/// one. See [`crate::contract::ContractLoad`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contract {
    pub(crate) contract_version: u32,
    pub(crate) dialect: Dialect,
    pub(crate) lifecycle: LifecycleDecl,
    pub(crate) flags: Vec<FlagDecl>,
    pub(crate) types: Vec<TypeDecl>,
    pub(crate) provenance: Provenance,
}

impl Contract {
    /// The format version the contract declares, which is the version its body
    /// was resolved against.
    pub fn contract_version(&self) -> u32 {
        self.contract_version
    }

    /// The editor dialect the corpus is written in.
    pub fn dialect(&self) -> &Dialect {
        &self.dialect
    }

    /// The lifecycle the corpus declares, which may be the explicit absence of
    /// one.
    pub fn lifecycle(&self) -> &LifecycleDecl {
        &self.lifecycle
    }

    /// The declared flags, in declaration order.
    pub fn flags(&self) -> &[FlagDecl] {
        &self.flags
    }

    /// The declared types, in declaration order.
    pub fn types(&self) -> &[TypeDecl] {
        &self.types
    }

    /// Where each resolved leaf value came from, in key order.
    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }

    /// The type declared under `name`.
    pub fn type_named(&self, name: &str) -> Option<&TypeDecl> {
        self.types.iter().find(|declared| declared.name == name)
    }

    /// Every type carrying `capability`, in declaration order.
    pub fn types_with(&self, capability: Capability) -> impl Iterator<Item = &TypeDecl> {
        self.types
            .iter()
            .filter(move |declared| declared.has(capability))
    }

    /// The one type declaring [`Capability::CatchAll`].
    ///
    /// Cardinality is enforced when the contract loads, so a resolved contract
    /// always has exactly one — the `Option` is for callers holding a
    /// `Contract` they did not load themselves.
    pub fn catch_all(&self) -> Option<&TypeDecl> {
        self.types_with(Capability::CatchAll).next()
    }

    /// Whether any type declares a property named `name`.
    pub fn declares_property(&self, name: &str) -> bool {
        self.declarations_of(name).next().is_some()
    }

    /// The kind a property name is declared with.
    ///
    /// One name declares one kind corpus-wide — that rule is enforced at load —
    /// so the first declaration answers for all of them.
    pub fn property_kind(&self, name: &str) -> Option<&PropertyKind> {
        self.declarations_of(name)
            .next()
            .map(|(_, property)| &property.kind)
    }

    /// Every type declaring a property named `name`, paired with that type's
    /// own declaration of it, in declaration order.
    pub fn declarations_of(&self, name: &str) -> impl Iterator<Item = (&TypeDecl, &PropertyDecl)> {
        self.types
            .iter()
            .filter_map(move |declared| Some((declared, declared.property(name)?)))
    }
}

/// The editor dialect the corpus is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dialect {
    pub(crate) links: LinkDialect,
}

impl Dialect {
    /// How links are spelled in the corpus.
    pub fn links(self) -> LinkDialect {
        self.links
    }
}

/// How a link is spelled.
///
/// M2 parses, validates, and explains this; the milestone that reads a note
/// consumes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkDialect {
    /// `[[target]]`.
    Wikilink,
    /// `[text](target.md)`.
    Markdown,
}

impl LinkDialect {
    /// Every dialect the format defines, in the order it declares them.
    pub const ALL: &'static [LinkDialect] = &[Self::Wikilink, Self::Markdown];

    /// The spelling the contract writes.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wikilink => "wikilink",
            Self::Markdown => "markdown",
        }
    }

    /// The dialect `name` spells, if the format defines one.
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|kind| kind.as_str() == name)
    }
}

impl fmt::Display for LinkDialect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What a corpus declares about its life axis.
///
/// The table is mandatory and the axis is optional, so a corpus with no
/// lifecycle states that it has none rather than staying silent: a forgotten
/// declaration and a deliberately absent one are never indistinguishable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LifecycleDecl {
    /// The corpus declares no life axis. This is a statement, not an omission.
    None,
    /// The corpus declares a life axis and how its ordinary state is encoded.
    Axis {
        /// The name of the `enum` property carrying the axis.
        axis: String,
        /// How the ordinary state is encoded on that property.
        ordinary: Ordinary,
    },
}

impl LifecycleDecl {
    /// Which of the two declarations this is, for structured output: `axis` or
    /// `none`.
    pub fn declared(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Axis { .. } => "axis",
        }
    }

    /// The property carrying the axis, when one is declared.
    pub fn axis(&self) -> Option<&str> {
        match self {
            Self::None => Option::None,
            Self::Axis { axis, .. } => Some(axis),
        }
    }

    /// How the ordinary state is encoded, when an axis is declared.
    pub fn ordinary(&self) -> Option<&Ordinary> {
        match self {
            Self::None => Option::None,
            Self::Axis { ordinary, .. } => Some(ordinary),
        }
    }
}

/// How a corpus encodes the ordinary state of its life axis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ordinary {
    /// The ordinary state is the *absence* of a value on the axis property.
    Absent,
    /// The ordinary state is a named member of the axis property's `enum`.
    Value(String),
}

impl Ordinary {
    /// The named ordinary value, when the ordinary state is a value.
    pub fn value(&self) -> Option<&str> {
        match self {
            Self::Absent => None,
            Self::Value(value) => Some(value),
        }
    }
}

/// A boolean property declared as a lifecycle flag.
///
/// Orthogonality is structural: a flag is a separate property from the axis, so
/// it cannot be a point on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlagDecl {
    pub(crate) property: String,
}

impl FlagDecl {
    /// The property the flag names.
    pub fn property(&self) -> &str {
        &self.property
    }
}

/// One declared note type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeDecl {
    pub(crate) name: String,
    pub(crate) capabilities: Vec<Capability>,
    pub(crate) properties: Vec<PropertyDecl>,
    pub(crate) relationships: Vec<RelationshipDecl>,
}

impl TypeDecl {
    /// The discriminator value every note of this type carries.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The capabilities the type declares, in declaration order.
    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    /// The properties the type declares, in declaration order.
    pub fn properties(&self) -> &[PropertyDecl] {
        &self.properties
    }

    /// The relationships the type declares, in declaration order.
    pub fn relationships(&self) -> &[RelationshipDecl] {
        &self.relationships
    }

    /// Whether the type declares `capability`.
    pub fn has(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }

    /// The property this type declares under `name`.
    pub fn property(&self, name: &str) -> Option<&PropertyDecl> {
        self.properties
            .iter()
            .find(|declared| declared.name == name)
    }

    /// The relationship this type declares under `predicate`.
    pub fn relationship(&self, predicate: &str) -> Option<&RelationshipDecl> {
        self.relationships
            .iter()
            .find(|declared| declared.predicate == predicate)
    }
}

/// What behavior a type binds to.
///
/// Configuration binds by capability and never by name, so the kernel reasons
/// over these declarations rather than over what a corpus calls its types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capability {
    /// A type the corpus is *about*: the target of entity resolution and of
    /// structured relationships. Any number of types may declare it.
    IdentityBearing,
    /// The bottom type that accepts anything, so capture never blocks on
    /// classification. Exactly one type declares it.
    CatchAll,
    /// A type no caller may modify. Any number of types may declare it.
    ClosedWrite,
}

impl Capability {
    /// Every capability the format defines, in the order it declares them.
    pub const ALL: &'static [Capability] =
        &[Self::IdentityBearing, Self::CatchAll, Self::ClosedWrite];

    /// The spelling the contract writes.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IdentityBearing => "identity-bearing",
            Self::CatchAll => "catch-all",
            Self::ClosedWrite => "closed-write",
        }
    }

    /// The capability `name` spells, if the format defines one.
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|kind| kind.as_str() == name)
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One property declared on one type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyDecl {
    pub(crate) name: String,
    pub(crate) kind: PropertyKind,
    pub(crate) required: bool,
}

impl PropertyDecl {
    /// The property's name, unique within its type.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The property's value kind.
    pub fn kind(&self) -> &PropertyKind {
        &self.kind
    }

    /// Whether every note of the declaring type must carry a value.
    pub fn required(&self) -> bool {
        self.required
    }
}

/// The closed lattice of value kinds, of which there are eight.
///
/// `integer` and `float` are distinct on the wire: `1` is not a `float` and
/// `1.0` is not an `integer`. There are **no value constraints** of any kind —
/// no pattern, no bounds, no format hint — because a declared constraint the
/// kernel never enforces misleads every agent that reads the contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropertyKind {
    /// Text.
    String,
    /// A whole number.
    Integer,
    /// A number written with a fractional part or an exponent.
    Float,
    /// `true` or `false`.
    Boolean,
    /// An RFC 3339 `full-date` — `YYYY-MM-DD`, and nothing else.
    ///
    /// The lexical form is the kind's entire meaning, so it is fixed here
    /// rather than left to whatever coercion a later frontmatter reader
    /// happens to perform.
    Date,
    /// An RFC 3339 `date-time` **with a mandatory offset** — for example
    /// `2026-07-31T09:15:00-04:00`. A local time with no offset is not a
    /// `datetime`.
    ///
    /// As with [`PropertyKind::Date`], the lexical form is the meaning; M2
    /// records it and nothing parses a note.
    DateTime,
    /// A closed set of named values.
    Enum {
        /// The members, in declaration order. Non-empty and free of repeats.
        values: Vec<String>,
    },
    /// A list of one scalar kind. Lists do not nest, and there is no list of
    /// `enum`: an `enum` needs its own `values`, which a `list` declaration has
    /// no way to carry.
    List {
        /// The kind of every element.
        of: ScalarKind,
    },
}

impl PropertyKind {
    /// The spelling the contract writes for the kind itself.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::String => ScalarKind::String.as_str(),
            Self::Integer => ScalarKind::Integer.as_str(),
            Self::Float => ScalarKind::Float.as_str(),
            Self::Boolean => ScalarKind::Boolean.as_str(),
            Self::Date => ScalarKind::Date.as_str(),
            Self::DateTime => ScalarKind::DateTime.as_str(),
            Self::Enum { .. } => "enum",
            Self::List { .. } => "list",
        }
    }

    /// The members of an `enum`.
    pub fn values(&self) -> Option<&[String]> {
        match self {
            Self::Enum { values } => Some(values),
            _ => None,
        }
    }

    /// The element kind of a `list`.
    pub fn element(&self) -> Option<ScalarKind> {
        match self {
            Self::List { of } => Some(*of),
            _ => None,
        }
    }

    /// The kind spelled out with whatever it carries, for a diagnostic message
    /// that has to tell two `enum` declarations apart.
    pub fn describe(&self) -> String {
        match self {
            Self::Enum { values } => format!("`enum` over {}", values.join(", ")),
            Self::List { of } => format!("`list` of `{of}`"),
            other => format!("`{}`", other.as_str()),
        }
    }
}

impl From<ScalarKind> for PropertyKind {
    fn from(kind: ScalarKind) -> Self {
        match kind {
            ScalarKind::String => Self::String,
            ScalarKind::Integer => Self::Integer,
            ScalarKind::Float => Self::Float,
            ScalarKind::Boolean => Self::Boolean,
            ScalarKind::Date => Self::Date,
            ScalarKind::DateTime => Self::DateTime,
        }
    }
}

/// A kind a `list` may hold: the six kinds that are neither `enum` nor `list`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarKind {
    /// Text.
    String,
    /// A whole number.
    Integer,
    /// A number written with a fractional part or an exponent.
    Float,
    /// `true` or `false`.
    Boolean,
    /// An RFC 3339 `full-date`, as [`PropertyKind::Date`] fixes it.
    Date,
    /// An RFC 3339 `date-time` with a mandatory offset, as
    /// [`PropertyKind::DateTime`] fixes it.
    DateTime,
}

impl ScalarKind {
    /// Every scalar kind, in the order the format declares them.
    pub const ALL: &'static [ScalarKind] = &[
        Self::String,
        Self::Integer,
        Self::Float,
        Self::Boolean,
        Self::Date,
        Self::DateTime,
    ];

    /// The spelling the contract writes.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::Boolean => "boolean",
            Self::Date => "date",
            Self::DateTime => "datetime",
        }
    }

    /// The scalar kind `name` spells, if the format defines one.
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|kind| kind.as_str() == name)
    }
}

impl fmt::Display for ScalarKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One relationship declared on one type.
///
/// `required = true` means at least one edge with this predicate must be
/// present. The maximum is undecided, so cardinality as a whole is deferred
/// rather than half-stated, and there is deliberately no key constraining which
/// types may be the far end.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationshipDecl {
    pub(crate) predicate: String,
    pub(crate) required: bool,
}

impl RelationshipDecl {
    /// The predicate, unique within its type.
    pub fn predicate(&self) -> &str {
        &self.predicate
    }

    /// Whether at least one edge with this predicate must be present.
    pub fn required(&self) -> bool {
        self.required
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn sample() -> Contract {
        Contract {
            contract_version: 1,
            dialect: Dialect {
                links: LinkDialect::Wikilink,
            },
            lifecycle: LifecycleDecl::Axis {
                axis: "status".to_owned(),
                ordinary: Ordinary::Absent,
            },
            flags: vec![FlagDecl {
                property: "leaned_on".to_owned(),
            }],
            types: vec![person(), capture()],
            provenance: Provenance::new(),
        }
    }

    fn person() -> TypeDecl {
        TypeDecl {
            name: "person".to_owned(),
            capabilities: vec![Capability::IdentityBearing],
            properties: vec![
                PropertyDecl {
                    name: "full_name".to_owned(),
                    kind: PropertyKind::String,
                    required: true,
                },
                PropertyDecl {
                    name: "status".to_owned(),
                    kind: PropertyKind::Enum {
                        values: vec!["draft".to_owned(), "archived".to_owned()],
                    },
                    required: false,
                },
            ],
            relationships: vec![RelationshipDecl {
                predicate: "works-at".to_owned(),
                required: false,
            }],
        }
    }

    fn capture() -> TypeDecl {
        TypeDecl {
            name: "capture".to_owned(),
            capabilities: vec![Capability::CatchAll, Capability::ClosedWrite],
            properties: Vec::new(),
            relationships: Vec::new(),
        }
    }

    #[test]
    fn a_contract_answers_for_its_own_declarations() {
        let contract = sample();
        let declared = (
            contract.contract_version(),
            contract.dialect().links(),
            contract.lifecycle().declared(),
            contract.flags()[0].property(),
        );
        assert_eq!(declared, (1, LinkDialect::Wikilink, "axis", "leaned_on"));
        assert!(contract.provenance().is_empty());
    }

    #[test]
    fn types_come_out_in_declaration_order_never_sorted() {
        let contract = sample();
        let names: Vec<&str> = contract.types().iter().map(TypeDecl::name).collect();
        assert_eq!(names, ["person", "capture"]);
    }

    #[test]
    fn a_type_is_found_by_name_and_missing_names_answer_nothing() {
        let contract = sample();
        let found = contract.type_named("capture").map(TypeDecl::name);
        assert_eq!(
            (found, contract.type_named("absent").is_some()),
            (Some("capture"), false)
        );
    }

    #[test]
    fn types_are_selected_by_the_capability_they_declare() {
        let contract = sample();
        let bearing: Vec<&str> = contract
            .types_with(Capability::IdentityBearing)
            .map(TypeDecl::name)
            .collect();
        assert_eq!(bearing, ["person"]);
        // The catch-all type also declares closed-write, so the two selections
        // overlap without either being a subset of the other.
        let closed = contract.types_with(Capability::ClosedWrite).count();
        assert_eq!(
            (contract.catch_all().map(TypeDecl::name), closed),
            (Some("capture"), 1)
        );
    }

    #[test]
    fn a_contract_with_no_catch_all_answers_nothing() {
        let mut contract = sample();
        contract.types = vec![person()];
        assert!(contract.catch_all().is_none());
    }

    #[test]
    fn a_property_name_is_looked_up_across_every_type() {
        let contract = sample();
        let declared = (
            contract.declares_property("status"),
            contract.declares_property("absent"),
        );
        assert_eq!(declared, (true, false));
        let kinds = (
            contract.property_kind("full_name"),
            contract.property_kind("absent"),
        );
        assert_eq!(kinds, (Some(&PropertyKind::String), None));
    }

    #[test]
    fn every_type_declaring_a_property_is_reported_with_its_own_declaration() {
        let contract = sample();
        let declarations: Vec<(&str, bool)> = contract
            .declarations_of("full_name")
            .map(|(declared, property)| (declared.name(), property.required()))
            .collect();
        assert_eq!(declarations, [("person", true)]);
        assert_eq!(contract.declarations_of("absent").count(), 0);
    }

    #[test]
    fn a_type_answers_for_its_properties_in_declaration_order() {
        let declared = person();
        let names: Vec<&str> = declared
            .properties()
            .iter()
            .map(PropertyDecl::name)
            .collect();
        assert_eq!(names, ["full_name", "status"]);
        let found = declared.property("status").map(PropertyDecl::name);
        assert_eq!(
            (found, declared.property("absent").is_some()),
            (Some("status"), false)
        );
    }

    #[test]
    fn a_relationship_is_found_by_its_predicate() {
        let declared = person();
        let works_at = &declared.relationships()[0];
        assert_eq!(
            (works_at.predicate(), works_at.required()),
            ("works-at", false)
        );
        let found = declared.relationship("works-at").is_some();
        assert_eq!(
            (found, declared.relationship("absent").is_some()),
            (true, false)
        );
        assert_eq!(capture().relationships(), []);
    }

    #[test]
    fn a_type_answers_which_capabilities_it_declares() {
        let declared = capture();
        let carried = (
            declared.has(Capability::CatchAll),
            declared.has(Capability::IdentityBearing),
            declared.capabilities().len(),
        );
        assert_eq!(carried, (true, false, 2));
    }

    #[test]
    fn a_lifecycle_axis_reports_its_property_and_its_ordinary_state() {
        let axis = LifecycleDecl::Axis {
            axis: "stage".to_owned(),
            ordinary: Ordinary::Value("current".to_owned()),
        };
        let declared = (
            axis.declared(),
            axis.axis(),
            axis.ordinary().and_then(Ordinary::value),
        );
        assert_eq!(declared, ("axis", Some("stage"), Some("current")));
    }

    #[test]
    fn a_corpus_with_no_axis_says_so_rather_than_omitting_it() {
        let none = LifecycleDecl::None;
        let stated = (none.declared(), none.axis(), none.ordinary().is_some());
        assert_eq!(stated, ("none", None, false));
        assert!(Ordinary::Absent.value().is_none());
    }

    #[test]
    fn capabilities_round_trip_through_their_spelling() {
        for capability in Capability::ALL.iter().copied() {
            assert_eq!(Capability::named(capability.as_str()), Some(capability));
            assert_eq!(capability.to_string(), capability.as_str());
        }
        let closed = (Capability::ALL.len(), Capability::named("identity_bearing"));
        assert_eq!(closed, (3, None));
    }

    #[test]
    fn link_dialects_round_trip_through_their_spelling() {
        for dialect in LinkDialect::ALL.iter().copied() {
            assert_eq!(LinkDialect::named(dialect.as_str()), Some(dialect));
            assert_eq!(dialect.to_string(), dialect.as_str());
        }
        let closed = (LinkDialect::ALL.len(), LinkDialect::named("obsidian"));
        assert_eq!(closed, (2, None));
    }

    #[test]
    fn scalar_kinds_round_trip_through_their_spelling() {
        let names: Vec<&str> = ScalarKind::ALL
            .iter()
            .copied()
            .map(ScalarKind::as_str)
            .collect();
        assert_eq!(
            names,
            ["string", "integer", "float", "boolean", "date", "datetime"]
        );
        for kind in ScalarKind::ALL.iter().copied() {
            assert_eq!(ScalarKind::named(kind.as_str()), Some(kind));
            assert_eq!(kind.to_string(), kind.as_str());
        }
        assert!(ScalarKind::named("enum").is_none());
    }

    #[test]
    fn every_scalar_kind_has_a_property_kind() {
        let kinds: Vec<&str> = ScalarKind::ALL
            .iter()
            .copied()
            .map(|kind| PropertyKind::from(kind).as_str())
            .collect();
        assert_eq!(
            kinds,
            ["string", "integer", "float", "boolean", "date", "datetime"]
        );
    }

    #[test]
    fn the_two_carrying_kinds_report_what_they_carry() {
        let values = PropertyKind::Enum {
            values: vec!["draft".to_owned()],
        };
        assert_eq!(
            (values.as_str(), values.values()),
            ("enum", Some(&["draft".to_owned()][..]))
        );
        let list = PropertyKind::List {
            of: ScalarKind::Date,
        };
        assert_eq!(
            (list.as_str(), list.element()),
            ("list", Some(ScalarKind::Date))
        );
    }

    #[test]
    fn a_scalar_kind_carries_neither_values_nor_an_element() {
        let carried = (
            PropertyKind::Boolean.values(),
            PropertyKind::Boolean.element(),
        );
        assert_eq!(carried, (None, None));
    }

    #[test]
    fn a_kind_describes_itself_precisely_enough_to_tell_two_enums_apart() {
        let one = PropertyKind::Enum {
            values: vec!["draft".to_owned(), "archived".to_owned()],
        };
        let other = PropertyKind::Enum {
            values: vec!["draft".to_owned()],
        };
        assert_eq!(one.describe(), "`enum` over draft, archived");
        assert_ne!(one.describe(), other.describe());
        let list = PropertyKind::List {
            of: ScalarKind::Integer,
        };
        assert_eq!(list.describe(), "`list` of `integer`");
        assert_eq!(PropertyKind::Float.describe(), "`float`");
    }

    #[test]
    fn the_model_clones_compares_and_formats() {
        let contract = sample();
        assert_eq!(contract.clone(), contract);
        assert_ne!(contract.types()[0], contract.types()[1]);
        let rendered = format!("{contract:?}");
        assert!(rendered.contains("Wikilink"), "{rendered}");
        assert!(rendered.contains("Absent"), "{rendered}");
    }

    #[test]
    fn every_leaf_of_the_model_clones_compares_and_formats() {
        let property = person().properties[1].clone();
        assert_eq!(property.clone(), property);
        assert!(format!("{property:?}").contains("archived"));
        let relationship = person().relationships[0].clone();
        assert_eq!(relationship.clone(), relationship);
        assert!(format!("{relationship:?}").contains("works-at"));
        let capability = person().capabilities[0];
        assert_ne!(capability, Capability::CatchAll);
    }

    #[test]
    fn the_small_copy_types_clone_compare_and_format() {
        let dialect = Dialect {
            links: LinkDialect::Markdown,
        };
        assert_eq!(dialect.clone(), dialect);
        assert!(format!("{dialect:?}").contains("Markdown"));
        let flag = FlagDecl {
            property: "leaned_on".to_owned(),
        };
        assert_eq!(flag.clone(), flag);
        assert!(format!("{flag:?}").contains("leaned_on"));
        let kinds = (ScalarKind::Date, ScalarKind::DateTime);
        assert_ne!(kinds.0, kinds.1);
        assert!(format!("{kinds:?}").contains("DateTime"));
    }
}
