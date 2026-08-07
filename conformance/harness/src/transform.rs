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

/// Removes every `[[flag]]` declaration, and with them any birth state that
/// named one.
///
/// The two go together rather than being two transformations: a birth state
/// names a flag, so stripping the roster while leaving a type born carrying one
/// of its members would derive a contract that breaks the birth-state rule
/// instead of one that declares no flags — a different fault than the one the
/// derived case is about.
///
/// # Errors
///
/// [`TargetNotFound`] when the contract declares no flag, since the derived
/// contract would then be a copy of one that already declares none.
pub fn strip_flags(text: &str) -> Transformed {
    let mut doc = Doc::parse(text);
    doc.find(|line| line.trim() == FLAG_HEADER)
        .ok_or_else(|| TargetNotFound::new("strip_flags", "`[[flag]]` declaration"))?;
    let mut kept: Vec<String> = Vec::new();
    let mut inside = false;
    for line in doc.lines.drain(..) {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == FLAG_HEADER;
        }
        if inside || trimmed.starts_with("born-flagged") {
            continue;
        }
        kept.push(line);
    }
    doc.lines = kept;
    Ok(doc.render())
}

/// The header every flag declaration opens with.
const FLAG_HEADER: &str = "[[flag]]";

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

/// One edit to a note's frontmatter block.
///
/// The three note-level derivations the fixtures record inventories are one
/// operation each: an undeclared type name or corrupted lexical form or
/// repointed link (`Set`), a deleted required property (`Delete`), and an
/// undeclared key (`Insert`).
pub enum NoteEdit<'a> {
    /// Rewrite `key`'s value.
    Set(&'a str, &'a str),
    /// Remove `key`'s line.
    Delete(&'a str),
    /// Insert `key: value` before the closing fence.
    Insert(&'a str, &'a str),
}

/// A note's committed text, distinguished from a contract's.
///
/// The two travel through the same derivation machinery, and a typed wrapper
/// is what keeps a note from being handed to a contract transformation.
pub struct NoteText<'a>(pub &'a str);

/// Applies one [`NoteEdit`] inside a note's frontmatter.
///
/// # Errors
///
/// [`TargetNotFound`] when the note has no frontmatter, or when a `Set` or
/// `Delete` names a key its frontmatter does not carry.
pub fn edit_note_key(note: &NoteText<'_>, edit: &NoteEdit<'_>) -> Transformed {
    let mut doc = Doc::parse(note.0);
    match edit {
        NoteEdit::Set(key, value) => {
            let at = frontmatter_key(&doc, key).ok_or_else(|| {
                TargetNotFound::new("edit_note_key", format!("frontmatter key `{key}`"))
            })?;
            doc.lines[at] = format!("{key}: {value}\n");
        }
        NoteEdit::Delete(key) => {
            let at = frontmatter_key(&doc, key).ok_or_else(|| {
                TargetNotFound::new("edit_note_key", format!("frontmatter key `{key}`"))
            })?;
            doc.lines.remove(at);
        }
        NoteEdit::Insert(key, value) => {
            let (_, close) = frontmatter_bounds(&doc)
                .ok_or_else(|| TargetNotFound::new("edit_note_key", "a frontmatter block"))?;
            doc.lines.insert(close, format!("{key}: {value}\n"));
        }
    }
    Ok(doc.render())
}

/// The line indices strictly inside a note's frontmatter fences.
///
/// `None` when the note has no frontmatter at all — the transformations below
/// refuse such a note rather than inventing a block, because a note the
/// transformation cannot find its target in would make the derived case test
/// nothing.
fn frontmatter_bounds(doc: &Doc) -> Option<(usize, usize)> {
    if doc.lines.first()?.trim_end() != "---" {
        return None;
    }
    let close = doc
        .lines
        .iter()
        .skip(1)
        .position(|line| line.trim_end() == "---")?;
    Some((1, close + 1))
}

/// The index of `key`'s line within the frontmatter block.
fn frontmatter_key(doc: &Doc, key: &str) -> Option<usize> {
    let (open, close) = frontmatter_bounds(doc)?;
    let prefix = format!("{key}:");
    doc.lines[open..close]
        .iter()
        .position(|line| line.trim_start().starts_with(&prefix))
        .map(|offset| open + offset)
}

/// The flag the birth-state derivation declares, and the property it names.
///
/// Deliberately not a word any corpus would use, for the reason
/// [`DERIVED_CATCH_ALL_TYPE`] is not: the derived declaration must collide with
/// nothing in any profile, and a reader meeting it in a note's frontmatter
/// should know at once that the harness put it there.
pub const DERIVED_BIRTH_FLAG: &str = "derived_born_flagged";

