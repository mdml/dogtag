//! Where a diagnostic points.
//!
//! Two file references exist and no more. An in-vault path is always relative
//! to the vault root and always uses forward slashes, so the same fault in the
//! same corpus renders identically on every machine; the installation record
//! renders as `$XDG_CONFIG_HOME/dogtag/installation.toml` and is never
//! expanded, so no diagnostic emits an account name.
//!
//! Positions carry a 1-based line, a 1-based column counted in **Unicode
//! scalar values** rather than bytes or UTF-16 units, and the 0-based byte
//! offset the column was derived from.
//!
//! The derived ordering on these types is load-bearing: it is the second and
//! third tier of the diagnostic order (see [`crate::diagnostic::order`]).
//! `InVault` sorts before `InstallationRecord`, in-vault paths sort by path,
//! a location with no span sorts before any span, and positions sort by line,
//! then column, then byte offset.

use core::fmt;

use super::VaultPath;
use crate::installation::RECORD_RELATIVE_PATH;

/// The file a diagnostic is about.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileRef {
    /// A path relative to the vault root, always with forward slashes — for
    /// example `.dogtag/contract.toml`. The payload is opaque because the
    /// spelling is the guarantee; see [`VaultPath`] for what producing one
    /// proves and what it does not.
    InVault(VaultPath),
    /// The local installation record, reported unexpanded.
    InstallationRecord,
}

impl FileRef {
    /// How the installation record is rendered, in every surface and every
    /// format. It is deliberately not the expanded path.
    ///
    /// It is the variable naming the configuration directory with
    /// [`RECORD_RELATIVE_PATH`] beneath it, and the two are held together while
    /// this crate compiles.
    /// A consumer that has to resolve the record on a real machine joins that
    /// constant onto the directory it resolved rather than taking this
    /// rendering apart: this is text a reader sees, and a path recovered from
    /// it would break the day it is reworded.
    pub const INSTALLATION_RECORD_PATH: &'static str = RECORD_RENDERING;

    /// The path as it is rendered to a reader.
    pub fn display_path(&self) -> &str {
        match self {
            Self::InVault(path) => path.as_str(),
            Self::InstallationRecord => Self::INSTALLATION_RECORD_PATH,
        }
    }
}

impl fmt::Display for FileRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_path())
    }
}

/// The rendering, and the assertion that it really is the configuration
/// variable with the record's own relative path beneath it.
///
/// The check runs while this crate compiles rather than in a test, so the two
/// declarations cannot drift at all: rewording the rendering without moving
/// [`RECORD_RELATIVE_PATH`] fails the build. It is a named constant rather than
/// an anonymous one because the walk below would then be reachable from nothing
/// a build counts, and would report itself as dead code on the minimum
/// supported toolchain.
const RECORD_RENDERING: &str = {
    let rendering = "$XDG_CONFIG_HOME/dogtag/installation.toml";
    assert!(ends_with(rendering, RECORD_RELATIVE_PATH));
    rendering
};

/// Whether `text` ends with `suffix`, answerable while the crate compiles.
///
/// `str::ends_with` is not a `const fn`, so the byte walk is written out. Bytes
/// rather than characters is exact here: one UTF-8 encoding ends with another
/// only where the scalars do.
const fn ends_with(text: &str, suffix: &str) -> bool {
    let (text, suffix) = (text.as_bytes(), suffix.as_bytes());
    if text.len() < suffix.len() {
        return false;
    }
    let offset = text.len() - suffix.len();
    let mut index = 0;
    while index < suffix.len() {
        if text[offset + index] != suffix[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// A point in a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    /// 1-based line number.
    pub line: u32,
    /// 1-based column, counted in Unicode scalar values.
    pub column: u32,
    /// 0-based byte offset from the start of the file.
    pub offset: usize,
}

impl Position {
    /// A position at `line`:`column`, `offset` bytes into the file.
    pub fn new(line: u32, column: u32, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }
}

/// A region of a file, with an optional end for faults that have a real extent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
    /// Where the span starts.
    pub start: Position,
    /// Where the span ends, when the fault has an extent worth reporting.
    pub end: Option<Position>,
}

impl Span {
    /// A span that points at a single position.
    pub fn at(start: Position) -> Self {
        Self { start, end: None }
    }

    /// A span running from `start` to `end`.
    pub fn between(start: Position, end: Position) -> Self {
        Self {
            start,
            end: Some(end),
        }
    }
}

/// A file, and optionally a region within it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Location {
    /// The file the diagnostic is about.
    pub file: FileRef,
    /// The region within it, when the fault is narrower than the file.
    pub span: Option<Span>,
}

