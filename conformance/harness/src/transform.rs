//! Textual transformations of a profile's own committed contract.
//!
//! Every non-conforming input this suite runs is **derived, never authored**.
//! A hand-written broken contract has to be written in *some* corpus's
//! vocabulary, which makes it a profile-specific fixture wearing a
//! shared-sounding name — the exact shape the no-waiver rule exists to catch.
//! Copying each profile's own contract and transforming it runs one assertion
//! against every profile's vocabulary by construction, and no broken contract
//! is checked in anywhere, in any profile, at any path.
//!
//! Two properties make that honest, and both are structural rather than
//! remembered:
//!
//! - **The transformations are textual.** A transform that parsed the document
//!   and re-serialized it would change the bytes through formatting alone,
//!   which would make the caller's *the bytes differ* assertion pass without
//!   the transformation having found anything. Every function here splits the
//!   text into lines that keep their own terminators, edits the lines it
//!   located, and concatenates — so an edit that touched nothing would
//!   reproduce the input byte for byte.
//! - **A transform that cannot find its target is an error.** Never a silent
//!   no-op: one profile spelling a table where another spells an array of
//!   tables is exactly how a derived case would stop testing anything while
//!   still reporting green.

use core::fmt;
use core::ops::Range;

/// A transformation that could not find what it was written to change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetNotFound {
    transform: &'static str,
    target: String,
}

impl TargetNotFound {
    /// The transformation that failed, and what it was looking for.
    pub fn new(transform: &'static str, target: impl Into<String>) -> Self {
        Self {
            transform,
            target: target.into(),
        }
    }

    /// The transformation's name.
    pub fn transform(&self) -> &str {
        self.transform
    }

    /// What it was looking for and did not find.
    pub fn target(&self) -> &str {
        &self.target
    }
}

impl fmt::Display for TargetNotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the `{}` transformation found no {} in the contract, so the derived case would test \
             nothing",
            self.transform, self.target
        )
    }
}

impl std::error::Error for TargetNotFound {}

/// What a transformation answers: the transformed text, or the target it could
/// not find.
pub type Transformed = Result<String, TargetNotFound>;

/// The capability whose cardinality the derived capability cases exercise.
const CATCH_ALL: &str = "\"catch-all\"";

/// The header of a property declaration, so a `name` key belonging to a type
/// is never mistaken for one belonging to a property.
const PROPERTY_HEADER: &str = "[[type.property]]";

/// The name the duplicate-catch-all transformation gives the type it appends.
///
/// Deliberately not a word any corpus would use: the transformation must not
/// collide with a declared type in any profile, and a reader seeing this name
/// in a diagnostic should know at once that it came from the harness.
pub const DERIVED_CATCH_ALL_TYPE: &str = "derived_second_catch_all";

/// A contract as lines that each keep their own terminator.
///
/// Concatenating [`Doc::lines`] reproduces the input byte for byte, which is
/// what keeps a caller's *the bytes differ* assertion meaningful: only an edit
/// can change the output.
struct Doc {
    lines: Vec<String>,
}

impl Doc {
    /// Split `text` without losing a single byte.
    fn parse(text: &str) -> Self {
        Doc {
            lines: text.split_inclusive('\n').map(str::to_owned).collect(),
        }
    }

    /// Reassemble the document.
    fn render(&self) -> String {
        self.lines.concat()
    }

    /// The first line satisfying `matches`.
    fn find(&self, matches: impl Fn(&str) -> bool) -> Option<usize> {
        self.lines.iter().position(|line| matches(line))
    }
}

/// Removes the catch-all capability from the type that declares it.
///
/// # Errors
///
/// [`TargetNotFound`] when no type declares the capability, which would make
/// the derived case a copy of a contract that already fails.
pub fn drop_catch_all(text: &str) -> Transformed {
    let missing = || TargetNotFound::new("drop_catch_all", "catch-all capability declaration");
    let mut doc = Doc::parse(text);
    let at = doc.find(declares_catch_all).ok_or_else(missing)?;
    doc.lines[at] = without_catch_all(&doc, at).ok_or_else(missing)?;
    Ok(doc.render())
}

/// Appends a second type declaring the catch-all capability.
///
/// The appended type is the smallest declaration that carries the capability,
/// so the only rule the derived contract breaks is the cardinality one.
///
/// # Errors
///
/// [`TargetNotFound`] when no type declares the capability, since there would
/// then be nothing to duplicate and the derived contract would break a
/// different rule than the one under test.
pub fn duplicate_catch_all(text: &str) -> Transformed {
    let doc = Doc::parse(text);
    doc.find(declares_catch_all).ok_or_else(|| {
        TargetNotFound::new("duplicate_catch_all", "catch-all capability declaration")
    })?;
    let mut rendered = doc.render();
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    rendered.push_str(&format!(
        "\n[[type]]\nname = \"{DERIVED_CATCH_ALL_TYPE}\"\ncapabilities = [{CATCH_ALL}]\n"
    ));
    Ok(rendered)
}

