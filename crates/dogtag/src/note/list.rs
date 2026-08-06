//! Corpus enumeration and filtering.

use crate::contract::{Contract, LifecycleDecl, Ordinary};
use crate::diagnostic::{Diagnostic, DiagnosticList, KernelDiagnostic, VaultPath};
use crate::vault::VaultRoot;

use super::{Binding, Note, PropertyValue, read, resolve, traverse};

/// The composable filters accepted by corpus enumeration.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListFilter {
    /// Match the bound type exactly.
    pub type_name: Option<String>,
    /// Match one literal, complete tag exactly.
    pub tag: Option<String>,
    /// Match the declared lifecycle axis value exactly.
    pub lifecycle: Option<String>,
    /// Match the ordinary lifecycle state in its declared encoding.
    pub ordinary: bool,
}

/// One body-free summary returned by [`list`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteSummary {
    path: VaultPath,
    type_name: Option<String>,
    lifecycle: Option<String>,
}

impl NoteSummary {
    /// The note's vault-relative identity.
    pub fn path(&self) -> &VaultPath {
        &self.path
    }

    /// The type the note bound to, when it bound successfully.
    pub fn type_name(&self) -> Option<&str> {
        self.type_name.as_deref()
    }

    /// The axis value the note carries, when the contract and type declare an axis.
    pub fn lifecycle(&self) -> Option<&str> {
        self.lifecycle.as_deref()
    }
}

/// The result of enumerating a corpus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListResult {
    notes: Vec<NoteSummary>,
    diagnostics: Vec<Diagnostic>,
}

impl ListResult {
    /// Matching notes, sorted by vault-relative path.
    pub fn notes(&self) -> &[NoteSummary] {
        &self.notes
    }

