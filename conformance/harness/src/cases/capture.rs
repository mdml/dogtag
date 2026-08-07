//! The M5 write cases: what `capture` does to a corpus, asserted through the
//! ordinary read surfaces.
//!
//! **The per-pair copy is the whole mechanism.** Every corpus-backed case
//! already runs against a temporary copy created fresh per pair, and a write
//! scenario is simply a scenario whose assertions follow an act: it derives any
//! situation it needs into the copy, mutates the copy, and reads the post-state
//! back through `list`, `show` and the corpus walk — the same doors any
//! consumer would use. No restore machinery, no fixture mutation, no schema
//! field, no third status. The committed corpora stay byte-identical through
//! every run, which the checkout's own cleanliness proves.
//!
//! The copies are not repositories, so every case here exercises **guest mode**
//! by construction; the one case about the commit constructs a repository
//! inside its copy first, which is a derivation like any other.

use dogtag::contract::{Contract, DEFAULT_CAPTURE_DIRECTORY, TypeDecl};
use dogtag::diagnostic::VaultPath;
use dogtag::note::{Note, read_corpus};
use dogtag::write::{
    Actor, CaptureRequest, CapturedAt, Outcome, ProvenanceKind, Recovery, WriteResult, capture,
    plan_capture,
};

use crate::transform::{DERIVED_BIRTH_FLAG, declare_derived_birth_state};

use super::corpus::Corpus;
use super::expect::{Checked, Subject, ids, rendered, require, require_contains, require_id};
use super::repository;

/// The instant every capture in these cases is made at.
///
/// Fixed rather than read from a clock: a name derived from the moment a suite
/// happened to run is a name no assertion can state, and a scenario that could
/// not state the name it expects could not tell a capture that landed where it
/// said from one that landed somewhere else.
const AT: u64 = 1_786_000_000;

/// The thought every capture in these cases carries.
///
/// An invented phrase no corpus holds, so *the note this act created* is
/// assertable against any profile's vocabulary.
const THOUGHT: &str = "Vellichor sweep of the harbour ledgers";

/// A request for `text`, by a named actor acting as an agent.
fn request(text: &str) -> CaptureRequest {
    CaptureRequest::new(
        text,
        CapturedAt::from_unix_seconds(AT),
        Actor::new(Some("A Conformance Run".to_owned()), ProvenanceKind::Agent),
    )
}

/// Captures `text` into `corpus`, against the corpus's own clean contract.
///
/// # Errors
///
/// A contract that does not load clean, or a root the SDK refuses.
fn captured(corpus: &Corpus, text: &str) -> Result<(Contract, WriteResult), String> {
    let contract = corpus.clean_contract()?;
    let result = capture(&corpus.vault_root()?, &contract, &request(text));
    Ok((contract, result))
}

/// The path an act says it created, or the reason there is none.
fn created(result: &WriteResult, of: Subject<'_>) -> Result<VaultPath, String> {
    match result.recovery() {
        Some(Recovery::Revert { path, .. } | Recovery::Delete { path }) => Ok(path),
        None => Err(format!(
            "{of} must name what it created, and named nothing; it reported {}",
            rendered(result.diagnostics())
        )),
    }
}

/// Requires that the act landed, saying what it reported when it did not.
fn require_landed(result: &WriteResult, of: Subject<'_>) -> Checked {
    require(result.landed(), || {
        format!(
            "{of} must land; it reported {}",
            rendered(result.diagnostics())
        )
    })
}

/// Where this corpus's captures land, declared or defaulted.
fn directory(contract: &Contract) -> &str {
    contract
        .capture()
        .map_or(DEFAULT_CAPTURE_DIRECTORY, |capture| capture.directory())
}

/// The note at `path`, read back through the shared read path.
///
/// # Errors
///
/// A corpus that does not hold the path the act named, which would mean the
/// result named a file the act did not create.
fn read_back(corpus: &Corpus, contract: &Contract, path: &VaultPath) -> Result<Note, String> {
    read_corpus(&corpus.vault_root()?, contract)
        .note(path)
        .cloned()
        .ok_or_else(|| format!("the corpus holds no note at `{}`", path.as_str()))
}

