//! Reading one note: its bytes, its frontmatter, its type, and its body.
//!
//! The order is the one the rules depend on. Bytes first, because a file that
//! cannot be read has nothing else to say. Then the frontmatter block, because
//! everything schema'd is in it. Then the type, because the type is the dispatch
//! key for every remaining rule. Then the note's contents against that type.
//!
//! Two refusals stop the pipeline and both stop it for the same reason: there
//! is nothing left to judge the note against. Invalid UTF-8 leaves no text; a
//! frontmatter block that did not load leaves no declarations to read the note
//! by, so it binds to nothing rather than being treated as a note with no
//! frontmatter at all — the catch-all binds *absence*, and a block that failed
//! to parse is not an absence.

use std::fs;
use std::io;
use std::path::Path;

use crate::contract::Contract;
use crate::diagnostic::{Diagnostic, KernelDiagnostic, Location, VaultPath};
use crate::encoding::{self, EncodingFault, Reading};

use super::body;
use super::findings::Findings;
use super::frontmatter::{self, Fault, FaultKind, Front};
use super::model::{Binding, Note};
use super::validate;

/// One note, read — or the reasons it could not be.
pub(crate) struct Read {
    /// The note, absent only when its bytes could not be read at all.
    pub(crate) note: Option<Note>,
    /// Everything reading it reported, in the deterministic total order.
    pub(crate) diagnostics: Vec<Diagnostic>,
}

/// What a note is read as: where it is, and the rules it is held to.
///
/// The two travel together because neither answers anything alone — a path with
/// no contract has no rules, and a contract with no path has no note — and
/// because every step below needs both.
#[derive(Clone, Copy)]
struct Subject<'a> {
    path: &'a VaultPath,
    contract: &'a Contract,
}

/// Reads the note at `path`, which is `absolute` on this machine.
pub(crate) fn note(absolute: &Path, path: &VaultPath, contract: &Contract) -> Read {
    let subject = Subject { path, contract };
    let bytes = match fs::read(absolute) {
        Ok(bytes) => bytes,
        Err(error) => return refused(path, &unreadable(path, &error)),
    };
    match encoding::inspect_all(&bytes) {
        Ok(reading) => from_text(subject, reading),
        Err(fault) => refused(
            path,
            &format!("`{path}` could not be read: {}", fault.describe()),
        ),
    }
}

/// The pipeline, once the note's bytes are text.
fn from_text(subject: Subject<'_>, reading: Reading) -> Read {
    let path = subject.path;
    let mut findings = Findings::new(path, &reading.text);
    for fault in &reading.faults {
        let offset = fault.offset().unwrap_or(0);
        findings.spanned(warned_about(*fault), fault.describe(), offset..offset);
    }
    let parsed = frontmatter::read(reading.text.as_str());
    for fault in &parsed.faults {
        frontmatter_fault(&mut findings, fault);
    }
    let body = reading.text.as_str()[parsed.body].to_owned();
    let note = build(&mut findings, subject, &parsed.front, body);
    Read {
        note: Some(note),
        diagnostics: findings.finish(),
    }
}

/// The document model, once the note's frontmatter is in hand.
fn build(findings: &mut Findings<'_>, subject: Subject<'_>, front: &Front, body: String) -> Note {
    let (path, contract) = (subject.path, subject.contract);
    // A block that did not load leaves no discriminator to read and no keys to
    // hold against a type. Binding it to the catch-all would treat a refusal as
    // an absence, which is the one thing the catch-all does not do.
    if matches!(front, Front::Refused) {
        return bare(path, Binding::Unbound { named: None }, body);
    }
    let entries = front.entries().unwrap_or_default();
    let bound = validate::bind(findings, contract, entries);
    let Some(declared) = bound.declared else {
        return bare(path, bound.binding, body);
    };
    let contents = validate::contents(findings, contract, declared, entries);
    Note {
        path: path.clone(),
        binding: bound.binding,
        properties: contents.properties,
        relationships: contents.relationships,
        tags: contents.tags,
        title: body::title(&body),
        body,
    }
}

/// A note that bound to no type, and so carries nothing type-directed.
fn bare(path: &VaultPath, binding: Binding, body: String) -> Note {
    Note {
        path: path.clone(),
        binding,
        properties: Vec::new(),
        relationships: Vec::new(),
        tags: Vec::new(),
        title: body::title(&body),
        body,
    }
}

/// A note whose bytes could not be read at all.
fn refused(path: &VaultPath, message: &str) -> Read {
    let at = Location::whole_file(crate::diagnostic::FileRef::InVault(path.clone()));
    Read {
        note: None,
        diagnostics: vec![
            Diagnostic::kernel(KernelDiagnostic::NoteUnreadable, message.to_owned()).at(at),
        ],
    }
}

fn unreadable(path: &VaultPath, error: &io::Error) -> String {
    format!("`{path}` could not be read: {error}")
}

/// The identifier a fault a note is read *despite* is reported under.
///
/// Exactly two faults arrive here, and both are warnings: the contract is
/// dogtag's own file and can be held to one encoding, while a corpus is decades
/// of files written by whatever wrote them, and refusing to read a note with
/// carriage returns would fail the obligation to read what is there. Every one
/// the bytes carry is reported rather than the first, because they are
/// independent facts and repairing one must not reveal another that was there
/// all along.
///
/// Invalid UTF-8 is not among them: it leaves no text to report against, so the
/// caller that could not read the bytes reports it instead.
fn warned_about(fault: EncodingFault) -> KernelDiagnostic {
    match fault {
        EncodingFault::ByteOrderMark => KernelDiagnostic::NoteByteOrderMark,
        _ => KernelDiagnostic::NoteCarriageReturnLineEnding,
    }
}

/// A refused frontmatter construct, under the identifier its refusal is.
fn frontmatter_fault(findings: &mut Findings<'_>, fault: &Fault) {
    let kind = match fault.kind {
        FaultKind::Invalid => KernelDiagnostic::NoteFrontmatterInvalid,
        FaultKind::Unsupported => KernelDiagnostic::NoteFrontmatterUnsupported,
    };
    findings.spanned(kind, fault.message.clone(), fault.span.clone());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::parse_contract;

    /// The smallest contract a note can be read against.
    const CONTRACT: &str = concat!(
        "contract_version = 2\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    #[test]
    fn a_note_that_is_not_there_is_a_diagnostic_against_it_rather_than_a_failure() {
        // The walk lists a note and the read meets an absence: a corpus can
        // change under a reader, and one file that vanished is a finding about
        // that file rather than the end of the corpus.
        let load = parse_contract(CONTRACT);
        let contract = load.contract.expect("a conforming contract");
        let path = VaultPath::kernel("gone.md");
        let read = note(Path::new("/nonexistent/dogtag/gone.md"), &path, &contract);
        assert!(read.note.is_none());
        assert_eq!(read.diagnostics.len(), 1);
        assert_eq!(read.diagnostics[0].id.as_str(), "note.unreadable");
        let message = &read.diagnostics[0].message;
        assert!(message.starts_with("`gone.md`"));
        assert!(
            !message.contains("/nonexistent"),
            "a machine path never reaches output: {message}"
        );
    }
}
