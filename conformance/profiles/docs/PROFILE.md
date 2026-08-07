# Fixture profile: `docs`

**Stands for:** the dev team.
**Corpus:** built at M4 (lexical retrieval), where a folder-organized markdown-link corpus stresses retrieval hardest.

## Distinguishing axes

- **Repository documentation shapes.** The corpus looks like a software repository's docs tree, not a curated vault: guides, references, decision records, per-directory indexes.
- **Folder organization.** Meaning lives in the directory structure rather than in per-note metadata; the contract must express that without the core learning any folder's name.
- **Markdown-link dialect.** References are standard markdown links, frequently path-qualified and relative. The dialect axis's other side from `dense`'s wikilinks.
- **Most files carrying no frontmatter.** The majority of notes have a body and nothing else, stressing how typing and required-property validation behave when the discriminator must come from declared defaults rather than explicit frontmatter.
- **Repeated basenames.** `README.md` recurs across the tree — repeated names as the *normal* case, not an edge case — stressing bare-name ambiguity reporting and path-qualified resolution.

## What the fixture is

A corpus authored fresh for the fixture, with its shape derived from real repository documentation trees (the product repository's own docs among them). **No existing file is copied and no real project, person or organization appears in it**; the shape is what carries the test value, and it was taken as counts and proportions alone.

The invented subject is *Quillon*, a fleet telemetry gateway: forty-four pages of guides, reference material, decision records, per-directory indexes, release notes and internals. Nothing about the product is real and nothing in it describes this repository.

## What the corpus holds

A vault root, its committed contract, and forty-four notes over sixteen directories. The shape, and the band each figure is held to by `conformance/harness/tests/floors.rs`:

| Property | This corpus | Floor |
| --- | --- | --- |
| Notes | 44 | 35–50 |
| Directories | 16 | 12–18 |
| Deepest note | 4 directories down | at least 4 |
| Directories holding a `README.md` | 5 of 16 (0.31) | 0.25–0.35 |
| Basenames borne by more than one note | 3 (`README.md` ×5, `overview.md` ×4, `limits.md` ×2) | at least 3 |
| Notes carrying no frontmatter | 31 of 44 (0.70) | a majority |
| Internal references | 225, all resolving | at least 3 per note, none dangling |
| References that leave the vault | 4 | 2–8 |
| Wikilinks | 0 | none |

Body lengths run from 26 to 105 lines with a median of 35, and every note carries an `# H1`.

## What the contract declares

- **Dialect `markdown`.** The other side of the dialect axis from `dense`.
- **A thin taxonomy.** Seven types for forty-four notes, because in a docs tree a guide is a guide by virtue of sitting under `guides/`. What the contract declares is the small set of shapes a note may *opt into*; the folder-borne meaning stays in the tree, where the core never sees it. A floor asserts the negative directly: no declared type name, property name, predicate or namespace prefix is spelled the way any directory in the corpus is, so the contract alone could not be used to recover the folder structure.
- **`page`, the catch-all, carrying the lifecycle axis.** This is the profile's load-bearing declaration. Thirty-one notes say nothing at all about their type, so the type they bind to comes from the capability declaration rather than from the note — and everything a frontmatter-less note then participates in, the lifecycle filter above all, it participates in because `page` declares `state`. A version-2 catch-all may require nothing, and this one requires nothing.
- **Lifecycle `state`, ordinary = absent.** An unmarked page is current; `draft`, `superseded` and `archived` each mark a departure. One note is `superseded`, so the non-ordinary half of the filter has something to answer.

**Held at contract version 2, deliberately.** From M5 the supported range reaches version 3, and this corpus stays where it is: together with the other below-ceiling profile it is the standing witness that the floor is real — a version-2 vault keeps loading, keeps validating, and gains `capture` through version 3's default table — and that version 3's write seats configure the verb rather than enable it. The cost is one `info` per run, `compat.newer-format-available`, which the conforming-contract scenario admits by name and by severity and nothing else. See [the M5 fixtures record](../../../docs/decisions/engineering/2026-08-07-m5-fixtures-and-conformance.md); moving this corpus to the current version is that record's decision to revisit, not an edit.

## The four situations the corpus exists to express

1. **Repeated-basename search hits distinguished by path.** Five notes are named `README.md`, four `overview.md`, two `limits.md`; a hit list over any of those names is only readable by path.
2. **A bare recurring name ambiguous while its path-qualified form resolves.** `README` names five notes and so names none of them; `reference/README.md` names exactly one. The corpus never *writes* an ambiguous bare name — its own style page tells authors to path-qualify a repeated one — so the committed corpus stays clean while the situation remains one derivation away, which is where `ambiguous-bare-name-yields-link-diagnostic` reaches it.
3. **Path-qualified relative links resolving under the dialect.** 184 of the 225 internal references are relative paths from the vault root, with and without the `.md` extension and with and without a `#fragment`, and every one resolves.
4. **A frontmatter-less note binding through the declared default and participating in filters.** Thirty-one of them, all bound by catch-all, all answered by `list --ordinary`.
