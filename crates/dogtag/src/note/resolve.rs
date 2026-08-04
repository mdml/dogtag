//! Resolving what a corpus's references name, once the whole corpus is in hand.
//!
//! Reading a note answers what *that note* says; which note a reference names
//! is a question about the corpus, so it is answered here, in one pass over
//! every note the traversal read.
//!
//! # Two doors, two findings, one rule
//!
//! The resolution rule is [`super::index`]'s and is the same at every door. The
//! *finding* is not, and the difference is who wrote the reference:
//!
//! - a **typed link** is a claim the corpus authored — "this note relates to
//!   that one, this way" — so it must resolve, and one that does not is
//!   `link.dangling-typed-link`: an edge with a dangling endpoint is not a
//!   relationship, it is a string;
//! - an **untyped reference** in prose claims nothing, so a dangling one is a
//!   finding at no severity. A reference to a note that does not exist yet
//!   belongs in prose until it does;
//! - a reference a **caller** hands the SDK is neither. Nothing is wrong with
//!   the corpus — the caller asked for a note this vault does not hold — so it
//!   is `link.target-not-found`, and [`UnresolvedReference`] is what carries it
//!   back.
//!
//! Ambiguity is the one finding both authored and caller-supplied references
//! share, because it is a defect of the reference in both: `link.ambiguous-
//! reference`, against the reference, carrying every candidate as evidence.
//! Two notes sharing a name with nothing referencing the bare name is a finding
//! at no severity — this rule only ever fires where something asked.

use crate::contract::LinkDialect;
use crate::diagnostic::{Diagnostic, FileRef, KernelDiagnostic, Location, Related, VaultPath};

use super::index::{Index, Resolution};
use super::links;
use super::model::Note;

/// Resolves every reference `notes` carries, and reports what a typed one must.
pub(crate) fn corpus(notes: &mut [Note], dialect: LinkDialect) -> Vec<Diagnostic> {
    let resolver = Resolver {
        index: Index::of(notes),
        paths: notes.iter().map(|note| note.path.clone()).collect(),
        dialect,
    };
    let mut reported = Vec::new();
    for note in notes.iter_mut() {
        reported.extend(resolver.note(note));
    }
    reported
}

/// The corpus every reference is resolved against.
struct Resolver {
    index: Index,
    paths: Vec<VaultPath>,
    dialect: LinkDialect,
}

impl Resolver {
    /// Resolves one note's references, in the order the note wrote them.
    ///
    /// Typed links first, in the contract's declaration order, then the body's
    /// references in document order — so what a note reports never depends on
    /// anything but the note and the corpus.
    fn note(&self, note: &mut Note) -> Vec<Diagnostic> {
        let mut reported = Vec::new();
        let edges = note
            .relationships
            .iter_mut()
            .flat_map(|relationship| relationship.edges.iter_mut());
        for edge in edges {
            let (target, refused) = self.link(&edge.written, &edge.at);
            edge.target = target;
            reported.extend(refused);
        }
        for reference in &mut note.references {
            let named = self.named(&reference.written);
            reference.target = named.only();
        }
        reported
    }

    /// One typed link: the note it names, and what to report when that is not
    /// exactly one note.
    fn link(&self, written: &str, at: &Location) -> (Option<VaultPath>, Option<Diagnostic>) {
        let reference = links::reference(self.dialect, written);
        match self.named(written) {
            Named::One(path) => (Some(path), None),
            Named::Ambiguous(candidates) => {
                (None, Some(ambiguous(reference, &candidates).at(at.clone())))
            }
            Named::Absent => (None, Some(dangling(reference).at(at.clone()))),
        }
    }

    /// What a reference names, read in this corpus's declared dialect.
    fn named(&self, written: &str) -> Named {
        match self.index.resolve(links::reference(self.dialect, written)) {
            Resolution::One(at) => Named::One(self.paths[at].clone()),
            Resolution::Ambiguous(candidates) => Named::Ambiguous(
                candidates
                    .iter()
                    .map(|at| self.paths[*at].clone())
                    .collect(),
            ),
            Resolution::Absent => Named::Absent,
        }
    }
}

/// What a reference named, in paths rather than positions.
enum Named {
    One(VaultPath),
    Ambiguous(Vec<VaultPath>),
    Absent,
}

impl Named {
    /// The one note named, when exactly one was.
    fn only(self) -> Option<VaultPath> {
        match self {
            Self::One(path) => Some(path),
            _ => None,
        }
    }
}

/// Why a reference a caller supplied named no note.
///
/// The caller-side counterpart of a dangling typed link, and deliberately a
/// different finding: a typed link that does not resolve says the *corpus*
/// claimed a relationship it does not have, while this says only that the
/// reference in hand names no note this vault holds. `show <ref>` is the first
/// consumer; the rule it obeys is the corpus's own, so it lives here rather
/// than in the command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnresolvedReference {
    reference: String,
    candidates: Vec<VaultPath>,
}

