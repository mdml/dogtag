//! Reference-resolution cases: bare names, ambiguity, path qualification, and
//! the dangling typed link.

use std::collections::HashMap;

use super::corpus::Corpus;
use super::derive::{self, Duplication, NoteDerivation};
use super::expect::{Checked, Subject, require, require_only};
use crate::transform::{NoteEdit, NoteText, edit_note_key};

/// `dangling-typed-link-diagnostic`.
pub fn dangling_typed_link(corpus: &Corpus) -> Checked {
    let (path, predicate) = first_relationship(corpus)?;
    let derived = derive::derived_note(
        corpus,
        &NoteDerivation {
            label: "dangling-link",
            note: &path,
        },
        |text| {
            edit_note_key(
                &NoteText(text),
                &NoteEdit::Set(&predicate, "\"[[derived-nowhere]]\""),
            )
        },
    )?;
    let subject = format!("the corpus after repointing `{predicate}` in `{path}` at nothing");
    require_only(
        derive::notes(&derived)?.diagnostics(),
        "link.dangling-typed-link",
        Subject::new(&subject),
    )
}

/// `bare-name-link-resolves-when-unambiguous`.
pub fn bare_name_resolves(corpus: &Corpus) -> Checked {
    let (name, path) = first_unique_name(corpus)?;
    let notes = derive::notes(corpus)?;
    let resolved = notes.resolve(&name).map_err(|unresolved| {
        format!(
            "the unique bare name `{name}` did not resolve: {}",
            unresolved.diagnostic().message
        )
    })?;
    require(resolved.path().as_str() == path, || {
        format!(
            "`{name}` resolved to `{}` rather than the one note bearing it, `{path}`",
            resolved.path().as_str()
        )
    })
}

/// `ambiguous-bare-name-yields-link-diagnostic`.
pub fn ambiguous_bare_name(corpus: &Corpus) -> Checked {
    let (name, path) = first_unique_name(corpus)?;
    let file = path.rsplit('/').next().unwrap_or(&path);
    let second = format!("derived-second/{file}");
    let reference = format!("# A derived reference\n\nSee [[{name}]].\n");
    let derived = derive::derived_duplicate(
        corpus,
        &Duplication {
            label: "ambiguous-name",
            note: &path,
            second: &second,
            planted: "derived-reference.md",
            reference: &reference,
        },
    )?;
    let notes = derive::notes(&derived)?;
    require(
        notes.notes().len() == derive::notes(corpus)?.notes().len() + 2,
        || {
            "the corpus must open and read normally with the duplicate and the reference in it"
                .to_string()
        },
    )?;
    require(
        notes
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.id.as_str() == "link.ambiguous-reference")
            .all(|diagnostic| diagnostic.location.is_some()),
        || {
            "the ambiguity diagnostic is addressed to the reference, so it carries its location"
                .to_string()
        },
    )?;
    let unresolved = match notes.resolve(&name) {
        Err(unresolved) => unresolved,
        Ok(resolved) => {
            return Err(format!(
                "the now-ambiguous name `{name}` resolved to `{}` instead of refusing",
                resolved.path().as_str()
            ));
        }
    };
    let diagnostic = unresolved.diagnostic();
    require(diagnostic.id.as_str() == "link.ambiguous-reference", || {
        format!(
            "the ambiguous reference reported `{}` rather than the ambiguity identifier",
            diagnostic.id.as_str()
        )
    })
}

/// `path-qualified-link-resolves`.
///
/// The note this runs against is a **nested** one, because the last assertion
/// drops the `.md` from its path and requires the remainder to resolve. For a
/// nested note that remainder still carries a `/` and is still path-qualified;
/// for a root-level note it carries neither a `/` nor the extension, which
/// makes it the *bare name* — and the derivation has just given that name a
/// second bearer, so it resolves to nothing by design. Running the assertion
/// there would report the bare-name rule working correctly as a failure of the
/// path-qualified one.
pub fn path_qualified_resolves(corpus: &Corpus) -> Checked {
    let (name, path) = first_unique_nested_name(corpus)?;
    let file = path.rsplit('/').next().unwrap_or(&path);
    let second = format!("derived-second/{file}");
    let reference = format!("# A derived reference\n\nSee [[{name}]].\n");
    let derived = derive::derived_duplicate(
        corpus,
        &Duplication {
            label: "path-qualified",
            note: &path,
            second: &second,
            planted: "derived-reference.md",
            reference: &reference,
        },
    )?;
    let notes = derive::notes(&derived)?;
    for target in [&path, &second] {
        let resolved = notes.resolve(target).map_err(|unresolved| {
            format!(
                "the path-qualified `{target}` did not resolve: {}",
                unresolved.diagnostic().message
            )
        })?;
        require(resolved.path().as_str() == *target, || {
            format!(
                "`{target}` resolved to `{}` — identity is the path, exactly",
                resolved.path().as_str()
            )
        })?;
    }
    let trimmed = path.trim_end_matches(".md");
    let resolved = notes
        .resolve(trimmed)
        .map_err(|unresolved| did_not_append_md(trimmed, &unresolved.diagnostic().message))?;
    require(resolved.path().as_str() == path, || {
        format!(
            "`{trimmed}` (no extension) resolved to `{}` rather than `{path}`",
            resolved.path().as_str()
        )
    })
}