/// Declares a birth state on the catch-all, at the version that defines one.
///
/// Three edits in one transformation, because they are one declaration: the
/// version rises to the one with the seats, the catch-all is born carrying
/// [`DERIVED_BIRTH_FLAG`] and declares it as a boolean property, and the
/// contract declares that property a flag. Split into three, each would derive
/// a contract that breaks a rule rather than one that declares a birth state.
///
/// It changes bytes on every profile, including one whose catch-all is already
/// born carrying something: the derived flag is a name no corpus uses, so the
/// existing list is extended rather than replaced.
///
/// # Errors
///
/// [`TargetNotFound`] when the contract declares no `contract_version`, or no
/// type carrying the catch-all capability.
pub fn declare_derived_birth_state(text: &str) -> Transformed {
    let missing = |what: &'static str| TargetNotFound::new("declare_derived_birth_state", what);
    let mut doc = Doc::parse(text);
    let version = doc
        .find(|line| line.trim_start().starts_with("contract_version"))
        .ok_or_else(|| missing("`contract_version` key"))?;
    doc.lines[version] = "contract_version = 3\n".to_owned();
    let capabilities = doc
        .find(declares_catch_all)
        .ok_or_else(|| missing("catch-all capability declaration"))?;
    let end = type_block_end(&doc, capabilities);
    doc.lines.splice(
        end..end,
        [format!(
            "\n  [[type.property]]\n  name = \"{DERIVED_BIRTH_FLAG}\"\n  kind = \"boolean\"\n"
        )],
    );
    born_flagged(&mut doc, capabilities, end);
    let mut rendered = doc.render();
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    rendered.push_str(&format!(
        "\n[[flag]]\nproperty = \"{DERIVED_BIRTH_FLAG}\"\n"
    ));
    Ok(rendered)
}

/// Where a `[[type]]` block ends: the next header that is not one of its own
/// sub-tables, or the end of the document.
fn type_block_end(doc: &Doc, from: usize) -> usize {
    doc.lines[from + 1..]
        .iter()
        .position(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with('[') && !trimmed.starts_with("[[type.")
        })
        .map_or(doc.lines.len(), |offset| from + 1 + offset)
}

/// Adds the derived flag to the catch-all's birth state, extending the list it
/// already declares rather than writing a second key beside it.
fn born_flagged(doc: &mut Doc, from: usize, end: usize) {
    let declared = doc.lines[from..end]
        .iter()
        .position(|line| line.trim_start().starts_with("born-flagged"));
    if let Some(offset) = declared {
        let at = from + offset;
        doc.lines[at] = doc.lines[at].replacen('[', &format!("[\"{DERIVED_BIRTH_FLAG}\", "), 1);
        return;
    }
    doc.lines.splice(
        from + 1..from + 1,
        [format!("born-flagged = [\"{DERIVED_BIRTH_FLAG}\"]\n")],
    );
}

/// The name of the type the tag-namespace derivations append.
///
/// Like [`DERIVED_CATCH_ALL_TYPE`], deliberately not a word any corpus would
/// use: the appended type must collide with nothing, and a reader meeting it
/// in a diagnostic should know it came from the harness.
pub const DERIVED_TAGGED_TYPE: &str = "derived_tagged";

/// The tag namespace the derivations declare on that type: a required, closed
/// namespace whose whole vocabulary is one value.
pub const DERIVED_NAMESPACE_PREFIX: &str = "derived/";

/// The single value inside the derived namespace's closed vocabulary.
pub const DERIVED_NAMESPACE_VALUE: &str = "present";

/// Appends a type declaring a required, closed tag namespace — and the
/// `[tags]` table itself when the contract has none.
///
/// One transformation serves both tag scenarios: a planted note of the
/// appended type with no matching tag misses the required namespace, and one
/// tagged outside the one-word vocabulary breaks the closed namespace. The
/// appended declaration is identical on every profile, so neither scenario
/// depends on a profile's own taxonomy.
///
/// # Errors
///
/// [`TargetNotFound`] when the contract declares no `contract_version` line —
/// the marker that this is a contract at all.
pub fn append_derived_tagged_type(text: &str) -> Transformed {
    let doc = Doc::parse(text);
    doc.find(|line| line.trim_start().starts_with("contract_version"))
        .ok_or_else(|| TargetNotFound::new("append_derived_tagged_type", "contract_version"))?;
    let mut rendered = doc.render();
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    if doc.find(|line| line.trim_end() == "[tags]").is_none() {
        rendered.push_str("\n[tags]\nproperty = \"tags\"\n");
    }
    rendered.push_str(&format!(
        "\n[[type]]\nname = \"{DERIVED_TAGGED_TYPE}\"\n\n  [[type.property]]\n  name = \"tags\"\n  \
         kind = \"list\"\n  of = \"string\"\n\n  [[type.tag-namespace]]\n  prefix = \
         \"{DERIVED_NAMESPACE_PREFIX}\"\n  required = true\n  values = \
         [\"{DERIVED_NAMESPACE_VALUE}\"]\n"
    ));
    Ok(rendered)
}
