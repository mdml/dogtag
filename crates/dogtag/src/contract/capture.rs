//! Reading the write seats: `[capture]` and a type's `born-flagged`.
//!
//! Both exist only at a version that defines them, so each entry point here
//! asks the declared version's schema first and answers *nothing* when the
//! version has no write seats — the same existence gate [`super::tags`] uses,
//! and for the same reason. A version-2 model carries no capture declaration at
//! all rather than one the parser resolved from a table that version's format
//! never had, so no provenance entry has to claim a source it does not have.
//!
//! What the gate deliberately does **not** do is disable the verb. `capture`
//! reads [`DEFAULT_CAPTURE_DIRECTORY`] and stamps nothing where a model carries
//! no seat, which is exactly what a version-3 contract declaring neither seat
//! resolves to. The seats configure the verb; they do not enable it.
//!
//! This module reads the declarations' **shape**. Whether a birth flag names a
//! property the type declares, and one the contract declares as a flag, are
//! rules over the whole contract and live in [`super::validate`] beside the
//! flag and lifecycle rules. The one rule that stays here is the repeat,
//! because naming both spellings needs two spans and the resolved model keeps
//! one name per type.

use toml::Spanned;
use toml::de::{DeTable, DeValue};

use crate::diagnostic::KernelDiagnostic;
use crate::document;

use super::model::CaptureDecl;
use super::sink::{Claim, KeyPath, Named, Report, Section, Seen, Sink};

/// The vault-relative directory captures land in where no declaration applies.
///
/// It is contract version 3's default for `[capture] directory`, and it is also
/// what a vault whose format has no capture seat reads: a version-1 or
/// version-2 contract cannot declare a landing spot, and refusing to capture
/// into a vault that cannot configure one would gate capture on configuration,
/// against never-lossy. Both readings name *this* constant rather than "the
/// current version's default", so a later version that changes its own default
/// moves what a contract declaring that version resolves to and leaves what a
/// seat-less vault has always done exactly where it is.
///
/// A named directory rather than the vault root: root-level captures interleave
/// with organized notes in every listing, and the point of the unfiled pile is
/// that it is visibly a pile.
pub const DEFAULT_CAPTURE_DIRECTORY: &str = "captures";

/// The key a type declares its birth state under.
const BORN_FLAGGED: &str = "born-flagged";

const DIRECTORY_HELP: &str = "`[capture] directory` is a vault-relative directory: `/`-separated names, none of them empty \
     and none beginning with `.`";

/// Reads the optional top-level `[capture]` table.
///
/// `None` is the whole of *this version defines no capture seat*. At a version
/// that defines one the answer is always `Some`: a contract that declares no
/// `[capture]` resolves the seat from the version's default table, because the
/// landing spot is a mechanic every vault has an answer for rather than a
/// declaration a vault may decline to make.
///
/// The one exception is a `[capture]` that did not resolve, which is recorded as
/// a dropped declaration so the rules over the whole contract do not conclude
/// from the model's silence what the file does not say.
pub(crate) fn table(sink: &mut Sink<'_>, root: &DeTable<'_>) -> Option<CaptureDecl> {
    let seats = sink.schema().write.as_ref()?;
    let default = seats.directory;
    let allowed = seats.capture;
    let leaf = KeyPath::root().child("capture").leaf("directory");
    let Some(value) = document::get(root, "capture") else {
        sink.defaulted(leaf.key);
        return Some(CaptureDecl {
            directory: default.to_owned(),
        });
    };
    let declared = read_table(sink, value, allowed, default);
    if declared.is_none() {
        sink.drop_declaration();
    }
    declared
}

fn read_table(
    sink: &mut Sink<'_>,
    value: &Spanned<DeValue<'_>>,
    allowed: &'static [&'static str],
    default: &str,
) -> Option<CaptureDecl> {
    let table = sink.table(value, "capture")?;
    let section = Section {
        table,
        span: value.span(),
        label: "`[capture]`".to_owned(),
        path: KeyPath::root().child("capture"),
    };
    sink.sweep(&section, allowed);
    let leaf = section.leaf("directory");
    let Some(declared) = section.get("directory") else {
        sink.defaulted(leaf.key);
        return Some(CaptureDecl {
            directory: default.to_owned(),
        });
    };
    let spelled = sink.string(declared, section.leaf("directory"))?;
    if !is_vault_relative_directory(spelled) {
        let message = format!("`[capture]` declares the directory `{spelled}`");
        let at = sink.location(declared.span());
        let report = Report::new(message).with_help(DIRECTORY_HELP.to_owned());
        sink.report(
            KernelDiagnostic::ContractCaptureDirectoryInvalid,
            report,
            at,
        );
        return None;
    }
    sink.written(leaf.key, declared.span());
    Some(CaptureDecl {
        directory: spelled.to_owned(),
    })
}

