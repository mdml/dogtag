//! Binding a note to its type, and reading it against that type's declarations.
//!
//! # The discriminator, and what the catch-all binds
//!
//! `type` is the format's **one reserved frontmatter word**, and its value must
//! be a scalar naming a declared type. A note with no frontmatter, or
//! frontmatter without a `type` key, is a member of the catch-all type —
//! derived from the capability declaration alone, with no new contract key, and
//! identically for a version-1 contract.
//!
//! **The catch-all binds absence, never error.** A `type` that is not a scalar,
//! or that names no declared type, binds to nothing: a note that says what it
//! is and is wrong is reported as wrong rather than silently reclassified.
//!
//! One consequence the record states the binding for and not the sequel, so it
//! is decided here and stated where it binds: a note that bound to no type
//! collects **no further `note.*` finding**. Every remaining rule reads a
//! declaration, and there is no declaration to read — the same reason the
//! record already gives for not validating the value of an undeclared key. The
//! contract side sets the pattern, where a version classification precedes and
//! suppresses structural validation.
//!
//! # Undeclared keys are `info`, and the tag property needs no exemption
//!
//! A key that is neither a declared property, a declared predicate, nor the
//! discriminator is reported at `info`, per key, against its span. The record
//! also exempts "the declared tag property", and that clause turns out to need
//! no code: `[tags]` **names a declared property**, so on a type that declares
//! it the key is an ordinary declared property and on a type that does not it
//! is an ordinary undeclared key. Reading it the other way — corpus-wide — would
//! let a value reach `show` as tags without ever having been validated against
//! a declaration, which is the premise the whole format rests on.

use crate::contract::{Contract, PropertyDecl, RelationshipDecl, TagNamespaceDecl, TypeDecl};
use crate::diagnostic::{KernelDiagnostic, Related};

use super::findings::Findings;
use super::frontmatter::{Entry, Shape, Value};
use super::lexical;
use super::model::{Binding, Edge, Property, Relationship};
use super::values;

/// The format's one reserved frontmatter word.
const DISCRIMINATOR: &str = "type";

/// What a note bound to, and the declarations that answer for it.
pub(crate) struct Bound<'a> {
    /// The binding the document model carries.
    pub(crate) binding: Binding,
    /// The type's declarations, when the note bound to a type.
    pub(crate) declared: Option<&'a TypeDecl>,
}

/// Everything a note's frontmatter says, once held to its type.
pub(crate) struct Contents {
    /// The declared properties the note carries, in declaration order.
    pub(crate) properties: Vec<Property>,
    /// The declared relationships the note carries, in declaration order.
    pub(crate) relationships: Vec<Relationship>,
    /// The declared tag property's values.
    pub(crate) tags: Vec<String>,
}

/// Binds a note to the type its discriminator names, or to the catch-all.
pub(crate) fn bind<'a>(
    findings: &mut Findings<'_>,
    contract: &'a Contract,
    entries: &[Entry],
) -> Bound<'a> {
    let Some(entry) = entries.iter().find(|entry| entry.key == DISCRIMINATOR) else {
        return catch_all(contract);
    };
    let Some(name) = entry.value.scalar() else {
        findings.spanned(
            KernelDiagnostic::NoteTypeInvalid,
            format!(
                "`{DISCRIMINATOR}` must name a declared type, and the note writes {}",
                entry.value.describe()
            ),
            entry.value.span.clone(),
        );
        return unbound(None);
    };
    match contract.type_named(name) {
        Some(declared) => Bound {
            binding: Binding::Declared {
                name: name.to_owned(),
            },
            declared: Some(declared),
        },
        None => {
            findings.spanned(
                KernelDiagnostic::NoteUnknownType,
                format!("the contract declares no type `{name}`"),
                entry.value.span.clone(),
            );
            unbound(Some(name.to_owned()))
        }
    }
}

