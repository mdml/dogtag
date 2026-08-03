//! The generated agent contract.
//!
//! This is the artifact whose whole purpose is non-drift. A vault's agent
//! instructions are **generated from its committed contract** rather than
//! written beside it, because hand-maintained instructions always eventually lie
//! and the cost of that lie compounds as agents do more of the writing.
//!
//! ```markdown
//! # Vault contract
//!
//! This vault is at `/canonical/path/to/vault`. …
//!
//! ## Types
//!
//! ### `person` — identity-bearing
//!
//! | property | kind | required |
//! | --- | --- | --- |
//! | `full_name` | string | yes |
//! ```
//!
//! The rules it keeps:
//!
//! - **The preamble names the resolved vault root.** An agent consuming piped
//!   output must receive the provenance with the instructions; printing the root
//!   to a terminal protects only a human who is watching.
//! - **Capabilities are rendered by capability, never by name.** A type's
//!   heading names what it declares, and a type declaring none says so.
//! - **Nothing the contract did not declare appears** — no invented advice, no
//!   example values, no note-authoring guidance — and nothing it did declare is
//!   omitted. A corpus with no life axis renders *the statement it is*, and so
//!   does a contract with no flags: an omission is indistinguishable from a bug.
//! - **The lexical forms are part of the meaning.** `datetime` renders naming
//!   RFC 3339 with a mandatory offset, `date` naming RFC 3339 `full-date`, and a
//!   `list` naming the kind of its elements.
//! - **Provenance is opt-in and annotates nothing when it is off.** The
//!   Markdown's job is instructing an agent, and a source annotation on every
//!   line makes it materially worse at that.
//! - **Declaration order is the emission order**, for types, properties,
//!   relationships and tag namespaces alike. Never alphabetical.
//! - **A tag namespace renders under the type that declares it**, as a third
//!   table beside the other two. The nesting is bounded by the construct: a
//!   namespace carries a flat vocabulary and nothing below it.
//!
//! A corpus names its own types and its own lifecycle states, and those names
//! reach this output. A heading, a paragraph and a table row are each one
//! *line*, so a value's line breaks fold to spaces — the fold this crate's
//! `text` module owns, shared with the plain-text diagnostic rendering — and a
//! cell's column rules are escaped on top of that. The JSON is where the bytes
//! survive exactly.

use super::yes_no;
use crate::contract::{
    CONTRACT_PATH, Contract, LifecycleDecl, LinkDialect, NamespaceMembership, Ordinary,
    PropertyDecl, PropertyKind, RelationshipDecl, ScalarKind, TagNamespaceDecl, TypeDecl,
};
use crate::diagnostic::Location;
use crate::provenance::{ProvenanceEntry, Source};
use crate::text::one_line;
use crate::vault::VaultRoot;

/// How each scalar kind reads, including the lexical forms that are its meaning.
const SCALARS: &[(ScalarKind, &str)] = &[
    (ScalarKind::String, "string"),
    (ScalarKind::Integer, "integer"),
    (ScalarKind::Float, "float"),
    (ScalarKind::Boolean, "boolean"),
    (ScalarKind::Date, "date (RFC 3339 full-date)"),
    (ScalarKind::DateTime, "datetime (RFC 3339, with offset)"),
];

/// How each dialect reads as an instruction.
const DIALECTS: &[(LinkDialect, &str)] = &[
    (
        LinkDialect::Wikilink,
        "References are written as wikilinks.",
    ),
    (
        LinkDialect::Markdown,
        "References are written as Markdown links.",
    ),
];

/// Renders a resolved contract as the vault's agent contract.
///
/// `provenance` appends each rendered value's source and location. It is opt-in:
/// with it off, **no annotation appears anywhere**.
pub fn contract_markdown(root: &VaultRoot, contract: &Contract, provenance: bool) -> String {
    let render = Render {
        contract,
        provenance,
    };
    format!("{}\n", render.blocks(root).join("\n\n"))
}

/// One rendering pass, carrying the two things every part of it needs.
struct Render<'a> {
    contract: &'a Contract,
    provenance: bool,
}