fn did_not_append_md(reference: &str, message: &str) -> String {
    format!("the extensionless path-qualified `{reference}` did not resolve: {message}")
}

/// The first note, by corpus order, whose declared relationship the dangling
/// derivation can repoint.
fn first_relationship(corpus: &Corpus) -> Result<(String, String), String> {
    let notes = derive::notes(corpus)?;
    notes
        .notes()
        .iter()
        .find_map(|note| {
            note.relationships()
                .iter()
                .find(|relationship| !relationship.edges().is_empty())
                .map(|relationship| {
                    (
                        note.path().as_str().to_owned(),
                        relationship.predicate().to_owned(),
                    )
                })
        })
        .ok_or_else(|| "no committed note carries a typed relationship".to_string())
}

/// The first note, by corpus order, whose bare name no other note shares.
///
/// `pub(super)` because `find`'s ambiguity case derives its duplicate from
/// the same starting point: a name that is unique until the case doubles it.
pub(super) fn first_unique_name(corpus: &Corpus) -> Result<(String, String), String> {
    unique_name(corpus, |_| true)?
        .ok_or_else(|| "every committed name is shared — nothing is unambiguous".to_string())
}

/// The same, narrowed to a note that sits in a directory rather than at the
/// vault root — so its path minus the `.md` extension is still path-qualified.
fn first_unique_nested_name(corpus: &Corpus) -> Result<(String, String), String> {
    unique_name(corpus, |path| path.contains('/'))?.ok_or_else(|| {
        "no committed note with an unshared name sits below the vault root, so no extensionless \
         path-qualified spelling exists to resolve"
            .to_string()
    })
}

/// The first note, by corpus order, whose bare name no other note shares and
/// whose path `admissible` accepts.
fn unique_name(
    corpus: &Corpus,
    admissible: fn(&str) -> bool,
) -> Result<Option<(String, String)>, String> {
    let notes = derive::notes(corpus)?;
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for note in notes.notes() {
        *counts.entry(note.name()).or_default() += 1;
    }
    Ok(notes
        .notes()
        .iter()
        .find(|note| counts[note.name()] == 1 && admissible(note.path().as_str()))
        .map(|note| (note.name().to_owned(), note.path().as_str().to_owned())))
}

#[cfg(test)]
mod tests {
    use super::super::derive::plant;
    use super::*;

    const CONTRACT: &str = concat!(
        "contract_version = 3\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[[type]]\nname = \"person\"\ncapabilities = [\"identity-bearing\"]\n",
        "\n  [[type.property]]\n  name = \"name\"\n  kind = \"string\"\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    #[test]
    fn a_corpus_without_a_carried_relationship_refuses_the_dangling_derivation() {
        let corpus = Corpus::holding("link-no-relationship", CONTRACT);
        plant(&corpus, "a.md", "---\ntype: person\nname: A\n---\n# A\n").expect("a note");
        let detail =
            dangling_typed_link(&corpus).expect_err("nothing carries a relationship to repoint");
        assert!(
            detail.contains("no committed note carries a typed relationship"),
            "{detail}"
        );
    }

    #[test]
    fn a_corpus_where_every_name_is_shared_refuses_the_bare_name_cases() {
        let corpus = Corpus::holding("link-all-shared", CONTRACT);
        plant(&corpus, "one/a.md", "---\ntype: person\nname: A\n---\n").expect("a note");
        plant(&corpus, "two/a.md", "---\ntype: person\nname: A\n---\n").expect("its twin");
        let detail = bare_name_resolves(&corpus).expect_err("no unique name exists");
        assert!(
            detail.contains("every committed name is shared"),
            "{detail}"
        );
    }

    /// A corpus whose only unshared name sits at the vault root has no
    /// extensionless path-qualified spelling at all — dropping the extension
    /// there yields the bare name, which the derivation has just made
    /// ambiguous. The case says so rather than reporting the bare-name rule
    /// working correctly as a path-qualification failure.
    #[test]
    fn a_corpus_whose_only_unique_name_is_at_the_root_refuses_the_path_qualified_case() {
        let corpus = Corpus::holding("link-root-only-unique", CONTRACT);
        plant(&corpus, "a.md", "---\ntype: person\nname: A\n---\n").expect("a root-level note");
        plant(&corpus, "one/b.md", "---\ntype: person\nname: B\n---\n").expect("a nested note");
        plant(&corpus, "two/b.md", "---\ntype: person\nname: B\n---\n").expect("its twin");
        let detail = path_qualified_resolves(&corpus)
            .expect_err("no nested note bears an unshared name here");
        assert!(detail.contains("sits below the vault root"), "{detail}");
        // The bare-name case is unaffected: the root-level note is still the
        // one note bearing its name.
        bare_name_resolves(&corpus).expect("a unique bare name still resolves");
    }
}
