//! Reading `[[type]]`, `[[type.property]]`, and `[[type.relationship]]`.
//!
//! Declaration order is the order the contract writes, and it survives to the
//! model unchanged. Only the unknown-key sweep runs in key order, because
//! `DeTable` iterates lexicographically — which is deterministic, and is what
//! makes a diagnostic list reproducible before it is ever sorted.
//!
//! A repeat is caught here rather than in [`super::validate`] because catching
//! it needs *two* spans — the repeat and the declaration it repeats — and the
//! resolved model keeps only one of each name. The same is true of a property
//! name declaring two different kinds on two types.

use core::ops::Range;

use toml::Spanned;
use toml::de::{DeTable, DeValue};

use crate::diagnostic::KernelDiagnostic;
use crate::document;

use super::model::{
    Capability, PropertyDecl, PropertyKind, RelationshipDecl, ScalarKind, TypeDecl,
};
use super::sink::{Claim, KeyPath, Repeat, Report, Section, Seen, Sink};

/// Every key `[[type]]` defines at contract version 1.
const TYPE_KEYS: &[&str] = &["capabilities", "name", "property", "relationship"];

/// Every key `[[type.relationship]]` defines at contract version 1.
const RELATIONSHIP_KEYS: &[&str] = &["predicate", "required"];

const NO_TYPES: &str = "the contract declares no type";

const NO_TYPES_HELP: &str =
    "a contract declares at least one type, and exactly one of them carries `catch-all`";

/// Reads every `[[type]]` the contract declares, in declaration order.
pub(crate) fn types(sink: &mut Sink<'_>, root: &DeTable<'_>) -> Vec<TypeDecl> {
    match document::get(root, "type") {
        None => {
            no_types(sink);
            Vec::new()
        }
        Some(value) => declared_types(sink, value),
    }
}

fn no_types(sink: &mut Sink<'_>) {
    let at = sink.whole_file();
    let report = Report::new(NO_TYPES.to_owned()).with_help(NO_TYPES_HELP.to_owned());
    sink.report(KernelDiagnostic::ContractNoTypes, report, at);
}

fn declared_types(sink: &mut Sink<'_>, value: &Spanned<DeValue<'_>>) -> Vec<TypeDecl> {
    let Some(array) = sink.array(value, "type") else {
        return Vec::new();
    };
    if array.is_empty() {
        no_types(sink);
        return Vec::new();
    }
    let mut catalog = Catalog::new();
    array
        .iter()
        .filter_map(|entry| declared_type(sink, entry, &mut catalog))
        .collect()
}

fn declared_type(
    sink: &mut Sink<'_>,
    value: &Spanned<DeValue<'_>>,
    catalog: &mut Catalog,
) -> Option<TypeDecl> {
    let table = sink.table(value, "type")?;
    let entry = Section {
        table,
        span: value.span(),
        label: "`[[type]]`".to_owned(),
        path: KeyPath::nameless(),
    };
    let named = sink.name_of(&entry, "name");
    let root = KeyPath::root();
    let section = refine(
        entry,
        Naming {
            parent: &root,
            collection: "type",
            name: named.as_ref().map(|found| found.text),
        },
    );
    sink.sweep(&section, TYPE_KEYS);
    let declared = TypeDecl {
        name: named
            .as_ref()
            .map_or_else(String::new, |f| f.text.to_owned()),
        capabilities: capabilities(sink, &section),
        properties: properties(sink, &section, &mut catalog.kinds),
        relationships: relationships(sink, &section),
    };
    let named = named?;
    sink.written(section.leaf("name").key, named.span.clone());
    let claim = Claim {
        message: format!("two types share the name `{}`", named.text),
        kind: KernelDiagnostic::ContractDuplicateType,
        named,
    };
    sink.keep(&mut catalog.types, claim).map(|()| declared)
}

/// The capabilities a type declares, defaulting to none at all.
fn capabilities(sink: &mut Sink<'_>, section: &Section<'_, '_>) -> Vec<Capability> {
    let leaf = section.leaf("capabilities");
    let Some(value) = section.get("capabilities") else {
        sink.defaulted(leaf.key);
        return Vec::new();
    };
    let Some(array) = sink.array(value, "capabilities") else {
        return Vec::new();
    };
    sink.written(leaf.key, value.span());
    array
        .iter()
        .filter_map(|entry| capability(sink, entry))
        .collect()
}

