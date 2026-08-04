//! The name index, and the one rule every reference resolves by.
//!
//! **Identity is the path.** A note's name — its file name without the `.md`
//! extension — is a per-reference resolution shorthand and nothing more: two
//! notes may legitimately share one, and the reference that meets both is what
//! carries the defect. A corpus is never asked to rename anything before it
//! will be read.
//!
//! The rule, which is one rule for every door — a typed link in frontmatter, an
//! untyped reference in prose, and a reference a caller hands the SDK:
//!
//! - a reference containing no `/` is a **bare name**, and resolves iff exactly
//!   one note in the corpus bears it;
//! - a reference containing a `/` is **path-qualified**, resolved against the
//!   vault root, with `.md` appended when it is absent.
//!
//! Nothing here touches the filesystem. A path-qualified reference is matched
//! against the paths the traversal already found, so `../elsewhere` and an
//! absolute spelling name no note rather than reaching outside the vault — the
//! containment is a property of the lookup rather than a check bolted onto it.
//!
//! # One narrowing the rule states and is worth stating twice
//!
//! The `/` is what picks the half, and the extension is appended only by the
//! path-qualified half. So `engine.md` — no `/` — is a **bare name**, and no
//! note bears the name `engine.md`; the note at `engine.md` is named `engine`.
//! The rule is taken as written rather than made lenient, because stripping an
//! extension off a bare name is a rule the packet does not state and would
//! make `engine` and `engine.md` two spellings of one name forever. It has a
//! visible cost under the `markdown` dialect, whose destinations carry the
//! extension by convention: `[Engine](engine.md)` beside a root-level
//! `engine.md` does not resolve, while `[Engine](engines/analytical.md)` does.

use std::collections::BTreeMap;

use super::model::Note;

/// The extension a note's file carries, and a reference may leave off.
const NOTE_EXTENSION: &str = ".md";

/// Every note in a corpus, by the two things a reference can name.
///
/// Positions rather than paths, because resolution runs while the corpus is
/// being filled in: an index holding borrowed notes could not coexist with
/// writing a resolved target back into one.
pub(crate) struct Index {
    by_name: BTreeMap<String, Vec<usize>>,
    by_path: BTreeMap<String, usize>,
}

/// What a reference named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Resolution {
    /// Exactly one note, by its position in the corpus.
    One(usize),
    /// A bare name several notes bear, in vault-relative path order.
    Ambiguous(Vec<usize>),
    /// Nothing this corpus holds.
    Absent,
}

impl Index {
    /// Indexes `notes`, which the traversal already put in path order.
    pub(crate) fn of(notes: &[Note]) -> Self {
        let mut by_name: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut by_path = BTreeMap::new();
        for (at, note) in notes.iter().enumerate() {
            by_name.entry(note.name().to_owned()).or_default().push(at);
            by_path.insert(note.path().as_str().to_owned(), at);
        }
        Self { by_name, by_path }
    }

    /// The note `reference` names, under the standing resolution rule.
    pub(crate) fn resolve(&self, reference: &str) -> Resolution {
        if reference.contains('/') {
            self.at_path(reference)
        } else {
            self.named(reference)
        }
    }

    /// A path-qualified reference resolves exactly, or not at all.
    fn at_path(&self, reference: &str) -> Resolution {
        self.by_path
            .get(&qualified(reference))
            .map_or(Resolution::Absent, |at| Resolution::One(*at))
    }

    /// A bare name resolves iff exactly one note bears it.
    fn named(&self, name: &str) -> Resolution {
        match self.by_name.get(name).map(Vec::as_slice) {
            Some([only]) => Resolution::One(*only),
            Some(several) => Resolution::Ambiguous(several.to_vec()),
            None => Resolution::Absent,
        }
    }
}

