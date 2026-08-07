//! The assertions a case makes, phrased as `Result` so a failing pair reports
//! a `FAIL` cell with a detail rather than unwinding the whole run.
//!
//! A panicking case would abort the test binary mid-matrix and take every
//! other pair's result with it, which is the opposite of what a cross-product
//! report is for.

use core::fmt;

use dogtag::compat::{SUPPORTED_CONTRACT_VERSIONS, VersionClass, classify};
use dogtag::contract::{ContractLoad, ContractUnresolved};
use dogtag::diagnostic::{Diagnostic, Severity};

/// What every case answers.
pub type Checked = Result<(), String>;

/// What an assertion is about, and what its failure detail leads with.
///
/// A named type rather than a bare string because every assertion below takes
/// one, and a bare `&str` beside another `&str` is exactly how a diagnostic
/// identifier ends up in the subject position and a subject in the identifier
/// position — a mix-up the compiler would have no way to see.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Subject<'a>(&'a str);

impl<'a> Subject<'a> {
    /// An assertion about `what`.
    pub fn new(what: &'a str) -> Self {
        Subject(what)
    }
}

impl<'a> From<&'a str> for Subject<'a> {
    fn from(what: &'a str) -> Self {
        Subject::new(what)
    }
}

impl<'a> From<&'a String> for Subject<'a> {
    fn from(what: &'a String) -> Self {
        Subject::new(what)
    }
}

impl fmt::Display for Subject<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// Fails with `detail` unless `holds`.
pub fn require(holds: bool, detail: impl FnOnce() -> String) -> Checked {
    if holds { Ok(()) } else { Err(detail()) }
}

/// Every diagnostic's identifier, in the order they were reported.
pub fn ids(diagnostics: &[Diagnostic]) -> Vec<&str> {
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic.id.as_str())
        .collect()
}

/// Every diagnostic rendered as `<id>: <message>`, for a failure detail that
/// says what actually happened.
pub fn rendered(diagnostics: &[Diagnostic]) -> String {
    if diagnostics.is_empty() {
        return "nothing".to_owned();
    }
    diagnostics
        .iter()
        .map(|diagnostic| format!("{}: {}", diagnostic.id.as_str(), diagnostic.message))
        .collect::<Vec<String>>()
        .join("; ")
}

/// The three assertions about a diagnostic list differ only in what they
/// demand; the half they share is *otherwise, say what was actually reported*,
/// which is the half a reader of a failing run needs.
fn require_reported(
    diagnostics: &[Diagnostic],
    holds: bool,
    demand: &str,
    subject: Subject<'_>,
) -> Checked {
    require(holds, || {
        format!(
            "{subject} must {demand}, but reported {}",
            rendered(diagnostics)
        )
    })
}

/// The subject reported nothing, at any severity.
pub fn require_clean<'a>(diagnostics: &[Diagnostic], subject: impl Into<Subject<'a>>) -> Checked {
    require_reported(
        diagnostics,
        diagnostics.is_empty(),
        "report zero diagnostics at any severity",
        subject.into(),
    )
}

/// The subject reported `id`, among whatever else it reported.
pub fn require_id<'a>(
    diagnostics: &[Diagnostic],
    id: &str,
    subject: impl Into<Subject<'a>>,
) -> Checked {
    require_reported(
        diagnostics,
        ids(diagnostics).contains(&id),
        &format!("report `{id}`"),
        subject.into(),
    )
}

/// The subject reported nothing about the corpus — and reported the one thing a
/// corpus below the current format version is entitled to carry, exactly when
/// its declared version is below it.
///
/// This is [`require_clean`] with the single exception the M5 fixtures record
/// creates by holding `dense` and `docs` at contract version 2 while the
/// supported range reaches 3: those corpora earn
/// `compat.newer-format-available`, which is a fact about the release reading
/// them rather than a finding about them. Before that split, no committed
/// fixture could demonstrate the `supported`-but-not-current classification and
/// stay clean, and the M4 fixtures record made it derived evidence for exactly
/// that reason; two committed corpora now demonstrate it.
///
/// The exception is deliberately narrower than a tolerance. Where the version is
/// below the ceiling this demands *exactly one* diagnostic, under that
/// identifier, at `info` — so the allowance cannot absorb a second finding, a
/// different identifier, or a promoted severity — and where it is not, this is
/// [`require_clean`] unchanged. `declared` is `None` for a contract that did not
/// resolve, which has no version to be entitled to anything by.
pub fn require_version_only<'a>(
    diagnostics: &[Diagnostic],
    declared: Option<u32>,
    subject: impl Into<Subject<'a>>,
) -> Checked {
    let subject = subject.into();
    let below = declared.is_some_and(|version| {
        classify(version, SUPPORTED_CONTRACT_VERSIONS) == VersionClass::Supported
    });
    if !below {
        return require_clean(diagnostics, subject);
    }
    require_only(diagnostics, NEWER_FORMAT, subject)?;
    require_reported(
        diagnostics,
        diagnostics[0].severity == Severity::Info,
        "classify a below-current version as information",
        subject,
    )
}

