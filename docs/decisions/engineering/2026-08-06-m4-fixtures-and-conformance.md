# The `docs` corpus and the M4 scenario set

- Status: accepted
- Date: 2026-08-06

## Context

The `docs` fixture profile has been specified since M1 and scheduled for M4, where a folder-organized, markdown-link, frontmatter-sparse, repeated-basename corpus stresses retrieval hardest ([the profile](../../../conformance/profiles/docs/PROFILE.md), [beta.md](../../beta.md#fixture-profiles)). The corpus is authored fresh; what carries the test value is its *shape*, derived from real repository documentation trees — this repository's own docs tree and `personal-toolchain`'s. The second tree is private, which makes provenance a privacy decision, and the `dense` corpus set the precedent: a numerically derived shape artifact, a source-blind authoring lane, and a human read-back ([the M3 fixtures record](2026-08-03-m3-fixtures-and-conformance.md)).

M4 also decides how the new retrieval scenarios distribute across the three built corpora, under the standing matrix rule that distinguishes a scenario that ran from one pending on an unbuilt corpus.

## Decision

### Shape derivation: numbers only

A derivation step produces one numeric shape-stats artifact from both documentation trees: directory fan-out and depth distributions, basename recurrence rates (how often `README.md` and `index.md` repeat), link-style ratios (path-qualified relative links versus bare names), frontmatter presence rate, and body-length distribution. **No directory name, file name, or content leaves the private tree** — the artifact is counts and distributions, nothing lexical.

### Authoring: blind lane, light gate

A source-blind lane authors the corpus fresh from the stats artifact and the profile's five axes; it never sees either source tree. The gate is lighter than `dense`'s extended privacy gate, deliberately: only structure is derived, from one private source, so the receipt is a single coincidence read-back by the maintainer rather than the full extended procedure. The stats artifact is deleted once the corpus lands, as `dense`'s note-shape artifact was. The rationale for the lighter gate is recorded here so the difference from `dense` reads as a decision, not a lapse.

### The corpus's contract

The `docs` contract exercises the axes the other corpora cannot: the markdown-link dialect setting, and typing that binds through declared defaults because most notes carry no frontmatter — the discriminator must come from the contract, not the note. Folder-borne meaning is expressed entirely by the contract the corpus ships; no folder name reaches the core, which is the seam claim this profile exists to test. The corpus stays at `contract_version = 2`: retrieval reads the existing model, and [the release record](2026-08-06-m4-release-and-cutover.md) states the version's non-motion affirmatively.

### The M4 scenario set

Scenarios split into a universal set, running on all three built corpora, and a `docs`-only set exercising the profile's distinguishing axes.

Universal — `dense`, `starter`, and `docs`:

- `search-membership-by-body-term` — a term present in known bodies returns exactly those notes.
- `search-phrase-matches-adjacent-words` — the quoted form matches in order, and not out of order.
- `search-prefix-wildcard` — the trailing-`*` form.
- `search-composes-with-list-filters` — query AND filters, including the lifecycle axis under both encodings without either vocabulary reaching the core.
- `search-empty-result-is-a-result` — exit `0`, empty membership, no diagnostic.
- `search-repeat-is-deterministic` — two runs, identical bytes.
- `find-resolves-unambiguous-name` and `find-ambiguity-lists-candidates` — the two resolution outcomes, the latter carrying `link.ambiguous-reference` with candidates as evidence.

`docs`-only — the axes no other corpus can express:

- `search-repeated-basenames-stay-distinct` — hits on recurring `README.md`s are distinguished by path, ambiguity as the normal case.
- `find-repeated-basename-requires-qualification` — a bare recurring name is ambiguous; the path-qualified form resolves.
- `markdown-link-resolution` — path-qualified relative links resolve under the dialect setting.
- `frontmatter-sparse-notes-bind-by-default` — a bodyless-frontmatter note types through the contract's declared default and participates in search and filters.

The matrix keeps distinguishing ran from pending, and every floor the new corpus raises is harness-enforced in the change that raises it — both are M3's standing rules, restated here only because the corpus count changes.

### Alternatives considered

- **Every scenario on every corpus.** Rejected: `starter`'s two notes cannot express repeated-basename ambiguity, and vacuous passes dilute what a green row means.
- **All retrieval scenarios `docs`-only.** Rejected: the wikilink side of the dialect axis would ship with zero retrieval coverage.
- **The full extended privacy gate.** Rejected as miscalibrated here: the heavy gate defends content and vocabulary, and neither leaves the source trees; the light gate is recorded with its reasoning instead.
- **Deriving shape from the public tree alone.** Rejected: one tree's shape is exactly what the corpus must not overfit to; the profile names both trees for shape diversity.

## Consequences

- **The light gate is a precedent** the next private-source derivation will cite; if that derivation carries anything lexical, this record's rationale does not transfer and the extended gate applies.
- **The stats artifact is short-lived by design**, so the corpus's provenance is reconstructible only from this record and the derivation code, not from the artifact.
- **`docs` lands as the third live column in every matrix**, which grows the conformance run and the smoke sequence; the release record carries the green-before-tag obligation.
- **Default-bound typing becomes load-bearing for the first time** — a defect in declared-default binding now fails retrieval scenarios, not just validation ones.
