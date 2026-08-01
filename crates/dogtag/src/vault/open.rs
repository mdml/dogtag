//! Opening a vault, including the ones that are broken.
//!
//! [`open`] reads **exactly one file**: the contract at the root's
//! [`VaultRoot::contract_path`]. It opens no note, parses no frontmatter,
//! resolves no link, and enumerates no directory under the root — that line is
//! where this milestone actually ends, and crossing it would decide a traversal
//! policy (what counts as a note, which directories are skipped, whether
//! symlinks are followed) by accident.
//!
//! It is a **pure function of its explicit arguments**. It does not read the
//! installation record — the caller loads that, choosing the path through
//! `XDG_CONFIG_HOME` — and it does not inspect the root's trust, because the
//! home directory is an environment fact only a consumer has. Nothing here
//! consults an environment variable, a current directory, or process-global
//! state.
//!
//! # Why [`Opened`] has the shape it has
//!
//! It always carries the root, the installation record's state and the
//! diagnostic list, and the resolved contract is a `Result` **inside** it.
//!
//! A single atomic `open() -> Result<Vault>` cannot produce the report a
//! diagnosing surface is required to produce when the contract's version is out
//! of range: the root and the registry facts would be lost inside the error,
//! and those are exactly the facts a broken vault most needs reported. An
//! infallible `open` carrying only diagnostics has the opposite defect: nothing
//! in the type system would stop a caller acting on an unresolved contract,
//! which is caller-owned semantic reinterpretation arrived at by omission
//! rather than by decision.
//!
//! So semantic operations take a resolved `&Contract` and cannot be reached
//! without one. The cost is that every caller unwraps a `Result` inside a
//! struct; the benefit is that the partial state is ordinary rather than
//! exceptional, which is right for a tool whose first job is diagnosis.

use crate::contract::{Contract, ContractUnresolved, load_contract};
use crate::diagnostic::{Diagnostic, DiagnosticList};
use crate::installation::Installation;
use crate::provenance::Provenance;

use super::VaultRoot;

/// A vault that has been opened, whether or not its contract resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opened {
    root: VaultRoot,
    installation: Installation,
    contract: Result<Contract, ContractUnresolved>,
    diagnostics: Vec<Diagnostic>,
}

impl Opened {
    /// The root this vault was opened at.
    pub fn root(&self) -> &VaultRoot {
        &self.root
    }

    /// The installation record's state, exactly as it was supplied.
    pub fn installation(&self) -> &Installation {
        &self.installation
    }

    /// The resolved contract, or why there is none.
    ///
    /// This is the gate: an operation that interprets a corpus takes the
    /// `&Contract` and so cannot run against a vault whose contract did not
    /// resolve.
    pub fn contract(&self) -> Result<&Contract, &ContractUnresolved> {
        self.contract.as_ref()
    }

    /// Everything opening the vault had to say — the contract's and the
    /// installation record's together — in the deterministic total order.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Where each resolved leaf value came from, across both assets, in key
    /// order.
    ///
    /// An asset that did not resolve contributes nothing rather than
    /// contributing placeholders: there are no resolved values to attribute.
    /// The two key spaces are disjoint — the contract owns `contract_version`,
    /// `dialect.*`, `lifecycle.*`, `flag.*` and `type.*`, the record owns
    /// `installation_version`, `actor.*` and `vault.*` — so merging them
    /// answers about the vault as a whole without either asset shadowing the
    /// other.
    pub fn provenance(&self) -> Provenance {
        let mut merged = Provenance::new();
        if let Ok(contract) = &self.contract {
            merged.merge(contract.provenance().clone());
        }
        if let Some(record) = self.installation.record() {
            merged.merge(record.provenance().clone());
        }
        merged
    }
}