/// The binding a note with no discriminator takes.
///
/// A resolved contract always declares exactly one catch-all, so the `None` arm
/// answers only for a contract a caller assembled some other way — and it
/// answers by binding to nothing rather than by choosing a type.
fn catch_all(contract: &Contract) -> Bound<'_> {
    match contract.catch_all() {
        Some(declared) => Bound {
            binding: Binding::CatchAll {
                name: declared.name().to_owned(),
            },
            declared: Some(declared),
        },
        None => unbound(None),
    }
}

fn unbound<'a>(named: Option<String>) -> Bound<'a> {
    Bound {
        binding: Binding::Unbound { named },
        declared: None,
    }
}

/// Reads a note's frontmatter against the type it bound to.
pub(crate) fn contents(
    findings: &mut Findings<'_>,
    contract: &Contract,
    declared: &TypeDecl,
    entries: &[Entry],
) -> Contents {
    let properties = properties(findings, contract, declared, entries);
    let relationships = relationships(findings, contract, declared, entries);
    undeclared(findings, declared, entries);
    // A type declaring a namespace declares the tag property too — the contract
    // refuses one without the other — so the key is looked up by name with no
    // second check that the type admits it.
    let tagged = contract
        .tags()
        .and_then(|tags| written(entries, tags.property()));
    namespaces(findings, declared, tagged);
    Contents {
        tags: tags(contract, &properties),
        properties,
        relationships,
    }
}

/// Every declared property, in declaration order, with what the note wrote.
fn properties(
    findings: &mut Findings<'_>,
    contract: &Contract,
    declared: &TypeDecl,
    entries: &[Entry],
) -> Vec<Property> {
    let mut carried = Vec::new();
    for property in declared.properties() {
        match written(entries, property.name()) {
            Some(value) => {
                let read = values::property(findings, property.name(), property.kind(), value);
                carried.extend(read.map(|value| Property {
                    name: property.name().to_owned(),
                    value,
                }));
            }
            None if property.required() => {
                missing_property(findings, contract, declared, property);
            }
            None => {}
        }
    }
    carried
}

/// Every declared relationship, in declaration order, with the edges it carries.
fn relationships(
    findings: &mut Findings<'_>,
    contract: &Contract,
    declared: &TypeDecl,
    entries: &[Entry],
) -> Vec<Relationship> {
    let mut carried = Vec::new();
    for relationship in declared.relationships() {
        let predicate = relationship.predicate();
        let edges = match written(entries, predicate) {
            Some(value) => edges(findings, predicate, value),
            None => Vec::new(),
        };
        if edges.is_empty() && relationship.required() {
            missing_relationship(findings, contract, declared, relationship);
        }
        carried.push(Relationship {
            predicate: predicate.to_owned(),
            edges,
        });
    }
    carried
}

/// The references a note wrote under one predicate.
///
/// A reference is carried exactly as the note wrote it, delimiters included:
/// which note it names is a question about the corpus, answered where the whole
/// corpus is in hand. Its **location** is carried with it for the same reason —
/// by the time the corpus is in hand this note's text is gone, and a `link.*`
/// finding is addressed to the reference itself. An empty scalar is *no* edge
/// rather than an empty one: a key with nothing after it claims no
/// relationship.
fn edges(findings: &mut Findings<'_>, predicate: &str, value: &Value) -> Vec<Edge> {
    match &value.shape {
        Shape::Scalar(text) if text.is_empty() => Vec::new(),
        Shape::Scalar(text) => vec![edge(findings, text, value)],
        Shape::Sequence(items) => sequence(findings, predicate, value, items),
        Shape::Mapping(_) => {
            relationship_invalid(findings, predicate, value);
            Vec::new()
        }
    }
}

/// A sequence of links, which is every item or none: one item that is not a
/// link makes the value not a sequence of links.
fn sequence(
    findings: &mut Findings<'_>,
    predicate: &str,
    value: &Value,
    items: &[Value],
) -> Vec<Edge> {
    let read: Option<Vec<Edge>> = items
        .iter()
        .map(|item| Some(edge(findings, item.scalar()?, item)))
        .collect();
    match read {
        Some(edges) => edges,
        None => {
            relationship_invalid(findings, predicate, value);
            Vec::new()
        }
    }
}

