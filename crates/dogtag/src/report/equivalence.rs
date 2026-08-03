//! The two contract renderings say the same thing.
//!
//! *Every declaration in the Markdown appears in the JSON and the reverse, and
//! neither carries a declaration the contract does not make.* That is an
//! acceptance criterion rather than a hope, so it is asserted **mechanically**:
//! a set of declaration atoms is extracted from the resolved [`Contract`], from
//! the rendered Markdown, and from the rendered JSON, by three readers that
//! share no code, and the three sets must be equal.
//!
//! The readers are deliberately independent. A helper shared between a renderer
//! and its checker proves only that the helper is self-consistent; two readers
//! written against the *documents* catch a declaration that one rendering drops,
//! misspells, or invents.
//!
//! An atom is one declaration, written so that a difference names itself in the
//! failure. The kind lattice is canonicalized away from either document's
//! spelling — the Markdown's lexical prose and the JSON's wire word both reduce
//! to the same atom — because the *lexical* forms are asserted where they are
//! rendered, and what this file asserts is the set of declarations.

use std::collections::BTreeSet;

use serde_json::Value;

use super::fixture::{
    ABSENT_ORDINARY, AWKWARD, Body, CLEAN, KINDS, MARKDOWN_LINKS, NAMED_ORDINARY, TAGGED, Tree,
    rendered,
};
use super::{contract_json, contract_markdown};
use crate::contract::{
    Contract, LifecycleDecl, NamespaceMembership, Ordinary, PropertyKind, TypeDecl,
};

/// What separates an atom's fields. Chosen so no corpus vocabulary holds it.
const FIELD: &str = " :: ";

/// One declaration, in the form every reader reduces to.
struct Declaration<'a> {
    owner: &'a str,
    name: &'a str,
    required: bool,
}

impl Declaration<'_> {
    /// This declaration as a property whose kind is `spelled`.
    fn property(&self, spelled: &str) -> String {
        let (owner, name, required) = (self.owner, self.name, self.required);
        format!("type.{owner}.property{FIELD}{name}{FIELD}{spelled}{FIELD}{required}")
    }

    /// This declaration as a relationship.
    fn relationship(&self) -> String {
        let (owner, name, required) = (self.owner, self.name, self.required);
        format!("type.{owner}.relationship{FIELD}{name}{FIELD}{required}")
    }

    /// This declaration as a tag namespace, whose name is its prefix.
    fn tag_namespace(&self, membership: &str) -> String {
        let (owner, name, required) = (self.owner, self.name, self.required);
        format!("type.{owner}.tag-namespace{FIELD}{name}{FIELD}{membership}{FIELD}{required}")
    }
}

/// The declarations a resolved contract makes.
fn from_contract(contract: &Contract) -> BTreeSet<String> {
    let mut atoms = BTreeSet::new();
    atoms.insert(format!(
        "contract_version{FIELD}{}",
        contract.contract_version()
    ));
    atoms.insert(format!(
        "dialect.links{FIELD}{}",
        contract.dialect().links()
    ));
    atoms.extend(model_lifecycle(contract.lifecycle()));
    atoms.extend(
        contract
            .tags()
            .map(|tags| format!("tags.property{FIELD}{}", tags.property())),
    );
    for flag in contract.flags() {
        atoms.insert(format!("flag{FIELD}{}", flag.property()));
    }
    for declared in contract.types() {
        atoms.extend(model_type(declared));
    }
    atoms
}

fn model_lifecycle(lifecycle: &LifecycleDecl) -> Vec<String> {
    let mut atoms = vec![format!("lifecycle.declared{FIELD}{}", lifecycle.declared())];
    atoms.extend(
        lifecycle
            .axis()
            .map(|axis| format!("lifecycle.axis{FIELD}{axis}")),
    );
    atoms.extend(lifecycle.ordinary().map(|ordinary| match ordinary {
        Ordinary::Absent => format!("lifecycle.ordinary{FIELD}absent"),
        Ordinary::Value(value) => format!("lifecycle.ordinary.value{FIELD}{value}"),
    }));
    atoms
}

fn model_type(declared: &TypeDecl) -> Vec<String> {
    let owner = declared.name();
    let mut atoms = vec![format!("type{FIELD}{owner}")];
    for capability in declared.capabilities() {
        atoms.push(format!("type.{owner}.capability{FIELD}{capability}"));
    }
    for property in declared.properties() {
        let declaration = Declaration {
            owner,
            name: property.name(),
            required: property.required(),
        };
        atoms.push(declaration.property(&model_kind(property.kind())));
    }
    for relationship in declared.relationships() {
        let declaration = Declaration {
            owner,
            name: relationship.predicate(),
            required: relationship.required(),
        };
        atoms.push(declaration.relationship());
    }
    for namespace in declared.tag_namespaces() {
        let declaration = Declaration {
            owner,
            name: namespace.prefix(),
            required: namespace.required(),
        };
        atoms.push(declaration.tag_namespace(&model_membership(namespace.membership())));
    }
    atoms
}

