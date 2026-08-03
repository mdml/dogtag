//! The validity rules enforced when a contract loads.
//!
//! These are the difference between a contract that *parses* and a contract
//! that is **correct**. Without them a contract declaring an empty `enum`, or
//! no catch-all, or a lifecycle axis that is not a property, loads with zero
//! diagnostics and is then rendered to an agent as the vault's rules.
//!
//! Every rule here reasons over **declarations**. None of them knows a
//! vocabulary word: the ordinary state is checkable because the contract says
//! which property carries the axis and how each type requires it, never because
//! the kernel recognizes a name.
//!
//! The rules that need *two* spans — a repeat and what it repeats — run during
//! the walk instead, in [`super::declarations`], because the resolved model
//! keeps only one declaration per name. The rules here point at a value through
//! the provenance already recorded for it, which is what keeps a rule from
//! carrying its own parallel span table.

use crate::diagnostic::{Diagnostic, KernelDiagnostic, Location, Related};

use super::model::{
    Capability, FlagDecl, LifecycleDecl, Ordinary, PropertyDecl, PropertyKind, ScalarKind, TypeDecl,
};
use super::parse::Parts;
use super::sink::{Report, Sink};

/// Applies every load-time validity rule to what the walk resolved.
pub(crate) fn run(sink: &mut Sink<'_>, parts: &Parts) {
    capabilities(sink, &parts.types);
    // The rules below conclude from the model's silence that no type declares
    // something. That inference is only sound when the model holds every
    // declaration the file makes; when the walk dropped one, the silence is
    // the parser's and the conclusion would contradict the file. The fault
    // that caused the drop is already reported, so nothing goes unsaid.
    if !sink.complete() {
        return;
    }
    flags(sink, &parts.flags, &parts.types);
    tags(sink, parts);
    if let Some(declared) = &parts.lifecycle {
        lifecycle(sink, declared, &parts.types);
    }
}

/// Where a leaf is written, or the file when nothing recorded it.
fn located(sink: &Sink<'_>, key: &str) -> Location {
    sink.provenance()
        .get(key)
        .and_then(|entry| entry.location.clone())
        .unwrap_or_else(|| sink.whole_file())
}

/// The kind a property name is declared with, taking the first declaration —
/// which answers for all of them, because a conflict is its own diagnostic.
fn kind_of<'a>(types: &'a [TypeDecl], name: &str) -> Option<&'a PropertyKind> {
    declarers(types, name)
        .next()
        .map(|(_, property)| property.kind())
}

/// Every type declaring a property named `name`, with its own declaration.
fn declarers<'a>(
    types: &'a [TypeDecl],
    name: &str,
) -> impl Iterator<Item = (&'a TypeDecl, &'a PropertyDecl)> {
    types
        .iter()
        .filter_map(move |declared| Some((declared, declared.property(name)?)))
}

/// Catch-all cardinality: exactly one type carries it — and, at a version whose
/// rules say so, what that one type is allowed to declare.
///
/// A contract that resolved no type at all has already been told so, and a
/// second complaint about its capabilities would be one fault reported twice.
fn capabilities(sink: &mut Sink<'_>, types: &[TypeDecl]) {
    if types.is_empty() {
        return;
    }
    let carriers: Vec<&TypeDecl> = types
        .iter()
        .filter(|declared| declared.has(Capability::CatchAll))
        .collect();
    match carriers.split_first() {
        None => missing_catch_all(sink),
        // The requiring rule reads *the* catch-all, so it runs only where the
        // contract has settled on one. Adding it to a cardinality fault would
        // report one broken contract twice and pass judgement on a type the
        // file has not yet said is the catch-all — the same "one fault, one
        // diagnostic" posture the guard below this match keeps.
        Some((only, [])) => catch_all_requires(sink, only),
        Some((first, rest)) => multiple_catch_all(sink, first, rest),
    }
}

fn missing_catch_all(sink: &mut Sink<'_>) {
    let at = sink.whole_file();
    let report = Report::new("no type declares the catch-all capability".to_owned()).with_help(
        "exactly one type carries `catch-all`, so capture never blocks on classification"
            .to_owned(),
    );
    sink.report(KernelDiagnostic::ContractMissingCatchAll, report, at);
}

fn multiple_catch_all(sink: &mut Sink<'_>, first: &TypeDecl, rest: &[&TypeDecl]) {
    let message = format!("{} types declare the catch-all capability", rest.len() + 1);
    let mut diagnostic = Diagnostic::kernel(KernelDiagnostic::ContractMultipleCatchAll, message)
        .at(capability_site(sink, first))
        .with_help("exactly one type may declare `catch-all`");
    for other in rest {
        let note = format!("the type `{}` also declares it", other.name());
        diagnostic = diagnostic.with_related(Related::new(note).at(capability_site(sink, other)));
    }
    sink.push(diagnostic);
}

fn capability_site(sink: &Sink<'_>, declared: &TypeDecl) -> Location {
    located(sink, &format!("type.{}.capabilities", declared.name()))
}

