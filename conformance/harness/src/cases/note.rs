//! Note-level corpus cases: binding, validation, and their derived negatives.

use dogtag::diagnostic::Severity;

use super::corpus::Corpus;
use super::derive::{self, NoteDerivation};
use super::expect::{Checked, Subject, require, require_clean, require_id, require_only};
use crate::transform::{
    DERIVED_NAMESPACE_PREFIX, DERIVED_TAGGED_TYPE, NoteEdit, NoteText, append_derived_tagged_type,
    edit_note_key,
};

/// The type name the unknown-type derivation writes: declared by no profile,
/// recognizable as the harness's in any diagnostic.
const DERIVED_UNDECLARED_TYPE: &str = "derived_undeclared_type";

/// `conforming-corpus-zero-diagnostics`.
pub fn conforming(corpus: &Corpus) -> Checked {
    require_clean(
        derive::notes(corpus)?.diagnostics(),
        Subject::new("the committed corpus"),
    )
}

/// `unknown-type-diagnostic`.
pub fn unknown_type(corpus: &Corpus) -> Checked {
    let (path, _) = first_discriminated(corpus)?;
    let derived = derive::derived_note(
        corpus,
        &NoteDerivation {
            label: "unknown-type",
            note: &path,
        },
        |text| {
            edit_note_key(
                &NoteText(text),
                &NoteEdit::Set("type", DERIVED_UNDECLARED_TYPE),
            )
        },
    )?;
    let notes = derive::notes(&derived)?;
    let subject = format!("the corpus holding `{path}` with an undeclared discriminator");
    require_only(
        notes.diagnostics(),
        "note.unknown-type",
        Subject::new(&subject),
    )?;
    require(
        notes.notes().len() == derive::notes(corpus)?.notes().len(),
        || "an undeclared discriminator must not eject the note from the corpus".to_string(),
    )
}

/// `missing-required-property-diagnostic`.
pub fn missing_required_property(corpus: &Corpus) -> Checked {
    let (path, property) = first_required_property(corpus)?;
    let derived = derive::derived_note(
        corpus,
        &NoteDerivation {
            label: "missing-required",
            note: &path,
        },
        |text| edit_note_key(&NoteText(text), &NoteEdit::Delete(&property)),
    )?;
    let subject = format!("the corpus after deleting `{property}` from `{path}`");
    require_only(
        derive::notes(&derived)?.diagnostics(),
        "note.missing-required-property",
        Subject::new(&subject),
    )
}

/// `undeclared-key-reported-as-info`.
pub fn undeclared_key_is_info(corpus: &Corpus) -> Checked {
    let (path, _) = first_discriminated(corpus)?;
    let derived = derive::derived_note(
        corpus,
        &NoteDerivation {
            label: "undeclared-key",
            note: &path,
        },
        |text| {
            edit_note_key(
                &NoteText(text),
                &NoteEdit::Insert("derived_undeclared_key", "1"),
            )
        },
    )?;
    let notes = derive::notes(&derived)?;
    let subject = format!("the corpus after inserting an undeclared key into `{path}`");
    require_only(
        notes.diagnostics(),
        "note.undeclared-property",
        Subject::new(&subject),
    )?;
    require(
        notes
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.severity == Severity::Info),
        || "an undeclared key is information, never a failure severity".to_string(),
    )
}

/// `untyped-note-binds-to-catch-all`.
pub fn untyped_binds_to_catch_all(corpus: &Corpus) -> Checked {
    let notes = derive::notes(corpus)?;
    let untyped = notes
        .notes()
        .iter()
        .find(|note| note.binding().discriminator().is_none())
        .ok_or_else(|| {
            "no committed note is untyped — the corpus no longer demonstrates the catch-all \
             binding"
                .to_string()
        })?;
    require(untyped.binding().bound_by() == "catch-all", || {
        format!(
            "`{}` carries no discriminator and must bind by catch-all, not `{}`",
            untyped.path().as_str(),
            untyped.binding().bound_by()
        )
    })
}

