//! Which files under a vault root are notes.
//!
//! **A note is any file with the `.md` extension under the vault root, found by
//! a walk that skips every directory whose name begins with `.` and does not
//! follow symlinks.** There is no configurable ignore list, and no contract key
//! that could grow into one.
//!
//! The dot rule is one rule doing three jobs: it excludes `.dogtag/`, because
//! the contract is not a note; it excludes `.git/`, `.obsidian/`, and every
//! other tool's private directory, without the format learning any tool's name;
//! and it costs one sentence to state. Its price is stated rather than
//! discovered — a user who keeps notes under a dotted directory loses them, and
//! loses them silently.
//!
//! Symlinks are not followed because **identity is the path**. A note reachable
//! through a link would hold two identities, every resolution answer would
//! depend on which one the walk met first, and a cycle would need detection
//! machinery whose only job is coping with a shape the identity model already
//! rules out. The rule is applied to every entry rather than only to
//! directories: a symlinked `.md` file is the same two-identities problem in
//! miniature, and the record's reason for refusing to follow links reaches it
//! word for word.
//!
//! Non-Markdown files are ignored silently — they are not notes and their
//! presence is not a finding.
//!
//! **Enumeration order never reaches output.** Entries are sorted by name
//! before the walk descends, so what the filesystem happened to hand back is
//! unobservable — including through the stable sort that carries diagnostics
//! sharing a location and an identifier, which the total order leaves tied.
//!
//! Sorting each directory's entries is not on its own the path order, and the
//! notes are sorted again before they are answered. A directory sorts by its
//! own name, so the walk meets `projects/` before `projects.md`; the paths sort
//! the other way, because `.` precedes `/`. Answering walk order would put a
//! folder note after the folder's notes while every diagnostic carrying those
//! same paths came out the other way round — two accessors of one traversal
//! contradicting each other.

use std::ffi::OsString;
use std::fs::{self, FileType};
use std::io;
use std::path::Path;

use crate::diagnostic::{
    Diagnostic, DiagnosticList, FileRef, KernelDiagnostic, Location, VaultPath,
};
use crate::vault::VaultRoot;

/// The extension that makes a file a note.
const NOTE_EXTENSION: &str = "md";

/// Every note under a vault root, and everything the walk could not read.
///
/// A directory that cannot be enumerated is a diagnostic against that
/// directory, never an abort: one unreadable corner must not make the corpus
/// unreadable, which is the same rule the record states for one unreadable
/// note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Traversal {
    notes: Vec<VaultPath>,
    diagnostics: Vec<Diagnostic>,
}

impl Traversal {
    /// Every note found, in vault-relative path order.
    pub fn notes(&self) -> &[VaultPath] {
        &self.notes
    }

    /// Everything the walk could not read, in the deterministic total order.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Finds every note under `root`.
///
/// This is a pure function of its argument: it consults no environment
/// variable, no current directory, and no process-global state, and it reads no
/// file — only directories.
pub fn traverse(root: &VaultRoot) -> Traversal {
    let mut walk = Walk {
        root,
        notes: Vec::new(),
        diagnostics: DiagnosticList::new(),
    };
    walk.directory(root.path(), "");
    let mut notes = walk.notes;
    notes.sort_unstable();
    Traversal {
        notes,
        diagnostics: walk.diagnostics.sorted(),
    }
}

/// One walk in progress.
struct Walk<'a> {
    root: &'a VaultRoot,
    notes: Vec<VaultPath>,
    diagnostics: DiagnosticList,
}

