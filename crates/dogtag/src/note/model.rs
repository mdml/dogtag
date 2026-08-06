//! What a note is, once it has been read against a contract.
//!
//! This is the **one shared shape** every surface answers with: identity, the
//! type and how it bound, properties as declared-kind values, relationships as
//! edges, tags, body, and title. A surface renders it; none of them grows a
//! second notion of what a note looks like.
//!
//! One thing rides alongside that shape rather than inside it: the untyped
//! references a note's body writes. No M3 surface renders them — `show` answers
//! the typed shape above — but the record has them parsed and resolved at this
//! milestone for the milestones that want them, so they are carried and reached
//! through one narrow accessor.
//!
//! Two things it deliberately is not. It is **not** a copy of the contract: a
//! property is here because the note wrote it, and what its kind *is* stays the
//! contract's answer. And a value is **not** coerced — every scalar is the
//! bytes the note wrote, because the declared kind decides what those bytes
//! mean and a parsed value would have decided it already.

use crate::diagnostic::{Location, VaultPath};

/// The declared property whose values are a note's aliases.
///
/// A convention on the declaration rather than a reserved word: retrieval
/// reads the property a type declares under this name, and a corpus whose
/// vocabulary wants `aliases` to mean something else simply is not matched by
/// alias. The values surface only where the bound type declares the property
/// — the tag-property precedent — so an undeclared `aliases` key stays an
/// ordinary undeclared key whose values never reach the model.
const ALIAS_PROPERTY: &str = "aliases";

/// One note, read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub(crate) path: VaultPath,
    pub(crate) binding: Binding,
    pub(crate) properties: Vec<Property>,
    pub(crate) relationships: Vec<Relationship>,
    pub(crate) references: Vec<Reference>,
    pub(crate) tags: Vec<String>,
    pub(crate) title: Option<String>,
    pub(crate) body: String,
}

impl Note {
    /// The note's identity: its vault-relative path.
    ///
    /// Identity is the path and nothing else — not the name, not the title, not
    /// a key in the frontmatter.
    pub fn path(&self) -> &VaultPath {
        &self.path
    }

    /// The note's name: its file name without the `.md` extension.
    ///
    /// A name is a **resolution shorthand**, never an identity: two notes may
    /// legitimately share one, and a reference that meets both is what carries
    /// the defect.
    pub fn name(&self) -> &str {
        let file = match self.path.as_str().rsplit_once('/') {
            Some((_, file)) => file,
            None => self.path.as_str(),
        };
        file.strip_suffix(".md").unwrap_or(file)
    }

    /// The type the note bound to, and how it bound.
    pub fn binding(&self) -> &Binding {
        &self.binding
    }

    /// The declared properties the note carries, in declaration order.
    pub fn properties(&self) -> &[Property] {
        &self.properties
    }

    /// The note's value for a declared property.
    pub fn property(&self, name: &str) -> Option<&PropertyValue> {
        self.properties
            .iter()
            .find(|property| property.name == name)
            .map(|property| &property.value)
    }

    /// The declared relationships the note carries, in declaration order.
    pub fn relationships(&self) -> &[Relationship] {
        &self.relationships
    }

    /// The note's edges under a declared predicate.
    pub fn relationship(&self, predicate: &str) -> Option<&[Edge]> {
        self.relationships
            .iter()
            .find(|relationship| relationship.predicate == predicate)
            .map(|relationship| relationship.edges.as_slice())
    }

    /// The untyped references the note's body writes, in the order it writes
    /// them.
    ///
    /// A prose reference is **not an edge**: it answers "connected", never
    /// "how", and a reference whose target does not exist yet legitimately
    /// belongs in prose until it does. So one that resolves to nothing is a
    /// finding at no severity and simply carries no target.
    ///
    /// No M3 surface renders these — `show`'s shape is the typed one — and the
    /// accessor is deliberately the narrowest thing that lets the milestones
    /// that want them (an index, backlinks) read what this one resolved.
    pub fn body_references(&self) -> &[Reference] {
        &self.references
    }

    /// The note's tags: the declared tag property's values, in the order the
    /// note wrote them.
    ///
    /// Empty where the contract declares no tag property, where the bound type
    /// does not declare it, and where the note simply carries none. Tags are
    /// content, and a corpus is never asked to enumerate them.
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// The note's aliases: its bound type's declared `aliases` property
    /// values — one alias for a scalar, one per element for a list.
    ///
    /// Crate-internal: the retrieval surfaces match these beside the note's
    /// name, and no public surface renders them yet.
    pub(crate) fn aliases(&self) -> Vec<&str> {
        match self.property(ALIAS_PROPERTY) {
            Some(PropertyValue::Scalar(value)) => vec![value.as_str()],
            Some(PropertyValue::List(values)) => values.iter().map(String::as_str).collect(),
            _ => Vec::new(),
        }
    }