/// `capture-lands-unfiled`.
pub fn lands_unfiled(corpus: &Corpus) -> Checked {
    let (contract, result) = captured(corpus, THOUGHT)?;
    let subject = Subject::new("a capture into a clean corpus");
    require_landed(&result, subject)?;
    let path = created(&result, subject)?;
    let landing = format!("{}/", directory(&contract));
    require(path.as_str().starts_with(&landing), || {
        format!(
            "a capture must land in `{landing}`, and landed at `{}`",
            path.as_str()
        )
    })?;
    let note = read_back(corpus, &contract, &path)?;
    // Bound by the absence of a discriminator, which is what makes an
    // unclassified capture a first-class member rather than an error.
    require(note.binding().bound_by() == "catch-all", || {
        format!(
            "a capture binds by catch-all, and `{}` bound by {}",
            path.as_str(),
            note.binding().bound_by()
        )
    })?;
    // Visible to the ordinary listing, which is the door a consumer would find
    // it by.
    let listed = dogtag::note::list(
        &corpus.vault_root()?,
        &contract,
        &dogtag::note::ListFilter::default(),
    );
    require(
        listed.notes().iter().any(|summary| summary.path() == &path),
        || format!("`{}` is not in the corpus listing", path.as_str()),
    )
}

/// `capture-body-is-verbatim`.
pub fn body_is_verbatim(corpus: &Corpus) -> Checked {
    // Two lines and a trailing newline: enough that trimming, re-wrapping or
    // appending a newline would each show.
    let text = format!("{THOUGHT}\nsecond line, kept\n");
    let (contract, result) = captured(corpus, &text)?;
    let subject = Subject::new("a verbatim capture");
    require_landed(&result, subject)?;
    let path = created(&result, subject)?;
    let note = read_back(corpus, &contract, &path)?;
    require(note.body() == text, || {
        format!(
            "the body must be the captured bytes; it is {:?} and the capture was {:?}",
            note.body(),
            text
        )
    })?;
    // Frontmatter carries nothing beyond what the contract says a note of this
    // type is born carrying: no discriminator, and no property nobody declared.
    stamped_exactly(&note, born_flagged(&contract), subject)
}

/// The flags this corpus's catch-all is born carrying.
fn born_flagged(contract: &Contract) -> &[String] {
    contract.catch_all().map_or(&[], TypeDecl::born_flagged)
}

/// `capture-preview-writes-nothing`.
pub fn preview_writes_nothing(corpus: &Corpus) -> Checked {
    let contract = corpus.clean_contract()?;
    let before = corpus.fingerprint()?;
    let result = plan_capture(&corpus.vault_root()?, &contract, &request(THOUGHT));
    let subject = Subject::new("a preview");
    require_landed(&result, subject)?;
    require(result.outcome() == &Outcome::Previewed, || {
        format!(
            "a preview writes nothing, and reported {:?}",
            result.outcome()
        )
    })?;
    // The plan still says what it *would* have created, which is the whole of
    // what a preview is for.
    require(result.plan().scope().len() == 1, || {
        format!(
            "a plan names the one file it intends; it named {:?}",
            result.plan().scope()
        )
    })?;
    let after = corpus.fingerprint()?;
    require(after == before, || {
        "a preview left the copy changed; it must be byte-identical afterward".to_owned()
    })
}

/// `capture-collision-appends-suffix`.
pub fn collision_appends_suffix(corpus: &Corpus) -> Checked {
    let (contract, first) = captured(corpus, THOUGHT)?;
    let subject = Subject::new("the first of two colliding captures");
    require_landed(&first, subject)?;
    let one = created(&first, subject)?;
    // The same text at the same instant, so the derived name is the same name.
    let second = capture(&corpus.vault_root()?, &contract, &request(THOUGHT));
    let subject = Subject::new("the second of two colliding captures");
    require_landed(&second, subject)?;
    let two = created(&second, subject)?;
    require(one != two, || {
        format!(
            "two captures must not share the path `{}`; the second overwrote the first",
            one.as_str()
        )
    })?;
    // Neither is lost: both are notes, and both hold the thought.
    for path in [&one, &two] {
        let note = read_back(corpus, &contract, path)?;
        require_contains(note.body(), THOUGHT, Subject::new(path.as_str()))?;
    }
    Ok(())
}

