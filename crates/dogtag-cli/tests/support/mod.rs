//! Scaffolding shared by the CLI's integration tests.
//!
//! Every test runs the built binary as a child process and controls that
//! child's world completely: its current directory, its home directory, the
//! configuration directory the installation record is read from, and whether
//! `DOGTAG_VAULT` and `NO_COLOR` are set. Nothing is set in the *test*
//! process, so the suite stays hermetic and runs in parallel — a test that
//! mutated the process environment would change what a concurrent test
//! resolved.
//!
//! Trees are built under the system temporary directory and taken away again
//! when the test ends. Each created directory is restricted to its owner:
//! otherwise a developer whose umask grants group write would see the root
//! trust warning fire and read it as a failure of the thing under test.

// Each test binary compiles this module separately and uses the part of it
// that it needs, so the unused remainder is expected rather than dead.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, process};

/// The committed conformance fixtures, compiled in from the repository.
///
/// They are included rather than read at run time so that a fixture which
/// moved fails the build instead of quietly failing a test.
pub const STARTER: &str =
    include_str!("../../../../conformance/profiles/starter/corpus/.dogtag/contract.toml");

/// The dense fixture: a realistic taxonomy, with flags and relationships.
pub const DENSE: &str =
    include_str!("../../../../conformance/profiles/dense/corpus/.dogtag/contract.toml");

/// A contract whose version this release does not read.
pub const TOO_NEW: &str = "contract_version = 4\n";

/// Distinguishes trees built inside the same process and second.
static SEQUENCE: AtomicU32 = AtomicU32::new(0);

/// A directory tree a test owns outright.
pub struct Tree {
    root: PathBuf,
}

impl Tree {
    /// A tree holding a home directory and a configuration directory.
    ///
    /// The root is canonical, so a temporary directory reached through a
    /// symlink — `/tmp` is one on macOS — cannot make a run emit the
    /// resolved-through-symlink diagnostic that a test did not ask for.
    pub fn new(label: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("the clock is after the Unix epoch")
            .as_nanos();
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = env::temp_dir().join(format!(
            "dogtag-cli-{label}-{}-{unique}-{sequence}",
            process::id()
        ));
        fs::create_dir_all(&root).expect("a tree this test owns");
        let tree = Self {
            root: fs::canonicalize(&root).expect("a tree this test just created"),
        };
        tree.dir("home/.config");
        tree
    }

    /// The tree's own root.
    pub fn path(&self) -> &Path {
        &self.root
    }

    /// The home directory every run in this tree is given.
    pub fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    /// The configuration directory the installation record is read from.
    pub fn config(&self) -> PathBuf {
        self.home().join(".config")
    }

    /// A directory at `relative`, restricted to its owner.
    pub fn dir(&self, relative: &str) -> PathBuf {
        let path = self.root.join(relative);
        fs::create_dir_all(&path).expect("a directory this test owns");
        self.restrict_to(&path);
        path
    }

    /// A file at `relative`, with the directories above it.
    pub fn write(&self, relative: &str, contents: &str) -> PathBuf {
        let path = self.root.join(relative);
        let parent = path.parent().expect("a file sits in a directory");
        fs::create_dir_all(parent).expect("a directory this test owns");
        self.restrict_to(parent);
        fs::write(&path, contents).expect("a file this test owns");
        path
    }

    /// A vault root at `relative`, holding `contract`.
    pub fn vault(&self, relative: &str, contract: &str) -> PathBuf {
        self.write(&format!("{relative}/.dogtag/contract.toml"), contract);
        self.root.join(relative)
    }

    /// The installation record, at the location a run in this tree reads.
    pub fn record(&self, contents: &str) -> PathBuf {
        self.write("home/.config/dogtag/installation.toml", contents)
    }

    /// Every path in the tree, in a stable order.
    ///
    /// This is what a test compares before and against after, to hold up the
    /// claim that a command wrote nothing anywhere.
    pub fn listing(&self) -> Vec<String> {
        let mut found = Vec::new();
        collect(&self.root, &self.root, &mut found);
        found.sort();
        found
    }

    /// Restricts `path` and everything between it and the root to its owner.
    fn restrict_to(&self, path: &Path) {
        let mut current = path;
        loop {
            restrict(current);
            match current.parent() {
                Some(parent) if parent.starts_with(&self.root) => current = parent,
                _ => return,
            }
        }
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).ok();
    }
}

