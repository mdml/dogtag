//! `unsupported-contract-version-refuses-with-diagnosis`: an out-of-range
//! contract is refused by its version, and diagnosis still runs.

use dogtag::compat::{SUPPORTED_CONTRACT_VERSIONS, VersionClass, classify};
use dogtag::contract::{Contract, ContractLoad};
use dogtag::diagnostic::Severity;
use dogtag::report::{Selection, SelectionRoute, doctor_json, doctor_report, doctor_text};
use dogtag::vault::Opened;

use crate::transform::set_contract_version;

use super::corpus::Corpus;

use super::expect::{
    Checked, NEWER_FORMAT, Subject, did_not_resolve, rendered, require, require_contains,
    require_only,
};

/// The two out-of-range versions and the identifier each must yield.
///
/// Distinct identifiers on purpose: *too new* and *below the floor* call for
/// different actions, and one identifier for both would make the report
/// unactionable exactly when it matters.
///
/// The too-new derivation moved from `2` to `3`, and then to `4`, each time the
/// supported range widened: a version the range now reads is not a version to
/// refuse, and a literal one above the ceiling is what this case is about. The
/// floor does not rise during the beta, so `0` stays where it is.
const OUT_OF_RANGE: &[(u32, &str)] = &[
    (4, "compat.contract-too-new"),
    (0, "compat.contract-below-supported-floor"),
];

/// The three contract-dependent sections, each of which must say it was not
/// evaluated rather than being blank or omitted.
const SECTIONS: &[&str] = &["types", "lifecycle", "dialect"];

/// Textual markers of a construct no version-1 contract may carry.
///
/// The rewrite-to-version-1 derivation is honest only when the contract uses
/// nothing a later version added: where it does, the derived contract must fail
/// loudly rather than load as a pretend-version-1, and this list is how the case
/// knows which of the two it is looking at. It grows with every version — the
/// tag vocabulary and the record kind at 2, the two write seats at 3 — and a
/// version that added a construct without adding its marker here would let a
/// derivation quietly stop testing anything.
const BEYOND_VERSION_ONE: &[&str] = &["[tags]", "\"record\"", "[capture]", "born-flagged"];

/// The supported range as a reader sees it written, from the constant rather
/// than from a literal: a release that widens the range must widen what this
/// case demands of the message in the same edit.
fn supported_range() -> String {
    format!(
        "{}..={}",
        SUPPORTED_CONTRACT_VERSIONS.start(),
        SUPPORTED_CONTRACT_VERSIONS.end()
    )
}

/// `unsupported-contract-version-refuses-with-diagnosis`.
pub fn unsupported_version_refuses(corpus: &Corpus) -> Checked {
    for (version, id) in OUT_OF_RANGE {
        refuses(corpus, *version, id)?;
    }
    Ok(())
}

/// One out-of-range version: exactly one diagnostic, then a report that still
/// diagnoses.
fn refuses(corpus: &Corpus, version: u32, id: &str) -> Checked {
    let label = format!("contract-version-{version}");
    let derived = corpus.derived(&label, |text| set_contract_version(text, version))?;
    let opened = derived.opened_without_a_record()?;
    // Exactly one: the refusal is the version's, not the parser's. An
    // out-of-range contract must not also produce a series of complaints about
    // keys this build does not recognize.
    let declaring = format!("a contract declaring version {version}");
    let subject = Subject::new(&declaring);
    require_only(opened.diagnostics(), id, subject)?;
    message_names_the_version_and_the_range(&rendered(opened.diagnostics()), version, subject)?;
    diagnosis_still_runs(&opened, version)
}

/// The refusal's *message* carries both facts the scenario asks it to name:
/// the version the contract declares, and the range this release reads.
///
/// The identifier says which of the two refusals it is; only the message says
/// what a reader would have to change, and an identifier assertion alone
/// leaves a message reduced to "unsupported contract version" conforming.
///
/// The two numbers are asserted rather than the prose around them, so a
/// reworded message stays conforming and an emptied one does not. Neither
/// needle can be satisfied by accident from the identifier the rendering leads
/// with: no diagnostic identifier in this namespace carries a digit.
fn message_names_the_version_and_the_range(
    reported: &str,
    version: u32,
    of: Subject<'_>,
) -> Checked {
    let subject = format!("the refusal of {of}");
    require_contains(reported, &version.to_string(), &subject)?;
    require_contains(reported, &supported_range(), &subject)
}

