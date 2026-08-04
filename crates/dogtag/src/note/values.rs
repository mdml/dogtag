//! Holding a frontmatter value to the kind its declaration names.
//!
//! Two faults come out of here and they are different questions. A value's
//! **shape** can be wrong — a mapping where a scalar was declared — and then
//! there is no value to carry, so the property is absent from the model with
//! the fault reported against it. A value's **lexical form** can be wrong —
//! `2026-8-3` for a `date` — and then the bytes are carried anyway: the
//! document model says what the note wrote, and the diagnostics say what is
//! wrong with it. A model that dropped every value it disagreed with would make
//! `show` unable to display the note a reader is trying to repair.
//!
//! A record's fields validate **exactly as properties do**, reusing the same
//! identifiers with the field path in the message: a record is a property whose
//! value has parts, not a second schema language.

use crate::contract::{Contract, FieldDecl, FieldKind, PropertyDecl, PropertyKind, ScalarKind};
use crate::diagnostic::KernelDiagnostic;

use super::findings::{Findings, declaration};
use super::frontmatter::{Entry, Value};
use super::lexical;
use super::model::{FieldValue, PropertyValue, RecordValue};

/// One declared property, and where the contract declares it.
///
/// The provenance prefix travels with the declaration so that a finding about
/// a missing record field can point its evidence at the field's own
/// `required = true` leaf, exactly as a missing property's evidence points at
/// the property's.
pub(crate) struct Declared<'a> {
    contract: &'a Contract,
    property: &'a PropertyDecl,
    /// The provenance prefix `type.<type>.property.<name>`.
    prefix: String,
}

impl<'a> Declared<'a> {
    /// The declaration `property` is, on the type named `type_name`.
    pub(crate) fn new(contract: &'a Contract, type_name: &str, property: &'a PropertyDecl) -> Self {
        Self {
            contract,
            property,
            prefix: format!("type.{type_name}.property.{}", property.name()),
        }
    }

    /// The record fields `declared`, carried with where they are declared.
    fn fields<'b>(&'b self, declared: &'b [FieldDecl]) -> Fields<'b> {
        Fields {
            contract: self.contract,
            prefix: &self.prefix,
            declared,
        }
    }
}

/// A record's declared fields, and where the contract declares them.
struct Fields<'a> {
    contract: &'a Contract,
    /// The provenance prefix `type.<type>.property.<name>` the field leaves
    /// sit under — never carrying a sequence index, because every element of a
    /// list of records is declared by the same fields.
    prefix: &'a str,
    declared: &'a [FieldDecl],
}

/// The value a note wrote for a declared property, when its shape admits one.
pub(crate) fn property(
    findings: &mut Findings<'_>,
    source: &Declared<'_>,
    value: &Value,
) -> Option<PropertyValue> {
    let (path, kind) = (source.property.name(), source.property.kind());
    let read = match kind {
        PropertyKind::Record { fields } => value.mapping().map(|entries| {
            PropertyValue::Record(record(findings, &source.fields(fields), path, entries))
        }),
        PropertyKind::ListOfRecord { fields } => value.sequence().map(|items| {
            PropertyValue::RecordList(records(findings, &source.fields(fields), path, items))
        }),
        PropertyKind::List { of } => value
            .sequence()
            .map(|items| PropertyValue::List(elements(findings, path, *of, items))),
        scalar => return one(findings, path, scalar, value).map(PropertyValue::Scalar),
    };
    if read.is_none() {
        mismatch(findings, path, &kind.describe(), value);
    }
    read
}

/// One scalar, checked against the kind's lexical form.
fn one(
    findings: &mut Findings<'_>,
    path: &str,
    kind: &PropertyKind,
    value: &Value,
) -> Option<String> {
    let Some(text) = value.scalar() else {
        mismatch(findings, path, &kind.describe(), value);
        return None;
    };
    if !fits(kind, text) {
        invalid(findings, path, &kind.describe(), value);
    }
    Some(text.to_owned())
}

/// Every element of a `list`, each checked against the element kind.
fn elements(
    findings: &mut Findings<'_>,
    path: &str,
    of: ScalarKind,
    items: &[Value],
) -> Vec<String> {
    let mut elements = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let path = format!("{path}[{index}]");
        let Some(text) = item.scalar() else {
            mismatch(findings, &path, &format!("`{of}`"), item);
            continue;
        };
        if !lexical::scalar(of, text) {
            invalid(findings, &path, &format!("`{of}`"), item);
        }
        elements.push(text.to_owned());
    }
    elements
}

