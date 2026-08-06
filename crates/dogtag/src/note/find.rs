//! Entity lookup: M3's reference resolution exposed as a verb.
//!
//! `find` answers the question daily reading asks most — *which note is this
//! name?* — with a **case-insensitive** match over note names and aliases,
//! narrowed by a type when one is given. A path-qualified reference — one
//! carrying a `/` or a trailing `.md` — resolves exactly, under the standing
//! routing rule every other door obeys.
//!
//! The outcomes are resolution's outcomes, not a new contract. An unambiguous
//! match answers the document-model summary; an ambiguous one raises
//! `link.ambiguous-reference` with every candidate as related evidence — the
//! same identifier, shape, and severity `show` raises, because it is the same
//! semantic event at a different door — and a name nothing bears is
//! `link.target-not-found`. A caller who wants the candidate list is served
//! by the diagnostic: the enumeration is the related evidence, verbatim.
//! `find` mints no diagnostic area of its own.

use crate::contract::Contract;
use crate::diagnostic::{Diagnostic, DiagnosticList, KernelDiagnostic, VaultPath};
use crate::vault::VaultRoot;

use super::model::Note;
use super::{Corpus, NoteSummary, index, list, resolve};

/// The result of asking a corpus to find one name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindResult {
    note: Option<NoteSummary>,
    diagnostics: Vec<Diagnostic>,
}

impl FindResult {
    /// The one note the name resolved to, or `None` when resolution refused.
    pub fn note(&self) -> Option<&NoteSummary> {
        self.note.as_ref()
    }

    /// Everything the lookup reported, in diagnostic order.
    ///
    /// The lookup is a full validation pass like `search`'s scan, so a broken
    /// corpus's findings are here on every run — prose-only findings
    /// included; an ambiguous name adds the refusal carrying every candidate
    /// as related evidence.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Finds the one note `name` resolves to, narrowed by `type_name` when given.
///
/// A bare name matches note names and aliases case-insensitively; a
/// path-qualified reference resolves exactly. The lookup reads the corpus
/// through the same full loading, traversal, and validation path `search`
/// scans by, so the two retrieval verbs agree about the same corpus's health:
/// a finding only prose can raise surfaces at this door too, even though the
/// *answer* is the body-free summary.
pub fn find(
    root: &VaultRoot,
    contract: &Contract,
    name: &str,
    type_name: Option<&str>,
) -> FindResult {
    let Corpus { notes, diagnostics } = super::read_corpus(root, contract);
    let mut collected = DiagnosticList::new();
    collected.extend(diagnostics);
    let candidates = candidates(&notes, name, type_name);
    let note = match candidates[..] {
        [one] => Some(list::summarize(one, contract)),
        [] => {
            collected.push(absent(name, type_name));
            None
        }
        _ => {
            let paths: Vec<VaultPath> = candidates.iter().map(|note| note.path().clone()).collect();
            collected.push(resolve::ambiguous(name, &paths));
            None
        }
    };
    FindResult {
        note,
        diagnostics: collected.sorted(),
    }
}

/// Every note `name` could mean, narrowed by the type filter.
fn candidates<'a>(notes: &'a [Note], name: &str, type_name: Option<&str>) -> Vec<&'a Note> {
    let matched: Vec<&Note> = if index::path_qualified(name) {
        exactly(notes, name)
    } else {
        let wanted = name.to_lowercase();
        notes.iter().filter(|note| bears(note, &wanted)).collect()
    };
    matched
        .into_iter()
        .filter(|note| type_name.is_none_or(|wanted| note.binding().type_name() == Some(wanted)))
        .collect()
}

/// The note a path-qualified reference names, under the standing exact rule.
fn exactly<'a>(notes: &'a [Note], reference: &str) -> Vec<&'a Note> {
    match index::Index::of(notes).resolve(reference) {
        index::Resolution::One(at) => vec![&notes[at]],
        // A path resolves exactly or not at all; ambiguity has no path form.
        _ => Vec::new(),
    }
}

/// Whether the note's name or one of its aliases is `wanted`, lowercased.
fn bears(note: &Note, wanted: &str) -> bool {
    note.name().to_lowercase() == wanted
        || note
            .aliases()
            .iter()
            .any(|alias| alias.to_lowercase() == wanted)
}

