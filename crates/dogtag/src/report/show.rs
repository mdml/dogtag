//! One note as the shared document model, and its plain-text rendering.

use crate::contract::{Contract, PropertyKind};
use crate::diagnostic::{Diagnostic, DiagnosticList, SeverityCounts};
use crate::note::{Corpus, Note, PropertyValue, RecordValue};
use crate::text::one_line;

/// The result of asking a corpus to show one reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShowReport {
    pub(super) note: Option<Note>,
    pub(super) contract: Contract,
    diagnostics: Vec<Diagnostic>,
    counts: SeverityCounts,
}

impl ShowReport {
    /// The note the reference named, or `None` when resolution refused it.
    pub fn note(&self) -> Option<&Note> {
        self.note.as_ref()
    }

    /// Everything reading and resolving the corpus reported, in deterministic order.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// How many diagnostics of each severity the report carries.
    pub fn counts(&self) -> SeverityCounts {
        self.counts
    }

    pub(super) fn kind(&self, name: &str) -> &PropertyKind {
        let type_name = self
            .note
            .as_ref()
            .and_then(|note| note.binding().type_name())
            .expect("only a bound note carries declared properties");
        self.contract
            .types()
            .iter()
            .find(|declared| declared.name() == type_name)
            .expect("a bound type comes from this contract")
            .property(name)
            .expect("the document model carries declared properties only")
            .kind()
    }
}

/// Resolves `reference` and builds the one report used by both renderings.
pub fn show_report(
    corpus: &Corpus,
    contract: &Contract,
    reference: &str,
    extra: &[Diagnostic],
) -> ShowReport {
    let mut collected = DiagnosticList::new();
    collected.extend(extra.iter().cloned());
    collected.extend(corpus.diagnostics().iter().cloned());
    let note = match corpus.resolve(reference) {
        Ok(note) => Some(note.clone()),
        Err(refused) => {
            collected.push(refused.diagnostic());
            None
        }
    };
    let counts = collected.counts();
    ShowReport {
        note,
        contract: contract.clone(),
        diagnostics: collected.sorted(),
        counts,
    }
}

/// Renders the selected note as SDK-owned plain text.
///
/// An unresolved reference has no document model to render and produces an empty string.
pub fn show_text(report: &ShowReport) -> String {
    let Some(note) = report.note() else {
        return String::new();
    };
    let binding = note.binding();
    let mut lines = vec![
        format!("path: {}", note.path()),
        format!(
            "title: {}",
            note.title().map_or_else(|| "-".to_owned(), one_line)
        ),
        format!("type: {}", binding.type_name().unwrap_or("-")),
        format!("bound by: {}", binding.bound_by()),
    ];
    lines.extend(properties_text(report, note));
    lines.extend(relationships_text(note));
    lines.extend(tags_text(note));
    lines.push("body:".to_owned());
    lines.push(note.body().to_owned());
    newline_terminated(lines.join("\n"))
}

fn properties_text(report: &ShowReport, note: &Note) -> Vec<String> {
    let mut lines = vec!["properties:".to_owned()];
    for property in note.properties() {
        let kind = report.kind(property.name()).as_str();
        lines.push(format!(
            "  {} ({kind}): {}",
            one_line(property.name()),
            value_text(property.value())
        ));
    }
    if note.properties().is_empty() {
        lines.push("  -".to_owned());
    }
    lines
}

fn relationships_text(note: &Note) -> Vec<String> {
    let mut lines = vec!["relationships:".to_owned()];
    for relationship in note.relationships() {
        for edge in relationship.edges() {
            lines.push(format!(
                "  {}: {} -> {}",
                one_line(relationship.predicate()),
                one_line(edge.written()),
                edge.target().map_or("-", |path| path.as_str())
            ));
        }
    }
    if note
        .relationships()
        .iter()
        .all(|relationship| relationship.edges().is_empty())
    {
        lines.push("  -".to_owned());
    }
    lines
}

fn tags_text(note: &Note) -> Vec<String> {
    let mut lines = vec!["tags:".to_owned()];
    lines.extend(
        note.tags()
            .iter()
            .map(|tag| format!("  - {}", one_line(tag))),
    );
    if note.tags().is_empty() {
        lines.push("  -".to_owned());
    }
    lines
}

fn newline_terminated(mut rendered: String) -> String {
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    rendered
}