fn model_kind(kind: &PropertyKind) -> String {
    match kind {
        PropertyKind::Enum { values } => format!("enum({})", values.join(";")),
        PropertyKind::List { of } => format!("list({of})"),
        scalar => scalar.as_str().to_owned(),
    }
}

fn model_membership(membership: &NamespaceMembership) -> String {
    match membership {
        NamespaceMembership::Closed { values } => format!("closed({})", values.join(";")),
        NamespaceMembership::Open => "open".to_owned(),
    }
}

/// The declarations a rendered JSON document carries.
fn from_json(document: &str) -> BTreeSet<String> {
    let parsed: Value = serde_json::from_str(document).expect("this module's own output is JSON");
    let contract = &parsed["contract"];
    let mut atoms = BTreeSet::new();
    atoms.insert(format!(
        "contract_version{FIELD}{}",
        contract["contract_version"]
    ));
    atoms.insert(format!(
        "dialect.links{FIELD}{}",
        as_str(&contract["dialect"]["links"])
    ));
    atoms.extend(json_lifecycle(&contract["lifecycle"]));
    if let Some(property) = contract["tags"]["property"].as_str() {
        atoms.insert(format!("tags.property{FIELD}{property}"));
    }
    for flag in array(&contract["flags"]) {
        atoms.insert(format!("flag{FIELD}{}", as_str(&flag["property"])));
    }
    for declared in array(&contract["types"]) {
        atoms.extend(json_type(declared));
    }
    atoms
}

fn json_lifecycle(lifecycle: &Value) -> Vec<String> {
    let mut atoms = vec![format!(
        "lifecycle.declared{FIELD}{}",
        as_str(&lifecycle["declared"])
    )];
    if let Some(axis) = lifecycle["axis"].as_str() {
        atoms.push(format!("lifecycle.axis{FIELD}{axis}"));
    }
    if let Some(value) = lifecycle["ordinary"]["value"].as_str() {
        atoms.push(format!("lifecycle.ordinary.value{FIELD}{value}"));
    } else if lifecycle["ordinary"]["absent"] == true {
        atoms.push(format!("lifecycle.ordinary{FIELD}absent"));
    }
    atoms
}

fn json_type(declared: &Value) -> Vec<String> {
    let owner = as_str(&declared["name"]);
    let mut atoms = vec![format!("type{FIELD}{owner}")];
    for capability in array(&declared["capabilities"]) {
        atoms.push(format!(
            "type.{owner}.capability{FIELD}{}",
            as_str(capability)
        ));
    }
    for property in array(&declared["properties"]) {
        let declaration = Declaration {
            owner,
            name: as_str(&property["name"]),
            required: property["required"] == true,
        };
        atoms.push(declaration.property(&json_kind(property)));
    }
    for relationship in array(&declared["relationships"]) {
        let declaration = Declaration {
            owner,
            name: as_str(&relationship["predicate"]),
            required: relationship["required"] == true,
        };
        atoms.push(declaration.relationship());
    }
    for namespace in array(&declared["tag_namespaces"]) {
        let declaration = Declaration {
            owner,
            name: as_str(&namespace["prefix"]),
            required: namespace["required"] == true,
        };
        atoms.push(declaration.tag_namespace(&json_membership(namespace)));
    }
    atoms
}

fn json_membership(namespace: &Value) -> String {
    let Some(values) = namespace["values"].as_array() else {
        return "open".to_owned();
    };
    let members: Vec<&str> = values.iter().map(as_str).collect();
    format!("closed({})", members.join(";"))
}

fn json_kind(property: &Value) -> String {
    match as_str(&property["kind"]) {
        "enum" => format!(
            "enum({})",
            array(&property["values"])
                .iter()
                .map(as_str)
                .collect::<Vec<&str>>()
                .join(";")
        ),
        "list" => format!("list({})", as_str(&property["of"])),
        scalar => scalar.to_owned(),
    }
}

fn as_str(value: &Value) -> &str {
    let expected = format!("a string was expected, and `{value}` is not one");
    value.as_str().expect(&expected)
}

