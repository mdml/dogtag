//! A private, temporary copy of one profile's corpus, and the derived copies
//! the non-conforming cases run against.

use std::fs;
use std::path::{Path, PathBuf};

use dogtag::contract::{Contract, ContractLoad, load_contract};
use dogtag::installation::{Installation, load_installation};
use dogtag::vault::{Opened, VaultRoot, open, root_at};

use crate::temptree::{TempTree, copy_tree};
use crate::transform::Transformed;

use super::expect::{Checked, did_not_resolve, require, require_clean};

/// The directory name a corpus copy is placed under inside its temp tree.
const VAULT: &str = "vault";

/// A path inside a temp tree that no installation record was ever written to,
/// so `load_installation` answers *absent* — which is a state, not a fault.
const NO_RECORD: &str = "no-installation-record.toml";

/// A temporary copy of one profile's corpus: a vault root and its committed
/// contract, and nothing else at M2.
///
/// Every executed pair gets a fresh one. The run writes into it — installation
/// records, nested directories, symbolic links, transformed contracts — and
/// must never write into the checkout, and the copy's permissions are
/// normalized so a developer's umask cannot decide a conformance result.
#[derive(Debug)]
pub struct Corpus {
    tree: TempTree,
    root: PathBuf,
}

impl Corpus {
    /// Copy the corpus at `source` into a fresh temp tree.
    ///
    /// # Errors
    ///
    /// Any filesystem failure, named so the pair reports why it could not even
    /// be set up.
    pub fn copy_of(source: &Path, label: &str) -> Result<Self, String> {
        let tree = TempTree::new(label);
        let root = tree.path().join(VAULT);
        copy_tree(source, &root).map_err(|error| {
            format!("copying the corpus at {} failed: {error}", source.display())
        })?;
        Ok(Corpus { tree, root })
    }

    /// The copied vault root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The temp tree the copy lives in, which is where a case puts anything
    /// that must sit *outside* the vault.
    pub fn tree(&self) -> &Path {
        self.tree.path()
    }

    /// The copy's root, verified through the SDK.
    ///
    /// # Errors
    ///
    /// The diagnostic `root_at` refused with, rendered.
    pub fn vault_root(&self) -> Result<VaultRoot, String> {
        root_at(&self.root).map_err(|diagnostic| {
            format!(
                "the copied corpus is not a vault root: {}: {}",
                diagnostic.id.as_str(),
                diagnostic.message
            )
        })
    }

    /// The committed contract's bytes.
    ///
    /// # Errors
    ///
    /// Any filesystem failure reading it.
    pub fn contract_text(&self) -> Result<String, String> {
        let path = self.vault_root()?.contract_path();
        fs::read_to_string(&path)
            .map_err(|error| format!("reading {} failed: {error}", path.display()))
    }

    /// Replaces the committed contract's bytes.
    ///
    /// # Errors
    ///
    /// Any filesystem failure writing it.
    pub fn write_contract(&self, text: &str) -> Checked {
        let path = self.vault_root()?.contract_path();
        fs::write(&path, text)
            .map_err(|error| format!("writing {} failed: {error}", path.display()))
    }

    /// Reads the committed contract, whatever it says.
    ///
    /// # Errors
    ///
    /// Only a root that will not verify; a contract that does not load is a
    /// [`ContractLoad`], not an error.
    pub fn load(&self) -> Result<ContractLoad, String> {
        Ok(load_contract(&self.vault_root()?.contract_path()))
    }

    /// Reads the committed contract and requires it to load with zero
    /// diagnostics at any severity.
    ///
    /// # Errors
    ///
    /// A diagnostic at any severity, or a contract that did not resolve.
    pub fn clean_contract(&self) -> Result<Contract, String> {
        let load = self.load()?;
        require_clean(&load.diagnostics, "the committed contract")?;
        load.contract
            .map_err(|why| did_not_resolve(&why, "the committed contract"))
    }

    /// Opens the vault against no installation record.
    ///
    /// # Errors
    ///
    /// Only a root that will not verify; `open` itself never fails.
    pub fn opened_without_a_record(&self) -> Result<Opened, String> {
        let installation = load_installation(&self.tree.path().join(NO_RECORD));
        Ok(open(self.vault_root()?, installation))
    }

