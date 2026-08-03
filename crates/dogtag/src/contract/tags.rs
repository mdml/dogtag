//! Reading the tag vocabulary: `[tags]` and `[[type.tag-namespace]]`.
//!
//! Both tables exist only at a version that defines them, so every entry point
//! here asks the declared version's schema first and answers "nothing" when the
//! version has no tag vocabulary. That is stronger than the key sweep, which
//! would only make `[tags]` *illegal* in a version-1 contract: the walk never
//! runs, so a version-1 model has no tag vocabulary in it at all rather than one
//! the parser resolved and the diagnostics happened to condemn.
//!
//! This module reads the declarations' **shape**. Whether the property `[tags]`
//! names is declared, and declared as a list of string, are rules over the whole
//! contract and live in [`super::validate`] beside the flag and lifecycle rules.
//! The one rule that stays here is the repeat, because naming both declarations
//! needs two spans and the resolved model keeps one namespace per prefix.

use core::ops::Range;

use toml::Spanned;
use toml::de::{DeTable, DeValue};

use crate::diagnostic::KernelDiagnostic;
use crate::document;

use super::schema::TagVocabulary;
use super::sink::{Claim, KeyPath, Named, Report, Section, Seen, Sink};
use super::vocabulary::{NamespaceMembership, TagNamespaceDecl, TagsDecl};

const MEMBERSHIP_HELP: &str = "a tag namespace declares either `values`, a non-empty list of unique strings, or `open = true`";

/// Reads the optional top-level `[tags]` table.
///
/// Three separate absences answer `None` and only one of them is a fault: a
/// version with no tag vocabulary, a contract that declares no `[tags]`, and a
/// `[tags]` that did not resolve — which is recorded as a dropped declaration so
/// the cross-reference rules do not conclude that a file declaring the table
/// declares no table.
pub(crate) fn table(sink: &mut Sink<'_>, root: &DeTable<'_>) -> Option<TagsDecl> {
    let allowed = sink.schema().tags.as_ref()?.tags;
    let value = document::get(root, "tags")?;
    let declared = read_table(sink, value, allowed);
    if declared.is_none() {
        sink.drop_declaration();
    }
    declared
}

fn read_table(
    sink: &mut Sink<'_>,
    value: &Spanned<DeValue<'_>>,
    allowed: &'static [&'static str],
) -> Option<TagsDecl> {
    let table = sink.table(value, "tags")?;
    let section = Section {
        table,
        span: value.span(),
        label: "`[tags]`".to_owned(),
        path: KeyPath::root().child("tags"),
    };
    sink.sweep(&section, allowed);
    let named = sink.name_of(&section, "property")?;
    sink.written(section.leaf("property").key, named.span.clone());
    Some(TagsDecl {
        property: named.text.to_owned(),
    })
}

/// Reads the `[[type.tag-namespace]]` entries one type declares, in declaration
/// order.
pub(crate) fn namespaces(sink: &mut Sink<'_>, section: &Section<'_, '_>) -> Vec<TagNamespaceDecl> {
    let Some(vocabulary) = sink.schema().tags.as_ref() else {
        return Vec::new();
    };
    let Some(value) = section.get("tag-namespace") else {
        return Vec::new();
    };
    let Some(array) = sink.array(value, "tag-namespace") else {
        return Vec::new();
    };
    let mut scope = Scope {
        parent: &section.path,
        vocabulary,
        seen: Seen::new(),
    };
    array
        .iter()
        .filter_map(|entry| namespace(sink, entry, &mut scope))
        .collect()
}

/// One type's namespace walk: where its entries are addressed, what the
/// declared version lets them write, and which prefixes are already claimed.
struct Scope<'a> {
    parent: &'a KeyPath,
    vocabulary: &'static TagVocabulary,
    seen: Seen,
}

