# Conformance

Every scenario runs against every fixture profile. **There are no waivers.** A scenario expressible against only one profile fails the harness and is triaged as either an incomplete configuration model or a personal convention mistaken for an invariant. This rule — not prose review — is what keeps one author's assumptions out of the kernel; see [beta.md](../docs/beta.md) (required properties) and [architecture.md](../docs/architecture.md).

## Layout

```
conformance/
  harness/       # crate `dogtag-conformance` (publish = false): loads, validates, cross-products, runs
  scenarios/     # one TOML per golden scenario; the harness asserts the exact count
  profiles/      # one directory per fixture profile: PROFILE.toml (parsed) + PROFILE.md (spec),
                 # plus corpus/ once built — and nothing else; a stray entry is a load error
    dense/  starter/  docs/  records/
```

## Scenarios

A scenario is a contract first and a test second:

```toml
id = "capability-cardinality-enforced"        # kebab-case, equals the filename stem
title = "Capability enumeration and cardinality are enforced when the contract loads"
milestone = "M2"                              # when it becomes executable (M2 | M3 so far)
status = "executable"                         # pending | executable — flips at its milestone
contract = """
Given/when/then prose in profile-agnostic terms.
"""
```

Contract prose binds behavior to **declared capabilities and declared axes** — "the declared catch-all type", "the declared lifecycle axis" — never to any corpus's vocabulary. No scenario may mention a type name, a lifecycle word, or a dialect assumption; vocabulary lives in profiles, and only declarations reach the core.

Which milestone a scenario carries follows what it operates on: a scenario that opens and diagnoses a vault and its committed contract is M2, and one that validates the corpus's notes is M3. Four scenarios written at M0 moved from M2 to M3 on that test when the M2 packet closed, and eight M2 scenarios were added alongside them — a metadata correction, not a waiver, since every scenario still runs against every profile.

### Non-conforming inputs are derived, never authored

A scenario that needs a broken contract — no catch-all type, two of them, an unsupported version, a missing lifecycle declaration — does not get a checked-in broken fixture. The harness copies **each profile's own contract** into a temporary directory and transforms it there.

That is not a convenience. A hand-authored broken contract has to be written in *some* corpus's vocabulary, which makes it a profile-specific fixture wearing a shared-sounding name — the exact shape this suite exists to catch. Deriving the input runs one assertion against every profile's vocabulary by construction, and no broken contract is checked in anywhere.

**A transformation must prove it changed something.** Each derived case asserts three things: the untransformed contract loads clean, the transformed bytes differ from the original, and the expected diagnostic identifier appears. Without the middle assertion, a transformation that fails to find its target — because one profile spells a table where another spells an array of tables — silently tests nothing. The transformations are textual for the same reason: one that parsed the document and re-serialised it would change the bytes through formatting alone, and the middle assertion would pass without the transformation having found anything. A transformation that cannot find its target is an error, never a no-op, and each one carries its own tests — a case that finds its target and changes the bytes, and a case where the target is absent and it reports failure.

Two input shapes are **not** contract transformations, and the rule says so rather than implying a coverage it does not have. The machine-local installation record has no per-profile source, so its cases run one identical input against every profile; the per-profile half of that scenario is the second assertion, that the profile's own contract is unaffected by anything the record tried. The three discovery scenarios need a synthetic directory *tree*, where the profile contributes a contract the sentinel test never parses, so what varies between profiles there is nothing. **Those are one case each, not four**, and the printed cross product should not be read as four.

The discovery tree is also **hermetic**, which is not a nicety. The upward walk has no boundary — not `.git`, not `$HOME`, not a mount point — so "no vault above here" would otherwise be a property of the machine, and one developer's directory layout would be reaching a conformance result. Before asserting, the harness walks from its temporary start directory to the filesystem root and proves no ancestor holds the sentinel, failing loudly with an explanatory message if one does.

Every corpus-backed case runs against a **temporary copy** of the profile's corpus, created fresh per pair, with normalized permissions. The run writes into that copy — installation records, nested directories, symbolic links, transformed contracts — so nothing it does touches the checkout, and a developer's umask cannot change a result.

### The no-waiver rule is structural

The scenario schema has **no field that could name a profile**. Deserialization uses serde's `deny_unknown_fields`, so an added `profiles = ["dense"]`, `skip`, `waive`, or `only` key fails parsing instead of creating an exemption — the place where a personal invariant would hide does not exist in the format. The profile schema is locked the same way in the other direction: a profile has no field with which to name a scenario. Harness tests assert both rejections (`waiver_shaped_fields_fail_scenario_parsing`, `waiver_shaped_fields_fail_profile_parsing`); if either test ever fails, someone widened the schema, and that is the exact change this suite exists to forbid.