fn edge(findings: &Findings<'_>, written: &str, value: &Value) -> Edge {
    Edge {
        written: written.to_owned(),
        target: None,
        at: findings.at(value.span.clone()),
    }
}

/// Every key the bound type declares nothing for, reported at `info`.
fn undeclared(findings: &mut Findings<'_>, declared: &TypeDecl, entries: &[Entry]) {
    for entry in entries {
        if declares(declared, &entry.key) {
            continue;
        }
        findings.spanned(
            KernelDiagnostic::NoteUndeclaredProperty,
            format!(
                "the type `{}` declares no `{}`, so its value is not validated",
                declared.name(),
                entry.key
            ),
            entry.key_span.clone(),
        );
    }
}

/// Holds a note's tags to the namespaces its type declares.
///
/// **Namespaces are evaluated independently, and a tag matching no declared
/// namespace is untouched at any severity.** Tags are content; what a namespace
/// describes is the part of a corpus's tagging the corpus chose to schematize,
/// never a licence to enumerate the rest.
///
/// A prefix is matched against the start of a tag and the tag is never split:
/// the kernel owns no separator convention, so the prefix carries whatever
/// separator the corpus writes and a member names the remainder after it.
fn namespaces(findings: &mut Findings<'_>, declared: &TypeDecl, tagged: Option<&Value>) {
    let items: &[Value] = tagged.and_then(Value::sequence).unwrap_or_default();
    for namespace in declared.tag_namespaces() {
        let matching: Vec<(&str, &Value)> = items
            .iter()
            .filter_map(|item| Some((item.scalar()?.strip_prefix(namespace.prefix())?, item)))
            .collect();
        if namespace.required() && matching.is_empty() {
            required_namespace(findings, namespace, tagged);
        }
        bounded(findings, namespace, &matching);
    }
}

/// A closed namespace admits only the members it declares.
fn bounded(findings: &mut Findings<'_>, namespace: &TagNamespaceDecl, matching: &[(&str, &Value)]) {
    let Some(values) = namespace.values() else {
        return;
    };
    for (remainder, tag) in matching {
        if !lexical::member(values, remainder) {
            outside(findings, namespace.prefix(), remainder, tag);
        }
    }
}

fn outside(findings: &mut Findings<'_>, prefix: &str, remainder: &str, tag: &Value) {
    findings.spanned(
        KernelDiagnostic::NoteTagOutsideVocabulary,
        format!("the namespace `{prefix}` does not declare the tag `{prefix}{remainder}`"),
        tag.span.clone(),
    );
}

/// A required namespace with no tag in it.
///
/// It points at the tags the note wrote when it wrote any, because that is
/// where the repair goes; a note that wrote none has no bytes to point at and
/// the finding is against the note itself.
fn required_namespace(
    findings: &mut Findings<'_>,
    namespace: &TagNamespaceDecl,
    tagged: Option<&Value>,
) {
    let prefix = namespace.prefix();
    let message =
        format!("the type requires a tag beginning `{prefix}`, and the note carries none");
    match tagged {
        Some(value) => findings.spanned(
            KernelDiagnostic::NoteRequiredNamespaceMissing,
            message,
            value.span.clone(),
        ),
        None => findings.absent(
            KernelDiagnostic::NoteRequiredNamespaceMissing,
            message,
            Related::new(format!("the namespace `{prefix}` is declared required")),
        ),
    }
}

/// Whether the bound type says anything at all about `key`.
fn declares(declared: &TypeDecl, key: &str) -> bool {
    key == DISCRIMINATOR || declared.property(key).is_some() || declared.relationship(key).is_some()
}