fn namespace(
    sink: &mut Sink<'_>,
    value: &Spanned<DeValue<'_>>,
    scope: &mut Scope<'_>,
) -> Option<TagNamespaceDecl> {
    let table = sink.table(value, "tag-namespace")?;
    let entry = Section {
        table,
        span: value.span(),
        label: "`[[type.tag-namespace]]`".to_owned(),
        path: KeyPath::nameless(),
    };
    // A prefix is read where every other declaration's name is read, because
    // provenance addresses a namespace's leaves by joining it into a dotted key
    // path — `type.log.tag-namespace.log/.required`. The record's own rule that
    // a prefix is non-empty comes with that seam; the rule that it holds no `.`
    // is the price of the addressing, since two prefixes differing only by one
    // would collide on a single provenance key and the second recorded would
    // silently replace the first. The record decides the first half and is
    // silent on the second, so a prefix holding a `.` is refused here.
    let named = sink.name_of(&entry, "prefix");
    let section = addressed(entry, scope.parent, named.as_ref());
    sink.sweep(&section, scope.vocabulary.namespace);
    let required = sink.optional_flag(&section, "required", scope.vocabulary.namespace_required);
    let membership = membership(sink, &section);
    let claimed = claim(sink, &mut scope.seen, named)?;
    sink.written(section.leaf("prefix").key, claimed.at);
    let Some(membership) = membership else {
        sink.drop_declaration();
        return None;
    };
    Some(TagNamespaceDecl {
        prefix: claimed.prefix,
        required,
        membership,
    })
}

/// The same entry, named and addressed once its own prefix has been read.
fn addressed<'a, 'i>(
    entry: Section<'a, 'i>,
    parent: &KeyPath,
    named: Option<&Named<'_>>,
) -> Section<'a, 'i> {
    let prefix = named.map(|found| found.text);
    let label = prefix.map_or_else(
        || entry.label.clone(),
        |prefix| format!("the tag namespace `{prefix}`"),
    );
    Section {
        label,
        path: parent.child("tag-namespace").child_opt(prefix),
        ..entry
    }
}

/// A prefix this type has not already claimed, and where it is written.
struct Claimed {
    prefix: String,
    at: Range<usize>,
}

fn claim(sink: &mut Sink<'_>, seen: &mut Seen, named: Option<Named<'_>>) -> Option<Claimed> {
    let named = named?;
    let claimed = Claimed {
        prefix: named.text.to_owned(),
        at: named.span.clone(),
    };
    let claim = Claim {
        message: format!(
            "the type declares two tag namespaces with the prefix `{}`",
            claimed.prefix
        ),
        kind: KernelDiagnostic::ContractDuplicateTagNamespace,
        named,
    };
    sink.keep(seen, claim)?;
    Some(claimed)
}

/// Exactly one of `values` and `open = true`.
///
/// The branch is on which key the contract *writes*, exactly as
/// `[lifecycle.ordinary]` branches: omission is never a decision, so `open =
/// false` on its own declares neither and `values` beside `open = false`
/// declares both.
fn membership(sink: &mut Sink<'_>, section: &Section<'_, '_>) -> Option<NamespaceMembership> {
    match (section.get("values"), section.get("open")) {
        (Some(values), None) => closed(sink, section, values),
        (None, Some(declared)) => unbounded(sink, section, declared),
        _ => {
            invalid(
                sink,
                section,
                "does not declare exactly one of `values` and `open = true`".to_owned(),
            );
            None
        }
    }
}

/// A closed vocabulary: present, non-empty, all strings, no repeats. Each
/// member names the remainder after the prefix.
fn closed(
    sink: &mut Sink<'_>,
    section: &Section<'_, '_>,
    value: &Spanned<DeValue<'_>>,
) -> Option<NamespaceMembership> {
    let at = value.span();
    let members: Option<Vec<String>> = sink
        .array(value, "values")?
        .iter()
        .map(|member| document::expect_string(member).ok().map(str::to_owned))
        .collect();
    let Some(values) = members else {
        invalid(
            sink,
            section,
            "declares a `values` entry that is not a string".to_owned(),
        );
        return None;
    };
    let values = checked(sink, section, values)?;
    sink.written(section.leaf("values").key, at);
    Some(NamespaceMembership::Closed { values })
}

fn checked(
    sink: &mut Sink<'_>,
    section: &Section<'_, '_>,
    values: Vec<String>,
) -> Option<Vec<String>> {
    if values.is_empty() {
        invalid(sink, section, "declares an empty `values`".to_owned());
        return None;
    }
    match repeated(&values) {
        None => Some(values),
        Some(repeat) => {
            let detail = format!("declares `{repeat}` twice in `values`");
            invalid(sink, section, detail);
            None
        }
    }
}

/// The first member that repeats an earlier one.
fn repeated(values: &[String]) -> Option<&String> {
    values
        .iter()
        .enumerate()
        .find_map(|(index, member)| values[..index].contains(member).then_some(member))
}

