//! The executed body of each graduated scenario.
//!
//! One entry per graduated scenario, keyed by the scenario's id. The table is
//! the whole dispatch: a scenario with no entry has **no execution path**,
//! which the report refuses rather than skips, so graduating a scenario
//! without landing its case fails the suite instead of quietly passing.
//!
//! Every case runs against a temporary copy of the profile's corpus, created
//! fresh per pair. Nothing here reads a note, parses frontmatter, resolves a
//! link, or enumerates a directory under a vault root — at M2 there are no
//! notes to read, and the two files a case opens are the committed contract
//! and an installation record it wrote itself.

mod contract;
mod corpus;
mod discovery;
mod expect;
mod explain;
mod installation;
mod provenance;
mod scan;
mod version;

use std::path::Path;

use corpus::Corpus;
use expect::Checked;

/// One scenario's executed body, against a temporary copy of a corpus.
type Case = fn(&Corpus) -> Checked;

/// Every graduated scenario, by id.
///
/// The ten M2 scenarios graduated together — graduation is all-or-nothing —
/// and the harness's own tests assert that every scenario tagged `M2` is
/// `executable`, so a straggler fails the suite rather than sitting here
/// missing.
const CASES: &[(&str, Case)] = &[
    (
        "capability-cardinality-enforced",
        contract::capability_cardinality,
    ),
    (
        "conforming-contract-loads-with-zero-diagnostics",
        contract::conforming_contract,
    ),
    (
        "contract-explain-renders-every-declaration",
        explain::contract_explain,
    ),
    (
        "contract-loads-with-provenance",
        provenance::contract_loads_with_provenance,
    ),
    (
        "explicit-vault-root-is-used-exactly",
        discovery::explicit_root_used_exactly,
    ),
    (
        "incomplete-vault-root-halts-discovery",
        discovery::incomplete_root_halts,
    ),
    (
        "installation-record-cannot-supply-contract-settings",
        installation::record_cannot_supply_contract_settings,
    ),
    (
        "lifecycle-declaration-is-mandatory",
        contract::lifecycle_declaration,
    ),
    (
        "unsupported-contract-version-refuses-with-diagnosis",
        version::unsupported_version_refuses,
    ),
    (
        "vault-root-discovered-from-a-nested-path",
        discovery::nested_path_discovery,
    ),
];

/// The case for a scenario id, or `None` when the scenario has no execution
/// path at all.
pub fn case_for(scenario_id: &str) -> Option<Case> {
    CASES
        .iter()
        .find(|(id, _)| *id == scenario_id)
        .map(|(_, case)| *case)
}

/// Run one case against a fresh temporary copy of `corpus_dir`.
///
/// # Errors
///
/// Whatever the case reported, or the reason its corpus copy could not be made.
pub fn run(case: Case, corpus_dir: &Path, label: &str) -> Checked {
    let corpus = Corpus::copy_of(corpus_dir, label)?;
    case(&corpus)
}

/// How many scenarios have an execution path, for the harness's own tests.
pub fn graduated_count() -> usize {
    CASES.len()
}