/// The catch-all type requires nothing, at a version whose rules say so.
///
/// Every untyped note binds to the catch-all, so a requiring catch-all has
/// `contract explain` render "accepts anything" beside requirements every
/// untyped note instantly fails. The rule is version-scoped rather than
/// universal: a version-1 contract that loaded clean must keep loading, and a
/// version-1 corpus in this shape collects missing-required findings on its
/// untyped notes instead.
///
/// The record says the diagnostic points at "the capability and the offending
/// declaration". A catch-all requiring three properties has three offending
/// declarations and one fault, so this anchors at the capability — the half
/// that makes a requirement misleading rather than merely strict — and names
/// each requirement as related evidence, the shape [`multiple_catch_all`]
/// already sets for a fault with several occurrences of one cause.
fn catch_all_requires(sink: &mut Sink<'_>, declared: &TypeDecl) {
    if sink.schema().rules.catch_all_may_require {
        return;
    }
    let evidence = requirements(sink, declared);
    if evidence.is_empty() {
        return;
    }
    let message = format!(
        "the catch-all type `{}` requires {} of its declarations",
        declared.name(),
        evidence.len()
    );
    let mut diagnostic = Diagnostic::kernel(KernelDiagnostic::ContractCatchAllRequires, message)
        .at(capability_site(sink, declared))
        .with_help(
            "every untyped note binds to the catch-all, so it can require nothing an untyped \
             note need not carry; move the requirement to a type notes opt into"
                .to_owned(),
        );
    for related in evidence {
        diagnostic = diagnostic.with_related(related);
    }
    sink.push(diagnostic);
}

/// One declaration a type requires: the table that declares it, and its name
/// within that table.
///
/// The table's own name does two jobs, which is what keeps them from drifting
/// apart: it addresses the `required` leaf in provenance, and it names the
/// declaration to the reader.
struct Requirement<'a> {
    table: &'a str,
    name: &'a str,
}

impl Requirement<'_> {
    /// The evidence line naming this requirement, pointing at the leaf that
    /// declares it.
    fn evidence(&self, sink: &Sink<'_>, owner: &TypeDecl) -> Related {
        let Self { table, name } = *self;
        let at = located(
            sink,
            &format!("type.{}.{table}.{name}.required", owner.name()),
        );
        Related::new(format!("`{name}` is a required {table}")).at(at)
    }
}

/// Everything one type requires, named as the evidence that says so:
/// properties, then relationships, then tag namespaces, each in declaration
/// order. Emission order is the contract's own so that the evidence a reader
/// sees does not depend on a walk order the file never shows them.
fn requirements(sink: &Sink<'_>, declared: &TypeDecl) -> Vec<Related> {
    let properties = declared
        .properties()
        .iter()
        .filter(|property| property.required())
        .map(|property| Requirement {
            table: "property",
            name: property.name(),
        });
    let relationships = declared
        .relationships()
        .iter()
        .filter(|relationship| relationship.required())
        .map(|relationship| Requirement {
            table: "relationship",
            name: relationship.predicate(),
        });
    let namespaces = declared
        .tag_namespaces()
        .iter()
        .filter(|namespace| namespace.required())
        .map(|namespace| Requirement {
            table: "tag-namespace",
            name: namespace.prefix(),
        });
    properties
        .chain(relationships)
        .chain(namespaces)
        .map(|found| found.evidence(sink, declared))
        .collect()
}

/// Each flag names a declared property whose kind is `boolean`.
fn flags(sink: &mut Sink<'_>, flags: &[FlagDecl], types: &[TypeDecl]) {
    for flag in flags {
        flag_property(sink, flag, types);
    }
}

fn flag_property(sink: &mut Sink<'_>, flag: &FlagDecl, types: &[TypeDecl]) {
    let name = flag.property();
    let at = located(sink, &format!("flag.{name}.property"));
    let Some(kind) = kind_of(types, name) else {
        let report = Report::new(format!("the flag names `{name}`, which no type declares"))
            .with_help("a flag names a property some type declares".to_owned());
        sink.report(KernelDiagnostic::ContractFlagPropertyUndeclared, report, at);
        return;
    };
    if kind == &PropertyKind::Boolean {
        return;
    }
    let report = Report::new(format!(
        "the flag names `{name}`, whose declared kind is {}",
        kind.describe()
    ))
    .with_help("a flag is a boolean property, so that it cannot be a point on an axis".to_owned());
    sink.report(KernelDiagnostic::ContractFlagPropertyNotBoolean, report, at);
}

/// A type that declares at least one tag namespace, and where its first one is
/// written — which is what every rule about that type's namespaces points at.
struct Carrier<'a> {
    declared: &'a TypeDecl,
    key: String,
}

fn carriers(types: &[TypeDecl]) -> Vec<Carrier<'_>> {
    types
        .iter()
        .filter_map(|declared| {
            let first = declared.tag_namespaces().first()?;
            Some(Carrier {
                declared,
                key: format!(
                    "type.{}.tag-namespace.{}.prefix",
                    declared.name(),
                    first.prefix()
                ),
            })
        })
        .collect()
}

/// The tag vocabulary as the whole-contract rules read it: the property
/// `[tags]` names, and every type that declares a namespace over it.
struct Vocabulary<'a> {
    property: &'a str,
    carriers: Vec<Carrier<'a>>,
}