/// Whether `spelled` names a directory inside the vault that the corpus can see.
///
/// One rule rather than four, and it is the four: a `/`-separated sequence of
/// non-empty components, none of which begins with `.`. An absolute path leads
/// with an empty component, a trailing slash trails one, and `.`, `..` and
/// `.dogtag` are components that begin with a dot — so escaping the root,
/// naming the root twice, and landing captures where the traversal skips dotted
/// directories are all refused by the same sentence.
fn is_vault_relative_directory(spelled: &str) -> bool {
    !spelled.is_empty()
        && spelled
            .split('/')
            .all(|component| !component.is_empty() && !component.starts_with('.'))
}

/// Reads the `born-flagged` a type declares, in declaration order.
///
/// The version gate answers with no birth state at all where the version has no
/// seat, which is the same answer as an undeclared seat at a version that has
/// one — and deliberately so: the difference between them is which source the
/// provenance records, not what a note is born carrying.
pub(crate) fn born_flagged(sink: &mut Sink<'_>, section: &Section<'_, '_>) -> Vec<String> {
    let Some(seats) = sink.schema().write.as_ref() else {
        return Vec::new();
    };
    let leaf = section.leaf(BORN_FLAGGED);
    let Some(value) = section.get(BORN_FLAGGED) else {
        sink.defaulted(leaf.key);
        return seats
            .born_flagged
            .iter()
            .copied()
            .map(str::to_owned)
            .collect();
    };
    let Some(array) = sink.array(value, BORN_FLAGGED) else {
        return Vec::new();
    };
    sink.written(leaf.key, value.span());
    let mut seen = Seen::new();
    array
        .iter()
        .filter_map(|entry| birth_flag(sink, entry, &mut seen))
        .collect()
}