fn array(value: &Value) -> &[Value] {
    let expected = format!("an array was expected, and `{value}` is not one");
    value.as_array().expect(&expected).as_slice()
}

/// One line of a rendered document.
///
/// A newtype rather than a `&str` because every reading below asks a question
/// about a *line* of Markdown — what it opens with, what it writes in code,
/// which cells it holds — rather than about text in general.
#[derive(Clone, Copy)]
struct Line<'a>(&'a str);

impl<'a> Line<'a> {
    /// The line as written.
    fn as_str(self) -> &'a str {
        self.0
    }

    /// Whether the line opens with `opening`.
    fn opens_with(self, opening: &str) -> bool {
        self.0.starts_with(opening)
    }

    /// What follows `opening`, when the line opens with it.
    fn after(self, opening: &str) -> Option<Self> {
        self.0.strip_prefix(opening).map(Self)
    }

    /// Everything the line writes in code spans.
    fn code(self) -> Vec<&'a str> {
        self.0.split('`').skip(1).step_by(2).collect()
    }

    /// The line's table cells, or nothing when it is not a table line.
    fn cells(self) -> Vec<Self> {
        self.0
            .strip_prefix("| ")
            .and_then(|rest| rest.strip_suffix(" |"))
            .map(|body| body.split(" | ").map(Self).collect())
            .unwrap_or_default()
    }

    /// The line's code span, without the backticks around it.
    fn unquoted(self) -> &'a str {
        self.0.trim_matches('`')
    }
}

/// Which table a row belongs to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Columns {
    Property,
    Relationship,
    Namespace,
}

/// A reader that walks a rendered Markdown document once.
struct Scan {
    atoms: BTreeSet<String>,
    kind: String,
    columns: Option<Columns>,
}

/// Which dialect each rendered sentence stands for.
const DIALECTS: &[(&str, &str)] = &[
    ("References are written as wikilinks.", "wikilink"),
    ("References are written as Markdown links.", "markdown"),
];

/// What one kind of line contributes to the atoms read so far.
type Handler = fn(&mut Scan, Line<'_>);

/// Which handler each kind of line belongs to, keyed by how the line opens.
///
/// The first match wins, so the openings are ordered from most to least
/// specific. A line matching nothing — a section heading, a rule, a statement of
/// absence — declares nothing and is skipped, which is the point: absence is not
/// a declaration and must not become an atom in one document only.
const LINES: &[(&str, Handler)] = &[
    ("### ", Scan::heading),
    ("| ", Scan::row),
    ("This vault is at ", Scan::preamble),
    ("This corpus declares no lifecycle axis.", Scan::no_axis),
    ("The life axis is the property ", Scan::axis),
    ("References are written as ", Scan::dialect),
    ("Tags are carried by the property ", Scan::tags),
    ("`", Scan::flag),
];

impl Scan {
    fn new() -> Self {
        Self {
            atoms: BTreeSet::new(),
            kind: String::new(),
            columns: None,
        }
    }

    fn read(mut self, document: &str) -> BTreeSet<String> {
        for line in document.lines() {
            self.line(Line(line));
        }
        self.atoms
    }

    fn line(&mut self, line: Line<'_>) {
        if let Some((_, handle)) = LINES.iter().find(|(opening, _)| line.opens_with(opening)) {
            handle(self, line);
        }
    }

    fn preamble(&mut self, line: Line<'_>) {
        let version = line
            .as_str()
            .split_once("(contract version ")
            .and_then(|(_, rest)| rest.split_once(')'))
            .map(|(version, _)| version)
            .expect("the preamble names the contract version");
        self.atoms
            .insert(format!("contract_version{FIELD}{version}"));
    }

    fn heading(&mut self, line: Line<'_>) {
        let rest = line.after("### ").expect("a heading opens with its rule");
        let (name, capabilities) = rest
            .as_str()
            .split_once(" — ")
            .expect("a heading names a type and its capabilities");
        self.kind = Line(name).unquoted().to_owned();
        self.atoms.insert(format!("type{FIELD}{}", self.kind));
        if capabilities == "no capabilities" {
            return;
        }
        for capability in capabilities.split(", ") {
            self.atoms
                .insert(format!("type.{}.capability{FIELD}{capability}", self.kind));
        }
    }

    fn row(&mut self, line: Line<'_>) {
        let cells = line.cells();
        match cells.first().copied().map(Line::as_str) {
            Some("property") => self.columns = Some(Columns::Property),
            Some("relationship") => self.columns = Some(Columns::Relationship),
            Some("tag namespace") => self.columns = Some(Columns::Namespace),
            Some("---") => (),
            _ => self.declaration(&cells),
        }
    }

    fn declaration(&mut self, cells: &[Line<'_>]) {
        let name = unescaped(cells[0].unquoted());
        let atom = match self.columns.expect("a row arrives after its own header") {
            Columns::Property => self
                .declared(&name, cells[2])
                .property(&unescaped(&kind(cells[1]))),
            Columns::Relationship => self.declared(&name, cells[1]).relationship(),
            Columns::Namespace => self
                .declared(&name, cells[2])
                .tag_namespace(&unescaped(&membership(cells[1]))),
        };
        self.atoms.insert(atom);
    }

    /// The declaration a row names, under the type being read.
    fn declared<'a>(&'a self, name: &'a str, required: Line<'a>) -> Declaration<'a> {
        Declaration {
            owner: &self.kind,
            name,
            required: required.as_str() == "yes",
        }
    }

    fn no_axis(&mut self, _line: Line<'_>) {
        self.atoms.insert(format!("lifecycle.declared{FIELD}none"));
    }

    fn axis(&mut self, line: Line<'_>) {
        let quoted = line.code();
        let axis = quoted.first().expect("the axis is named in code");
        self.atoms.insert(format!("lifecycle.declared{FIELD}axis"));
        self.atoms.insert(format!("lifecycle.axis{FIELD}{axis}"));
        if line.as_str().contains("is absent;") {
            self.atoms
                .insert(format!("lifecycle.ordinary{FIELD}absent"));
            return;
        }
        let value = quoted.get(2).expect("a named ordinary state is in code");
        self.atoms
            .insert(format!("lifecycle.ordinary.value{FIELD}{value}"));
    }

    fn dialect(&mut self, line: Line<'_>) {
        let links = DIALECTS
            .iter()
            .find(|(sentence, _)| *sentence == line.as_str())
            .map(|(_, links)| *links)
            .expect("a dialect sentence names a dialect");
        self.atoms.insert(format!("dialect.links{FIELD}{links}"));
    }

    fn flag(&mut self, line: Line<'_>) {
        let property = line.code().first().copied().expect("a flag names it");
        self.atoms.insert(format!("flag{FIELD}{property}"));
    }

    fn tags(&mut self, line: Line<'_>) {
        let property = line
            .code()
            .first()
            .copied()
            .expect("the tag property is named in code");
        self.atoms.insert(format!("tags.property{FIELD}{property}"));
    }
}