/// `capture-exit-is-the-transaction-verdict`.
pub fn exit_is_the_transaction_verdict(corpus: &Corpus) -> Checked {
    // A corpus transformed to carry a validation error: a note claiming a type
    // no contract declares, which every profile refuses under one identifier.
    let broken = corpus.derived_broken_note()?;
    let (contract, result) = captured(&broken, THOUGHT)?;
    let subject = Subject::new("a capture into a corpus that already carried an error");
    require_landed(&result, subject)?;
    // The corpus's findings ride the result rather than becoming the verdict.
    require_id(result.diagnostics(), "note.unknown-type", subject)?;
    let path = created(&result, subject)?;
    read_back(&broken, &contract, &path).map(|_| ())
}

/// `capture-without-actor-warns`.
pub fn without_actor_warns(corpus: &Corpus) -> Checked {
    let contract = corpus.clean_contract()?;
    // No installation record is in scope at all: the actor the SDK is handed is
    // the one an unconfigured installation resolves to, which is nobody.
    let anonymous = CaptureRequest::new(
        THOUGHT,
        CapturedAt::from_unix_seconds(AT),
        Actor::new(None, ProvenanceKind::Agent),
    );
    let result = capture(&corpus.vault_root()?, &contract, &anonymous);
    let subject = Subject::new("a capture with no actor to attribute it to");
    require_landed(&result, subject)?;
    require_id(result.diagnostics(), "write.actor-unattributed", subject)?;
    // A warning, never a refusal: the write landed and is readable.
    let path = created(&result, subject)?;
    read_back(corpus, &contract, &path).map(|_| ())
}

/// `capture-result-names-recovery`.
pub fn result_names_recovery(corpus: &Corpus) -> Checked {
    let (contract, result) = captured(corpus, THOUGHT)?;
    let subject = Subject::new("a capture in guest mode");
    require_landed(&result, subject)?;
    // The copies are not repositories, so recovery is the created path and
    // deleting it is what undoes the act.
    let Some(Recovery::Delete { path }) = result.recovery() else {
        return Err(format!(
            "a capture into a copy that is not a repository recovers by deleting the file it \
             created; it answered {:?}",
            result.recovery()
        ));
    };
    read_back(corpus, &contract, &path)?;
    require(result.plan().scope() == [path.clone()], || {
        format!(
            "the plan's intended scope and the created path disagree: {:?} against `{}`",
            result.plan().scope(),
            path.as_str()
        )
    })
}

/// `capture-commits-at-birth`.
pub fn commits_at_birth(corpus: &Corpus) -> Checked {
    let constructed = corpus.copy("capture-repository")?;
    repository::construct(&constructed)?;
    let (contract, result) = captured(&constructed, THOUGHT)?;
    let subject = Subject::new("a capture into a copy constructed as a repository");
    require_landed(&result, subject)?;
    let Outcome::Committed { path, commit } = result.outcome() else {
        return Err(format!(
            "a capture into a repository commits at birth; it answered {:?} and reported {}",
            result.outcome(),
            rendered(result.diagnostics())
        ));
    };
    read_back(&constructed, &contract, path)?;
    let (message, files) = repository::contents(constructed.root(), commit)?;
    // Pathspec-scoped: exactly the created file, and nothing a concurrent
    // writer might have left in the tree.
    require(files == [path.as_str().to_owned()], || {
        format!(
            "the commit must hold exactly `{}`; it holds {files:?}",
            path.as_str()
        )
    })?;
    let carrying = Subject::new("the capture's commit message");
    require_contains(&message, "Dogtag-Actor: A Conformance Run", carrying)?;
    require_contains(&message, "Dogtag-Provenance: agent", carrying)
}

