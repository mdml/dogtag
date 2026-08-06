//! Reports, and the renderings the SDK owns.
//!
//! This module carries the `doctor` report model, the structured-output schema
//! version, the JSON serialization of both reports, and the generated agent
//! contract in Markdown. Those land with the M2 report surface. Rendering
//! lives here rather than in a consumer so that an agent receives the same
//! vault contract whichever door it enters by.
//!
//! # Why rendering is kernel work
//!
//! A vault's agent contract is **generated rather than written**, because
//! hand-maintained agent instructions always eventually lie and the cost of
//! that lie compounds as agents do more of the writing. If a consumer owned the
//! Markdown, the CLI, the MCP server and the TypeScript binding would each grow
//! their own, and an agent would receive a different vault contract depending on
//! which door it entered — exactly the drift generation exists to prevent. So
//! the SDK owns the Markdown, both JSON serializations, and the plain-text
//! diagnostic rendering; a consumer owns argument parsing, environment
//! resolution, colour, stream routing, and the mapping from severity to an exit
//! code.
//!
//! # What is fixed here
//!
//! - **[`doctor_report`] never refuses.** With an unusable contract it still
//!   reports the resolved root, the installation state and the version
//!   classification, and marks every contract-dependent section *not evaluated*
//!   with the reason. Stopping at the version check would hide whether the right
//!   vault was even found, exactly when the reader is most confused.
//! - **Only the resolved vault's registry entry is reported**, never the whole
//!   registry. A registry enumerates every vault its owner has registered, by
//!   chosen name and absolute path; this SDK is agent-facing, so a full
//!   inventory would travel into an agent's context and its provider's logs in
//!   answer to a question about one vault. There is deliberately no flag for the
//!   inventory.
//! - **The installation record renders unexpanded**, as
//!   [`FileRef::INSTALLATION_RECORD_PATH`], so no output emits an account name.
//! - **The two contract renderings are semantically equal.** Every declaration
//!   in the Markdown appears in the JSON and the reverse, and neither carries a
//!   declaration the contract does not make.
//! - **Determinism.** Types, properties and relationships render in
//!   *declaration* order and never sorted; provenance renders in key order; JSON
//!   object keys are emitted in a fixed order. Identical input produces
//!   byte-identical output on any machine: no map iteration reaches the output,
//!   no absolute path but the vault root does, and there is no timestamp and no
//!   locale anywhere in it.
//!
//! [`FileRef::INSTALLATION_RECORD_PATH`]: crate::diagnostic::FileRef::INSTALLATION_RECORD_PATH

mod check;
mod doctor;
mod find;
mod json;
mod list;
mod markdown;
mod search;
mod show;

#[cfg(test)]
mod equivalence;
#[cfg(test)]
mod fixture;

use core::ops::RangeInclusive;

use crate::compat::{
    self, SUPPORTED_CONTRACT_VERSIONS, SUPPORTED_INSTALLATION_VERSIONS, VersionClass,
};
use crate::contract::{
    Capability, Contract, ContractUnresolved, LifecycleDecl, LinkDialect, UnresolvedReason,
};
use crate::diagnostic::{Diagnostic, DiagnosticList, SeverityCounts};
use crate::installation::{Installation, InstallationRecord, VaultEntry};
use crate::vault::{Opened, VaultRoot};

pub use check::{CheckReport, check_report, check_text};
pub use doctor::doctor_text;
pub use find::find_text;
pub use json::{
    check_json, contract_json, doctor_json, find_json, list_json, search_json, show_json,
};
pub use list::list_text;
pub use markdown::contract_markdown;
pub use search::search_text;
pub use show::{ShowReport, show_report, show_text};

/// The version of the structured output this module emits.
///
/// A **third clock**, deliberately separate from `contract_version` and from the
/// crate's own version. Three clocks is one more than anyone wants; the
/// alternative was coupling output stability to format stability, which would
/// make every format bump a breaking change for every consumer parsing the JSON
/// and every schema fix a format bump.
///
/// It ticks when a field name or a field's type changes, which is the line the
/// diagnostics record's 2026-08-01 amendment drew from the other side when it
/// declined to tick for a change that moved no field — and it ticks **once per
/// milestone**, per the M3 surfaces record's 2026-08-04 amendment, so a
/// consumer pinning the version sees one bump per milestone however many
/// shapes the milestone adds. Version 2 is the tag vocabulary: `contract.tags`
/// and a `tag_namespaces` collection on every type. Version 3 is M4's
/// retrieval surface, taken when its first report shape — the `search`
/// document — landed; the milestone's later shapes ride the same version.
pub const SCHEMA_VERSION: u32 = 3;