fn capability(sink: &mut Sink<'_>, value: &Spanned<DeValue<'_>>) -> Option<Capability> {
    // The provenance of the array as a whole is recorded by the caller, so this
    // read deliberately addresses nothing and records nothing.
    let spelled = sink.string(value, KeyPath::nameless().leaf("capabilities"))?;
    let found = Capability::named(spelled);
    if found.is_none() {
        let message = format!("`{spelled}` is not a capability this contract version defines");
        let report = Report::new(message).with_help(listing(
            "the capabilities are",
            Capability::ALL.iter().map(|kind| kind.as_str()),
        ));
        let at = sink.location(value.span());
        sink.report(KernelDiagnostic::ContractUnknownCapability, report, at);
    }
    found
}

/// The properties a type declares, in declaration order.
fn properties(
    sink: &mut Sink<'_>,
    section: &Section<'_, '_>,
    kinds: &mut Kinds,
) -> Vec<PropertyDecl> {
    let Some(array) = collection(sink, section, "property") else {
        return Vec::new();
    };
    let mut scope = Scope {
        parent: &section.path,
        seen: Seen::new(),
    };
    let mut declared = Vec::new();
    for entry in array {
        if let Some(property) = declared_property(sink, entry, &mut scope) {
            conflicting_kind(sink, kinds, &property);
            declared.push(property.decl);
        }
    }
    declared
}

fn declared_property(
    sink: &mut Sink<'_>,
    value: &Spanned<DeValue<'_>>,
    scope: &mut Scope<'_>,
) -> Option<Declared> {
    let table = sink.table(value, "property")?;
    let entry = Section {
        table,
        span: value.span(),
        label: "`[[type.property]]`".to_owned(),
        path: KeyPath::nameless(),
    };
    let named = sink.name_of(&entry, "name");
    let section = refine(
        entry,
        Naming {
            parent: scope.parent,
            collection: "property",
            name: named.as_ref().map(|found| found.text),
        },
    );
    let declared = spelled_kind(sink, &section);
    sink.sweep(&section, property_keys(declared.spelled));
    let required = sink.optional_flag(&section, "required");
    let kind = property_kind(sink, &section, &declared)?;
    let named = named?;
    sink.written(section.leaf("name").key, named.span.clone());
    let name = named.text.to_owned();
    let claim = Claim {
        message: format!("the type declares two properties named `{name}`"),
        kind: KernelDiagnostic::ContractDuplicateProperty,
        named,
    };
    sink.keep(&mut scope.seen, claim)?;
    Some(Declared {
        decl: PropertyDecl {
            name,
            kind,
            required,
        },
        at: declared.at,
    })
}

/// The `kind` a property spells, and where it spells it.
struct Spelled<'a> {
    spelled: Option<&'a str>,
    at: Range<usize>,
}

fn spelled_kind<'a>(sink: &mut Sink<'_>, section: &Section<'a, '_>) -> Spelled<'a> {
    let value = sink.required(section, "kind");
    Spelled {
        at: value.map_or_else(|| section.span.clone(), Spanned::span),
        spelled: value.and_then(|value| sink.string(value, section.leaf("kind"))),
    }
}

/// Which keys a property declaration may carry, which depends on the kind it
/// declares: `values` belongs to an `enum` and `of` to a `list`.
///
/// A property whose kind did not resolve is allowed both, so an unknown kind
/// produces one diagnostic rather than three.
fn property_keys(spelled: Option<&str>) -> &'static [&'static str] {
    match spelled {
        Some("enum") => &["kind", "name", "required", "values"],
        Some("list") => &["kind", "name", "of", "required"],
        Some(other) if ScalarKind::named(other).is_some() => &["kind", "name", "required"],
        _ => &["kind", "name", "of", "required", "values"],
    }
}

