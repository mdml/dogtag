//! Reading the mandatory `[lifecycle]` table.
//!
//! The table is mandatory and the axis inside it is optional, so a corpus with
//! no life axis *says so* rather than staying silent. Omission is never a
//! decision here: a forgotten table and a deliberately absent one would
//! otherwise be indistinguishable, in a format where every other omission of a
//! required construct is fatal.
//!
//! This module reads the declaration's **shape**. Whether the axis it names is
//! a declared `enum`, and whether the ordinary state is consistent with how
//! that property is required, are rules over the whole contract and live in
//! [`super::validate`].

use toml::Spanned;
use toml::de::{DeTable, DeValue};

use crate::diagnostic::{Diagnostic, KernelDiagnostic};
use crate::document;

use super::model::{LifecycleDecl, Ordinary};
use super::sink::{KeyPath, Report, Section, Sink};

/// Every key `[lifecycle]` defines at contract version 1.
const LIFECYCLE_KEYS: &[&str] = &["axis", "none", "ordinary"];

/// Every key `[lifecycle.ordinary]` defines at contract version 1.
const ORDINARY_KEYS: &[&str] = &["absent", "value"];

const INCOMPLETE_HELP: &str =
    "declare either `axis` with `ordinary`, or `none = true` for a corpus with no life axis";

/// Reads the `[lifecycle]` table, or reports its absence.
pub(crate) fn declaration(sink: &mut Sink<'_>, root: &DeTable<'_>) -> Option<LifecycleDecl> {
    let Some(value) = document::get(root, "lifecycle") else {
        let at = sink.whole_file();
        let report = Report::new(
            "the contract declares no `[lifecycle]` table; every contract declares one".to_owned(),
        )
        .with_help(INCOMPLETE_HELP.to_owned());
        sink.report(KernelDiagnostic::ContractMissingLifecycle, report, at);
        return None;
    };
    let table = sink.table(value, "lifecycle")?;
    let section = Section {
        table,
        span: value.span(),
        label: "`[lifecycle]`".to_owned(),
        path: KeyPath::root().child("lifecycle"),
    };
    sink.sweep(&section, LIFECYCLE_KEYS);
    resolve(sink, &section)
}

/// Chooses between the two declarations the table may carry.
fn resolve(sink: &mut Sink<'_>, section: &Section<'_, '_>) -> Option<LifecycleDecl> {
    let none = section.get("none");
    let axis = section.get("axis").or_else(|| section.get("ordinary"));
    match (none, axis) {
        (Some(none), Some(axis)) => {
            exclusive(sink, none, axis);
            None
        }
        (Some(none), Option::None) => no_axis(sink, section, none),
        (Option::None, _) => an_axis(sink, section),
    }
}

/// `none = true` is exclusive with `axis` and `ordinary`.
fn exclusive(sink: &mut Sink<'_>, none: &Spanned<DeValue<'_>>, axis: &Spanned<DeValue<'_>>) {
    let related = sink.related("the axis is declared here", axis.span());
    let diagnostic = Diagnostic::kernel(
        KernelDiagnostic::ContractLifecycleNoneWithAxis,
        "`[lifecycle]` declares `none` alongside an axis",
    )
    .at(sink.location(none.span()))
    .with_related(related)
    .with_help(INCOMPLETE_HELP);
    sink.push(diagnostic);
}

/// The corpus states that it has no life axis.
fn no_axis(
    sink: &mut Sink<'_>,
    section: &Section<'_, '_>,
    none: &Spanned<DeValue<'_>>,
) -> Option<LifecycleDecl> {
    if sink.boolean(none, section.leaf("none"))? {
        return Some(LifecycleDecl::None);
    }
    incomplete(sink, section);
    None
}

/// The corpus declares a life axis and how its ordinary state is encoded.
fn an_axis(sink: &mut Sink<'_>, section: &Section<'_, '_>) -> Option<LifecycleDecl> {
    let (Some(axis), Some(ordinary)) = (section.get("axis"), section.get("ordinary")) else {
        incomplete(sink, section);
        return None;
    };
    let axis = sink.string(axis, section.leaf("axis"))?.to_owned();
    let ordinary = ordinary_state(sink, section, ordinary)?;
    Some(LifecycleDecl::Axis { axis, ordinary })
}

fn incomplete(sink: &mut Sink<'_>, section: &Section<'_, '_>) {
    let at = sink.location(section.span.clone());
    let report = Report::new(
        "`[lifecycle]` declares neither an axis with its ordinary state nor `none = true`"
            .to_owned(),
    )
    .with_help(INCOMPLETE_HELP.to_owned());
    sink.report(KernelDiagnostic::ContractLifecycleIncomplete, report, at);
}