/// `capture-birth-state-stamps-the-flag`.
pub fn birth_state_stamps_the_flag(corpus: &Corpus) -> Checked {
    // What this corpus declares, whatever that is: the flags its catch-all is
    // born carrying are exactly the properties a capture into it comes with.
    let (contract, result) = captured(corpus, THOUGHT)?;
    let subject = Subject::new("a capture into the committed corpus");
    require_landed(&result, subject)?;
    let path = created(&result, subject)?;
    let note = read_back(corpus, &contract, &path)?;
    stamped_exactly(&note, born_flagged(&contract), subject)?;
    // And the other side, derived so that every profile exercises it: a
    // contract declaring a birth state stamps it.
    let declaring = corpus.derived("capture-birth-state", declare_derived_birth_state)?;
    let (derived, result) = captured(&declaring, THOUGHT)?;
    let subject = Subject::new("a capture into a corpus declaring a birth state");
    require_landed(&result, subject)?;
    let path = created(&result, subject)?;
    let note = read_back(&declaring, &derived, &path)?;
    stamped_exactly(&note, born_flagged(&derived), subject)?;
    require(
        born_flagged(&derived)
            .iter()
            .any(|flag| flag == DERIVED_BIRTH_FLAG),
        || "the derivation must declare a birth state, and declared none".to_owned(),
    )
}

/// The note carries exactly the flags its type is born carrying, and nothing
/// else.
///
/// Compared as sets rather than as sequences: the model reports a note's
/// properties in the order the *type* declares them, and the birth state is a
/// roster rather than an ordering, so a corpus whose two declarations disagree
/// about order is not a corpus that stamped the wrong thing.
fn stamped_exactly(note: &Note, born: &[String], of: Subject<'_>) -> Checked {
    let mut stamped: Vec<&str> = note
        .properties()
        .iter()
        .map(dogtag::note::Property::name)
        .collect();
    stamped.sort_unstable();
    let mut expected: Vec<&str> = born.iter().map(String::as_str).collect();
    expected.sort_unstable();
    require(stamped == expected, || {
        format!("{of} must be born carrying {expected:?}; it carries {stamped:?}")
    })
}