impl UnresolvedReference {
    /// The reference exactly as it was supplied.
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// Every note a bare name met, when it met more than one.
    ///
    /// Empty where the reference simply named nothing; ambiguity is the only
    /// refusal that has candidates to offer.
    pub fn candidates(&self) -> &[VaultPath] {
        &self.candidates
    }

    /// The diagnostic this refusal is reported as.
    ///
    /// It carries no location: the reference came from a caller rather than
    /// from a file, and a diagnostic names such a subject in its message rather
    /// than inventing a place for it.
    pub fn diagnostic(&self) -> Diagnostic {
        if self.candidates.is_empty() {
            not_found(&self.reference)
        } else {
            ambiguous(&self.reference, &self.candidates)
        }
    }
}

/// Resolves a caller's reference against a corpus's notes.
pub(crate) fn reference<'a>(
    notes: &'a [Note],
    reference: &str,
) -> Result<&'a Note, UnresolvedReference> {
    match Index::of(notes).resolve(reference) {
        Resolution::One(at) => Ok(&notes[at]),
        Resolution::Ambiguous(candidates) => Err(UnresolvedReference {
            reference: reference.to_owned(),
            candidates: candidates
                .iter()
                .map(|at| notes[*at].path.clone())
                .collect(),
        }),
        Resolution::Absent => Err(UnresolvedReference {
            reference: reference.to_owned(),
            candidates: Vec::new(),
        }),
    }
}

/// A typed link naming no note. The message is the record's own sentence.
fn dangling(reference: &str) -> Diagnostic {
    Diagnostic::kernel(
        KernelDiagnostic::LinkDanglingTypedLink,
        format!("`{reference}` names no note, and a typed link must resolve"),
    )
    .with_help(
        "until the note exists, the reference belongs in prose rather than in a relationship",
    )
}

/// A caller's reference naming no note.
fn not_found(reference: &str) -> Diagnostic {
    Diagnostic::kernel(
        KernelDiagnostic::LinkTargetNotFound,
        format!("`{reference}` names no note in this vault"),
    )
}

/// A bare name several notes bear, carrying every one of them as evidence.
fn ambiguous(reference: &str, candidates: &[VaultPath]) -> Diagnostic {
    let mut reported = Diagnostic::kernel(
        KernelDiagnostic::LinkAmbiguousReference,
        format!(
            "`{reference}` is a bare name that {} notes bear, so it names none of them",
            candidates.len()
        ),
    )
    .with_help("qualify the reference with enough of its path to pick one");
    for candidate in candidates {
        reported = reported.with_related(bearer(candidate));
    }
    reported
}

fn bearer(path: &VaultPath) -> Related {
    Related::new(format!("`{path}` bears that name"))
        .at(Location::whole_file(FileRef::InVault(path.clone())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::note::model::Binding;

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

    #[test]
    fn a_callers_reference_answers_the_note_it_names() {
        let notes = [note("people/ada.md"), note("people/babbage.md")];
        let found = reference(&notes, "ada").expect("one note bears it");
        assert_eq!(found.path().as_str(), "people/ada.md");
        let qualified = reference(&notes, "people/babbage").expect("a path resolves exactly");
        assert_eq!(qualified.path().as_str(), "people/babbage.md");
    }

    #[test]
    fn a_callers_reference_naming_nothing_is_target_not_found_with_no_candidates() {
        let notes = [note("people/ada.md")];
        let refused = reference(&notes, "babbage").expect_err("no note bears it");
        assert_eq!(refused.reference(), "babbage");
        assert!(refused.candidates().is_empty());
        let reported = refused.diagnostic();
        assert_eq!(reported.id.as_str(), "link.target-not-found");
        assert!(
            reported.message.contains("`babbage`"),
            "{}",
            reported.message
        );
        assert!(
            reported.location.is_none(),
            "a caller's reference is in no file, so the message names it instead"
        );
    }

    #[test]
    fn a_callers_ambiguous_reference_carries_every_candidate_as_evidence() {
        let notes = [note("2025/daily.md"), note("2026/daily.md")];
        let refused = reference(&notes, "daily").expect_err("two notes bear it");
        assert_eq!(
            refused
                .candidates()
                .iter()
                .map(VaultPath::as_str)
                .collect::<Vec<_>>(),
            ["2025/daily.md", "2026/daily.md"]
        );
        let reported = refused.diagnostic();
        assert_eq!(reported.id.as_str(), "link.ambiguous-reference");
        assert!(reported.message.contains("2 notes"), "{}", reported.message);
        assert_eq!(reported.related.len(), 2);
        assert!(reported.related[0].message.contains("`2025/daily.md`"));
        assert!(reported.related[0].location.is_some());
    }

    #[test]
    fn an_unresolved_reference_clones_compares_and_formats() {
        let notes = [note("a.md")];
        let refused = reference(&notes, "b").expect_err("no note bears it");
        assert_eq!(refused.clone(), refused);
        assert_ne!(refused, reference(&notes, "c").expect_err("nor this"));
        assert!(format!("{refused:?}").contains("UnresolvedReference"));
    }
}