/// Reads `[lifecycle.ordinary]`, which declares exactly one of `value` and
/// `absent`.
fn ordinary_state(
    sink: &mut Sink<'_>,
    section: &Section<'_, '_>,
    value: &Spanned<DeValue<'_>>,
) -> Option<Ordinary> {
    let table = sink.table(value, "ordinary")?;
    let ordinary = Section {
        table,
        span: value.span(),
        label: "`[lifecycle.ordinary]`".to_owned(),
        path: section.path.child("ordinary"),
    };
    sink.sweep(&ordinary, ORDINARY_KEYS);
    match (ordinary.get("value"), ordinary.get("absent")) {
        (Some(named), Option::None) => named_state(sink, &ordinary, named),
        (Option::None, Some(absent)) => absent_state(sink, &ordinary, absent),
        _ => {
            ordinary_invalid(sink, &ordinary);
            None
        }
    }
}

/// The ordinary state is a named member of the axis property's `enum`.
fn named_state(
    sink: &mut Sink<'_>,
    section: &Section<'_, '_>,
    value: &Spanned<DeValue<'_>>,
) -> Option<Ordinary> {
    let named = sink.string(value, section.leaf("value"))?;
    Some(Ordinary::Value(named.to_owned()))
}

/// The ordinary state is the absence of a value.
///
/// `absent` must be the boolean `true`. A non-boolean and a `false` are both
/// this identifier's business rather than the generic wrong-type one, so one
/// fault produces one diagnostic.
fn absent_state(
    sink: &mut Sink<'_>,
    section: &Section<'_, '_>,
    value: &Spanned<DeValue<'_>>,
) -> Option<Ordinary> {
    if document::expect_bool(value).ok() != Some(true) {
        ordinary_invalid(sink, section);
        return None;
    }
    sink.written(section.leaf("absent").key, value.span());
    Some(Ordinary::Absent)
}