/// The declarations a rendered Markdown document carries.
fn from_markdown(document: &str) -> BTreeSet<String> {
    Scan::new().read(document)
}

/// A cell's kind, reduced from the Markdown's lexical prose to a bare atom.
fn kind(cell: Line<'_>) -> String {
    if let Some(element) = cell.after("list of ") {
        return format!("list({})", kind(element));
    }
    if let Some(members) = cell.after("enum (") {
        return format!("enum({})", members.code().join(";"));
    }
    let spelled = cell.as_str();
    spelled
        .split_once(" (")
        .map_or(spelled, |(base, _)| base)
        .to_owned()
}

/// A cell's membership, reduced from the Markdown's member list to a bare atom.
///
/// A closed vocabulary writes each member in code and an open namespace writes
/// a bare word, so the presence of a code span is what tells them apart — and a
/// closed vocabulary whose one member is the word `open` still reads as closed.
fn membership(cell: Line<'_>) -> String {
    let members = cell.code();
    if members.is_empty() {
        return "open".to_owned();
    }
    format!("closed({})", members.join(";"))
}

/// A cell's text with the column rule the renderer escaped restored.
///
/// The Markdown renderer's `cell` replaces every `|` with `\|`, so every `|` in
/// a rendered cell stands behind a backslash the renderer put there, and
/// removing exactly those is that replacement's inverse. It forgives nothing:
/// it is *reading* a Markdown cell, which is this reader's job.
///
/// Applied to a row's cells and to nothing else. A heading, a lifecycle
/// sentence and a flag remark are folded but never escaped, so undoing a
/// backslash there would invent a difference rather than remove one.
fn unescaped(cell: &str) -> String {
    cell.replace(r"\|", "|")
}