/// `open = true` declares the namespace without bounding its membership.
fn unbounded(
    sink: &mut Sink<'_>,
    section: &Section<'_, '_>,
    value: &Spanned<DeValue<'_>>,
) -> Option<NamespaceMembership> {
    if document::expect_bool(value).ok() != Some(true) {
        invalid(
            sink,
            section,
            "declares `open` as something other than `true`".to_owned(),
        );
        return None;
    }
    sink.written(section.leaf("open").key, value.span());
    Some(NamespaceMembership::Open)
}

fn invalid(sink: &mut Sink<'_>, section: &Section<'_, '_>, detail: String) {
    let report =
        Report::new(format!("{} {detail}", section.label)).with_help(MEMBERSHIP_HELP.to_owned());
    let at = sink.location(section.span.clone());
    sink.report(KernelDiagnostic::ContractInvalidTagNamespace, report, at);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::schema;
    use crate::contract::sink::tests::{root_of, text_of};
    use crate::diagnostic::DiagnosticList;
    use crate::provenance::{Provenance, Source};

    struct Read {
        tags: Option<TagsDecl>,
        namespaces: Vec<TagNamespaceDecl>,
        diagnostics: DiagnosticList,
        provenance: Provenance,
    }

    /// Reads `source` as a whole contract root: its `[tags]` table, and the
    /// namespaces the one `[[type]]` it may declare carries.
    fn read_at(schema: &'static schema::Schema, source: &str) -> Read {
        let text = text_of(source);
        let document = root_of(&text);
        let mut sink = Sink::new(&text, schema);
        let root = document.get_ref();
        let tags = table(&mut sink, root);
        let namespaces = one_types_namespaces(&mut sink, root);
        let (diagnostics, provenance) = sink.finish();
        Read {
            tags,
            namespaces,
            diagnostics,
            provenance,
        }
    }

    fn read(source: &str) -> Read {
        read_at(&schema::VERSION_2, source)
    }

    /// The namespaces the first `[[type]]` declares, when the source has one.
    fn one_types_namespaces(sink: &mut Sink<'_>, root: &DeTable<'_>) -> Vec<TagNamespaceDecl> {
        let Some(value) = document::get(root, "type") else {
            return Vec::new();
        };
        let array = sink.array(value, "type").expect("an array of types");
        let entry = array.first().expect("one type");
        let table = sink.table(entry, "type").expect("a table");
        let section = Section {
            table,
            span: entry.span(),
            label: "the type `log`".to_owned(),
            path: KeyPath::root().child("type").child_opt(Some("log")),
        };
        namespaces(sink, &section)
    }

    fn ids(read: &Read) -> Vec<&str> {
        read.diagnostics
            .as_slice()
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect()
    }

    /// A type declaring the namespaces `body` spells.
    fn typed(body: &str) -> String {
        format!("[tags]\nproperty = \"labels\"\n\n[[type]]\nname = \"log\"\n{body}")
    }

    /// One namespace declaring `body` beyond its prefix.
    fn one(body: &str) -> String {
        typed(&format!(
            "\n  [[type.tag-namespace]]\n  prefix = \"log/\"\n{body}"
        ))
    }

    #[test]
    fn a_tags_table_names_the_property_and_records_where_it_is_written() {
        let read = read("[tags]\nproperty = \"labels\"\n");
        assert!(read.diagnostics.is_empty());
        assert_eq!(
            read.tags.map(|tags| tags.property).as_deref(),
            Some("labels")
        );
        assert_eq!(
            read.provenance
                .get("tags.property")
                .map(|entry| entry.source),
            Some(Source::Contract)
        );
    }

    #[test]
    fn a_contract_declaring_no_tags_table_declares_no_tag_vocabulary() {
        let read = read("[[type]]\nname = \"log\"\n");
        assert!(read.diagnostics.is_empty());
        assert!(read.tags.is_none());
        assert!(read.namespaces.is_empty());
    }

    #[test]
    fn a_version_that_defines_no_tag_vocabulary_reads_neither_table() {
        // The construct is absent from a version-1 model rather than merely
        // illegal in it: nothing here reports, because the version's key sets
        // have already refused both keys, and nothing here resolves either.
        let source = one("  open = true\n");
        let read = read_at(&schema::VERSION_1, &source);
        assert!(read.diagnostics.is_empty());
        assert!(read.tags.is_none());
        assert!(read.namespaces.is_empty());
    }

    #[test]
    fn a_tags_table_that_is_not_a_table_is_reported_as_a_wrong_type() {
        assert_eq!(
            ids(&read("tags = \"labels\"\n")),
            ["contract.value-wrong-type"]
        );
    }

    #[test]
    fn a_tags_table_without_a_property_is_a_missing_key() {
        assert_eq!(ids(&read("[tags]\n")), ["contract.missing-key"]);
    }

    #[test]
    fn an_unknown_key_in_the_tags_table_is_fatal() {
        let read = read("[tags]\nproperty = \"labels\"\nprefix = \"t/\"\n");
        assert_eq!(ids(&read), ["contract.unknown-key"]);
        assert!(read.diagnostics.as_slice()[0].message.contains("`[tags]`"));
    }

    #[test]
    fn a_tag_property_that_cannot_name_a_declaration_is_refused() {
        assert_eq!(
            ids(&read("[tags]\nproperty = \"a.b\"\n")),
            ["contract.declaration-name-invalid"]
        );
    }

    #[test]
    fn a_namespace_reads_its_prefix_its_vocabulary_and_whether_it_is_required() {
        let read = read(&one(
            "  required = true\n  values = [\"workout\", \"reading\"]\n",
        ));
        assert!(read.diagnostics.is_empty());
        let declared = &read.namespaces[0];
        assert_eq!(declared.prefix(), "log/");
        assert!(declared.required());
        assert_eq!(
            declared.values(),
            Some(&["workout".to_owned(), "reading".to_owned()][..])
        );
        assert!(!declared.membership().is_open());
    }

    #[test]
    fn an_open_namespace_declares_no_vocabulary_at_all() {
        let read = read(&one("  open = true\n"));
        assert!(read.diagnostics.is_empty());
        assert_eq!(read.namespaces[0].membership(), &NamespaceMembership::Open);
        assert!(read.namespaces[0].values().is_none());
    }

    #[test]
    fn an_omitted_required_takes_the_declaring_versions_default() {
        let read = read(&one("  open = true\n"));
        assert!(!read.namespaces[0].required());
        assert_eq!(
            read.provenance
                .get("type.log.tag-namespace.log/.required")
                .map(|entry| entry.source),
            Some(Source::Default {
                contract_version: 2
            })
        );
    }

    #[test]
    fn every_leaf_of_a_namespace_records_where_it_is_written() {
        let read = read(&one("  required = true\n  values = [\"workout\"]\n"));
        let keys: Vec<&str> = read
            .provenance
            .entries()
            .map(|entry| entry.key.as_str())
            .collect();
        assert_eq!(
            keys,
            [
                "tags.property",
                "type.log.tag-namespace.log/.prefix",
                "type.log.tag-namespace.log/.required",
                "type.log.tag-namespace.log/.values",
            ]
        );
    }

    #[test]
    fn an_open_namespace_records_the_declaration_it_made() {
        let read = read(&one("  open = true\n"));
        assert!(
            read.provenance
                .get("type.log.tag-namespace.log/.open")
                .is_some()
        );
    }

    #[test]
    fn namespaces_read_in_declaration_order_and_are_never_sorted() {
        let read = read(&typed(concat!(
            "\n  [[type.tag-namespace]]\n  prefix = \"z/\"\n  open = true\n",
            "\n  [[type.tag-namespace]]\n  prefix = \"a/\"\n  open = true\n",
        )));
        let prefixes: Vec<&str> = read
            .namespaces
            .iter()
            .map(TagNamespaceDecl::prefix)
            .collect();
        assert_eq!(prefixes, ["z/", "a/"]);
    }

    #[test]
    fn a_namespace_collection_of_the_wrong_shape_is_reported_as_a_wrong_type() {
        assert_eq!(
            ids(&read(&typed("tag-namespace = 1\n"))),
            ["contract.value-wrong-type"]
        );
        assert_eq!(
            ids(&read(&typed("tag-namespace = [1]\n"))),
            ["contract.value-wrong-type"]
        );
    }

    #[test]
    fn a_namespace_without_a_prefix_is_a_missing_key() {
        let read = read(&typed("\n  [[type.tag-namespace]]\n  open = true\n"));
        assert_eq!(ids(&read), ["contract.missing-key"]);
        assert!(read.namespaces.is_empty());
    }

    #[test]
    fn an_empty_prefix_cannot_name_a_declaration() {
        let read = read(&typed(
            "\n  [[type.tag-namespace]]\n  prefix = \"\"\n  open = true\n",
        ));
        assert_eq!(ids(&read), ["contract.declaration-name-invalid"]);
    }

    #[test]
    fn a_prefix_holding_a_dot_cannot_name_a_declaration() {
        // Its leaves would be addressed at `type.log.tag-namespace.v1.2/…`,
        // which is where another declaration's key path already goes.
        let read = read(&typed(
            "\n  [[type.tag-namespace]]\n  prefix = \"v1.2/\"\n  open = true\n",
        ));
        assert_eq!(ids(&read), ["contract.declaration-name-invalid"]);
    }

    #[test]
    fn an_unknown_key_on_a_namespace_is_fatal() {
        let read = read(&one("  open = true\n  separator = \"/\"\n"));
        assert_eq!(ids(&read), ["contract.unknown-key"]);
        assert!(
            read.diagnostics.as_slice()[0]
                .message
                .contains("the tag namespace `log/`")
        );
    }

    #[test]
    fn a_nameless_namespace_is_still_named_by_its_table_in_a_message() {
        let read = read(&typed(
            "\n  [[type.tag-namespace]]\n  separator = \"/\"\n  open = true\n",
        ));
        assert!(
            read.diagnostics
                .as_slice()
                .iter()
                .any(|diagnostic| diagnostic.message.contains("`[[type.tag-namespace]]`"))
        );
    }

    #[test]
    fn two_namespaces_sharing_a_prefix_point_at_both() {
        let read = read(&typed(concat!(
            "\n  [[type.tag-namespace]]\n  prefix = \"log/\"\n  open = true\n",
            "\n  [[type.tag-namespace]]\n  prefix = \"log/\"\n  open = true\n",
        )));
        assert_eq!(ids(&read), ["contract.duplicate-tag-namespace"]);
        assert_eq!(read.diagnostics.as_slice()[0].related.len(), 1);
        assert_eq!(read.namespaces.len(), 1);
    }

    #[test]
    fn declaring_both_a_vocabulary_and_openness_is_invalid() {
        let read = read(&one("  values = [\"workout\"]\n  open = true\n"));
        assert_eq!(ids(&read), ["contract.invalid-tag-namespace"]);
        assert!(read.diagnostics.as_slice()[0].help.is_some());
        assert!(read.namespaces.is_empty());
    }

    #[test]
    fn declaring_neither_a_vocabulary_nor_openness_is_invalid() {
        assert_eq!(ids(&read(&one(""))), ["contract.invalid-tag-namespace"]);
    }

    #[test]
    fn open_false_declares_neither_rather_than_an_open_namespace() {
        // `[lifecycle.ordinary]`'s own model: the branch is on presence, and a
        // key written `false` is not a declaration.
        let read = read(&one("  open = false\n"));
        assert_eq!(ids(&read), ["contract.invalid-tag-namespace"]);
        assert!(
            read.diagnostics.as_slice()[0]
                .message
                .contains("something other than `true`")
        );
    }

    #[test]
    fn a_vocabulary_beside_open_false_declares_both_rather_than_a_closed_one() {
        let read = read(&one("  values = [\"workout\"]\n  open = false\n"));
        assert_eq!(ids(&read), ["contract.invalid-tag-namespace"]);
        assert!(
            read.diagnostics.as_slice()[0]
                .message
                .contains("exactly one")
        );
    }

    #[test]
    fn a_non_boolean_open_is_the_namespaces_own_fault() {
        assert_eq!(
            ids(&read(&one("  open = \"yes\"\n"))),
            ["contract.invalid-tag-namespace"]
        );
    }

    #[test]
    fn values_that_are_not_an_array_are_reported_as_a_wrong_type() {
        assert_eq!(
            ids(&read(&one("  values = \"workout\"\n"))),
            ["contract.value-wrong-type"]
        );
    }

    #[test]
    fn an_empty_vocabulary_is_invalid() {
        let read = read(&one("  values = []\n"));
        assert_eq!(ids(&read), ["contract.invalid-tag-namespace"]);
        assert!(read.diagnostics.as_slice()[0].message.contains("empty"));
    }

    #[test]
    fn a_non_string_member_is_invalid() {
        assert_eq!(
            ids(&read(&one("  values = [\"workout\", 2]\n"))),
            ["contract.invalid-tag-namespace"]
        );
    }

    #[test]
    fn a_repeated_member_is_invalid() {
        let read = read(&one("  values = [\"workout\", \"workout\"]\n"));
        assert_eq!(ids(&read), ["contract.invalid-tag-namespace"]);
        assert!(read.diagnostics.as_slice()[0].message.contains("twice"));
        assert!(repeated(&["a".to_owned(), "b".to_owned()]).is_none());
    }
}