/// `doctor` never refuses: the resolved root, the installation state and the
/// version classification are still reported, and every contract-dependent
/// section says it was not evaluated together with the reason.
fn diagnosis_still_runs(opened: &Opened, version: u32) -> Checked {
    let report = doctor_report(opened, Selection::new(SelectionRoute::Discovery, None), &[]);
    let json = doctor_json(&report);
    let diagnosed = format!("the doctor report for a version-{version} contract");
    let subject = Subject::new(&diagnosed);
    // The resolved root, so a reader confronting a broken vault can still see
    // which vault it is.
    require_contains(&json, &opened.root().display(), subject)?;
    // The installation state, which does not depend on the contract at all.
    require_contains(&json, "\"state\": \"absent\"", subject)?;
    version_is_reported(&json, version, subject)?;
    require_contains(&json, "\"state\": \"unresolved\"", subject)?;
    sections_are_not_evaluated(&json, &doctor_text(&report), subject)
}

/// The version found, the range this release reads, and where the one sits
/// relative to the other.
///
/// The range is asserted **here** as well as in the diagnostic's message
/// because the structured report carries it as an object of its own, which is
/// where a consumer diffing two runs reads it. Asserted only of the message,
/// that object could be deleted outright with this suite still green.
fn version_is_reported(json: &str, version: u32, subject: Subject<'_>) -> Checked {
    require_contains(json, &format!("\"found\": {version}"), subject)?;
    // Every run of whitespace folded to one space, so the needle is one
    // legible object rather than one sensitive to how deeply the report
    // happens to indent it.
    let folded = json.split_whitespace().collect::<Vec<&str>>().join(" ");
    require_contains(&folded, &supported_object(), subject)?;
    let class = classify(version, SUPPORTED_CONTRACT_VERSIONS).as_str();
    require_contains(json, &format!("\"classification\": \"{class}\""), subject)
}

/// The supported range as the structured report writes it, whitespace folded.
fn supported_object() -> String {
    format!(
        "\"supported\": {{ \"min\": {}, \"max\": {} }}",
        SUPPORTED_CONTRACT_VERSIONS.start(),
        SUPPORTED_CONTRACT_VERSIONS.end()
    )
}

/// Each section is an object carrying `"evaluated": false` and a non-empty
/// reason — never a `null`, and never an omission, because a consumer cannot
/// tell an omitted section from a section its own reader forgot.
fn sections_are_not_evaluated(json: &str, text: &str, subject: Subject<'_>) -> Checked {
    require(
        json.matches("\"evaluated\": false").count() == SECTIONS.len(),
        || {
            format!(
                "{subject} must mark all {} sections not evaluated:\n{json}",
                SECTIONS.len()
            )
        },
    )?;
    require(!json.contains("\"reason\": \"\""), || {
        format!("{subject} carries an empty reason:\n{json}")
    })?;
    for section in SECTIONS {
        require_contains(text, &format!("{section:<16}not evaluated ("), subject)?;
    }
    Ok(())
}

/// `supported-contract-version-loads-with-info`.
///
/// Two arms, and which one a profile takes is decided by its own committed
/// bytes rather than by naming it.
///
/// **A corpus already below the ceiling is its own evidence.** From M5 the
/// fixture contracts sit on both sides of the current version on purpose, so
/// `dense` and `docs` demonstrate the classification from committed bytes: the
/// load is full — nothing degraded, nothing skipped — and the one diagnostic is
/// the compatibility `info`. That is stronger than a derivation, because the
/// bytes are the ones every other scenario reads.
///
/// **A corpus at the ceiling demonstrates the guard instead.** Rewriting it down
/// to version 1 must fail loudly rather than load as a pretend-version-1: a
/// contract stamped at the current version uses something that version added,
/// and a build that quietly resolved it against version 1's table would be
/// changing a vault's semantics while recording provenance that says it had not.
///
/// The third shape — a contract at the ceiling that uses nothing the ceiling
/// added, rewritten down and still loading — has no witness among the built
/// corpora and is deliberately not asserted. Asserting it would mean authoring a
/// contract to assert it against, which is the derived-not-authored rule's whole
/// subject.
pub fn supported_version_loads_with_info(corpus: &Corpus) -> Checked {
    let load = corpus.load()?;
    let declared = load
        .contract
        .as_ref()
        .map(Contract::contract_version)
        .map_err(|why| did_not_resolve(why, "the committed contract"))?;
    if classify(declared, SUPPORTED_CONTRACT_VERSIONS) == VersionClass::Supported {
        return committed_below_the_ceiling(&load, declared);
    }
    at_the_ceiling(corpus, declared)
}

