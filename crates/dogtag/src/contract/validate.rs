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
    Capability, FlagDecl, LifecycleDecl, Ordinary, PropertyDecl, PropertyKind, TypeDecl,
};
use super::parse::Parts;
use super::sink::{Report, Sink};

/// Applies every load-time validity rule to what the walk resolved.
pub(crate) fn run(sink: &mut Sink<'_>, parts: &Parts) {
    capabilities(sink, &parts.types);
    flags(sink, &parts.flags, &parts.types);
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

/// Catch-all cardinality: exactly one type carries it.
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
        Some((_, [])) => {}
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
    use crate::contract::sink::tests::{root_of, text_of};
    use crate::diagnostic::DiagnosticList;

    fn read(source: &str) -> DiagnosticList {
        let text = text_of(source);
        let document = root_of(&text);
        let mut sink = Sink::new(&text, 1);
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

    #[test]
    fn a_contract_with_no_type_says_only_that() {
        let source = "[dialect]\nlinks = \"wikilink\"\n\n[lifecycle]\nnone = true\n";
        assert_eq!(ids(&read(source)), ["contract.no-types"]);
    }

    fn flagged(property: &str, kind: &str, extra: &str) -> String {
        let body =
            format!("\n[[type.property]]\nname = \"{property}\"\nkind = \"{kind}\"\n{extra}");
        format!("[[flag]]\nproperty = \"leaned_on\"\n\n{}", contract(&body))
    }

    #[test]
    fn a_flag_naming_a_boolean_property_is_accepted() {
        assert!(read(&flagged("leaned_on", "boolean", "")).is_empty());
    }

    #[test]
    fn a_flag_naming_no_declared_property_is_refused() {
        let diagnostics = read(&flagged("other", "boolean", ""));
        assert_eq!(ids(&diagnostics), ["contract.flag-property-undeclared"]);
        assert!(diagnostics.as_slice()[0].location.is_some());
    }

    #[test]
    fn a_flag_naming_a_property_that_is_not_boolean_is_refused() {
        let diagnostics = read(&flagged("leaned_on", "string", ""));
        assert_eq!(ids(&diagnostics), ["contract.flag-property-not-boolean"]);
        assert!(diagnostics.as_slice()[0].message.contains("`string`"));
    }

    /// A contract whose lifecycle declares `axis` with `ordinary`, over a type
    /// declaring the axis property with `{required}`.
    fn axis(ordinary: &str, kind: &str, required: &str) -> String {
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
        assert!(read(&axis("{ absent = true }", "enum", "false")).is_empty());
    }

    #[test]
    fn a_named_ordinary_state_over_a_required_axis_is_accepted() {
        assert!(read(&axis("{ value = \"current\" }", "enum", "true")).is_empty());
    }

    #[test]
    fn an_axis_naming_no_declared_property_is_refused() {
        let source = axis("{ absent = true }", "enum", "false")
            .replace("\"status\"\n  kind", "\"other\"\n  kind");
        let diagnostics = read(&source);
        assert_eq!(ids(&diagnostics), ["contract.lifecycle-axis-undeclared"]);
        assert!(diagnostics.as_slice()[0].location.is_some());
    }

    #[test]
    fn an_axis_that_is_not_an_enum_is_refused() {
        let source = axis("{ absent = true }", "string", "false")
            .replace("  values = [\"draft\", \"current\"]\n", "");
        assert_eq!(ids(&read(&source)), ["contract.lifecycle-axis-not-enum"]);
    }

    #[test]
    fn a_named_ordinary_state_outside_the_axis_enum_is_refused() {
        let diagnostics = read(&axis("{ value = \"shipped\" }", "enum", "true"));
        assert_eq!(
            ids(&diagnostics),
            ["contract.lifecycle-ordinary-value-undeclared"]
        );
        let help = diagnostics.as_slice()[0].help.as_deref();
        assert_eq!(help, Some("the axis declares `draft`, `current`"));
    }

    #[test]
    fn a_named_ordinary_state_over_an_optional_axis_is_refused() {
        let diagnostics = read(&axis("{ value = \"current\" }", "enum", "false"));
        assert_eq!(
            ids(&diagnostics),
            ["contract.lifecycle-ordinary-value-optional"]
        );
        assert!(diagnostics.as_slice()[0].message.contains("`capture`"));
    }

    #[test]
    fn an_absent_ordinary_state_over_a_required_axis_is_refused() {
        let diagnostics = read(&axis("{ absent = true }", "enum", "true"));
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
            axis("{ value = \"current\" }", "enum", "true"),
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

    #[test]
    fn a_rule_falls_back_to_the_file_when_nothing_recorded_a_location() {
        let text = text_of("a = 1\n");
        let sink = Sink::new(&text, 1);
        assert!(located(&sink, "lifecycle.axis").span.is_none());
    }
}
