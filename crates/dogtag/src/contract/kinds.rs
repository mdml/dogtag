//! The closed lattice of value kinds, and what the two carrying kinds carry.
//!
//! A kind is what a declaration says a value *is*, and every kind's lexical
//! form is part of its meaning. There are **no value constraints** anywhere in
//! it — no pattern, no bounds, no format hint — because a declared constraint
//! the kernel never enforces misleads every agent that reads the contract.
//!
//! Two kinds carry something of their own, and both carry it here rather than
//! beside the kind on the declaration: an `enum` carries its members, and a
//! `record` carries its fields. That is not a convenience. One property name
//! declares one kind corpus-wide, so *what a kind carries is part of the kind*
//! — two types declaring the same name as records over different fields
//! disagree, exactly as two `enum` declarations over different values do, and
//! a shape held next to the kind instead could not say so.
//!
//! The record kind is **one level deep, and deliberately not recursive**. A
//! field's kind is drawn from this same lattice — read at full width, so `enum`
//! with its own `values` is included — and a field may be neither a `record`
//! nor a `list`. That bound is what keeps `contract explain`'s nested rendering
//! finite, and a second level is far easier to add later than to remove.
//!
//! Which kinds a contract may *write* is the declared version's answer rather
//! than this module's: `record`, and `record` as a `list`'s element, exist only
//! at contract version 2 and above, so a version-1 contract spelling either is
//! refused as a kind that version does not define and a version-1 model carries
//! no record anywhere.

use core::fmt;

/// What a property declares its values are.
///
/// `integer` and `float` are distinct on the wire: `1` is not a `float` and
/// `1.0` is not an `integer`.
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
    /// A record: a fixed set of named fields, one level deep.
    Record {
        /// The fields, in declaration order. Non-empty and uniquely named.
        fields: Vec<FieldDecl>,
    },
    /// A list of records, every element carrying the same fields.
    ///
    /// The contract spells it `kind = "list"` with `of = "record"`, and the
    /// fields are declared on the property itself — one place a field is
    /// declared, whichever of the two record shapes carries it.
    ListOfRecord {
        /// The fields every element carries, in declaration order.
        fields: Vec<FieldDecl>,
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
            Self::List { .. } | Self::ListOfRecord { .. } => "list",
            Self::Record { .. } => "record",
        }
    }

    /// The members of an `enum`.
    pub fn values(&self) -> Option<&[String]> {
        match self {
            Self::Enum { values } => Some(values),
            _ => None,
        }
    }

    /// The element kind of a `list` whose elements are scalars.
    ///
    /// A `list` of `record` answers `None` and carries its shape in
    /// [`PropertyKind::fields`] instead, because a record is not a member of
    /// the scalar lattice [`ScalarKind`] enumerates.
    pub fn element(&self) -> Option<ScalarKind> {
        match self {
            Self::List { of } => Some(*of),
            _ => None,
        }
    }

    /// The fields of a `record`, or of a `list` of `record`.
    pub fn fields(&self) -> Option<&[FieldDecl]> {
        match self {
            Self::Record { fields } | Self::ListOfRecord { fields } => Some(fields),
            _ => None,
        }
    }

    /// The kind spelled out with whatever it carries, for a diagnostic message
    /// that has to tell two `enum` or two `record` declarations apart.
    pub fn describe(&self) -> String {
        match self {
            Self::Enum { values } => format!("`enum` over {}", values.join(", ")),
            Self::List { of } => format!("`list` of `{of}`"),
            Self::Record { fields } => format!("`record` with {}", describe_fields(fields)),
            Self::ListOfRecord { fields } => {
                format!("`list` of `record` with {}", describe_fields(fields))
            }
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

/// The scalar lattice: the six kinds that carry nothing of their own.
///
/// It is what a `list` holds and what a record field draws from. A `list` may
/// also hold a record at a version that defines the record kind, which
/// [`PropertyKind::ListOfRecord`] carries rather than this enum, because a
/// record carries fields and a scalar kind carries nothing.
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

/// One field of a record.
///
/// Its name obeys the same never-empty, never-dotted rule as every other
/// declaration name, and under the same identifier: provenance addresses a
/// field by joining the names above it — `type.person.property.legal_name.
/// field.given.kind` — so a dotted field name would address another
/// declaration's key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDecl {
    pub(crate) name: String,
    pub(crate) kind: FieldKind,
    pub(crate) required: bool,
}

impl FieldDecl {
    /// The field's name, unique within its record.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The field's value kind.
    pub fn kind(&self) -> &FieldKind {
        &self.kind
    }

    /// Whether every record value must carry this field.
    pub fn required(&self) -> bool {
        self.required
    }

    /// The field named and spelled out, for a message that has to tell two
    /// records over the same field names apart.
    pub fn describe(&self) -> String {
        format!("{}: {}", self.name, self.kind.describe())
    }
}

/// What a record field may hold.
///
/// The scalar lattice, at full width: the six scalar kinds, and `enum` with the
/// closed set of members an `enum` needs to mean anything. `record` and `list`
/// are absent because a field may be neither — the one-level bound the record
/// kind is scoped to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldKind {
    /// One of the six scalar kinds.
    Scalar(ScalarKind),
    /// A closed set of named values.
    Enum {
        /// The members, in declaration order. Non-empty and free of repeats.
        values: Vec<String>,
    },
}

