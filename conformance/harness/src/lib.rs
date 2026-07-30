//! Conformance harness for the Dogtag SDK.
//!
//! Every scenario in `conformance/scenarios/` runs against every fixture
//! profile in `conformance/profiles/`. There is no waiver mechanism: a
//! scenario expressible against only one profile fails the harness and is
//! triaged as either an incomplete configuration model or a personal
//! convention mistaken for an invariant. The mechanical channels for a
//! waiver — schema fields, stray files, a filtered cross product — are
//! enforced structurally (see [`Scenario`] and the strict directory
//! loaders); keeping the prose contracts profile-agnostic still rests on
//! review discipline until execution wiring closes the loop.
//!
//! At M1 every scenario is `pending` and every profile corpus is `scheduled`,
//! so the harness produces the complete scenarios × profiles matrix of
//! pending outcomes and nothing executes. The harness deliberately does not
//! depend on the `dogtag` SDK today (there is nothing to call); when the
//! first scenario graduates to `executable`, execution wiring lands here and
//! consumes only the SDK's public API.
//!
//! The crate is organized by the three stages of a harness run: the strict
//! fixture schemas ([`Scenario`], [`Profile`], and their parsers), the
//! strict directory loaders ([`load_scenarios`], [`load_profiles`]), and the
//! cross-product report ([`report`], [`pending_matrix`]). Everything is
//! re-exported here; the modules are an internal arrangement.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod error;
mod loader;
mod report;
mod schema;

pub use error::HarnessError;
pub use loader::{load_profiles, load_profiles_from, load_scenarios, load_scenarios_from};
pub use report::{Outcome, Pair, pending_matrix, report};
pub use schema::{
    CorpusStatus, Milestone, Profile, REQUIRED_PROFILES, Scenario, ScenarioStatus, is_kebab_case,
    parse_profile, parse_scenario,
};

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