fn property_kind(
    sink: &mut Sink<'_>,
    section: &Section<'_, '_>,
    declared: &Spelled<'_>,
) -> Option<PropertyKind> {
    match declared.spelled? {
        "enum" => enum_values(sink, section).map(|values| PropertyKind::Enum { values }),
        "list" => list_of(sink, section).map(|of| PropertyKind::List { of }),
        other => scalar_kind(sink, other, declared.at.clone()),
    }
}

fn scalar_kind(sink: &mut Sink<'_>, spelled: &str, at: Range<usize>) -> Option<PropertyKind> {
    let found = ScalarKind::named(spelled).map(PropertyKind::from);
    if found.is_none() {
        let message = format!("`{spelled}` is not a value kind this contract version defines");
        let names = ScalarKind::ALL.iter().map(|kind| kind.as_str());
        let report =
            Report::new(message).with_help(listing("the kinds are", names.chain(["enum", "list"])));
        let at = sink.location(at);
        sink.report(KernelDiagnostic::ContractUnknownPropertyKind, report, at);
    }
    found
}

/// The members of an `enum`: present, non-empty, all strings, no repeats.
fn enum_values(sink: &mut Sink<'_>, section: &Section<'_, '_>) -> Option<Vec<String>> {
    let Some(value) = section.get("values") else {
        invalid_enum(sink, "declares `kind = \"enum\"` but no `values`", None);
        return None;
    };
    let at = value.span();
    let members: Option<Vec<String>> = sink
        .array(value, "values")?
        .iter()
        .map(|member| document::expect_string(member).ok().map(str::to_owned))
        .collect();
    let Some(members) = members else {
        invalid_enum(
            sink,
            "declares a `values` entry that is not a string",
            Some(at),
        );
        return None;
    };
    check_members(sink, &members, at.clone())?;
    sink.written(section.leaf("values").key, at);
    Some(members)
}

fn check_members(sink: &mut Sink<'_>, members: &[String], at: Range<usize>) -> Option<()> {
    if members.is_empty() {
        invalid_enum(sink, "declares an empty `values`", Some(at));
        return None;
    }
    let mut seen = Seen::new();
    let repeat = members
        .iter()
        .find(|member| seen.claim(member.as_str(), at.clone()).is_some());
    let Some(repeat) = repeat else {
        return Some(());
    };
    let message = format!("declares `{repeat}` twice in `values`");
    invalid_enum(sink, &message, Some(at));
    None
}

fn invalid_enum(sink: &mut Sink<'_>, detail: &str, at: Option<Range<usize>>) {
    let report = Report::new(format!("a property {detail}"))
        .with_help("an `enum` declares a non-empty list of unique strings".to_owned());
    let at = at.map_or_else(|| sink.whole_file(), |span| sink.location(span));
    sink.report(KernelDiagnostic::ContractInvalidEnumValues, report, at);
}

/// The element kind of a `list`, which is one of the six scalar kinds. Lists do
/// not nest, and there is no list of `enum`.
fn list_of(sink: &mut Sink<'_>, section: &Section<'_, '_>) -> Option<ScalarKind> {
    let Some(value) = section.get("of") else {
        invalid_list(
            sink,
            "declares `kind = \"list\"` but no `of`".to_owned(),
            None,
        );
        return None;
    };
    let at = value.span();
    let spelled = sink.string(value, section.leaf("of"))?;
    let found = ScalarKind::named(spelled);
    if found.is_none() {
        let message = format!("declares `of = \"{spelled}\"`, which is not a scalar kind");
        invalid_list(sink, message, Some(at));
    }
    found
}

fn invalid_list(sink: &mut Sink<'_>, detail: String, at: Option<Range<usize>>) {
    let report = Report::new(format!("a property {detail}")).with_help(listing(
        "a `list` holds one of",
        ScalarKind::ALL.iter().map(|kind| kind.as_str()),
    ));
    let at = at.map_or_else(|| sink.whole_file(), |span| sink.location(span));
    sink.report(KernelDiagnostic::ContractInvalidListOf, report, at);
}

/// The relationships a type declares, in declaration order.
fn relationships(sink: &mut Sink<'_>, section: &Section<'_, '_>) -> Vec<RelationshipDecl> {
    let Some(array) = collection(sink, section, "relationship") else {
        return Vec::new();
    };
    let mut scope = Scope {
        parent: &section.path,
        seen: Seen::new(),
    };
    array
        .iter()
        .filter_map(|entry| declared_relationship(sink, entry, &mut scope))
        .collect()
}