impl Render<'_> {
    /// Every block of the document, in order, to be joined by blank lines.
    fn blocks(&self, root: &VaultRoot) -> Vec<String> {
        let mut blocks = vec!["# Vault contract".to_owned(), self.preamble(root)];
        blocks.extend(self.annotations(&["contract_version".to_owned()]));
        blocks.push("## Types".to_owned());
        for declared in self.contract.types() {
            blocks.extend(self.type_blocks(declared));
        }
        blocks.push("## Lifecycle".to_owned());
        blocks.push(lifecycle_text(self.contract.lifecycle()));
        blocks.extend(self.annotations(&lifecycle_keys(self.contract.lifecycle())));
        blocks.push("## Flags".to_owned());
        blocks.extend(self.flag_blocks());
        blocks.push("## Tags".to_owned());
        blocks.extend(self.tags_blocks());
        blocks.push("## Dialect".to_owned());
        blocks.push(dialect_text(self.contract.dialect().links()).to_owned());
        blocks.extend(self.annotations(&["dialect.links".to_owned()]));
        blocks
    }

    /// The paragraph that names the vault and where these rules came from.
    fn preamble(&self, root: &VaultRoot) -> String {
        format!(
            "This vault is at `{}`. These rules are generated from its committed contract at \
             `{CONTRACT_PATH}` (contract version {}) and are what the vault enforces. Do not edit \
             this rendering; edit the contract.",
            one_line(&root.display()),
            self.contract.contract_version()
        )
    }

    /// One type: its heading, then what it declares.
    fn type_blocks(&self, declared: &TypeDecl) -> Vec<String> {
        let mut blocks = vec![heading(declared)];
        blocks.extend(self.annotations(&type_keys(declared)));
        blocks.extend(self.declaration_blocks(declared));
        blocks
    }

    /// A type's properties and relationships, or the statements that it has
    /// none of either.
    ///
    /// The two statements share a block, because a type that declares nothing is
    /// one short remark rather than two paragraphs.
    fn declaration_blocks(&self, declared: &TypeDecl) -> Vec<String> {
        let properties = self.properties(declared);
        let relationships = self.relationships(declared);
        let mut blocks = if declared.properties().is_empty() && declared.relationships().is_empty()
        {
            vec![format!("{properties}\n{relationships}")]
        } else {
            vec![properties, relationships]
        };
        blocks.extend(self.namespace_block(declared));
        blocks
    }

    /// A type's tag-namespace table, when it declares any.
    ///
    /// Silence when it declares none, unlike the statements of absence the two
    /// tables above make. The tag vocabulary is a construct only contract
    /// version 2 defines, so "this type declares no tag namespaces" under every
    /// heading of a version-1 contract would state something about a construct
    /// that version does not have — and a version-2 type carrying the tag
    /// property without a namespace is ordinary tagging, which the format
    /// describes nowhere and this rendering therefore describes nowhere.
    fn namespace_block(&self, declared: &TypeDecl) -> Vec<String> {
        if declared.tag_namespaces().is_empty() {
            return Vec::new();
        }
        let rows = declared
            .tag_namespaces()
            .iter()
            .map(|namespace| self.namespace_row(declared, namespace))
            .collect();
        vec![table(
            self.header(&["tag namespace", "membership", "required"]),
            rows,
        )]
    }

    /// One tag namespace's row.
    fn namespace_row(&self, declared: &TypeDecl, namespace: &TagNamespaceDecl) -> Vec<String> {
        self.row(Row {
            name: namespace.prefix(),
            between: Some(membership_text(namespace.membership())),
            required: namespace.required(),
            source: required_key(declared, "tag-namespace", namespace.prefix()),
        })
    }

    /// The property a corpus carries its tags on, or the statement that it
    /// declares no tag vocabulary.
    fn tags_blocks(&self) -> Vec<String> {
        let Some(tags) = self.contract.tags() else {
            return vec!["This contract declares no tag vocabulary.".to_owned()];
        };
        let mut blocks = vec![format!(
            "Tags are carried by the property `{}`, one tag per element.",
            one_line(tags.property())
        )];
        blocks.extend(self.annotations(&["tags.property".to_owned()]));
        blocks
    }

    /// A type's property table, or the statement that it declares none.
    fn properties(&self, declared: &TypeDecl) -> String {
        let rows = declared
            .properties()
            .iter()
            .map(|property| self.property_row(declared, property))
            .collect();
        self.declarations(rows, &["property", "kind", "required"], "properties")
    }

    /// A type's relationship table, or the statement that it declares none.
    fn relationships(&self, declared: &TypeDecl) -> String {
        let rows = declared
            .relationships()
            .iter()
            .map(|relationship| self.relationship_row(declared, relationship))
            .collect();
        self.declarations(rows, &["relationship", "required"], "relationships")
    }

    /// A table over `rows`, or the statement that the type declares no `plural`.
    fn declarations(&self, rows: Vec<Vec<String>>, columns: &[&str], plural: &str) -> String {
        if rows.is_empty() {
            return format!("This type declares no {plural}.");
        }
        table(self.header(columns), rows)
    }

    /// One property's row.
    fn property_row(&self, declared: &TypeDecl, property: &PropertyDecl) -> Vec<String> {
        self.row(Row {
            name: property.name(),
            between: Some(kind_text(property.kind())),
            required: property.required(),
            source: required_key(declared, "property", property.name()),
        })
    }

    /// One relationship's row.
    fn relationship_row(
        &self,
        declared: &TypeDecl,
        relationship: &RelationshipDecl,
    ) -> Vec<String> {
        self.row(Row {
            name: relationship.predicate(),
            between: None,
            required: relationship.required(),
            source: required_key(declared, "relationship", relationship.predicate()),
        })
    }

    /// One declaration's row: its quoted name, whatever its own table carries
    /// between, whether it is required, and where that `required` came from.
    ///
    /// Every table in this rendering has that shape, which is what lets three
    /// tables share one row builder rather than three that drift.
    fn row(&self, row: Row<'_>) -> Vec<String> {
        let mut cells = vec![cell(&format!("`{}`", row.name))];
        cells.extend(row.between.as_deref().map(cell));
        cells.push(yes_no(row.required).to_owned());
        cells.extend(self.source_cells(&row.source));
        cells
    }

    /// A table's header, which gains a source column only under provenance.
    fn header(&self, columns: &[&str]) -> Vec<String> {
        let mut header: Vec<String> = columns.iter().map(|column| (*column).to_owned()).collect();
        if self.provenance {
            header.push("source".to_owned());
        }
        header
    }

    /// A row's trailing source cell, when there is a source column at all.
    ///
    /// The annotated leaf is `required`, which is the leaf whose origin a reader
    /// cannot infer: a name and a kind are always written, and `required` is the
    /// one a format default can supply. *Is this property optional because the
    /// author decided so, or because nobody said?* is the question provenance
    /// exists to answer.
    fn source_cells(&self, key: &str) -> Vec<String> {
        if !self.provenance {
            return Vec::new();
        }
        let recorded = self
            .contract
            .provenance()
            .get(key)
            .map_or_else(|| "not recorded".to_owned(), annotation);
        vec![cell(&recorded)]
    }

    /// A bullet list of the annotations for `keys`, or nothing at all.
    ///
    /// Nothing at all is the common case: with provenance off this is where
    /// every annotation outside a table would have gone, and none is emitted.
    fn annotations(&self, keys: &[String]) -> Vec<String> {
        if !self.provenance {
            return Vec::new();
        }
        let bullets: Vec<String> = keys
            .iter()
            .filter_map(|key| self.contract.provenance().get(key))
            .map(|entry| format!("- `{}` — {}", entry.key, annotation(entry)))
            .collect();
        if bullets.is_empty() {
            return Vec::new();
        }
        vec![bullets.join("\n")]
    }

    /// Each declared flag as its own remark, or the statement that there are
    /// none.
    fn flag_blocks(&self) -> Vec<String> {
        if self.contract.flags().is_empty() {
            return vec!["This contract declares no flags.".to_owned()];
        }
        let mut blocks = Vec::new();
        for flag in self.contract.flags() {
            blocks.push(format!(
                "`{}` — a boolean property, orthogonal to the life axis.",
                one_line(flag.property())
            ));
            blocks.extend(self.annotations(&[format!("flag.{}.property", flag.property())]));
        }
        blocks
    }
}