/// The note's tags: the declared tag property's values, when it declares one.
///
/// Only a list of values is tags. A corpus may name a tag property whose kind
/// is something else — the contract only refuses that where a type also
/// declares a namespace — and a scalar is not a set of tags however it is spelt.
fn tags(contract: &Contract, properties: &[Property]) -> Vec<String> {
    let Some(tags) = contract.tags() else {
        return Vec::new();
    };
    properties
        .iter()
        .find(|property| property.name == tags.property())
        .and_then(|property| property.value.list())
        .map(<[String]>::to_vec)
        .unwrap_or_default()
}

/// The value a note wrote under `key`, when it wrote one.
fn written<'a>(entries: &'a [Entry], key: &str) -> Option<&'a Value> {
    entries
        .iter()
        .find(|entry| entry.key == key)
        .map(|entry| &entry.value)
}

fn missing_property(
    findings: &mut Findings<'_>,
    contract: &Contract,
    declared: &TypeDecl,
    property: &PropertyDecl,
) {
    let name = property.name();
    findings.absent(
        KernelDiagnostic::NoteMissingRequiredProperty,
        format!(
            "the type `{}` requires `{name}`, and the note writes no value for it",
            declared.name()
        ),
        declaration(
            contract,
            &format!("type.{}.property.{name}.required", declared.name()),
            format!(
                "`{name}` is declared {} and required",
                property.kind().describe()
            ),
        ),
    );
}

fn missing_relationship(
    findings: &mut Findings<'_>,
    contract: &Contract,
    declared: &TypeDecl,
    relationship: &RelationshipDecl,
) {
    let predicate = relationship.predicate();
    findings.absent(
        KernelDiagnostic::NoteMissingRequiredRelationship,
        format!(
            "the type `{}` requires at least one `{predicate}`, and the note writes none",
            declared.name()
        ),
        declaration(
            contract,
            &format!("type.{}.relationship.{predicate}.required", declared.name()),
            format!("`{predicate}` is declared required"),
        ),
    );
}

fn relationship_invalid(findings: &mut Findings<'_>, predicate: &str, value: &Value) {
    findings.spanned(
        KernelDiagnostic::NoteRelationshipValueInvalid,
        format!(
            "`{predicate}` is a relationship, so its value is a link or a sequence of links, \
             and the note writes {}",
            value.describe()
        ),
        value.span.clone(),
    );
}

/// Evidence pointing at where the contract writes the requirement.
///
/// A requirement is always written rather than defaulted — `required` defaults
/// to `false` — so the provenance the contract already recorded is the location,
/// and evidence with no location is what a contract assembled some other way
/// gets.
fn declaration(contract: &Contract, key: &str, message: String) -> Related {
    let mut related = Related::new(message);
    related.location = contract
        .provenance()
        .get(key)
        .and_then(|entry| entry.location.clone());
    related
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{Dialect, LifecycleDecl, LinkDialect};
    use crate::diagnostic::VaultPath;
    use crate::encoding::inspect;
    use crate::provenance::Provenance;

    /// A contract no loader produces.
    ///
    /// Catch-all cardinality is enforced when a contract loads, so a corpus
    /// with no bottom type can only reach here from a caller that assembled the
    /// contract some other way. The binding answers by binding to nothing,
    /// rather than by choosing a type the contract did not nominate.
    fn without_a_catch_all() -> Contract {
        Contract {
            contract_version: 2,
            dialect: Dialect {
                links: LinkDialect::Wikilink,
            },
            lifecycle: LifecycleDecl::None,
            tags: None,
            flags: Vec::new(),
            types: Vec::new(),
            provenance: Provenance::new(),
        }
    }

    #[test]
    fn a_note_with_no_discriminator_binds_to_nothing_where_no_type_is_the_bottom() {
        let text = inspect(b"---\n---\n").expect("well-formed");
        let mut findings = Findings::new(&VaultPath::kernel("inbox.md"), &text);
        let contract = without_a_catch_all();
        let bound = bind(&mut findings, &contract, &[]);
        assert_eq!(bound.binding, Binding::Unbound { named: None });
        assert!(bound.declared.is_none());
        assert!(findings.finish().is_empty());
    }
}