/// One name inside a type's `born-flagged`.
fn birth_flag(
    sink: &mut Sink<'_>,
    value: &Spanned<DeValue<'_>>,
    seen: &mut Seen,
) -> Option<String> {
    // The provenance of the array as a whole is recorded by the caller, so this
    // read deliberately addresses nothing and records nothing — the same shape
    // a capability's read has, for the same reason.
    let spelled = sink.string(value, KeyPath::nameless().leaf(BORN_FLAGGED))?;
    let claim = Claim {
        message: format!("the type is born carrying `{spelled}` twice"),
        kind: KernelDiagnostic::ContractDuplicateBirthFlag,
        named: Named {
            text: spelled,
            span: value.span(),
        },
    };
    sink.keep(seen, claim)?;
    Some(spelled.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::contract::schema;
    use crate::contract::sink::tests::{root_of, text_of};
    use crate::diagnostic::DiagnosticList;
    use crate::provenance::{Provenance, Source};

    /// A contract that loads clean at any version, with `body` spliced in.
    fn contract(body: &str) -> String {
        format!(
            concat!(
                "[dialect]\nlinks = \"wikilink\"\n",
                "\n[lifecycle]\nnone = true\n",
                "{}",
                "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
            ),
            body
        )
    }

    /// What the walk resolved for `source` at `schema`, with what it reported
    /// and where each leaf came from.
    fn read(
        schema: &'static schema::Schema,
        source: &str,
    ) -> (Option<CaptureDecl>, DiagnosticList, Provenance) {
        let text = text_of(source);
        let document = root_of(&text);
        let mut sink = Sink::new(&text, schema);
        let declared = table(&mut sink, document.get_ref());
        let (diagnostics, provenance) = sink.finish();
        (declared, diagnostics, provenance)
    }

    fn ids(diagnostics: &DiagnosticList) -> Vec<&str> {
        diagnostics
            .as_slice()
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect()
    }

    /// A version with no write seats resolves no capture declaration at all —
    /// absent from, never defaulted into — and records nothing about one.
    #[test]
    fn a_version_without_the_seat_resolves_no_capture_declaration() {
        let (declared, diagnostics, provenance) = read(
            &schema::VERSION_2,
            &contract("\n[capture]\ndirectory = \"x\"\n"),
        );
        assert_eq!(declared, None);
        assert!(diagnostics.as_slice().is_empty());
        assert!(provenance.get("capture.directory").is_none());
    }

    /// An undeclared `[capture]` resolves to the version's default, attributed
    /// to the version that defines it and written nowhere.
    #[test]
    fn an_undeclared_capture_table_resolves_to_the_versions_default() {
        let (declared, diagnostics, provenance) = read(&schema::VERSION_3, &contract(""));
        assert_eq!(
            declared.as_ref().map(CaptureDecl::directory),
            Some(DEFAULT_CAPTURE_DIRECTORY)
        );
        assert!(diagnostics.as_slice().is_empty());
        let entry = provenance.get("capture.directory").expect("recorded");
        assert_eq!(
            entry.source,
            Source::Default {
                contract_version: 3
            }
        );
        assert!(entry.location.is_none());
    }

    /// A declared `[capture]` that omits `directory` resolves the same way a
    /// missing table does: the leaf is what has a default, not the table.
    #[test]
    fn a_capture_table_without_a_directory_resolves_to_the_default() {
        let (declared, diagnostics, provenance) =
            read(&schema::VERSION_3, &contract("\n[capture]\n"));
        assert_eq!(
            declared.as_ref().map(CaptureDecl::directory),
            Some(DEFAULT_CAPTURE_DIRECTORY)
        );
        assert!(diagnostics.as_slice().is_empty());
        assert!(matches!(
            provenance
                .get("capture.directory")
                .map(|entry| entry.source),
            Some(Source::Default { .. })
        ));
    }

    #[test]
    fn a_declared_directory_is_read_and_records_where_it_is_written() {
        let (declared, diagnostics, provenance) = read(
            &schema::VERSION_3,
            &contract("\n[capture]\ndirectory = \"inbox/raw\"\n"),
        );
        assert_eq!(
            declared.as_ref().map(CaptureDecl::directory),
            Some("inbox/raw")
        );
        assert!(diagnostics.as_slice().is_empty());
        let entry = provenance.get("capture.directory").expect("recorded");
        assert_eq!(entry.source, Source::Contract);
        assert!(entry.location.is_some());
    }

    #[test]
    fn a_directory_outside_the_visible_corpus_is_refused_with_its_repair() {
        let (declared, diagnostics, _) = read(
            &schema::VERSION_3,
            &contract("\n[capture]\ndirectory = \"../outside\"\n"),
        );
        assert_eq!(declared, None);
        assert_eq!(ids(&diagnostics), ["contract.capture-directory-invalid"]);
        let reported = &diagnostics.as_slice()[0];
        assert!(reported.message.contains("../outside"));
        assert!(
            reported
                .help
                .as_deref()
                .is_some_and(|help| help.contains("none beginning with `.`"))
        );
        assert!(reported.location.is_some());
    }

    /// Written at the very top, because a bare key after a table header belongs
    /// to that table rather than to the root.
    #[test]
    fn a_capture_table_of_the_wrong_shape_is_reported_as_a_wrong_type() {
        let source = format!("capture = \"captures\"\n{}", contract(""));
        let (declared, diagnostics, _) = read(&schema::VERSION_3, &source);
        assert_eq!(declared, None);
        assert_eq!(ids(&diagnostics), ["contract.value-wrong-type"]);
    }

    #[test]
    fn a_directory_that_is_not_a_string_is_reported_as_a_wrong_type() {
        let (declared, diagnostics, _) = read(
            &schema::VERSION_3,
            &contract("\n[capture]\ndirectory = 1\n"),
        );
        assert_eq!(declared, None);
        assert_eq!(ids(&diagnostics), ["contract.value-wrong-type"]);
    }

    #[test]
    fn an_unknown_key_inside_the_capture_table_is_fatal() {
        let (_, diagnostics, _) = read(
            &schema::VERSION_3,
            &contract("\n[capture]\ndirectory = \"captures\"\nprefix = \"x\"\n"),
        );
        assert!(ids(&diagnostics).contains(&"contract.unknown-key"));
    }

    /// What the whole body walk resolved as the catch-all's birth state, with
    /// what it reported and where the leaf came from.
    ///
    /// A birth state is a leaf on a `[[type]]`, so it is reached through the
    /// declaration walk rather than through [`table`] — the same walk a contract
    /// reaches it by.
    fn born(
        schema: &'static schema::Schema,
        source: &str,
    ) -> (Vec<String>, DiagnosticList, Provenance) {
        let text = text_of(source);
        let document = root_of(&text);
        let mut sink = Sink::new(&text, schema);
        let parts = super::super::parse::body(&mut sink, document.get_ref());
        let (diagnostics, provenance) = sink.finish();
        let flags = parts
            .types
            .first()
            .map(|declared| declared.born_flagged.clone())
            .unwrap_or_default();
        (flags, diagnostics, provenance)
    }

    /// The catch-all declaring `born-flagged = [...]` over a boolean property
    /// the contract also declares as a flag, so nothing but the birth state is
    /// under test.
    fn born_source(spelled: &str) -> String {
        format!(
            concat!(
                "[dialect]\nlinks = \"wikilink\"\n",
                "\n[lifecycle]\nnone = true\n",
                "\n[[flag]]\nproperty = \"needs_triage\"\n",
                "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
                "born-flagged = {}\n",
                "\n  [[type.property]]\n  name = \"needs_triage\"\n  kind = \"boolean\"\n",
            ),
            spelled
        )
    }

    #[test]
    fn a_declared_birth_state_is_read_and_records_where_it_is_written() {
        let (flags, diagnostics, provenance) =
            born(&schema::VERSION_3, &born_source("[\"needs_triage\"]"));
        assert_eq!(flags, ["needs_triage"]);
        assert!(diagnostics.as_slice().is_empty());
        let entry = provenance
            .get("type.capture.born-flagged")
            .expect("recorded");
        assert_eq!(entry.source, Source::Contract);
        assert!(entry.location.is_some());
    }

    /// An undeclared birth state stamps nothing, attributed to the version that
    /// says so — the same shape a defaulted capability list has.
    #[test]
    fn an_undeclared_birth_state_stamps_nothing() {
        let source = born_source("[]").replace("born-flagged = []\n", "");
        let (flags, diagnostics, provenance) = born(&schema::VERSION_3, &source);
        assert!(flags.is_empty());
        assert!(diagnostics.as_slice().is_empty());
        assert_eq!(
            provenance
                .get("type.capture.born-flagged")
                .map(|entry| entry.source),
            Some(Source::Default {
                contract_version: 3
            })
        );
    }

    /// A version with no seat records nothing about a birth state, and the key
    /// is an unknown key rather than a declaration it read and condemned.
    #[test]
    fn a_version_without_the_seat_records_no_birth_state() {
        let (flags, diagnostics, provenance) =
            born(&schema::VERSION_2, &born_source("[\"needs_triage\"]"));
        assert!(flags.is_empty());
        assert_eq!(ids(&diagnostics), ["contract.unknown-key"]);
        assert!(provenance.get("type.capture.born-flagged").is_none());
    }

    #[test]
    fn one_type_born_carrying_the_same_flag_twice_is_refused() {
        let (flags, diagnostics, _) = born(
            &schema::VERSION_3,
            &born_source("[\"needs_triage\", \"needs_triage\"]"),
        );
        assert_eq!(ids(&diagnostics), ["contract.duplicate-birth-flag"]);
        assert_eq!(flags, ["needs_triage"]);
    }

    #[test]
    fn a_birth_state_of_the_wrong_shape_is_reported_as_a_wrong_type() {
        let (flags, diagnostics, _) = born(&schema::VERSION_3, &born_source("\"needs_triage\""));
        assert!(flags.is_empty());
        assert!(ids(&diagnostics).contains(&"contract.value-wrong-type"));
    }

    #[test]
    fn a_birth_flag_that_is_not_a_string_is_reported_as_a_wrong_type() {
        let (flags, diagnostics, _) = born(&schema::VERSION_3, &born_source("[1]"));
        assert!(flags.is_empty());
        assert!(ids(&diagnostics).contains(&"contract.value-wrong-type"));
    }

    #[test]
    fn the_default_directory_is_a_vault_relative_directory_by_its_own_rule() {
        assert!(is_vault_relative_directory(DEFAULT_CAPTURE_DIRECTORY));
    }

    #[test]
    fn a_nested_relative_directory_is_one() {
        for spelled in ["captures", "inbox/raw", "a/b/c", "notes 2026"] {
            assert!(is_vault_relative_directory(spelled), "`{spelled}`");
        }
    }

    /// Each refusal is its own branch of the one rule: nothing at all, an
    /// absolute path, an empty component in the middle, a trailing slash, the
    /// two relative names, and a dotted directory the traversal would skip.
    #[test]
    fn everything_that_leaves_the_visible_corpus_is_refused() {
        for spelled in [
            "",
            "/etc",
            "a//b",
            "captures/",
            ".",
            "..",
            "../outside",
            "a/../..",
            ".dogtag",
            ".dogtag/captures",
            "captures/.hidden",
        ] {
            assert!(!is_vault_relative_directory(spelled), "`{spelled}`");
        }
    }
}
