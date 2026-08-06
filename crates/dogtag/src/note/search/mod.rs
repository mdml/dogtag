//! Lexical retrieval: a corpus scan, not an index.
//!
//! Each search walks the corpus through the same shared loading, traversal,
//! and validation path `check`, `list`, and `show` use, so a broken corpus
//! surfaces its diagnostics on every retrieval and no second reading of the
//! model can drift. There is deliberately **no persistent index, no staleness
//! semantics, and no write path**: the full walk over the largest observable
//! vault measures well under the interactive bar, and the recorded trigger —
//! a measured median over one second on a real vault — is what would land an
//! index at M6, not argument.
//!
//! A query matches a note's **body and its identity**: the vault-relative
//! path, the title, and the note's aliases. A search that cannot find a note
//! by its own title loses the cutover comparison on day one, which is why
//! body-only matching was rejected. Aliases ride the declared property named
//! `aliases` on the note's bound type — the tag-property precedent: the
//! values surface only where a type declares the property, and a corpus that
//! declares no such property simply has no aliases to match.
//!
//! [`ListFilter`]'s four filters compose with the query, ANDed, under `list`'s
//! own rules — search is enumeration plus a text predicate, and it must not
//! grow a second filter vocabulary.
//!
//! **Ordering is not contract.** Hits are relevance-ordered with a
//! deterministic tie-break on the vault-relative path, so identical runs
//! produce identical bytes — but conformance asserts membership and count, so
//! ranking can improve without an amendment.

mod query;
mod tokens;

use core::cmp::Ordering;
use core::ops::Range;

use crate::contract::Contract;
use crate::diagnostic::{Diagnostic, DiagnosticList, VaultPath};
use crate::vault::VaultRoot;

use super::model::{Note, PropertyValue};
use super::{Corpus, ListFilter, list, read_corpus};

use query::Query;
use tokens::Token;

/// The declared property whose values are a note's aliases.
///
/// A convention on the declaration rather than a reserved word: the kernel
/// reads the property a type declares under this name, and a corpus whose
/// vocabulary wants `aliases` to mean something else simply is not matched by
/// alias.
const ALIAS_PROPERTY: &str = "aliases";

/// How many bytes of the note's own text a snippet quotes on each side of the
/// first match.
const SNIPPET_CONTEXT: usize = 40;

/// One search hit: identity, bound type, and a matched-context snippet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    path: VaultPath,
    type_name: Option<String>,
    snippet: Option<String>,
}

impl SearchHit {
    /// The note's vault-relative identity.
    pub fn path(&self) -> &VaultPath {
        &self.path
    }

    /// The type the note bound to, when it bound successfully.
    pub fn type_name(&self) -> Option<&str> {
        self.type_name.as_deref()
    }

    /// The note's own text around the first body match, or the matched title
    /// or alias where only the identity matched.
    ///
    /// `None` where the match is the path alone, which the hit already names.
    pub fn snippet(&self) -> Option<&str> {
        self.snippet.as_deref()
    }
}

/// One search, as asked: the query text, the filters, and the cap.
///
/// The three travel together because none of them answers anything alone —
/// a query is matched under its filters, and the cap is part of what the
/// caller asked for, not a rendering choice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchRequest {
    /// The query text, under the M4 grammar.
    pub query: String,
    /// The four filters, ANDed with the query under `list`'s rules.
    pub filter: ListFilter,
    /// Keep at most this many hits, best-ranked first.
    pub limit: usize,
}

/// The result of searching a corpus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchResult {
    hits: Vec<SearchHit>,
    diagnostics: Vec<Diagnostic>,
}

impl SearchResult {
    /// Matching notes, relevance-ordered with a path tie-break.
    pub fn hits(&self) -> &[SearchHit] {
        &self.hits
    }

