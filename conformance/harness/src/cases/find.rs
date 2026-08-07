//! Entity-lookup cases: the two resolution outcomes `find` answers with.

use dogtag::note::{FindResult, find};

use super::corpus::Corpus;
use super::derive::{self, Planting};
use super::expect::{Checked, require, require_clean, require_same_names};
use super::link::first_unique_name;

/// `find-resolves-unambiguous-name`.
pub fn resolves_unambiguous_name(corpus: &Corpus) -> Checked {
    let derived = derive::derived_planting(
        corpus,
        &Planting {
            label: "find-unambiguous",
            notes: &[("derived-find/quixarine-entry.md", "# A planted entry\n")],
        },
    )?;
    for spelling in ["quixarine-entry", "QUIXARINE-ENTRY"] {
        let result = found(&derived, spelling)?;
        require_clean(result.diagnostics(), &format!("finding `{spelling}`"))?;
        let note = result
            .note()
            .ok_or_else(|| format!("`{spelling}` names exactly one note and must resolve"))?;
        let planted = note.path().as_str() == "derived-find/quixarine-entry.md";
        require(planted, || wrong_bearer(spelling, note.path().as_str()))?;
        require(note.type_name().is_some(), || untyped_summary(spelling))?;
    }
    Ok(())
}

/// The details only a misbehaving SDK can produce, built as functions so the
/// text a failing run would print is itself tested — the closures that raise
/// them can never run under a passing suite.
pub(super) fn wrong_bearer(spelling: &str, answered: &str) -> String {
    format!("`{spelling}` resolved to `{answered}` rather than the planted bearer")
}

fn untyped_summary(spelling: &str) -> String {
    format!("the summary must say which type bound `{spelling}`")
}

/// An ambiguous name answered a note instead of refusing.
pub(super) fn resolved_ambiguity(name: &str) -> String {
    format!("the now-ambiguous `{name}` must refuse rather than pick a bearer")
}

/// The lookup raised the wrong number of caller-addressed refusals.
fn wrong_refusals(counted: usize) -> String {
    format!(
        "the lookup must refuse exactly once for the caller's name, but reported {counted} \
         unlocated ambiguity diagnostics"
    )
}

/// `find-ambiguity-lists-candidates`.
pub fn ambiguity_lists_candidates(corpus: &Corpus) -> Checked {
    let (name, path) = first_unique_name(corpus)?;
    let file = path.rsplit('/').next().unwrap_or(&path);
    let second = format!("derived-second/{file}");
    let original = derive::note_text(corpus, &path)?;
    let derived = derive::derived_planting(
        corpus,
        &Planting {
            label: "find-ambiguous",
            notes: &[(&second, &original)],
        },
    )?;
    let result = found(&derived, &name)?;
    require(result.note().is_none(), || resolved_ambiguity(&name))?;
    require_same_names(
        &[path, second],
        &caller_refusal_candidates(&result)?,
        "the refusal's related evidence",
    )
}

/// The bearers the caller's own ambiguity refusal names as related evidence.
///
/// The caller's refusal carries no location — the name came from the caller,
/// not a file — which is what tells it apart from any located ambiguity the
/// derivation may have caused inside the corpus itself. There must be exactly
/// one of them, whatever else the lookup reported.
///
/// `pub(super)` because the recurring-basename case in
/// [`super::docs_native`] reads the same refusal the same way, and two
/// readings of one diagnostic are two chances to read it differently.
pub(super) fn caller_refusal_candidates(result: &FindResult) -> Result<Vec<String>, String> {
    let refusals: Vec<_> = result
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.id.as_str() == "link.ambiguous-reference")
        .filter(|diagnostic| diagnostic.location.is_none())
        .collect();
    require(refusals.len() == 1, || wrong_refusals(refusals.len()))?;
    Ok(refusals[0]
        .related
        .iter()
        .filter_map(|related| related.location.as_ref())
        .map(|location| location.file.display_path().to_owned())
        .collect())
}

/// The corpus asked to find `name`, through the SDK.
pub(super) fn found(corpus: &Corpus, name: &str) -> Result<FindResult, String> {
    let contract = corpus.clean_contract()?;
    Ok(find(&corpus.vault_root()?, &contract, name, None))
}

#[cfg(test)]
mod tests {
    use super::super::derive::plant;
    use super::*;

    const CONTRACT: &str = concat!(
        "contract_version = 3\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    #[test]
    fn a_corpus_where_every_name_is_shared_refuses_the_ambiguity_derivation() {
        let corpus = Corpus::holding("find-all-shared", CONTRACT);
        plant(&corpus, "one/a.md", "# A\n").expect("a note");
        plant(&corpus, "two/a.md", "# Its twin\n").expect("its twin");
        let detail =
            ambiguity_lists_candidates(&corpus).expect_err("no unique name exists to double");
        assert!(
            detail.contains("every committed name is shared"),
            "{detail}"
        );
    }

    #[test]
    fn a_committed_bearer_of_the_planted_name_fails_the_unambiguous_case() {
        let corpus = Corpus::holding("find-taken-name", CONTRACT);
        plant(&corpus, "committed/quixarine-entry.md", "# Already here\n").expect("a note");
        let detail = resolves_unambiguous_name(&corpus)
            .expect_err("the planted name is no longer unambiguous");
        assert!(detail.contains("quixarine-entry"), "{detail}");
    }

    #[test]
    fn the_details_a_misbehaving_sdk_would_earn_say_what_went_wrong() {
        // A passing suite can never run the closures that raise these, so the
        // text itself is held here: each detail names its subject and what
        // actually arrived.
        let bearer = wrong_bearer("entry", "elsewhere.md");
        assert!(bearer.contains("`entry`"), "{bearer}");
        assert!(bearer.contains("`elsewhere.md`"), "{bearer}");
        assert!(untyped_summary("entry").contains("`entry`"));
        assert!(resolved_ambiguity("daily").contains("`daily`"));
        assert!(wrong_refusals(2).contains("reported 2"));
    }

    #[test]
    fn a_corpus_that_is_not_clean_refuses_both_find_derivations() {
        let corpus = Corpus::holding("find-dirty", CONTRACT);
        plant(&corpus, "a.md", "# A\n").expect("a note");
        plant(&corpus, "broken.md", "---\ntype: nothing\n---\n").expect("a broken note");
        let detail = resolves_unambiguous_name(&corpus)
            .expect_err("a corpus that is not clean derives nothing");
        assert!(detail.contains("the corpus before"), "{detail}");
        let detail = ambiguity_lists_candidates(&corpus)
            .expect_err("the ambiguity derivation owes the same cleanliness");
        assert!(detail.contains("the corpus before"), "{detail}");
    }
}
