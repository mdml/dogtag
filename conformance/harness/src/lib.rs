//! Conformance harness for the Dogtag SDK.
//!
//! Every scenario in `conformance/scenarios/` runs against every fixture
//! profile in `conformance/profiles/`. There is no waiver mechanism: a
//! scenario expressible against only one profile fails the harness and is
//! triaged as either an incomplete configuration model or a personal
//! convention mistaken for an invariant. The mechanical channels for a
//! waiver — schema fields, stray files, a filtered cross product — are
//! enforced structurally (see [`Scenario`] and the strict directory loaders);
//! keeping the prose contracts profile-agnostic still rests on review
//! discipline.
//!
//! A pair runs when its scenario is `executable` and its profile's corpus is
//! `built`; every other pair reports why it did not — the scenario still
//! prose, the corpus not built, or both. The matrix distinguishes the
//! outcomes: a pair that ran and a pair skipped for want of a corpus render
//! differently, so a run covering a subset of profiles cannot read as a
//! complete matrix.
//!
//! The harness consumes the SDK's **public API only**, which makes it a
//! permanent test that the public API is sufficient: any private hook it
//! needed would be an architecture bug rather than a reason to widen anything.
//!
//! The crate is organized by the stages of a harness run: the strict fixture
//! schemas ([`Scenario`], [`Profile`], and their parsers), the strict
//! directory loaders ([`load_scenarios`], [`load_profiles`]), the execution
//! path ([`Execution`], [`SdkExecution`]), and the cross-product report
//! ([`report`], [`matrix`]). [`TempTree`] and the contract [`transform`]
//! module support the executed cases and the harness's own tests alike.
//! Everything is re-exported here; the modules are an internal arrangement.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod cases;
mod error;
mod execution;
mod loader;
mod report;
mod schema;
mod temptree;

pub mod transform;

pub use error::HarnessError;
pub use execution::{Execution, NoExecution, SdkExecution};
pub use loader::{load_profiles, load_profiles_from, load_scenarios, load_scenarios_from};
pub use report::{Outcome, Pair, matrix, report};
pub use schema::{
    CORPORA_EVER_BUILT, CorpusStatus, Milestone, Profile, REQUIRED_PROFILES, Scenario,
    ScenarioStatus, is_kebab_case, parse_profile, parse_scenario,
};
pub use temptree::{TempTree, copy_tree};

use std::path::PathBuf;

/// Root of the `conformance/` directory (parent of the harness crate).
pub fn conformance_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("harness crate lives directly under conformance/")
        .to_path_buf()
}

/// `conformance/scenarios/`.
pub fn scenarios_dir() -> PathBuf {
    conformance_root().join("scenarios")
}

/// `conformance/profiles/`.
pub fn profiles_dir() -> PathBuf {
    conformance_root().join("profiles")
}

/// How many scenarios have an execution path behind them.
///
/// Exposed so the harness's own tests can assert it against the number of
/// scenarios that have graduated: graduation is all-or-nothing, so the two
/// numbers must agree.
pub fn graduated_case_count() -> usize {
    cases::graduated_count()
}