    /// Everything the search reported, in diagnostic order.
    ///
    /// The scan is a full validation pass, so a broken corpus's findings are
    /// here on every retrieval. An empty result is a result, not a
    /// diagnostic.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Searches notes under `root` for what `request` asks.
///
/// Bare terms are OR-combined and relevance-ranked, `"quoted phrases"` match
/// adjacent words in order, and a trailing `*` is a prefix wildcard. Every
/// supplied filter composes with the query, ANDed, under `list`'s rules.
///
/// A query the grammar cannot read is `search.invalid-query`, an error, and
/// no scan runs behind it — exactly as a lifecycle filter against a corpus
/// declaring no axis is refused before enumeration.
pub fn search(root: &VaultRoot, contract: &Contract, request: &SearchRequest) -> SearchResult {
    let (parsed, mut faults) = match query::parse(&request.query) {
        Ok(parsed) => (Some(parsed), Vec::new()),
        Err(fault) => (None, vec![fault.diagnostic(&request.query)]),
    };
    faults.extend(list::axis_refusal(contract, &request.filter));
    match (parsed, faults.is_empty()) {
        (Some(parsed), true) => scan(root, contract, &parsed, request),
        _ => refusal(faults),
    }
}

/// The scan itself, once the query and the filters are known to be readable.
fn scan(
    root: &VaultRoot,
    contract: &Contract,
    query: &Query,
    request: &SearchRequest,
) -> SearchResult {
    let Corpus { notes, diagnostics } = read_corpus(root, contract);
    let mut scored: Vec<Scored> = notes
        .iter()
        .filter(|note| list::matches(note, contract, &request.filter))
        .filter_map(|note| evaluate(note, query))
        .collect();
    scored.sort_by(rank);
    scored.truncate(request.limit);
    SearchResult {
        hits: scored.into_iter().map(|scored| scored.hit).collect(),
        diagnostics,
    }
}

/// A run refused before any note was read: the faults are the whole answer.
fn refusal(faults: Vec<Diagnostic>) -> SearchResult {
    let mut diagnostics = DiagnosticList::new();
    diagnostics.extend(faults);
    SearchResult {
        hits: Vec::new(),
        diagnostics: diagnostics.sorted(),
    }
}

/// How well a note matched, in the order the comparisons run.
///
/// Derived ordering reads the fields top to bottom: how many of the query's
/// atoms matched at all, then how often the note's identity — title, path,
/// aliases — matched, then how often anything did. None of this is contract;
/// the deterministic tie-break on the path is.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Relevance {
    matched_atoms: usize,
    identity: usize,
    occurrences: usize,
}

/// One matching note, ready to be ordered.
struct Scored {
    relevance: Relevance,
    hit: SearchHit,
}

/// Best relevance first; the vault-relative path breaks every tie.
fn rank(left: &Scored, right: &Scored) -> Ordering {
    right
        .relevance
        .cmp(&left.relevance)
        .then_with(|| left.hit.path.as_str().cmp(right.hit.path.as_str()))
}

/// Holds `note` against `query`, answering a scored hit iff anything matched.
fn evaluate(note: &Note, query: &Query) -> Option<Scored> {
    let fields = Fields::of(note);
    let tally = fields.tally(query);
    if tally.matched_atoms == 0 {
        return None;
    }
    Some(Scored {
        relevance: Relevance {
            matched_atoms: tally.matched_atoms,
            identity: tally.identity,
            occurrences: tally.occurrences,
        },
        hit: SearchHit {
            path: note.path().clone(),
            type_name: note.binding().type_name().map(str::to_owned),
            snippet: fields.snippet(note, &tally),
        },
    })
}

/// Everything of one note a query is matched against, tokenized once.
struct Fields<'a> {
    body: Vec<Token>,
    title: Vec<Token>,
    path: Vec<Token>,
    aliases: Vec<(&'a str, Vec<Token>)>,
}

/// What matching one note against the whole query found.
#[derive(Default)]
struct Tally {
    matched_atoms: usize,
    identity: usize,
    occurrences: usize,
    first_body: Option<Range<usize>>,
    first_alias: Option<usize>,
}