/// How a caller arrived at the vault it is reporting on.
///
/// Selection is a consumer's decision — argv, the environment, the current
/// directory — and this records *which* decision was taken so a report can say
/// so. A reader confronting the wrong vault needs to know which route chose it
/// before anything else in the report helps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionRoute {
    /// Upward discovery from a starting directory.
    Discovery,
    /// An explicit path, given as an argument.
    FlagPath,
    /// A registry name, given as an argument.
    FlagName,
    /// An explicit path, taken from the environment.
    EnvironmentPath,
    /// A registry name, taken from the environment.
    EnvironmentName,
}

/// One route's spellings: the wire word, and the phrase a reader sees.
///
/// A table rather than a `match` in each of two places, so a route's spellings
/// are declared together and cannot drift apart.
struct RouteSpelling {
    route: SelectionRoute,
    wire: &'static str,
    selector: &'static str,
    placeholder: &'static str,
    registered: &'static str,
}

/// Every route, once, with everything that is said about it.
const ROUTES: &[RouteSpelling] = &[
    RouteSpelling {
        route: SelectionRoute::Discovery,
        wire: "discovery",
        selector: "",
        placeholder: "upward discovery from the current directory",
        registered: "",
    },
    RouteSpelling {
        route: SelectionRoute::FlagPath,
        wire: "flag-path",
        selector: "--vault ",
        placeholder: "<path>",
        registered: "",
    },
    RouteSpelling {
        route: SelectionRoute::FlagName,
        wire: "flag-name",
        selector: "--vault ",
        placeholder: "<name>",
        registered: " (registered)",
    },
    RouteSpelling {
        route: SelectionRoute::EnvironmentPath,
        wire: "environment-path",
        selector: "DOGTAG_VAULT ",
        placeholder: "<path>",
        registered: "",
    },
    RouteSpelling {
        route: SelectionRoute::EnvironmentName,
        wire: "environment-name",
        selector: "DOGTAG_VAULT ",
        placeholder: "<name>",
        registered: " (registered)",
    },
];

impl SelectionRoute {
    /// Every route, in the order this module declares them.
    pub const ALL: &'static [SelectionRoute] = &[
        Self::Discovery,
        Self::FlagPath,
        Self::FlagName,
        Self::EnvironmentPath,
        Self::EnvironmentName,
    ];

    /// The lowercase wire spelling, used by every structured format.
    pub fn as_str(self) -> &'static str {
        self.spelling().wire
    }

    /// This route's row of the table.
    ///
    /// The lookup cannot miss: [`ROUTES`] carries every variant, which a test
    /// proves with a `match` the compiler checks for exhaustiveness.
    fn spelling(self) -> &'static RouteSpelling {
        ROUTES
            .iter()
            .find(|spelling| spelling.route == self)
            .expect("every selection route is spelled in the table")
    }
}

/// Which vault a caller asked for, and how.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    how: SelectionRoute,
    requested: Option<String>,
}

impl Selection {
    /// A selection by `how`, carrying the argument that drove it.
    ///
    /// [`SelectionRoute::Discovery`] carries nothing, because nothing was
    /// requested: a starting directory chose the vault. Every other route
    /// carries the argument **exactly as it was given**, so a reader sees the
    /// text that resolved to the root beside the root itself.
    pub fn new(how: SelectionRoute, requested: Option<String>) -> Self {
        Self { how, requested }
    }

    /// The route this selection took.
    pub fn how(&self) -> SelectionRoute {
        self.how
    }

    /// The argument that drove it, when there was one.
    pub fn requested(&self) -> Option<&str> {
        self.requested.as_deref()
    }

