//! The four situations the `docs` corpus exhibits **natively**, derived into
//! whichever corpus each case runs against.
//!
//! These four scenarios were written as the M4 record's `docs`-only half, and
//! the record's amendment adjudicated what that could mean under a rule that
//! admits no waivers: the harness structurally cannot scope a scenario to one
//! profile, so each case *derives* its situation — the recurring basename, the
//! reference in the declared dialect, the frontmatter-less note — into a fresh
//! copy of whatever corpus it is handed, exactly as the standing ambiguity
//! cases already double a name anywhere.
//!
//! What "docs-only" names after that is the corpus that exhibits these
//! situations **in its committed form**, which is a claim about a fixture
//! rather than about a scenario, and `tests/floors.rs` is where it is
//! asserted — of `docs` alone. Nothing here reads that claim: every needle,
//! name and reference below is invented and planted, so each case runs
//! meaningfully against every profile whose corpus is built.
//!
//! Every expected set is nonempty by construction and says so. A membership
//! assertion over an empty set passes without evidence, which is the one way
//! a derived case can go green having tested nothing.

use std::collections::BTreeSet;

use dogtag::contract::LinkDialect;
use dogtag::note::{ListFilter, Note, Reference, SearchHit, list};

use super::corpus::Corpus;
use super::derive::{self, Planting};
use super::expect::{
    Checked, Subject, require, require_clean, require_contains, require_same_names,
};
use super::{find, search};

// ---------------------------------------------------------------------------
// The recurring basename, planted: `search-repeated-basenames-stay-distinct`
// and `find-repeated-basename-requires-qualification`.
// ---------------------------------------------------------------------------

/// The basename the recurrence cases plant, invented so that no committed
/// corpus can already bear it — and so the count assertion below reads the
/// derivation's own work rather than a coincidence.
const RECURRING_FILE: &str = "quorvex-readme.md";

/// The bare name that file spells: what a reference or a lookup would have to
/// pick one bearer of.
const RECURRING_NAME: &str = "quorvex-readme";

/// The word the search case queries for, planted in every bearer that carries
/// one.
const RECURRING_TERM: &str = "vondramyx";

/// One bearer of the recurring basename.
struct Bearer {
    /// The invented directory it sits in.
    directory: &'static str,
    /// Its own invented word, which its snippet must quote: a snippet taken
    /// from one bearer and reported under another's path is exactly the
    /// misattribution this scenario forbids.
    marker: &'static str,
    /// Whether this bearer carries the queried term. The one that does not is
    /// what makes the search assertion narrower than "every sharer".
    carries: bool,
}

/// The bearers planted, all under one basename and none of them the same note.
const BEARERS: [Bearer; 4] = [
    Bearer {
        directory: "derived-recurring/alpha",
        marker: "alphavex",
        carries: true,
    },
    Bearer {
        directory: "derived-recurring/beta",
        marker: "betavex",
        carries: true,
    },
    Bearer {
        directory: "derived-recurring/gamma",
        marker: "gammavex",
        carries: true,
    },
    Bearer {
        directory: "derived-recurring/delta",
        marker: "deltavex",
        carries: false,
    },
];

impl Bearer {
    /// Where this bearer lands.
    fn path(&self) -> String {
        format!("{}/{RECURRING_FILE}", self.directory)
    }

    /// What it holds: one line, so the matched context and the marker are
    /// within a snippet's reach of each other.
    fn body(&self) -> String {
        if self.carries {
            format!("A {RECURRING_TERM} passage, {}.\n", self.marker)
        } else {
            format!("No passage at all, {}.\n", self.marker)
        }
    }
}

/// `search-repeated-basenames-stay-distinct`.
pub fn repeated_basenames_stay_distinct(corpus: &Corpus) -> Checked {
    let derived = plant_recurrence(corpus, "search-repeated-basenames")?;
    let matching: Vec<&Bearer> = BEARERS.iter().filter(|bearer| bearer.carries).collect();
    require(matching.len() >= 2, || too_few_bearers(matching.len()))?;
    let expected: Vec<String> = matching.iter().map(|bearer| bearer.path()).collect();
    let borrowed: Vec<&str> = expected.iter().map(String::as_str).collect();

    let result = search::searched(&derived, RECURRING_TERM)?;
    require_clean(
        result.diagnostics(),
        Subject::new("the search over the recurring basename"),
    )?;
    // Membership and count together: a hit per bearer, no bearer dropped for
    // another, and no two bearers merged into one hit under a shared name.
    search::require_hits(&result, &borrowed, "the query over the recurring basename")?;
    for hit in result.hits() {
        let marker = marker_at(hit)?;
        let quoted = hit
            .snippet()
            .is_some_and(|snippet| snippet.contains(marker));
        require(quoted, || misattributed(hit, marker))?;
    }
    Ok(())
}

