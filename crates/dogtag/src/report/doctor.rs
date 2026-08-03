//! The `doctor` report as a reader sees it.
//!
//! One report, deterministic, and colourless — colour belongs to whichever
//! consumer knows whether its destination is a terminal, and is applied around
//! this output rather than inside it.
//!
//! ```text
//! vault
//!   root          /canonical/path/to/vault
//!   selected by   upward discovery from the current directory
//! contract
//!   path          .dogtag/contract.toml
//!   present       yes
//!   version       2 (current; supported 1..=2)
//! types           4 declared
//!   identity-bearing   person, place
//!   catch-all          capture
//!   closed-write       source
//! lifecycle       axis "status", ordinary state is the absence of a value
//! dialect         links: wikilink
//! installation
//!   path          $XDG_CONFIG_HOME/dogtag/installation.toml
//!   state         loaded
//!   version       1 (current; supported 1..=1)
//!   actor         A Maintainer
//!   registry      this vault is registered as "work"
//!
//! no diagnostics
//! ```
//!
//! **Every line that depends on something absent says what is absent.** A
//! contract that did not resolve turns each of the three contract-dependent
//! sections into `not evaluated (<reason>)`; a machine with no installation
//! record does the same to the record's three. Never a blank, and never a
//! dropped line: an omission is indistinguishable from a bug, and the report is
//! read hardest exactly when something is wrong with it.
//!
//! **Every row is one line, and only a diagnostic block opens one.** A corpus's
//! vocabulary and the vault's own directory name reach this grid as free text,
//! and the report is scanned by line, so a value folds its line breaks to
//! spaces on the way into a row — the fold this crate's `text` module owns.
//! Without it a planted contract could name a type across two lines and put a
//! line shaped exactly like a diagnostic headline into a report that raised no
//! diagnostic at all.
//!
//! The rendering names no command. This is the SDK's report, and the CLI, the
//! MCP server and a binding all reach it; a line naming one of them would be a
//! lie in the other two.

use super::{DoctorReport, Evaluated, InstallationFacts, Sections, VersionFacts, yes_no};
use crate::compat::VersionClass;
use crate::contract::{CONTRACT_PATH, LifecycleDecl, Ordinary};
use crate::diagnostic::{FileRef, render_plain};
use crate::text::one_line;

/// A column of labels and a column of values.
///
/// The report is a grid rather than prose because it is scanned rather than
/// read: a reader looking for the root should find it without reading the
/// lifecycle line.
struct Grid {
    indent: usize,
    column: usize,
}

impl Grid {
    /// One line: the label, padded out, and its value.
    ///
    /// The value folds to a single line on the way in. A row **is** a line, and
    /// the values reaching one include a corpus's own vocabulary and its root's
    /// own directory name — free text, in a rendering that is scanned line by
    /// line — so a line break would emit a second line a reader would take for
    /// the report's own grammar, up to and including a line shaped exactly like
    /// a diagnostic headline. Folding here rather than at each call site is what
    /// makes that true of a field added later as well.
    fn row(&self, label: &str, value: &str) -> String {
        let padding = " ".repeat(self.indent);
        let width = self.column - self.indent;
        format!("{padding}{label:width$}{}", one_line(value))
    }
}

/// A fact belonging to the heading above it.
const FIELD: Grid = Grid {
    indent: 2,
    column: 16,
};

/// A section that is its own heading, because its value is the answer.
const SECTION: Grid = Grid {
    indent: 0,
    column: 16,
};

/// The types declaring one capability.
const CAPABILITY: Grid = Grid {
    indent: 2,
    column: 21,
};

/// How each classification reads in a sentence, as against on the wire.
const CLASSES: &[(VersionClass, &str)] = &[
    (VersionClass::BelowFloor, "below the supported floor"),
    (VersionClass::Supported, "supported"),
    (VersionClass::Current, "current"),
    (VersionClass::TooNew, "too new"),
];

/// Why a record contributes no facts at all, per state.
///
/// How the text names a run whose selection resolved no vault.
const NO_ROOT: &str = "none resolved";

/// A *loaded* record is absent from this table on purpose: a loaded record that
/// declares no actor has said something, and "not evaluated" would be false.
const RECORD_SILENCE: &[(&str, &str)] = &[
    ("absent", "this machine has no installation record"),
    ("unusable", "the installation record could not be used"),
];