    /// The registry name this selection resolved through, if it resolved
    /// through one at all.
    ///
    /// A name route's argument *is* the registered name, so the entry that
    /// answered it can be looked up by the name rather than re-derived by
    /// comparing paths. That matters because the two are not comparable: a
    /// registered path is stored literally, and the root it resolved to is
    /// canonical, so any symlinked component made a lexical comparison fail
    /// and the report claimed no entry for a vault it had reached *through*
    /// one.
    fn registry_name(&self) -> Option<&str> {
        match self.how {
            SelectionRoute::FlagName | SelectionRoute::EnvironmentName => self.requested(),
            SelectionRoute::Discovery
            | SelectionRoute::FlagPath
            | SelectionRoute::EnvironmentPath => None,
        }
    }

    /// The route as a reader sees it, with the argument filled in.
    ///
    /// A route whose argument was not supplied renders its placeholder rather
    /// than a blank, so the line always names a route.
    fn describe(&self) -> String {
        let spelling = self.how.spelling();
        let subject = self.requested.as_deref().unwrap_or(spelling.placeholder);
        format!("{}{subject}{}", spelling.selector, spelling.registered)
    }
}

/// A boolean as a reader reads it.
///
/// Shared by the two renderings so that *present* and *required* answer with the
/// same word, and so that one of them cannot quietly start saying `true`.
fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

/// Where a declared version sits, and what range it was judged against.
///
/// The range travels with the finding because the classification is meaningless
/// without it: *too new* is a statement about this build, not about the asset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VersionFacts {
    found: Option<u32>,
    min: u32,
    max: u32,
    classification: Option<VersionClass>,
}

impl VersionFacts {
    /// The classification of `found` against `supported`, or the range alone
    /// when the asset never got far enough to declare a version.
    fn new(found: Option<u32>, supported: &RangeInclusive<u32>) -> Self {
        Self {
            found,
            min: *supported.start(),
            max: *supported.end(),
            classification: found.map(|version| compat::classify(version, supported.clone())),
        }
    }

    /// The classification of a version no `u32` holds, which is the one case
    /// where the class is known and the number cannot be reported.
    fn unrepresentable(class: VersionClass, supported: &RangeInclusive<u32>) -> Self {
        Self {
            found: None,
            min: *supported.start(),
            max: *supported.end(),
            classification: Some(class),
        }
    }
}

/// What reading the contract established, whether or not it resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ContractFacts {
    present: bool,
    state: &'static str,
    reason: Option<String>,
    version: VersionFacts,
}

impl ContractFacts {
    /// The facts a resolved contract carries.
    fn resolved(contract: &Contract) -> Self {
        Self {
            present: true,
            state: "loaded",
            reason: None,
            version: VersionFacts::new(
                Some(contract.contract_version()),
                &SUPPORTED_CONTRACT_VERSIONS,
            ),
        }
    }

    /// The facts an unresolved contract carries.
    ///
    /// A file that is not there is the one refusal where *present* is false;
    /// every other one read bytes.
    /// The facts a contract carries when no vault resolved to hold one.
    ///
    /// Distinct from `unresolved`: that one read a vault and failed to read
    /// its contract, so it knows whether a file was there. Here there was no
    /// vault to look in, and claiming a version or a presence would be an
    /// answer nothing established.
    fn not_evaluated(reason: &str) -> Self {
        Self {
            present: false,
            state: "not evaluated",
            reason: Some(reason.to_owned()),
            version: VersionFacts::new(None, &SUPPORTED_CONTRACT_VERSIONS),
        }
    }

    fn unresolved(unresolved: &ContractUnresolved) -> Self {
        Self {
            present: unresolved.reason != UnresolvedReason::Missing,
            state: "unresolved",
            reason: Some(unresolved.reason.describe()),
            version: declared_contract_version(unresolved),
        }
    }
}

/// What the contract declared where a version belongs.
///
/// A version outside the supported range that no `u32` holds is classified
/// without being representable, so the classification is kept and the number
/// is reported as undeclared rather than as a number the file does not carry.
fn declared_contract_version(unresolved: &ContractUnresolved) -> VersionFacts {
    match (unresolved.version, unresolved.reason) {
        (None, UnresolvedReason::VersionUnusable(class)) => {
            VersionFacts::unrepresentable(class, &SUPPORTED_CONTRACT_VERSIONS)
        }
        (found, _) => VersionFacts::new(found, &SUPPORTED_CONTRACT_VERSIONS),
    }
}

/// One registered vault, as a report names it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct RegistryEntry {
    name: String,
    path: String,
}