/// The tag vocabulary reads across the whole contract: a type declaring a
/// namespace declares the property `[tags]` names, and that property is a list
/// of string.
///
/// Bound to the types that declare a namespace, exactly as the record binds it.
/// A `[tags]` table naming a property nothing declares, in a corpus where no
/// type declares a namespace, is not a fault this rule reaches.
fn tags(sink: &mut Sink<'_>, parts: &Parts) {
    let carriers = carriers(&parts.types);
    let Some(declared) = &parts.tags else {
        for carrier in &carriers {
            tags_table_missing(sink, carrier);
        }
        return;
    };
    if carriers.is_empty() {
        return;
    }
    let vocabulary = Vocabulary {
        property: declared.property(),
        carriers,
    };
    for carrier in &vocabulary.carriers {
        tag_property(sink, &vocabulary, carrier);
    }
    tag_property_kind(sink, &vocabulary, &parts.types);
}

fn tags_table_missing(sink: &mut Sink<'_>, carrier: &Carrier<'_>) {
    let at = located(sink, &carrier.key);
    let report = Report::new(format!(
        "the type `{}` declares a tag namespace, and the contract declares no `[tags]` table",
        carrier.declared.name()
    ))
    .with_help(
        "`[tags]` names the property a corpus carries its tags on, and a namespace describes that \
         property's vocabulary"
            .to_owned(),
    );
    sink.report(KernelDiagnostic::ContractTagsTableMissing, report, at);
}

fn tag_property(sink: &mut Sink<'_>, vocabulary: &Vocabulary<'_>, carrier: &Carrier<'_>) {
    let property = vocabulary.property;
    if carrier.declared.property(property).is_some() {
        return;
    }
    let at = located(sink, &carrier.key);
    let report = Report::new(format!(
        "the type `{}` declares a tag namespace but not `{property}`, which `[tags]` names",
        carrier.declared.name()
    ))
    .with_help(
        "a namespace describes the tags a note of its type carries, so the type declares the tag \
         property itself"
            .to_owned(),
    );
    sink.report(KernelDiagnostic::ContractTagPropertyUndeclared, report, at);
}

/// One name declares one kind corpus-wide, so the tag property's shape has one
/// answer and this rule asks it once.
fn tag_property_kind(sink: &mut Sink<'_>, vocabulary: &Vocabulary<'_>, types: &[TypeDecl]) {
    let property = vocabulary.property;
    let Some(kind) = kind_of(types, property) else {
        // No type declares it at all, which the rule above already reported
        // against every type that needed it.
        return;
    };
    if matches!(
        kind,
        PropertyKind::List {
            of: ScalarKind::String
        }
    ) {
        return;
    }
    let at = located(sink, "tags.property");
    let report = Report::new(format!(
        "`[tags]` names `{property}`, whose declared kind is {}",
        kind.describe()
    ))
    .with_help("the tag property is a `list` of `string`, one tag per element".to_owned());
    sink.report(
        KernelDiagnostic::ContractTagPropertyNotListOfString,
        report,
        at,
    );
}

/// The lifecycle axis, and the consistency of its ordinary state.
fn lifecycle(sink: &mut Sink<'_>, declared: &LifecycleDecl, types: &[TypeDecl]) {
    let LifecycleDecl::Axis { axis, ordinary } = declared else {
        return;
    };
    let Some(values) = axis_values(sink, axis, types) else {
        return;
    };
    let axis = Axis {
        name: axis,
        values,
        types,
    };
    match ordinary {
        Ordinary::Absent => absent_is_ordinary(sink, &axis),
        Ordinary::Value(value) => value_is_ordinary(sink, &axis, value),
    }
}

/// The axis names a property declared on at least one type, whose kind is
/// `enum`. The values come from that declaration and are never restated inside
/// `[lifecycle]`, so the two cannot drift.
fn axis_values<'a>(sink: &mut Sink<'_>, name: &str, types: &'a [TypeDecl]) -> Option<&'a [String]> {
    let at = located(sink, "lifecycle.axis");
    let Some(kind) = kind_of(types, name) else {
        let report = Report::new(format!(
            "the lifecycle axis names `{name}`, which no type declares"
        ))
        .with_help("the axis names a property some type declares as an `enum`".to_owned());
        sink.report(
            KernelDiagnostic::ContractLifecycleAxisUndeclared,
            report,
            at,
        );
        return None;
    };
    let Some(values) = kind.values() else {
        let report = Report::new(format!(
            "the lifecycle axis names `{name}`, whose declared kind is {}",
            kind.describe()
        ))
        .with_help("the axis is an `enum`, and its members are the lifecycle states".to_owned());
        sink.report(KernelDiagnostic::ContractLifecycleAxisNotEnum, report, at);
        return None;
    };
    Some(values)
}

/// The declared axis, and every type that declares the property carrying it.
struct Axis<'a> {
    name: &'a str,
    values: &'a [String],
    types: &'a [TypeDecl],
}