/// Renders the `doctor` report as text.
///
/// The output ends in a newline and holds no colour, no timestamp and no
/// absolute path but the vault root, so the same vault renders identically on
/// every machine. The discovery and trust diagnostics it carries do name
/// machine paths; that is the decision recorded in the compatibility record,
/// not an exception taken here.
pub fn doctor_text(report: &DoctorReport) -> String {
    let mut lines = vault_block(report);
    lines.extend(contract_block(report));
    lines.extend(sections_block(report));
    lines.extend(installation_block(report));
    format!("{}\n\n{}", lines.join("\n"), diagnostics_text(report))
}

/// Which vault this is, and which decision chose it.
fn vault_block(report: &DoctorReport) -> Vec<String> {
    vec![
        "vault".to_owned(),
        FIELD.row("root", report.root.as_deref().unwrap_or(NO_ROOT)),
        FIELD.row("selected by", &report.selection.describe()),
    ]
}

/// Whether there is a contract, and what version it declares.
fn contract_block(report: &DoctorReport) -> Vec<String> {
    vec![
        "contract".to_owned(),
        FIELD.row("path", CONTRACT_PATH),
        FIELD.row("present", yes_no(report.contract.present)),
        FIELD.row("version", &version_text(&report.contract.version)),
    ]
}

/// The three sections that exist only because a contract resolved.
fn sections_block(report: &DoctorReport) -> Vec<String> {
    match &report.sections {
        Sections::Evaluated(evaluated) => evaluated_block(evaluated),
        Sections::NotEvaluated(reason) => not_evaluated_block(reason),
    }
}

/// Each contract-dependent section, saying why it has no answer.
fn not_evaluated_block(reason: &str) -> Vec<String> {
    let statement = format!("not evaluated ({reason})");
    ["types", "lifecycle", "dialect"]
        .iter()
        .map(|label| SECTION.row(label, &statement))
        .collect()
}

/// Each contract-dependent section, answering.
fn evaluated_block(evaluated: &Evaluated) -> Vec<String> {
    vec![
        SECTION.row("types", &format!("{} declared", evaluated.types_declared)),
        CAPABILITY.row("identity-bearing", &names(&evaluated.identity_bearing)),
        CAPABILITY.row("catch-all", &names(evaluated.catch_all.as_slice())),
        CAPABILITY.row("closed-write", &names(&evaluated.closed_write)),
        SECTION.row("lifecycle", &lifecycle_text(&evaluated.lifecycle)),
        SECTION.row("dialect", &format!("links: {}", evaluated.links)),
    ]
}

/// The installation record's own facts, and this vault's entry in it.
fn installation_block(report: &DoctorReport) -> Vec<String> {
    let facts = &report.installation;
    vec![
        "installation".to_owned(),
        FIELD.row("path", FileRef::INSTALLATION_RECORD_PATH),
        FIELD.row("state", facts.state),
        FIELD.row("version", &record_version(facts)),
        FIELD.row("actor", &record_actor(facts)),
        FIELD.row("registry", &record_registry(facts)),
    ]
}

/// Everything the run had to say, or that it had nothing to say.
fn diagnostics_text(report: &DoctorReport) -> String {
    if report.diagnostics().is_empty() {
        return "no diagnostics\n".to_owned();
    }
    render_plain(report.diagnostics())
}

/// A declared version and where it sits, or that none was declared.
///
/// The supported range is printed either way: the classification is a statement
/// about this build, and it is unreadable without the range it judged against.
fn version_text(facts: &VersionFacts) -> String {
    let range = format!("supported {}..={}", facts.min, facts.max);
    match (facts.found, facts.classification) {
        (Some(found), Some(class)) => format!("{found} ({}; {range})", class_text(class)),
        (None, Some(class)) => {
            format!("beyond `0..={}` ({}; {range})", u32::MAX, class_text(class))
        }
        _ => format!("not declared ({range})"),
    }
}

/// A classification in a reader's words.
fn class_text(class: VersionClass) -> &'static str {
    CLASSES
        .iter()
        .find(|(known, _)| *known == class)
        .map(|(_, text)| *text)
        .expect("every classification reads in a sentence")
}