/// What reading the installation record established.
#[derive(Clone, Debug, PartialEq, Eq)]
struct InstallationFacts {
    state: &'static str,
    version: Option<VersionFacts>,
    actor: Option<String>,
    entry: Option<RegistryEntry>,
}

impl InstallationFacts {
    /// The record's facts, holding only the entry for the vault being reported.
    fn new(installation: &Installation, root: &VaultRoot, selection: &Selection) -> Self {
        let record = installation.record();
        Self {
            state: installation.state().as_str(),
            version: record.map(InstallationFacts::declared_version),
            actor: record
                .and_then(InstallationRecord::actor)
                .map(|actor| actor.name().to_owned()),
            entry: record.and_then(|record| entry_for(record, root, selection)),
        }
    }

    /// The record's facts when no vault resolved, so no entry can be selected.
    ///
    /// The registry is not listed here for the same reason it is not listed
    /// anywhere: a report names the entry for the vault being reported, and
    /// there is no vault being reported.
    fn without_a_vault(installation: &Installation) -> Self {
        let record = installation.record();
        Self {
            state: installation.state().as_str(),
            version: record.map(InstallationFacts::declared_version),
            actor: record
                .and_then(InstallationRecord::actor)
                .map(|actor| actor.name().to_owned()),
            entry: None,
        }
    }

    /// The record's own version, classified against the range this build reads.
    fn declared_version(record: &InstallationRecord) -> VersionFacts {
        VersionFacts::new(
            Some(record.installation_version()),
            &SUPPORTED_INSTALLATION_VERSIONS,
        )
    }
}

/// The entry registering `root`, if the record registers it at all.
///
/// The comparison is **lexical**, against the registered path exactly as it is
/// written: a registered path is never expanded and never re-resolved, so
/// answering this cannot become a filesystem operation. A `doctor` run opens
/// exactly two files, and resolving registry paths would reach a third thing —
/// a directory outside the vault, that the reader did not ask about.
fn entry_for(
    record: &InstallationRecord,
    root: &VaultRoot,
    selection: &Selection,
) -> Option<RegistryEntry> {
    match selection.registry_name() {
        Some(name) => record.entry(name).map(named_entry),
        None => registered_as(record, root),
    }
}

/// One registry entry as the report names it.
fn named_entry(entry: &VaultEntry) -> RegistryEntry {
    RegistryEntry {
        name: entry.name().to_owned(),
        path: entry.path().to_string_lossy().into_owned(),
    }
}

fn registered_as(record: &InstallationRecord, root: &VaultRoot) -> Option<RegistryEntry> {
    record
        .vaults()
        .iter()
        .find(|entry| entry.path() == root.path())
        .map(named_entry)
}

/// The answers that exist only because a contract resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Evaluated {
    types_declared: usize,
    identity_bearing: Vec<String>,
    catch_all: Option<String>,
    closed_write: Vec<String>,
    lifecycle: LifecycleDecl,
    links: LinkDialect,
}

impl Evaluated {
    fn new(contract: &Contract) -> Self {
        Self {
            types_declared: contract.types().len(),
            identity_bearing: declaring(contract, Capability::IdentityBearing),
            catch_all: contract.catch_all().map(|kind| kind.name().to_owned()),
            closed_write: declaring(contract, Capability::ClosedWrite),
            lifecycle: contract.lifecycle().clone(),
            links: contract.dialect().links(),
        }
    }
}

/// The names of every type declaring `capability`, in declaration order.
fn declaring(contract: &Contract, capability: Capability) -> Vec<String> {
    contract
        .types_with(capability)
        .map(|declared| declared.name().to_owned())
        .collect()
}

/// The contract-dependent part of the report: either the answers, or the reason
/// there are none.
///
/// One state for all three sections, because all three depend on the same
/// contract and fail together. Each still renders its own *not evaluated* entry:
/// an omission is indistinguishable from a bug, and a reader diffing two reports
/// needs the section to be there either way.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Sections {
    Evaluated(Evaluated),
    NotEvaluated(String),
}

impl Sections {
    fn new(contract: Result<&Contract, &ContractUnresolved>) -> Self {
        match contract {
            Ok(contract) => Self::Evaluated(Evaluated::new(contract)),
            Err(unresolved) => Self::NotEvaluated(unresolved.reason.describe()),
        }
    }
}

