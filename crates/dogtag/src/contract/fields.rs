//! Reading `[[type.property.field]]`: the fields a record property declares.
//!
//! Fields are declared **on the property**, identically for `kind = "record"`
//! and for `kind = "list"` with `of = "record"`. The version-2 record's prose
//! attaches the field list to `kind = "record"` alone and then permits `of =
//! "record"` without saying where *that* shape's fields are declared; the
//! sketch it adopts as written answers it, and answers it with a `list` whose
//! fields sit in `[[type.property.field]]` on the property. That is the reading
//! taken here — one place a field is declared, for both shapes — because it is
//! what the adopted sketch literally shows and the only reading under which `of
//! = "record"` is usable at all.
//!
//! A field's kind is the scalar lattice at full width, `enum` with its own
//! `values` included, and never `record` or `list`. The one-level bound is the
//! whole reason `contract explain`'s nested rendering terminates.
//!
//! Whether the *construct* exists at all is the declared version's answer, and
//! the caller has already asked it: this module is reached only with the
//! version's own [`RecordKind`] in hand, so a version defining no record kind
//! never walks a field list rather than walking one and discarding it.

use core::ops::Range;

use toml::Spanned;
use toml::de::DeValue;

use crate::diagnostic::KernelDiagnostic;

use super::declarations::{enum_values, listing};
use super::kinds::{FieldDecl, FieldKind, ScalarKind};
use super::schema::RecordKind;
use super::sink::{Claim, KeyPath, Named, Report, Section, Seen, Sink};

const FIELDS_HELP: &str = "a record declares at least one `[[type.property.field]]`, and a field is never itself a \
     `record` or a `list` — one level of nesting, exactly as the record kind is scoped";

/// Reads the field list a record property declares, in declaration order.
///
/// `None` is a record whose fields did not resolve, which the caller turns into
/// a dropped declaration: a property whose shape is unknown is worse than one
/// that is absent, because every later reader would validate notes against a
/// record the file never managed to describe.
pub(crate) fn declared(
    sink: &mut Sink<'_>,
    section: &Section<'_, '_>,
    records: &'static RecordKind,
) -> Option<Vec<FieldDecl>> {
    let Some(value) = section.get("field") else {
        invalid(sink, section, "declares no `field`");
        return None;
    };
    let entries = sink.array(value, "field")?;
    if entries.is_empty() {
        invalid(sink, section, "declares an empty `field` list");
        return None;
    }
    let mut scope = Scope {
        parent: &section.path,
        records,
        seen: Seen::new(),
    };
    let fields: Vec<FieldDecl> = entries
        .iter()
        .filter_map(|entry| field(sink, entry, &mut scope))
        .collect();
    // Every field or none. A record missing one of its fields is a shape the
    // file does not describe, and the fault that lost it is already reported.
    (fields.len() == entries.len()).then_some(fields)
}

/// One property's field walk: where its fields are addressed, what the declared
/// version lets them write, and which names are already claimed.
struct Scope<'a> {
    parent: &'a KeyPath,
    records: &'static RecordKind,
    seen: Seen,
}

fn field(
    sink: &mut Sink<'_>,
    value: &Spanned<DeValue<'_>>,
    scope: &mut Scope<'_>,
) -> Option<FieldDecl> {
    let table = sink.table(value, "field")?;
    let entry = Section {
        table,
        span: value.span(),
        label: "`[[type.property.field]]`".to_owned(),
        path: KeyPath::nameless(),
    };
    let named = sink.name_of(&entry, "name");
    let section = addressed(entry, scope.parent, named.as_ref());
    let spelled = spelled_kind(sink, &section);
    let allowed = scope.records.field_keys(spelled.kind);
    sink.sweep(&section, allowed);
    let required = sink.optional_flag(&section, "required", scope.records.field_required);
    // The kind is read before the name is claimed and required after it, for
    // the reason a property's is: a second field of the same name goes
    // unreported until the first one's kind is fixed otherwise.
    let kind = field_kind(sink, &section, &spelled);
    let named = named?;
    sink.written(section.leaf("name").key, named.span.clone());
    let name = named.text.to_owned();
    let claim = Claim {
        message: format!("the record declares two fields named `{name}`"),
        kind: KernelDiagnostic::ContractDuplicateField,
        named,
    };
    sink.keep(&mut scope.seen, claim)?;
    Some(FieldDecl {
        name,
        kind: kind?,
        required,
    })
}