/// The record's declared version, or why there is not one.
fn record_version(facts: &InstallationFacts) -> String {
    facts.version.as_ref().map_or_else(
        || silent(facts, "the record declares no version"),
        version_text,
    )
}

/// The record's actor, or why there is not one.
fn record_actor(facts: &InstallationFacts) -> String {
    facts
        .actor
        .clone()
        .unwrap_or_else(|| silent(facts, "the record declares no actor"))
}

/// How this vault is registered, or that it is not.
fn record_registry(facts: &InstallationFacts) -> String {
    facts.entry.as_ref().map_or_else(
        || silent(facts, "this vault is not in the registry"),
        |entry| format!("this vault is registered as \"{}\"", entry.name),
    )
}

/// What to say when a record-dependent fact is missing.
///
/// A record that could not be read says so once, in every line that depended on
/// it. A record that loaded and simply did not declare the thing says *that*
/// instead, in `declared` — the two are different findings and a reader acts on
/// them differently.
fn silent(facts: &InstallationFacts, declared: &str) -> String {
    RECORD_SILENCE
        .iter()
        .find(|(state, _)| *state == facts.state)
        .map_or_else(
            || declared.to_owned(),
            |(_, reason)| format!("not evaluated ({reason})"),
        )
}

/// A list of names, or that the list is empty.
fn names(names: &[String]) -> String {
    if names.is_empty() {
        return "none declared".to_owned();
    }
    names.join(", ")
}