impl<'a> Fields<'a> {
    fn of(note: &'a Note) -> Self {
        Self {
            body: tokens::scan(note.body()),
            title: tokens::scan(note.title().unwrap_or("")),
            path: tokens::scan(note.path().as_str()),
            aliases: aliases(note)
                .into_iter()
                .map(|alias| (alias, tokens::scan(alias)))
                .collect(),
        }
    }

    fn tally(&self, query: &Query) -> Tally {
        let mut tally = Tally::default();
        for atom in &query.atoms {
            self.weigh(atom, &mut tally);
        }
        tally
    }

    /// One atom's occurrences across every field, folded into the tally.
    fn weigh(&self, atom: &query::Atom, tally: &mut Tally) {
        let body = atom.count(&self.body);
        if body > 0 {
            tally.note_body(atom.first(&self.body));
        }
        let title = atom.count(&self.title);
        let identity = title + atom.count(&self.path) + self.weigh_aliases(atom, tally);
        if body + identity > 0 {
            tally.matched_atoms += 1;
        }
        tally.identity += identity;
        tally.occurrences += body + identity;
    }

    /// The atom's occurrences across the note's aliases, remembering the
    /// first alias anything matched — the snippet's fallback subject.
    fn weigh_aliases(&self, atom: &query::Atom, tally: &mut Tally) -> usize {
        let mut total = 0;
        for (index, (_, tokens)) in self.aliases.iter().enumerate() {
            let count = atom.count(tokens);
            if count > 0 {
                tally.note_alias(index);
            }
            total += count;
        }
        total
    }

    /// The matched context this note is quoted by: the body around the first
    /// match, else the matched alias — and nothing where the path alone
    /// matched, because the hit already names it.
    ///
    /// There is deliberately no title fallback: the title is the first H1,
    /// whose words are the body's own, so a title match always has a body
    /// window quoting the heading itself.
    fn snippet(&self, note: &Note, tally: &Tally) -> Option<String> {
        if let Some(span) = &tally.first_body {
            return Some(window(note.body(), span));
        }
        tally
            .first_alias
            .map(|index| self.aliases[index].0.to_owned())
    }
}

impl Tally {
    /// Keeps the earliest body match, whichever atom found it.
    fn note_body(&mut self, span: Option<Range<usize>>) {
        let span = span.expect("a counted match is a found match");
        let earlier = self
            .first_body
            .as_ref()
            .is_none_or(|seen| span.start < seen.start);
        if earlier {
            self.first_body = Some(span);
        }
    }

    /// Keeps the first alias in note order that anything matched.
    fn note_alias(&mut self, index: usize) {
        let earlier = self.first_alias.is_none_or(|seen| index < seen);
        if earlier {
            self.first_alias = Some(index);
        }
    }
}

/// The note's aliases: its bound type's declared `aliases` property values.
fn aliases(note: &Note) -> Vec<&str> {
    match note.property(ALIAS_PROPERTY) {
        Some(PropertyValue::Scalar(value)) => vec![value.as_str()],
        Some(PropertyValue::List(values)) => values.iter().map(String::as_str).collect(),
        _ => Vec::new(),
    }
}

/// The note's own text around `span`, with an ellipsis marking each cut edge.
fn window(body: &str, span: &Range<usize>) -> String {
    let from = boundary_back(body, span.start.saturating_sub(SNIPPET_CONTEXT));
    let to = boundary_forward(body, (span.end + SNIPPET_CONTEXT).min(body.len()));
    let mut snippet = String::new();
    if from > 0 {
        snippet.push('…');
    }
    snippet.push_str(body[from..to].trim());
    if to < body.len() {
        snippet.push('…');
    }
    snippet
}

