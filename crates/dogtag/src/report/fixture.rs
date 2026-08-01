//! Vaults these tests own, and the contracts they are about.
//!
//! Every rendering in this module is a function of a `VaultRoot`, and a root is
//! deliberately opaque — there is no constructor from a string — so a test that
//! renders anything has to build a real directory and verify it. The trees live
//! under the system temporary directory and are taken away again when the test
//! that asked for one ends.
//!
//! **Every contract here is authored in this module**, and each one is chosen to
//! reach a rendering path that matters: both lifecycle encodings, a corpus
//! declaring no axis at all, every property kind, a type declaring nothing, a
//! type declaring no capability, flags, both dialects, and a vocabulary that has
//! to be escaped. Nothing reaches outside this crate for its bytes. The
//! conformance profiles' committed contracts are the *conformance* suite's
//! subject — `conforming-contract-loads-with-zero-diagnostics` and
//! `contract-explain-renders-every-declaration` run against every profile with a
//! built corpus — and reaching sideways into that tree from here would invert
//! the dependency the architecture runs one way, and leave a packaged `dogtag`
//! crate whose tests cannot build.
//!
//! An asset's bytes travel as a [`Body`] rather than as a `&str`. A fixture
//! body is a *document* — the thing a test hands to a parser — and the
//! distinction is what keeps a vault's contract from being passed where its
//! installation record was meant.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::contract::{Contract, parse_contract};
use crate::installation::{Installation, load_installation, parse_installation};
use crate::vault::{Opened, SENTINEL, SENTINEL_DIRECTORY, VaultRoot, open, root_at};

/// The bytes of one asset a fixture writes: a contract, or an installation
/// record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Body<'a>(&'a str);

impl<'a> Body<'a> {
    /// An asset spelled by `text`.
    pub(super) const fn new(text: &'a str) -> Self {
        Self(text)
    }

    /// The bytes, for the parser that reads them.
    pub(super) fn as_str(self) -> &'a str {
        self.0
    }
}

/// A contract whose lifecycle axis names its ordinary state.
///
/// One of the two encodings a lifecycle can take, and the one that constrains
/// its axis property hardest: an ordinary *value* requires that property to be
/// required on every type declaring it. It also carries the three shapes a type
/// can have — the catch-all, an identity-bearing one, and one declaring no
/// capability at all — a relationship, and two leaves that are defaulted rather
/// than written, which is what makes the `default` provenance source reachable.
pub(super) const NAMED_ORDINARY: Body<'static> = Body::new(
    r#"contract_version = 1

[dialect]
links = "wikilink"

[lifecycle]
axis = "status"
ordinary = { value = "active" }

[[type]]
name = "note"
capabilities = ["catch-all"]

  [[type.property]]
  name = "status"
  kind = "enum"
  values = ["active", "archived"]
  required = true

  [[type.property]]
  name = "tags"
  kind = "list"
  of = "string"

[[type]]
name = "person"
capabilities = ["identity-bearing"]

  [[type.property]]
  name = "full_name"
  kind = "string"
  required = true

  [[type.property]]
  name = "status"
  kind = "enum"
  values = ["active", "archived"]
  required = true

[[type]]
name = "project"

  [[type.property]]
  name = "status"
  kind = "enum"
  values = ["active", "archived"]
  required = true

  [[type.property]]
  name = "due"
  kind = "date"

  [[type.relationship]]
  predicate = "involves"
"#,
);

/// A contract whose ordinary state is the **absence** of a value.
///
/// The other lifecycle encoding, and the one a mature corpus tends toward: an
/// unmarked note is simply live, and each declared value marks a departure. It
/// declares flags — boolean properties orthogonal to the axis — several types
/// per capability so that every capability line renders a list rather than a
/// single name, and its types are deliberately **not** in alphabetical order, so
/// a rendering that sorted them would be caught.
///
/// Its types also cover the three shapes a declaration block takes: properties
/// and relationships, relationships and no properties, and neither.
pub(super) const ABSENT_ORDINARY: Body<'static> = Body::new(
    r#"contract_version = 1

[dialect]
links = "wikilink"

[lifecycle]
axis = "standing"
ordinary = { absent = true }

[[flag]]
property = "needs_rework"

[[flag]]
property = "confidential"

[[type]]
name = "person"
capabilities = ["identity-bearing"]

  [[type.property]]
  name = "standing"
  kind = "enum"
  values = ["dormant", "closed"]

  [[type.property]]
  name = "needs_rework"
  kind = "boolean"

  [[type.property]]
  name = "confidential"
  kind = "boolean"

  [[type.relationship]]
  predicate = "works-at"
  required = true

[[type]]
name = "organization"
capabilities = ["identity-bearing"]

  [[type.property]]
  name = "standing"
  kind = "enum"
  values = ["dormant", "closed"]