/// The lifecycle declaration as a statement, including the statement that there
/// is no axis.
fn lifecycle_text(lifecycle: &LifecycleDecl) -> String {
    match (lifecycle.axis(), lifecycle.ordinary()) {
        (Some(axis), Some(Ordinary::Absent)) => {
            format!("axis \"{axis}\", ordinary state is the absence of a value")
        }
        (Some(axis), Some(Ordinary::Value(value))) => {
            format!("axis \"{axis}\", ordinary state is \"{value}\"")
        }
        _ => "no axis declared".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixture::{
        ABSENT_ORDINARY, AWKWARD, Body, CLEAN, FORGERY, FORGING_ENUM_VALUE, FORGING_EVIDENCE,
        FORGING_TYPE_NAME, NAME_FORGERY, NAMED_ORDINARY, RECORD, Tree, assert_holds, no_record,
        opened, registering, shown,
    };
    use super::super::{Selection, SelectionRoute, doctor_report};
    use super::*;
    use crate::diagnostic::{Diagnostic, KernelDiagnostic};
    use crate::installation::parse_installation;
    use crate::text::headline_lines;
    use crate::vault::open;
    use std::fs;

    fn discovery() -> Selection {
        Selection::new(SelectionRoute::Discovery, None)
    }

    fn text_of(tree: &Tree, body: Body<'_>, record: Body<'_>) -> String {
        doctor_text(&doctor_report(
            &opened(tree, body, record),
            discovery(),
            &[],
        ))
    }

    #[test]
    fn a_healthy_vault_renders_the_whole_grid() {
        let tree = Tree::new("text-healthy");
        let root = tree.vault(NAMED_ORDINARY);
        let expected = shown(root.path());
        let record = registering(&root);
        let report = doctor_report(
            &open(root, parse_installation(&record)),
            Selection::new(SelectionRoute::FlagName, Some("work".to_owned())),
            &[],
        );
        assert_eq!(
            doctor_text(&report),
            format!(
                concat!(
                    "vault\n",
                    "  root          {expected}\n",
                    "  selected by   --vault work (registered)\n",
                    "contract\n",
                    "  path          .dogtag/contract.toml\n",
                    "  present       yes\n",
                    "  version       2 (current; supported 1..=2)\n",
                    "types           3 declared\n",
                    "  identity-bearing   person\n",
                    "  catch-all          note\n",
                    "  closed-write       none declared\n",
                    "lifecycle       axis \"status\", ordinary state is \"active\"\n",
                    "dialect         links: wikilink\n",
                    "installation\n",
                    "  path          $XDG_CONFIG_HOME/dogtag/installation.toml\n",
                    "  state         loaded\n",
                    "  version       1 (current; supported 1..=1)\n",
                    "  actor         A Maintainer\n",
                    "  registry      this vault is registered as \"work\"\n",
                    "\n",
                    "no diagnostics\n",
                ),
                expected = expected
            )
        );
    }

    #[test]
    fn an_unusable_contract_still_reports_the_root_and_the_record() {
        let tree = Tree::new("text-unusable");
        let rendered = text_of(&tree, Body::new("contract_version = 3\n"), RECORD);
        let reason = "the contract declares a version above the supported range 1..=2";
        assert_holds(&rendered, "  present       yes\n");
        assert_holds(&rendered, "  version       3 (too new; supported 1..=2)\n");
        assert_holds(
            &rendered,
            &format!("types           not evaluated ({reason})\n"),
        );
        assert_holds(
            &rendered,
            &format!("lifecycle       not evaluated ({reason})\n"),
        );
        assert_holds(
            &rendered,
            &format!("dialect         not evaluated ({reason})\n"),
        );
        assert_holds(&rendered, "  actor         A Maintainer\n");
        assert_holds(&rendered, "error[compat.contract-too-new]");
    }

    #[test]
    fn a_version_no_u32_holds_is_reported_as_too_new_rather_than_as_undeclared() {
        let tree = Tree::new("text-beyond-domain");
        let rendered = text_of(&tree, Body::new("contract_version = 4294967296\n"), RECORD);
        assert_holds(
            &rendered,
            "  version       beyond `0..=4294967295` (too new; supported 1..=2)\n",
        );
        assert_holds(&rendered, "error[compat.contract-too-new]");
    }

    #[test]
    fn a_missing_contract_says_so_rather_than_leaving_the_version_blank() {
        let tree = Tree::new("text-missing");
        let root = tree.vault(CLEAN);
        fs::remove_file(root.contract_path()).expect("a contract this test owns");
        let report = doctor_report(
            &open(root, parse_installation(RECORD.as_str())),
            discovery(),
            &[],
        );
        let rendered = doctor_text(&report);
        assert_holds(&rendered, "  present       no\n");
        assert_holds(
            &rendered,
            "  version       not declared (supported 1..=2)\n",
        );
        assert_holds(
            &rendered,
            "types           not evaluated (the vault holds no contract file)\n",
        );
    }

    #[test]
    fn a_machine_with_no_record_says_why_each_of_its_lines_is_empty() {
        let tree = Tree::new("text-no-record");
        let report = doctor_report(&open(tree.vault(CLEAN), no_record(&tree)), discovery(), &[]);
        let rendered = doctor_text(&report);
        let reason = "not evaluated (this machine has no installation record)";
        assert_holds(&rendered, "  state         absent\n");
        assert_holds(&rendered, &format!("  version       {reason}\n"));
        assert_holds(&rendered, &format!("  actor         {reason}\n"));
        assert_holds(&rendered, &format!("  registry      {reason}\n"));
    }

    #[test]
    fn a_record_that_could_not_be_used_says_that_instead() {
        let tree = Tree::new("text-unusable-record");
        let rendered = text_of(
            &tree,
            CLEAN,
            Body::new("installation_version = 1\nstray = true\n"),
        );
        let reason = "not evaluated (the installation record could not be used)";
        assert_holds(&rendered, "  state         unusable\n");
        assert_holds(&rendered, &format!("  actor         {reason}\n"));
        assert_holds(&rendered, &format!("  registry      {reason}\n"));
    }

    #[test]
    fn a_loaded_record_that_declares_nothing_reports_the_declaration_rather_than_the_state() {
        let tree = Tree::new("text-bare-record");
        let rendered = text_of(&tree, CLEAN, Body::new("installation_version = 1\n"));
        assert_holds(&rendered, "  state         loaded\n");
        assert_holds(&rendered, "  actor         the record declares no actor\n");
        assert_holds(
            &rendered,
            "  registry      this vault is not in the registry\n",
        );
    }

    #[test]
    fn a_corpus_with_no_life_axis_states_it_rather_than_dropping_the_line() {
        let tree = Tree::new("text-no-axis");
        assert_holds(
            &text_of(&tree, CLEAN, RECORD),
            "lifecycle       no axis declared\n",
        );
    }

    #[test]
    fn an_axis_whose_ordinary_state_is_absence_says_which_it_is() {
        let tree = Tree::new("text-absent-axis");
        assert_holds(
            &text_of(&tree, ABSENT_ORDINARY, RECORD),
            "lifecycle       axis \"standing\", ordinary state is the absence of a value\n",
        );
    }

    #[test]
    fn every_capability_line_is_present_even_where_no_type_declares_it() {
        let tree = Tree::new("text-capabilities");
        let rendered = text_of(&tree, ABSENT_ORDINARY, RECORD);
        assert_holds(&rendered, "types           5 declared\n");
        assert_holds(&rendered, "  identity-bearing   person, organization\n");
        assert_holds(&rendered, "  catch-all          unfiled\n");
        assert_holds(&rendered, "  closed-write       clipping, snapshot\n");
        let bare = text_of(&tree, CLEAN, RECORD);
        assert_holds(&bare, "  identity-bearing   none declared\n");
        assert_holds(&bare, "  closed-write       none declared\n");
        assert_holds(&bare, "  catch-all          capture\n");
    }

    #[test]
    fn a_corpus_vocabulary_carrying_awkward_characters_survives_the_grid() {
        let tree = Tree::new("text-awkward");
        let rendered = text_of(&tree, AWKWARD, RECORD);
        assert_holds(
            &rendered,
            "lifecycle       axis \"état\", ordinary state is \"a \" quote\"\n",
        );
        assert_holds(&rendered, "dialect         links: markdown\n");
    }

    #[test]
    fn diagnostics_replace_the_no_diagnostics_line_and_never_join_it() {
        let tree = Tree::new("text-diagnostics");
        let planted = Diagnostic::kernel(KernelDiagnostic::DiscoveryNestedVault, "an ancestor");
        let report = doctor_report(
            &opened(&tree, CLEAN, RECORD),
            discovery(),
            std::slice::from_ref(&planted),
        );
        let rendered = doctor_text(&report);
        assert!(!rendered.contains("no diagnostics"));
        assert!(rendered.ends_with("warning[discovery.nested-vault]: an ancestor\n"));
    }

    #[test]
    fn a_type_name_carrying_a_line_break_cannot_forge_a_headline_in_a_clean_report() {
        let tree = Tree::new("text-forged-type");
        let rendered = text_of(&tree, FORGING_TYPE_NAME, RECORD);
        assert_eq!(
            headline_lines(&rendered),
            0,
            "this vault raised nothing, so no line may claim the kernel spoke"
        );
        assert_holds(
            &rendered,
            &format!("  catch-all          capture {NAME_FORGERY}\n"),
        );
        assert_holds(&rendered, "no diagnostics\n");
    }

    #[test]
    fn an_enum_value_carrying_a_carriage_return_cannot_forge_a_second_headline() {
        let tree = Tree::new("text-forged-enum");
        let rendered = text_of(&tree, FORGING_ENUM_VALUE, RECORD);
        assert_eq!(headline_lines(&rendered), 1);
        assert_holds(
            &rendered,
            "error[contract.lifecycle-ordinary-value-undeclared]:",
        );
        assert_holds(
            &rendered,
            &format!("  help: the axis declares `draft {FORGERY}`\n"),
        );
        assert!(!rendered.contains('\r'), "no row and no block carries one");
    }

    #[test]
    fn a_type_name_reaching_a_note_line_cannot_forge_a_headline_either() {
        let tree = Tree::new("text-forged-evidence");
        let rendered = text_of(&tree, FORGING_EVIDENCE, RECORD);
        assert_eq!(headline_lines(&rendered), 1);
        assert_holds(&rendered, "error[contract.multiple-catch-all]:");
        assert_holds(
            &rendered,
            &format!("  note: the type `scrap {NAME_FORGERY}` also declares it ("),
        );
    }

    #[test]
    fn every_route_and_every_classification_reads_in_a_sentence() {
        for (class, expected) in CLASSES {
            assert_eq!(class_text(*class), *expected);
        }
        assert_eq!(
            class_text(VersionClass::BelowFloor),
            "below the supported floor"
        );
        assert_eq!(yes_no(true), "yes");
        assert_eq!(yes_no(false), "no");
    }

    #[test]
    fn the_same_report_renders_the_same_bytes_every_time() {
        let tree = Tree::new("text-deterministic");
        let report = doctor_report(&opened(&tree, ABSENT_ORDINARY, RECORD), discovery(), &[]);
        assert_eq!(doctor_text(&report), doctor_text(&report));
    }
}