fn declared_relationship(
    sink: &mut Sink<'_>,
    value: &Spanned<DeValue<'_>>,
    scope: &mut Scope<'_>,
) -> Option<RelationshipDecl> {
    let table = sink.table(value, "relationship")?;
    let entry = Section {
        table,
        span: value.span(),
        label: "`[[type.relationship]]`".to_owned(),
        path: KeyPath::nameless(),
    };
    let named = sink.name_of(&entry, "predicate");
    let section = refine(
        entry,
        Naming {
            parent: scope.parent,
            collection: "relationship",
            name: named.as_ref().map(|found| found.text),
        },
    );
    sink.sweep(&section, RELATIONSHIP_KEYS);
    let required = sink.optional_flag(&section, "required");
    let named = named?;
    sink.written(section.leaf("predicate").key, named.span.clone());
    let predicate = named.text.to_owned();
    let claim = Claim {
        message: format!("the type declares two relationships with the predicate `{predicate}`"),
        kind: KernelDiagnostic::ContractDuplicatePredicate,
        named,
    };
    sink.keep(&mut scope.seen, claim)?;
    Some(RelationshipDecl {
        predicate,
        required,
    })
}

/// The array of tables a type declares under `key`, when it declares one.
fn collection<'a, 'i>(
    sink: &mut Sink<'_>,
    section: &Section<'a, 'i>,
    key: &'static str,
) -> Option<&'a [Spanned<DeValue<'i>>]> {
    let value = section.get(key)?;
    Some(sink.array(value, key)?.as_ref())
}

/// How a nested declaration is addressed and named once its own name is read.
struct Naming<'a> {
    parent: &'a KeyPath,
    collection: &'static str,
    name: Option<&'a str>,
}

fn refine<'a, 'i>(entry: Section<'a, 'i>, naming: Naming<'_>) -> Section<'a, 'i> {
    let label = naming.name.map_or_else(
        || entry.label.clone(),
        |name| format!("the {} `{name}`", naming.collection),
    );
    Section {
        label,
        path: naming
            .parent
            .child(naming.collection)
            .child_opt(naming.name),
        ..entry
    }
}

/// One property, with where its `kind` is written, so a conflict with another
/// type's declaration of the same name can point at both.
struct Declared {
    decl: PropertyDecl,
    at: Range<usize>,
}

/// The names one enclosing scope has already claimed.
struct Scope<'a> {
    parent: &'a KeyPath,
    seen: Seen,
}

/// The kind each property name has been declared with, so far.
struct Kinds {
    entries: Vec<(String, PropertyKind, Range<usize>)>,
}

impl Kinds {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// The earlier, different declaration of this property's kind, if any.
    fn conflict(&mut self, declared: &Declared) -> Option<(PropertyKind, Range<usize>)> {
        let name = declared.decl.name();
        let earlier = self
            .entries
            .iter()
            .find(|(seen, _, _)| seen == name)
            .map(|(_, kind, first)| (kind.clone(), first.clone()));
        let Some((kind, first)) = earlier else {
            self.entries.push((
                name.to_owned(),
                declared.decl.kind().clone(),
                declared.at.clone(),
            ));
            return None;
        };
        (&kind != declared.decl.kind()).then_some((kind, first))
    }
}

/// One property name declares one kind corpus-wide, so a corpus-wide question
/// about it has one answer.
fn conflicting_kind(sink: &mut Sink<'_>, kinds: &mut Kinds, declared: &Declared) {
    let Some((first_kind, first)) = kinds.conflict(declared) else {
        return;
    };
    sink.repeated(
        KernelDiagnostic::ContractPropertyKindConflict,
        Repeat {
            message: format!(
                "the property `{}` is declared as {} here and as {} on an earlier type",
                declared.decl.name(),
                declared.decl.kind().describe(),
                first_kind.describe()
            ),
            at: declared.at.clone(),
            first,
        },
    );
}