/// One row of a declaration table, before the source column is decided.
struct Row<'a> {
    /// The declaration's own name, rendered quoted in the first column.
    name: &'a str,
    /// What this table carries between the name and `required`, if anything.
    between: Option<String>,
    required: bool,
    /// The provenance key of the `required` the row renders.
    source: String,
}

/// The provenance key of one declaration's `required` leaf.
fn required_key(declared: &TypeDecl, collection: &str, name: &str) -> String {
    format!("type.{}.{collection}.{name}.required", declared.name())
}

/// A type's heading, naming what it declares rather than what it is called.
fn heading(declared: &TypeDecl) -> String {
    format!(
        "### `{}` — {}",
        one_line(declared.name()),
        capabilities_text(declared)
    )
}

/// The capabilities a type declares, or that it declares none.
fn capabilities_text(declared: &TypeDecl) -> String {
    if declared.capabilities().is_empty() {
        return "no capabilities".to_owned();
    }
    declared
        .capabilities()
        .iter()
        .map(|capability| capability.as_str())
        .collect::<Vec<&str>>()
        .join(", ")
}

/// A property kind, spelled with everything that is part of its meaning.
fn kind_text(kind: &PropertyKind) -> String {
    match kind {
        PropertyKind::Enum { values } => enum_text(values),
        PropertyKind::List { of } => format!("list of {}", scalar_text(*of)),
        scalar => scalar_text(
            ScalarKind::named(scalar.as_str()).expect("a kind that is neither enum nor list"),
        )
        .to_owned(),
    }
}

