# Product Decision Records (PDRs)

Product decisions recorded the way [ADRs](../engineering/README.md) record technical ones — but for *what the product is*, not *how it's built*. This directory holds one file per decision; the [six-document set](../../../README.md) carries the product thesis the decisions hang off.

A PDR is the product analog of an ADR, after Luca Rossi's pattern: 👁️ Intent, 🎨 Design, ⚖️ Tradeoffs — plus a 🖥️ **Surfaces** section, because Dogtag's product decisions are deliberately UI-independent and the interesting part is *how each surface satisfies the same intent*.

## What deserves a PDR

The novelty test: **anything genuinely new in what the product does**, that we'd want to remember when we hit a similar problem again. Not the reuse of a pattern we already understand well.

- ✅ "Faceted browse, not a query builder" — a real product stance with discarded alternatives.
- ✅ "Wikilinks are colored by target type and carry an overridable alias" — new interaction model.
- ❌ "Add a 'copy file link' right-click action" — no novelty, no interesting decision.

When in doubt, ask: *would a different person building a different UI for Dogtag need to know this to build the same product?* If yes, it's a PDR. If it's only true of one surface's implementation, it belongs in that surface's own decision record, not here.

## Voice — timeless, and for any PKM

**The test for every argument in a PDR: would it convince someone building basically *any* personal knowledge system, at basically *any* time?** If the reasoning only lands for one corpus, one tool, or one moment, it isn't a product decision yet — it's personal history wearing a PDR's clothes. A PDR is a durable record; write it to outlive the vault that prompted it.

Two properties this demands:

- **Timeless.** The rationale doesn't peg to a moment — no "recently," no "now that we've migrated," no current-state snapshots, no dated tooling in the argument. A reader years from now should find the case as sound as today. (Concrete *mechanics* date; the *reason* must not.)
- **Universal — "why is this the choice for basically everyone's PKM."** The argument is a first-principles claim about what a knowledge system needs, not about Dogtag's path. A reader who has never seen this repository should finish understanding *why this is the right design for a PKM*, full stop.

What that rules out of the Intent / Design / Tradeoffs:

- **No archaeology.** Migration counts, prior tools, corpus-specific numbers, "the path our vault took" — personal history, not product reasons. They may corroborate in a clearly-labelled aside ("receipts"), but the argument must stand without them.
- **Concrete examples are fine; keep them generic and timeless.** "e.g. a person/place/org class" illustrates; "we consolidated ours last spring" is archaeology.
- **Dogtag-specific or time-bound mechanics live in the config/invariant line or the Surfaces section** — as *this* implementation at *this* moment, never smuggled into the general argument.

## Template

```
# <Product decision title>

- **Status:** proposed | adopted | superseded by [<link>]
- **Decided / updated:** YYYY-MM-DD
- **Layer:** substrate | surface | both
- **Config vs. invariant:** <one line — what here is swappable per-vault vs. a hard product stance>

## 👁️ Intent

What the user needs to accomplish — the job-to-be-done, stated UI-independently and for any PKM user.

## 🎨 Design

What we decided the product does. The behavior, not the implementation.

## ⚖️ Tradeoffs

Alternatives discarded and why — argued as "why is this the choice for basically everyone's PKM," not
"why one particular vault went this way." (The part that survives longest — write it well, write it
timeless.)

## 🖥️ Surfaces

How each surface delivers the Intent, and what's UI-independent vs. per-surface.
- **TUI** — …
- **MCP** — …
- **CLI / SDK** — …
- **GUI editor (fallback)** — … (omit a surface if it genuinely doesn't apply)
```

## Conventions

- **Naming:** topic-slug, **no date prefix** — `wikilink-ui.md`, `search-and-filter.md`. A PDR is a *living product position* that evolves in place, not a dated event; the `Decided / updated` line carries the dates. (This is a deliberate divergence from ADR `YYYY-MM-DD-slug.md` naming, which marks point-in-time decisions.) Stable slugs keep links durable as the content matures.
- **Config vs. invariant is mandatory** in the header. It's the seam that lets one product serve meaningfully different vaults: mark which parts of *this* decision are per-vault config-points and which are hard product stances. Turning an invariant into a config-point is a deliberate, visible act, and the header is where it's visible.
- **Honest rationale, same rule as ADRs.** The Design/Tradeoffs must reflect what was *actually reasoned* — not a plausible "why" reverse-engineered from the code or a screenshot. If the why isn't articulated yet, ask, or write generically and mark it open. A confabulated product rationale is durable false history.
- **Interview before writing.** Draft a PDR from the product owner's stated reasoning, gathered by asking — the need, why this design is right for any PKM, what the alternatives lose. Implementation history corroborates as receipts; it is not the source of the "why," and writing from it alone produces personal-history rationale (see Voice above).
- **Cite, don't duplicate.** Link the technical decision record for mechanics, the vault contract for vocabulary. A PDR is the product decision; it points at the receipts.
- **Lifecycle:** land as `proposed` while the decision is still being talked through; flip to `adopted` once it's settled (often: already shipped in some surface). Superseded PDRs are **never deleted** — mark `Status: superseded by <link>` and leave the file, same as ADRs.

## Shared definitions — one home, everyone links

PDRs overlap by design — they form a **graph, not a tree**. `search-and-filter` leans on the index; `wikilink-ui` leans on the type system; both lean on the schema vocabulary. The discipline that keeps the graph from rotting: **every shared concept has exactly one canonical home, and every PDR that needs it links there instead of restating it.**

- **Substrate *facts*** — the indexed column set, the predicate vocabulary, the type list, ranking weights — live in their canonical home: each vault's committed contract, validated by the SDK's contract types (`crates/dogtag`, forthcoming — see [Architecture](../../architecture.md)). PDRs **reference** them. Never snapshot source-of-truth state in prose: don't enumerate a column set or a vocabulary inside a PDR — it drifts. State the rule, point at the source. (The [`search-and-filter`](search-and-filter.md) exemplar was fixed for exactly this.)
- **Product *concepts*** that don't have a home yet — the substrate/surfaces model, the config/invariant framing, "the index" as a product idea — live in the doc set ([Product](../../product.md), [Abstractions](../../abstractions.md)) or in a *foundational* PDR, and other PDRs reference that.
- **Foundational vs. surface PDRs.** A few PDRs *define* shared substrate (`note-types`, `relationships`, `markdown-flavor`); the rest *consume* them (`search-and-filter`, `wikilink-ui`, `inbox-workflow`, …). Write the foundational ones first so the surface ones have something to point at.
- **Config or invariant for a shared thing?** Usually *layered*, not one label — resolve it at the right layer. The worked example is "the indexed columns" in [`search-and-filter`](search-and-filter.md): the *existence* of the index is an invariant; the *column set + weights* are config living in the vault contract.