impl Axis<'_> {
    /// The first type declaring the axis property for which `wanted` does not
    /// match how it requires the property.
    fn required_elsewhere(&self, wanted: bool) -> Option<&TypeDecl> {
        declarers(self.types, self.name)
            .find(|(_, property)| property.required() != wanted)
            .map(|(declared, _)| declared)
    }
}

/// A named ordinary state must be a member of the axis enum, and the axis must
/// be required on every type that declares it: the ordinary state cannot be a
/// value notes are allowed to omit.
fn value_is_ordinary(sink: &mut Sink<'_>, axis: &Axis<'_>, value: &str) {
    let at = located(sink, "lifecycle.ordinary.value");
    if !axis.values.iter().any(|member| member.as_str() == value) {
        let report = Report::new(format!(
            "the ordinary state is `{value}`, which `{}` does not declare",
            axis.name
        ))
        .with_help(listing(axis.values));
        sink.report(
            KernelDiagnostic::ContractLifecycleOrdinaryValueUndeclared,
            report,
            at,
        );
        return;
    }
    let Some(optional) = axis.required_elsewhere(true) else {
        return;
    };
    let report = Report::new(format!(
        "the ordinary state is `{value}`, but the type `{}` declares `{}` as optional",
        optional.name(),
        axis.name
    ))
    .with_help(
        "the ordinary state cannot be a value notes are allowed to omit; declare the axis \
         `required = true` on every type that carries it"
            .to_owned(),
    );
    sink.report(
        KernelDiagnostic::ContractLifecycleOrdinaryValueOptional,
        report,
        at,
    );
}

/// An absent ordinary state requires the axis to be optional on every type that
/// declares it: absence cannot be the ordinary state of a property every note
/// must carry.
fn absent_is_ordinary(sink: &mut Sink<'_>, axis: &Axis<'_>) {
    let Some(required) = axis.required_elsewhere(false) else {
        return;
    };
    let at = located(sink, "lifecycle.ordinary.absent");
    let report = Report::new(format!(
        "the ordinary state is the absence of `{}`, but the type `{}` requires it",
        axis.name,
        required.name()
    ))
    .with_help(
        "absence cannot be the ordinary state of a property every note must carry; declare the \
         axis `required = false` on every type that carries it"
            .to_owned(),
    );
    sink.report(
        KernelDiagnostic::ContractLifecycleOrdinaryAbsentRequired,
        report,
        at,
    );
}