    /// Writes an installation record into the temp tree and returns its path.
    ///
    /// The record lives beside the vault rather than in the machine's real
    /// configuration directory: every SDK entry point takes the path
    /// explicitly, so nothing here has to touch a developer's own record.
    ///
    /// # Errors
    ///
    /// Any filesystem failure writing it.
    pub fn write_record(&self, name: &str, text: &str) -> Result<PathBuf, String> {
        let path = self.tree.path().join(name);
        fs::write(&path, text)
            .map_err(|error| format!("writing {} failed: {error}", path.display()))?;
        Ok(path)
    }

    /// A second copy whose committed contract has been transformed.
    ///
    /// Constructing one performs two of the three assertions every derived
    /// case owes: **the untransformed contract loads clean**, and **the
    /// transformed bytes differ from the original**. The third — that the
    /// expected diagnostic identifier appears — belongs to the case, because
    /// only the case knows which identifier it is about.
    ///
    /// # Errors
    ///
    /// A contract that does not load clean before transformation, a transform
    /// that could not find its target, a transform that changed no bytes, or a
    /// filesystem failure.
    pub fn derived(
        &self,
        label: &str,
        transform: impl Fn(&str) -> Transformed,
    ) -> Result<Self, String> {
        self.clean_contract()?;
        let original = self.contract_text()?;
        let transformed = transform(&original).map_err(|failure| failure.to_string())?;
        require(transformed != original, || {
            format!(
                "the `{label}` transformation left the contract byte-identical, so the derived \
                 case would test nothing"
            )
        })?;
        let derived = Corpus::copy_of(&self.root, label)?;
        derived.write_contract(&transformed)?;
        Ok(derived)
    }
}

/// A record that loads: the smallest one carrying an actor and a registry
/// entry, so the `installation` provenance source is reachable at all.
///
/// The path is filled in per corpus, because a registry entry naming a path
/// that is not a vault root is itself a diagnostic.
pub fn valid_record(root: &Path) -> String {
    format!(
        "installation_version = 1\n\n[actor]\nname = \"A Maintainer\"\n\n\
         [[vault]]\nname = \"fixture\"\npath = \"{}\"\n",
        root.display()
    )
}

/// Requires that `installation` loaded and reported nothing.
///
/// # Errors
///
/// Any diagnostic, or a state other than loaded.
pub fn require_record_loaded(installation: &Installation) -> Checked {
    require_clean(installation.diagnostics(), "the installation record")?;
    require(installation.record().is_some(), || {
        format!(
            "the installation record must load, but its state is `{}`",
            installation.state()
        )
    })
}

/// A contract that loads clean and declares that this corpus has no life axis.
///
/// The harness's own tests are **not conformance cases**. They never run
/// against a profile, never read a corpus, and never outlive their temporary
/// directory, so a contract written here is an input to a test *of the
/// assertions themselves* rather than a checked-in negative fixture. The
/// derived-not-authored rule governs what a scenario runs against, and none of
/// these is one.
#[cfg(test)]
pub const NO_AXIS: &str = concat!(
    "contract_version = 1\n",
    "\n[dialect]\nlinks = \"wikilink\"\n",
    "\n[lifecycle]\nnone = true\n",
    "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
);

/// The same contract declaring a life axis whose ordinary state is the absence
/// of a value, over a type that carries a property, a relationship and a flag —
/// so every section of a rendering of it has something in it.
#[cfg(test)]
pub const WITH_AXIS: &str = concat!(
    "contract_version = 1\n",
    "\n[dialect]\nlinks = \"wikilink\"\n",
    "\n[lifecycle]\naxis = \"status\"\nordinary = { absent = true }\n",
    "\n[[flag]]\nproperty = \"leaned_on\"\n",
    "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    "\n  [[type.property]]\n  name = \"title\"\n  kind = \"string\"\n  required = true\n",
    "\n  [[type.property]]\n  name = \"leaned_on\"\n  kind = \"boolean\"\n",
    "\n  [[type.property]]\n  name = \"status\"\n  kind = \"enum\"\n",
    "  values = [\"archived\"]\n  required = false\n",
    "\n  [[type.relationship]]\n  predicate = \"mentions\"\n",
);