impl Location {
    /// A location naming a file and nothing more precise.
    pub fn whole_file(file: FileRef) -> Self {
        Self { file, span: None }
    }

    /// A location naming a region within a file.
    pub fn in_file(file: FileRef, span: Span) -> Self {
        Self {
            file,
            span: Some(span),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cmp::Ordering;

    fn contract() -> FileRef {
        FileRef::InVault(VaultPath::kernel(".dogtag/contract.toml"))
    }

    #[test]
    fn in_vault_paths_render_as_written() {
        assert_eq!(contract().display_path(), ".dogtag/contract.toml");
        assert_eq!(contract().to_string(), ".dogtag/contract.toml");
    }

    #[test]
    fn the_installation_record_renders_unexpanded() {
        let file = FileRef::InstallationRecord;
        assert_eq!(
            file.display_path(),
            "$XDG_CONFIG_HOME/dogtag/installation.toml"
        );
        assert_eq!(file.to_string(), FileRef::INSTALLATION_RECORD_PATH);
    }

    #[test]
    fn the_rendering_is_the_variable_above_the_record_path() {
        // The relationship itself is asserted where the crate compiles; this is
        // the walk that answers it, held up against the cases it has to get
        // right for that assertion to mean anything.
        assert!(ends_with(
            FileRef::INSTALLATION_RECORD_PATH,
            RECORD_RELATIVE_PATH
        ));
        assert!(ends_with(RECORD_RELATIVE_PATH, RECORD_RELATIVE_PATH));
        assert!(!ends_with(
            RECORD_RELATIVE_PATH,
            "dogtag/installation.tomls"
        ));
        assert!(!ends_with(RECORD_RELATIVE_PATH, "dogtag/installation.tom_"));
    }

    #[test]
    fn constructors_build_the_documented_shapes() {
        let start = Position::new(2, 5, 17);
        let end = Position::new(2, 9, 21);
        assert_eq!(Span::at(start), Span { start, end: None });
        assert_eq!(
            Span::between(start, end),
            Span {
                start,
                end: Some(end)
            }
        );
        assert_eq!(
            Location::whole_file(contract()),
            Location {
                file: contract(),
                span: None
            }
        );
        assert_eq!(
            Location::in_file(contract(), Span::at(start)),
            Location {
                file: contract(),
                span: Some(Span::at(start))
            }
        );
    }

    #[test]
    fn locations_clone_and_format() {
        let location = Location::in_file(
            contract(),
            Span::between(Position::new(1, 1, 0), Position::new(1, 4, 3)),
        );
        let copy = location.clone();
        assert_eq!(copy, location);
        assert!(format!("{location:?}").contains("contract.toml"));
        assert!(format!("{:?}", location.span).contains("Position"));
    }

    #[test]
    fn in_vault_files_sort_before_the_installation_record() {
        let vault = contract();
        let record = FileRef::InstallationRecord;
        assert!(vault < record);
        assert_eq!(vault.cmp(&record), Ordering::Less);
        assert_eq!(record.cmp(&record.clone()), Ordering::Equal);
    }

    #[test]
    fn in_vault_files_sort_by_path() {
        let a = FileRef::InVault(VaultPath::kernel(".dogtag/a.toml"));
        let b = FileRef::InVault(VaultPath::kernel(".dogtag/b.toml"));
        assert!(a < b);
        assert_eq!(a.cmp(&b), Ordering::Less);
    }

    #[test]
    fn positions_sort_by_line_then_column_then_offset() {
        assert!(Position::new(1, 9, 99) < Position::new(2, 1, 0));
        assert!(Position::new(2, 1, 99) < Position::new(2, 2, 0));
        assert!(Position::new(2, 2, 3) < Position::new(2, 2, 4));
        assert_eq!(
            Position::new(2, 2, 3).cmp(&Position::new(2, 2, 3)),
            Ordering::Equal
        );
    }

    #[test]
    fn a_location_without_a_span_sorts_before_one_with_a_span() {
        let bare = Location::whole_file(contract());
        let spanned = Location::in_file(contract(), Span::at(Position::new(1, 1, 0)));
        assert!(bare < spanned);
        assert_eq!(bare.cmp(&spanned), Ordering::Less);
    }

    #[test]
    fn a_span_without_an_end_sorts_before_one_with_an_end() {
        let start = Position::new(3, 1, 40);
        assert!(Span::at(start) < Span::between(start, Position::new(3, 4, 43)));
        assert_eq!(Span::at(start).cmp(&Span::at(start)), Ordering::Equal);
    }
}