One channel is **not** closed structurally, and pretending otherwise would be worse than naming it: `corpus = "scheduled"` removes a profile from every scenario at once. The loader checks only that the declared status matches the disk, so deleting a corpus directory and reverting the status would be a mechanically valid way to make a failing profile stop failing. The rule that closes it is written rather than typed — **a corpus that has been `built` never returns to `scheduled`** — and the harness now ratchets it: `CORPORA_EVER_BUILT` names `dense` and `starter`, and a profile named there that declares `scheduled` fails to load. The list only grows, so un-building a corpus means deleting a line from it — a named, greppable, reviewable act rather than a quiet status flip. Rejected alternatives — declared-and-reviewed exemptions, a two-tier shared/profile-specific split — both make single-profile scenarios routine, and an exemption list is exactly where a personal invariant hides.

## Profiles

Four profiles, each standing for one persona and together spreading across every axis the configuration seam claims to absorb — type taxonomy, capability assignment, property requirements, predicate vocabulary, lifecycle encoding, name resolution, and dialect. The roster is exact: the harness fails if a profile is missing or a fifth appears unspecified. See each profile's `PROFILE.md` for its full specification.

| Profile | Stands for | Contract built |
| --- | --- | --- |
| `dense` | the PKM enthusiast with an established corpus | M2 |
| `starter` | a fresh install | M2 |
| `docs` | the dev team | M4 (lexical retrieval) |
| `records` | the decision maker | pre-E1 (before assisted adoption) |

### Why the specifications land before the corpora

The committed vault-contract format is an M2 decision. A fixture corpus written before the format exists would freeze a guess and then defend it. So all four profile *specifications* landed at M1 — early enough that no scenario could be written with one profile's axes quietly assumed away — and each corpus lands at the milestone that defines what it must be written in (`dense` and `starter`, at M2) or the milestone whose hypothesis it stresses (`docs`, `records`).

### What `built` means

`corpus = "built"` means **the fixture vault exists**: a vault root and its committed contract. It does not promise notes. No M2 scenario reads a note, and notes for `dense` and `starter` land at M3 alongside the document model that defines how they are written — authoring them earlier would freeze a guess about a format M3 decides.

The schema deliberately gained no third status for this. Any state between `scheduled` and `built` is somewhere a profile can sit indefinitely while the matrix reports something other than a skip, which is what a waiver looks like from the inside. Each `PROFILE.md` states exactly what its corpus holds at each stage instead.

## The harness

`cargo test -p dogtag-conformance` (or `just conformance`, which adds `--nocapture` so the matrix prints):

1. every scenario file parses under the strict schema, with unique kebab-case ids equal to their filename stems;
2. waiver-shaped fields fail parsing, on scenarios and on profiles;
3. the report is the full **scenarios × profiles cross product** — every pair present exactly once, |report| = |scenarios| × |profiles| — and each pair's outcome follows from the two facts about it: an executable scenario against a built corpus *runs*, and everything else is pending on the scenario, on the corpus, or on both;
4. the profile roster is exactly the four above, and a corpus named in `CORPORA_EVER_BUILT` is built;
5. every scenario tagged with the current milestone is `executable`, so a straggler fails the suite rather than sitting out a graduation;
6. the matrix prints, one row per scenario, one column per profile, every cell filled.

The matrix distinguishes a pair that **ran** from one **skipped** for want of a corpus, because a run reaching two of four profiles must not read as a complete matrix. Five cells, each named in a legend printed beneath the table and counted separately in the summary line:

| cell | meaning |
| --- | --- |
| `pass` | ran and passed |
| `FAIL` | ran and failed; the detail prints beneath the matrix |
| `pending` | the scenario is still prose; the corpus is built |
| `no-corpus` | the scenario is executable; the corpus is not built — a **skip**, not a result |
| `pending,no-corpus` | both |

At M2 that is 19 scenarios × 4 profiles = 76 pairs, of which 10 × 2 = 20 ran: the ten M2 scenarios against `dense` and `starter`. M2's cross-profile evidence is those two profiles, not four.

The harness crate depends on `serde`, `toml`, and the **`dogtag` SDK**, whose *public API only* it consumes — the same door any other consumer enters by. That makes conformance a permanent test that the public API is sufficient: a private hook the harness turned out to need would be an architecture bug, not a reason to widen anything.

## Graduating a scenario

A scenario graduates by flipping `status = "pending"` to `"executable"` at its milestone, alongside the execution wiring in the harness. Graduation is all-or-nothing: an executable scenario runs against **every profile whose corpus is built**, and the harness refuses to produce a report at all — rather than quietly marking pairs pending — if a runnable pair exists without an execution path. There is no partial graduation, no per-profile rollout, and no mechanism to exclude a profile that fails. The execution path the report takes is **not a filter**: an executor cannot decline a pair, because answering "no execution path" refuses the whole report instead of skipping the pair.

The corpus axis is the honest qualifier on that sentence. A pair whose corpus is still `scheduled` reports pending *on the corpus*, and the printed matrix must say so rather than rendering it identically to a scenario nobody has written — otherwise a run against two of four profiles reads as a complete matrix. Two things follow. A milestone's real cross-profile evidence is the profiles whose corpora exist at that milestone, not the width of the table. And **a scenario whose Given describes notes may not graduate against a corpus that holds none**: the loader checks only that `corpus/` is a directory, so a note scenario run against a contract-only corpus would satisfy "every note …" vacuously over the empty set and report green.
