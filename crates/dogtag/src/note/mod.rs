//! The public document model: what a note is, and what reading one answers.
//!
//! A note is a plain Markdown file whose frontmatter is the schema'd plane and
//! whose body is unschema'd prose. This module is where the corpus is finally
//! read: which files are notes ([`traverse`]), what their frontmatter says, and
//! how each note measures up against the type the contract declares for it.
//!
//! Two rules run through everything here and are worth stating once.
//!
//! **Identity is the path.** A note's identity is its vault-relative path, and
//! nothing else — not its name, not its title, not a key in its frontmatter. A
//! bare name is a per-reference resolution shorthand, and two notes may
//! legitimately share one.
//!
//! **The declared kind decides what a value means.** Every scalar is read as
//! its bytes and validated against the kind its declaration names; nothing is
//! coerced, and the parser never guesses. That is why `NO` is a string rather
//! than a boolean, and why `1` satisfies `integer` and not `float`.

mod body;
mod findings;
mod frontmatter;
mod index;
mod lexical;
mod links;
mod model;
mod read;
mod resolve;
mod traverse;
mod validate;
mod values;

use crate::contract::Contract;
use crate::diagnostic::{Diagnostic, DiagnosticList, VaultPath};
use crate::vault::VaultRoot;

pub use model::{
    Binding, Edge, FieldValue, Note, Property, PropertyValue, RecordValue, Reference, Relationship,
};
pub use resolve::UnresolvedReference;
pub use traverse::{Traversal, traverse};

/// Every note in a vault, read against its contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Corpus {
    notes: Vec<Note>,
    diagnostics: Vec<Diagnostic>,
}

impl Corpus {
    /// Every note that could be read, in vault-relative path order.
    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    /// The note at a vault-relative path.
    pub fn note(&self, path: &VaultPath) -> Option<&Note> {
        self.notes.iter().find(|note| &note.path == path)
    }

    /// The note a reference names.
    ///
    /// One rule, at every door — the rule this corpus's own typed links
    /// resolved by, and the rule a reference handed in from outside obeys:
    ///
    /// - a reference containing no `/` is a **bare name**, and resolves iff
    ///   exactly one note bears it;
    /// - a reference containing a `/` is **path-qualified**, resolved against
    ///   the vault root, with `.md` appended when it is absent.
    ///
    /// Nothing here touches the filesystem: a reference is matched against the
    /// notes this corpus holds, so no spelling reaches outside the vault.
    ///
    /// # Errors
    ///
    /// [`UnresolvedReference`] when no note bears the reference, and when a
    /// bare name is one several notes bear — which is a defect of the
    /// reference rather than of the corpus, so it carries every candidate.
    pub fn resolve(&self, reference: &str) -> Result<&Note, UnresolvedReference> {
        resolve::reference(&self.notes, reference)
    }