/// `required-tag-namespace-missing`.
pub fn required_namespace_missing(corpus: &Corpus) -> Checked {
    let derived = corpus.derived("required-namespace", append_derived_tagged_type)?;
    derive::plant(
        &derived,
        "derived-tagged.md",
        &format!("---\ntype: {DERIVED_TAGGED_TYPE}\n---\n# Derived\n"),
    )?;
    require_only(
        derive::notes(&derived)?.diagnostics(),
        "note.required-namespace-missing",
        Subject::new(
            "a planted note of the derived type carrying no tag in the required namespace",
        ),
    )
}

/// `closed-namespace-value-outside-vocabulary`.
pub fn closed_namespace_outside_vocabulary(corpus: &Corpus) -> Checked {
    let derived = corpus.derived("closed-namespace", append_derived_tagged_type)?;
    derive::plant(
        &derived,
        "derived-tagged.md",
        &format!(
            "---\ntype: {DERIVED_TAGGED_TYPE}\ntags: [{DERIVED_NAMESPACE_PREFIX}absent]\n---\n# \
             Derived\n"
        ),
    )?;
    require_id(
        derive::notes(&derived)?.diagnostics(),
        "note.tag-outside-vocabulary",
        Subject::new("a planted tag inside the derived namespace's prefix but outside its values"),
    )
}

/// The first note, by corpus order, carrying an explicit type discriminator.
fn first_discriminated(corpus: &Corpus) -> Result<(String, String), String> {
    let notes = derive::notes(corpus)?;
    notes
        .notes()
        .iter()
        .find_map(|note| {
            note.binding()
                .discriminator()
                .map(|discriminator| (note.path().as_str().to_owned(), discriminator.to_owned()))
        })
        .ok_or_else(|| "no committed note carries an explicit type discriminator".to_string())
}

/// The first note whose declared type requires a property the note carries.
fn first_required_property(corpus: &Corpus) -> Result<(String, String), String> {
    let contract = corpus.clean_contract()?;
    let notes = derive::notes(corpus)?;
    for note in notes.notes() {
        let Some(declared) = note
            .binding()
            .type_name()
            .and_then(|name| contract.type_named(name))
        else {
            continue;
        };
        for property in declared.properties() {
            if property.required() && note.property(property.name()).is_some() {
                return Ok((note.path().as_str().to_owned(), property.name().to_owned()));
            }
        }
    }
    Err("no committed note carries a required property to delete".to_string())
}

#[cfg(test)]
mod tests {
    use super::super::derive::plant;
    use super::*;

    const CONTRACT: &str = concat!(
        "contract_version = 2\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[[type]]\nname = \"person\"\ncapabilities = [\"identity-bearing\"]\n",
        "\n  [[type.property]]\n  name = \"name\"\n  kind = \"string\"\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    #[test]
    fn a_fully_typed_corpus_no_longer_demonstrates_the_catch_all() {
        let corpus = Corpus::holding("note-all-typed", CONTRACT);
        plant(&corpus, "a.md", "---\ntype: person\nname: A\n---\n").expect("a note");
        let detail =
            untyped_binds_to_catch_all(&corpus).expect_err("every note carries a discriminator");
        assert!(detail.contains("no committed note is untyped"), "{detail}");
    }

    #[test]
    fn a_corpus_of_untyped_notes_refuses_the_discriminator_derivations() {
        let corpus = Corpus::holding("note-untyped-only", CONTRACT);
        plant(&corpus, "a.md", "# Just capture\n").expect("a note");
        let detail = unknown_type(&corpus).expect_err("nothing carries a discriminator");
        assert!(
            detail.contains("no committed note carries an explicit type"),
            "{detail}"
        );
        let detail =
            missing_required_property(&corpus).expect_err("nothing carries a required property");
        assert!(
            detail.contains("no committed note carries a required property"),
            "{detail}"
        );
    }
}