/// Rewrites the declared `contract_version`.
///
/// # Errors
///
/// [`TargetNotFound`] when the contract declares no version at all — a
/// different fault, and not the one the derived case is about.
pub fn set_contract_version(text: &str, version: u32) -> Transformed {
    let mut doc = Doc::parse(text);
    let at = doc
        .find(|line| line.trim_start().starts_with("contract_version"))
        .ok_or_else(|| TargetNotFound::new("set_contract_version", "`contract_version` key"))?;
    doc.lines[at] = format!("contract_version = {version}\n");
    Ok(doc.render())
}

/// Removes the `[lifecycle]` table entirely, so the contract declares neither
/// an axis nor its absence.
///
/// # Errors
///
/// [`TargetNotFound`] when there is no `[lifecycle]` table to remove.
pub fn delete_lifecycle_table(text: &str) -> Transformed {
    replace_lifecycle(Doc::parse(text), "delete_lifecycle_table", Vec::new())
}

/// Replaces the `[lifecycle]` table with the declaration that this corpus has
/// no life axis — a statement, and one a contract is entitled to make.
///
/// # Errors
///
/// [`TargetNotFound`] when there is no `[lifecycle]` table to replace.
pub fn replace_lifecycle_with_none(text: &str) -> Transformed {
    let body = vec![
        "[lifecycle]\n".to_owned(),
        "none = true\n".to_owned(),
        "\n".to_owned(),
    ];
    replace_lifecycle(Doc::parse(text), "replace_lifecycle_with_none", body)
}

/// Flips the `required` flag on the first declaration of `property`.
///
/// The lifecycle rules tie the encoding of the ordinary state to whether the
/// axis property is required, so flipping one declaration contradicts whichever
/// encoding the contract declares — an *absent* ordinary state against a
/// property that is required somewhere, or a *named* one against a property
/// that is optional somewhere.
///
/// # Errors
///
/// [`TargetNotFound`] when no property declaration of that name carries an
/// explicit `required` key to flip.
pub fn flip_property_required(text: &str, property: &str) -> Transformed {
    let missing = || {
        TargetNotFound::new(
            "flip_property_required",
            format!("`required` key on a declaration of `{property}`"),
        )
    };
    let mut doc = Doc::parse(text);
    let declared = property_declaration(&doc, property).ok_or_else(missing)?;
    let at = required_key(&doc, declared).ok_or_else(missing)?;
    doc.lines[at] = flip_boolean(&doc, at).ok_or_else(missing)?;
    Ok(doc.render())
}

/// `true` for a `capabilities` key that lists the catch-all capability.
fn declares_catch_all(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("capabilities") && trimmed.contains(CATCH_ALL)
}

/// The spellings of the catch-all capability inside an array, longest first so
/// the separator travels with it and the array stays well-formed.
const CATCH_ALL_SPELLINGS: &[&str] = &[
    "\"catch-all\", ",
    "\"catch-all\",",
    ", \"catch-all\"",
    "\"catch-all\"",
];

/// The line at `at` with the catch-all capability removed from its array.
fn without_catch_all(doc: &Doc, at: usize) -> Option<String> {
    let line = &doc.lines[at];
    CATCH_ALL_SPELLINGS
        .iter()
        .find(|spelling| line.contains(**spelling))
        .map(|spelling| line.replacen(spelling, "", 1))
}

/// Replaces the `[lifecycle]` table's lines with `body`.
fn replace_lifecycle(mut doc: Doc, transform: &'static str, body: Vec<String>) -> Transformed {
    let region = lifecycle_region(&doc)
        .ok_or_else(|| TargetNotFound::new(transform, "`[lifecycle]` table"))?;
    doc.lines.splice(region, body);
    Ok(doc.render())
}

/// The `[lifecycle]` table's lines: its header through everything belonging to
/// it, stopping at the next table header.
fn lifecycle_region(doc: &Doc) -> Option<Range<usize>> {
    let start = doc.find(|line| line.trim() == "[lifecycle]")?;
    let end = doc.lines[start + 1..]
        .iter()
        .position(|line| line.trim_start().starts_with('['))
        .map_or(doc.lines.len(), |offset| start + 1 + offset);
    Some(start..end)
}

/// The line naming `property` inside a property declaration.
///
/// The enclosing header is tracked so a type whose `name` happens to equal a
/// property's is never mistaken for the property.
fn property_declaration(doc: &Doc, property: &str) -> Option<usize> {
    let named = format!("name = \"{property}\"");
    let mut inside = false;
    for (index, line) in doc.lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == PROPERTY_HEADER;
        } else if inside && trimmed == named {
            return Some(index);
        }
    }
    None
}

/// The `required` key of the declaration beginning at `from`, if it carries one.
fn required_key(doc: &Doc, from: usize) -> Option<usize> {
    for (offset, line) in doc.lines[from..].iter().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('[') {
            return None;
        }
        if trimmed.starts_with("required") {
            return Some(from + offset);
        }
    }
    None
}

/// The two boolean spellings, each paired with its opposite.
const BOOLEANS: &[(&str, &str)] = &[("true", "false"), ("false", "true")];

/// The line at `at` with its boolean value negated.
fn flip_boolean(doc: &Doc, at: usize) -> Option<String> {
    let line = &doc.lines[at];
    BOOLEANS
        .iter()
        .find(|(written, _)| line.contains(*written))
        .map(|(written, opposite)| line.replacen(written, opposite, 1))
}