/// The classification a corpus below the current format version earns.
pub const NEWER_FORMAT: &str = "compat.newer-format-available";

/// The version a load settled on, when it settled on one.
///
/// A contract that did not resolve declares no version this assertion may reason
/// from: whatever it says about itself, it is not a corpus at that version.
pub fn declared_version(load: &ContractLoad) -> Option<u32> {
    load.contract
        .as_ref()
        .ok()
        .map(dogtag::contract::Contract::contract_version)
}

/// The subject reported `id` and nothing else — the refusal is the version's,
/// not the parser's.
pub fn require_only<'a>(
    diagnostics: &[Diagnostic],
    id: &str,
    subject: impl Into<Subject<'a>>,
) -> Checked {
    let subject = subject.into();
    require_reported(
        diagnostics,
        diagnostics.len() == 1,
        "report exactly one diagnostic",
        subject,
    )?;
    require_id(diagnostics, id, subject)
}

/// The subject's text carries `needle`.
pub fn require_contains<'a>(text: &str, needle: &str, subject: impl Into<Subject<'a>>) -> Checked {
    let subject = subject.into();
    require(text.contains(needle), || {
        format!("{subject} must carry `{needle}`, but does not:\n{text}")
    })
}

/// Every element of `expected` appears in `found`, and nothing else does.
///
/// Both directions in one assertion: a rendering that drops a declaration and
/// one that invents a declaration the contract never made are the same kind of
/// fault, and a scenario that asserts only one of them is half a scenario.
pub fn require_same_names<'a>(
    expected: &[String],
    found: &[String],
    subject: impl Into<Subject<'a>>,
) -> Checked {
    let subject = subject.into();
    for name in expected {
        require(found.contains(name), || {
            format!("{subject} omits the declared `{name}`; it carries {found:?}")
        })?;
    }
    for name in found {
        require(expected.contains(name), || {
            format!("{subject} carries `{name}`, which the contract does not declare")
        })?;
    }
    Ok(())
}