#[cfg(test)]
impl Corpus {
    /// A corpus directory holding nothing at all, so it is not a vault root.
    ///
    /// # Panics
    ///
    /// If the directory cannot be created. A test that cannot obtain scratch
    /// space has nothing to assert.
    pub fn empty(label: &str) -> Self {
        let tree = TempTree::new(label);
        let root = tree.path().join(VAULT);
        fs::create_dir_all(&root).expect("a corpus directory");
        Corpus { tree, root }
    }

    /// A vault whose committed contract is `text`, and which holds nothing
    /// else.
    ///
    /// # Panics
    ///
    /// If the contract cannot be written.
    pub fn holding(label: &str, text: &str) -> Self {
        let corpus = Corpus::empty(label);
        let sentinel = corpus.root.join(dogtag::vault::SENTINEL_DIRECTORY);
        fs::create_dir_all(&sentinel).expect("a sentinel directory");
        fs::write(sentinel.join(dogtag::vault::CONTRACT_FILE_NAME), text)
            .expect("a committed contract");
        corpus
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use dogtag::installation::load_installation;

    /// A corpus that could not be copied names the path it was copying: a pair
    /// that was never set up must not read as a pair that failed an assertion.
    #[test]
    fn copying_a_corpus_that_is_not_there_names_the_source() {
        let missing = Path::new("/dogtag-conformance-no-such-directory/corpus");
        let detail = Corpus::copy_of(missing, "missing-source")
            .expect_err("a corpus that is not there cannot be copied");
        assert!(
            detail.contains("copying the corpus at"),
            "the failure says what it was doing: {detail}"
        );
        assert!(
            detail.contains("no-such-directory"),
            "the failure names the source: {detail}"
        );
    }

    /// A directory holding no contract is not a vault root, and the refusal
    /// carries the diagnostic the SDK gave rather than only the fact of it.
    #[test]
    fn a_corpus_holding_no_contract_is_not_a_vault_root() {
        let corpus = Corpus::empty("corpus-without-a-contract");
        let detail = corpus
            .vault_root()
            .expect_err("a directory with no contract is not a vault root");
        assert!(
            detail.contains("the copied corpus is not a vault root"),
            "the failure says what is wrong: {detail}"
        );
        assert!(
            detail.contains("discovery.not-a-vault-root"),
            "the failure carries the identifier: {detail}"
        );
    }

    /// A committed contract that reports anything at all is refused, and the
    /// refusal repeats what it reported.
    #[test]
    fn a_contract_that_reports_anything_is_not_a_clean_contract() {
        let corpus = Corpus::holding("corpus-unusable-contract", "contract_version = 99\n");
        let detail = corpus
            .clean_contract()
            .expect_err("a contract outside the supported range is not clean");
        assert!(
            detail.contains("the committed contract"),
            "the failure names the subject: {detail}"
        );
        assert!(
            detail.contains("compat.contract-too-new"),
            "the failure carries what was reported: {detail}"
        );
    }

    /// A transformation that changed no bytes would make its derived case
    /// vacuous, so the derived copy refuses to exist rather than testing
    /// nothing while reporting green.
    #[test]
    fn a_transformation_that_changes_nothing_is_refused() {
        let corpus = Corpus::holding("corpus-identity-transform", NO_AXIS);
        let detail = corpus
            .derived("identity", |text| Ok(text.to_owned()))
            .expect_err("a transformation that changes nothing is refused");
        assert!(
            detail.contains("byte-identical"),
            "the failure says what happened: {detail}"
        );
        assert!(
            detail.contains("would test nothing"),
            "the failure says why it matters: {detail}"
        );
    }

    /// A record that is not there has not loaded, and the assertion names the
    /// state it is in rather than only that it is not the one wanted.
    #[test]
    fn require_record_loaded_names_the_state_of_a_record_that_did_not_load() {
        let corpus = Corpus::empty("corpus-absent-record");
        let installation = load_installation(&corpus.tree().join(NO_RECORD));
        let detail = require_record_loaded(&installation)
            .expect_err("a record that is not there has not loaded");
        assert!(
            detail.contains("the installation record must load"),
            "the failure says what was wanted: {detail}"
        );
        assert!(
            detail.contains("absent"),
            "the failure names the state: {detail}"
        );
    }
}