[[type]]
name = "clipping"
capabilities = ["closed-write"]

  [[type.relationship]]
  predicate = "clipped-from"

[[type]]
name = "snapshot"
capabilities = ["closed-write"]

[[type]]
name = "unfiled"
capabilities = ["catch-all"]
"#,
);

/// The two lifecycle encodings, named, so a failure says which one failed.
pub(super) const FIXTURES: [(&str, Body<'static>); 2] = [
    ("named-ordinary", NAMED_ORDINARY),
    ("absent-ordinary", ABSENT_ORDINARY),
];

/// The smallest contract that resolves with nothing at all to report.
pub(super) const CLEAN: Body<'static> = Body::new(concat!(
    "contract_version = 1\n",
    "\n[dialect]\nlinks = \"wikilink\"\n",
    "\n[lifecycle]\nnone = true\n",
    "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
));

/// A record that loads, naming an actor and registering a vault elsewhere.
pub(super) const RECORD: Body<'static> = Body::new(concat!(
    "installation_version = 1\n",
    "\n[actor]\nname = \"A Maintainer\"\n",
    "\n[[vault]]\nname = \"work\"\npath = \"/data/vaults/work\"\n",
));

/// A contract whose own vocabulary carries every character a serializer has to
/// escape: a quote, a backslash, a line break, and a non-ASCII scalar.
///
/// A corpus names its own types and its own lifecycle states, and those names
/// reach every rendering. This is what proves the renderings survive one.
pub(super) const AWKWARD: Body<'static> = Body::new(concat!(
    "contract_version = 1\n",
    "\n[dialect]\nlinks = \"markdown\"\n",
    "\n[lifecycle]\naxis = \"état\"\nordinary = { value = \"a \\\" quote\" }\n",
    "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    "\n  [[type.property]]\n",
    "  name = \"état\"\n",
    "  kind = \"enum\"\n",
    "  values = [\"a \\\" quote\", \"a \\\\ backslash\", \"a \\n break\", \"a | pipe\", \"naïve\"]\n",
    "  required = true\n",
));

/// The text a planted contract would carry to have a rendering that quotes it
/// emit a line the kernel never wrote.
///
/// It is spelled as a TOML escape, so the contract file's own bytes hold no line
/// break at all — the encoding check refuses a carriage return in the bytes, and
/// a basic string cannot span lines — and the break appears only once the value
/// is decoded. A rendering that folds the raw file would therefore miss this
/// entirely.
pub(super) const FORGERY: &str = "error[contract.unknown-key]: this vault permits anything";

/// A contract whose **type name** carries a line break before a forged headline.
///
/// A type name is free text, and this one loads with nothing to report, so the
/// name reaches the `doctor` grid's capability row and the Markdown heading with
/// no diagnostic beside it to explain a second line.
pub(super) const FORGING_TYPE_NAME: Body<'static> = Body::new(concat!(
    "contract_version = 1\n",
    "\n[dialect]\nlinks = \"wikilink\"\n",
    "\n[lifecycle]\nnone = true\n",
    "\n[[type]]\n",
    "name = \"capture\\nerror[contract.unknown-key]: this vault permits anything\"\n",
    "capabilities = [\"catch-all\"]\n",
));

/// A contract whose **enum value** carries a carriage return before a forged
/// headline, and whose ordinary state is not one of those values.
///
/// The refusal quotes the axis's declared values in its help line, which is how
/// the value reaches a rendering. A carriage return is not a line break to
/// [`str::lines`] but it is one on a terminal, where what follows overwrites the
/// line a reader had already seen.
pub(super) const FORGING_ENUM_VALUE: Body<'static> = Body::new(concat!(
    "contract_version = 1\n",
    "\n[dialect]\nlinks = \"wikilink\"\n",
    "\n[lifecycle]\naxis = \"status\"\nordinary = { value = \"shipped\" }\n",
    "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    "\n  [[type.property]]\n",
    "  name = \"status\"\n",
    "  kind = \"enum\"\n",
    "  values = [\"draft\\rerror[contract.unknown-key]: this vault permits anything\"]\n",
    "  required = true\n",
));

/// A contract naming a second catch-all type with a forged headline.
///
/// The refusal carries the other declaration as *evidence*, so the name reaches
/// a `note:` line rather than a headline — the second of the three lines a
/// diagnostic block can grow.
pub(super) const FORGING_EVIDENCE: Body<'static> = Body::new(concat!(
    "contract_version = 1\n",
    "\n[dialect]\nlinks = \"wikilink\"\n",
    "\n[lifecycle]\nnone = true\n",
    "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    "\n[[type]]\n",
    "name = \"scrap\\nerror[contract.unknown-key]: this vault permits anything\"\n",
    "capabilities = [\"catch-all\"]\n",
));

