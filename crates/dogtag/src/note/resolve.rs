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
//! # Ambiguity is every door's finding, including prose
//!
//! `link.ambiguous-reference` fires at all three doors, against the reference,
//! carrying every candidate as evidence. The record states the ambiguity rule
//! without qualifying it by plane, and carves out only *danglingness* for an
//! untyped reference — with a reason that does not reach ambiguity: a prose
//! reference "belongs in prose until its target exists", and an ambiguous
//! reference's targets all exist. It names none of them, which is the
//! markdown-flavor position that **ambiguity is a defect of the link, not of
//! the corpus**. The record's other half of the rule points the same way: two
//! notes sharing a name is a finding at no severity *"with nothing referencing
//! the bare name"*, and a body that writes the bare name is something
//! referencing it.
//!
//! So the plane decides one thing only — whether *absence* is reported — and
//! nothing else.
//!
//! # What resolves is the target, and the target alone
//!
//! [`super::links`] splits a written reference into target, fragment, and (for
//! a wikilink) display alias. Resolution reads the target: the alias is
//! cosmetic and the fragment is unvalidated at M3, so `[[engine#history|the
//! Engine]]` resolves — and, when it must, is reported — as `engine`. The one
//! shape the fragment decides is the self-reference: an empty target with a
//! nonempty fragment (`[[#heading]]`) names the note it is written in, which
//! always exists, so it is never dangling and never ambiguous.

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
        let resolver = InNote {
            corpus: self,
            own: note.path.clone(),
        };
        let mut reported = Vec::new();
        let edges = note
            .relationships
            .iter_mut()
            .flat_map(|relationship| relationship.edges.iter_mut());
        for edge in edges {
            let (target, refused) = resolver.resolve(&edge.written, &edge.at, Absence::Reported);
            edge.target = target;
            reported.extend(refused);
        }
        for reference in &mut note.references {
            let (target, refused) =
                resolver.resolve(&reference.written, &reference.at, Absence::Excused);
            reference.target = target;
            reported.extend(refused);
        }
        reported
    }
}

/// The corpus, seen from the one note whose references are being resolved.
///
/// The containing note is part of what a reference means — `[[#heading]]`
/// names it — so it is held here rather than passed alongside every reference.
struct InNote<'a> {
    corpus: &'a Resolver,
    own: VaultPath,
}

impl InNote<'_> {
    /// One reference: the note it names, and what to report when that is not
    /// exactly one note. The same rule at both doors — only `absence` differs.
    fn resolve(
        &self,
        written: &str,
        at: &Location,
        absence: Absence,
    ) -> (Option<VaultPath>, Option<Diagnostic>) {
        match (self.named(written), absence) {
            (Named::One(path), _) => (Some(path), None),
            (Named::Ambiguous(candidates), _) => {
                (None, Some(self.ambiguity(written, &candidates, at)))
            }
            (Named::Absent, Absence::Reported) => {
                (None, Some(dangling(self.target(written)).at(at.clone())))
            }
            (Named::Absent, Absence::Excused) => (None, None),
        }
    }

    /// A bare name this corpus's own text wrote and several notes bear.
    fn ambiguity(&self, written: &str, candidates: &[VaultPath], at: &Location) -> Diagnostic {
        ambiguous(self.target(written), candidates).at(at.clone())
    }

    /// The target `written` resolves by: the reference part with its dialect's
    /// delimiters, alias, and fragment split away.
    fn target<'a>(&self, written: &'a str) -> &'a str {
        links::reference(self.corpus.dialect, written).target
    }

    /// What a reference names, read in this corpus's declared dialect.
    ///
    /// An empty target with a nonempty fragment — `[[#heading]]` — names the
    /// note it is written in, which always exists, so it never dangles.
    fn named(&self, written: &str) -> Named {
        let reference = links::reference(self.corpus.dialect, written);
        if reference.names_own_note() {
            return Named::One(self.own.clone());
        }
        let paths = &self.corpus.paths;
        match self.corpus.index.resolve(reference.target) {
            Resolution::One(at) => Named::One(paths[at].clone()),
            Resolution::Ambiguous(candidates) => {
                Named::Ambiguous(candidates.iter().map(|at| paths[*at].clone()).collect())
            }
            Resolution::Absent => Named::Absent,
        }
    }
}

/// The one thing a reference's plane decides: whether *absence* is reported.
#[derive(Clone, Copy)]
enum Absence {
    /// A typed link claims a relationship, so a target that is not there is
    /// `link.dangling-typed-link`.
    Reported,
    /// A prose reference claims nothing, and belongs in prose until its
    /// target exists.
    Excused,
}

/// What a reference named, in paths rather than positions.
enum Named {
    One(VaultPath),
    Ambiguous(Vec<VaultPath>),
    Absent,
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
