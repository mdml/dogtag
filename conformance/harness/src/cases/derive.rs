//! Note-level derivations: the corpus copies the negative note cases run
//! against, built through [`Corpus`]'s public surface.
//!
//! Each constructor performs two of the three assertions every derived case
//! owes — **the untransformed corpus validates with zero diagnostics** and
//! **the derivation changed something** — leaving the third, the expected
//! identifier, to the case that knows it.

use std::fs;

use crate::transform::Transformed;

use super::corpus::Corpus;
use super::expect::{Checked, require, require_clean};

/// One note-transformation derivation: which note, under what label.
pub struct NoteDerivation<'a> {
    /// The temp-tree label the derived copy carries.
    pub label: &'a str,
    /// The committed note the transformation targets.
    pub note: &'a str,
}

/// One planting derivation: new notes written into a fresh copy of a corpus
/// that was clean before them.
pub struct Planting<'a> {
    /// The temp-tree label the derived copy carries.
    pub label: &'a str,
    /// The notes to plant: vault-relative path, whole text.
    pub notes: &'a [(&'a str, &'a str)],
}

/// One duplicate-and-reference derivation: a note duplicated under a second
/// path, with a planted note referencing the now-ambiguous name.
pub struct Duplication<'a> {
    /// The temp-tree label the derived copy carries.
    pub label: &'a str,
    /// The committed note to duplicate.
    pub note: &'a str,
    /// The second path the duplicate lands under.
    pub second: &'a str,
    /// The planted note that carries the reference.
    pub planted: &'a str,
    /// The planted note's whole text.
    pub reference: &'a str,
}

/// The corpus read through the SDK against its own clean contract.
///
/// # Errors
///
/// A contract that does not load clean, or a root the SDK refuses.
pub fn notes(corpus: &Corpus) -> Result<dogtag::note::Corpus, String> {
    let contract = corpus.clean_contract()?;
    Ok(dogtag::note::read_corpus(&corpus.vault_root()?, &contract))
}

/// One note's committed text.
///
/// # Errors
///
/// A note that cannot be read, named by path.
pub fn note_text(corpus: &Corpus, note: &str) -> Result<String, String> {
    fs::read_to_string(corpus.root().join(note))
        .map_err(|error| format!("reading the note `{note}` failed: {error}"))
}

/// A second copy in which exactly one note has been transformed.
///
/// # Errors
///
/// A corpus that is not clean before transformation, a transform that could
/// not find its target or changed no bytes, or a filesystem failure.
pub fn derived_note(
    corpus: &Corpus,
    spec: &NoteDerivation<'_>,
    transform: impl Fn(&str) -> Transformed,
) -> Result<Corpus, String> {
    let NoteDerivation { label, note } = spec;
    require_clean(
        notes(corpus)?.diagnostics(),
        &format!("the corpus before `{label}`"),
    )?;
    let original = note_text(corpus, note)?;
    let transformed = transform(&original).map_err(|failure| failure.to_string())?;
    require(transformed != original, || {
        format!(
            "the `{label}` transformation left `{note}` byte-identical, so the derived case \
             would test nothing"
        )
    })?;
    let derived = Corpus::copy_of(corpus.root(), label)?;
    fs::write(derived.root().join(note), transformed)
        .map_err(|error| format!("writing the derived note `{note}` failed: {error}"))?;
    Ok(derived)
}

/// A second copy holding the planted notes beside everything committed.
///
/// The retrieval cases derive their needles this way: an invented word
/// planted in known notes is what makes "exactly those notes" assertable
/// against any profile's corpus, whatever its vocabulary.
///
/// # Errors
///
/// A corpus that is not clean before derivation, or a filesystem failure.
pub fn derived_planting(corpus: &Corpus, spec: &Planting<'_>) -> Result<Corpus, String> {
    let Planting { label, notes: new } = spec;
    require_clean(
        notes(corpus)?.diagnostics(),
        &format!("the corpus before `{label}`"),
    )?;
    let derived = Corpus::copy_of(corpus.root(), label)?;
    for (note, text) in *new {
        plant(&derived, note, text)?;
    }
    Ok(derived)
}

