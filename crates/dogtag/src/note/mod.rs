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

mod traverse;

pub use traverse::{Traversal, traverse};