/// Opens the vault at `root`, against an installation record the caller loaded.
///
/// The contract is read from `root`'s [`contract_path`], and every diagnostic
/// either asset produced travels in one sorted list: a caller reporting on a
/// vault reports on the vault, not on two files it has to interleave itself.
///
/// This never fails. A missing, unreadable, malformed, out-of-range or invalid
/// contract is reported through [`Opened::contract`] and the diagnostics, with
/// the root and the installation state intact beside it.
///
/// [`contract_path`]: VaultRoot::contract_path
pub fn open(root: VaultRoot, installation: Installation) -> Opened {
    let load = load_contract(&root.contract_path());
    let mut diagnostics = DiagnosticList::new();
    diagnostics.extend(load.diagnostics);
    diagnostics.extend(installation.diagnostics().iter().cloned());
    Opened {
        root,
        installation,
        contract: load.contract,
        diagnostics: diagnostics.sorted(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compat::VersionClass;
    use crate::contract::UnresolvedReason;
    use crate::diagnostic::order;
    use crate::installation::{InstallationState, load_installation, parse_installation};
    use crate::provenance::Source;
    use crate::vault::{SENTINEL, root_at, tree::Tree};
    use core::cmp::Ordering;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering as Atomic};

    /// The smallest contract that resolves with nothing at all to report.
    const CLEAN: &str = concat!(
        "contract_version = 1\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    /// A contract whose lifecycle axis names its ordinary state, with a flag,
    /// a relationship, and the other link dialect.
    const NAMED_ORDINARY: &str = concat!(
        "contract_version = 1\n",
        "\n[dialect]\nlinks = \"markdown\"\n",
        "\n[lifecycle]\naxis = \"status\"\nordinary = { value = \"active\" }\n",
        "\n[[flag]]\nproperty = \"pinned\"\n",
        "\n[[type]]\nname = \"note\"\ncapabilities = [\"catch-all\"]\n",
        "\n  [[type.property]]\n  name = \"status\"\n  kind = \"enum\"\n",
        "  values = [\"active\", \"archived\"]\n  required = true\n",
        "\n  [[type.property]]\n  name = \"pinned\"\n  kind = \"boolean\"\n",
        "\n  [[type.relationship]]\n  predicate = \"involves\"\n",
    );

    /// A contract whose ordinary state is the **absence** of a value.
    const ABSENT_ORDINARY: &str = concat!(
        "contract_version = 1\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\naxis = \"standing\"\nordinary = { absent = true }\n",
        "\n[[type]]\nname = \"person\"\ncapabilities = [\"identity-bearing\"]\n",
        "\n  [[type.property]]\n  name = \"standing\"\n  kind = \"enum\"\n",
        "  values = [\"dormant\", \"closed\"]\n",
        "\n[[type]]\nname = \"unfiled\"\ncapabilities = [\"catch-all\"]\n",
    );

    /// Every contract this module authors, named so a failure says which.
    ///
    /// They are authored here rather than read from the conformance profiles.
    /// Whether *those* load clean is the conformance suite's own assertion —
    /// `conforming-contract-loads-with-zero-diagnostics` runs it against every
    /// profile with a built corpus — and reaching sideways into that tree from
    /// here would point the dependency the wrong way and leave a packaged crate
    /// whose tests cannot build, since packaging carries only crate-local
    /// files. The bytes are still written to a temporary vault and opened from
    /// disk, because opening a real directory is the thing under test.
    const AUTHORED: [(&str, &str); 3] = [
        ("no axis", CLEAN),
        ("a named ordinary state", NAMED_ORDINARY),
        ("an absent ordinary state", ABSENT_ORDINARY),
    ];

    /// A record that loads, registering one vault and naming an actor.
    const RECORD: &str = concat!(
        "installation_version = 1\n",
        "\n[actor]\nname = \"A Maintainer\"\n",
        "\n[[vault]]\nname = \"work\"\npath = \"/data/vaults/work\"\n",
    );

    /// A directory name no other call in this process will pick.
    ///
    /// Every vault a test asks for is named here rather than by the test: none
    /// of these tests cares what the directory is called, only what its
    /// contract says.
    fn next_name() -> String {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        format!("vault-{}", COUNTER.fetch_add(1, Atomic::Relaxed))
    }

    /// A vault root of its own whose contract is `body`.
    fn vault_holding(tree: &Tree, body: &str) -> VaultRoot {
        let path = tree.vault(&next_name());
        fs::write(path.join(SENTINEL), body).expect("a contract this test owns");
        root_at(&path).expect("a directory holding the sentinel is a vault root")
    }

    /// The state of a machine that has never registered a vault. Absence is a
    /// state rather than a fault, so it carries no diagnostic of its own — and
    /// that is what lets a clean vault open with an empty list.
    fn no_record(tree: &Tree) -> Installation {
        load_installation(&tree.absent("no-installation-record.toml"))
    }

    /// A vault opened against a machine with no installation record.
    fn opened_holding(tree: &Tree, body: &str) -> Opened {
        open(vault_holding(tree, body), no_record(tree))
    }

    /// Why a contract did not resolve — asserting, for every one of them, that
    /// the root, the record and the diagnostics survived the failure.
    fn unresolved(tree: &Tree, body: &str) -> UnresolvedReason {
        let root = vault_holding(tree, body);
        let expected = root.path().to_path_buf();
        let opened = open(root, parse_installation(RECORD));
        assert_eq!(opened.root().path(), expected);
        assert!(opened.installation().record().is_some());
        assert!(!opened.diagnostics().is_empty());
        opened
            .contract()
            .expect_err("the contract must not resolve")
            .reason
    }

    fn ids(opened: &Opened) -> Vec<&str> {
        opened
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect()
    }

    fn keys(provenance: &Provenance) -> Vec<String> {
        provenance
            .entries()
            .map(|entry| entry.key.clone())
            .collect()
    }

    #[test]
    fn a_clean_vault_opens_with_nothing_at_all_to_say() {
        let tree = Tree::new("open-clean");
        let opened = opened_holding(&tree, CLEAN);
        let contract = opened.contract().expect("a resolved contract");
        assert_eq!(
            contract.catch_all().map(|kind| kind.name()),
            Some("capture")
        );
        assert!(opened.diagnostics().is_empty());
        assert_eq!(opened.installation().state(), &InstallationState::Absent);
    }

    #[test]
    fn every_shape_a_contract_takes_opens_with_no_diagnostic_at_any_severity() {
        let tree = Tree::new("open-authored");
        for (shape, body) in AUTHORED {
            let opened = opened_holding(&tree, body);
            let reported = ids(&opened);
            assert!(
                opened.contract().is_ok(),
                "a contract declaring {shape} must resolve"
            );
            assert!(
                reported.is_empty(),
                "a contract declaring {shape} reported {reported:?}"
            );
        }
    }

    #[test]
    fn opening_reads_the_contract_at_the_root_and_nothing_else() {
        let tree = Tree::new("open-one-file");
        let root = vault_holding(&tree, CLEAN);
        // A note beside the contract, and a directory under the root: neither
        // is opened, neither is enumerated, and neither changes the answer.
        fs::write(root.path().join("note.md"), "not read\n").expect("a note this test owns");
        fs::create_dir(root.path().join("subdirectory")).expect("a directory this test owns");
        let opened = open(root, no_record(&tree));
        assert!(opened.contract().is_ok());
        assert!(opened.diagnostics().is_empty());
    }

    #[test]
    fn a_vault_whose_contract_is_gone_keeps_its_root_and_its_record() {
        let tree = Tree::new("open-missing");
        let root = vault_holding(&tree, CLEAN);
        let expected = root.path().to_path_buf();
        fs::remove_file(root.contract_path()).expect("a contract this test owns");
        let opened = open(root, parse_installation(RECORD));
        assert_eq!(opened.root().path(), expected);
        assert_eq!(
            opened.contract().expect_err("no contract").reason,
            UnresolvedReason::Missing
        );
        assert_eq!(ids(&opened), ["contract.unreadable"]);
        assert!(opened.installation().record().is_some());
    }

    #[test]
    fn a_contract_that_cannot_be_read_is_reported_rather_than_returned() {
        let tree = Tree::new("open-unreadable");
        let root = vault_holding(&tree, CLEAN);
        let contract = root.contract_path();
        fs::remove_file(&contract).expect("a contract this test owns");
        fs::create_dir(&contract).expect("a directory where the contract was");
        let opened = open(root, parse_installation(RECORD));
        assert_eq!(
            opened.contract().expect_err("unreadable").reason,
            UnresolvedReason::Unreadable
        );
        assert_eq!(ids(&opened), ["contract.unreadable"]);
    }

    #[test]
    fn every_way_a_contract_fails_to_resolve_keeps_the_rest_of_the_report() {
        let tree = Tree::new("open-unresolved");
        assert_eq!(
            unresolved(&tree, "contract_version = 1\r\n"),
            UnresolvedReason::Encoding
        );
        assert_eq!(
            unresolved(&tree, "contract_version = = 1\n"),
            UnresolvedReason::Malformed
        );
        assert_eq!(
            unresolved(&tree, "contract_version = 0\n"),
            UnresolvedReason::VersionUnusable(VersionClass::BelowFloor)
        );
        assert_eq!(
            unresolved(&tree, "contract_version = 2\n"),
            UnresolvedReason::VersionUnusable(VersionClass::TooNew)
        );
        assert_eq!(
            unresolved(&tree, "contract_version = 1\n"),
            UnresolvedReason::Invalid
        );
    }

    #[test]
    fn an_out_of_range_version_is_reported_under_its_compatibility_identifier() {
        let tree = Tree::new("open-version");
        let below = opened_holding(&tree, "contract_version = 0\n");
        assert_eq!(ids(&below), ["compat.contract-below-supported-floor"]);
        let above = opened_holding(&tree, "contract_version = 2\n");
        assert_eq!(ids(&above), ["compat.contract-too-new"]);
        assert_eq!(
            above.contract().expect_err("too new").version,
            Some(2),
            "the declared version survives the refusal"
        );
    }

    #[test]
    fn both_assets_diagnostics_arrive_together_in_the_total_order() {
        let tree = Tree::new("open-merged");
        let root = vault_holding(&tree, "contract_version = 1\nstray = true\n");
        let record = parse_installation("installation_version = 1\nstray = true\n");
        let opened = open(root, record);
        let reported = ids(&opened);
        assert!(reported.contains(&"contract.unknown-key"));
        assert!(reported.contains(&"installation.unknown-key"));
        assert!(
            opened
                .diagnostics()
                .windows(2)
                .all(|pair| order::compare(&pair[0], &pair[1]) != Ordering::Greater),
            "the merged list must be in the deterministic total order: {reported:?}"
        );
        assert_eq!(opened.installation().state(), &InstallationState::Unusable);
    }

    #[test]
    fn provenance_merges_both_assets_in_key_order() {
        let tree = Tree::new("open-provenance");
        let opened = open(vault_holding(&tree, CLEAN), parse_installation(RECORD));
        let merged = opened.provenance();
        let contract = opened.contract().expect("resolved").provenance().len();
        let record = opened
            .installation()
            .record()
            .expect("loaded")
            .provenance()
            .len();
        assert_eq!(merged.len(), contract + record);
        assert_eq!(
            merged.get("dialect.links").map(|entry| entry.source),
            Some(Source::Contract)
        );
        assert_eq!(
            merged.get("vault.work.path").map(|entry| entry.source),
            Some(Source::Installation)
        );
        let mut ordered = keys(&merged);
        ordered.sort();
        assert_eq!(keys(&merged), ordered);
    }

    #[test]
    fn an_asset_that_did_not_resolve_contributes_no_provenance() {
        let tree = Tree::new("open-partial");
        let record_only = open(
            vault_holding(&tree, "contract_version = 2\n"),
            parse_installation(RECORD),
        );
        assert_eq!(
            keys(&record_only.provenance()),
            [
                "actor.name",
                "installation_version",
                "vault.work.name",
                "vault.work.path"
            ]
        );
        let contract_only = keys(&opened_holding(&tree, CLEAN).provenance());
        assert!(contract_only.contains(&"contract_version".to_owned()));
        assert!(!contract_only.iter().any(|key| key.starts_with("vault.")));
    }

    #[test]
    fn an_opened_vault_clones_compares_and_formats() {
        let tree = Tree::new("open-derives");
        let opened = opened_holding(&tree, CLEAN);
        let copy = opened.clone();
        assert_eq!(copy, opened);
        assert_ne!(opened, opened_holding(&tree, "contract_version = 2\n"));
        assert!(format!("{opened:?}").contains("capture"));
    }
}