/// A vault's configuration health, as `doctor` reports it.
///
/// Built once by [`doctor_report`] and rendered by [`doctor_text`] or
/// [`doctor_json`], so the two renderings cannot answer differently.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoctorReport {
    root: Option<String>,
    selection: Selection,
    contract: ContractFacts,
    installation: InstallationFacts,
    sections: Sections,
    counts: SeverityCounts,
    diagnostics: Vec<Diagnostic>,
}

impl DoctorReport {
    /// Everything the run had to say, in the deterministic total order.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// How many diagnostics of each severity the report carries.
    ///
    /// This is what a consumer maps to an exit code: severity alone decides it.
    pub fn counts(&self) -> SeverityCounts {
        self.counts
    }
}

/// Builds the `doctor` report for an opened vault.
///
/// `extra` is how diagnostics a consumer raised outside [`crate::vault::open`] —
/// root trust, discovery — reach the report. They are merged with the vault's
/// own and the whole list is put into the deterministic total order, because a
/// surface reporting on a vault reports on the vault rather than on the several
/// operations that inspected it.
///
/// This never refuses. A contract that did not resolve costs the three
/// contract-dependent sections and nothing else: the root, how it was selected,
/// the installation state and the version classification are all still reported,
/// and each section says *not evaluated* with the reason.
pub fn doctor_report(opened: &Opened, selection: Selection, extra: &[Diagnostic]) -> DoctorReport {
    let mut collected = DiagnosticList::new();
    collected.extend(opened.diagnostics().iter().cloned());
    collected.extend(extra.iter().cloned());
    let counts = collected.counts();
    let installation = InstallationFacts::new(opened.installation(), opened.root(), &selection);
    DoctorReport {
        root: Some(opened.root().display().into_owned()),
        selection,
        contract: opened
            .contract()
            .map_or_else(ContractFacts::unresolved, ContractFacts::resolved),
        installation,
        sections: Sections::new(opened.contract()),
        counts,
        diagnostics: collected.sorted(),
    }
}

/// Builds the `doctor` report for a run whose vault never resolved.
///
/// `doctor` never refuses. The compatibility record says so about a contract
/// outside the supported range, and the reasoning is the same one step earlier:
/// a selection that named nothing is exactly when a reader most needs the facts
/// that *are* known — whether an installation record exists, what it declares,
/// and what was looked for. Refusing with a bare exit code hands a `--format
/// json` consumer an empty stream during the very run it was added to triage.
///
/// Every vault-dependent section says *not evaluated* with the reason, which is
/// the shape [`doctor_report`] already uses for a contract that did not resolve.
pub fn doctor_unresolved(
    installation: &Installation,
    selection: Selection,
    diagnostics: &[Diagnostic],
) -> DoctorReport {
    const REASON: &str = "the vault did not resolve";
    let mut collected = DiagnosticList::new();
    collected.extend(diagnostics.iter().cloned());
    let counts = collected.counts();
    DoctorReport {
        root: None,
        selection,
        contract: ContractFacts::not_evaluated(REASON),
        installation: InstallationFacts::without_a_vault(installation),
        sections: Sections::NotEvaluated(REASON.to_owned()),
        counts,
        diagnostics: collected.sorted(),
    }
}

#[cfg(test)]
mod tests {
    use super::fixture::{Body, CLEAN, RECORD, Tree, no_record, opened, registering};
    use super::*;
    use crate::diagnostic::{KernelDiagnostic, Severity};
    use crate::installation::parse_installation;
    use crate::vault::open;
    use std::fs;

    fn requested(how: SelectionRoute, given: &str) -> Selection {
        Selection::new(how, Some(given.to_owned()))
    }

    fn discovery() -> Selection {
        Selection::new(SelectionRoute::Discovery, None)
    }

    /// A report's contract-dependent answers, when it has any.
    fn evaluated(report: &DoctorReport) -> Option<&Evaluated> {
        match &report.sections {
            Sections::Evaluated(evaluated) => Some(evaluated),
            Sections::NotEvaluated(_) => None,
        }
    }

    fn report_of(tree: &Tree, body: Body<'_>) -> DoctorReport {
        doctor_report(&opened(tree, body, RECORD), discovery(), &[])
    }