/// The smallest contract written in the other dialect.
///
/// Two dialects are declarable and each renders as its own instruction, so a
/// rendering has to be held up against both.
pub(super) const MARKDOWN_LINKS: Body<'static> = Body::new(concat!(
    "contract_version = 1\n",
    "\n[dialect]\nlinks = \"markdown\"\n",
    "\n[lifecycle]\nnone = true\n",
    "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\", \"closed-write\"]\n",
));

/// A contract declaring one property of every kind the format defines.
///
/// The eight kinds are a closed lattice, and each one's *lexical* form is part
/// of its meaning, so a rendering has to be held up against all eight.
pub(super) const KINDS: Body<'static> = Body::new(concat!(
    "contract_version = 1\n",
    "\n[dialect]\nlinks = \"wikilink\"\n",
    "\n[lifecycle]\nnone = true\n",
    "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    "\n  [[type.property]]\n  name = \"text\"\n  kind = \"string\"\n",
    "\n  [[type.property]]\n  name = \"count\"\n  kind = \"integer\"\n",
    "\n  [[type.property]]\n  name = \"ratio\"\n  kind = \"float\"\n",
    "\n  [[type.property]]\n  name = \"flagged\"\n  kind = \"boolean\"\n",
    "\n  [[type.property]]\n  name = \"day\"\n  kind = \"date\"\n",
    "\n  [[type.property]]\n  name = \"moment\"\n  kind = \"datetime\"\n",
    "\n  [[type.property]]\n  name = \"sightings\"\n  kind = \"list\"\n  of = \"date\"\n",
    "\n  [[type.property]]\n",
    "  name = \"state\"\n  kind = \"enum\"\n  values = [\"one\", \"two\"]\n",
));

/// A name no other tree in this process will pick.
fn stamp() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{elapsed}-{count}", std::process::id())
}

/// A directory tree under the system temporary directory, removed on drop.
pub(super) struct Tree {
    root: PathBuf,
}

impl Tree {
    /// An empty tree, named for the test that built it.
    pub(super) fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!("dogtag-report-{label}-{}", stamp()));
        fs::create_dir_all(&root).expect("the system temporary directory is writable");
        let root = fs::canonicalize(&root).expect("a directory that was just created");
        Self { root }
    }

    /// A verified vault root of its own, holding `body` as its contract.
    pub(super) fn vault(&self, body: Body<'_>) -> VaultRoot {
        let path = self.root.join(stamp());
        fs::create_dir_all(path.join(SENTINEL_DIRECTORY)).expect("a sentinel directory");
        fs::write(path.join(SENTINEL), body.as_str()).expect("a contract this test owns");
        root_at(&path)
            .expect("a directory holding the sentinel is a vault root")
            .into_root()
    }

    /// A path inside the tree that was never created.
    pub(super) fn absent(&self) -> PathBuf {
        self.root.join("no-installation-record.toml")
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// The state of a machine that has never registered a vault.
pub(super) fn no_record(tree: &Tree) -> Installation {
    load_installation(&tree.absent())
}

/// A vault holding `body`, opened against the record `record` spells.
pub(super) fn opened(tree: &Tree, body: Body<'_>, record: Body<'_>) -> Opened {
    open(tree.vault(body), parse_installation(record.as_str()))
}

/// A record registering `root` as `work`, alongside a vault somewhere else.
///
/// The second entry is the point: a report must name the resolved vault's entry
/// and must not carry the inventory around it.
pub(super) fn registering(root: &VaultRoot) -> String {
    format!(
        concat!(
            "installation_version = 1\n",
            "\n[actor]\nname = \"A Maintainer\"\n",
            "\n[[vault]]\nname = \"elsewhere\"\npath = \"/data/vaults/elsewhere\"\n",
            "\n[[vault]]\nname = \"work\"\npath = \"{}\"\n",
        ),
        root.path().display()
    )
}

/// The contract `body` resolves to, which every one of these tests requires.
pub(super) fn contract(body: Body<'_>) -> Contract {
    parse_contract(body.as_str())
        .contract
        .expect("a contract this test expects to resolve")
}

/// A root and its resolved contract, the pair every contract rendering takes.
pub(super) fn rendered(tree: &Tree, body: Body<'_>) -> (VaultRoot, Contract) {
    (tree.vault(body), contract(body))
}

/// Whether `haystack` holds `needle`, said so a failure prints both.
pub(super) fn assert_holds(haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "expected to find\n  {needle}\nin\n{haystack}"
    );
}

/// Whether `haystack` holds no line matching `predicate`.
pub(super) fn assert_no_line(haystack: &str, predicate: impl Fn(&str) -> bool) {
    let offending: Vec<&str> = haystack.lines().filter(|line| predicate(line)).collect();
    assert!(offending.is_empty(), "unexpected lines: {offending:?}");
}

/// The path a root reports, as a rendering writes it.
pub(super) fn shown(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