/// The same entry, named and addressed once its own name has been read.
fn addressed<'a, 'i>(
    entry: Section<'a, 'i>,
    parent: &KeyPath,
    named: Option<&Named<'_>>,
) -> Section<'a, 'i> {
    let name = named.map(|found| found.text);
    let label = name.map_or_else(|| entry.label.clone(), |name| format!("the field `{name}`"));
    Section {
        label,
        path: parent.child("field").child_opt(name),
        ..entry
    }
}

/// The `kind` a field spells, and where it spells it.
struct Spelled<'a> {
    kind: Option<&'a str>,
    at: Range<usize>,
}

fn spelled_kind<'a>(sink: &mut Sink<'_>, section: &Section<'a, '_>) -> Spelled<'a> {
    let value = sink.required(section, "kind");
    Spelled {
        at: value.map_or_else(|| section.span.clone(), Spanned::span),
        kind: value.and_then(|value| sink.string(value, section.leaf("kind"))),
    }
}

/// The scalar lattice at full width, and nothing below it.
fn field_kind(
    sink: &mut Sink<'_>,
    section: &Section<'_, '_>,
    spelled: &Spelled<'_>,
) -> Option<FieldKind> {
    match spelled.kind? {
        "enum" => enum_values(sink, section).map(|values| FieldKind::Enum { values }),
        // `record` and `list` are refused as *nesting* rather than reported as
        // kinds the format does not define, because version 2 defines both.
        // What it does not define is a second level, and a message saying the
        // kind does not exist would be false about the one thing to fix.
        nested @ ("record" | "list") => {
            let detail = format!("declares `kind = \"{nested}\"`");
            invalid_at(sink, section, &detail, spelled.at.clone());
            None
        }
        other => scalar(sink, other, spelled.at.clone()).map(FieldKind::Scalar),
    }
}

fn scalar(sink: &mut Sink<'_>, spelled: &str, at: Range<usize>) -> Option<ScalarKind> {
    let found = ScalarKind::named(spelled);
    if found.is_none() {
        let message = format!("`{spelled}` is not a value kind this contract version defines");
        let names = ScalarKind::ALL.iter().map(|kind| kind.as_str());
        let report = Report::new(message)
            .with_help(listing("a field's kind is one of", names.chain(["enum"])));
        let at = sink.location(at);
        sink.report(KernelDiagnostic::ContractUnknownPropertyKind, report, at);
    }
    found
}

/// A record whose field list is missing, empty, or holds a field the one-level
/// bound refuses.
fn invalid(sink: &mut Sink<'_>, section: &Section<'_, '_>, detail: &str) {
    let at = section.span.clone();
    invalid_at(sink, section, detail, at);
}