/// An `enum`, with its members in declaration order.
fn enum_text(values: &[String]) -> String {
    let members: Vec<String> = values.iter().map(|value| format!("`{value}`")).collect();
    format!("enum ({})", members.join(", "))
}

/// A namespace's membership: its closed vocabulary in declaration order, or
/// that it declares none.
///
/// A member is quoted and `open` is not, so a closed vocabulary whose one
/// member happens to be the word `open` still reads as the vocabulary it is.
fn membership_text(membership: &NamespaceMembership) -> String {
    let Some(values) = membership.values() else {
        return "open".to_owned();
    };
    let members: Vec<String> = values.iter().map(|value| format!("`{value}`")).collect();
    members.join(", ")
}

/// A scalar kind's reading.
fn scalar_text(kind: ScalarKind) -> &'static str {
    SCALARS
        .iter()
        .find(|(known, _)| *known == kind)
        .map(|(_, spelling)| *spelling)
        .expect("every scalar kind names its lexical form")
}

/// The dialect as an instruction.
fn dialect_text(links: LinkDialect) -> &'static str {
    DIALECTS
        .iter()
        .find(|(known, _)| *known == links)
        .map(|(_, spelling)| *spelling)
        .expect("every dialect reads as an instruction")
}

/// The lifecycle declaration as a statement, including the statement that a
/// corpus has no life axis.
fn lifecycle_text(lifecycle: &LifecycleDecl) -> String {
    match (lifecycle.axis(), lifecycle.ordinary()) {
        (Some(axis), Some(Ordinary::Absent)) => format!(
            "The life axis is the property `{axis}`. A note is in the ordinary state when \
             `{axis}` is absent; any declared value marks a departure from it.",
            axis = one_line(axis)
        ),
        (Some(axis), Some(Ordinary::Value(value))) => format!(
            "The life axis is the property `{axis}`. A note is in the ordinary state when \
             `{axis}` is `{value}`; any other declared value marks a departure from it.",
            axis = one_line(axis),
            value = one_line(value)
        ),
        _ => "This corpus declares no lifecycle axis.".to_owned(),
    }
}

/// The provenance keys the lifecycle section renders values from.
fn lifecycle_keys(lifecycle: &LifecycleDecl) -> Vec<String> {
    match (lifecycle.axis(), lifecycle.ordinary()) {
        (Some(_), Some(Ordinary::Absent)) => {
            vec![
                "lifecycle.axis".to_owned(),
                "lifecycle.ordinary.absent".to_owned(),
            ]
        }
        (Some(_), Some(Ordinary::Value(_))) => {
            vec![
                "lifecycle.axis".to_owned(),
                "lifecycle.ordinary.value".to_owned(),
            ]
        }
        _ => vec!["lifecycle.none".to_owned()],
    }
}

