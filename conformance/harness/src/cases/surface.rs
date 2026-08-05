//! Surface cases: the shared document-model shape behind `show`, and the
//! declaration-answered lifecycle filter behind `list`.

use std::collections::BTreeSet;

use dogtag::contract::Ordinary;
use dogtag::note::{ListFilter, list};
use dogtag::report::show_report;

use super::corpus::Corpus;
use super::derive;
use super::expect::{Checked, Subject, require, require_clean, require_id};
use crate::transform::replace_lifecycle_with_none;

/// `show-returns-document-model`.
///
/// Every committed note renders through the one shared shape; asserting over
/// the whole corpus is what makes the shape's profile-independence a fact
/// rather than a sample.
pub fn show_returns_document_model(corpus: &Corpus) -> Checked {
    let contract = corpus.clean_contract()?;
    let notes = derive::notes(corpus)?;
    for note in notes.notes() {
        let path = note.path().as_str();
        // A root-level path carries no separator, so the reference grammar
        // reads it as a bare *name* — `.md` included — which names nothing.
        // Resolving such a note by its name is the workaround the grammar
        // leaves; the gap itself is recorded for adjudication.
        let reference = if path.contains('/') {
            path
        } else {
            note.name()
        };
        let report = show_report(&notes, &contract, reference, &[]);
        require_clean(report.diagnostics(), Subject::new(path))?;
        let shown = report
            .note()
            .ok_or_else(|| format!("`{path}` was shown and yet carries no document model"))?;
        require(shown.path().as_str() == path, || {
            format!(
                "`{path}`'s identity is its path; the model says `{}`",
                shown.path().as_str()
            )
        })?;
        require(!shown.binding().bound_by().is_empty(), || {
            format!("`{path}` must report how it bound")
        })?;
        for property in shown.properties() {
            require(!property.name().is_empty(), || {
                format!("`{path}` carries a property with no declared name")
            })?;
        }
        let _ = shown.body();
        let _ = shown.relationships();
    }
    Ok(())
}

/// `list-filters-by-declared-lifecycle-axis`.
pub fn list_filters_by_axis(corpus: &Corpus) -> Checked {
    let contract = corpus.clean_contract()?;
    let root = corpus.vault_root()?;
    let everything = list(&root, &contract, &unfiltered());
    require_clean(
        everything.diagnostics(),
        Subject::new("the unfiltered listing"),
    )?;

    let ordinary = contract
        .lifecycle()
        .ordinary()
        .ok_or_else(|| "a built profile declares a lifecycle axis; none was found".to_string())?;
    let expected: BTreeSet<&str> = everything
        .notes()
        .iter()
        .filter(|summary| match ordinary {
            Ordinary::Absent => summary.lifecycle().is_none() && summary.type_name().is_some(),
            Ordinary::Value(value) => summary.lifecycle() == Some(value.as_str()),
        })
        .map(|summary| summary.path().as_str())
        .collect();
    let filter = ListFilter {
        ordinary: true,
        ..unfiltered()
    };
    let filtered = list(&root, &contract, &filter);
    let answered: BTreeSet<&str> = filtered
        .notes()
        .iter()
        .map(|summary| summary.path().as_str())
        .collect();
    require(answered == expected, || {
        format!("the ordinary filter answered {answered:?} where the declaration says {expected:?}")
    })?;

    if let Some(value) = first_non_ordinary_value(corpus, ordinary)? {
        let filter = ListFilter {
            lifecycle: Some(value.clone()),
            ..unfiltered()
        };
        let outside = list(&root, &contract, &filter);
        require(
            outside
                .notes()
                .iter()
                .all(|summary| summary.lifecycle() == Some(value.as_str())),
            || format!("filtering for `{value}` answered a note not in that state"),
        )?;
    }

    let derived = corpus.derived("lifecycle-none", replace_lifecycle_with_none)?;
    let contract = derived.clean_contract()?;
    let refused = list(
        &derived.vault_root()?,
        &contract,
        &ListFilter {
            ordinary: true,
            ..unfiltered()
        },
    );
    require_id(
        refused.diagnostics(),
        "note.lifecycle-axis-absent",
        Subject::new("a lifecycle filter against a corpus declaring no axis"),
    )
}

fn unfiltered() -> ListFilter {
    ListFilter {
        type_name: None,
        tag: None,
        lifecycle: None,
        ordinary: false,
    }
}

/// A declared non-ordinary axis value some note actually carries, when the
/// corpus holds one; `starter`'s corpus is legitimately all-ordinary.
fn first_non_ordinary_value(
    corpus: &Corpus,
    ordinary: &Ordinary,
) -> Result<Option<String>, String> {
    let notes = derive::notes(corpus)?;
    Ok(notes
        .notes()
        .iter()
        .filter_map(|note| note.property(contract_axis(corpus)?.as_str()))
        .filter_map(|value| value.scalar())
        .find(|value| match ordinary {
            Ordinary::Absent => true,
            Ordinary::Value(named) => value != named,
        })
        .map(str::to_owned))
}

fn contract_axis(corpus: &Corpus) -> Option<String> {
    corpus
        .clean_contract()
        .ok()?
        .lifecycle()
        .axis()
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::super::derive::plant;
    use super::*;

    const NONE_CONTRACT: &str = concat!(
        "contract_version = 2\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    #[test]
    fn a_corpus_declaring_no_axis_refuses_the_filter_case_up_front() {
        let corpus = Corpus::holding("surface-no-axis", NONE_CONTRACT);
        plant(&corpus, "a.md", "# Capture\n").expect("a note");
        let detail =
            list_filters_by_axis(&corpus).expect_err("the case needs a declared axis to filter by");
        assert!(detail.contains("none was found"), "{detail}");
    }
}