    /// Everything enumeration reported, in diagnostic order.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Enumerates notes under `root`, applying every supplied filter with AND semantics.
pub fn list(root: &VaultRoot, contract: &Contract, filter: &ListFilter) -> ListResult {
    if let Some(refused) = axis_refusal(contract, filter) {
        return ListResult {
            notes: Vec::new(),
            diagnostics: vec![refused],
        };
    }

    let traversal = traverse(root);
    let mut diagnostics = DiagnosticList::new();
    diagnostics.extend(traversal.diagnostics().iter().cloned());
    let mut notes = Vec::new();
    for path in traversal.notes() {
        let read = read::summary(&root.path().join(path.as_str()), path, contract);
        notes.extend(read.note);
        diagnostics.extend(read.diagnostics);
    }
    diagnostics.extend(resolve::corpus(&mut notes, contract.dialect().links()));
    let notes = notes
        .iter()
        .filter(|note| matches(note, contract, filter))
        .map(|note| summarize(note, contract))
        .collect();
    ListResult {
        notes,
        diagnostics: diagnostics.sorted(),
    }
}

/// The refusal a lifecycle filter meets against a corpus that declares no axis.
///
/// Shared with `search`, whose filters are this module's under this module's
/// rules: one filter vocabulary, one refusal, whichever surface asks.
pub(super) fn axis_refusal(contract: &Contract, filter: &ListFilter) -> Option<Diagnostic> {
    (lifecycle_requested(filter) && matches!(contract.lifecycle(), LifecycleDecl::None)).then(
        || {
            Diagnostic::kernel(
                KernelDiagnostic::NoteLifecycleAxisAbsent,
                "this corpus declares no lifecycle axis to filter",
            )
        },
    )
}

fn lifecycle_requested(filter: &ListFilter) -> bool {
    filter.lifecycle.is_some() || filter.ordinary
}

/// Whether `note` satisfies every supplied filter, ANDed.
///
/// `pub(super)` for `search`, which composes the same four filters with its
/// text predicate rather than growing a second filter vocabulary.
pub(super) fn matches(note: &Note, contract: &Contract, filter: &ListFilter) -> bool {
    filter
        .type_name
        .as_deref()
        .is_none_or(|wanted| note.binding().type_name() == Some(wanted))
        && filter
            .tag
            .as_deref()
            .is_none_or(|wanted| note.tags().iter().any(|tag| tag == wanted))
        && lifecycle_matches(note, contract, filter)
}

fn lifecycle_matches(note: &Note, contract: &Contract, filter: &ListFilter) -> bool {
    if !lifecycle_requested(filter) {
        return true;
    }
    let LifecycleDecl::Axis { axis, ordinary } = contract.lifecycle() else {
        return true;
    };
    let Some(type_name) = note.binding().type_name() else {
        return false;
    };
    if !type_participates(contract, type_name, axis) {
        return false;
    }
    let value = note.property(axis).and_then(PropertyValue::scalar);
    filter
        .lifecycle
        .as_deref()
        .is_none_or(|wanted| value == Some(wanted))
        && ordinary_matches(value, ordinary, filter.ordinary)
}

fn type_participates(contract: &Contract, type_name: &str, axis: &str) -> bool {
    contract
        .type_named(type_name)
        .is_some_and(|kind| kind.property(axis).is_some())
}

fn ordinary_matches(value: Option<&str>, ordinary: &Ordinary, requested: bool) -> bool {
    !requested
        || match ordinary {
            Ordinary::Absent => value.is_none(),
            Ordinary::Value(wanted) => value == Some(wanted),
        }
}

fn summarize(note: &Note, contract: &Contract) -> NoteSummary {
    let lifecycle = contract.lifecycle().axis().and_then(|axis| {
        let type_name = note.binding().type_name()?;
        contract.type_named(type_name)?.property(axis)?;
        note.property(axis)?.scalar().map(str::to_owned)
    });
    NoteSummary {
        path: note.path().clone(),
        type_name: match note.binding() {
            Binding::Declared { name } | Binding::CatchAll { name } => Some(name.clone()),
            Binding::Unbound { .. } => None,
        },
        lifecycle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::parse_contract;
    use crate::report::{list_json, list_text};
    use crate::vault::{SENTINEL, VaultRoot, tree::Tree};
    use std::fs;

    const ABSENT: &str = concat!(
        "contract_version = 2\n\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\naxis = \"stage\"\nordinary = { absent = true }\n",
        "\n[tags]\nproperty = \"tags\"\n",
        "\n[[type]]\nname = \"work\"\ncapabilities = [\"identity-bearing\"]\n",
        "  [[type.property]]\n  name = \"stage\"\n  kind = \"enum\"\n  values = [\"active\", \"done\"]\n",
        "  [[type.property]]\n  name = \"tags\"\n  kind = \"list\"\n  of = \"string\"\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    fn corpus(label: &str, contract: &str) -> (Tree, VaultRoot, Contract) {
        let tree = Tree::new(label);
        let root = tree.vault("vault");
        fs::write(root.join(SENTINEL), contract).expect("contract");
        fs::write(
            root.join("a.md"),
            "---\ntype: work\nstage: active\ntags: [topic/a]\n---\n# ignored [[missing]]\n",
        )
        .expect("note");
        fs::write(
            root.join("b.md"),
            "---\ntype: work\ntags: [topic/b]\n---\n# ignored\n",
        )
        .expect("note");
        fs::write(root.join("c.md"), "capture\n").expect("note");
        let contract = parse_contract(contract).contract.expect("resolved");
        (tree, VaultRoot::new(root), contract)
    }

    fn paths(result: &ListResult) -> Vec<&str> {
        result
            .notes()
            .iter()
            .map(|note| note.path().as_str())
            .collect()
    }

    fn read_note(root: &VaultRoot, contract: &Contract, path: &str) -> Note {
        let relative = root.relative(&root.path().join(path)).expect("inside root");
        read::summary(&root.path().join(path), &relative, contract)
            .note
            .expect("readable note")
    }

    #[test]
    fn enumeration_is_sorted_body_free_and_every_filter_composes() {
        let (_tree, root, contract) = corpus("list-filters", ABSENT);
        let all = list(&root, &contract, &ListFilter::default());
        assert_eq!(paths(&all), ["a.md", "b.md", "c.md"]);
        assert!(all.diagnostics().is_empty());
        assert_eq!(all.notes()[0].type_name(), Some("work"));
        assert_eq!(all.notes()[0].lifecycle(), Some("active"));
        assert_eq!(all.notes()[2].type_name(), Some("capture"));
        assert_eq!(all.notes()[2].lifecycle(), None);

        let filtered = list(
            &root,
            &contract,
            &ListFilter {
                type_name: Some("work".into()),
                tag: Some("topic/a".into()),
                lifecycle: Some("active".into()),
                ordinary: false,
            },
        );
        assert_eq!(paths(&filtered), ["a.md"]);
        assert!(
            list(
                &root,
                &contract,
                &ListFilter {
                    tag: Some("topic".into()),
                    ..ListFilter::default()
                }
            )
            .notes()
            .is_empty()
        );
        for filter in [
            ListFilter {
                type_name: Some("missing".into()),
                ..ListFilter::default()
            },
            ListFilter {
                lifecycle: Some("done".into()),
                ..ListFilter::default()
            },
        ] {
            assert!(list(&root, &contract, &filter).notes().is_empty());
        }
        assert_eq!(
            paths(&list(
                &root,
                &contract,
                &ListFilter {
                    ordinary: true,
                    ..ListFilter::default()
                }
            )),
            ["b.md"]
        );
    }

    #[test]
    fn named_ordinary_and_unbound_notes_take_their_declared_meanings() {
        let named = ABSENT
            .replace(
                "ordinary = { absent = true }",
                "ordinary = { value = \"active\" }",
            )
            .replace(
                "name = \"stage\"\n  kind = \"enum\"",
                "name = \"stage\"\n  kind = \"enum\"\n  required = true",
            );
        let (_tree, root, contract) = corpus("list-named", &named);
        assert_eq!(
            paths(&list(
                &root,
                &contract,
                &ListFilter {
                    ordinary: true,
                    ..ListFilter::default()
                }
            )),
            ["a.md"]
        );
        fs::write(root.path().join("d.md"), "---\ntype: unknown\n---\n").expect("note");
        let result = list(&root, &contract, &ListFilter::default());
        assert!(result.notes().iter().any(|note| note.type_name().is_none()));
    }

    #[test]
    fn lifecycle_filters_refuse_a_declared_absence_and_renderings_share_the_result() {
        let none = ABSENT.replace(
            "axis = \"stage\"\nordinary = { absent = true }",
            "none = true",
        );
        let (_tree, root, contract) = corpus("list-none", &none);
        for filter in [
            ListFilter {
                lifecycle: Some("active".into()),
                ..ListFilter::default()
            },
            ListFilter {
                ordinary: true,
                ..ListFilter::default()
            },
        ] {
            let refused = list(&root, &contract, &filter);
            assert!(refused.notes().is_empty());
            assert_eq!(
                refused.diagnostics()[0].id.as_str(),
                "note.lifecycle-axis-absent"
            );
        }
        let result = list(&root, &contract, &ListFilter::default());
        assert_eq!(
            list_text(&result),
            "a.md\twork\nb.md\twork\nc.md\tcapture\n"
        );
        let json = list_json(&result, result.diagnostics());
        assert!(json.contains("\"report\": \"list\""));
        assert!(json.contains("\"lifecycle\": null"));

        let note = read_note(&root, &contract, "a.md");
        assert!(lifecycle_matches(&note, &contract, &ListFilter::default()));
        assert!(lifecycle_matches(
            &note,
            &contract,
            &ListFilter {
                lifecycle: Some("active".into()),
                ..ListFilter::default()
            }
        ));
    }

    #[test]
    fn nonparticipants_never_match_a_lifecycle_filter() {
        let (_tree, root, contract) = corpus("list-participation", ABSENT);
        let capture = read_note(&root, &contract, "c.md");
        assert!(!lifecycle_matches(
            &capture,
            &contract,
            &ListFilter {
                lifecycle: Some("active".into()),
                ..ListFilter::default()
            }
        ));
        fs::write(root.path().join("d.md"), "---\ntype: unknown\n---\n").expect("note");
        let unbound = read_note(&root, &contract, "d.md");
        assert!(!lifecycle_matches(
            &unbound,
            &contract,
            &ListFilter {
                ordinary: true,
                ..ListFilter::default()
            }
        ));
    }
}