/// Every element of a `list` of `record`, each held to the same fields.
fn records(
    findings: &mut Findings<'_>,
    fields: &Fields<'_>,
    path: &str,
    items: &[Value],
) -> Vec<RecordValue> {
    let mut records = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let path = format!("{path}[{index}]");
        match item.mapping() {
            Some(entries) => records.push(record(findings, fields, &path, entries)),
            None => mismatch(findings, &path, "`record`", item),
        }
    }
    records
}

/// One record value: its declared fields, and what the note wrote for them.
fn record(
    findings: &mut Findings<'_>,
    fields: &Fields<'_>,
    path: &str,
    entries: &[Entry],
) -> RecordValue {
    let values = declared(findings, fields, path, entries);
    undeclared(findings, path, fields.declared, entries);
    RecordValue { fields: values }
}

/// Every declared field, in declaration order, with what the note wrote.
fn declared(
    findings: &mut Findings<'_>,
    fields: &Fields<'_>,
    path: &str,
    entries: &[Entry],
) -> Vec<FieldValue> {
    let mut values = Vec::new();
    for field in fields.declared {
        let path = format!("{path}.{}", field.name());
        match entries.iter().find(|entry| entry.key == field.name()) {
            Some(entry) => values.push(field_value(findings, &path, field, entry)),
            None if field.required() => missing_field(findings, fields, &path, field),
            None => {}
        }
    }
    values
}

/// Every field the record declares nothing for, reported at `info`.
fn undeclared(findings: &mut Findings<'_>, path: &str, fields: &[FieldDecl], entries: &[Entry]) {
    for entry in entries {
        if !fields.iter().any(|field| field.name() == entry.key) {
            undeclared_field(findings, path, entry);
        }
    }
}

/// One field's value, checked exactly as a property's is.
///
/// A field's value is a scalar or the note never got here: the frontmatter
/// subset's shape rule refuses a mapping or a sequence inside a record, so the
/// block would not have loaded at all.
fn field_value(
    findings: &mut Findings<'_>,
    path: &str,
    field: &FieldDecl,
    entry: &Entry,
) -> FieldValue {
    let text = entry.value.scalar().unwrap_or_default();
    if !field_fits(field.kind(), text) {
        invalid(findings, path, &field.kind().describe(), &entry.value);
    }
    FieldValue {
        name: field.name().to_owned(),
        value: text.to_owned(),
    }
}

/// A required field the note does not write, with evidence pointing at the
/// field's own `required = true` leaf — exactly as a missing property's
/// evidence points at the property's.
fn missing_field(findings: &mut Findings<'_>, fields: &Fields<'_>, path: &str, field: &FieldDecl) {
    findings.absent(
        KernelDiagnostic::NoteMissingRequiredProperty,
        format!("the record requires the field `{path}`, and the note writes no value for it"),
        declaration(
            fields.contract,
            &format!("{}.field.{}.required", fields.prefix, field.name()),
            format!(
                "the field is declared {} and required",
                field.kind().describe()
            ),
        ),
    );
}

fn undeclared_field(findings: &mut Findings<'_>, path: &str, entry: &Entry) {
    findings.spanned(
        KernelDiagnostic::NoteUndeclaredProperty,
        format!(
            "the record `{path}` declares no field `{}`, so its value is not validated",
            entry.key
        ),
        entry.key_span.clone(),
    );
}

/// Whether a scalar or `enum` kind admits these bytes.
///
/// The scalar kinds are reached through their spelling, which is the one
/// [`ScalarKind`] writes and [`PropertyKind`] delegates to, so the lattice
/// stays stated in one place. A carrying kind never reaches here.
fn fits(kind: &PropertyKind, text: &str) -> bool {
    match kind {
        PropertyKind::Enum { values } => lexical::member(values, text),
        other => {
            ScalarKind::named(other.as_str()).is_some_and(|scalar| lexical::scalar(scalar, text))
        }
    }
}

fn field_fits(kind: &FieldKind, text: &str) -> bool {
    match kind {
        FieldKind::Scalar(scalar) => lexical::scalar(*scalar, text),
        FieldKind::Enum { values } => lexical::member(values, text),
    }
}

/// The note wrote a shape the declared kind cannot hold.
fn mismatch(findings: &mut Findings<'_>, path: &str, kind: &str, value: &Value) {
    findings.spanned(
        KernelDiagnostic::NotePropertyKindInvalid,
        format!(
            "`{path}` is declared {kind}, and the note writes {}",
            value.describe()
        ),
        value.span.clone(),
    );
}

/// The note wrote bytes the declared kind's lexical form does not admit.
fn invalid(findings: &mut Findings<'_>, path: &str, kind: &str, value: &Value) {
    let text = value.scalar().unwrap_or_default();
    findings.spanned(
        KernelDiagnostic::NotePropertyKindInvalid,
        format!("`{path}` is declared {kind}, and `{text}` is not one"),
        value.span.clone(),
    );
}