/// Every path under `directory`, spelled relative to `base`.
fn collect(base: &Path, directory: &Path, found: &mut Vec<String>) {
    let entries = fs::read_dir(directory).expect("a directory this test owns");
    for entry in entries {
        let path = entry.expect("an entry this test owns").path();
        let relative = path.strip_prefix(base).expect("a path inside the tree");
        found.push(relative.to_string_lossy().into_owned());
        if path.is_dir() {
            collect(base, &path, found);
        }
    }
}

/// Takes group and other write away from `path`.
#[cfg(unix)]
fn restrict(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .expect("a directory this test owns");
}

/// A no-op where there is no Unix mode to set.
#[cfg(not(unix))]
fn restrict(_path: &Path) {}

/// An installation record registering `root` under `name`.
pub fn registering(name: &str, root: &Path) -> String {
    format!(
        "installation_version = 1\n\n[actor]\nname = \"A Maintainer\"\n\n[[vault]]\nname = \
         \"{name}\"\npath = \"{}\"\n",
        root.display()
    )
}

/// The `dogtag` binary, with every ambient fact this suite controls set.
///
/// The current directory starts at the home directory, which holds no vault:
/// a test that wants discovery to succeed says where it is running from.
pub fn dogtag(tree: &Tree) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_dogtag"));
    command
        .env("HOME", tree.home())
        .env("XDG_CONFIG_HOME", tree.config())
        .env_remove("DOGTAG_VAULT")
        .env_remove("NO_COLOR")
        .current_dir(tree.home());
    command
}

/// What running the binary produced.
#[derive(Debug)]
pub struct Finished {
    /// The exit code, which is always one of `0`, `1` and `2`.
    pub code: i32,
    /// Everything written to standard output.
    pub stdout: String,
    /// Everything written to standard error.
    pub stderr: String,
}

/// Runs the command to completion.
pub fn run(command: &mut Command) -> Finished {
    let output = command.output().expect("the dogtag binary runs");
    Finished {
        code: output
            .status
            .code()
            .expect("the binary exits rather than being killed by a signal"),
        stdout: String::from_utf8(output.stdout).expect("standard output is UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("standard error is UTF-8"),
    }
}

impl Finished {
    /// Whether either stream carries an escape sequence.
    pub fn has_escapes(&self) -> bool {
        self.stdout.contains('\u{1b}') || self.stderr.contains('\u{1b}')
    }
}

/// Proves that no ancestor of `start` holds a vault sentinel.
///
/// Discovery walks to the filesystem root with no boundary, so "there is no
/// vault above here" is otherwise a property of the *machine* rather than of
/// the fixture. A test asserting `discovery.no-vault-found` calls this first
/// and fails loudly, with the offending directory named, rather than passing
/// or silently skipping.
pub fn assert_no_vault_above(start: &Path) {
    for directory in start.ancestors() {
        assert!(
            !directory.join(".dogtag/contract.toml").exists(),
            "`{}` holds a vault sentinel, so `no vault found` is not this fixture's answer to \
             assert — move the temporary directory or remove that contract",
            directory.display()
        );
    }
}

/// A delimiter scan over what should be one JSON document.
#[derive(Default)]
struct Scan {
    depth: i32,
    string: bool,
    escaped: bool,
}

impl Scan {
    /// Consumes one character.
    fn step(&mut self, character: char) {
        match (self.escaped, self.string) {
            (true, _) => self.escaped = false,
            (false, true) => self.inside(character),
            (false, false) => self.outside(character),
        }
    }

    /// A character inside a string, where only the quote and the escape count.
    fn inside(&mut self, character: char) {
        match character {
            '\\' => self.escaped = true,
            '"' => self.string = false,
            _ => {}
        }
    }

    /// A character outside a string, where the delimiters count.
    fn outside(&mut self, character: char) {
        match character {
            '"' => self.string = true,
            '{' | '[' => self.depth += 1,
            '}' | ']' => self.depth -= 1,
            _ => {}
        }
    }
}

/// Whether `text` is one balanced, newline-terminated JSON document.
///
/// This test target takes no dependency beyond the standard library, so this
/// is a delimiter scan rather than a parse: it tracks nesting outside strings
/// and honours escapes, which is enough to catch a truncated document, an
/// unbalanced one, or anything printed beside it on the same stream.
pub fn well_formed_json(text: &str) -> bool {
    let mut scan = Scan::default();
    for character in text.chars() {
        scan.step(character);
    }
    scan.depth == 0 && !scan.string && text.starts_with('{') && text.ends_with("}\n")
}