    #[test]
    fn every_route_is_spelled_exactly_once_in_the_table() {
        // The `match` is the proof: the compiler rejects this test if a variant
        // gains no spelling, and the table lookup cannot then miss.
        for route in SelectionRoute::ALL {
            let expected = match route {
                SelectionRoute::Discovery => "discovery",
                SelectionRoute::FlagPath => "flag-path",
                SelectionRoute::FlagName => "flag-name",
                SelectionRoute::EnvironmentPath => "environment-path",
                SelectionRoute::EnvironmentName => "environment-name",
            };
            assert_eq!(route.as_str(), expected);
        }
        assert_eq!(SelectionRoute::ALL.len(), ROUTES.len());
    }

    #[test]
    fn a_route_names_its_argument_where_it_has_one() {
        let phrases = [
            (SelectionRoute::FlagPath, "./work", "--vault ./work"),
            (
                SelectionRoute::FlagName,
                "work",
                "--vault work (registered)",
            ),
            (
                SelectionRoute::EnvironmentPath,
                "/data/work",
                "DOGTAG_VAULT /data/work",
            ),
            (
                SelectionRoute::EnvironmentName,
                "work",
                "DOGTAG_VAULT work (registered)",
            ),
        ];
        for (route, given, expected) in phrases {
            assert_eq!(requested(route, given).describe(), expected);
        }
        let nothing_was_requested = discovery().describe();
        assert_eq!(
            nothing_was_requested,
            "upward discovery from the current directory"
        );
    }

    #[test]
    fn a_route_with_no_argument_renders_its_placeholder_rather_than_a_blank() {
        let bare = Selection::new(SelectionRoute::FlagName, None);
        let asked = (bare.describe(), bare.requested(), bare.how());
        assert_eq!(
            asked,
            (
                "--vault <name> (registered)".to_owned(),
                None,
                SelectionRoute::FlagName
            )
        );
    }

    #[test]
    fn selections_clone_compare_and_format() {
        let selection = requested(SelectionRoute::FlagPath, "./work");
        assert_eq!(selection.clone(), selection);
        let rendered = format!("{selection:?}");
        assert!(rendered.contains("FlagPath"), "{rendered}");
        assert_ne!(selection, discovery());
    }

    #[test]
    fn a_resolved_contract_reports_its_version_and_its_capabilities() {
        let tree = Tree::new("model-resolved");
        let report = report_of(&tree, CLEAN);
        assert_eq!(
            report.contract,
            ContractFacts {
                present: true,
                state: "loaded",
                reason: None,
                version: VersionFacts {
                    found: Some(2),
                    min: 1,
                    max: 2,
                    classification: Some(VersionClass::Current),
                },
            }
        );
        let evaluated = evaluated(&report).expect("a resolved contract evaluates its sections");
        assert_eq!(
            (evaluated.types_declared, evaluated.catch_all.as_deref()),
            (1, Some("capture"))
        );
        assert_eq!(
            (&evaluated.lifecycle, evaluated.links),
            (&LifecycleDecl::None, LinkDialect::Wikilink)
        );
        let unclaimed = evaluated.identity_bearing.len() + evaluated.closed_write.len();
        assert_eq!(unclaimed, 0, "the one type declares only the catch-all");
    }

    #[test]
    fn an_unresolved_contract_keeps_every_fact_that_does_not_depend_on_it() {
        let tree = Tree::new("model-unresolved");
        let report = report_of(&tree, Body::new("contract_version = 3\n"));
        let refused = (
            report.contract.state,
            report.contract.present,
            report.contract.version.classification,
        );
        assert_eq!(refused, ("unresolved", true, Some(VersionClass::TooNew)));
        let reason = UnresolvedReason::VersionUnusable(VersionClass::TooNew).describe();
        assert_eq!(report.sections, Sections::NotEvaluated(reason));
        assert!(evaluated(&report).is_none());
        let survived = (
            report.installation.state,
            report.installation.actor.as_deref(),
            report.root.is_some(),
        );
        assert_eq!(survived, ("loaded", Some("A Maintainer"), true));
    }

    #[test]
    fn a_contract_that_is_not_there_is_the_one_report_that_says_absent() {
        let tree = Tree::new("model-missing");
        let root = tree.vault(CLEAN);
        fs::remove_file(root.contract_path()).expect("a contract this test owns");
        let report = doctor_report(
            &open(root, parse_installation(RECORD.as_str())),
            discovery(),
            &[],
        );
        assert!(!report.contract.present);
        assert_eq!(
            report.contract.version,
            VersionFacts {
                found: None,
                min: 1,
                max: 2,
                classification: None,
            },
            "the range this build reads is a fact whatever the file says"
        );
    }

