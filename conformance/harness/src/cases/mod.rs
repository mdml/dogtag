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

mod capture;
mod contract;
mod corpus;
mod derive;
mod discovery;
mod docs_native;
mod expect;
mod explain;
mod find;
mod installation;
mod link;
mod note;
mod provenance;
mod repository;
mod scan;
mod search;
mod surface;
mod version;

use std::path::Path;

use corpus::Corpus;
use expect::Checked;

/// One scenario's executed body, against a temporary copy of a corpus.
type Case = fn(&Corpus) -> Checked;

/// Every graduated scenario, by id.
///
/// Each milestone's scenarios graduated together — graduation is
/// all-or-nothing — and the harness's own tests assert that every scenario
/// tagged with a closed milestone is `executable`, so a straggler fails the
/// suite rather than sitting here missing.
const CASES: &[(&str, Case)] = &[
    (
        "ambiguous-bare-name-yields-link-diagnostic",
        link::ambiguous_bare_name,
    ),
    (
        "capture-birth-state-stamps-the-flag",
        capture::birth_state_stamps_the_flag,
    ),
    ("capture-body-is-verbatim", capture::body_is_verbatim),
    (
        "capture-collision-appends-suffix",
        capture::collision_appends_suffix,
    ),
    ("capture-commits-at-birth", capture::commits_at_birth),
    (
        "capture-exit-is-the-transaction-verdict",
        capture::exit_is_the_transaction_verdict,
    ),
    ("capture-lands-unfiled", capture::lands_unfiled),
    (
        "capture-preview-writes-nothing",
        capture::preview_writes_nothing,
    ),
    (
        "capture-repeat-is-deterministic",
        capture::repeat_is_deterministic,
    ),
    (
        "capture-result-names-recovery",
        capture::result_names_recovery,
    ),
    ("capture-without-actor-warns", capture::without_actor_warns),
    (
        "bare-name-link-resolves-when-unambiguous",
        link::bare_name_resolves,
    ),
    (
        "capability-cardinality-enforced",
        contract::capability_cardinality,
    ),
    (
        "closed-namespace-value-outside-vocabulary",
        note::closed_namespace_outside_vocabulary,
    ),
    (
        "conforming-contract-loads-with-zero-diagnostics",
        contract::conforming_contract,
    ),
    ("conforming-corpus-zero-diagnostics", note::conforming),
    (
        "contract-explain-renders-every-declaration",
        explain::contract_explain,
    ),
    (
        "contract-loads-with-provenance",
        provenance::contract_loads_with_provenance,
    ),
    ("dangling-typed-link-diagnostic", link::dangling_typed_link),
    (
        "explicit-vault-root-is-used-exactly",
        discovery::explicit_root_used_exactly,
    ),
    (
        "find-ambiguity-lists-candidates",
        find::ambiguity_lists_candidates,
    ),
    (
        "find-repeated-basename-requires-qualification",
        docs_native::repeated_basename_requires_qualification,
    ),
    (
        "find-resolves-unambiguous-name",
        find::resolves_unambiguous_name,
    ),
    (
        "frontmatter-sparse-notes-bind-by-default",
        docs_native::sparse_notes_bind_by_default,
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
        "list-filters-by-declared-lifecycle-axis",
        surface::list_filters_by_axis,
    ),
    (
        "markdown-link-resolution",
        docs_native::dialect_links_resolve,
    ),
    (
        "missing-required-property-diagnostic",
        note::missing_required_property,
    ),
    (
        "path-qualified-link-resolves",
        link::path_qualified_resolves,
    ),
    (
        "required-tag-namespace-missing",
        note::required_namespace_missing,
    ),
    (
        "search-composes-with-list-filters",
        search::composes_with_list_filters,
    ),
    (
        "search-empty-result-is-a-result",
        search::empty_result_is_a_result,
    ),
    (
        "search-membership-by-body-term",
        search::membership_by_body_term,
    ),
    (
        "search-phrase-matches-adjacent-words",
        search::phrase_matches_adjacent_words,
    ),
    ("search-prefix-wildcard", search::prefix_wildcard),
    (
        "search-repeat-is-deterministic",
        search::repeat_is_deterministic,
    ),
    (
        "search-repeated-basenames-stay-distinct",
        docs_native::repeated_basenames_stay_distinct,
    ),
    (
        "show-returns-document-model",
        surface::show_returns_document_model,
    ),
    (
        "supported-contract-version-loads-with-info",
        version::supported_version_loads_with_info,
    ),
    (
        "undeclared-key-reported-as-info",
        note::undeclared_key_is_info,
    ),
    ("unknown-type-diagnostic", note::unknown_type),
    (
        "unsupported-contract-version-refuses-with-diagnosis",
        version::unsupported_version_refuses,
    ),
    (
        "untyped-note-binds-to-catch-all",
        note::untyped_binds_to_catch_all,
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