/// A name that resolved to nothing, said with the type narrowing it carried.
///
/// The identifier and severity are `link.target-not-found`'s at every door;
/// only the message knows a type filter was in play, because a note the
/// filter excluded is not honestly described as absent from the vault.
fn absent(name: &str, type_name: Option<&str>) -> Diagnostic {
    match type_name {
        None => resolve::not_found(name),
        Some(wanted) => Diagnostic::kernel(
            KernelDiagnostic::LinkTargetNotFound,
            format!("`{name}` names no note of type `{wanted}` in this vault"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::parse_contract;
    use crate::vault::{SENTINEL, tree::Tree};
    use std::fs;

    const CONTRACT: &str = concat!(
        "contract_version = 2\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\naxis = \"stage\"\nordinary = { absent = true }\n",
        "\n[[type]]\nname = \"work\"\ncapabilities = [\"identity-bearing\"]\n",
        "  [[type.property]]\n  name = \"stage\"\n  kind = \"enum\"\n  values = [\"active\", \"done\"]\n",
        "  [[type.property]]\n  name = \"aliases\"\n  kind = \"list\"\n  of = \"string\"\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    fn found(
        label: &str,
        notes: &[(&str, &str)],
        name: &str,
        type_name: Option<&str>,
    ) -> FindResult {
        let tree = Tree::new(label);
        let root = tree.vault("vault");
        fs::write(root.join(SENTINEL), CONTRACT).expect("a contract this test owns");
        for (relative, text) in notes {
            let path = root.join(relative);
            let parent = path.parent().expect("a note under the root has a parent");
            fs::create_dir_all(parent).expect("a directory this test owns");
            fs::write(path, text).expect("a note this test owns");
        }
        let resolved = parse_contract(CONTRACT).contract.expect("resolved");
        find(&VaultRoot::new(root), &resolved, name, type_name)
    }

    fn ids(result: &FindResult) -> Vec<&str> {
        result
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect()
    }

    #[test]
    fn an_unambiguous_name_answers_the_document_model_summary() {
        let notes: &[(&str, &str)] = &[
            ("people/ada.md", "---\ntype: work\nstage: active\n---\n"),
            ("people/babbage.md", "# Charles\n"),
        ];
        let result = found("find-one", notes, "ada", None);
        assert!(result.diagnostics().is_empty());
        let note = result.note().expect("one note bears it");
        assert_eq!(note.path().as_str(), "people/ada.md");
        assert_eq!(note.type_name(), Some("work"));
        assert_eq!(note.lifecycle(), Some("active"));
    }

    #[test]
    fn the_name_match_is_case_insensitive_over_names_and_aliases() {
        let notes: &[(&str, &str)] = &[
            ("people/ada.md", "---\ntype: work\n---\n"),
            (
                "engines/analytical.md",
                "---\ntype: work\naliases: [\"The Engine\"]\n---\n",
            ),
        ];
        let by_name = found("find-case", notes, "ADA", None);
        assert_eq!(
            by_name.note().map(|note| note.path().as_str()),
            Some("people/ada.md")
        );
        let by_alias = found("find-alias", notes, "the engine", None);
        assert_eq!(
            by_alias.note().map(|note| note.path().as_str()),
            Some("engines/analytical.md")
        );
    }

    #[test]
    fn an_ambiguous_name_is_the_same_refusal_show_raises_with_every_candidate() {
        let notes: &[(&str, &str)] = &[
            ("2025/daily.md", "# Daily\n"),
            ("2026/daily.md", "# Daily\n"),
        ];
        let result = found("find-ambiguous", notes, "daily", None);
        assert!(result.note().is_none());
        assert_eq!(ids(&result), ["link.ambiguous-reference"]);
        let refused = &result.diagnostics()[0];
        assert!(refused.message.contains("`daily`"), "{}", refused.message);
        let evidence: Vec<&str> = refused
            .related
            .iter()
            .map(|related| {
                related
                    .location
                    .as_ref()
                    .expect("located")
                    .file
                    .display_path()
            })
            .collect();
        assert_eq!(evidence, ["2025/daily.md", "2026/daily.md"]);
    }

    #[test]
    fn an_alias_shared_with_a_name_is_ambiguity_like_any_other() {
        let notes: &[(&str, &str)] = &[
            ("engine.md", "# The file named engine\n"),
            (
                "engines/analytical.md",
                "---\ntype: work\naliases: [Engine]\n---\n",
            ),
        ];
        let result = found("find-cross-plane", notes, "engine", None);
        assert!(result.note().is_none());
        assert_eq!(ids(&result), ["link.ambiguous-reference"]);
        assert_eq!(result.diagnostics()[0].related.len(), 2);
    }

    #[test]
    fn a_name_nothing_bears_is_target_not_found() {
        let result = found("find-absent", &[("ada.md", "# Ada\n")], "babbage", None);
        assert!(result.note().is_none());
        assert_eq!(ids(&result), ["link.target-not-found"]);
        assert!(
            result.diagnostics()[0]
                .message
                .contains("names no note in this vault")
        );
    }

    #[test]
    fn the_type_filter_narrows_the_candidates_before_anything_is_decided() {
        let notes: &[(&str, &str)] = &[
            ("2025/daily.md", "---\ntype: work\n---\n"),
            ("2026/daily.md", "# untyped\n"),
        ];
        let narrowed = found("find-narrowed", notes, "daily", Some("work"));
        assert_eq!(
            narrowed.note().map(|note| note.path().as_str()),
            Some("2025/daily.md"),
            "the ambiguity dissolves under the filter"
        );
        let excluded = found("find-excluded", notes, "daily", Some("person"));
        assert!(excluded.note().is_none());
        let message = &excluded.diagnostics()[0].message;
        assert_eq!(ids(&excluded), ["link.target-not-found"]);
        assert!(message.contains("of type `person`"), "{message}");
    }

    #[test]
    fn a_path_qualified_reference_resolves_exactly_and_case_sensitively() {
        let notes: &[(&str, &str)] = &[
            ("2025/daily.md", "# Daily\n"),
            ("2026/daily.md", "# Daily\n"),
            ("welcome.md", "# Welcome\n"),
        ];
        let by_path = found("find-path", notes, "2025/daily", None);
        assert_eq!(
            by_path.note().map(|note| note.path().as_str()),
            Some("2025/daily.md"),
            "the path picks one bearer of an otherwise ambiguous name"
        );
        let by_extension = found("find-root-path", notes, "welcome.md", None);
        assert_eq!(
            by_extension.note().map(|note| note.path().as_str()),
            Some("welcome.md"),
            "a trailing .md is path-qualified even at the root"
        );
        let cased = found("find-path-case", notes, "2025/Daily", None);
        assert!(cased.note().is_none());
        assert_eq!(ids(&cased), ["link.target-not-found"]);
    }

    #[test]
    fn a_broken_corpus_surfaces_its_diagnostics_on_every_lookup() {
        let notes: &[(&str, &str)] = &[
            ("ada.md", "# Ada\n"),
            ("broken.md", "---\ntype: nothing\n---\n"),
        ];
        let result = found("find-broken", notes, "ada", None);
        assert!(result.note().is_some());
        assert_eq!(ids(&result), ["note.unknown-type"]);
    }

    #[test]
    fn a_finding_only_prose_can_raise_surfaces_at_this_door_too() {
        // The lookup is a full validation pass like search's scan: the two
        // retrieval verbs must not disagree about the same corpus's health,
        // so an ambiguous prose reference — a finding the body-free summary
        // walk cannot see — is reported here as everywhere else.
        let notes: &[(&str, &str)] = &[
            ("ada.md", "# Ada\n\nSee [[daily]].\n"),
            ("2025/daily.md", "# Daily\n"),
            ("2026/daily.md", "# Daily\n"),
        ];
        let result = found("find-prose-ambiguity", notes, "ada", None);
        assert!(result.note().is_some());
        assert_eq!(ids(&result), ["link.ambiguous-reference"]);
    }

    #[test]
    fn a_find_result_clones_compares_and_formats() {
        let result = found("find-derives", &[("ada.md", "# Ada\n")], "ada", None);
        let copy = result.clone();
        assert_eq!(copy, result);
        assert_ne!(
            result,
            found("find-derives-absent", &[("ada.md", "# Ada\n")], "b", None)
        );
        assert!(format!("{result:?}").contains("ada.md"));
    }

    #[test]
    fn renderings_share_the_result_and_a_refusal_renders_no_note() {
        let notes: &[(&str, &str)] = &[("people/ada.md", "---\ntype: work\nstage: active\n---\n")];
        let result = found("find-render", notes, "ada", None);
        assert_eq!(
            crate::report::find_text(&result),
            "people/ada.md\twork\tactive\n"
        );
        let json = crate::report::find_json(&result, result.diagnostics());
        assert!(json.starts_with("{\n  \"schema_version\": 3,\n  \"report\": \"find\",\n"));
        assert!(json.contains("\"note\": {"));
        assert!(json.contains("\"lifecycle\": \"active\""));
        let refused = found("find-render-refusal", notes, "absent", None);
        assert_eq!(crate::report::find_text(&refused), "");
        let refusal = crate::report::find_json(&refused, refused.diagnostics());
        assert!(refusal.contains("\"note\": null"));
        assert!(refusal.contains("link.target-not-found"));
        let again = crate::report::find_json(&result, result.diagnostics());
        assert_eq!(json, again, "identical input renders identical bytes");
    }
}