/// The provenance keys a type's heading renders values from.
fn type_keys(declared: &TypeDecl) -> Vec<String> {
    let name = declared.name();
    vec![
        format!("type.{name}.name"),
        format!("type.{name}.capabilities"),
    ]
}

/// One leaf's source and location, as the annotation a reader sees.
fn annotation(entry: &ProvenanceEntry) -> String {
    if let Source::Default { contract_version } = entry.source {
        return format!("(default, contract version {contract_version})");
    }
    let at = entry
        .location
        .as_ref()
        .map_or_else(String::new, |location| format!("`{}` ", located(location)));
    format!("{at}({})", entry.source)
}

/// A location as `path:line:column`, or as the path alone.
fn located(location: &Location) -> String {
    match location.span {
        Some(span) => format!(
            "{}:{}:{}",
            location.file, span.start.line, span.start.column
        ),
        None => location.file.to_string(),
    }
}

/// A table: a header, its rule, and one line per row.
fn table(header: Vec<String>, rows: Vec<Vec<String>>) -> String {
    let rule = vec!["---".to_owned(); header.len()];
    let mut lines = vec![row_line(&header), row_line(&rule)];
    lines.extend(rows.iter().map(|row| row_line(row)));
    lines.join("\n")
}

/// One table line.
fn row_line(cells: &[String]) -> String {
    format!("| {} |", cells.join(" | "))
}