/// `find-repeated-basename-requires-qualification`.
pub fn repeated_basename_requires_qualification(corpus: &Corpus) -> Checked {
    let derived = plant_recurrence(corpus, "find-repeated-basename")?;
    let bearers: Vec<String> = BEARERS.iter().map(Bearer::path).collect();
    require(bearers.len() >= 2, || too_few_bearers(bearers.len()))?;

    let bare = find::found(&derived, RECURRING_NAME)?;
    require(bare.note().is_none(), || {
        find::resolved_ambiguity(RECURRING_NAME)
    })?;
    require_same_names(
        &bearers,
        &find::caller_refusal_candidates(&bare)?,
        "the refusal's related evidence",
    )?;

    // The path is what picks one bearer, under either spelling the standing
    // routing rule accepts: with the extension, and without it.
    for bearer in &bearers {
        for spelling in [bearer.as_str(), bearer.trim_end_matches(".md")] {
            let result = find::found(&derived, spelling)?;
            require_clean(result.diagnostics(), &format!("finding `{spelling}`"))?;
            let note = result.note().ok_or_else(|| unqualified(spelling))?;
            require(note.path().as_str() == bearer, || {
                find::wrong_bearer(spelling, note.path().as_str())
            })?;
        }
    }
    Ok(())
}

/// A copy of the corpus holding every bearer, having proved the name recurs.
///
/// The count is the construction guarantee both recurrence cases rest on: the
/// planted notes are all there, the SDK reads the name they share as one name,
/// and nothing committed was already bearing it.
fn plant_recurrence(corpus: &Corpus, label: &str) -> Result<Corpus, String> {
    let planted: Vec<(String, String)> = BEARERS
        .iter()
        .map(|bearer| (bearer.path(), bearer.body()))
        .collect();
    let notes: Vec<(&str, &str)> = planted
        .iter()
        .map(|(path, text)| (path.as_str(), text.as_str()))
        .collect();
    let derived = derive::derived_planting(
        corpus,
        &Planting {
            label,
            notes: &notes,
        },
    )?;
    let bearing = derive::notes(&derived)?
        .notes()
        .iter()
        .filter(|note| note.name() == RECURRING_NAME)
        .count();
    require(bearing == BEARERS.len(), || not_recurring(bearing))?;
    Ok(derived)
}

/// The marker belonging to the bearer this hit answered.
fn marker_at(hit: &SearchHit) -> Result<&'static str, String> {
    BEARERS
        .iter()
        .find(|bearer| bearer.path() == hit.path().as_str())
        .map(|bearer| bearer.marker)
        .ok_or_else(|| unplanted_hit(hit))
}

// ---------------------------------------------------------------------------
// The declared link dialect: `markdown-link-resolution`.
// ---------------------------------------------------------------------------