impl Walk<'_> {
    /// Descends into one directory, named by its vault-relative spelling.
    fn directory(&mut self, path: &Path, relative: &str) {
        match entries(path) {
            Ok(found) => {
                for entry in found {
                    match entry.name.to_str() {
                        Some(name) => {
                            self.entry(&path.join(name), &join(relative, name), entry.kind);
                        }
                        None => self.nameless(relative, &entry),
                    }
                }
            }
            Err(error) => self.unreadable(path, relative, &error),
        }
    }

    /// Classifies one entry: a directory to descend into, a note, or neither.
    fn entry(&mut self, path: &Path, relative: &str, kind: FileType) {
        if kind.is_symlink() {
            return;
        }
        if kind.is_dir() {
            if !hidden(relative) {
                self.directory(path, relative);
            }
        } else if is_note(path) {
            // The `None` a path outside the root would give cannot arise: every
            // path here was joined onto the root, out of names already known to
            // be valid UTF-8. Extending over the option says so without a
            // branch that could never be taken.
            self.notes.extend(self.root.relative(path));
        }
    }

    /// A directory that could not be enumerated, located at its vault-relative
    /// path exactly as an unreadable note is. Never an abort: one unreadable
    /// corner must not make the corpus unreadable.
    fn unreadable(&mut self, path: &Path, relative: &str, error: &io::Error) {
        let at = self
            .root
            .relative(path)
            .map(|directory| Location::whole_file(FileRef::InVault(directory)));
        self.diagnostics.push(Diagnostic {
            location: at,
            ..Diagnostic::kernel(
                KernelDiagnostic::NoteUnreadable,
                format!(
                    "the directory `{}` could not be read: {error}",
                    spell(relative)
                ),
            )
        });
    }

    /// An entry whose name is not valid UTF-8, which the vault-relative
    /// spelling cannot hold.
    ///
    /// Reading on through a lossy respelling would misreport the failure as
    /// the file's absence — so the walk stops at the honest fact instead, and
    /// only for an entry it would otherwise have read: a symlink, a hidden
    /// directory, and a file that is not a note are passed over exactly as
    /// their well-named counterparts are. The diagnostic carries no structured
    /// location, because the one thing a location holds is the spelling this
    /// name does not have.
    fn nameless(&mut self, relative: &str, entry: &Entry) {
        if !read_despite_name(entry) {
            return;
        }
        let name = join(relative, &entry.name.to_string_lossy());
        self.diagnostics.push(Diagnostic::kernel(
            KernelDiagnostic::NoteUnreadable,
            format!("`{name}` could not be read: its name is not valid UTF-8"),
        ));
    }
}

/// Whether a badly named entry would have been read under a well-formed name.
///
/// The membership rules run on the name the entry actually has: a symlink is
/// never followed, a hidden directory is invisible wholesale, and only the
/// note extension makes a file a note.
fn read_despite_name(entry: &Entry) -> bool {
    if entry.kind.is_symlink() {
        return false;
    }
    if entry.kind.is_dir() {
        return !entry.name.to_string_lossy().starts_with('.');
    }
    is_note(Path::new(&entry.name))
}

/// One directory entry, with the type the listing already knew.
struct Entry {
    name: OsString,
    kind: FileType,
}

/// Every entry of one directory, sorted by name.
///
/// The sort is what the notes' own sort cannot do for the walk's diagnostics:
/// two entries whose names are not valid UTF-8 share an identifier and carry no
/// location, so the total order leaves them tied and a stable sort keeps the
/// order they were emitted in — which is this one.
///
/// A single entry that cannot be classified fails the whole listing, which is
/// what keeps the walk's error handling to one place: a directory is either
/// enumerated or reported, and there is no third state in which some of its
/// contents are notes and the rest are unexplained.
fn entries(path: &Path) -> io::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        entries.push(Entry {
            kind: entry.file_type()?,
            name: entry.file_name(),
        });
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(entries)
}

/// Whether a vault-relative path's last segment begins with a `.`.
fn hidden(relative: &str) -> bool {
    let name = match relative.rsplit_once('/') {
        Some((_, name)) => name,
        None => relative,
    };
    name.starts_with('.')
}

/// Whether a path's extension makes it a note.
///
/// The comparison is exact, so `NOTE.MD` is not a note. The rule names one
/// extension rather than a set of spellings, and a case-folding rule would
/// answer differently on two filesystems for the same bytes.
fn is_note(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == NOTE_EXTENSION)
}

/// A vault-relative path with one more segment beneath it.
fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}/{name}")
    }
}