/// A value inside a table cell, with the column rule escaped.
///
/// The line fold is [`one_line`]'s, shared with the plain-text diagnostic
/// rendering; only the column rule is this format's own.
fn cell(value: &str) -> String {
    one_line(value).replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use super::super::fixture::{
        ABSENT_ORDINARY, AWKWARD, Body, CLEAN, FIXTURES, KINDS, NAMED_ORDINARY, TAGGED, Tree,
        assert_holds, assert_no_line, rendered, shown,
    };
    use super::*;
    use crate::diagnostic::{FileRef, Position, Span, VaultPath};

    fn markdown(tree: &Tree, body: Body<'_>, provenance: bool) -> String {
        let (root, contract) = rendered(tree, body);
        contract_markdown(&root, &contract, provenance)
    }

    fn location(span: Option<Span>) -> Location {
        Location {
            file: FileRef::InVault(VaultPath::kernel(".dogtag/contract.toml")),
            span,
        }
    }

    #[test]
    fn the_preamble_names_the_resolved_root_so_piped_output_carries_it() {
        let tree = Tree::new("markdown-preamble");
        let (root, contract) = rendered(&tree, NAMED_ORDINARY);
        let document = contract_markdown(&root, &contract, false);
        assert!(document.starts_with("# Vault contract\n\n"));
        assert_holds(
            &document,
            &format!("This vault is at `{}`.", shown(root.path())),
        );
        assert_holds(&document, "`.dogtag/contract.toml` (contract version 2)");
        assert_holds(&document, "Do not edit this rendering; edit the contract.");
        assert!(document.ends_with(".\n"));
    }

    #[test]
    fn a_type_heading_names_the_capabilities_it_declares() {
        let tree = Tree::new("markdown-capabilities");
        let document = markdown(&tree, NAMED_ORDINARY, false);
        assert_holds(&document, "### `note` — catch-all\n");
        assert_holds(&document, "### `person` — identity-bearing\n");
        assert_holds(&document, "### `project` — no capabilities\n");
    }

    #[test]
    fn a_type_declaring_nothing_says_so_rather_than_rendering_an_empty_table() {
        let tree = Tree::new("markdown-empty-type");
        assert_holds(
            &markdown(&tree, CLEAN, false),
            "### `capture` — catch-all\n\nThis type declares no properties.\nThis type declares \
             no relationships.\n",
        );
    }

    #[test]
    fn a_type_with_properties_and_no_relationships_still_says_it_has_none() {
        let tree = Tree::new("markdown-no-relationships");
        assert_holds(
            &markdown(&tree, NAMED_ORDINARY, false),
            "This type declares no relationships.",
        );
    }

    #[test]
    fn a_property_table_renders_a_row_per_declaration_in_declaration_order() {
        let tree = Tree::new("markdown-properties");
        assert_holds(
            &markdown(&tree, NAMED_ORDINARY, false),
            concat!(
                "| property | kind | required |\n",
                "| --- | --- | --- |\n",
                "| `status` | enum (`active`, `archived`) | yes |\n",
                "| `tags` | list of string | no |\n",
            ),
        );
    }

    #[test]
    fn a_relationship_table_renders_its_predicate_and_whether_it_is_required() {
        let tree = Tree::new("markdown-relationships");
        assert_holds(
            &markdown(&tree, NAMED_ORDINARY, false),
            concat!(
                "| relationship | required |\n",
                "| --- | --- |\n",
                "| `involves` | no |\n",
            ),
        );
    }

    #[test]
    fn the_lexical_form_of_a_kind_is_part_of_what_it_renders() {
        let tree = Tree::new("markdown-kinds");
        let document = markdown(&tree, KINDS, false);
        assert_holds(&document, "| `text` | string | no |\n");
        assert_holds(&document, "| `count` | integer | no |\n");
        assert_holds(&document, "| `ratio` | float | no |\n");
        assert_holds(&document, "| `flagged` | boolean | no |\n");
        assert_holds(&document, "| `day` | date (RFC 3339 full-date) | no |\n");
        assert_holds(
            &document,
            "| `moment` | datetime (RFC 3339, with offset) | no |\n",
        );
        assert_holds(
            &document,
            "| `sightings` | list of date (RFC 3339 full-date) | no |\n",
        );
        assert_holds(&document, "| `state` | enum (`one`, `two`) | no |\n");
    }

    #[test]
    fn a_corpus_with_no_life_axis_renders_the_statement_it_is() {
        let tree = Tree::new("markdown-no-axis");
        assert_holds(
            &markdown(&tree, CLEAN, false),
            "## Lifecycle\n\nThis corpus declares no lifecycle axis.\n",
        );
    }

    #[test]
    fn an_axis_renders_how_its_ordinary_state_is_encoded() {
        let tree = Tree::new("markdown-axis");
        assert_holds(
            &markdown(&tree, ABSENT_ORDINARY, false),
            "The life axis is the property `standing`. A note is in the ordinary state when \
             `standing` is absent; any declared value marks a departure from it.",
        );
        assert_holds(
            &markdown(&tree, NAMED_ORDINARY, false),
            "The life axis is the property `status`. A note is in the ordinary state when \
             `status` is `active`; any other declared value marks a departure from it.",
        );
    }

    #[test]
    fn a_contract_with_no_flags_renders_the_statement_it_is() {
        let tree = Tree::new("markdown-no-flags");
        assert_holds(
            &markdown(&tree, NAMED_ORDINARY, false),
            "## Flags\n\nThis contract declares no flags.\n",
        );
    }

    #[test]
    fn each_declared_flag_is_rendered_as_the_orthogonal_property_it_is() {
        let tree = Tree::new("markdown-flags");
        let document = markdown(&tree, ABSENT_ORDINARY, false);
        assert_holds(
            &document,
            "`needs_rework` — a boolean property, orthogonal to the life axis.\n",
        );
        assert_holds(
            &document,
            "`confidential` — a boolean property, orthogonal to the life axis.\n",
        );
    }

    #[test]
    fn a_contract_with_no_tag_vocabulary_renders_the_statement_it_is() {
        let tree = Tree::new("markdown-no-tags");
        assert_holds(
            &markdown(&tree, NAMED_ORDINARY, false),
            "## Tags\n\nThis contract declares no tag vocabulary.\n",
        );
    }

    #[test]
    fn a_tag_vocabulary_names_the_property_that_carries_it() {
        let tree = Tree::new("markdown-tags");
        assert_holds(
            &markdown(&tree, TAGGED, false),
            "## Tags\n\nTags are carried by the property `labels`, one tag per element.\n",
        );
    }

    #[test]
    fn a_namespace_table_renders_a_row_per_declaration_in_declaration_order() {
        let tree = Tree::new("markdown-namespaces");
        assert_holds(
            &markdown(&tree, TAGGED, false),
            concat!(
                "| tag namespace | membership | required |\n",
                "| --- | --- | --- |\n",
                "| `log/` | `workout`, `meditation`, `a \\| pipe` | yes |\n",
                "| `topic/` | open | no |\n",
            ),
        );
    }

    #[test]
    fn a_type_declaring_no_namespace_renders_no_namespace_table() {
        // Silence rather than a statement of absence: the construct exists only
        // at contract version 2, so every type of a version-1 contract would
        // otherwise carry a remark about a construct its format does not have.
        let tree = Tree::new("markdown-no-namespaces");
        assert_no_line(&markdown(&tree, NAMED_ORDINARY, false), |line| {
            line.starts_with("| tag namespace")
        });
    }

    #[test]
    fn with_provenance_on_a_namespace_table_gains_a_source_column() {
        let tree = Tree::new("markdown-provenance-namespaces");
        let document = markdown(&tree, TAGGED, true);
        assert_holds(
            &document,
            "| tag namespace | membership | required | source |\n",
        );
        assert_holds(
            &document,
            "| `topic/` | open | no | (default, contract version 2) |\n",
        );
        assert_holds(&document, "- `tags.property` — `.dogtag/contract.toml:");
    }

    #[test]
    fn the_dialect_renders_as_an_instruction() {
        let tree = Tree::new("markdown-dialect");
        assert_holds(
            &markdown(&tree, NAMED_ORDINARY, false),
            "## Dialect\n\nReferences are written as wikilinks.\n",
        );
        assert_holds(
            &markdown(&tree, AWKWARD, false),
            "## Dialect\n\nReferences are written as Markdown links.\n",
        );
    }

    #[test]
    fn with_provenance_off_no_annotation_appears_anywhere() {
        let tree = Tree::new("markdown-no-provenance");
        for (name, body) in FIXTURES {
            let document = markdown(&tree, body, false);
            assert!(
                !document.contains("| source |"),
                "`{name}` grew a source column"
            );
            assert!(
                !document.contains("(contract)"),
                "`{name}` annotated a source"
            );
            assert!(
                !document.contains("(default, contract version"),
                "`{name}` annotated a default"
            );
            assert_no_line(&document, |line| line.starts_with("- `"));
        }
    }

    #[test]
    fn with_provenance_on_a_table_gains_a_source_column() {
        let tree = Tree::new("markdown-provenance-table");
        let document = markdown(&tree, NAMED_ORDINARY, true);
        assert_holds(&document, "| property | kind | required | source |\n");
        assert_holds(&document, "| --- | --- | --- | --- |\n");
        assert_holds(
            &document,
            "| `status` | enum (`active`, `archived`) | yes | `.dogtag/contract.toml:18:14` \
             (contract) |\n",
        );
        assert_holds(
            &document,
            "| `tags` | list of string | no | (default, contract version 2) |\n",
        );
        assert_holds(&document, "| relationship | required | source |\n");
        assert_holds(
            &document,
            "| `involves` | no | (default, contract version 2) |\n",
        );
    }

    #[test]
    fn with_provenance_on_every_value_outside_a_table_is_annotated_too() {
        let tree = Tree::new("markdown-provenance-values");
        let document = markdown(&tree, ABSENT_ORDINARY, true);
        assert_holds(
            &document,
            "- `contract_version` — `.dogtag/contract.toml:1:20` (contract)\n",
        );
        assert_holds(
            &document,
            "- `dialect.links` — `.dogtag/contract.toml:4:9` (contract)\n",
        );
        assert_holds(
            &document,
            "- `lifecycle.axis` — `.dogtag/contract.toml:7:8` (contract)\n",
        );
        assert_holds(
            &document,
            "- `lifecycle.ordinary.absent` — `.dogtag/contract.toml:8:23` (contract)\n",
        );
        assert_holds(
            &document,
            "- `flag.needs_rework.property` — `.dogtag/contract.toml:11:12` (contract)\n",
        );
        assert_holds(&document, "- `type.person.name` — `.dogtag/contract.toml:");
        assert_holds(
            &document,
            "- `type.person.capabilities` — `.dogtag/contract.toml:",
        );
    }

    #[test]
    fn a_corpus_that_declares_no_axis_annotates_the_declaration_it_did_make() {
        let tree = Tree::new("markdown-provenance-none");
        assert_holds(
            &markdown(&tree, CLEAN, true),
            "- `lifecycle.none` — `.dogtag/contract.toml:7:8` (contract)\n",
        );
    }

    #[test]
    fn a_defaulted_capability_list_is_annotated_as_the_default_it_is() {
        let tree = Tree::new("markdown-provenance-default");
        assert_holds(
            &markdown(&tree, NAMED_ORDINARY, true),
            "- `type.project.capabilities` — (default, contract version 2)\n",
        );
    }

    #[test]
    fn a_vocabulary_carrying_a_column_rule_or_a_line_break_keeps_the_table_intact() {
        let tree = Tree::new("markdown-awkward");
        let document = markdown(&tree, AWKWARD, false);
        assert_holds(&document, "### `capture` — catch-all\n");
        assert_holds(&document, "| `état` | enum (");
        assert_holds(&document, r"`a \| pipe`");
        assert_holds(&document, "`a   break`");
        assert_holds(&document, "`a \\ backslash`");
        assert_holds(&document, "naïve");
        let rows = document
            .lines()
            .filter(|line| line.starts_with("| `état`"))
            .count();
        assert_eq!(rows, 1, "an awkward value must not split its row");
        assert_holds(
            &document,
            "The life axis is the property `état`. A note is in the ordinary state when `état` \
             is `a \" quote`;",
        );
    }

    #[test]
    fn the_same_contract_renders_the_same_bytes_every_time() {
        let tree = Tree::new("markdown-deterministic");
        for (name, body) in FIXTURES {
            let (root, contract) = rendered(&tree, body);
            for provenance in [false, true] {
                assert_eq!(
                    contract_markdown(&root, &contract, provenance),
                    contract_markdown(&root, &contract, provenance),
                    "`{name}` did not render identically twice"
                );
            }
        }
    }

    #[test]
    fn types_render_in_declaration_order_and_never_sorted() {
        let tree = Tree::new("markdown-order");
        let (root, contract) = rendered(&tree, ABSENT_ORDINARY);
        let document = contract_markdown(&root, &contract, false);
        let headings: Vec<&str> = document
            .lines()
            .filter(|line| line.starts_with("### "))
            .collect();
        let declared: Vec<String> = contract.types().iter().map(heading).collect();
        assert_eq!(headings, declared);
        let mut sorted = headings.clone();
        sorted.sort_unstable();
        assert_ne!(headings, sorted, "the fixture must not already be sorted");
    }

    #[test]
    fn an_annotation_reads_its_source_even_where_there_is_nothing_to_point_at() {
        let unlocated = ProvenanceEntry {
            key: "dialect.links".to_owned(),
            source: Source::Contract,
            location: None,
        };
        assert_eq!(annotation(&unlocated), "(contract)");
        let whole_file = ProvenanceEntry {
            key: "dialect.links".to_owned(),
            source: Source::Installation,
            location: Some(location(None)),
        };
        assert_eq!(
            annotation(&whole_file),
            "`.dogtag/contract.toml` (installation)"
        );
        assert_eq!(
            located(&location(Some(Span::at(Position::new(4, 9, 31))))),
            ".dogtag/contract.toml:4:9"
        );
    }

    #[test]
    fn a_leaf_with_no_recorded_provenance_says_that_rather_than_inventing_one() {
        let tree = Tree::new("markdown-unrecorded");
        let (_, contract) = rendered(&tree, CLEAN);
        let render = Render {
            contract: &contract,
            provenance: true,
        };
        assert_eq!(render.source_cells("no.such.leaf"), ["not recorded"]);
        assert!(
            Render {
                contract: &contract,
                provenance: false,
            }
            .source_cells("contract_version")
            .is_empty()
        );
        assert!(render.annotations(&["no.such.leaf".to_owned()]).is_empty());
    }
}