    /// Everything reading the corpus reported, in the deterministic total
    /// order.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Reads every note under `root` against `contract`.
///
/// Semantic operations take a **resolved** contract and cannot be reached
/// without one, so reading a corpus against rules that did not resolve is not
/// expressible here.
///
/// This never fails. A note that cannot be read is a diagnostic against that
/// note and the rest of the corpus is read anyway: one unreadable note must not
/// make a corpus unreadable.
pub fn read_corpus(root: &VaultRoot, contract: &Contract) -> Corpus {
    let traversal = traverse(root);
    let mut diagnostics = DiagnosticList::new();
    diagnostics.extend(traversal.diagnostics().iter().cloned());
    let mut notes = Vec::new();
    for path in traversal.notes() {
        let read = read::note(&root.path().join(path.as_str()), path, contract);
        notes.extend(read.note);
        diagnostics.extend(read.diagnostics);
    }
    // Resolution runs last, because it is the one question a single note cannot
    // answer: which note a reference names is a fact about the whole corpus.
    diagnostics.extend(resolve::corpus(&mut notes, contract.dialect().links()));
    Corpus {
        notes,
        diagnostics: diagnostics.sorted(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::parse_contract;
    use crate::vault::{SENTINEL, tree::Tree};
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A contract exercising every shape a note can be held to.
    const CONTRACT: &str = concat!(
        "contract_version = 2\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[tags]\nproperty = \"labels\"\n",
        "\n[[type]]\nname = \"person\"\ncapabilities = [\"identity-bearing\"]\n",
        "\n  [[type.property]]\n  name = \"full_name\"\n  kind = \"string\"\n  required = true\n",
        "\n  [[type.property]]\n  name = \"status\"\n  kind = \"enum\"\n",
        "  values = [\"draft\", \"archived\"]\n",
        "\n  [[type.property]]\n  name = \"born_on\"\n  kind = \"date\"\n",
        "\n  [[type.property]]\n  name = \"labels\"\n  kind = \"list\"\n  of = \"string\"\n",
        "\n  [[type.property]]\n  name = \"scores\"\n  kind = \"list\"\n  of = \"integer\"\n",
        "\n  [[type.property]]\n  name = \"legal_name\"\n  kind = \"record\"\n",
        "\n    [[type.property.field]]\n    name = \"given\"\n    kind = \"string\"\n",
        "    required = true\n",
        "\n    [[type.property.field]]\n    name = \"family\"\n    kind = \"string\"\n",
        "\n    [[type.property.field]]\n    name = \"usage\"\n    kind = \"enum\"\n",
        "    values = [\"legal\", \"preferred\"]\n",
        "\n  [[type.property]]\n  name = \"waypoints\"\n  kind = \"list\"\n  of = \"record\"\n",
        "\n    [[type.property.field]]\n    name = \"caption\"\n    kind = \"string\"\n",
        "    required = true\n",
        "\n    [[type.property.field]]\n    name = \"reached_on\"\n    kind = \"date\"\n",
        "\n  [[type.relationship]]\n  predicate = \"works-at\"\n  required = true\n",
        "\n  [[type.relationship]]\n  predicate = \"mentions\"\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    /// A contract that declares no tag vocabulary at all.
    const UNTAGGED: &str = concat!(
        "contract_version = 2\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[[type]]\nname = \"person\"\ncapabilities = [\"identity-bearing\"]\n",
        "\n  [[type.property]]\n  name = \"labels\"\n  kind = \"list\"\n  of = \"string\"\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    /// A contract whose one identity-bearing type describes its own tagging.
    const NAMESPACED: &str = concat!(
        "contract_version = 2\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[tags]\nproperty = \"labels\"\n",
        "\n[[type]]\nname = \"person\"\ncapabilities = [\"identity-bearing\"]\n",
        "\n  [[type.property]]\n  name = \"labels\"\n  kind = \"list\"\n  of = \"string\"\n",
        "\n  [[type.tag-namespace]]\n  prefix = \"role/\"\n  required = true\n",
        "  values = [\"founder\", \"advisor\"]\n",
        "\n  [[type.tag-namespace]]\n  prefix = \"topic/\"\n  open = true\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    /// A note of the `person` type that satisfies every rule the type states.
    const CONFORMING: &str = concat!(
        "---\n",
        "type: person\n",
        "full_name: Ada Lovelace\n",
        "status: draft\n",
        "born_on: 1815-12-10\n",
        "labels: [role/founder, topic/computing]\n",
        "legal_name:\n  given: Augusta\n  family: Lovelace\n  usage: legal\n",
        "waypoints:\n  - caption: first program\n    reached_on: 1843-01-01\n",
        "works-at: \"[[engine]]\"\n",
        "---\n",
        "# Ada Lovelace\n\nprose\n",
    );

    /// A contract whose corpus writes its links the other way.
    const MARKDOWN: &str = concat!(
        "contract_version = 2\n",
        "\n[dialect]\nlinks = \"markdown\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[[type]]\nname = \"person\"\ncapabilities = [\"identity-bearing\"]\n",
        "\n  [[type.relationship]]\n  predicate = \"works-at\"\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    /// The notes the fixtures link to, held by every corpus these tests build.
    ///
    /// A typed link must resolve, so a corpus that claims a relationship holds
    /// the note it claims one with. Standing targets are what keep a test about
    /// a note's own structure from also being a test about a dangling link.
    const TARGETS: &[(&str, &str)] = &[
        ("engine.md", "# The Analytical Engine\n"),
        ("society.md", "# The Analytical Society\n"),
    ];

    /// A directory name no other call in this process will pick.
    fn next_name() -> String {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        format!("corpus-{}", COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// A vault of its own, holding `notes`, read against `CONTRACT`.
    fn read(tree: &Tree, notes: &[(&str, &str)]) -> Corpus {
        read_against(tree, CONTRACT, notes)
    }

    fn read_against(tree: &Tree, contract: &str, notes: &[(&str, &str)]) -> Corpus {
        let root = tree.vault(&next_name());
        fs::write(root.join(SENTINEL), contract).expect("a contract this test owns");
        for (relative, body) in TARGETS.iter().chain(notes.iter()) {
            write(&root.join(relative), body.as_bytes());
        }
        let load = parse_contract(contract);
        let resolved = load.contract.expect("a conforming contract");
        read_corpus(&VaultRoot::new(root), &resolved)
    }

    fn write(path: &std::path::Path, bytes: &[u8]) {
        let parent = path.parent().expect("a note under the root has a parent");
        fs::create_dir_all(parent).expect("a directory this test owns");
        fs::write(path, bytes).expect("a note this test owns");
    }

    fn ids(corpus: &Corpus) -> Vec<&str> {
        corpus
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect()
    }

    /// The corpus's one note under test, which is every note but the standing
    /// link targets.
    fn only(corpus: &Corpus) -> &Note {
        let under_test: Vec<&Note> = corpus
            .notes()
            .iter()
            .filter(|note| {
                !TARGETS
                    .iter()
                    .any(|(path, _)| *path == note.path().as_str())
            })
            .collect();
        assert_eq!(under_test.len(), 1, "exactly one note under test");
        under_test[0]
    }

    /// The corpus's note at a vault-relative path, which the test wrote.
    fn corpus_note<'a>(corpus: &'a Corpus, path: &str) -> &'a Note {
        corpus
            .notes()
            .iter()
            .find(|note| note.path().as_str() == path)
            .expect("a note this test wrote")
    }

    /// Every path the corpus holds, in the order it answers them.
    fn paths(corpus: &Corpus) -> Vec<&str> {
        corpus
            .notes()
            .iter()
            .map(|note| note.path().as_str())
            .collect()
    }

    /// The corpus's one note, and its one diagnostic's message.
    fn one_finding<'a>(corpus: &'a Corpus, id: &str) -> &'a str {
        let reported = ids(corpus);
        assert_eq!(reported, [id]);
        corpus.diagnostics()[0].message.as_str()
    }

    #[test]
    fn a_conforming_corpus_reads_with_nothing_at_all_to_report() {
        let tree = Tree::new("corpus-clean");
        let corpus = read(&tree, &[("people/ada.md", CONFORMING)]);
        let reported = ids(&corpus);
        assert!(reported.is_empty(), "{reported:?}");
        let note = only(&corpus);
        assert_eq!(note.path().as_str(), "people/ada.md");
        assert_eq!(note.binding().type_name(), Some("person"));
        assert_eq!(note.binding().bound_by(), "declaration");
        assert_eq!(note.title(), Some("Ada Lovelace"));
        assert_eq!(note.body(), "# Ada Lovelace\n\nprose\n");
    }

    #[test]
    fn a_conforming_note_carries_every_shape_of_value_as_the_bytes_it_wrote() {
        let tree = Tree::new("corpus-values");
        let corpus = read(&tree, &[("ada.md", CONFORMING)]);
        let note = only(&corpus);
        assert_eq!(
            note.property("full_name").and_then(PropertyValue::scalar),
            Some("Ada Lovelace")
        );
        assert_eq!(
            note.property("labels").and_then(PropertyValue::list),
            Some(&["role/founder".to_owned(), "topic/computing".to_owned()][..])
        );
        let legal = note.property("legal_name").expect("a record").records();
        assert_eq!(legal[0].field("given"), Some("Augusta"));
        assert_eq!(legal[0].field("usage"), Some("legal"));
        let waypoints = note.property("waypoints").expect("a list").records();
        assert_eq!(waypoints[0].field("caption"), Some("first program"));
        assert_eq!(waypoints[0].field("reached_on"), Some("1843-01-01"));
        assert_eq!(note.tags(), ["role/founder", "topic/computing"]);
    }

    #[test]
    fn an_edge_carries_the_reference_as_written_and_the_note_it_resolved_to() {
        let tree = Tree::new("corpus-edges");
        let corpus = read(&tree, &[("ada.md", CONFORMING)]);
        let edges = only(&corpus).relationship("works-at").expect("declared");
        assert_eq!(
            edges[0].written(),
            "[[engine]]",
            "the reference is the note's own bytes, delimiters included"
        );
        assert_eq!(
            edges[0].target().map(VaultPath::as_str),
            Some("engine.md"),
            "and the target is the note identity it names"
        );
    }

    #[test]
    fn a_note_with_no_frontmatter_belongs_to_the_catch_all_type() {
        let tree = Tree::new("corpus-untyped");
        let corpus = read(&tree, &[("inbox.md", "# A thought\n")]);
        assert!(corpus.diagnostics().is_empty());
        let note = only(&corpus);
        assert_eq!(note.binding().type_name(), Some("capture"));
        assert_eq!(note.binding().bound_by(), "catch-all");
        assert_eq!(note.binding().discriminator(), None);
    }

    #[test]
    fn frontmatter_without_a_type_key_belongs_to_the_catch_all_too() {
        let tree = Tree::new("corpus-no-discriminator");
        let corpus = read(&tree, &[("inbox.md", "---\nnote: a thought\n---\n")]);
        let note = only(&corpus);
        assert_eq!(note.binding().bound_by(), "catch-all");
        assert_eq!(
            ids(&corpus),
            ["note.undeclared-property"],
            "the catch-all declares nothing, so the key it wrote is undeclared"
        );
    }

    #[test]
    fn a_type_naming_nothing_the_contract_declares_is_reported_and_binds_to_nothing() {
        let tree = Tree::new("corpus-unknown-type");
        let note = "---\ntype: persno\nfull_name: Ada\nstray: one\n---\n";
        let corpus = read(&tree, &[("ada.md", note)]);
        let message = one_finding(&corpus, "note.unknown-type");
        assert!(message.contains("`persno`"), "{message}");
        let read = only(&corpus);
        assert_eq!(read.binding().type_name(), None);
        assert_eq!(read.binding().bound_by(), "none");
        assert_eq!(
            read.binding().discriminator(),
            Some("persno"),
            "the note is shown what it said, rather than reclassified"
        );
    }

    #[test]
    fn a_type_that_is_not_a_scalar_is_reported_and_never_falls_back_to_the_catch_all() {
        let tree = Tree::new("corpus-type-invalid");
        let corpus = read(&tree, &[("ada.md", "---\ntype: [person]\n---\n")]);
        let message = one_finding(&corpus, "note.type-invalid");
        assert!(message.contains("a sequence"), "{message}");
        assert_eq!(only(&corpus).binding().bound_by(), "none");
    }

    #[test]
    fn a_note_bound_to_nothing_collects_no_further_finding() {
        // Every remaining rule reads a declaration, and there is none to read:
        // the missing required property, the undeclared key and the missing
        // relationship all stay unsaid.
        let tree = Tree::new("corpus-suppressed");
        let note = "---\ntype: persno\nstray: one\nborn_on: not-a-date\n---\n";
        let corpus = read(&tree, &[("ada.md", note)]);
        assert_eq!(ids(&corpus), ["note.unknown-type"]);
    }

    #[test]
    fn an_undeclared_key_is_info_per_key_and_points_at_the_key_it_names() {
        let tree = Tree::new("corpus-undeclared");
        let note = concat!(
            "---\ntype: person\nfull_name: Ada\nworks-at: \"[[engine]]\"\n",
            "created: 2026-08-03\nupdated: 2026-08-03\n---\n",
        );
        let corpus = read(&tree, &[("ada.md", note)]);
        assert_eq!(
            ids(&corpus),
            ["note.undeclared-property", "note.undeclared-property"]
        );
        let first = &corpus.diagnostics()[0];
        assert_eq!(first.severity, crate::diagnostic::Severity::Info);
        assert!(first.message.contains("`created`"), "{}", first.message);
        let span = first
            .location
            .as_ref()
            .expect("located")
            .span
            .expect("spanned");
        assert_eq!((span.start.line, span.start.column), (5, 1));
    }

    #[test]
    fn a_missing_required_property_names_it_and_points_at_the_note_itself() {
        let tree = Tree::new("corpus-missing");
        let note = "---\ntype: person\nworks-at: \"[[engine]]\"\n---\n";
        let corpus = read(&tree, &[("ada.md", note)]);
        let message = one_finding(&corpus, "note.missing-required-property");
        assert!(message.contains("`full_name`"), "{message}");
        let location = corpus.diagnostics()[0].location.as_ref().expect("located");
        assert!(
            location.span.is_none(),
            "an absence has no bytes to point at"
        );
        let evidence = &corpus.diagnostics()[0].related[0];
        assert!(evidence.message.contains("required"));
        assert!(
            evidence.location.is_some(),
            "the requirement is written in the contract, and that is where it points"
        );
    }

    #[test]
    fn a_required_relationship_with_no_edge_is_reported_against_the_note() {
        let tree = Tree::new("corpus-missing-edge");
        let note = "---\ntype: person\nfull_name: Ada\nworks-at:\n---\n";
        let corpus = read(&tree, &[("ada.md", note)]);
        let message = one_finding(&corpus, "note.missing-required-relationship");
        assert!(message.contains("`works-at`"), "{message}");
        assert_eq!(only(&corpus).relationship("works-at"), Some(&[][..]));
    }

    #[test]
    fn a_relationship_value_that_is_not_a_link_is_reported() {
        let tree = Tree::new("corpus-edge-shape");
        let note = "---\ntype: person\nfull_name: Ada\nworks-at:\n  at: Engine\n---\n";
        let corpus = read(&tree, &[("ada.md", note)]);
        assert_eq!(
            ids(&corpus),
            [
                "note.missing-required-relationship",
                "note.relationship-value-invalid"
            ]
        );
    }

    #[test]
    fn a_value_that_fails_its_kinds_lexical_form_names_the_kind_and_the_span() {
        let tree = Tree::new("corpus-lexical");
        let note = concat!(
            "---\ntype: person\nfull_name: Ada\nworks-at: \"[[engine]]\"\n",
            "born_on: 1815-12-1\nstatus: published\n---\n",
        );
        let corpus = read(&tree, &[("ada.md", note)]);
        assert_eq!(
            ids(&corpus),
            ["note.property-kind-invalid", "note.property-kind-invalid"]
        );
        assert!(corpus.diagnostics()[0].message.contains("`date`"));
        assert!(corpus.diagnostics()[1].message.contains("draft, archived"));
        assert_eq!(
            only(&corpus)
                .property("born_on")
                .and_then(PropertyValue::scalar),
            Some("1815-12-1"),
            "the model says what the note wrote, and the diagnostic says what is wrong with it"
        );
    }

    #[test]
    fn a_value_whose_shape_the_kind_cannot_hold_leaves_the_property_absent() {
        let tree = Tree::new("corpus-shape");
        let note = concat!(
            "---\ntype: person\nfull_name: Ada\nworks-at: \"[[engine]]\"\n",
            "labels: one\nlegal_name: Augusta\n---\n",
        );
        let corpus = read(&tree, &[("ada.md", note)]);
        assert_eq!(
            ids(&corpus),
            ["note.property-kind-invalid", "note.property-kind-invalid"]
        );
        let read = only(&corpus);
        assert!(read.property("labels").is_none());
        assert!(read.property("legal_name").is_none());
    }

    #[test]
    fn a_records_fields_validate_exactly_as_properties_do_under_the_field_path() {
        let tree = Tree::new("corpus-record");
        let note = concat!(
            "---\ntype: person\nfull_name: Ada\nworks-at: \"[[engine]]\"\n",
            "legal_name:\n  family: Lovelace\n  middle: Augusta\n---\n",
        );
        let corpus = read(&tree, &[("ada.md", note)]);
        assert_eq!(
            ids(&corpus),
            ["note.missing-required-property", "note.undeclared-property"]
        );
        let missing = &corpus.diagnostics()[0].message;
        assert!(missing.contains("`legal_name.given`"), "{missing}");
        assert!(corpus.diagnostics()[1].message.contains("`legal_name`"));
    }

    #[test]
    fn every_element_of_a_list_of_records_is_held_to_the_same_fields() {
        let tree = Tree::new("corpus-record-list");
        let note = concat!(
            "---\ntype: person\nfull_name: Ada\nworks-at: \"[[engine]]\"\n",
            "waypoints:\n  - caption: first\n    reached_on: 1843-1-1\n  - reached_on: 1843-01-02\n",
            "---\n",
        );
        let corpus = read(&tree, &[("ada.md", note)]);
        assert_eq!(
            ids(&corpus),
            [
                "note.missing-required-property",
                "note.property-kind-invalid"
            ]
        );
        let missing = &corpus.diagnostics()[0].message;
        assert!(missing.contains("`waypoints[1].caption`"), "{missing}");
        assert!(
            corpus.diagnostics()[1]
                .message
                .contains("`waypoints[0].reached_on`")
        );
    }

    #[test]
    fn an_element_of_a_list_is_held_to_the_element_kind_under_its_index() {
        let tree = Tree::new("corpus-list-element");
        let note = concat!(
            "---\ntype: person\nfull_name: Ada\nworks-at: \"[[engine]]\"\n",
            "labels:\n  - role/founder\n  - one: two\n",
            "waypoints:\n  - one\n---\n",
        );
        let corpus = read(&tree, &[("ada.md", note)]);
        assert_eq!(
            ids(&corpus),
            ["note.property-kind-invalid", "note.property-kind-invalid"]
        );
        assert!(corpus.diagnostics()[0].message.contains("`labels[1]`"));
        assert!(corpus.diagnostics()[1].message.contains("`waypoints[0]`"));
    }

    #[test]
    fn a_sequence_of_links_is_read_as_one_edge_each() {
        let tree = Tree::new("corpus-edge-list");
        let note = concat!(
            "---\ntype: person\nfull_name: Ada\n",
            "works-at:\n  - \"[[engine]]\"\n  - \"[[society]]\"\n---\n",
        );
        let corpus = read(&tree, &[("ada.md", note)]);
        let reported = ids(&corpus);
        assert!(reported.is_empty(), "{reported:?}");
        let edges = only(&corpus).relationship("works-at").expect("declared");
        let written: Vec<&str> = edges.iter().map(Edge::written).collect();
        assert_eq!(written, ["[[engine]]", "[[society]]"]);
        let targets: Vec<&str> = edges
            .iter()
            .filter_map(|edge| edge.target().map(VaultPath::as_str))
            .collect();
        assert_eq!(targets, ["engine.md", "society.md"]);
    }

    #[test]
    fn a_sequence_holding_something_that_is_not_a_link_is_reported() {
        let tree = Tree::new("corpus-edge-list-shape");
        let note = concat!(
            "---\ntype: person\nfull_name: Ada\n",
            "works-at:\n  - at: Engine\n---\n",
        );
        let corpus = read(&tree, &[("ada.md", note)]);
        assert_eq!(
            ids(&corpus),
            [
                "note.missing-required-relationship",
                "note.relationship-value-invalid"
            ]
        );
    }

    #[test]
    fn a_predicate_the_note_never_writes_carries_no_edge_at_all() {
        let tree = Tree::new("corpus-edge-absent");
        let note = "---\ntype: person\nfull_name: Ada\n---\n";
        let corpus = read(&tree, &[("ada.md", note)]);
        assert_eq!(ids(&corpus), ["note.missing-required-relationship"]);
        assert_eq!(only(&corpus).relationship("works-at"), Some(&[][..]));
    }

    #[test]
    fn an_optional_relationship_with_no_edge_is_not_a_finding() {
        let tree = Tree::new("corpus-edge-optional");
        let corpus = read_against(
            &tree,
            UNTAGGED,
            &[("ada.md", "---\ntype: person\nlabels: [one]\n---\n")],
        );
        let reported = ids(&corpus);
        assert!(reported.is_empty(), "{reported:?}");
        assert!(
            only(&corpus).tags().is_empty(),
            "a corpus that declares no tag vocabulary has no tags to report"
        );
    }

    #[test]
    fn a_scalar_kind_meeting_a_collection_leaves_the_property_absent() {
        let tree = Tree::new("corpus-scalar-shape");
        let note = concat!(
            "---\ntype: person\nfull_name: Ada\nworks-at: \"[[engine]]\"\n",
            "born_on: [1815-12-10]\n---\n",
        );
        let corpus = read(&tree, &[("ada.md", note)]);
        assert_eq!(ids(&corpus), ["note.property-kind-invalid"]);
        assert!(only(&corpus).property("born_on").is_none());
    }

    #[test]
    fn a_list_element_is_held_to_its_element_kinds_lexical_form() {
        let tree = Tree::new("corpus-list-lexical");
        let note = concat!(
            "---\ntype: person\nfull_name: Ada\nworks-at: \"[[engine]]\"\n",
            "scores: [1, one]\n---\n",
        );
        let corpus = read(&tree, &[("ada.md", note)]);
        let message = one_finding(&corpus, "note.property-kind-invalid");
        assert!(message.contains("`scores[1]`"), "{message}");
        assert!(message.contains("`integer`"), "{message}");
    }

    #[test]
    fn a_records_optional_field_may_simply_be_absent() {
        let tree = Tree::new("corpus-record-optional");
        let note = concat!(
            "---\ntype: person\nfull_name: Ada\nworks-at: \"[[engine]]\"\n",
            "legal_name:\n  given: Augusta\n  usage: nickname\n---\n",
        );
        let corpus = read(&tree, &[("ada.md", note)]);
        let message = one_finding(&corpus, "note.property-kind-invalid");
        assert!(message.contains("`legal_name.usage`"), "{message}");
        let record = &only(&corpus)
            .property("legal_name")
            .expect("a record")
            .records()[0];
        assert_eq!(record.field("family"), None);
        assert_eq!(record.field("given"), Some("Augusta"));
    }

    /// A corpus of one `person` note carrying `labels`, read against the
    /// contract that describes its tagging.
    fn tagged(tree: &Tree, labels: &str) -> Corpus {
        let note = format!("---\ntype: person\nlabels: {labels}\n---\n");
        read_against(tree, NAMESPACED, &[("ada.md", &note)])
    }

    #[test]
    fn a_required_namespace_with_no_tag_in_it_points_at_the_tags_the_note_wrote() {
        let tree = Tree::new("corpus-namespace-missing");
        let corpus = tagged(&tree, "[topic/computing]");
        let message = one_finding(&corpus, "note.required-namespace-missing");
        assert!(message.contains("`role/`"), "{message}");
        let span = corpus.diagnostics()[0]
            .location
            .as_ref()
            .expect("located")
            .span
            .expect("spanned");
        assert_eq!((span.start.line, span.start.column), (3, 9));
    }

    #[test]
    fn a_note_carrying_no_tags_at_all_is_told_so_against_the_note_itself() {
        let tree = Tree::new("corpus-namespace-untagged");
        let corpus = read_against(&tree, NAMESPACED, &[("ada.md", "---\ntype: person\n---\n")]);
        let message = one_finding(&corpus, "note.required-namespace-missing");
        assert!(message.contains("`role/`"), "{message}");
        let location = corpus.diagnostics()[0].location.as_ref().expect("located");
        assert!(
            location.span.is_none(),
            "an absence has no bytes to point at"
        );
        assert!(
            corpus.diagnostics()[0].related[0]
                .message
                .contains("required")
        );
    }

    #[test]
    fn a_tag_outside_a_closed_namespaces_vocabulary_is_reported_against_the_tag() {
        let tree = Tree::new("corpus-namespace-vocabulary");
        let corpus = tagged(&tree, "[role/chair]");
        let message = one_finding(&corpus, "note.tag-outside-vocabulary");
        assert!(message.contains("`role/chair`"), "{message}");
    }

    #[test]
    fn an_open_namespace_bounds_nothing_and_an_undeclared_prefix_is_untouched() {
        let tree = Tree::new("corpus-namespace-open");
        let corpus = tagged(
            &tree,
            "[role/advisor, topic/anything-at-all, other/entirely]",
        );
        let reported = ids(&corpus);
        assert!(reported.is_empty(), "{reported:?}");
        assert_eq!(
            only(&corpus).tags(),
            ["role/advisor", "topic/anything-at-all", "other/entirely"],
            "tags are content: what no namespace describes still reaches the model"
        );
    }

    #[test]
    fn every_namespace_is_evaluated_independently_of_every_other() {
        let tree = Tree::new("corpus-namespace-independent");
        let corpus = tagged(&tree, "[role/chair, role/founder]");
        assert_eq!(
            ids(&corpus),
            ["note.tag-outside-vocabulary"],
            "one tag satisfies the requirement while another is outside the vocabulary"
        );
    }

    #[test]
    fn a_note_whose_bytes_are_not_text_is_reported_and_the_corpus_still_reads() {
        let tree = Tree::new("corpus-unreadable");
        let root = tree.vault(&next_name());
        fs::write(root.join(SENTINEL), CONTRACT).expect("a contract this test owns");
        for (relative, body) in TARGETS {
            write(&root.join(relative), body.as_bytes());
        }
        write(&root.join("ada.md"), CONFORMING.as_bytes());
        write(&root.join("broken.md"), b"# Title \xff\n");
        let load = parse_contract(CONTRACT);
        let contract = load.contract.expect("a conforming contract");
        let corpus = read_corpus(&VaultRoot::new(root), &contract);
        assert_eq!(ids(&corpus), ["note.unreadable"]);
        assert_eq!(
            corpus.notes().len(),
            3,
            "one unreadable note is not a corpus"
        );
        assert!(corpus.diagnostics()[0].message.contains("valid UTF-8"));
    }

    #[test]
    fn a_byte_order_mark_and_a_carriage_return_are_warnings_and_the_note_is_read() {
        let tree = Tree::new("corpus-encoding");
        let note =
            "\u{feff}---\r\ntype: person\r\nfull_name: Ada\r\nworks-at: \"[[engine]]\"\r\n---\r\n";
        let corpus = read(&tree, &[("ada.md", note)]);
        assert_eq!(
            ids(&corpus),
            ["note.byte-order-mark", "note.carriage-return-line-ending"],
            "both, because they are independent facts about the file"
        );
        for diagnostic in corpus.diagnostics() {
            assert_eq!(diagnostic.severity, crate::diagnostic::Severity::Warning);
        }
        assert_eq!(only(&corpus).binding().type_name(), Some("person"));
    }

    #[test]
    fn a_refused_frontmatter_construct_leaves_the_note_bound_to_nothing() {
        let tree = Tree::new("corpus-refused");
        let note = "---\ntype: person\nalias: *base\n---\n# Ada\n";
        let corpus = read(&tree, &[("ada.md", note)]);
        let message = one_finding(&corpus, "note.frontmatter-unsupported");
        assert!(message.contains("an alias"), "{message}");
        let read = only(&corpus);
        assert_eq!(
            read.binding().bound_by(),
            "none",
            "a block that failed to load is not a note with no frontmatter"
        );
        assert_eq!(read.title(), Some("Ada"));
    }

    #[test]
    fn a_refused_construct_written_in_brackets_is_refused_just_the_same() {
        // Reading the alias as the literal tag `*base` would be the silent
        // reinterpretation the hand-written subset exists to prevent, and a
        // note that never loaded is not a note carrying a tag nobody declared.
        let tree = Tree::new("corpus-refused-flow");
        let note = "---\ntype: person\nlabels: [role/founder, *base]\n---\n";
        let corpus = read_against(&tree, NAMESPACED, &[("ada.md", note)]);
        let message = one_finding(&corpus, "note.frontmatter-unsupported");
        assert!(message.contains("an alias"), "{message}");
        let read = only(&corpus);
        assert!(read.tags().is_empty(), "{:?}", read.tags());
        assert_eq!(read.binding().bound_by(), "none");
    }

    #[test]
    fn a_record_field_written_twice_in_brackets_is_refused_rather_than_last_wins() {
        let tree = Tree::new("corpus-refused-flow-repeat");
        let note = concat!(
            "---\ntype: person\nfull_name: Ada\nworks-at: \"[[engine]]\"\n",
            "legal_name: {given: Augusta, given: Ada}\n---\n",
        );
        let corpus = read(&tree, &[("ada.md", note)]);
        let message = one_finding(&corpus, "note.frontmatter-unsupported");
        assert!(message.contains("written twice"), "{message}");
    }

    #[test]
    fn a_frontmatter_block_that_is_not_the_grammar_is_reported_as_invalid() {
        let tree = Tree::new("corpus-invalid");
        let corpus = read(&tree, &[("ada.md", "---\nnot an entry\n---\n")]);
        let message = one_finding(&corpus, "note.frontmatter-invalid");
        assert!(message.contains("`key: value`"), "{message}");
    }

    #[test]
    fn tags_come_from_the_declared_tag_property_and_only_where_the_type_declares_it() {
        let tree = Tree::new("corpus-tags");
        let corpus = read(&tree, &[("inbox.md", "---\nlabels: [role/founder]\n---\n")]);
        let note = only(&corpus);
        assert_eq!(note.binding().type_name(), Some("capture"));
        assert!(
            note.tags().is_empty(),
            "the catch-all declares no tag property, so the key is undeclared rather than tags"
        );
        assert_eq!(ids(&corpus), ["note.undeclared-property"]);
    }

    #[test]
    fn a_corpus_answers_in_path_order_and_its_diagnostics_in_the_total_order() {
        let tree = Tree::new("corpus-order");
        let stray =
            "---\ntype: person\nfull_name: Ada\nworks-at: \"[[engine]]\"\nstray: one\n---\n";
        let corpus = read(
            &tree,
            &[("z.md", stray), ("a.md", stray), ("m/b.md", CONFORMING)],
        );
        assert_eq!(
            paths(&corpus),
            ["a.md", "engine.md", "m/b.md", "society.md", "z.md"]
        );
        let located: Vec<&str> = corpus
            .diagnostics()
            .iter()
            .map(|diagnostic| {
                diagnostic
                    .location
                    .as_ref()
                    .expect("located")
                    .file
                    .display_path()
            })
            .collect();
        assert_eq!(located, ["a.md", "z.md"]);
    }

    #[test]
    fn a_folder_note_and_its_folder_answer_in_one_order_rather_than_two() {
        // `notes()` and `diagnostics()` are two views of one corpus, and a
        // folder note is where the walk's order and the paths' order diverge:
        // the walk descends into `projects/` before it reaches `projects.md`,
        // while `.` precedes `/` so the paths run the other way.
        let tree = Tree::new("corpus-folder-note");
        let stray =
            "---\ntype: person\nfull_name: Ada\nworks-at: \"[[engine]]\"\nstray: one\n---\n";
        let corpus = read(
            &tree,
            &[("projects/alpha.md", stray), ("projects.md", stray)],
        );
        let located: Vec<&str> = corpus
            .diagnostics()
            .iter()
            .map(|diagnostic| {
                diagnostic
                    .location
                    .as_ref()
                    .expect("located")
                    .file
                    .display_path()
            })
            .collect();
        assert_eq!(
            paths(&corpus),
            [
                "engine.md",
                "projects.md",
                "projects/alpha.md",
                "society.md"
            ]
        );
        assert_eq!(
            located,
            ["projects.md", "projects/alpha.md"],
            "one order, whichever accessor is asked"
        );
    }

    /// A `person` note whose one `works-at` edge is `reference`.
    fn linking(reference: &str) -> String {
        format!("---\ntype: person\nfull_name: Ada\nworks-at: {reference}\n---\n")
    }

    #[test]
    fn a_typed_link_naming_no_note_is_reported_against_the_reference_itself() {
        let tree = Tree::new("corpus-dangling");
        let corpus = read(&tree, &[("ada.md", &linking("\"[[difference]]\""))]);
        let message = one_finding(&corpus, "link.dangling-typed-link");
        assert!(message.contains("`difference`"), "{message}");
        assert!(message.contains("must resolve"), "{message}");
        let location = corpus.diagnostics()[0].location.as_ref().expect("located");
        assert_eq!(location.file.display_path(), "ada.md");
        let span = location.span.expect("the reference has bytes to point at");
        assert_eq!((span.start.line, span.start.column), (4, 11));
        let edge = &only(&corpus).relationship("works-at").expect("declared")[0];
        assert_eq!(edge.target(), None, "an edge that resolved to nothing");
    }

    #[test]
    fn a_path_qualified_link_resolves_with_or_without_the_extension() {
        let tree = Tree::new("corpus-qualified");
        for reference in [
            "\"[[engines/analytical]]\"",
            "\"[[engines/analytical.md]]\"",
        ] {
            let corpus = read_against(
                &tree,
                CONTRACT,
                &[
                    ("people/ada.md", &linking(reference)),
                    ("engines/analytical.md", "# The Analytical Engine\n"),
                ],
            );
            let reported = ids(&corpus);
            assert!(reported.is_empty(), "{reference}: {reported:?}");
            let note = corpus_note(&corpus, "people/ada.md");
            let edge = &note.relationship("works-at").expect("declared")[0];
            assert_eq!(
                edge.target().map(VaultPath::as_str),
                Some("engines/analytical.md")
            );
        }
    }

    #[test]
    fn a_reference_with_no_slash_is_a_bare_name_even_when_it_carries_the_extension() {
        // The two halves of the rule read the reference, never the corpus: a
        // reference with no `/` is a name, and `engine.md` is not the name any
        // note bears — `engine` is. Appending the extension is the
        // path-qualified half's business.
        let tree = Tree::new("corpus-bare-extension");
        let corpus = read(&tree, &[("ada.md", &linking("\"[[engine.md]]\""))]);
        assert_eq!(ids(&corpus), ["link.dangling-typed-link"]);
    }

    #[test]
    fn a_bare_name_two_notes_bear_is_reported_against_the_link_with_both_candidates() {
        // Ambiguity is a defect of the link, not of the corpus: the corpus is
        // read, and every candidate comes back so the reference can be fixed.
        let tree = Tree::new("corpus-ambiguous");
        let corpus = read(
            &tree,
            &[
                ("ada.md", &linking("\"[[daily]]\"")),
                ("2025/daily.md", "# Daily\n"),
                ("2026/daily.md", "# Daily\n"),
            ],
        );
        let message = one_finding(&corpus, "link.ambiguous-reference");
        assert!(message.contains("`daily`"), "{message}");
        let evidence: Vec<&str> = corpus.diagnostics()[0]
            .related
            .iter()
            .map(|related| {
                related
                    .location
                    .as_ref()
                    .expect("located")
                    .file
                    .display_path()
            })
            .collect();
        assert_eq!(evidence, ["2025/daily.md", "2026/daily.md"]);
    }

    #[test]
    fn two_notes_sharing_a_name_that_nothing_references_is_not_a_finding() {
        let tree = Tree::new("corpus-shared-name");
        let corpus = read(
            &tree,
            &[
                ("2025/daily.md", "# Daily\n"),
                ("2026/daily.md", "# Daily\n"),
            ],
        );
        let reported = ids(&corpus);
        assert!(reported.is_empty(), "{reported:?}");
    }

    #[test]
    fn a_link_written_in_a_dialect_the_contract_does_not_declare_names_no_note() {
        // The dialect is the contract's, corpus-wide, and never sniffed per
        // link — so the other dialect's spelling is bytes rather than a link.
        let tree = Tree::new("corpus-dialect");
        let note = |reference: &str| format!("---\ntype: person\nworks-at: {reference}\n---\n");
        let engine = ("engines/analytical.md", "# The Analytical Engine\n");
        let resolved = read_against(
            &tree,
            MARKDOWN,
            &[
                ("ada.md", &note("\"[The Engine](engines/analytical.md)\"")),
                engine,
            ],
        );
        assert!(ids(&resolved).is_empty(), "{:?}", ids(&resolved));
        let ada = corpus_note(&resolved, "ada.md");
        let edge = &ada.relationship("works-at").expect("declared")[0];
        assert_eq!(
            edge.target().map(VaultPath::as_str),
            Some("engines/analytical.md")
        );
        let crossed = read_against(
            &tree,
            MARKDOWN,
            &[("ada.md", &note("\"[[engines/analytical]]\"")), engine],
        );
        assert_eq!(ids(&crossed), ["link.dangling-typed-link"]);
    }

    #[test]
    fn an_undelimited_relationship_value_is_the_reference_it_spells() {
        let tree = Tree::new("corpus-undelimited");
        let corpus = read(&tree, &[("ada.md", &linking("engine"))]);
        let reported = ids(&corpus);
        assert!(reported.is_empty(), "{reported:?}");
        let edge = &only(&corpus).relationship("works-at").expect("declared")[0];
        assert_eq!(edge.target().map(VaultPath::as_str), Some("engine.md"));
    }

    #[test]
    fn the_bodys_untyped_references_resolve_and_a_dangling_one_is_never_a_finding() {
        let tree = Tree::new("corpus-body-references");
        let note = concat!(
            "---\ntype: person\nfull_name: Ada\nworks-at: \"[[engine]]\"\n---\n",
            "# Ada\n\nWorked on [[society]], and later on [[difference]].\n",
        );
        let corpus = read(&tree, &[("ada.md", note)]);
        let reported = ids(&corpus);
        assert!(reported.is_empty(), "{reported:?}");
        let references = only(&corpus).body_references();
        let read: Vec<(&str, Option<&str>)> = references
            .iter()
            .map(|reference| {
                (
                    reference.written(),
                    reference.target().map(VaultPath::as_str),
                )
            })
            .collect();
        assert_eq!(
            read,
            [
                ("[[society]]", Some("society.md")),
                ("[[difference]]", None),
            ],
            "a prose reference belongs in prose until its target exists"
        );
    }

    #[test]
    fn a_corpus_resolves_a_callers_reference_by_the_rule_its_own_links_obeyed() {
        let tree = Tree::new("corpus-resolve");
        let corpus = read(
            &tree,
            &[
                ("people/ada.md", CONFORMING),
                ("2025/daily.md", "# Daily\n"),
                ("2026/daily.md", "# Daily\n"),
            ],
        );
        let bare = corpus.resolve("ada").expect("one note bears it");
        assert_eq!(bare.path().as_str(), "people/ada.md");
        let qualified = corpus.resolve("people/ada").expect("a path resolves");
        assert_eq!(qualified.path().as_str(), "people/ada.md");
        let missing = corpus.resolve("babbage").expect_err("no note bears it");
        assert_eq!(missing.diagnostic().id.as_str(), "link.target-not-found");
        assert!(missing.candidates().is_empty());
        let ambiguous = corpus.resolve("daily").expect_err("two notes bear it");
        assert_eq!(
            ambiguous.diagnostic().id.as_str(),
            "link.ambiguous-reference"
        );
        assert_eq!(ambiguous.candidates().len(), 2);
        assert_eq!(ambiguous.reference(), "daily");
    }

    #[test]
    fn a_corpus_looks_a_note_up_by_its_identity() {
        let tree = Tree::new("corpus-lookup");
        let corpus = read(&tree, &[("people/ada.md", CONFORMING)]);
        let found = corpus.note(only(&corpus).path()).expect("held");
        assert_eq!(found.name(), "ada");
        assert!(
            corpus
                .note(&crate::diagnostic::VaultPath::kernel("absent.md"))
                .is_none()
        );
    }

    #[test]
    fn a_corpus_whose_root_cannot_be_read_reports_it_and_holds_no_note() {
        let tree = Tree::new("corpus-gone");
        let load = parse_contract(CONTRACT);
        let contract = load.contract.expect("a conforming contract");
        let corpus = read_corpus(&VaultRoot::new(tree.absent("never-created")), &contract);
        assert!(corpus.notes().is_empty());
        assert_eq!(ids(&corpus), ["note.unreadable"]);
    }

    #[test]
    fn a_corpus_clones_compares_and_formats() {
        let tree = Tree::new("corpus-derives");
        let corpus = read(&tree, &[("ada.md", CONFORMING)]);
        let copy = corpus.clone();
        assert_eq!(copy, corpus);
        assert_ne!(corpus, read(&tree, &[]));
        assert!(format!("{corpus:?}").contains("Lovelace"));
    }
}