/// Why a contract did not resolve, as a failure detail.
///
/// A bare *the contract did not resolve* sends a reader back to the vault to
/// find out what stopped it. The reason is the content of the failure, so it
/// travels with it — and phrasing it once here is what keeps every case that
/// reports an unresolved contract saying the same thing.
pub fn did_not_resolve<'a>(
    unresolved: &ContractUnresolved,
    subject: impl Into<Subject<'a>>,
) -> String {
    format!(
        "{} did not resolve: {}",
        subject.into(),
        unresolved.reason.describe()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use dogtag::contract::UnresolvedReason;
    use dogtag::diagnostic::{DiagnosticId, Severity};

    /// A diagnostic under a consumer identifier. These tests are about what an
    /// assertion *says*, not about any kernel fault, and `ext.` is the
    /// namespace a consumer mints identifiers in.
    fn reported(id: &str, message: &str) -> Diagnostic {
        Diagnostic::new(
            DiagnosticId::external(id).expect("an `ext.` identifier"),
            Severity::Error,
            message,
        )
    }

    /// Two diagnostics, for the assertions that are about how many arrived.
    fn two() -> Vec<Diagnostic> {
        vec![
            reported("ext.harness.first", "the first fault"),
            reported("ext.harness.second", "the second fault"),
        ]
    }

    /// The detail a failing assertion answered, or a panic naming what it
    /// accepted instead. An assertion helper that cannot fail is the fault
    /// these tests exist to catch, so every one of them starts here.
    fn refusal(checked: Checked) -> String {
        checked.expect_err("the assertion must fail")
    }

    /// The detail is built only when the assertion fails, and it is answered
    /// verbatim.
    #[test]
    fn require_answers_its_detail_only_when_the_assertion_fails() {
        assert_eq!(
            require(true, || panic!("a satisfied assertion builds no detail")),
            Ok(())
        );
        assert_eq!(
            refusal(require(false, || "the detail".to_owned())),
            "the detail"
        );
    }

    /// Nothing reported renders as `nothing`, so a detail never trails off
    /// into an empty string where the reader expects a fault.
    #[test]
    fn rendered_says_nothing_when_no_diagnostic_was_reported() {
        assert_eq!(rendered(&[]), "nothing");
    }

    /// Each diagnostic renders as `<id>: <message>`: the detail says what was
    /// reported, not merely how much of it there was.
    #[test]
    fn rendered_carries_every_identifier_and_message() {
        assert_eq!(
            rendered(&two()),
            "ext.harness.first: the first fault; ext.harness.second: the second fault"
        );
    }

    /// The clean assertion names the subject, the demand, and what arrived.
    #[test]
    fn require_clean_names_the_subject_and_what_was_reported() {
        let detail = refusal(require_clean(&two(), "opening the vault"));
        assert!(
            detail.contains("opening the vault"),
            "the subject: {detail}"
        );
        assert!(
            detail.contains("report zero diagnostics at any severity"),
            "the demand: {detail}"
        );
        assert!(
            detail.contains("ext.harness.first: the first fault"),
            "what arrived: {detail}"
        );
    }

    /// The identifier assertion names the identifier it wanted and what
    /// arrived instead — including when nothing did.
    #[test]
    fn require_id_names_the_identifier_it_wanted() {
        let wrong = refusal(require_id(
            &two(),
            "ext.harness.third",
            "the derived contract",
        ));
        assert!(
            wrong.contains("must report `ext.harness.third`"),
            "the identifier: {wrong}"
        );
        assert!(
            wrong.contains("ext.harness.second: the second fault"),
            "what arrived: {wrong}"
        );
        let silent = refusal(require_id(&[], "ext.harness.third", "the derived contract"));
        assert!(silent.contains("but reported nothing"), "silence: {silent}");
    }

    /// A second diagnostic is refused on the count, before any identifier is
    /// examined: *the refusal is the version's, not the parser's* is a claim
    /// about how many complaints an out-of-range contract earns.
    #[test]
    fn require_only_refuses_a_second_diagnostic() {
        let detail = refusal(require_only(
            &two(),
            "ext.harness.first",
            "a version-2 contract",
        ));
        assert!(
            detail.contains("must report exactly one diagnostic"),
            "the count: {detail}"
        );
        assert!(
            detail.contains("ext.harness.first: the first fault"),
            "what arrived: {detail}"
        );
    }

    /// One diagnostic under the wrong identifier is refused on the identifier.
    #[test]
    fn require_only_refuses_the_wrong_identifier() {
        let one = [reported("ext.harness.first", "the first fault")];
        let detail = refusal(require_only(
            &one,
            "ext.harness.second",
            "a version-2 contract",
        ));
        assert!(
            detail.contains("must report `ext.harness.second`"),
            "the identifier: {detail}"
        );
    }

    /// The containment assertion prints the text it searched: *this string is
    /// not in there* is unactionable without the there.
    #[test]
    fn require_contains_prints_the_text_it_searched() {
        let detail = refusal(require_contains(
            "a rendering",
            "the needle",
            "the Markdown",
        ));
        assert!(detail.contains("the Markdown"), "the subject: {detail}");
        assert!(detail.contains("`the needle`"), "the needle: {detail}");
        assert!(detail.contains("a rendering"), "the haystack: {detail}");
    }

    /// A rendering that drops a declaration is named as an omission...
    #[test]
    fn require_same_names_reports_an_omission() {
        let expected = vec!["person".to_owned()];
        let found: Vec<String> = Vec::new();
        let detail = refusal(require_same_names(
            &expected,
            &found,
            "the Markdown's headings",
        ));
        assert!(
            detail.contains("omits the declared `person`"),
            "the omission: {detail}"
        );
    }

    /// ...and one that invents a declaration the contract never made is named
    /// as an invention. Both directions, because an assertion that carries
    /// only one of them is half an assertion.
    #[test]
    fn require_same_names_reports_an_invention() {
        let expected: Vec<String> = Vec::new();
        let found = vec!["ghost".to_owned()];
        let detail = refusal(require_same_names(&expected, &found, "the JSON's names"));
        assert!(
            detail.contains("carries `ghost`"),
            "the invention: {detail}"
        );
        assert!(
            detail.contains("which the contract does not declare"),
            "why it is one: {detail}"
        );
    }

    /// An unresolved contract reports the reason it gave, not the bare fact.
    #[test]
    fn did_not_resolve_names_the_reason_the_contract_gave() {
        let unresolved = ContractUnresolved {
            reason: UnresolvedReason::Malformed,
            version: None,
        };
        let detail = did_not_resolve(&unresolved, "the committed contract");
        assert!(
            detail.contains("the committed contract"),
            "the subject: {detail}"
        );
        assert!(
            detail.contains("not well-formed TOML"),
            "the reason: {detail}"
        );
    }
}