/// `capture-repeat-is-deterministic`.
pub fn repeat_is_deterministic(corpus: &Corpus) -> Checked {
    let contract = corpus.clean_contract()?;
    let root = corpus.vault_root()?;
    // The plan for identical input is identical: the actor, the intended scope,
    // and what the corpus already had to say.
    let first = plan_capture(&root, &contract, &request(THOUGHT));
    let second = plan_capture(&root, &contract, &request(THOUGHT));
    require(first.plan() == second.plan(), || {
        format!(
            "two plans for one input must be identical: {:?} against {:?}",
            first.plan(),
            second.plan()
        )
    })?;
    require(
        ids(first.diagnostics()) == ids(second.diagnostics()),
        || "two previews of one input must report the same thing".to_owned(),
    )?;
    // And two acts are two notes: the timestamped identity is what a repeat
    // differs by, and the collision rule is what keeps both.
    let one = capture(&root, &contract, &request(THOUGHT));
    let two = capture(&root, &contract, &request(THOUGHT));
    let subject = Subject::new("a repeated capture");
    require_landed(&one, subject)?;
    require_landed(&two, subject)?;
    require(created(&one, subject)? != created(&two, subject)?, || {
        "two acts must produce two notes".to_owned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A contract whose catch-all no caller may modify, so every act against it
    /// is refused.
    ///
    /// The harness's own tests are **not conformance cases** — they never run
    /// against a profile and never outlive their temporary directory — so a
    /// contract written here is an input to a test *of the assertions
    /// themselves* rather than a checked-in negative fixture.
    const CLOSED: &str = concat!(
        "contract_version = 3\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[[type]]\nname = \"note\"\ncapabilities = [\"catch-all\", \"closed-write\"]\n",
    );

    /// The same contract, open to writers.
    const OPEN: &str = concat!(
        "contract_version = 3\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[[type]]\nname = \"note\"\ncapabilities = [\"catch-all\"]\n",
    );

    /// A corpus every act is refused against.
    fn refusing(label: &str) -> Corpus {
        Corpus::holding(label, CLOSED)
    }

    /// Every case, against a corpus whose catch-all refuses every write.
    ///
    /// The details a misbehaving SDK would earn: each case must say that the
    /// act did not land, and say what it reported instead, rather than
    /// reporting green or panicking. Run over the whole set, so a case added
    /// without a legible refusal fails here.
    #[test]
    fn the_details_a_misbehaving_sdk_would_earn_say_what_went_wrong() {
        let cases: &[(&str, super::super::Case)] = &[
            ("lands-unfiled", lands_unfiled),
            ("body-verbatim", body_is_verbatim),
            ("preview", preview_writes_nothing),
            ("collision", collision_appends_suffix),
            ("verdict", exit_is_the_transaction_verdict),
            ("unattributed", without_actor_warns),
            ("recovery", result_names_recovery),
            ("commits", commits_at_birth),
            ("birth-state", birth_state_stamps_the_flag),
            ("repeat", repeat_is_deterministic),
        ];
        for (label, case) in cases {
            let corpus = refusing(&format!("capture-refused-{label}"));
            let detail = case(&corpus).expect_err("a closed catch-all refuses every write");
            assert!(
                detail.contains("write.closed-write"),
                "`{label}` must say what happened: {detail}"
            );
        }
    }

    /// A result that created nothing names nothing, and the detail says what it
    /// reported instead of what it did not do.
    #[test]
    fn a_result_that_created_nothing_names_what_it_reported() {
        let corpus = refusing("capture-created-nothing");
        let (_, result) = captured(&corpus, THOUGHT).expect("a contract that loads");
        let detail =
            created(&result, Subject::new("a refused act")).expect_err("nothing was created");
        assert!(detail.contains("must name what it created"), "{detail}");
        assert!(detail.contains("write.closed-write"), "{detail}");
    }

    /// An act that did not land is refused with what it reported, so a failing
    /// pair says why rather than only that.
    #[test]
    fn an_act_that_did_not_land_is_refused_with_what_it_reported() {
        let corpus = refusing("capture-did-not-land");
        let (_, result) = captured(&corpus, THOUGHT).expect("a contract that loads");
        let detail = require_landed(&result, Subject::new("a refused act"))
            .expect_err("a closed catch-all refuses");
        assert!(detail.contains("must land"), "{detail}");
        assert!(detail.contains("write.closed-write"), "{detail}");
    }

    /// A path the corpus does not hold is a result naming a file the act did
    /// not create, and the detail names the path.
    #[test]
    fn a_path_the_corpus_does_not_hold_names_itself() {
        let corpus = Corpus::holding("capture-absent-note", OPEN);
        let contract = corpus.clean_contract().expect("a contract that loads");
        let root = corpus.vault_root().expect("a vault root");
        let landed = capture(&root, &contract, &request(THOUGHT));
        let path = created(&landed, Subject::new("the act")).expect("it landed");
        read_back(&corpus, &contract, &path).expect("the note it created");
        std::fs::remove_file(corpus.root().join(path.as_str())).expect("removing the note");
        let detail = read_back(&corpus, &contract, &path).expect_err("the note is gone");
        assert!(detail.contains("holds no note at"), "{detail}");
        assert!(detail.contains(path.as_str()), "{detail}");
    }

    /// The birth-state assertion names both rosters, so a reader sees which
    /// flag was stamped and which was expected.
    #[test]
    fn a_note_stamped_with_the_wrong_flags_names_both_rosters() {
        let corpus = Corpus::holding("capture-wrong-flags", OPEN);
        let contract = corpus.clean_contract().expect("a contract that loads");
        let root = corpus.vault_root().expect("a vault root");
        let landed = capture(&root, &contract, &request(THOUGHT));
        let path = created(&landed, Subject::new("the act")).expect("it landed");
        let note = read_back(&corpus, &contract, &path).expect("the note");
        let expected = ["needs_triage".to_owned()];
        let detail = stamped_exactly(&note, &expected, Subject::new("the capture"))
            .expect_err("this corpus declares no birth state");
        assert!(detail.contains("must be born carrying"), "{detail}");
        assert!(detail.contains("needs_triage"), "{detail}");
    }

    /// Where a corpus declares no capture directory, the default is what a
    /// capture lands in — and the case reads it from the same place the SDK
    /// does rather than restating it.
    #[test]
    fn the_landing_directory_is_the_declaration_or_the_default() {
        let corpus = Corpus::holding("capture-directory", OPEN);
        let contract = corpus.clean_contract().expect("a contract that loads");
        assert_eq!(directory(&contract), DEFAULT_CAPTURE_DIRECTORY);
        assert!(born_flagged(&contract).is_empty());
    }
}