/// A path-qualified reference as a vault-relative path spells it.
fn qualified(reference: &str) -> String {
    if reference.ends_with(NOTE_EXTENSION) {
        reference.to_owned()
    } else {
        format!("{reference}{NOTE_EXTENSION}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::VaultPath;
    use crate::note::model::Binding;

    /// A corpus of notes at these paths, in the path order a traversal gives.
    fn corpus(paths: &[&'static str]) -> Vec<Note> {
        let mut paths = paths.to_vec();
        paths.sort_unstable();
        paths.into_iter().map(note).collect()
    }

    fn note(path: &'static str) -> Note {
        Note {
            path: VaultPath::kernel(path),
            binding: Binding::Unbound { named: None },
            properties: Vec::new(),
            relationships: Vec::new(),
            references: Vec::new(),
            tags: Vec::new(),
            title: None,
            body: String::new(),
        }
    }

    fn resolve(paths: &[&'static str], reference: &str) -> Resolution {
        Index::of(&corpus(paths)).resolve(reference)
    }

    #[test]
    fn a_bare_name_resolves_when_exactly_one_note_bears_it() {
        let corpus = ["people/ada.md", "engines/analytical.md"];
        assert_eq!(resolve(&corpus, "ada"), Resolution::One(1));
        assert_eq!(resolve(&corpus, "analytical"), Resolution::One(0));
    }

    #[test]
    fn a_bare_name_two_notes_bear_names_neither_of_them() {
        // Ambiguity is a defect of the link, not of the corpus: the corpus is
        // read, and the candidates come back so the reference can be repaired.
        let corpus = ["2026/daily.md", "2025/daily.md", "people/ada.md"];
        assert_eq!(resolve(&corpus, "daily"), Resolution::Ambiguous(vec![0, 1]));
    }

    #[test]
    fn a_name_no_note_bears_resolves_to_nothing() {
        assert_eq!(resolve(&["people/ada.md"], "babbage"), Resolution::Absent);
    }

    #[test]
    fn a_path_qualified_reference_resolves_exactly_with_or_without_the_extension() {
        let corpus = ["people/ada.md", "people/babbage.md"];
        let found = (
            resolve(&corpus, "people/ada"),
            resolve(&corpus, "people/ada.md"),
        );
        assert_eq!(found, (Resolution::One(0), Resolution::One(0)));
    }

    #[test]
    fn a_path_that_names_no_note_resolves_to_nothing_however_it_is_spelled() {
        // Containment is a property of the lookup: a reference is matched
        // against the paths the traversal found, so nothing outside the vault
        // is reachable and no check has to say so.
        let corpus = ["people/ada.md"];
        let nothing = [
            resolve(&corpus, "people/babbage"),
            resolve(&corpus, "../outside/ada"),
            resolve(&corpus, "/people/ada.md"),
            resolve(&corpus, "people/"),
        ];
        assert!(
            nothing.iter().all(|found| *found == Resolution::Absent),
            "{nothing:?}"
        );
    }

    #[test]
    fn a_path_qualified_reference_is_never_read_as_a_bare_name() {
        // `people/ada` and `ada` are two references, and only one of them is
        // the shorthand.
        let corpus = ["people/ada.md"];
        let found = (resolve(&corpus, "ada"), resolve(&corpus, "elsewhere/ada"));
        assert_eq!(found, (Resolution::One(0), Resolution::Absent));
    }

    #[test]
    fn a_notes_name_is_its_file_name_without_the_extension_wherever_it_sits() {
        let corpus = ["inbox.md", "a/b/deep.md"];
        let found = (resolve(&corpus, "inbox"), resolve(&corpus, "deep"));
        assert_eq!(found, (Resolution::One(1), Resolution::One(0)));
    }

    #[test]
    fn a_reference_is_qualified_by_appending_the_extension_only_when_it_lacks_one() {
        assert_eq!(qualified("people/ada"), "people/ada.md");
        assert_eq!(qualified("people/ada.md"), "people/ada.md");
        assert_eq!(qualified("people/ada.markdown"), "people/ada.markdown.md");
    }

    #[test]
    fn a_resolution_clones_compares_and_formats() {
        let one = Resolution::One(0);
        assert_eq!(one.clone(), one);
        assert_ne!(one, Resolution::Absent);
        assert!(format!("{:?}", Resolution::Ambiguous(vec![0, 1])).contains("Ambiguous"));
    }
}