/// How a message names a directory inside the vault.
///
/// The root's vault-relative spelling is the empty string, which would render
/// as nothing at all, so it spells itself `.` — and no machine path reaches
/// output either way.
fn spell(relative: &str) -> &str {
    if relative.is_empty() { "." } else { relative }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::tree::Tree;
    use std::path::PathBuf;

    /// A vault root of its own, with `relative` written into it as note bodies.
    fn vault(tree: &Tree, name: &str, notes: &[&str]) -> VaultRoot {
        let path = tree.vault(name);
        for note in notes {
            let file = path.join(note);
            let parent = file.parent().expect("a note under the root has a parent");
            fs::create_dir_all(parent).expect("a directory this test owns");
            fs::write(&file, "body\n").expect("a note this test owns");
        }
        VaultRoot::new(path)
    }

    fn found(traversal: &Traversal) -> Vec<&str> {
        traversal.notes().iter().map(VaultPath::as_str).collect()
    }

    fn ids(traversal: &Traversal) -> Vec<&str> {
        traversal
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect()
    }

    #[test]
    fn every_markdown_file_under_the_root_is_a_note_whatever_folder_it_sits_in() {
        let tree = Tree::new("traverse-members");
        let root = vault(
            &tree,
            "members",
            &["top.md", "people/ada.md", "people/deep/nested.md"],
        );
        let traversal = traverse(&root);
        assert_eq!(
            found(&traversal),
            ["people/ada.md", "people/deep/nested.md", "top.md"]
        );
        assert!(traversal.diagnostics().is_empty());
    }

    #[test]
    fn a_file_that_is_not_markdown_is_ignored_without_a_finding() {
        let tree = Tree::new("traverse-others");
        let root = vault(
            &tree,
            "others",
            &["note.md", "image.png", "plain.txt", "NOTE.MD", "makefile"],
        );
        let traversal = traverse(&root);
        assert_eq!(found(&traversal), ["note.md"]);
        assert!(traversal.diagnostics().is_empty());
    }

    #[test]
    fn a_dotted_directory_is_invisible_wholesale_and_says_nothing_about_it() {
        let tree = Tree::new("traverse-dotted");
        let root = vault(
            &tree,
            "dotted",
            &[
                "kept.md",
                ".dogtag/not-a-note.md",
                ".git/objects/loose.md",
                ".obsidian/plugin.md",
            ],
        );
        let traversal = traverse(&root);
        assert_eq!(found(&traversal), ["kept.md"]);
        assert!(traversal.diagnostics().is_empty());
    }

    #[test]
    fn a_dotted_file_is_still_a_note_because_the_rule_names_directories() {
        let tree = Tree::new("traverse-dotfile");
        let root = vault(&tree, "dotfile", &[".hidden.md"]);
        assert_eq!(found(&traverse(&root)), [".hidden.md"]);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_is_neither_a_note_nor_a_directory_to_descend_into() {
        let tree = Tree::new("traverse-symlink");
        let outside = tree.dir("outside");
        fs::write(outside.join("stray.md"), "body\n").expect("a note this test owns");
        let root = vault(&tree, "linked", &["real.md"]);
        std::os::unix::fs::symlink(&outside, root.path().join("elsewhere"))
            .expect("a symlink this test owns");
        std::os::unix::fs::symlink(outside.join("stray.md"), root.path().join("alias.md"))
            .expect("a symlink this test owns");
        assert_eq!(found(&traverse(&root)), ["real.md"]);
    }

    #[test]
    fn a_directory_that_cannot_be_read_is_reported_and_never_aborts_the_walk() {
        let tree = Tree::new("traverse-unreadable");
        let root = VaultRoot::new(tree.absent("never-created"));
        let traversal = traverse(&root);
        assert!(traversal.notes().is_empty());
        assert_eq!(ids(&traversal), ["note.unreadable"]);
        let reported = &traversal.diagnostics()[0];
        assert!(reported.message.contains("the directory `.`"));
        assert_eq!(
            reported.location,
            Some(Location::whole_file(FileRef::InVault(VaultPath::kernel(
                ""
            )))),
            "the vault-relative directory path is a structured location, exactly as an \
             unreadable note's is; only a machine path never becomes one"
        );
    }

    // Linux only, not unix: APFS refuses to create a name that is not valid UTF-8
    // (EILSEQ), so on macOS these tests would fail at their own setup.
    #[cfg(target_os = "linux")]
    #[test]
    fn an_entry_whose_name_is_not_utf_8_is_reported_honestly_rather_than_read_lossily() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let tree = Tree::new("traverse-nameless");
        let root = vault(&tree, "nameless", &["kept.md"]);
        fs::write(
            root.path().join(OsStr::from_bytes(b"bad\xffname.md")),
            "body\n",
        )
        .expect("a file this test owns");
        fs::create_dir(root.path().join(OsStr::from_bytes(b"dir\xff")))
            .expect("a directory this test owns");
        let traversal = traverse(&root);
        assert_eq!(found(&traversal), ["kept.md"]);
        assert_eq!(ids(&traversal), ["note.unreadable", "note.unreadable"]);
        let messages: Vec<&str> = traversal
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect();
        for message in &messages {
            assert!(
                message.contains("its name is not valid UTF-8"),
                "the failure is the name, never a misleading `No such file or directory`: \
                 {message}"
            );
            assert!(
                !message.contains("No such file"),
                "nothing read through a lossy respelling: {message}"
            );
        }
        assert!(messages[0].contains("bad\u{fffd}name.md"), "{messages:?}");
        assert!(messages[1].contains("dir\u{fffd}"), "{messages:?}");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_badly_named_entry_the_walk_would_not_have_read_stays_as_silent_as_ever() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let tree = Tree::new("traverse-nameless-silent");
        let root = vault(&tree, "nameless-silent", &["kept.md"]);
        fs::write(root.path().join(OsStr::from_bytes(b"image\xff.png")), [])
            .expect("a file this test owns");
        fs::create_dir(root.path().join(OsStr::from_bytes(b".hidden\xff")))
            .expect("a directory this test owns");
        std::os::unix::fs::symlink(
            root.path().join("kept.md"),
            root.path().join(OsStr::from_bytes(b"alias\xff.md")),
        )
        .expect("a symlink this test owns");
        let traversal = traverse(&root);
        assert_eq!(found(&traversal), ["kept.md"]);
        let reported = ids(&traversal);
        assert!(
            reported.is_empty(),
            "passed over whatever their names: {reported:?}"
        );
    }

    #[test]
    fn notes_come_out_in_path_order_whatever_order_the_filesystem_gave() {
        let tree = Tree::new("traverse-order");
        let root = vault(&tree, "order", &["z.md", "a.md", "m/b.md", "m/a.md"]);
        let traversal = traverse(&root);
        let mut sorted = found(&traversal);
        sorted.sort_unstable();
        assert_eq!(found(&traversal), sorted);
        assert_eq!(sorted, ["a.md", "m/a.md", "m/b.md", "z.md"]);
    }

    #[test]
    fn a_folder_note_beside_its_own_folder_still_comes_out_in_path_order() {
        // The one shape where sorting each directory's entries is not the path
        // order: the walk meets `projects` before `projects.md`, because a
        // directory sorts by its own name, while the joined paths sort the
        // other way because `.` precedes `/`. Answering walk order here would
        // disagree with the order the diagnostics carrying those paths take.
        let tree = Tree::new("traverse-folder-note");
        let root = vault(
            &tree,
            "folder-note",
            &["projects/beta.md", "projects/alpha.md", "projects.md"],
        );
        let traversal = traverse(&root);
        assert_eq!(
            found(&traversal),
            ["projects.md", "projects/alpha.md", "projects/beta.md"]
        );
    }

    #[test]
    fn a_vault_holding_no_note_at_all_finds_nothing_and_reports_nothing() {
        let tree = Tree::new("traverse-empty");
        let root = vault(&tree, "empty", &[]);
        let traversal = traverse(&root);
        assert!(traversal.notes().is_empty());
        assert!(traversal.diagnostics().is_empty());
    }

    #[test]
    fn a_directory_spells_itself_by_its_vault_relative_path_and_the_root_as_a_dot() {
        assert_eq!(spell(""), ".");
        assert_eq!(spell("people/deep"), "people/deep");
        assert_eq!(join("", "people"), "people");
        assert_eq!(join("people", "ada.md"), "people/ada.md");
    }

    #[test]
    fn the_membership_helpers_answer_for_the_shapes_a_walk_meets() {
        assert!(hidden(".git"));
        assert!(hidden("notes/.trash"));
        assert!(!hidden("notes"));
        assert!(!hidden(""));
        assert!(is_note(Path::new("a.md")));
        assert!(!is_note(Path::new("a.markdown")));
        assert!(!is_note(&PathBuf::from("a")));
    }

    #[test]
    fn a_traversal_clones_compares_and_formats() {
        let tree = Tree::new("traverse-derives");
        let root = vault(&tree, "derives", &["one.md"]);
        let traversal = traverse(&root);
        let copy = traversal.clone();
        assert_eq!(copy, traversal);
        assert_ne!(traversal, traverse(&vault(&tree, "other", &[])));
        assert!(format!("{traversal:?}").contains("one.md"));
    }
}