/// A second copy shaped by a [`Duplication`].
///
/// # Errors
///
/// A corpus that is not clean before derivation, or a filesystem failure.
pub fn derived_duplicate(corpus: &Corpus, spec: &Duplication<'_>) -> Result<Corpus, String> {
    let Duplication {
        label,
        note,
        second,
        planted,
        reference,
    } = spec;
    require_clean(
        notes(corpus)?.diagnostics(),
        &format!("the corpus before `{label}`"),
    )?;
    let original = note_text(corpus, note)?;
    let derived = Corpus::copy_of(corpus.root(), label)?;
    plant(&derived, second, &original)?;
    plant(&derived, planted, reference)?;
    Ok(derived)
}

/// Writes a new note into a derived copy.
///
/// # Errors
///
/// Any filesystem failure, named with the note's path.
pub fn plant(corpus: &Corpus, note: &str, text: &str) -> Checked {
    let target = corpus.root().join(note);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("creating `{note}`'s directory failed: {error}"))?;
    }
    fs::write(&target, text).map_err(|error| format!("planting `{note}` failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTRACT: &str = concat!(
        "contract_version = 3\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[[type]]\nname = \"person\"\ncapabilities = [\"identity-bearing\"]\n",
        "\n  [[type.property]]\n  name = \"name\"\n  kind = \"string\"\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    fn tiny(label: &str) -> Corpus {
        let corpus = Corpus::holding(label, CONTRACT);
        plant(&corpus, "a.md", "---\ntype: person\nname: A\n---\n# A\n").expect("a note");
        corpus
    }

    #[test]
    fn an_identity_transformation_is_refused_as_testing_nothing() {
        let corpus = tiny("derive-identity");
        let detail = derived_note(
            &corpus,
            &NoteDerivation {
                label: "identity",
                note: "a.md",
            },
            |text| Ok(text.to_owned()),
        )
        .expect_err("a transformation that changes nothing derives nothing");
        assert!(detail.contains("byte-identical"), "{detail}");
        assert!(detail.contains("would test nothing"), "{detail}");
    }

    #[test]
    fn a_note_that_is_not_there_cannot_be_read_or_derived() {
        let corpus = tiny("derive-missing-note");
        let detail = note_text(&corpus, "missing.md").expect_err("no such note");
        assert!(detail.contains("missing.md"), "{detail}");
        let detail = derived_note(
            &corpus,
            &NoteDerivation {
                label: "missing",
                note: "missing.md",
            },
            |text| Ok(text.to_owned()),
        )
        .expect_err("a missing note derives nothing");
        assert!(detail.contains("missing.md"), "{detail}");
    }

    #[test]
    fn a_corpus_that_is_not_clean_derives_nothing() {
        let corpus = tiny("derive-dirty");
        plant(&corpus, "broken.md", "---\ntype: undeclared_kind\n---\n").expect("a broken note");
        let refusal = derived_note(
            &corpus,
            &NoteDerivation {
                label: "dirty",
                note: "a.md",
            },
            |text| Ok(format!("{text}\n")),
        )
        .expect_err("a corpus that is not clean before derivation proves nothing");
        assert!(refusal.contains("the corpus before `dirty`"), "{refusal}");
        let refusal = derived_duplicate(
            &corpus,
            &Duplication {
                label: "dirty-duplicate",
                note: "a.md",
                second: "b/a.md",
                planted: "ref.md",
                reference: "See [[a]].\n",
            },
        )
        .expect_err("the duplicate derivation owes the same cleanliness");
        assert!(
            refusal.contains("the corpus before `dirty-duplicate`"),
            "{refusal}"
        );
        let refusal = derived_planting(
            &corpus,
            &Planting {
                label: "dirty-planting",
                notes: &[("planted.md", "# Planted\n")],
            },
        )
        .expect_err("the planting derivation owes the same cleanliness");
        assert!(
            refusal.contains("the corpus before `dirty-planting`"),
            "{refusal}"
        );
    }

    #[test]
    fn a_planting_lands_every_note_it_was_given() {
        let corpus = tiny("derive-planting");
        let derived = derived_planting(
            &corpus,
            &Planting {
                label: "planting",
                notes: &[
                    ("derived-planting/one.md", "# One\n"),
                    ("derived-planting/two.md", "# Two\n"),
                ],
            },
        )
        .expect("a clean corpus accepts a planting");
        let read = notes(&derived).expect("the derived corpus reads");
        assert_eq!(read.notes().len(), 3, "the committed note and both planted");
    }
}