/// Everything a `[[type]]` walk has claimed so far.
struct Catalog {
    types: Seen,
    kinds: Kinds,
}

impl Catalog {
    fn new() -> Self {
        Self {
            types: Seen::new(),
            kinds: Kinds::new(),
        }
    }
}

/// A help line listing a closed set, so the set is never restated by hand.
fn listing<'a>(lead: &str, names: impl Iterator<Item = &'a str>) -> String {
    let quoted: Vec<String> = names.map(|name| format!("`{name}`")).collect();
    format!("{lead} {}", quoted.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::sink::tests::{root_of, text_of};
    use crate::diagnostic::DiagnosticList;
    use crate::provenance::{Provenance, Source};

    struct Read {
        types: Vec<TypeDecl>,
        diagnostics: DiagnosticList,
        provenance: Provenance,
    }

    fn read(source: &str) -> Read {
        let text = text_of(source);
        let document = root_of(&text);
        let mut sink = Sink::new(&text, 1);
        let types = types(&mut sink, document.get_ref());
        let (diagnostics, provenance) = sink.finish();
        Read {
            types,
            diagnostics,
            provenance,
        }
    }

    fn ids(read: &Read) -> Vec<&str> {
        read.diagnostics
            .as_slice()
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect()
    }

    const PERSON: &str = concat!(
        "[[type]]\n",
        "name = \"person\"\n",
        "capabilities = [\"identity-bearing\"]\n",
        "\n",
        "  [[type.property]]\n",
        "  name = \"full_name\"\n",
        "  kind = \"string\"\n",
        "  required = true\n",
        "\n",
        "  [[type.relationship]]\n",
        "  predicate = \"works-at\"\n",
        "  required = false\n",
    );

    #[test]
    fn a_type_reads_with_its_properties_and_relationships_in_order() {
        let read = read(PERSON);
        assert!(read.diagnostics.is_empty());
        let declared = &read.types[0];
        assert_eq!(declared.name(), "person");
        assert_eq!(declared.capabilities(), [Capability::IdentityBearing]);
        assert_eq!(declared.properties()[0].name(), "full_name");
        assert_eq!(declared.relationships()[0].predicate(), "works-at");
    }

    #[test]
    fn every_leaf_of_a_type_records_where_it_is_written() {
        let read = read(PERSON);
        let keys: Vec<&str> = read
            .provenance
            .entries()
            .map(|entry| entry.key.as_str())
            .collect();
        assert_eq!(
            keys,
            [
                "type.person.capabilities",
                "type.person.name",
                "type.person.property.full_name.kind",
                "type.person.property.full_name.name",
                "type.person.property.full_name.required",
                "type.person.relationship.works-at.predicate",
                "type.person.relationship.works-at.required",
            ]
        );
    }

    #[test]
    fn an_omitted_optional_leaf_is_attributed_to_the_contract_version() {
        let read = read("[[type]]\nname = \"capture\"\n");
        let capabilities = read
            .provenance
            .get("type.capture.capabilities")
            .expect("recorded");
        assert_eq!(
            capabilities.source,
            Source::Default {
                contract_version: 1
            }
        );
        assert!(capabilities.location.is_none());
        assert_eq!(read.types[0].capabilities(), []);
    }

    #[test]
    fn declaration_order_is_never_sorted() {
        let read = read("[[type]]\nname = \"zeta\"\n\n[[type]]\nname = \"alpha\"\n");
        let names: Vec<&str> = read.types.iter().map(TypeDecl::name).collect();
        assert_eq!(names, ["zeta", "alpha"]);
    }

    #[test]
    fn a_contract_declaring_no_type_says_so_once() {
        assert_eq!(ids(&read("contract_version = 1\n")), ["contract.no-types"]);
        assert_eq!(ids(&read("type = []\n")), ["contract.no-types"]);
    }

    #[test]
    fn a_type_array_of_the_wrong_type_is_reported_as_a_wrong_type() {
        assert_eq!(
            ids(&read("type = \"person\"\n")),
            ["contract.value-wrong-type"]
        );
    }

    #[test]
    fn a_type_entry_that_is_not_a_table_is_reported_as_a_wrong_type() {
        assert_eq!(
            ids(&read("type = [\"person\"]\n")),
            ["contract.value-wrong-type"]
        );
    }

    #[test]
    fn an_unknown_key_on_a_type_is_fatal() {
        let read = read("[[type]]\nname = \"a\"\ncolour = \"red\"\n");
        assert_eq!(ids(&read), ["contract.unknown-key"]);
        assert!(
            read.diagnostics.as_slice()[0]
                .message
                .contains("the type `a`")
        );
    }

    #[test]
    fn a_type_without_a_name_is_reported_and_still_yields_its_own_faults() {
        let read = read("[[type]]\ncapabilities = [\"nope\"]\n");
        assert_eq!(
            ids(&read),
            ["contract.missing-key", "contract.unknown-capability"]
        );
        assert!(read.types.is_empty());
        assert!(
            read.provenance.is_empty(),
            "nothing under it is addressable"
        );
    }

    #[test]
    fn a_non_string_type_name_is_reported_as_a_wrong_type() {
        assert_eq!(
            ids(&read("[[type]]\nname = 4\n")),
            ["contract.value-wrong-type"]
        );
    }

    #[test]
    fn two_types_sharing_a_name_point_at_both() {
        let read = read("[[type]]\nname = \"a\"\n\n[[type]]\nname = \"a\"\n");
        assert_eq!(ids(&read), ["contract.duplicate-type"]);
        assert_eq!(read.diagnostics.as_slice()[0].related.len(), 1);
        assert_eq!(read.types.len(), 1);
    }

    #[test]
    fn an_unknown_capability_names_the_closed_set() {
        let read = read("[[type]]\nname = \"a\"\ncapabilities = [\"writeable\"]\n");
        assert_eq!(ids(&read), ["contract.unknown-capability"]);
        let help = read.diagnostics.as_slice()[0].help.as_deref();
        assert_eq!(
            help,
            Some("the capabilities are `identity-bearing`, `catch-all`, `closed-write`")
        );
    }

    #[test]
    fn capabilities_of_the_wrong_shape_are_reported_as_wrong_types() {
        assert_eq!(
            ids(&read(
                "[[type]]\nname = \"a\"\ncapabilities = \"catch-all\"\n"
            )),
            ["contract.value-wrong-type"]
        );
        assert_eq!(
            ids(&read("[[type]]\nname = \"a\"\ncapabilities = [1]\n")),
            ["contract.value-wrong-type"]
        );
    }

    #[test]
    fn a_property_of_every_scalar_kind_reads() {
        let mut source = "[[type]]\nname = \"a\"\n".to_owned();
        for kind in ScalarKind::ALL {
            source.push_str(&format!(
                "\n[[type.property]]\nname = \"p_{kind}\"\nkind = \"{kind}\"\n"
            ));
        }
        let read = read(&source);
        assert!(read.diagnostics.is_empty());
        assert_eq!(read.types[0].properties().len(), 6);
        assert_eq!(read.types[0].properties()[4].kind(), &PropertyKind::Date);
    }

    #[test]
    fn an_omitted_required_defaults_to_false() {
        let read =
            read("[[type]]\nname = \"a\"\n\n[[type.property]]\nname = \"p\"\nkind = \"string\"\n");
        assert!(!read.types[0].properties()[0].required());
        assert_eq!(
            read.provenance
                .get("type.a.property.p.required")
                .map(|entry| entry.source),
            Some(Source::Default {
                contract_version: 1
            })
        );
    }

    #[test]
    fn an_unknown_property_kind_names_the_closed_lattice() {
        let read =
            read("[[type]]\nname = \"a\"\n\n[[type.property]]\nname = \"p\"\nkind = \"url\"\n");
        assert_eq!(ids(&read), ["contract.unknown-property-kind"]);
        let help = read.diagnostics.as_slice()[0]
            .help
            .as_deref()
            .expect("advice");
        assert!(help.ends_with("`datetime`, `enum`, `list`"));
    }

    #[test]
    fn a_property_without_a_kind_is_a_missing_key() {
        let read = read("[[type]]\nname = \"a\"\n\n[[type.property]]\nname = \"p\"\n");
        assert_eq!(ids(&read), ["contract.missing-key"]);
    }

    #[test]
    fn a_property_without_a_name_is_a_missing_key() {
        let read = read("[[type]]\nname = \"a\"\n\n[[type.property]]\nkind = \"string\"\n");
        assert_eq!(ids(&read), ["contract.missing-key"]);
        assert!(read.types[0].properties().is_empty());
    }

    #[test]
    fn two_properties_on_one_type_sharing_a_name_point_at_both() {
        let source = concat!(
            "[[type]]\nname = \"a\"\n",
            "\n[[type.property]]\nname = \"p\"\nkind = \"string\"\n",
            "\n[[type.property]]\nname = \"p\"\nkind = \"string\"\n",
        );
        let read = read(source);
        assert_eq!(ids(&read), ["contract.duplicate-property"]);
        assert_eq!(read.diagnostics.as_slice()[0].related.len(), 1);
    }

    #[test]
    fn one_property_name_declares_one_kind_across_types() {
        let source = concat!(
            "[[type]]\nname = \"a\"\n",
            "\n[[type.property]]\nname = \"p\"\nkind = \"string\"\n",
            "\n[[type]]\nname = \"b\"\n",
            "\n[[type.property]]\nname = \"p\"\nkind = \"integer\"\n",
        );
        let read = read(source);
        assert_eq!(ids(&read), ["contract.property-kind-conflict"]);
        assert!(
            read.diagnostics.as_slice()[0]
                .message
                .contains("`integer` here and as `string`")
        );
    }

    #[test]
    fn two_enums_over_different_values_are_a_kind_conflict() {
        let source = concat!(
            "[[type]]\nname = \"a\"\n",
            "\n[[type.property]]\nname = \"p\"\nkind = \"enum\"\nvalues = [\"x\"]\n",
            "\n[[type]]\nname = \"b\"\n",
            "\n[[type.property]]\nname = \"p\"\nkind = \"enum\"\nvalues = [\"y\"]\n",
        );
        assert_eq!(ids(&read(source)), ["contract.property-kind-conflict"]);
    }

    #[test]
    fn one_property_name_repeated_with_the_same_kind_is_fine() {
        let source = concat!(
            "[[type]]\nname = \"a\"\n",
            "\n[[type.property]]\nname = \"p\"\nkind = \"boolean\"\n",
            "\n[[type]]\nname = \"b\"\n",
            "\n[[type.property]]\nname = \"p\"\nkind = \"boolean\"\n",
        );
        assert!(read(source).diagnostics.is_empty());
    }

    fn property(kind: &str, extra: &str) -> String {
        format!(
            "[[type]]\nname = \"a\"\n\n[[type.property]]\nname = \"p\"\nkind = \"{kind}\"\n{extra}"
        )
    }

    #[test]
    fn an_enum_declares_a_non_empty_list_of_unique_strings() {
        let read = read(&property("enum", "values = [\"draft\", \"archived\"]\n"));
        assert!(read.diagnostics.is_empty());
        assert_eq!(
            read.types[0].properties()[0].kind().values(),
            Some(&["draft".to_owned(), "archived".to_owned()][..])
        );
    }

    #[test]
    fn an_enum_without_values_is_invalid() {
        let read = read(&property("enum", ""));
        assert_eq!(ids(&read), ["contract.invalid-enum-values"]);
        assert!(read.diagnostics.as_slice()[0].location.is_some());
    }

    #[test]
    fn an_empty_enum_is_invalid() {
        assert_eq!(
            ids(&read(&property("enum", "values = []\n"))),
            ["contract.invalid-enum-values"]
        );
    }

    #[test]
    fn a_non_string_enum_member_is_invalid() {
        assert_eq!(
            ids(&read(&property("enum", "values = [\"a\", 2]\n"))),
            ["contract.invalid-enum-values"]
        );
    }

    #[test]
    fn a_repeated_enum_member_is_invalid() {
        let read = read(&property("enum", "values = [\"a\", \"a\"]\n"));
        assert_eq!(ids(&read), ["contract.invalid-enum-values"]);
        assert!(read.diagnostics.as_slice()[0].message.contains("twice"));
    }

    #[test]
    fn values_that_are_not_an_array_are_reported_as_a_wrong_type() {
        assert_eq!(
            ids(&read(&property("enum", "values = \"draft\"\n"))),
            ["contract.value-wrong-type"]
        );
    }

    #[test]
    fn a_list_declares_a_scalar_element_kind() {
        let read = read(&property("list", "of = \"string\"\n"));
        assert!(read.diagnostics.is_empty());
        assert_eq!(
            read.types[0].properties()[0].kind().element(),
            Some(ScalarKind::String)
        );
    }

    #[test]
    fn a_list_without_an_of_is_invalid() {
        assert_eq!(
            ids(&read(&property("list", ""))),
            ["contract.invalid-list-of"]
        );
    }

    #[test]
    fn a_list_may_not_nest_and_may_not_hold_an_enum() {
        assert_eq!(
            ids(&read(&property("list", "of = \"list\"\n"))),
            ["contract.invalid-list-of"]
        );
        assert_eq!(
            ids(&read(&property("list", "of = \"enum\"\n"))),
            ["contract.invalid-list-of"]
        );
    }

    #[test]
    fn a_non_string_of_is_reported_as_a_wrong_type() {
        assert_eq!(
            ids(&read(&property("list", "of = 1\n"))),
            ["contract.value-wrong-type"]
        );
    }

    #[test]
    fn values_are_illegal_outside_an_enum_and_of_outside_a_list() {
        assert_eq!(
            ids(&read(&property("string", "values = [\"a\"]\n"))),
            ["contract.unknown-key"]
        );
        assert_eq!(
            ids(&read(&property(
                "enum",
                "values = [\"a\"]\nof = \"string\"\n"
            ))),
            ["contract.unknown-key"]
        );
    }

    #[test]
    fn an_unresolved_kind_does_not_also_complain_about_values() {
        assert_eq!(
            ids(&read(&property(
                "url",
                "values = [\"a\"]\nof = \"string\"\n"
            ))),
            ["contract.unknown-property-kind"]
        );
    }

    #[test]
    fn a_relationship_declares_a_predicate_and_whether_it_is_required() {
        let source = "[[type]]\nname = \"a\"\n\n[[type.relationship]]\npredicate = \"cites\"\nrequired = true\n";
        let read = read(source);
        assert!(read.diagnostics.is_empty());
        assert!(read.types[0].relationships()[0].required());
    }

    #[test]
    fn a_relationship_without_a_predicate_is_a_missing_key() {
        let read = read("[[type]]\nname = \"a\"\n\n[[type.relationship]]\nrequired = true\n");
        assert_eq!(ids(&read), ["contract.missing-key"]);
        assert!(read.types[0].relationships().is_empty());
    }

    #[test]
    fn two_relationships_sharing_a_predicate_point_at_both() {
        let source = concat!(
            "[[type]]\nname = \"a\"\n",
            "\n[[type.relationship]]\npredicate = \"cites\"\n",
            "\n[[type.relationship]]\npredicate = \"cites\"\n",
        );
        let read = read(source);
        assert_eq!(ids(&read), ["contract.duplicate-predicate"]);
        assert_eq!(read.types[0].relationships().len(), 1);
    }

    #[test]
    fn there_is_no_targets_key_on_a_relationship() {
        let source = "[[type]]\nname = \"a\"\n\n[[type.relationship]]\npredicate = \"cites\"\ntargets = [\"a\"]\n";
        let read = read(source);
        assert_eq!(ids(&read), ["contract.unknown-key"]);
        assert!(read.diagnostics.as_slice()[0].message.contains("`targets`"));
    }

    #[test]
    fn a_collection_of_the_wrong_shape_is_reported_once_each() {
        assert_eq!(
            ids(&read("[[type]]\nname = \"a\"\nproperty = 1\n")),
            ["contract.value-wrong-type"]
        );
        assert_eq!(
            ids(&read("[[type]]\nname = \"a\"\nrelationship = [1]\n")),
            ["contract.value-wrong-type"]
        );
    }

    #[test]
    fn a_help_line_lists_a_closed_set_without_restating_it() {
        assert_eq!(listing("one of", ["a", "b"].into_iter()), "one of `a`, `b`");
    }
}