fn listing(values: &[String]) -> String {
    let quoted: Vec<String> = values.iter().map(|value| format!("`{value}`")).collect();
    format!("the axis declares {}", quoted.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::parse;
    use crate::contract::schema;
    use crate::contract::sink::tests::{root_of, text_of};
    use crate::diagnostic::DiagnosticList;

    fn read(source: &str) -> DiagnosticList {
        read_at(&schema::VERSION_1, source)
    }

    fn read_at(schema: &'static schema::Schema, source: &str) -> DiagnosticList {
        let text = text_of(source);
        let document = root_of(&text);
        let mut sink = Sink::new(&text, schema);
        parse::body(&mut sink, document.get_ref());
        let (diagnostics, _) = sink.finish();
        diagnostics
    }

    fn ids(diagnostics: &DiagnosticList) -> Vec<&str> {
        diagnostics
            .as_slice()
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect()
    }

    /// A contract that loads clean, with `{body}` spliced in after the header.
    fn contract(body: &str) -> String {
        format!(
            concat!(
                "[dialect]\n",
                "links = \"wikilink\"\n",
                "\n",
                "[lifecycle]\n",
                "none = true\n",
                "\n",
                "[[type]]\n",
                "name = \"capture\"\n",
                "capabilities = [\"catch-all\"]\n",
                "{}"
            ),
            body
        )
    }

    #[test]
    fn a_repeated_property_is_reported_even_when_the_first_kind_does_not_resolve() {
        // The property used to be discarded before it claimed its name, so the
        // second declaration of `p` looked like the first and the duplicate
        // surfaced only once the kind was corrected.
        let source = contract(concat!(
            "\n",
            "[[type.property]]\n",
            "name = \"p\"\n",
            "kind = \"urll\"\n",
            "\n",
            "[[type.property]]\n",
            "name = \"p\"\n",
            "kind = \"string\"\n",
        ));
        let diagnostics = read(&source);
        let mut found = ids(&diagnostics);
        found.sort_unstable();
        assert_eq!(
            found,
            [
                "contract.duplicate-property",
                "contract.unknown-property-kind"
            ]
        );
    }

    #[test]
    fn an_axis_property_with_an_unresolved_kind_is_not_also_called_undeclared() {
        // The property is dropped because its kind does not resolve, and the
        // cross-reference rules then see a model with no `status` in it. They
        // used to conclude that no type declares it — of a file that declares
        // it two lines above the axis. One fault, one diagnostic.
        let source = concat!(
            "[dialect]\n",
            "links = \"wikilink\"\n",
            "\n",
            "[lifecycle]\n",
            "axis = \"status\"\n",
            "ordinary = { absent = true }\n",
            "\n",
            "[[type]]\n",
            "name = \"capture\"\n",
            "capabilities = [\"catch-all\"]\n",
            "\n",
            "[[type.property]]\n",
            "name = \"status\"\n",
            "kind = \"enumm\"\n",
        );
        assert_eq!(ids(&read(source)), ["contract.unknown-property-kind"]);
    }

    #[test]
    fn a_flag_naming_a_property_with_an_unresolved_kind_is_not_also_called_undeclared() {
        let source = concat!(
            "[dialect]\n",
            "links = \"wikilink\"\n",
            "\n",
            "[lifecycle]\n",
            "none = true\n",
            "\n",
            "[[flag]]\n",
            "property = \"pinned\"\n",
            "\n",
            "[[type]]\n",
            "name = \"capture\"\n",
            "capabilities = [\"catch-all\"]\n",
            "\n",
            "[[type.property]]\n",
            "name = \"pinned\"\n",
            "kind = \"boolian\"\n",
        );
        assert_eq!(ids(&read(source)), ["contract.unknown-property-kind"]);
    }

    #[test]
    fn a_conforming_contract_raises_nothing() {
        assert!(read(&contract("")).is_empty());
    }

    #[test]
    fn a_contract_with_no_catch_all_is_refused() {
        let source = contract("capabilities = []\n").replace("capabilities = []\n", "");
        let source = source.replace("capabilities = [\"catch-all\"]\n", "");
        assert_eq!(ids(&read(&source)), ["contract.missing-catch-all"]);
    }

    #[test]
    fn two_catch_all_types_point_at_every_declaration() {
        let source = contract("\n[[type]]\nname = \"note\"\ncapabilities = [\"catch-all\"]\n");
        let diagnostics = read(&source);
        assert_eq!(ids(&diagnostics), ["contract.multiple-catch-all"]);
        let diagnostic = &diagnostics.as_slice()[0];
        assert_eq!(
            diagnostic.message,
            "2 types declare the catch-all capability"
        );
        assert_eq!(diagnostic.related.len(), 1);
        assert!(diagnostic.related[0].message.contains("`note`"));
    }

    #[test]
    fn three_catch_all_types_carry_two_pieces_of_evidence() {
        let source = contract(concat!(
            "\n[[type]]\nname = \"note\"\ncapabilities = [\"catch-all\"]\n",
            "\n[[type]]\nname = \"scrap\"\ncapabilities = [\"catch-all\"]\n",
        ));
        let diagnostics = read(&source);
        assert_eq!(diagnostics.as_slice()[0].related.len(), 2);
    }

    /// A contract declaring one catch-all type, which each case below completes
    /// with that type's own declarations. It carries no `contract_version`, so
    /// the same bytes can be read at either version's schema and the answers
    /// compared — which is the whole of "the rule is version-scoped".
    const CATCH_ALL: &str = concat!(
        "[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    /// A property every note of the declaring type must carry.
    const REQUIRED_PROPERTY: &str =
        "\n  [[type.property]]\n  name = \"title\"\n  kind = \"string\"\n  required = true\n";

    /// A relationship every note of the declaring type must claim.
    const REQUIRED_RELATIONSHIP: &str =
        "\n  [[type.relationship]]\n  predicate = \"mentions\"\n  required = true\n";

    /// A tag namespace every note of the declaring type must have a tag in,
    /// with the tag property it reads and the `[tags]` table naming it.
    const REQUIRED_NAMESPACE: &str = concat!(
        "\n  [[type.property]]\n  name = \"labels\"\n  kind = \"list\"\n  of = \"string\"\n",
        "\n  [[type.tag-namespace]]\n  prefix = \"log/\"\n  required = true\n  open = true\n",
        "\n[tags]\nproperty = \"labels\"\n",
    );

    /// The one diagnostic `diagnostics` holds, or a panic naming what it holds
    /// instead.
    fn only(diagnostics: &DiagnosticList) -> &Diagnostic {
        assert_eq!(ids(diagnostics), ["contract.catch-all-requires"]);
        &diagnostics.as_slice()[0]
    }

    #[test]
    fn a_catch_all_requiring_a_property_is_refused_at_version_2() {
        let source = [CATCH_ALL, REQUIRED_PROPERTY].concat();
        let diagnostics = read_at(&schema::VERSION_2, &source);
        let reported = only(&diagnostics);
        assert!(reported.message.contains("`capture`"));
        assert_eq!(
            reported.related.len(),
            1,
            "one offending declaration, named once"
        );
        assert_eq!(
            reported.related[0].message,
            "`title` is a required property"
        );
    }

    #[test]
    fn the_refusal_points_at_the_capability_and_at_the_declaration() {
        // The fault is the pairing, so the diagnostic is anchored where the
        // type says it accepts anything, and the evidence is where it says a
        // note must carry something.
        let source = [CATCH_ALL, REQUIRED_PROPERTY].concat();
        let diagnostics = read_at(&schema::VERSION_2, &source);
        let reported = only(&diagnostics);
        let at = reported.location.as_ref().expect("a location");
        let capabilities = source.find("[\"catch-all\"]").expect("the capability list");
        assert_eq!(
            at.span.as_ref().map(|span| span.start.offset),
            Some(capabilities),
            "the diagnostic points at the capability"
        );
        let evidence = reported.related[0].location.as_ref().expect("a location");
        let required = source.find("required = true").expect("the required leaf");
        assert_eq!(
            evidence.span.as_ref().map(|span| span.start.offset),
            Some(required + "required = ".len()),
            "the evidence points at the leaf that requires it"
        );
    }

    #[test]
    fn a_catch_all_requiring_a_relationship_is_refused_at_version_2() {
        let source = [CATCH_ALL, REQUIRED_RELATIONSHIP].concat();
        let diagnostics = read_at(&schema::VERSION_2, &source);
        assert_eq!(
            only(&diagnostics).related[0].message,
            "`mentions` is a required relationship"
        );
    }

    #[test]
    fn a_catch_all_requiring_a_tag_namespace_is_refused_at_version_2() {
        let source = [CATCH_ALL, REQUIRED_NAMESPACE].concat();
        let diagnostics = read_at(&schema::VERSION_2, &source);
        assert_eq!(
            only(&diagnostics).related[0].message,
            "`log/` is a required tag-namespace"
        );
    }

    #[test]
    fn every_requirement_is_named_by_one_diagnostic_in_declaration_order() {
        let source = [
            CATCH_ALL,
            REQUIRED_PROPERTY,
            REQUIRED_RELATIONSHIP,
            REQUIRED_NAMESPACE,
        ]
        .concat();
        let diagnostics = read_at(&schema::VERSION_2, &source);
        let reported = only(&diagnostics);
        assert_eq!(
            reported.message,
            "the catch-all type `capture` requires 3 of its declarations"
        );
        let named: Vec<&str> = reported
            .related
            .iter()
            .map(|related| related.message.as_str())
            .collect();
        assert_eq!(
            named,
            [
                "`title` is a required property",
                "`mentions` is a required relationship",
                "`log/` is a required tag-namespace",
            ]
        );
    }

    #[test]
    fn the_same_catch_all_still_loads_at_version_1() {
        // Validity is part of a version's schema, and version 1's is frozen: a
        // contract that loaded clean at `0.1.0-beta.1` keeps loading, or the
        // upgrade promise the floor policy exists to keep is broken. The same
        // bytes, two schemas, two answers.
        let source = [CATCH_ALL, REQUIRED_PROPERTY, REQUIRED_RELATIONSHIP].concat();
        assert!(read_at(&schema::VERSION_1, &source).is_empty());
        assert_eq!(
            ids(&read_at(&schema::VERSION_2, &source)),
            ["contract.catch-all-requires"]
        );
    }

    #[test]
    fn a_catch_all_requiring_nothing_is_accepted_at_version_2() {
        let optional = concat!(
            "\n  [[type.property]]\n  name = \"title\"\n  kind = \"string\"\n",
            "\n  [[type.relationship]]\n  predicate = \"mentions\"\n",
        );
        assert!(read_at(&schema::VERSION_2, &[CATCH_ALL, optional].concat()).is_empty());
    }

    #[test]
    fn two_catch_all_types_are_not_also_told_what_they_require() {
        // One fault, one diagnostic: the requiring rule reads *the* catch-all,
        // and a contract that declares two has not said which one that is.
        let second = concat!(
            "\n[[type]]\nname = \"note\"\ncapabilities = [\"catch-all\"]\n",
            "\n  [[type.property]]\n  name = \"body\"\n  kind = \"string\"\n",
            "  required = true\n",
        );
        let source = [CATCH_ALL, REQUIRED_PROPERTY, second].concat();
        assert_eq!(
            ids(&read_at(&schema::VERSION_2, &source)),
            ["contract.multiple-catch-all"]
        );
    }

    #[test]
    fn a_type_name_holding_a_dot_is_refused() {
        // `t.property.p` would address exactly what type `t`'s property `p`
        // addresses, and provenance keeps one entry per key.
        let source = contract("").replace("\"capture\"", "\"t.property.p\"");
        let diagnostics = read(&source);
        assert_eq!(ids(&diagnostics), ["contract.declaration-name-invalid"]);
        assert!(diagnostics.as_slice()[0].message.contains("`t.property.p`"));
    }

    #[test]
    fn an_empty_type_name_is_refused() {
        let source = contract("").replace("\"capture\"", "\"\"");
        let diagnostics = read(&source);
        assert_eq!(ids(&diagnostics), ["contract.declaration-name-invalid"]);
        assert!(diagnostics.as_slice()[0].help.is_some());
    }

    #[test]
    fn the_name_rule_reaches_a_property_name_too() {
        // `name_of` is the one place a declaration's name is read, so the rule
        // covers a property, a relationship predicate and a flag with it.
        let source = contract("\n[[type.property]]\nname = \"a.b\"\nkind = \"string\"\n");
        assert_eq!(ids(&read(&source)), ["contract.declaration-name-invalid"]);
    }

    #[test]
    fn an_enum_member_holding_a_dot_still_loads() {
        // A member is a value rather than an address: nothing joins it into a
        // key path, so the rule that constrains names does not reach it.
        let body = "\n[[type.property]]\nname = \"status\"\nkind = \"enum\"\nvalues = [\"a.b\"]\n";
        assert!(read(&contract(body)).is_empty());
    }

    #[test]
    fn a_contract_with_no_type_says_only_that() {
        let source = "[dialect]\nlinks = \"wikilink\"\n\n[lifecycle]\nnone = true\n";
        assert_eq!(ids(&read(source)), ["contract.no-types"]);
    }

    /// A contract declaring a flag named `leaned_on`, over a type declaring
    /// `property` with `kind`.
    fn flagged(property: &str, kind: &str) -> String {
        let body = format!("\n[[type.property]]\nname = \"{property}\"\nkind = \"{kind}\"\n");
        format!("[[flag]]\nproperty = \"leaned_on\"\n\n{}", contract(&body))
    }

    #[test]
    fn a_flag_naming_a_boolean_property_is_accepted() {
        assert!(read(&flagged("leaned_on", "boolean")).is_empty());
    }

    #[test]
    fn a_flag_naming_no_declared_property_is_refused() {
        let diagnostics = read(&flagged("other", "boolean"));
        assert_eq!(ids(&diagnostics), ["contract.flag-property-undeclared"]);
        assert!(diagnostics.as_slice()[0].location.is_some());
    }

    #[test]
    fn a_flag_naming_a_property_that_is_not_boolean_is_refused() {
        let diagnostics = read(&flagged("leaned_on", "string"));
        assert_eq!(ids(&diagnostics), ["contract.flag-property-not-boolean"]);
        assert!(diagnostics.as_slice()[0].message.contains("`string`"));
    }

    /// A contract whose lifecycle declares an axis with `ordinary`, over a type
    /// declaring the axis property with `required`.
    fn axis(ordinary: &str, kind: &str, required: bool) -> String {
        format!(
            concat!(
                "[dialect]\n",
                "links = \"wikilink\"\n",
                "\n",
                "[lifecycle]\n",
                "axis = \"status\"\n",
                "ordinary = {}\n",
                "\n",
                "[[type]]\n",
                "name = \"capture\"\n",
                "capabilities = [\"catch-all\"]\n",
                "\n",
                "  [[type.property]]\n",
                "  name = \"status\"\n",
                "  kind = \"{}\"\n",
                "  values = [\"draft\", \"current\"]\n",
                "  required = {}\n",
            ),
            ordinary, kind, required
        )
    }

    #[test]
    fn an_absent_ordinary_state_over_an_optional_axis_is_accepted() {
        assert!(read(&axis("{ absent = true }", "enum", false)).is_empty());
    }

    #[test]
    fn a_named_ordinary_state_over_a_required_axis_is_accepted() {
        assert!(read(&axis("{ value = \"current\" }", "enum", true)).is_empty());
    }

    #[test]
    fn an_axis_naming_no_declared_property_is_refused() {
        let source = axis("{ absent = true }", "enum", false)
            .replace("\"status\"\n  kind", "\"other\"\n  kind");
        let diagnostics = read(&source);
        assert_eq!(ids(&diagnostics), ["contract.lifecycle-axis-undeclared"]);
        assert!(diagnostics.as_slice()[0].location.is_some());
    }

    #[test]
    fn an_axis_that_is_not_an_enum_is_refused() {
        let source = axis("{ absent = true }", "string", false)
            .replace("  values = [\"draft\", \"current\"]\n", "");
        assert_eq!(ids(&read(&source)), ["contract.lifecycle-axis-not-enum"]);
    }

    #[test]
    fn a_named_ordinary_state_outside_the_axis_enum_is_refused() {
        let diagnostics = read(&axis("{ value = \"shipped\" }", "enum", true));
        assert_eq!(
            ids(&diagnostics),
            ["contract.lifecycle-ordinary-value-undeclared"]
        );
        let help = diagnostics.as_slice()[0].help.as_deref();
        assert_eq!(help, Some("the axis declares `draft`, `current`"));
    }

    #[test]
    fn a_named_ordinary_state_over_an_optional_axis_is_refused() {
        let diagnostics = read(&axis("{ value = \"current\" }", "enum", false));
        assert_eq!(
            ids(&diagnostics),
            ["contract.lifecycle-ordinary-value-optional"]
        );
        assert!(diagnostics.as_slice()[0].message.contains("`capture`"));
    }

    #[test]
    fn an_absent_ordinary_state_over_a_required_axis_is_refused() {
        let diagnostics = read(&axis("{ absent = true }", "enum", true));
        assert_eq!(
            ids(&diagnostics),
            ["contract.lifecycle-ordinary-absent-required"]
        );
        assert!(diagnostics.as_slice()[0].message.contains("requires it"));
    }

    #[test]
    fn the_axis_must_agree_on_every_type_that_declares_it() {
        let source = format!(
            "{}{}",
            axis("{ value = \"current\" }", "enum", true),
            concat!(
                "\n[[type]]\n",
                "name = \"person\"\n",
                "\n  [[type.property]]\n",
                "  name = \"status\"\n",
                "  kind = \"enum\"\n",
                "  values = [\"draft\", \"current\"]\n",
                "  required = false\n",
            )
        );
        let diagnostics = read(&source);
        assert_eq!(
            ids(&diagnostics),
            ["contract.lifecycle-ordinary-value-optional"]
        );
        assert!(diagnostics.as_slice()[0].message.contains("`person`"));
    }

    #[test]
    fn a_corpus_declaring_no_axis_reaches_no_lifecycle_rule() {
        assert!(read(&contract("")).is_empty());
    }

    /// A version-2 contract declaring one catch-all type, which each tag case
    /// below completes with the type's own declarations and, after them, the
    /// `[tags]` table when it declares one. A root table may follow an array of
    /// tables, so both halves compose into one source.
    const TAGGED: &str = concat!(
        "contract_version = 2\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[[type]]\nname = \"log\"\ncapabilities = [\"catch-all\"]\n",
    );

    /// The `[tags]` table naming `labels`.
    const TAGS: &str = "\n[tags]\nproperty = \"labels\"\n";

    /// The tag property, declared as the list of string the format requires.
    const LABELS: &str =
        "\n  [[type.property]]\n  name = \"labels\"\n  kind = \"list\"\n  of = \"string\"\n";

    /// One open namespace over it.
    const NAMESPACE: &str = "\n  [[type.tag-namespace]]\n  prefix = \"log/\"\n  open = true\n";

    #[test]
    fn a_type_declaring_a_namespace_over_a_declared_list_of_string_is_accepted() {
        assert!(
            read_at(
                &schema::VERSION_2,
                &[TAGGED, LABELS, NAMESPACE, TAGS].concat()
            )
            .is_empty()
        );
    }

    #[test]
    fn a_namespace_in_a_contract_with_no_tags_table_is_refused() {
        let diagnostics = read_at(&schema::VERSION_2, &[TAGGED, LABELS, NAMESPACE].concat());
        assert_eq!(ids(&diagnostics), ["contract.tags-table-missing"]);
        let reported = &diagnostics.as_slice()[0];
        assert!(reported.message.contains("the type `log`"));
        assert!(
            reported
                .location
                .as_ref()
                .is_some_and(|at| at.span.is_some())
        );
    }

    #[test]
    fn a_namespace_on_a_type_that_does_not_declare_the_tag_property_is_refused() {
        let diagnostics = read_at(&schema::VERSION_2, &[TAGGED, NAMESPACE, TAGS].concat());
        assert_eq!(ids(&diagnostics), ["contract.tag-property-undeclared"]);
        assert!(diagnostics.as_slice()[0].message.contains("`labels`"));
    }

    #[test]
    fn a_tag_property_declared_as_something_other_than_a_list_of_string_is_refused() {
        let scalar = "\n  [[type.property]]\n  name = \"labels\"\n  kind = \"string\"\n";
        let diagnostics = read_at(
            &schema::VERSION_2,
            &[TAGGED, scalar, NAMESPACE, TAGS].concat(),
        );
        assert_eq!(
            ids(&diagnostics),
            ["contract.tag-property-not-list-of-string"]
        );
        assert!(diagnostics.as_slice()[0].message.contains("`string`"));
    }

    #[test]
    fn a_list_of_something_other_than_string_is_refused_as_the_tag_property() {
        let dates =
            "\n  [[type.property]]\n  name = \"labels\"\n  kind = \"list\"\n  of = \"date\"\n";
        let diagnostics = read_at(
            &schema::VERSION_2,
            &[TAGGED, dates, NAMESPACE, TAGS].concat(),
        );
        assert_eq!(
            ids(&diagnostics),
            ["contract.tag-property-not-list-of-string"]
        );
    }

    #[test]
    fn a_tag_property_no_type_declares_is_reported_once_rather_than_twice() {
        // The per-type rule already says the declaring type is missing it, and
        // a kind rule over a kind nobody declared would be a second complaint
        // about one fault.
        let elsewhere = "\n[[type]]\nname = \"other\"\n";
        let diagnostics = read_at(
            &schema::VERSION_2,
            &[TAGGED, NAMESPACE, TAGS, elsewhere].concat(),
        );
        assert_eq!(ids(&diagnostics), ["contract.tag-property-undeclared"]);
    }

    #[test]
    fn a_tags_table_over_a_corpus_declaring_no_namespace_reaches_no_rule() {
        // The record binds the rule to types that declare a namespace, so a
        // `[tags]` naming a property nothing declares is not this rule's
        // business.
        assert!(read_at(&schema::VERSION_2, &[TAGGED, TAGS].concat()).is_empty());
    }

    #[test]
    fn a_tags_table_that_did_not_resolve_is_not_also_called_absent() {
        // The table is written, so concluding "the contract declares no
        // `[tags]` table" would contradict the file in front of the reader.
        let broken = "\n[tags]\nproperty = 4\n";
        let diagnostics = read_at(
            &schema::VERSION_2,
            &[TAGGED, LABELS, NAMESPACE, broken].concat(),
        );
        assert_eq!(ids(&diagnostics), ["contract.value-wrong-type"]);
    }

    #[test]
    fn a_rule_falls_back_to_the_file_when_nothing_recorded_a_location() {
        let text = text_of("a = 1\n");
        let sink = Sink::new(&text, &schema::VERSION_1);
        assert!(located(&sink, "lifecycle.axis").span.is_none());
    }
}