/// `atoms` as the Markdown is *permitted* to spell them.
///
/// The surfaces record's 2026-08-01 amendment decides that the two renderings
/// carry the same declarations and may differ in the spelling of a value
/// holding a character a Markdown table cannot carry. One such difference
/// survives reading: a control character folds to a space, and no reader can
/// undo that, so it is stated here as the equivalence relation rather than
/// repaired.
///
/// Applied **only to the contract's and the JSON's atoms**, never to the
/// Markdown's, so a value that reached the rendering unfolded is still a
/// difference. The fold is spelled out here rather than borrowed from
/// [`crate::text`], so that changing the renderer's fold fails this check
/// instead of moving with it.
fn markdown_spelling(atoms: &BTreeSet<String>) -> BTreeSet<String> {
    atoms.iter().map(|atom| folded(atom)).collect()
}

/// One atom with every control character folded to a space.
fn folded(atom: &str) -> String {
    atom.chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every contract these renderings are held up against, named so a failure
    /// says which one disagreed.
    const SUBJECTS: [(&str, Body<'static>); 7] = [
        ("starter", NAMED_ORDINARY),
        ("dense", ABSENT_ORDINARY),
        ("clean", CLEAN),
        ("kinds", KINDS),
        ("markdown-links", MARKDOWN_LINKS),
        ("tagged", TAGGED),
        // The vocabulary carrying a column rule and a line break. It was the
        // one fixture this list omitted, and it was omitted because it fails:
        // the check was arranged around its own counterexample.
        ("awkward", AWKWARD),
    ];

    #[test]
    fn the_markdown_and_the_json_declare_exactly_what_the_contract_declares() {
        let tree = Tree::new("equivalence");
        for (name, body) in SUBJECTS {
            let (root, contract) = rendered(&tree, body);
            let declared = from_contract(&contract);
            let markdown = from_markdown(&contract_markdown(&root, &contract, false));
            let json = from_json(&contract_json(&root, &contract));
            let spelling = markdown_spelling(&declared);
            assert_eq!(
                spelling.len(),
                declared.len(),
                "`{name}`: the fold merged two declarations into one"
            );
            assert_eq!(
                json, declared,
                "`{name}`: the JSON and the contract disagree"
            );
            assert_eq!(
                markdown, spelling,
                "`{name}`: the Markdown and the contract disagree"
            );
            assert_eq!(
                markdown,
                markdown_spelling(&json),
                "`{name}`: the two renderings disagree"
            );
        }
    }

    #[test]
    fn turning_provenance_on_adds_no_declaration_and_removes_none() {
        let tree = Tree::new("equivalence-provenance");
        for (name, body) in SUBJECTS {
            let (root, contract) = rendered(&tree, body);
            let annotated = from_markdown(&contract_markdown(&root, &contract, true));
            let declared = markdown_spelling(&from_contract(&contract));
            assert!(!declared.is_empty(), "`{name}` declares nothing at all");
            assert_eq!(
                annotated, declared,
                "`{name}`: annotating the Markdown changed what it declares"
            );
        }
    }

    #[test]
    fn the_readers_notice_a_declaration_that_one_document_dropped() {
        let tree = Tree::new("equivalence-negative");
        let (root, contract) = rendered(&tree, NAMED_ORDINARY);
        let document = contract_markdown(&root, &contract, false);
        let declared = markdown_spelling(&from_contract(&contract));
        let without = document.replace("| `due` | date (RFC 3339 full-date) | no |\n", "");
        assert_ne!(document, without, "the row this test removes must exist");
        assert_ne!(
            from_markdown(&without),
            declared,
            "a dropped declaration must be detected"
        );
        let renamed = document.replace("`involves`", "`invokes`");
        assert_ne!(
            from_markdown(&renamed),
            declared,
            "a misspelled declaration must be detected"
        );
    }

    #[test]
    fn a_kind_reduces_to_the_same_atom_from_either_document() {
        let spellings = [
            ("string", "string"),
            ("date (RFC 3339 full-date)", "date"),
            ("datetime (RFC 3339, with offset)", "datetime"),
            ("enum (`one`, `two`)", "enum(one;two)"),
            ("list of date (RFC 3339 full-date)", "list(date)"),
        ];
        for (spelled, atom) in spellings {
            assert_eq!(kind(Line(spelled)), atom);
        }
        assert!(Line("no cells here").cells().is_empty());
        assert_eq!(Line("`quoted`").unquoted(), "quoted");
    }

    #[test]
    fn a_membership_reduces_to_the_same_atom_from_either_document() {
        assert_eq!(membership(Line("open")), "open");
        assert_eq!(
            membership(Line("`workout`, `meditation`")),
            "closed(workout;meditation)"
        );
        // A vocabulary of one member spelled `open` is still a vocabulary,
        // because a member is written in code and the open namespace is not.
        assert_eq!(membership(Line("`open`")), "closed(open)");
    }
}
