//! `installation-record-cannot-supply-contract-settings`: the partition
//! between the committed contract and the machine-local record is structural.
//!
//! **This case has no per-profile source.** One machine-local record serves
//! every profile, so the smuggling half runs one identical input against each
//! of them. The per-profile half is the second assertion — that the profile's
//! *own* contract, resolved with a smuggling record **in scope**, is the
//! contract that vault resolves with no record at all. Saying so is part of
//! the rule: pretending the cross product multiplies this case would claim a
//! coverage it does not have.

use dogtag::installation::{Installation, load_installation};
use dogtag::report::contract_json;
use dogtag::vault::{Opened, open};

use super::corpus::Corpus;
use super::expect::{Checked, did_not_resolve, require, require_id};

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

/// The dialect, named on its own because the per-profile half below runs one
/// particular smuggled setting rather than *whichever one is fifth*.
const SMUGGLED_DIALECT: &str = "\n[dialect]\nlinks = \"markdown\"\n";

/// The five contract-owned settings the scenario enumerates, each appended to
/// a record that is otherwise well-formed.
///
/// **Five inputs, about three checked behaviours.** The record's walk sweeps
/// the keys of the table it is given, so the first three all arrive as the one
/// root key `type` and are indistinguishable to it; what the first three add
/// is that a reader of this list can see the scenario's enumeration is covered
/// input by input, not that three separate rejections were observed.
const SMUGGLED: &[(&str, &str)] = &[
    ("a type declaration", "\n[[type]]\nname = \"smuggled\"\n"),
    (
        "a capability assignment",
        "\n[[type]]\nname = \"smuggled\"\ncapabilities = [\"catch-all\"]\n",
    ),
    (
        "a property requirement",
        "\n[[type.property]]\nname = \"title\"\nkind = \"string\"\nrequired = true\n",
    ),
    ("the lifecycle declaration", "\n[lifecycle]\nnone = true\n"),
    ("the dialect", SMUGGLED_DIALECT),
];

/// `installation-record-cannot-supply-contract-settings`.
pub fn record_cannot_supply_contract_settings(corpus: &Corpus) -> Checked {
    for (index, (what, table)) in SMUGGLED.iter().enumerate() {
        refuses(corpus, index, what, table)?;
    }
    // The per-profile half, run against this profile's own committed contract.
    contract_is_unaffected(corpus, SMUGGLED_DIALECT)
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

/// The record the per-profile half writes, named so a reader of the temp tree
/// can tell it from the smuggling attempts above.
const IN_SCOPE: &str = "record-in-scope.toml";

/// What the per-profile half's two assertions are about.
const IN_SCOPE_VAULT: &str = "a vault opened with a smuggling record in scope";
const IN_SCOPE_CONTRACT: &str =
    "the committed contract of a vault opened with a smuggling record in scope";

/// The per-profile half: this profile's **own** committed contract, resolved
/// with a smuggling record in scope, is the contract the same vault resolves
/// with no record at all — every declaration, and every leaf's attribution.
///
/// The earlier version of this assertion called `clean_contract()` on a vault
/// no record had ever been opened against. The record and the contract never
/// met, so *unaffected* held by the harness's own filesystem construction, and
/// the case was an unlabelled second copy of the conforming-contract case.
/// Here the vault is opened **with** the record, and the record is proved to
/// have been in scope by finding its refusal in that vault's own diagnostic
/// list.
///
/// What that makes observable is bounded, and the bound is worth stating
/// plainly rather than dressing up. `open` reads the contract from the vault
/// root and never passes the installation record into contract resolution, so
/// as this SDK is shaped today no record can reach a contract value or a
/// contract leaf's source by any route, and this comparison cannot fail. What
/// it is, honestly, is a regression guard placed where the failure would
/// appear: the two artifacts are really built with the record present, and
/// `contract_json` carries every leaf's source, so the day resolution starts
/// consulting the record a leaf would render as `installation` and this names
/// the line that changed. The earlier version could not have caught that.
fn contract_is_unaffected(corpus: &Corpus, table: &str) -> Checked {
    // The control: this profile's own contract, loading clean, with no record
    // anywhere near it.
    let control = contract_json(&corpus.vault_root()?, &corpus.clean_contract()?);
    let path = corpus.write_record(IN_SCOPE, &format!("{WELL_FORMED}{table}"))?;
    let opened = open(corpus.vault_root()?, load_installation(&path));
    require_id(opened.diagnostics(), UNKNOWN_KEY, IN_SCOPE_VAULT)?;
    same_rendering(&control, &rendering(&opened)?, IN_SCOPE_CONTRACT)
}

/// An opened vault's contract as JSON — every declaration and every leaf's
/// source — or why there is none.
fn rendering(opened: &Opened) -> Result<String, String> {
    opened
        .contract()
        .map(|contract| contract_json(opened.root(), contract))
        .map_err(|why| did_not_resolve(why, IN_SCOPE_CONTRACT))
}

/// Two renderings of one contract, compared line by line so a failure names
/// the leaf that changed rather than reprinting two documents at the reader.
fn same_rendering(control: &str, observed: &str, subject: &str) -> Checked {
    require(control == observed, || {
        let what = control
            .lines()
            .zip(observed.lines())
            .find(|(expected, found)| expected != found)
            .map_or_else(
                || "one rendering stops where the other continues".to_owned(),
                |(expected, found)| format!("`{expected}` became `{found}`"),
            );
        format!(
            "{subject} must be the contract the same vault resolves with no record in scope, \
             but {what}"
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

    /// A contract that resolved differently with the record in scope is not
    /// unaffected, and the failure names the leaf that changed rather than
    /// reprinting the document — including when one rendering simply stops.
    #[test]
    fn a_contract_that_changed_under_a_record_names_what_changed() {
        let changed = same_rendering(
            "  \"links\": \"wikilink\"",
            "  \"links\": \"markdown\"",
            IN_SCOPE_CONTRACT,
        )
        .expect_err("a contract that resolved differently is not unaffected");
        assert!(
            changed.contains("`  \"links\": \"wikilink\"` became `  \"links\": \"markdown\"`"),
            "the failure names the leaf that changed: {changed}"
        );
        let truncated = same_rendering("one\ntwo", "one", IN_SCOPE_CONTRACT)
            .expect_err("a rendering carrying half the leaves is not the same rendering");
        assert!(
            truncated.contains("stops where the other continues"),
            "the failure says how the two differ: {truncated}"
        );
    }

    /// A vault whose contract did not resolve has no rendering to compare, so
    /// the case reports the reason rather than unwinding the matrix.
    #[test]
    fn a_contract_that_did_not_resolve_reports_the_reason() {
        let corpus = Corpus::holding("installation-unresolved-contract", "contract_version = 4\n");
        let opened = corpus
            .opened_without_a_record()
            .expect("a directory holding a contract is a vault root");
        let detail = rendering(&opened).expect_err("a version-2 contract does not resolve");
        assert!(
            detail.contains(IN_SCOPE_CONTRACT),
            "the failure names the subject: {detail}"
        );
        assert!(
            detail.contains("did not resolve"),
            "the failure says what happened: {detail}"
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