fn invalid_at(sink: &mut Sink<'_>, section: &Section<'_, '_>, detail: &str, at: Range<usize>) {
    let report =
        Report::new(format!("{} {detail}", section.label)).with_help(FIELDS_HELP.to_owned());
    let at = sink.location(at);
    sink.report(KernelDiagnostic::ContractInvalidRecordFields, report, at);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::declarations;
    use crate::contract::kinds::PropertyKind;
    use crate::contract::model::TypeDecl;
    use crate::contract::schema;
    use crate::contract::sink::tests::{root_of, text_of};
    use crate::diagnostic::DiagnosticList;
    use crate::provenance::{Provenance, Source};

    struct Read {
        types: Vec<TypeDecl>,
        diagnostics: DiagnosticList,
        provenance: Provenance,
    }

    fn read_at(schema: &'static schema::Schema, source: &str) -> Read {
        let text = text_of(source);
        let document = root_of(&text);
        let mut sink = Sink::new(&text, schema);
        let types = declarations::types(&mut sink, document.get_ref());
        let (diagnostics, provenance) = sink.finish();
        Read {
            types,
            diagnostics,
            provenance,
        }
    }

    fn read(source: &str) -> Read {
        read_at(&schema::VERSION_2, source)
    }

    fn ids(read: &Read) -> Vec<&str> {
        read.diagnostics
            .as_slice()
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect()
    }

    /// The kind the one property of the one type resolved to.
    fn resolved(read: &Read) -> Option<&PropertyKind> {
        Some(read.types.first()?.properties().first()?.kind())
    }

    /// A type declaring the properties `body` spells.
    fn typed(body: &str) -> String {
        format!("[[type]]\nname = \"person\"\n{body}")
    }

    /// A property of the given shape, declaring `body` under it.
    fn shaped(shape: &str, body: &str) -> String {
        typed(&format!(
            "\n  [[type.property]]\n  name = \"legal_name\"\n{shape}{body}"
        ))
    }

    /// A `record` property declaring `body` as its field list.
    fn record(body: &str) -> String {
        shaped("  kind = \"record\"\n", body)
    }

    /// A `record` property whose one field declares `body`.
    fn one(body: &str) -> String {
        record(&format!("\n    [[type.property.field]]\n{body}"))
    }

    const GIVEN: &str = "    name = \"given\"\n    kind = \"string\"\n";

    #[test]
    fn a_record_declares_its_own_fields_in_declaration_order() {
        let read = read(&record(concat!(
            "\n    [[type.property.field]]\n    name = \"given\"\n    kind = \"string\"\n",
            "    required = true\n",
            "\n    [[type.property.field]]\n    name = \"family\"\n    kind = \"string\"\n",
        )));
        assert!(read.diagnostics.is_empty());
        let fields = resolved(&read)
            .and_then(PropertyKind::fields)
            .expect("a record");
        let declared: Vec<(&str, &str, bool)> = fields
            .iter()
            .map(|field| (field.name(), field.kind().as_str(), field.required()))
            .collect();
        assert_eq!(
            declared,
            [("given", "string", true), ("family", "string", false)]
        );
    }

    #[test]
    fn a_list_may_name_record_in_of_and_declares_its_fields_on_the_property() {
        // The adopted sketch's own shape: the fields sit under the property,
        // not under some separately declared record the format has no way to
        // name.
        let source = shaped(
            "  kind = \"list\"\n  of = \"record\"\n",
            &format!("\n    [[type.property.field]]\n{GIVEN}"),
        );
        let read = read(&source);
        assert!(read.diagnostics.is_empty());
        let kind = resolved(&read).expect("a property");
        let shape = (kind.as_str(), kind.element(), kind.fields().map(<[_]>::len));
        // A record is not a member of the scalar lattice, so the typed element
        // answers nothing and the fields answer instead.
        assert_eq!(shape, ("list", None, Some(1)));
    }

    #[test]
    fn a_fields_kind_may_be_any_member_of_the_scalar_lattice() {
        let mut body = String::new();
        for kind in ScalarKind::ALL {
            body.push_str(&format!(
                "\n    [[type.property.field]]\n    name = \"f_{kind}\"\n    kind = \"{kind}\"\n"
            ));
        }
        let read = read(&record(&body));
        assert!(read.diagnostics.is_empty());
        let fields = resolved(&read)
            .and_then(PropertyKind::fields)
            .expect("a record");
        assert_eq!(fields.len(), 6);
        assert_eq!(fields[5].kind().scalar(), Some(ScalarKind::DateTime));
    }

    #[test]
    fn a_field_may_declare_an_enum_with_its_own_values() {
        // "Drawn from the existing scalar lattice" read at full width: the
        // record kind's canonical instance is a labeled channel, which is
        // exactly where a closed label set earns its keep.
        let read = read(&one(
            "    name = \"label\"\n    kind = \"enum\"\n    values = [\"home\", \"work\"]\n",
        ));
        assert!(read.diagnostics.is_empty());
        let fields = resolved(&read)
            .and_then(PropertyKind::fields)
            .expect("a record");
        assert_eq!(
            fields[0].kind().values(),
            Some(&["home".to_owned(), "work".to_owned()][..])
        );
    }

    #[test]
    fn an_enum_field_is_held_to_the_same_values_rule_a_property_is() {
        let read = read(&one("    name = \"label\"\n    kind = \"enum\"\n"));
        assert_eq!(ids(&read), ["contract.invalid-enum-values"]);
        assert!(
            read.diagnostics.as_slice()[0]
                .message
                .contains("the field `label`")
        );
    }

    #[test]
    fn an_omitted_required_takes_the_declaring_versions_default() {
        let read = read(&one(GIVEN));
        let fields = resolved(&read)
            .and_then(PropertyKind::fields)
            .expect("a record");
        assert!(!fields[0].required());
        assert_eq!(
            read.provenance
                .get("type.person.property.legal_name.field.given.required")
                .map(|entry| entry.source),
            Some(Source::Default {
                contract_version: 2
            })
        );
    }

    #[test]
    fn every_leaf_of_a_field_records_where_it_is_written() {
        let read = read(&one(
            "    name = \"label\"\n    kind = \"enum\"\n    values = [\"home\"]\n",
        ));
        let under = "type.person.property.legal_name.field.label";
        for leaf in ["name", "kind", "values"] {
            assert!(
                read.provenance.get(&format!("{under}.{leaf}")).is_some(),
                "`{leaf}` records nothing"
            );
        }
    }

    #[test]
    fn a_record_that_declares_no_field_is_invalid() {
        let read = read(&record(""));
        assert_eq!(ids(&read), ["contract.invalid-record-fields"]);
        assert!(read.diagnostics.as_slice()[0].help.is_some());
        assert!(read.types[0].properties().is_empty(), "the property drops");
    }

    #[test]
    fn a_record_declaring_an_empty_field_list_is_invalid() {
        let read = read(&record("  field = []\n"));
        assert_eq!(ids(&read), ["contract.invalid-record-fields"]);
        assert!(read.diagnostics.as_slice()[0].message.contains("empty"));
    }

    #[test]
    fn a_field_list_of_the_wrong_shape_is_reported_as_a_wrong_type() {
        assert_eq!(
            ids(&read(&record("  field = 1\n"))),
            ["contract.value-wrong-type"]
        );
        assert_eq!(
            ids(&read(&record("  field = [1]\n"))),
            ["contract.value-wrong-type"]
        );
    }

    #[test]
    fn a_field_may_be_neither_a_record_nor_a_list() {
        // One level of nesting. Refused as nesting rather than as a kind the
        // format does not define, because contract version 2 defines both.
        for nested in ["record", "list"] {
            let read = read(&one(&format!(
                "    name = \"inner\"\n    kind = \"{nested}\"\n"
            )));
            assert_eq!(ids(&read), ["contract.invalid-record-fields"], "{nested}");
            assert!(
                read.diagnostics.as_slice()[0]
                    .message
                    .contains(&format!("kind = \"{nested}\"")),
                "{nested}"
            );
        }
    }

    #[test]
    fn a_field_never_carries_of_and_is_told_so_beside_the_nesting_it_tried() {
        // Two faults rather than one: `list` is a kind a field may not hold,
        // and `of` is a key no field carries at any kind. Each has its own
        // repair, so each is said.
        let read = read(&one(
            "    name = \"inner\"\n    kind = \"list\"\n    of = \"string\"\n",
        ));
        assert_eq!(
            ids(&read),
            ["contract.unknown-key", "contract.invalid-record-fields"]
        );
    }

    #[test]
    fn a_field_kind_outside_the_lattice_names_the_kinds_a_field_may_hold() {
        let read = read(&one("    name = \"given\"\n    kind = \"url\"\n"));
        assert_eq!(ids(&read), ["contract.unknown-property-kind"]);
        let help = read.diagnostics.as_slice()[0]
            .help
            .as_deref()
            .expect("advice");
        assert_eq!(
            help,
            "a field's kind is one of `string`, `integer`, `float`, `boolean`, `date`, \
             `datetime`, `enum`"
        );
    }

    #[test]
    fn an_unresolved_field_kind_does_not_also_complain_about_its_values() {
        assert_eq!(
            ids(&read(&one(
                "    name = \"given\"\n    kind = \"url\"\n    values = [\"a\"]\n"
            ))),
            ["contract.unknown-property-kind"]
        );
    }

    #[test]
    fn values_are_illegal_on_a_field_that_is_not_an_enum() {
        assert_eq!(
            ids(&read(&one(
                "    name = \"given\"\n    kind = \"string\"\n    values = [\"a\"]\n"
            ))),
            ["contract.unknown-key"]
        );
    }

    #[test]
    fn a_field_without_a_name_or_without_a_kind_is_a_missing_key() {
        assert_eq!(
            ids(&read(&one("    kind = \"string\"\n"))),
            ["contract.missing-key"]
        );
        assert_eq!(
            ids(&read(&one("    name = \"given\"\n"))),
            ["contract.missing-key"]
        );
    }

    #[test]
    fn a_field_name_obeys_the_rule_every_declaration_name_obeys() {
        // The same identifier, because provenance addresses a field by joining
        // the names above it and a dotted one would address something else.
        for name in ["", "a.b"] {
            let read = read(&one(&format!(
                "    name = \"{name}\"\n    kind = \"string\"\n"
            )));
            assert_eq!(
                ids(&read),
                ["contract.declaration-name-invalid"],
                "`{name}`"
            );
        }
    }

    #[test]
    fn a_non_string_field_name_is_reported_as_a_wrong_type() {
        assert_eq!(
            ids(&read(&one("    name = 4\n    kind = \"string\"\n"))),
            ["contract.value-wrong-type"]
        );
    }

    #[test]
    fn an_unknown_key_on_a_field_is_fatal() {
        let read = read(&one(
            "    name = \"given\"\n    kind = \"string\"\n    format = \"title\"\n",
        ));
        assert_eq!(ids(&read), ["contract.unknown-key"]);
        assert!(
            read.diagnostics.as_slice()[0]
                .message
                .contains("the field `given`")
        );
    }

    #[test]
    fn a_nameless_field_is_still_named_by_its_table_in_a_message() {
        let read = read(&one("    kind = \"string\"\n    format = \"title\"\n"));
        assert!(
            read.diagnostics
                .as_slice()
                .iter()
                .any(|diagnostic| diagnostic.message.contains("`[[type.property.field]]`"))
        );
    }

    #[test]
    fn two_fields_of_one_record_sharing_a_name_point_at_both() {
        let read = read(&record(concat!(
            "\n    [[type.property.field]]\n    name = \"given\"\n    kind = \"string\"\n",
            "\n    [[type.property.field]]\n    name = \"given\"\n    kind = \"date\"\n",
        )));
        assert_eq!(ids(&read), ["contract.duplicate-field"]);
        assert_eq!(read.diagnostics.as_slice()[0].related.len(), 1);
        assert!(read.types[0].properties().is_empty(), "the record drops");
    }

    #[test]
    fn a_records_fields_are_part_of_the_kind_its_name_declares() {
        // One property name declares one kind corpus-wide, and a record's
        // fields are what its kind *is* — the same reason two `enum`
        // declarations over different values disagree.
        let source = concat!(
            "[[type]]\nname = \"person\"\n",
            "\n  [[type.property]]\n  name = \"legal_name\"\n  kind = \"record\"\n",
            "\n    [[type.property.field]]\n    name = \"given\"\n    kind = \"string\"\n",
            "\n[[type]]\nname = \"author\"\n",
            "\n  [[type.property]]\n  name = \"legal_name\"\n  kind = \"record\"\n",
            "\n    [[type.property.field]]\n    name = \"given\"\n    kind = \"date\"\n",
        );
        let read = read(source);
        assert_eq!(ids(&read), ["contract.property-kind-conflict"]);
        assert!(
            read.diagnostics.as_slice()[0]
                .message
                .contains("`record` with given: `date`")
        );
    }

    #[test]
    fn a_version_that_defines_no_record_kind_refuses_both_of_its_spellings() {
        // Absent from a version-1 model rather than merely illegal in it: the
        // lattice itself is version-scoped, so widening it for version 2 does
        // not widen it for a contract that declares version 1.
        let bare = read_at(&schema::VERSION_1, &record(""));
        assert_eq!(ids(&bare), ["contract.unknown-property-kind"]);
        let listed = read_at(
            &schema::VERSION_1,
            &shaped("  kind = \"list\"\n  of = \"record\"\n", ""),
        );
        assert_eq!(ids(&listed), ["contract.invalid-list-of"]);
    }

    #[test]
    fn a_field_list_is_an_unknown_key_wherever_no_record_is_declared() {
        let under = format!("\n    [[type.property.field]]\n{GIVEN}");
        // At version 1, on the one spelling that would carry it at version 2.
        let listed = read_at(
            &schema::VERSION_1,
            &shaped("  kind = \"list\"\n  of = \"string\"\n", &under),
        );
        assert!(ids(&listed).contains(&"contract.unknown-key"));
        // At version 2, on a list whose elements are not records.
        let scalars = read(&shaped("  kind = \"list\"\n  of = \"string\"\n", &under));
        assert_eq!(ids(&scalars), ["contract.unknown-key"]);
    }

    #[test]
    fn an_unresolved_shape_does_not_also_complain_about_the_field_list_under_it() {
        let under = format!("\n    [[type.property.field]]\n{GIVEN}");
        assert_eq!(
            ids(&read(&shaped("  kind = \"recrod\"\n", &under))),
            ["contract.unknown-property-kind"]
        );
        assert_eq!(
            ids(&read(&shaped(
                "  kind = \"list\"\n  of = \"recrod\"\n",
                &under
            ))),
            ["contract.invalid-list-of"]
        );
    }

    #[test]
    fn a_list_whose_of_is_not_a_string_is_still_reported_once() {
        assert_eq!(
            ids(&read(&shaped("  kind = \"list\"\n  of = 1\n", ""))),
            ["contract.value-wrong-type"]
        );
    }
}