/// `markdown-link-resolution`.
///
/// The dialect is the contract's, so the case writes its references in
/// whichever one this corpus declares and spells its counter-example in the
/// other. That is what makes the scenario answerable against every profile:
/// `docs` is the markdown side of the axis and the others are the wikilink
/// side, and the rule under test — one dialect per corpus, declared once and
/// never sniffed per link — is the same rule on both.
pub fn dialect_links_resolve(corpus: &Corpus) -> Checked {
    let declared = corpus.clean_contract()?.dialect().links();
    let foreign = other_dialect(declared)?;
    let nested = "derived-dialect/target.md";
    // A root-level target carries no `/`, so its trailing `.md` is the whole
    // of what qualifies it as a path.
    let root = "derived-dialect-root.md";
    let source = "derived-dialect/source.md";
    let unread = spelled(foreign, nested);
    let body = format!(
        "# A derived source\n\nThe nested target: {}.\n\nThe root-level target: {}.\n\nA \
         reference in the dialect this corpus does not declare: {unread}.\n",
        spelled(declared, nested),
        spelled(declared, root),
    );
    let derived = derive::derived_planting(
        corpus,
        &Planting {
            label: "dialect-links",
            notes: &[
                (nested, "# The nested target\n"),
                (root, "# The root-level target\n"),
                (source, &body),
            ],
        },
    )?;

    let notes = derive::notes(&derived)?;
    require_clean(
        notes.diagnostics(),
        Subject::new("the corpus holding the derived references"),
    )?;
    let written = notes
        .notes()
        .iter()
        .find(|note| note.path().as_str() == source)
        .ok_or_else(|| unplanted_source(source))?;

    // Two references, not three: the foreign spelling is bytes in the body,
    // which the containment assertion below shows it is still sitting in.
    let expected = [nested.to_owned(), root.to_owned()];
    let references = written.body_references();
    require(references.len() == expected.len(), || {
        wrong_reference_count(expected.len(), references.len())
    })?;
    let resolved: Vec<String> = references
        .iter()
        .map(|reference| {
            reference
                .target()
                .map(|target| target.as_str().to_owned())
                .ok_or_else(|| unresolved_reference(reference))
        })
        .collect::<Result<_, String>>()?;
    require_same_names(&expected, &resolved, "the resolved references")?;
    require_contains(
        written.body(),
        &unread,
        Subject::new("the derived source's body"),
    )
}

/// A reference to `target`, spelled the way `dialect` spells one.
fn spelled(dialect: LinkDialect, target: &str) -> String {
    match dialect {
        LinkDialect::Wikilink => format!("[[{target}]]"),
        LinkDialect::Markdown => format!("[the target]({target})"),
    }
}

/// A dialect the contract does not declare, for the counter-example.
fn other_dialect(declared: LinkDialect) -> Result<LinkDialect, String> {
    LinkDialect::ALL
        .iter()
        .copied()
        .find(|dialect| *dialect != declared)
        .ok_or_else(|| no_other_dialect(declared))
}

// ---------------------------------------------------------------------------
// Binding through the declared default:
// `frontmatter-sparse-notes-bind-by-default`.
// ---------------------------------------------------------------------------

/// The word the sparse case queries for.
const SPARSE_TERM: &str = "wexlorbin";

/// The frontmatter-less note: the discriminator comes from the contract.
const SPARSE_IMPLICIT: &str = "derived-sparse/implicit.md";

/// Its twin, which says out loud what the contract tells the other one.
const SPARSE_EXPLICIT: &str = "derived-sparse/explicit.md";