    /// The note's display title: its first H1, when it has one.
    ///
    /// Display metadata, never identity — a title is free to change without
    /// breaking a single link.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// The note's body, as uninterpreted text.
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// Which type a note bound to, and what bound it.
///
/// The catch-all binds **absence**, never error: a note that says what it is
/// and is wrong is reported as wrong rather than silently reclassified.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Binding {
    /// The note's `type` names a type the contract declares.
    Declared {
        /// The declared type's name.
        name: String,
    },
    /// The note carries no `type` key, so it belongs to the catch-all type.
    CatchAll {
        /// The catch-all type's name.
        name: String,
    },
    /// The note bound to no type at all, and no type-directed rule was applied
    /// to it: there is no declaration to apply.
    Unbound {
        /// What the note's `type` said, when it said something readable.
        named: Option<String>,
    },
}

impl Binding {
    /// The type's name, when the note bound to one.
    pub fn type_name(&self) -> Option<&str> {
        match self {
            Self::Declared { name } | Self::CatchAll { name } => Some(name),
            Self::Unbound { .. } => None,
        }
    }

    /// What the note's discriminator said, whether or not it named a type.
    pub fn discriminator(&self) -> Option<&str> {
        match self {
            Self::Declared { name } => Some(name),
            Self::CatchAll { .. } => None,
            Self::Unbound { named } => named.as_deref(),
        }
    }

    /// What bound the note, for structured output: `declaration`, `catch-all`,
    /// or `none`.
    pub fn bound_by(&self) -> &'static str {
        match self {
            Self::Declared { .. } => "declaration",
            Self::CatchAll { .. } => "catch-all",
            Self::Unbound { .. } => "none",
        }
    }
}

/// One declared property, with the value the note wrote for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Property {
    pub(crate) name: String,
    pub(crate) value: PropertyValue,
}

impl Property {
    /// The property's declared name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The value the note wrote.
    pub fn value(&self) -> &PropertyValue {
        &self.value
    }
}

/// A property's value, shaped by the kind its declaration names.
///
/// Every leaf is the bytes the note wrote. Nothing is parsed into a number, a
/// date, or a boolean, because the kind is what says a value *is* one and the
/// SDK's answer must be the corpus's own bytes rather than a round trip through
/// some other type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropertyValue {
    /// One scalar's bytes.
    Scalar(String),
    /// A list's elements, in the order the note wrote them.
    List(Vec<String>),
    /// A record's fields.
    Record(RecordValue),
    /// A list of records, in the order the note wrote them.
    RecordList(Vec<RecordValue>),
}

impl PropertyValue {
    /// The scalar's bytes, when the value is one scalar.
    pub fn scalar(&self) -> Option<&str> {
        match self {
            Self::Scalar(text) => Some(text),
            _ => None,
        }
    }

    /// A list's elements, when the value is a list of scalars.
    pub fn list(&self) -> Option<&[String]> {
        match self {
            Self::List(items) => Some(items),
            _ => None,
        }
    }

    /// The record values the property carries: one for a `record`, and every
    /// element for a `list` of `record`.
    pub fn records(&self) -> &[RecordValue] {
        match self {
            Self::Record(record) => core::slice::from_ref(record),
            Self::RecordList(records) => records,
            _ => &[],
        }
    }
}

/// One record value: the declared fields the note wrote, in declaration order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordValue {
    pub(crate) fields: Vec<FieldValue>,
}

impl RecordValue {
    /// The fields the note wrote, in the declaring order.
    pub fn fields(&self) -> &[FieldValue] {
        &self.fields
    }

    /// The note's value for a declared field.
    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| field.value.as_str())
    }
}

/// One declared field of a record value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldValue {
    pub(crate) name: String,
    pub(crate) value: String,
}

impl FieldValue {
    /// The field's declared name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The bytes the note wrote.
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// One declared relationship, with the edges the note wrote under it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Relationship {
    pub(crate) predicate: String,
    pub(crate) edges: Vec<Edge>,
}

impl Relationship {
    /// The predicate the type declares.
    pub fn predicate(&self) -> &str {
        &self.predicate
    }

    /// The edges the note wrote, in order.
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
}