fn boundary_back(text: &str, mut at: usize) -> usize {
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

fn boundary_forward(text: &str, mut at: usize) -> usize {
    while !text.is_char_boundary(at) {
        at += 1;
    }
    at
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::parse_contract;
    use crate::vault::{SENTINEL, tree::Tree};
    use std::fs;

    /// A contract with an axis, tags, and an `aliases` declaration on the one
    /// identity-bearing type — every field search reads, declared.
    const CONTRACT: &str = concat!(
        "contract_version = 2\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\naxis = \"stage\"\nordinary = { absent = true }\n",
        "\n[tags]\nproperty = \"tags\"\n",
        "\n[[type]]\nname = \"work\"\ncapabilities = [\"identity-bearing\"]\n",
        "  [[type.property]]\n  name = \"stage\"\n  kind = \"enum\"\n  values = [\"active\", \"done\"]\n",
        "  [[type.property]]\n  name = \"tags\"\n  kind = \"list\"\n  of = \"string\"\n",
        "  [[type.property]]\n  name = \"aliases\"\n  kind = \"list\"\n  of = \"string\"\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    /// A contract that declares `aliases` as a single string.
    const SCALAR_ALIAS: &str = concat!(
        "contract_version = 2\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[[type]]\nname = \"work\"\ncapabilities = [\"identity-bearing\"]\n",
        "  [[type.property]]\n  name = \"aliases\"\n  kind = \"string\"\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    fn asking(query: &str) -> SearchRequest {
        SearchRequest {
            query: query.to_owned(),
            filter: ListFilter::default(),
            limit: 20,
        }
    }

    fn searched(label: &str, contract: &str, notes: &[(&str, &str)], query: &str) -> SearchResult {
        searched_with(label, contract, notes, &asking(query))
    }

    fn searched_with(
        label: &str,
        contract: &str,
        notes: &[(&str, &str)],
        request: &SearchRequest,
    ) -> SearchResult {
        let tree = Tree::new(label);
        let root = tree.vault("vault");
        fs::write(root.join(SENTINEL), contract).expect("a contract this test owns");
        for (relative, text) in notes {
            let path = root.join(relative);
            let parent = path.parent().expect("a note under the root has a parent");
            fs::create_dir_all(parent).expect("a directory this test owns");
            fs::write(path, text).expect("a note this test owns");
        }
        let resolved = parse_contract(contract).contract.expect("resolved");
        search(&VaultRoot::new(root), &resolved, request)
    }

    fn paths(result: &SearchResult) -> Vec<&str> {
        result
            .hits()
            .iter()
            .map(|hit| hit.path().as_str())
            .collect()
    }

    #[test]
    fn bare_terms_are_or_combined_across_body_title_path_and_aliases() {
        let notes: &[(&str, &str)] = &[
            ("prose.md", "# Elsewhere\n\nThe analytical engine.\n"),
            ("titled.md", "# The Engine\n\nnothing else\n"),
            ("engine.md", "unrelated words\n"),
            (
                "known-as.md",
                "---\ntype: work\naliases: [\"the Engine\"]\n---\nprose\n",
            ),
            ("silent.md", "no match here\n"),
        ];
        let result = searched("search-or", CONTRACT, notes, "engine daily");
        assert!(result.diagnostics().is_empty());
        assert_eq!(
            paths(&result),
            ["titled.md", "engine.md", "known-as.md", "prose.md"],
            "identity matches outrank the body-only match; ties fall to the path"
        );
    }

    #[test]
    fn every_hit_carries_its_identity_its_type_and_its_context() {
        let notes: &[(&str, &str)] = &[(
            "work/alpha.md",
            "---\ntype: work\nstage: active\n---\n# Alpha\n\nThe analytical engine restarted.\n",
        )];
        let result = searched("search-hit", CONTRACT, notes, "engine");
        let hit = &result.hits()[0];
        assert_eq!(hit.path().as_str(), "work/alpha.md");
        assert_eq!(hit.type_name(), Some("work"));
        assert_eq!(
            hit.snippet(),
            Some("# Alpha\n\nThe analytical engine restarted."),
            "a short body is quoted whole, cut nowhere"
        );
    }

    #[test]
    fn a_long_body_is_quoted_around_the_first_match_with_cut_edges_marked() {
        let body = format!("# T\n\n{} engine {}\n", "x".repeat(120), "y".repeat(120));
        let notes: &[(&str, &str)] = &[("long.md", &body)];
        let result = searched("search-window", CONTRACT, notes, "engine");
        let snippet = result.hits()[0].snippet().expect("a body match");
        assert!(snippet.starts_with('…'), "{snippet}");
        assert!(snippet.ends_with('…'), "{snippet}");
        assert!(snippet.contains("engine"));
        assert_eq!(snippet.matches('x').count(), SNIPPET_CONTEXT - 1);
        assert_eq!(snippet.matches('y').count(), SNIPPET_CONTEXT - 1);
    }

    #[test]
    fn a_snippet_window_lands_on_character_boundaries() {
        let body = format!("{} engine {}\n", "λ".repeat(30), "λ".repeat(30));
        let notes: &[(&str, &str)] = &[("greek.md", &body)];
        let result = searched("search-boundary", CONTRACT, notes, "engine");
        let snippet = result.hits()[0].snippet().expect("a body match");
        assert!(snippet.contains("engine"), "{snippet}");
    }

    #[test]
    fn an_identity_only_match_quotes_the_first_matched_alias_and_a_path_match_nothing() {
        let notes: &[(&str, &str)] = &[
            (
                "known-as.md",
                "---\ntype: work\naliases: [\"Difference Engine\", \"the Engine\"]\n---\nprose\n",
            ),
            ("engine.md", "unrelated\n"),
        ];
        let result = searched("search-context", CONTRACT, notes, "engine");
        let by_path: Vec<(&str, Option<&str>)> = result
            .hits()
            .iter()
            .map(|hit| (hit.path().as_str(), hit.snippet()))
            .collect();
        assert_eq!(
            by_path,
            [
                ("known-as.md", Some("Difference Engine")),
                ("engine.md", None),
            ],
            "two alias occurrences outrank one path occurrence"
        );
    }

    #[test]
    fn a_title_match_is_quoted_by_the_body_window_holding_the_heading() {
        let notes: &[(&str, &str)] = &[("titled.md", "# The Engine Room\n\nprose\n")];
        let result = searched("search-title", CONTRACT, notes, "engine");
        assert_eq!(
            result.hits()[0].snippet(),
            Some("# The Engine Room\n\nprose"),
            "the title is the body's own first heading, so the window quotes it"
        );
    }

    #[test]
    fn a_scalar_alias_declaration_is_one_alias() {
        let notes: &[(&str, &str)] = &[(
            "scalar.md",
            "---\ntype: work\naliases: the Engine\n---\nprose\n",
        )];
        let result = searched("search-scalar-alias", SCALAR_ALIAS, notes, "engine");
        assert_eq!(result.hits()[0].snippet(), Some("the Engine"));
    }

    #[test]
    fn an_aliases_key_on_a_type_that_does_not_declare_it_is_not_matched() {
        // The convention rides the declaration: on the catch-all, which
        // declares no such property, the key is an undeclared key at info and
        // its values never reach the model, so there is nothing to match.
        let notes: &[(&str, &str)] = &[("plain.md", "---\naliases: [engine]\n---\nprose\n")];
        let result = searched("search-undeclared-alias", CONTRACT, notes, "engine");
        assert!(paths(&result).is_empty());
        let reported: Vec<&str> = result
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect();
        assert_eq!(reported, ["note.undeclared-property"]);
    }

    #[test]
    fn filters_compose_with_the_query_under_lists_own_rules() {
        let notes: &[(&str, &str)] = &[
            (
                "active.md",
                "---\ntype: work\nstage: active\ntags: [topic/x]\n---\nengine\n",
            ),
            ("done.md", "---\ntype: work\nstage: done\n---\nengine\n"),
            ("untyped.md", "engine\n"),
        ];
        let filtered = searched_with(
            "search-filters",
            CONTRACT,
            notes,
            &SearchRequest {
                filter: ListFilter {
                    type_name: Some("work".into()),
                    tag: Some("topic/x".into()),
                    lifecycle: Some("active".into()),
                    ordinary: false,
                },
                ..asking("engine")
            },
        );
        assert_eq!(paths(&filtered), ["active.md"]);
        let ordinary = searched_with(
            "search-ordinary",
            CONTRACT,
            notes,
            &SearchRequest {
                filter: ListFilter {
                    ordinary: true,
                    ..ListFilter::default()
                },
                ..asking("engine")
            },
        );
        assert!(
            paths(&ordinary).is_empty(),
            "both typed notes carry a stage, and the untyped note does not participate"
        );
    }

    #[test]
    fn a_lifecycle_filter_against_a_declared_absence_is_refused_before_the_scan() {
        let none = CONTRACT.replace(
            "axis = \"stage\"\nordinary = { absent = true }",
            "none = true",
        );
        let result = searched_with(
            "search-no-axis",
            &none,
            &[("note.md", "engine\n")],
            &SearchRequest {
                filter: ListFilter {
                    lifecycle: Some("active".into()),
                    ..ListFilter::default()
                },
                ..asking("engine")
            },
        );
        assert!(result.hits().is_empty());
        assert_eq!(
            result.diagnostics()[0].id.as_str(),
            "note.lifecycle-axis-absent"
        );
    }

    #[test]
    fn an_invalid_query_is_refused_before_the_scan_with_every_fault_reported() {
        let none = CONTRACT.replace(
            "axis = \"stage\"\nordinary = { absent = true }",
            "none = true",
        );
        let result = searched_with(
            "search-both-faults",
            &none,
            &[("note.md", "engine\n")],
            &SearchRequest {
                filter: ListFilter {
                    ordinary: true,
                    ..ListFilter::default()
                },
                ..asking("\"never closed")
            },
        );
        let reported: Vec<&str> = result
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect();
        assert_eq!(
            reported,
            ["note.lifecycle-axis-absent", "search.invalid-query"],
            "both refusals are reported, and neither ran a scan"
        );
        assert!(result.hits().is_empty());
    }

    #[test]
    fn an_empty_result_is_a_result_and_not_a_diagnostic() {
        let result = searched(
            "search-empty",
            CONTRACT,
            &[("note.md", "prose\n")],
            "absent",
        );
        assert!(result.hits().is_empty());
        assert!(result.diagnostics().is_empty());
    }

    #[test]
    fn the_limit_keeps_the_best_ranked_hits() {
        let notes: &[(&str, &str)] = &[
            ("a.md", "engine\n"),
            ("b.md", "engine engine\n"),
            ("c.md", "engine\n"),
        ];
        let result = searched_with(
            "search-limit",
            CONTRACT,
            notes,
            &SearchRequest {
                limit: 2,
                ..asking("engine")
            },
        );
        assert_eq!(
            paths(&result),
            ["b.md", "a.md"],
            "the most occurrences first, then the path tie-break, then the cut"
        );
    }

    #[test]
    fn matching_more_of_the_query_outranks_matching_one_atom_often() {
        let notes: &[(&str, &str)] = &[
            ("breadth.md", "engine daily\n"),
            ("depth.md", "engine engine engine\n"),
        ];
        let result = searched("search-coverage", CONTRACT, notes, "engine daily");
        assert_eq!(paths(&result), ["breadth.md", "depth.md"]);
    }

    #[test]
    fn identical_runs_produce_identical_results() {
        let notes: &[(&str, &str)] = &[
            ("a.md", "engine\n"),
            ("b.md", "engine\n"),
            ("c/d.md", "# Engine\n"),
        ];
        let first = searched("search-repeat", CONTRACT, notes, "engine");
        let second = searched("search-repeat-again", CONTRACT, notes, "engine");
        assert_eq!(first, second);
    }

    #[test]
    fn a_broken_corpus_surfaces_its_diagnostics_on_every_retrieval() {
        let notes: &[(&str, &str)] = &[
            ("engine.md", "# Engine\n"),
            ("broken.md", "---\ntype: nothing\n---\n"),
        ];
        let result = searched("search-broken", CONTRACT, notes, "engine");
        assert_eq!(paths(&result), ["engine.md"]);
        let reported: Vec<&str> = result
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect();
        assert_eq!(reported, ["note.unknown-type"]);
    }

    #[test]
    fn an_unbound_note_still_matches_and_carries_no_type() {
        let notes: &[(&str, &str)] = &[("broken.md", "---\ntype: nothing\n---\nengine\n")];
        let result = searched("search-unbound", CONTRACT, notes, "engine");
        let hit = &result.hits()[0];
        assert_eq!(hit.type_name(), None);
    }

    #[test]
    fn the_snippet_quotes_the_earliest_body_match_whichever_atom_found_it() {
        // The second atom's match sits before the first atom's, so the window
        // must move back to it: the snippet quotes the earliest match, not
        // the first atom's.
        let body = format!("engine {} daily\n", "x".repeat(120));
        let notes: &[(&str, &str)] = &[("wide.md", &body)];
        let result = searched("search-earliest", CONTRACT, notes, "daily engine");
        let snippet = result.hits()[0].snippet().expect("a body match");
        assert!(snippet.starts_with("engine"), "{snippet}");
    }

    #[test]
    fn the_fallback_alias_is_the_first_in_note_order_whichever_atom_matched_it() {
        let notes: &[(&str, &str)] = &[(
            "known-as.md",
            "---\ntype: work\naliases: [\"Difference Engine\", \"the Machine\"]\n---\nprose\n",
        )];
        let result = searched("search-alias-order", CONTRACT, notes, "machine difference");
        assert_eq!(
            result.hits()[0].snippet(),
            Some("Difference Engine"),
            "the second atom matched the earlier alias, and note order wins"
        );
    }

    #[test]
    fn renderings_share_the_result_and_the_text_folds_line_breaks() {
        // A snippet is corpus text on its way into a line-oriented rendering,
        // so its line breaks fold to spaces; the JSON carries the bytes
        // exactly, escaped rather than emitted.
        let notes: &[(&str, &str)] = &[
            ("multi.md", "# A\nengine here\n"),
            ("engine.md", "unrelated\n"),
        ];
        let result = searched("search-render", CONTRACT, notes, "engine");
        let text = crate::report::search_text(&result);
        assert_eq!(
            text, "engine.md\tcapture\nmulti.md\tcapture\t# A engine here\n",
            "the path-only hit has no snippet column, and the window's line break folds"
        );
        let json = crate::report::search_json(&result, result.diagnostics());
        assert!(json.starts_with("{\n  \"schema_version\": 3,\n  \"report\": \"search\",\n"));
        assert!(json.ends_with("}\n"));
        assert!(json.contains("\"hits\": ["));
        assert!(json.contains("\"snippet\": \"# A\\nengine here\""));
        assert!(json.contains("\"snippet\": null"));
        assert!(json.contains("\"type\": \"capture\""));
        let again = crate::report::search_json(&result, result.diagnostics());
        assert_eq!(json, again, "identical input renders identical bytes");
    }

    #[test]
    fn a_search_result_clones_compares_and_formats() {
        let result = searched(
            "search-derives",
            CONTRACT,
            &[("engine.md", "# Engine\n")],
            "engine",
        );
        let copy = result.clone();
        assert_eq!(copy, result);
        assert!(format!("{result:?}").contains("engine.md"));
        assert_ne!(
            result,
            searched("search-derives-empty", CONTRACT, &[], "engine")
        );
        let request = asking("engine");
        assert_eq!(request.clone(), request);
        assert_ne!(request, asking("daily"));
        assert!(format!("{request:?}").contains("engine"));
    }
}
