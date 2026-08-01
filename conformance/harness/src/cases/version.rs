//! `unsupported-contract-version-refuses-with-diagnosis`: an out-of-range
//! contract is refused by its version, and diagnosis still runs.

use dogtag::compat::{SUPPORTED_CONTRACT_VERSIONS, classify};
use dogtag::report::{Selection, SelectionRoute, doctor_json, doctor_report, doctor_text};
use dogtag::vault::Opened;

use crate::transform::set_contract_version;

use super::corpus::Corpus;
use super::expect::{Checked, require, require_contains, require_only};

/// The two out-of-range versions and the identifier each must yield.
///
/// Distinct identifiers on purpose: *too new* and *below the floor* call for
/// different actions, and one identifier for both would make the report
/// unactionable exactly when it matters.
const OUT_OF_RANGE: &[(u32, &str)] = &[
    (2, "compat.contract-too-new"),
    (0, "compat.contract-below-supported-floor"),
];

/// The three contract-dependent sections, each of which must say it was not
/// evaluated rather than being blank or omitted.
const SECTIONS: &[&str] = &["types", "lifecycle", "dialect"];

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
    require_only(
        opened.diagnostics(),
        id,
        &format!("a contract declaring version {version}"),
    )?;
    diagnosis_still_runs(&opened, version)
}

/// `doctor` never refuses: the resolved root, the installation state and the
/// version classification are still reported, and every contract-dependent
/// section says it was not evaluated together with the reason.
fn diagnosis_still_runs(opened: &Opened, version: u32) -> Checked {
    let report = doctor_report(opened, Selection::new(SelectionRoute::Discovery, None), &[]);
    let json = doctor_json(&report);
    let subject = format!("the doctor report for a version-{version} contract");
    // The resolved root, so a reader confronting a broken vault can still see
    // which vault it is.
    require_contains(&json, &opened.root().display(), &subject)?;
    // The installation state, which does not depend on the contract at all.
    require_contains(&json, "\"state\": \"absent\"", &subject)?;
    // The version found, and where it sits relative to what this build reads.
    require_contains(&json, &format!("\"found\": {version}"), &subject)?;
    let class = classify(version, SUPPORTED_CONTRACT_VERSIONS).as_str();
    require_contains(&json, &format!("\"classification\": \"{class}\""), &subject)?;
    require_contains(&json, "\"state\": \"unresolved\"", &subject)?;
    sections_are_not_evaluated(&json, &doctor_text(&report), &subject)
}

/// Each section is an object carrying `"evaluated": false` and a non-empty
/// reason — never a `null`, and never an omission, because a consumer cannot
/// tell an omitted section from a section its own reader forgot.
fn sections_are_not_evaluated(json: &str, text: &str, subject: &str) -> Checked {
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

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::corpus::NO_AXIS;

    /// A report body marking every contract-dependent section not evaluated,
    /// each with a reason.
    const NOT_EVALUATED: &str = concat!(
        "\"types\": { \"evaluated\": false, \"reason\": \"version 2\" },\n",
        "\"lifecycle\": { \"evaluated\": false, \"reason\": \"version 2\" },\n",
        "\"dialect\": { \"evaluated\": false, \"reason\": \"version 2\" }\n",
    );

    /// The text rendering of the same, in the column the reader reads.
    const NOT_EVALUATED_TEXT: &str = concat!(
        "types           not evaluated (version 2)\n",
        "lifecycle       not evaluated (version 2)\n",
        "dialect         not evaluated (version 2)\n",
    );

    /// The subject every assertion below is about.
    const SUBJECT: &str = "the doctor report for a version-2 contract";

    /// The refusal must be the version's, not the parser's — so a contract
    /// reporting something other than the expected identifier fails the case
    /// and names what it reported instead.
    #[test]
    fn a_version_reporting_another_identifier_fails_the_case() {
        let corpus = Corpus::holding("version-unexpected-identifier", NO_AXIS);
        let detail = refuses(&corpus, 2, "compat.contract-below-supported-floor")
            .expect_err("a version-2 contract is too new, not below the floor");
        assert!(
            detail.contains("must report `compat.contract-below-supported-floor`"),
            "the failure names the identifier: {detail}"
        );
        assert!(
            detail.contains("compat.contract-too-new"),
            "the failure says what arrived: {detail}"
        );
    }

    /// A section that is simply missing is a section a consumer's own reader
    /// might have dropped, so the count is asserted rather than the presence.
    #[test]
    fn a_report_missing_a_section_is_refused_on_the_count() {
        let json = "\"types\": { \"evaluated\": false, \"reason\": \"version 2\" }\n";
        let detail = sections_are_not_evaluated(json, NOT_EVALUATED_TEXT, SUBJECT)
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
        let json = NOT_EVALUATED.replace("\"version 2\"", "\"\"");
        let detail = sections_are_not_evaluated(&json, NOT_EVALUATED_TEXT, SUBJECT)
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
        let detail = sections_are_not_evaluated(NOT_EVALUATED, "", SUBJECT)
            .expect_err("a rendering with no sections carries none of them");
        assert!(
            detail.contains("types           not evaluated ("),
            "the failure names the section and its column: {detail}"
        );
    }
}