/// One typed link: the reference the note wrote, and the note it resolves to.
///
/// A typed link **must** resolve — an edge with a dangling endpoint is not a
/// relationship, it is a string — so a target of `None` is always accompanied
/// by a `link.*` error against the reference itself. *Which* note a reference
/// names is a question about the corpus rather than about this note, so it is
/// answered where the whole corpus is in hand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    pub(crate) written: String,
    pub(crate) target: Option<VaultPath>,
    /// Where the reference is written.
    ///
    /// Carried on the edge because a `link.*` finding is addressed to the
    /// reference itself, and by the time the whole corpus is in hand the note's
    /// text — the only thing that turns a byte range into a line and a column —
    /// has been dropped.
    pub(crate) at: Location,
}

impl Edge {
    /// The reference exactly as the note wrote it, delimiters included.
    pub fn written(&self) -> &str {
        &self.written
    }

    /// The note the reference resolves to.
    ///
    /// `None` where the reference resolved to nothing, or where a bare name is
    /// one several notes bear — both of which are reported.
    pub fn target(&self) -> Option<&VaultPath> {
        self.target.as_ref()
    }
}

/// One untyped reference, written in a note's body.
///
/// The prose-level reference, which the metadata plane's typed links sit beside
/// rather than replace. It carries no predicate, so it makes no claim about
/// *how* two notes relate, and **a dangling untyped reference is a finding at
/// no severity**: a reference to a note that does not exist yet belongs in
/// prose until it does, and danglingness is only a defect where a relationship
/// was claimed.
///
/// Danglingness is the *only* thing prose is excused from. **Ambiguity is a
/// defect of the reference itself** — its candidates all exist and it names
/// none of them — so a bare name several notes bear is reported here exactly as
/// it is on a typed link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reference {
    pub(crate) written: String,
    pub(crate) target: Option<VaultPath>,
    /// Where the reference is written, for the one finding prose can carry.
    ///
    /// Held for the same reason [`Edge::at`] is: resolution runs once the whole
    /// corpus is in hand, by which time the note's text — the only thing that
    /// turns a byte range into a line and a column — has been dropped.
    pub(crate) at: Location,
}

impl Reference {
    /// The reference exactly as the note wrote it, delimiters included.
    pub fn written(&self) -> &str {
        &self.written
    }