fn ordinary_invalid(sink: &mut Sink<'_>, section: &Section<'_, '_>) {
    let at = sink.location(section.span.clone());
    let report = Report::new(
        "`[lifecycle.ordinary]` does not declare exactly one of `value` and `absent = true`"
            .to_owned(),
    )
    .with_help(
        "write `ordinary = { value = \"…\" }` for a named ordinary state, or \
         `ordinary = { absent = true }` when the ordinary state is no value at all"
            .to_owned(),
    );
    sink.report(
        KernelDiagnostic::ContractLifecycleOrdinaryInvalid,
        report,
        at,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::sink::tests::{root_of, text_of};
    use crate::diagnostic::DiagnosticList;

    struct Read {
        declaration: Option<LifecycleDecl>,
        diagnostics: DiagnosticList,
    }

    fn read(source: &str) -> Read {
        let text = text_of(source);
        let document = root_of(&text);
        let mut sink = Sink::new(&text, 1);
        let declaration = declaration(&mut sink, document.get_ref());
        let (diagnostics, _) = sink.finish();
        Read {
            declaration,
            diagnostics,
        }
    }

    fn ids(read: &Read) -> Vec<&str> {
        read.diagnostics
            .as_slice()
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect()
    }

    #[test]
    fn an_axis_with_an_absent_ordinary_state_reads() {
        let read = read("[lifecycle]\naxis = \"status\"\nordinary = { absent = true }\n");
        assert!(read.diagnostics.is_empty());
        assert_eq!(
            read.declaration,
            Some(LifecycleDecl::Axis {
                axis: "status".to_owned(),
                ordinary: Ordinary::Absent,
            })
        );
    }

    #[test]
    fn an_axis_with_a_named_ordinary_state_reads() {
        let read = read("[lifecycle]\naxis = \"stage\"\nordinary = { value = \"current\" }\n");
        assert!(read.diagnostics.is_empty());
        assert_eq!(
            read.declaration.as_ref().and_then(LifecycleDecl::axis),
            Some("stage")
        );
        assert_eq!(
            read.declaration.and_then(|declared| declared
                .ordinary()
                .and_then(Ordinary::value)
                .map(str::to_owned)),
            Some("current".to_owned())
        );
    }

    #[test]
    fn a_corpus_with_no_axis_declares_it() {
        let read = read("[lifecycle]\nnone = true\n");
        assert!(read.diagnostics.is_empty());
        assert_eq!(read.declaration, Some(LifecycleDecl::None));
    }

    #[test]
    fn an_absent_table_is_a_load_error() {
        let read = read("contract_version = 1\n");
        assert_eq!(ids(&read), ["contract.missing-lifecycle"]);
        assert!(read.declaration.is_none());
    }

    #[test]
    fn a_table_that_is_not_a_table_is_reported_as_a_wrong_type() {
        let read = read("lifecycle = \"none\"\n");
        assert_eq!(ids(&read), ["contract.value-wrong-type"]);
    }

    #[test]
    fn an_unknown_key_anywhere_in_the_table_is_fatal() {
        let read = read("[lifecycle]\nnone = true\nphase = \"beta\"\n");
        assert_eq!(ids(&read), ["contract.unknown-key"]);
    }

    #[test]
    fn an_unknown_key_inside_the_ordinary_table_is_fatal() {
        let read = read("[lifecycle]\naxis = \"a\"\nordinary = { absent = true, why = 1 }\n");
        assert_eq!(ids(&read), ["contract.unknown-key"]);
    }

    #[test]
    fn none_is_exclusive_with_an_axis() {
        let read = read("[lifecycle]\nnone = true\naxis = \"status\"\n");
        assert_eq!(ids(&read), ["contract.lifecycle-none-with-axis"]);
        assert_eq!(read.diagnostics.as_slice()[0].related.len(), 1);
    }

    #[test]
    fn none_is_exclusive_with_an_ordinary_state() {
        let read = read("[lifecycle]\nnone = true\nordinary = { absent = true }\n");
        assert_eq!(ids(&read), ["contract.lifecycle-none-with-axis"]);
    }

    #[test]
    fn an_empty_table_declares_nothing() {
        let read = read("[lifecycle]\n");
        assert_eq!(ids(&read), ["contract.lifecycle-incomplete"]);
    }

    #[test]
    fn an_axis_without_an_ordinary_state_is_incomplete() {
        let read = read("[lifecycle]\naxis = \"status\"\n");
        assert_eq!(ids(&read), ["contract.lifecycle-incomplete"]);
    }

    #[test]
    fn an_ordinary_state_without_an_axis_is_incomplete() {
        let read = read("[lifecycle]\nordinary = { absent = true }\n");
        assert_eq!(ids(&read), ["contract.lifecycle-incomplete"]);
    }

    #[test]
    fn declaring_none_false_declares_nothing() {
        let read = read("[lifecycle]\nnone = false\n");
        assert_eq!(ids(&read), ["contract.lifecycle-incomplete"]);
    }

    #[test]
    fn a_non_boolean_none_is_reported_once() {
        let read = read("[lifecycle]\nnone = \"yes\"\n");
        assert_eq!(ids(&read), ["contract.value-wrong-type"]);
    }

    #[test]
    fn a_non_string_axis_is_reported_as_a_wrong_type() {
        let read = read("[lifecycle]\naxis = 4\nordinary = { absent = true }\n");
        assert_eq!(ids(&read), ["contract.value-wrong-type"]);
    }

    #[test]
    fn an_ordinary_state_that_is_not_a_table_is_reported_as_a_wrong_type() {
        let read = read("[lifecycle]\naxis = \"a\"\nordinary = \"absent\"\n");
        assert_eq!(ids(&read), ["contract.value-wrong-type"]);
    }

    #[test]
    fn an_ordinary_state_declaring_both_encodings_is_invalid() {
        let read = read("[lifecycle]\naxis = \"a\"\nordinary = { absent = true, value = \"c\" }\n");
        assert_eq!(ids(&read), ["contract.lifecycle-ordinary-invalid"]);
        assert!(read.diagnostics.as_slice()[0].help.is_some());
    }

    #[test]
    fn an_ordinary_state_declaring_neither_encoding_is_invalid() {
        let read = read("[lifecycle]\naxis = \"a\"\nordinary = {}\n");
        assert_eq!(ids(&read), ["contract.lifecycle-ordinary-invalid"]);
    }

    #[test]
    fn a_non_boolean_absent_is_the_ordinary_states_own_fault() {
        let read = read("[lifecycle]\naxis = \"a\"\nordinary = { absent = \"yes\" }\n");
        assert_eq!(ids(&read), ["contract.lifecycle-ordinary-invalid"]);
    }

    #[test]
    fn absent_false_is_not_a_declaration() {
        let read = read("[lifecycle]\naxis = \"a\"\nordinary = { absent = false }\n");
        assert_eq!(ids(&read), ["contract.lifecycle-ordinary-invalid"]);
    }

    #[test]
    fn a_non_string_ordinary_value_is_reported_as_a_wrong_type() {
        let read = read("[lifecycle]\naxis = \"a\"\nordinary = { value = 1 }\n");
        assert_eq!(ids(&read), ["contract.value-wrong-type"]);
    }
}