/// The committed contract is below the ceiling: it loads fully, and says so.
fn committed_below_the_ceiling(load: &ContractLoad, declared: u32) -> Checked {
    let describing = format!("a committed contract declaring the supported version {declared}");
    let subject = Subject::new(&describing);
    require(load.contract.is_ok(), || {
        format!("{subject} must load fully — nothing degraded, nothing skipped")
    })?;
    // Exactly one, and that one: a below-ceiling corpus earns the
    // classification and nothing else, so the allowance every other scenario
    // extends to these two profiles cannot be hiding a second finding.
    require_only(&load.diagnostics, NEWER_FORMAT, subject)?;
    require(load.diagnostics[0].severity == Severity::Info, || {
        "the classification is information, never a failure severity".to_string()
    })?;
    // Naming the newer version is the whole point: a message that only says a
    // newer format exists tells a reader nothing to do about it.
    require_contains(
        &rendered(&load.diagnostics),
        &SUPPORTED_CONTRACT_VERSIONS.end().to_string(),
        subject,
    )
}

/// The committed contract is at the ceiling: rewriting it down fails loudly.
fn at_the_ceiling(corpus: &Corpus, declared: u32) -> Checked {
    let original = corpus.contract_text()?;
    let derived = corpus.derived("supported-v1", |text| set_contract_version(text, 1))?;
    let describing = format!("a contract declaring version {declared} rewritten to version 1");
    let subject = Subject::new(&describing);
    // The guard on the guard: the claim below is only meaningful because the
    // contract uses a construct version 1 does not define, so that is asserted
    // rather than assumed.
    require(
        BEYOND_VERSION_ONE
            .iter()
            .any(|marker| original.contains(marker)),
        || {
            format!(
                "{subject} would load as a pretend-version-1 and this case would test nothing: a \
                 contract at the current version must use something a later version added"
            )
        },
    )?;
    let load = derived.load()?;
    require(load.contract.is_err(), || {
        format!(
            "{subject} must fail loudly, not load; it reported {}",
            rendered(&load.diagnostics)
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use dogtag::contract::parse_contract;
    use dogtag::diagnostic::{Diagnostic, DiagnosticId};

    use super::super::corpus::NO_AXIS;

    /// A report body marking every contract-dependent section not evaluated,
    /// each with a reason.
    const NOT_EVALUATED: &str = concat!(
        "\"types\": { \"evaluated\": false, \"reason\": \"version 3\" },\n",
        "\"lifecycle\": { \"evaluated\": false, \"reason\": \"version 3\" },\n",
        "\"dialect\": { \"evaluated\": false, \"reason\": \"version 3\" }\n",
    );

    /// The text rendering of the same, in the column the reader reads.
    const NOT_EVALUATED_TEXT: &str = concat!(
        "types           not evaluated (version 3)\n",
        "lifecycle       not evaluated (version 3)\n",
        "dialect         not evaluated (version 3)\n",
    );

    /// The subject every assertion below is about.
    const SUBJECT: &str = "the doctor report for a version-4 contract";

    /// The refusal must be the version's, not the parser's — so a contract
    /// reporting something other than the expected identifier fails the case
    /// and names what it reported instead.
    #[test]
    fn a_version_reporting_another_identifier_fails_the_case() {
        let corpus = Corpus::holding("version-unexpected-identifier", NO_AXIS);
        let detail = refuses(&corpus, 4, "compat.contract-below-supported-floor")
            .expect_err("a version-4 contract is too new, not below the floor");
        assert!(
            detail.contains("must report `compat.contract-below-supported-floor`"),
            "the failure names the identifier: {detail}"
        );
        assert!(
            detail.contains("compat.contract-too-new"),
            "the failure says what arrived: {detail}"
        );
    }

    /// A committed contract below the ceiling that reports a second finding is
    /// refused on the count: the allowance the conforming-contract scenario
    /// extends to these two profiles is one diagnostic, and this is where that
    /// is held.
    #[test]
    fn a_below_ceiling_contract_reporting_anything_else_fails_the_case() {
        // A load that resolved and carries the classification, with a second
        // finding pushed onto it. Assembled rather than provoked: a contract
        // fault fatal enough to report twice is fatal enough not to resolve, so
        // the shape this assertion guards against is one only an assembled load
        // can present.
        let mut load =
            parse_contract(&NO_AXIS.replace("contract_version = 3", "contract_version = 2"));
        load.diagnostics.push(Diagnostic::new(
            DiagnosticId::external("ext.harness.second").expect("an `ext.` identifier"),
            Severity::Error,
            "a second finding",
        ));
        let detail = committed_below_the_ceiling(&load, 2)
            .expect_err("a second diagnostic is not the classification alone");
        assert!(
            detail.contains("must report exactly one diagnostic"),
            "the count: {detail}"
        );
        assert!(
            detail.contains("a second finding"),
            "what arrived: {detail}"
        );
    }

    /// ...and one that does not load at all is refused on the load, before any
    /// question about what it reported.
    #[test]
    fn a_below_ceiling_contract_that_does_not_load_fails_the_case() {
        let load = parse_contract("contract_version = 2\n");
        let detail = committed_below_the_ceiling(&load, 2)
            .expect_err("a contract missing every mandatory table does not load fully");
        assert!(detail.contains("must load fully"), "the demand: {detail}");
    }

    /// A contract at the ceiling that uses nothing a later version added would
    /// load as a pretend-version-1, so the case refuses rather than asserting a
    /// failure that would not happen.
    #[test]
    fn a_ceiling_contract_using_no_later_construct_fails_the_case() {
        let corpus = Corpus::holding("version-ceiling-without-constructs", NO_AXIS);
        let detail = at_the_ceiling(&corpus, 3)
            .expect_err("a contract using nothing version 3 added tests nothing when rewritten");
        assert!(
            detail.contains("would load as a pretend-version-1"),
            "what would happen: {detail}"
        );
        assert!(
            detail.contains("must use something a later version added"),
            "why it matters: {detail}"
        );
    }

    /// A message reduced to the bare fact of the refusal names neither the
    /// version it found nor the range it reads, and the failure says which of
    /// the two facts is missing rather than only that something is.
    #[test]
    fn a_refusal_that_names_neither_the_version_nor_the_range_fails_the_case() {
        let bare = message_names_the_version_and_the_range(
            "compat.contract-too-new: unsupported contract version",
            3,
            Subject::new("a contract declaring version 3"),
        )
        .expect_err("a message naming neither fact names the version least of all");
        assert!(
            bare.contains("the refusal of a contract declaring version 3 must carry `3`"),
            "the failure names the subject and the version: {bare}"
        );
        let half = message_names_the_version_and_the_range(
            "compat.contract-too-new: the contract declares version 3",
            3,
            Subject::new("a contract declaring version 3"),
        )
        .expect_err("a message that stops before the range names only half of it");
        assert!(
            half.contains("must carry `1..=3`"),
            "the failure names the range: {half}"
        );
    }

    /// A structured report that drops the `supported` object still carries the
    /// version and the classification, which is exactly the shape an assertion
    /// on those two alone would accept.
    #[test]
    fn a_report_that_drops_the_supported_range_fails_the_case() {
        let json = "\"found\": 4,\n\"classification\": \"too-new\"\n";
        let detail = version_is_reported(json, 4, Subject::new(SUBJECT))
            .expect_err("a report carrying no supported range does not name one");
        assert!(
            detail.contains("must carry `\"supported\": { \"min\": 1, \"max\": 3 }`"),
            "the failure names the object and the range: {detail}"
        );
    }

    /// A section that is simply missing is a section a consumer's own reader
    /// might have dropped, so the count is asserted rather than the presence.
    #[test]
    fn a_report_missing_a_section_is_refused_on_the_count() {
        let json = "\"types\": { \"evaluated\": false, \"reason\": \"version 3\" }\n";
        let detail = sections_are_not_evaluated(json, NOT_EVALUATED_TEXT, Subject::new(SUBJECT))
            .expect_err("one section of three is not all of them");
        assert!(
            detail.contains("must mark all 3 sections not evaluated"),
            "the failure says how many: {detail}"
        );
        assert!(
            detail.contains(json),
            "the failure carries the body: {detail}"
        );
    }

    /// A section marked not evaluated with an empty reason is worse than an
    /// omission: it reads as an answer.
    #[test]
    fn a_section_with_an_empty_reason_is_refused() {
        let json = NOT_EVALUATED.replace("\"version 3\"", "\"\"");
        let detail = sections_are_not_evaluated(&json, NOT_EVALUATED_TEXT, Subject::new(SUBJECT))
            .expect_err("an empty reason is not a reason");
        assert!(
            detail.contains("carries an empty reason"),
            "the failure says what is wrong: {detail}"
        );
    }

    /// The text rendering carries the same three sections, in the column a
    /// reader reads them in.
    #[test]
    fn a_text_rendering_missing_a_section_is_refused() {
        let detail = sections_are_not_evaluated(NOT_EVALUATED, "", Subject::new(SUBJECT))
            .expect_err("a rendering with no sections carries none of them");
        assert!(
            detail.contains("types           not evaluated ("),
            "the failure names the section and its column: {detail}"
        );
    }
}
