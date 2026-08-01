//! `installation-record-cannot-supply-contract-settings`: the partition
//! between the committed contract and the machine-local record is structural.
//!
//! **This case has no per-profile source.** One machine-local record serves
//! every profile, so the smuggling half runs one identical input against each
//! of them. The per-profile half is the second assertion — that the profile's
//! *own* contract still loads clean and is unaffected by anything the local
//! record tried. Saying so is part of the rule: pretending the cross product
//! multiplies this case would claim a coverage it does not have.

use dogtag::installation::{Installation, load_installation};

use super::corpus::Corpus;
use super::expect::{Checked, require, require_id};

/// The diagnostic that refuses a record straying outside its authority.
///
/// Nothing polices a list of forbidden keys: the record's key space simply
/// does not contain the contract's, and an unknown key is fatal. The partition
/// is therefore structural rather than enforced, which is why a setting can
/// never be *quietly* supplied from the wrong scope.
const UNKNOWN_KEY: &str = "installation.unknown-key";

/// A well-formed record, to which each contract-owned table is appended in
/// turn. Without this the rejections below would prove nothing.
const WELL_FORMED: &str = "installation_version = 1\n\n[actor]\nname = \"A Maintainer\"\n";

/// The contract-owned settings a local record might try to supply: a type
/// declaration, the dialect, and the lifecycle declaration.
const SMUGGLED: &[(&str, &str)] = &[
    (
        "a type declaration",
        "\n[[type]]\nname = \"smuggled\"\ncapabilities = [\"catch-all\"]\n",
    ),
    ("the dialect", "\n[dialect]\nlinks = \"markdown\"\n"),
    ("the lifecycle declaration", "\n[lifecycle]\nnone = true\n"),
];

/// `installation-record-cannot-supply-contract-settings`.
pub fn record_cannot_supply_contract_settings(corpus: &Corpus) -> Checked {
    for (index, (what, table)) in SMUGGLED.iter().enumerate() {
        refuses(corpus, index, what, table)?;
    }
    // The per-profile half: this profile's own committed contract is
    // untouched by any of it, and still loads with zero diagnostics.
    corpus.clean_contract().map(|_| ())
}

/// One smuggling attempt: a well-formed record plus a contract-owned table.
fn refuses(corpus: &Corpus, index: usize, what: &str, table: &str) -> Checked {
    let name = format!("smuggling-{index}.toml");
    let path = corpus.write_record(&name, &format!("{WELL_FORMED}{table}"))?;
    let installation = load_installation(&path);
    require_id(
        installation.diagnostics(),
        UNKNOWN_KEY,
        &format!("a record supplying {what}"),
    )?;
    must_not_load(&installation, what)
}

/// A record that strayed outside its authority does not load **at all**.
///
/// There is no half-loaded record keeping the keys it was entitled to: an
/// unknown key is fatal, so the setting can never be quietly supplied from the
/// wrong scope while the rest of the record still takes effect.
fn must_not_load(installation: &Installation, what: &str) -> Checked {
    require(installation.record().is_none(), || {
        format!(
            "a record supplying {what} must not load, but its state is `{}`",
            installation.state()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A record that smuggles nothing reports no unknown key, so the first
    /// half of the assertion fails — and says what the record reported instead
    /// of only that it was not what was wanted.
    #[test]
    fn a_record_that_smuggles_nothing_fails_the_assertion() {
        let corpus = Corpus::empty("installation-nothing-smuggled");
        let detail = refuses(&corpus, 0, "nothing at all", "")
            .expect_err("a record within its authority reports no unknown key");
        assert!(
            detail.contains("a record supplying nothing at all"),
            "the failure names the subject: {detail}"
        );
        assert!(
            detail.contains("must report `installation.unknown-key`"),
            "the failure names the identifier: {detail}"
        );
        assert!(
            detail.contains("but reported nothing"),
            "the failure says what arrived: {detail}"
        );
    }

    /// The second half is about the record not loading, and it names the state
    /// the record reached rather than only that it was the wrong one.
    #[test]
    fn a_record_that_loads_names_the_state_it_reached() {
        let corpus = Corpus::empty("installation-record-loads");
        let path = corpus
            .write_record("well-formed.toml", WELL_FORMED)
            .expect("a record is written beside the vault");
        let detail = must_not_load(&load_installation(&path), "the dialect")
            .expect_err("a well-formed record loads");
        assert!(
            detail.contains("a record supplying the dialect must not load"),
            "the failure names the subject: {detail}"
        );
        assert!(
            detail.contains("`loaded`"),
            "the failure names the state: {detail}"
        );
    }
}