/// `frontmatter-sparse-notes-bind-by-default`.
///
/// The pair is the assertion: two notes of one type, one having said so and
/// one having said nothing at all, must differ in exactly one respect — *what*
/// bound them — and in no other. Retrieval must not be able to tell them
/// apart.
pub fn sparse_notes_bind_by_default(corpus: &Corpus) -> Checked {
    let contract = corpus.clean_contract()?;
    // A contract that declares no catch-all does not resolve at all — it earns
    // `contract.missing-catch-all`, which `clean_contract` above has already
    // refused — so this is a guard on an `Option` rather than a reachable arm.
    let catch_all = contract
        .catch_all()
        .ok_or_else(no_catch_all)?
        .name()
        .to_owned();
    let explicit = format!("---\ntype: {catch_all}\n---\n# Told\n\n{SPARSE_TERM} is here too.\n");
    let derived = derive::derived_planting(
        corpus,
        &Planting {
            label: "sparse-binding",
            notes: &[
                (
                    SPARSE_IMPLICIT,
                    &format!("# Untold\n\n{SPARSE_TERM} is here.\n"),
                ),
                (SPARSE_EXPLICIT, &explicit),
            ],
        },
    )?;
    let notes = derive::notes(&derived)?;
    require_clean(
        notes.diagnostics(),
        Subject::new("the corpus holding the frontmatter-less note"),
    )?;

    for (path, bound_by) in [
        (SPARSE_IMPLICIT, "catch-all"),
        (SPARSE_EXPLICIT, "declaration"),
    ] {
        let note = notes
            .notes()
            .iter()
            .find(|note| note.path().as_str() == path)
            .ok_or_else(|| unplanted_source(path))?;
        require(note.binding().bound_by() == bound_by, || {
            wrong_binding(note, bound_by)
        })?;
        require(
            note.binding().type_name() == Some(catch_all.as_str()),
            || wrong_bound_type(note, &catch_all),
        )?;
    }

    let both = [SPARSE_IMPLICIT, SPARSE_EXPLICIT];
    search::require_hits(
        &search::searched(&derived, SPARSE_TERM)?,
        &both,
        "the search over the frontmatter-less note and its explicit twin",
    )?;
    // The type filter answers for the frontmatter-less note under the type
    // the contract bound it to, at both doors that take one.
    let filter = ListFilter {
        type_name: Some(catch_all.clone()),
        ..ListFilter::default()
    };
    search::require_hits(
        &search::searched_with(&derived, SPARSE_TERM, filter.clone())?,
        &both,
        &format!("the same search narrowed to `{catch_all}`"),
    )?;
    let listed: BTreeSet<String> = list(&derived.vault_root()?, &contract, &filter)
        .notes()
        .iter()
        .map(|note| note.path().as_str().to_owned())
        .collect();
    for path in both {
        require(listed.contains(path), || unlisted(path, &catch_all))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The details only a misbehaving SDK — or a derivation that stopped deriving —
// can produce, built as functions so the text a failing run would print is
// itself tested. The closures that raise them can never run under a passing
// suite.
// ---------------------------------------------------------------------------

/// The bearer table stopped describing a recurrence.
fn too_few_bearers(counted: usize) -> String {
    format!(
        "a recurring name needs at least two bearers to be recurring at all, and {counted} were \
         planted, so the assertion would run over nothing"
    )
}

/// The planted name does not recur in the corpus the SDK read.
fn not_recurring(counted: usize) -> String {
    format!(
        "the derived corpus must hold {} notes named `{RECURRING_NAME}` and holds {counted}: \
         either the planting did not land, or something committed already bore the name",
        BEARERS.len()
    )
}

/// A hit quoted context that is not its own.
fn misattributed(hit: &SearchHit, marker: &str) -> String {
    format!(
        "`{}`'s snippet must quote its own `{marker}`, or one note's context has been attributed \
         to another note's identity",
        hit.path()
    )
}

/// A hit arrived under a path nothing was planted at.
fn unplanted_hit(hit: &SearchHit) -> String {
    format!(
        "`{}` was answered, and no bearer was planted there",
        hit.path()
    )
}

/// A path-qualified spelling refused to resolve.
fn unqualified(spelling: &str) -> String {
    format!("the path-qualified `{spelling}` names exactly one bearer and must resolve")
}

/// A note the derivation planted was not read back.
fn unplanted_source(path: &str) -> String {
    format!("the planted `{path}` is not in the corpus the SDK read")
}

/// The body's references were miscounted — the foreign spelling was read as a
/// link, or a declared one was not.
fn wrong_reference_count(expected: usize, counted: usize) -> String {
    format!(
        "the body writes {expected} references in the declared dialect and one in the other, \
         which is bytes; {counted} references were read"
    )
}

/// A path-qualified reference in the declared dialect resolved to nothing.
fn unresolved_reference(reference: &Reference) -> String {
    format!(
        "the reference `{}` names a planted note by path and must resolve to it",
        reference.written()
    )
}

/// The format declares only one link dialect, so nothing is foreign to it.
fn no_other_dialect(declared: LinkDialect) -> String {
    format!(
        "the counter-example needs a dialect the contract does not declare, and `{declared}` is \
         the only one the format defines"
    )
}

/// A resolved contract carried no catch-all, which resolution forbids.
fn no_catch_all() -> String {
    "every contract that resolves declares exactly one catch-all type, and this one carries none"
        .to_owned()
}

/// A note bound through something other than what its frontmatter says.
fn wrong_binding(note: &Note, expected: &str) -> String {
    format!(
        "`{}` must report binding by `{expected}`, and reports `{}`",
        note.path(),
        note.binding().bound_by()
    )
}

/// A note bound to a type other than the declared catch-all.
fn wrong_bound_type(note: &Note, expected: &str) -> String {
    format!(
        "`{}` must bind to `{expected}`, and bound to {:?}",
        note.path(),
        note.binding().type_name()
    )
}

/// The type filter did not answer for a note bound to that very type.
fn unlisted(path: &str, type_name: &str) -> String {
    format!("`{path}` bound to `{type_name}` and must answer that type's filter")
}

#[cfg(test)]
mod tests {
    use super::super::derive::plant;
    use super::*;

    /// A contract per dialect, so the dialect case can be run against both
    /// sides of the axis without either fixture corpus in hand.
    fn contract_of(dialect: LinkDialect) -> String {
        format!(
            concat!(
                "contract_version = 3\n",
                "\n[dialect]\nlinks = \"{}\"\n",
                "\n[lifecycle]\nnone = true\n",
                "\n[[type]]\nname = \"person\"\ncapabilities = [\"identity-bearing\"]\n",
                "\n  [[type.property]]\n  name = \"name\"\n  kind = \"string\"\n",
                "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
            ),
            dialect.as_str()
        )
    }

    /// A one-note corpus under the named dialect.
    fn tiny(label: &str, dialect: LinkDialect) -> Corpus {
        let corpus = Corpus::holding(label, &contract_of(dialect));
        plant(&corpus, "committed/a.md", "# A committed note\n").expect("a note");
        corpus
    }

    /// Where the sample note the detail tests name sits. It is deliberately
    /// **not** one of the planted bearers, so `marker_at` answers for it the
    /// way it answers for any path outside the table.
    const SAMPLE: &str = "sample.md";

    /// A corpus holding one note that carries the recurrence term, a marker
    /// word and one resolving reference.
    ///
    /// The details below are about a hit, a note and a reference, so the tests
    /// name real ones the SDK produced rather than values invented for the
    /// occasion — the same reason `search`'s detail tests search a corpus for
    /// their sample hit.
    fn sample() -> Corpus {
        let corpus = tiny("native-sample", LinkDialect::Wikilink);
        plant(
            &corpus,
            SAMPLE,
            &format!("# Sample\n\n{RECURRING_TERM}, alphavex, [[committed/a]].\n"),
        )
        .expect("a note");
        corpus
    }

    /// Every case derives, so every case owes the same cleanliness before it.
    #[test]
    fn a_corpus_that_is_not_clean_refuses_every_derived_case() {
        let corpus = tiny("native-dirty", LinkDialect::Wikilink);
        plant(&corpus, "broken.md", "---\ntype: nothing\n---\n").expect("a broken note");
        for (case, name) in [
            (
                repeated_basenames_stay_distinct as fn(&Corpus) -> Checked,
                "search recurrence",
            ),
            (repeated_basename_requires_qualification, "find recurrence"),
            (dialect_links_resolve, "dialect"),
            (sparse_notes_bind_by_default, "sparse binding"),
        ] {
            let detail = case(&corpus).expect_err(name);
            assert!(detail.contains("the corpus before"), "{name}: {detail}");
        }
    }

    /// The recurrence is the derivation's own: a committed note already
    /// bearing the planted name is a failure naming the count, not a quietly
    /// widened candidate list.
    #[test]
    fn a_committed_bearer_of_the_planted_name_fails_both_recurrence_cases() {
        let corpus = tiny("native-taken-name", LinkDialect::Wikilink);
        plant(&corpus, &format!("committed/{RECURRING_FILE}"), "# Here\n").expect("a note");
        for (case, name) in [
            (
                repeated_basenames_stay_distinct as fn(&Corpus) -> Checked,
                "search recurrence",
            ),
            (repeated_basename_requires_qualification, "find recurrence"),
        ] {
            let detail = case(&corpus).expect_err(name);
            assert!(detail.contains("already bore the name"), "{name}: {detail}");
        }
    }

    /// A committed note carrying a planted needle widens the expected set, and
    /// the membership assertions say so rather than accepting the surplus.
    #[test]
    fn a_committed_needle_fails_the_membership_cases() {
        let corpus = tiny("native-widened", LinkDialect::Wikilink);
        plant(
            &corpus,
            "taken.md",
            &format!("{RECURRING_TERM} and {SPARSE_TERM}\n"),
        )
        .expect("a note");
        for (case, name) in [
            (
                repeated_basenames_stay_distinct as fn(&Corpus) -> Checked,
                "search recurrence",
            ),
            (sparse_notes_bind_by_default, "sparse binding"),
        ] {
            let detail = case(&corpus).expect_err(name);
            assert!(detail.contains("taken.md"), "{name}: {detail}");
        }
    }

    /// The dialect case runs on both sides of the axis: the declared spelling
    /// resolves and the other one is bytes, whichever way round they are.
    #[test]
    fn the_dialect_case_holds_under_either_declared_dialect() {
        for dialect in LinkDialect::ALL.iter().copied() {
            let corpus = tiny(&format!("native-dialect-{dialect}"), dialect);
            dialect_links_resolve(&corpus)
                .unwrap_or_else(|detail| panic!("under `{dialect}`: {detail}"));
        }
    }

    /// A corpus declaring no catch-all cannot demonstrate default binding —
    /// and cannot be opened either: the absence is refused at contract
    /// resolution, one door before this case looks for the type. The refusal
    /// the case reports is therefore the contract's, not the binding's.
    #[test]
    fn a_corpus_with_no_catch_all_is_refused_before_the_sparse_case_looks() {
        let contract = concat!(
            "contract_version = 3\n",
            "\n[dialect]\nlinks = \"wikilink\"\n",
            "\n[lifecycle]\nnone = true\n",
            "\n[[type]]\nname = \"person\"\ncapabilities = [\"identity-bearing\"]\n",
        );
        let corpus = Corpus::holding("native-no-catch-all", contract);
        let detail = sparse_notes_bind_by_default(&corpus)
            .expect_err("the case needs a declared default to bind through");
        assert!(detail.contains("contract.missing-catch-all"), "{detail}");
    }

    /// A passing suite can never run the closures that raise these, so the
    /// text itself is held here: each detail names its subject and what
    /// actually arrived.
    #[test]
    fn the_details_a_misbehaving_sdk_would_earn_say_what_went_wrong() {
        let corpus = sample();
        let read = derive::notes(&corpus).expect("the sample corpus reads");
        let note = read
            .notes()
            .iter()
            .find(|note| note.path().as_str() == SAMPLE)
            .expect("the sample note");
        let result = search::searched(&corpus, RECURRING_TERM).expect("a clean search");
        let hit = &result.hits()[0];

        assert!(too_few_bearers(1).contains("and 1 were planted"));
        assert!(not_recurring(9).contains("and holds 9"));
        let misattributed = misattributed(hit, "alphavex");
        assert!(misattributed.contains("`alphavex`"), "{misattributed}");
        assert!(
            misattributed.contains("another note's identity"),
            "{misattributed}"
        );
        assert!(unplanted_hit(hit).contains("no bearer was planted"));
        assert!(unqualified("a/b.md").contains("`a/b.md`"));
        assert!(unplanted_source("a.md").contains("is not in the corpus"));
        assert!(wrong_reference_count(2, 3).contains("3 references were read"));
        let dangling = unresolved_reference(&note.body_references()[0]);
        assert!(dangling.contains("must resolve to it"), "{dangling}");
        assert!(dangling.contains("committed/a"), "{dangling}");
        let only = no_other_dialect(LinkDialect::Markdown);
        assert!(only.contains("`markdown`"), "{only}");
        assert!(no_catch_all().contains("exactly one catch-all"));
        assert!(wrong_binding(note, "declaration").contains("reports `catch-all`"));
        assert!(wrong_bound_type(note, "person").contains("bound to Some(\"capture\")"));
        assert!(unlisted("a.md", "capture").contains("must answer that type's filter"));
    }

    /// A hit under a path no bearer was planted at earns its own detail. The
    /// lookup is total over the planted table, so the honest way to reach the
    /// refusal is to ask it about a hit the derivation did not plant.
    #[test]
    fn a_hit_outside_the_planted_table_has_no_marker() {
        let corpus = sample();
        let result = search::searched(&corpus, RECURRING_TERM).expect("a clean search");
        let detail = marker_at(&result.hits()[0]).expect_err("nothing was planted there");
        assert!(detail.contains(SAMPLE), "{detail}");
    }
}