    #[test]
    fn only_the_resolved_vaults_entry_is_reported() {
        let tree = Tree::new("model-registry");
        let root = tree.vault(CLEAN);
        let record = registering(&root);
        let report = doctor_report(&open(root, parse_installation(&record)), discovery(), &[]);
        let entry = report.installation.entry.expect("this vault is registered");
        assert_eq!(entry.name, "work");
        assert!(
            !entry.path.contains("elsewhere"),
            "the other registered vault must not reach the report"
        );
    }

    #[test]
    fn a_name_route_reports_the_entry_that_answered_it_however_the_path_is_spelled() {
        // A registered path is stored literally and the root it resolves to is
        // canonical, so comparing the two lexically fails for any entry whose
        // path is spelled differently — and the report then claimed no entry
        // for a vault it had reached *through* one. The name that resolved it
        // is the answer, and it needs no filesystem call to be right.
        let tree = Tree::new("model-registry-by-name");
        let root = tree.vault(CLEAN);
        let record = format!(
            "installation_version = 1\n\n[[vault]]\nname = \"work\"\npath = \"{}/../{}\"\n",
            root.path().display(),
            root.path()
                .file_name()
                .expect("the fixture root has a name")
                .to_string_lossy()
        );
        let opened = open(root, parse_installation(&record));
        let by_name = doctor_report(&opened, requested(SelectionRoute::FlagName, "work"), &[]);
        let entry = by_name
            .installation
            .entry
            .expect("the name that resolved this vault names its entry");
        assert_eq!(entry.name, "work");
    }

    #[test]
    fn a_vault_the_registry_does_not_carry_reports_no_entry() {
        let tree = Tree::new("model-unregistered");
        let report = report_of(&tree, CLEAN);
        assert_eq!(report.installation.entry, None);
        assert_eq!(
            report.installation.version.and_then(|facts| facts.found),
            Some(1)
        );
    }

    #[test]
    fn a_machine_with_no_record_reports_absence_rather_than_a_fault() {
        let tree = Tree::new("model-absent");
        let report = doctor_report(&open(tree.vault(CLEAN), no_record(&tree)), discovery(), &[]);
        assert_eq!(
            report.installation,
            InstallationFacts {
                state: "absent",
                version: None,
                actor: None,
                entry: None,
            }
        );
    }

    #[test]
    fn consumer_diagnostics_join_the_vaults_own_in_one_sorted_list() {
        let tree = Tree::new("model-extra");
        let planted = Diagnostic::kernel(KernelDiagnostic::DiscoveryNestedVault, "an ancestor");
        let vault = opened(&tree, Body::new("contract_version = 3\n"), RECORD);
        let report = doctor_report(&vault, discovery(), std::slice::from_ref(&planted));
        let reported: Vec<&str> = report
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect();
        assert_eq!(
            reported,
            ["discovery.nested-vault", "compat.contract-too-new"],
            "an unlocated diagnostic sorts before a located one"
        );
        assert_eq!(
            report.counts(),
            SeverityCounts {
                error: 1,
                warning: 1,
                info: 0
            }
        );
    }

    #[test]
    fn a_clean_vault_reports_nothing_at_any_severity() {
        let tree = Tree::new("model-clean");
        let report = report_of(&tree, CLEAN);
        assert!(report.diagnostics().is_empty());
        assert_eq!(report.counts(), SeverityCounts::zero());
    }

    #[test]
    fn reports_clone_compare_and_format() {
        let tree = Tree::new("model-derives");
        let report = report_of(&tree, CLEAN);
        assert_eq!(report.clone(), report);
        let refused = report_of(&tree, Body::new("contract_version = 3\n"));
        assert_ne!(report, refused);
        let rendered = format!("{report:?} {:?}", Severity::Error);
        assert!(rendered.contains("capture") && rendered.contains("Error"));
    }

    #[test]
    fn the_schema_version_is_its_own_clock() {
        // One tick per milestone: 2 was M3's, 3 is M4's, taken when the
        // `search` document — the milestone's first report shape — landed.
        assert_eq!(SCHEMA_VERSION, 3);
    }
}