impl FieldKind {
    /// The spelling the contract writes for the kind itself.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Scalar(kind) => kind.as_str(),
            Self::Enum { .. } => "enum",
        }
    }

    /// The scalar kind, when the field declares one.
    pub fn scalar(&self) -> Option<ScalarKind> {
        match self {
            Self::Scalar(kind) => Some(*kind),
            Self::Enum { .. } => None,
        }
    }

    /// The members of an `enum`.
    pub fn values(&self) -> Option<&[String]> {
        match self {
            Self::Scalar(_) => None,
            Self::Enum { values } => Some(values),
        }
    }

    /// The kind spelled out with whatever it carries, for a message that has to
    /// tell two `enum` fields apart.
    pub fn describe(&self) -> String {
        match self {
            Self::Scalar(kind) => format!("`{kind}`"),
            Self::Enum { values } => format!("`enum` over {}", values.join(", ")),
        }
    }
}

impl fmt::Display for FieldKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Every field of one record, spelled out, so two records over the same names
/// are still told apart by what those names hold.
pub(crate) fn describe_fields(fields: &[FieldDecl]) -> String {
    let described: Vec<String> = fields.iter().map(FieldDecl::describe).collect();
    described.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields() -> Vec<FieldDecl> {
        vec![
            FieldDecl {
                name: "given".to_owned(),
                kind: FieldKind::Scalar(ScalarKind::String),
                required: true,
            },
            FieldDecl {
                name: "label".to_owned(),
                kind: FieldKind::Enum {
                    values: vec!["home".to_owned(), "work".to_owned()],
                },
                required: false,
            },
        ]
    }

    #[test]
    fn a_field_reports_its_name_its_kind_and_whether_it_is_required() {
        let declared = &fields()[0];
        assert_eq!(
            (
                declared.name(),
                declared.kind().as_str(),
                declared.required()
            ),
            ("given", "string", true)
        );
        assert_eq!(declared.kind().scalar(), Some(ScalarKind::String));
        assert!(declared.kind().values().is_none());
    }

    #[test]
    fn a_field_may_declare_an_enum_with_its_own_members() {
        let declared = &fields()[1];
        assert_eq!(
            (declared.kind().as_str(), declared.required()),
            ("enum", false)
        );
        assert_eq!(
            declared.kind().values(),
            Some(&["home".to_owned(), "work".to_owned()][..])
        );
        assert!(declared.kind().scalar().is_none());
    }

    #[test]
    fn a_field_describes_itself_precisely_enough_to_tell_two_records_apart() {
        assert_eq!(fields()[0].describe(), "given: `string`");
        assert_eq!(fields()[1].describe(), "label: `enum` over home, work");
        assert_eq!(
            describe_fields(&fields()),
            "given: `string`, label: `enum` over home, work"
        );
        assert_eq!(describe_fields(&[]), "");
    }

    #[test]
    fn a_field_kind_renders_as_the_spelling_the_contract_writes() {
        for kind in ScalarKind::ALL.iter().copied() {
            assert_eq!(FieldKind::Scalar(kind).to_string(), kind.as_str());
        }
        assert_eq!(
            FieldKind::Enum {
                values: vec!["one".to_owned()]
            }
            .to_string(),
            "enum"
        );
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
        assert!(PropertyKind::Boolean.fields().is_none());
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
    fn the_two_record_shapes_carry_the_same_fields_and_spell_themselves_apart() {
        // Which is why the fields live inside the kind: a `list` of `record`
        // and a bare `record` over the same fields are different kinds, and one
        // property name declares one kind corpus-wide.
        let bare = PropertyKind::Record { fields: fields() };
        let listed = PropertyKind::ListOfRecord { fields: fields() };
        let spelled = (bare.as_str(), listed.as_str(), listed.element());
        assert_eq!(spelled, ("record", "list", None));
        assert_eq!(listed.fields(), bare.fields());
        assert_ne!(bare, listed);
    }

    #[test]
    fn a_record_describes_itself_precisely_enough_to_tell_two_of_them_apart() {
        let described = (
            PropertyKind::Record { fields: fields() }.describe(),
            PropertyKind::ListOfRecord { fields: fields() }.describe(),
        );
        let expected = "`record` with given: `string`, label: `enum` over home, work";
        assert_eq!(
            described,
            (expected.to_owned(), format!("`list` of {expected}"))
        );
    }

    #[test]
    fn the_field_declarations_clone_compare_and_format() {
        let declared = fields();
        assert_eq!(declared[0].clone(), declared[0]);
        assert_ne!(declared[0], declared[1]);
        assert!(format!("{:?}", declared[1]).contains("home"));
        assert_eq!(declared[0].kind().clone(), *declared[0].kind());
    }
}