    /// The note the reference resolves to, when it resolves to exactly one.
    ///
    /// `None` where the reference names nothing — which is no finding — and
    /// where a bare name is one several notes bear, which is reported.
    pub fn target(&self) -> Option<&VaultPath> {
        self.target.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note() -> Note {
        Note {
            path: VaultPath::kernel("people/ada.md"),
            binding: Binding::Declared {
                name: "person".to_owned(),
            },
            properties: vec![
                Property {
                    name: "full_name".to_owned(),
                    value: PropertyValue::Scalar("Ada Lovelace".to_owned()),
                },
                Property {
                    name: "labels".to_owned(),
                    value: PropertyValue::List(vec!["role/founder".to_owned()]),
                },
            ],
            relationships: vec![Relationship {
                predicate: "works-at".to_owned(),
                edges: vec![Edge {
                    written: "[[Analytical Engine]]".to_owned(),
                    target: Some(VaultPath::kernel("engines/analytical.md")),
                    at: at(),
                }],
            }],
            references: vec![Reference {
                written: "[[Charles Babbage]]".to_owned(),
                target: None,
                at: at(),
            }],
            tags: vec!["role/founder".to_owned()],
            title: Some("Ada Lovelace".to_owned()),
            body: "# Ada Lovelace\n".to_owned(),
        }
    }

    /// Somewhere in the note, for an edge that has to carry a location.
    fn at() -> Location {
        Location::whole_file(crate::diagnostic::FileRef::InVault(VaultPath::kernel(
            "people/ada.md",
        )))
    }

    fn record() -> RecordValue {
        RecordValue {
            fields: vec![FieldValue {
                name: "given".to_owned(),
                value: "Ada".to_owned(),
            }],
        }
    }

    #[test]
    fn a_note_answers_with_its_identity_and_never_with_its_name() {
        let note = note();
        let identity = (note.path().as_str(), note.name(), note.title());
        assert_eq!(identity, ("people/ada.md", "ada", Some("Ada Lovelace")));
        assert_eq!(note.body(), "# Ada Lovelace\n");
    }

    #[test]
    fn a_name_is_the_file_name_without_its_extension_wherever_the_note_sits() {
        let root = Note {
            path: VaultPath::kernel("inbox.md"),
            ..note()
        };
        let odd = Note {
            path: VaultPath::kernel("a/b/no-extension"),
            ..note()
        };
        assert_eq!((root.name(), odd.name()), ("inbox", "no-extension"));
    }

    #[test]
    fn a_note_answers_for_the_properties_it_carries_and_only_those() {
        let note = note();
        let names: Vec<&str> = note.properties().iter().map(Property::name).collect();
        assert_eq!(names, ["full_name", "labels"]);
        let full_name = note.property("full_name").and_then(PropertyValue::scalar);
        let found = (full_name, note.property("absent").is_some());
        assert_eq!(found, (Some("Ada Lovelace"), false));
        assert_eq!(note.properties()[0].value().scalar(), Some("Ada Lovelace"));
    }

    #[test]
    fn a_note_answers_for_its_edges_and_the_notes_they_resolved_to() {
        let note = note();
        let edges = note.relationship("works-at").expect("declared");
        assert_eq!(edges[0].written(), "[[Analytical Engine]]");
        assert_eq!(
            edges[0].target().map(VaultPath::as_str),
            Some("engines/analytical.md")
        );
        let declared = &note.relationships()[0];
        let carried = (
            declared.predicate(),
            declared.edges().len(),
            note.relationship("absent").is_some(),
        );
        assert_eq!(carried, ("works-at", 1, false));
    }

    #[test]
    fn a_note_answers_for_the_untyped_references_its_body_writes() {
        // A prose reference to a note that does not exist yet is not a defect,
        // so a reference simply carries no target and nothing is reported.
        let note = note();
        let reference = &note.body_references()[0];
        assert_eq!(
            (reference.written(), reference.target()),
            ("[[Charles Babbage]]", None)
        );
    }

    #[test]
    fn a_binding_says_which_type_and_what_bound_it() {
        let declared = Binding::Declared {
            name: "person".to_owned(),
        };
        let catch_all = Binding::CatchAll {
            name: "capture".to_owned(),
        };
        let unbound = Binding::Unbound {
            named: Some("persno".to_owned()),
        };
        let named = (
            declared.type_name(),
            catch_all.type_name(),
            unbound.type_name(),
        );
        assert_eq!(named, (Some("person"), Some("capture"), None));
        // The catch-all binds absence, so there is nothing it was told to be.
        let said = (
            declared.discriminator(),
            catch_all.discriminator(),
            unbound.discriminator(),
            Binding::Unbound { named: None }.discriminator(),
        );
        assert_eq!(said, (Some("person"), None, Some("persno"), None));
        let bound_by = [
            declared.bound_by(),
            catch_all.bound_by(),
            unbound.bound_by(),
        ];
        assert_eq!(bound_by, ["declaration", "catch-all", "none"]);
    }

    #[test]
    fn a_value_answers_only_for_the_shape_its_kind_gave_it() {
        let scalar = PropertyValue::Scalar("one".to_owned());
        let list = PropertyValue::List(vec!["one".to_owned()]);
        let single = PropertyValue::Record(record());
        let many = PropertyValue::RecordList(vec![record(), record()]);
        let scalars = (
            scalar.scalar(),
            list.scalar(),
            single.scalar(),
            many.scalar(),
        );
        assert_eq!(scalars, (Some("one"), None, None, None));
        let listed = (scalar.list(), list.list(), many.list());
        assert_eq!(listed, (None, Some(&["one".to_owned()][..]), None));
        let records = (
            scalar.records().len(),
            single.records().len(),
            many.records().len(),
        );
        assert_eq!(records, (0, 1, 2));
    }

    #[test]
    fn a_record_value_answers_for_the_fields_the_note_wrote() {
        let record = record();
        let found = (record.field("given"), record.field("family"));
        assert_eq!(found, (Some("Ada"), None));
        let first = &record.fields()[0];
        assert_eq!((first.name(), first.value()), ("given", "Ada"));
    }

    #[test]
    fn the_document_model_clones_compares_and_formats() {
        let untagged = Note {
            tags: Vec::new(),
            ..note()
        };
        let note = note();
        assert_eq!(note.clone(), note);
        assert_ne!(note, untagged);
        let rendered = format!("{note:?}");
        assert_eq!(note.tags(), ["role/founder"]);
        assert!(rendered.contains("Lovelace"));
        assert!(format!("{:?}", record()).contains("given"));
        assert!(format!("{:?}", Binding::Unbound { named: None }).contains("Unbound"));
        let reference = &note.body_references()[0];
        assert_eq!(reference.clone(), *reference);
        assert!(format!("{reference:?}").contains("Babbage"));
    }
}