fn value_text(value: &PropertyValue) -> String {
    match value {
        PropertyValue::Scalar(value) => one_line(value),
        PropertyValue::List(values) => format!("[{}]", joined(values)),
        PropertyValue::Record(record) => record_text(record),
        PropertyValue::RecordList(records) => {
            format!(
                "[{}]",
                records
                    .iter()
                    .map(record_text)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

fn joined(values: &[String]) -> String {
    values
        .iter()
        .map(|value| one_line(value))
        .collect::<Vec<_>>()
        .join(", ")
}

fn record_text(record: &RecordValue) -> String {
    let fields = record
        .fields()
        .iter()
        .map(|field| format!("{}: {}", one_line(field.name()), one_line(field.value())))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{{fields}}}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::parse_contract;
    use crate::note::read_corpus;
    use crate::vault::{SENTINEL, VaultRoot, tree::Tree};
    use std::fs;

    const CONTRACT: &str = concat!(
        "contract_version = 2\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[tags]\nproperty = \"labels\"\n",
        "\n[[type]]\nname = \"person\"\ncapabilities = [\"identity-bearing\"]\n",
        "\n  [[type.property]]\n  name = \"name\"\n  kind = \"string\"\n",
        "\n  [[type.property]]\n  name = \"labels\"\n  kind = \"list\"\n  of = \"string\"\n",
        "\n  [[type.property]]\n  name = \"identity\"\n  kind = \"record\"\n",
        "\n    [[type.property.field]]\n    name = \"given\"\n    kind = \"string\"\n",
        "\n  [[type.property]]\n  name = \"visits\"\n  kind = \"list\"\n  of = \"record\"\n",
        "\n    [[type.property.field]]\n    name = \"place\"\n    kind = \"string\"\n",
        "\n  [[type.relationship]]\n  predicate = \"knows\"\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    const ADA: &str = concat!(
        "---\n",
        "type: person\n",
        "name: Ada Lovelace\n",
        "labels: [role/founder, topic/computing]\n",
        "identity: {given: Augusta}\n",
        "visits: [{place: London}, {place: Paris}]\n",
        "knows: \"[[people/charles]]\"\n",
        "---\n",
        "# Ada Lovelace\n\nBody.\n",
    );

    fn corpus(tree: &Tree) -> (Contract, Corpus) {
        let root = tree.vault("vault");
        fs::write(root.join(SENTINEL), CONTRACT).expect("contract");
        fs::create_dir_all(root.join("people")).expect("notes directory");
        fs::write(root.join("people/ada.md"), ADA).expect("note");
        fs::write(root.join("people/charles.md"), "# Charles\n").expect("target");
        fs::write(root.join("capture.md"), "body").expect("capture");
        fs::write(root.join("other/ada.md"), "# Another Ada\n").unwrap_or_else(|_| {
            fs::create_dir_all(root.join("other")).expect("other directory");
            fs::write(root.join("other/ada.md"), "# Another Ada\n").expect("other note");
        });
        let contract = parse_contract(CONTRACT)
            .contract
            .expect("resolved contract");
        let corpus = read_corpus(&VaultRoot::new(root), &contract);
        (contract, corpus)
    }

    #[test]
    fn a_report_resolves_one_note_and_renders_every_document_model_part() {
        let tree = Tree::new("show-render");
        let (contract, corpus) = corpus(&tree);
        let report = show_report(&corpus, &contract, "people/ada", &[]);
        assert_eq!(
            report.note().map(|note| note.path().as_str()),
            Some("people/ada.md")
        );
        assert_eq!(report.counts(), SeverityCounts::zero());
        let text = show_text(&report);
        for expected in [
            "path: people/ada.md",
            "title: Ada Lovelace",
            "type: person",
            "name (string): Ada Lovelace",
            "labels (list): [role/founder, topic/computing]",
            "identity (record): {given: Augusta}",
            "visits (list): [{place: London}, {place: Paris}]",
            "knows: [[people/charles]] -> people/charles.md",
            "  - role/founder",
            "# Ada Lovelace\n\nBody.",
        ] {
            assert!(text.contains(expected), "missing {expected:?} from {text}");
        }
        let json = crate::report::show_json(&report);
        assert!(json.contains("\"report\": \"show\""));
        assert!(json.contains("\"bound_by\": \"declaration\""));
        assert!(json.contains("\"target\": \"people/charles.md\""));
    }

    #[test]
    fn missing_and_ambiguous_references_are_reports_without_notes() {
        let tree = Tree::new("show-refusal");
        let (contract, corpus) = corpus(&tree);
        for (reference, identifier) in [
            ("missing", "link.target-not-found"),
            ("ada", "link.ambiguous-reference"),
        ] {
            let report = show_report(&corpus, &contract, reference, &[]);
            assert!(report.note().is_none());
            assert_eq!(report.diagnostics().last().unwrap().id.as_str(), identifier);
            assert_eq!(show_text(&report), "");
            assert!(crate::report::show_json(&report).contains("\"note\": null"));
        }
    }

    #[test]
    fn an_untitled_empty_catch_all_renders_each_explicit_absence() {
        let tree = Tree::new("show-empty");
        let (contract, corpus) = corpus(&tree);
        let report = show_report(&corpus, &contract, "capture", &[]);
        let text = show_text(&report);
        assert!(text.contains("title: -\ntype: capture\nbound by: catch-all"));
        assert!(text.contains("properties:\n  -\nrelationships:\n  -\ntags:\n  -"));
        assert!(text.ends_with("body\n"));
    }
}
