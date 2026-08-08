# Documentation architecture and roadmap ownership

- Status: accepted
- Date: 2026-07-30

## Context

[Repository layout](2026-07-30-repository-layout.md) put the six product documents at the repository root and explicitly rejected nesting them, on the reasoning that the set is the product spec rather than auxiliary documentation and that `README.md` has to sit at the root regardless. That reasoning was sound for six documents. It stops being sound at the volume M2 adds — an implementation and contract-format record set — because the root would become the place where every document that has nowhere better to go accumulates, and a path would carry no information at all.

There is a second reason to fix this now rather than later. Dogtag is built to read and reason over Markdown corpora. Its own repository is the first corpus anyone will point it at, and a corpus whose paths carry no semantic role is a poor demonstration of the product's premise.

The corpus has **two orthogonal axes**, and choosing which one to cut on first is the whole decision:

- **Form** — a narrative spec document, a decision record, a temporal plan, an operating procedure. These differ in how they are written, how they age, and how they are read.
- **Domain** — about Dogtag, or about this repository. This is the axis the two decision-trail READMEs already maintain, with a written routing test: *would it still matter to someone reimplementing dogtag from the docs alone?*

Cutting on domain first forces the five spec documents to pick a domain side, and `ARCHITECTURE.md` is where that forcing hurts: it is unambiguously technical, unambiguously about Dogtag rather than about this repository, and cited by seven of the ten product decision records, and by their README, as the canonical home for substrate facts. Any tree that requires adjudicating it has picked the wrong first cut.

## Decision

**Form is the first cut; domain is the second, applied only where it is load-bearing.**

```
docs/
  README.md                    # the corpus index and the spec set's reading order
  product.md                   # the spec set — narrative, durable, about Dogtag
  abstractions.md
  architecture.md
  beta.md
  strategy.md
  roadmap.md                   # temporal: what is active now, what has shipped
  decisions/
    README.md                  # the one home of the PDR-vs-ADR routing rule
    product/                   # product decision records — timeless, for any PKM
    engineering/               # this repository's build decisions
```

- **The spec set is flat under `docs/`.** `architecture.md` sits with `abstractions.md` and `beta.md` because it is the same kind of document, which is what makes the domain question stop being a question. The set stays flat until the narrative spec documents — `product`, `abstractions`, `architecture`, `beta`, `strategy` — pass eight; `README.md` and `roadmap.md` are not spec documents and do not count. Structure does not get another level before volume justifies it.
- **The two decision trails are grouped and named for their domain.** `docs/decisions/product/` and `docs/decisions/engineering/` replace `docs/decisions/` and `docs/adr/`, which named the same kind of artifact two different ways. Filenames and the two naming conventions are unchanged: engineering records keep `YYYY-MM-DD-slug.md` because they are dated events, product records keep stable topic slugs because they are living positions. `docs/decisions/README.md` becomes the single home of the routing test; each trail's README keeps its own framing of what it holds but links to that file rather than restating the test, and [AGENTS.md](../../../AGENTS.md) keeps the test inline — a contributor should not have to click to learn which record to write — naming the canonical home.
- **Temporal material is named apart from durable material, and is licensed to go stale.** `docs/roadmap.md` is the only document in the tree expected to be false in a month. It names the active milestone and its acceptance criteria and records completion receipts; it does not restate the milestone ladder, which `beta.md` owns. Note what this bullet does *not* claim: `roadmap.md` is a plain sibling of `beta.md`, so the durability difference is carried by the filename and by the document's own header, not by the path. It gains a directory — keeping `roadmap.md` inside it, because `now.md` would fail the naming rule below — only when the roadmap needs more than one page, which happens when work begins that the beta milestone ladder does not cover.
- **A filename must be self-describing without its directory.** Dogtag surfaces bare names in indexes and search results, where `overview.md` and `case.md` are unreadable and `product.md` is not. This rule is why the `docs/product.md` stutter that the rejected `docs/product/product.md` would have carried is not worth renaming away from, and why the registry below is `security-exceptions.toml` rather than `exceptions.toml`. `README.md` is the standing exemption: the name is a platform contract that makes a file its directory's rendered index, which is why seven of them coexist here. It is also the reason an index over this corpus has to key on title rather than filename.
- **Names are lowercase inside `docs/`, uppercase at the root.** GitHub's special-casing of uppercase names applies at the root and `.github/` only, so uppercase inside `docs/` buys nothing and would clash with the lowercase names both decision trails already used. `README.md`, `AGENTS.md`, `CLAUDE.md`, and `LICENSE` keep uppercase at the root, where the convention is load-bearing. `conformance/profiles/*/PROFILE.md` keeps uppercase: it is a sidecar paired with `PROFILE.toml`, a different convention.
- **The security-exception registry is configuration, not documentation.** `docs/security/exceptions.toml` moves to `security-exceptions.toml` at the root, beside `deny.toml` — the tool config whose ignores it governs and against which a script cross-checks it in both directions — and beside `coverage-baseline.toml`, which is the same kind of machine-read policy data with a commented schema. This is the layout ADR's "operational files at the root" clause, not an exception to it.
- **No redirect stubs at the old paths.** A stub is a second path for one concept, which is the dual-canonical-home failure the [extraction ADR](2026-07-30-document-extraction-sanitization.md) rejected, and it is a document Dogtag would index as real. Inbound links to the old paths still resolve at the `v0.1.0-beta.0` tag, where the tree is unchanged.

### Roadmap ownership

**This repository is canonical for milestone status, acceptance criteria, and completion receipts.** The maintainers' private planning record keeps personal context, participant identities, private strategy, unpublished evidence, and infrastructure detail, and points here for status.

The migration is deliberately minimal because `beta.md` already carries the structural half of the roadmap — the milestone ladder, the per-milestone cutover rule, the ship test, and the fixture schedule, all public since M1. What was missing publicly is only which rung is active and what the finished rungs produced, which is one document. The private scope record is not imported: its forward-looking half would duplicate `beta.md`, and its remaining substance is either already reconciled into this trail or is material that stays private.

### Alternatives considered

- **Leaving the six documents at the root.** Rejected, and this record amends the ADR that decided it. The original argument protects a *reading order*, not a *path* — and the reading order survives the move intact, carried by `README.md` and `AGENTS.md`, both of which stay at the root and both of which a reader reaches first. What root placement actually buys is one fewer click for a reader who was already told where to click.
- **Cutting on domain first: `docs/product/` and `docs/engineering/`.** Rejected after being the starting proposal. It forces `architecture.md` to be adjudicated, and it leaves `docs/engineering/` containing exactly one child directory — single-child nesting one level up, which is the tell that the axis is wrong. It also pushes the product decision records a level further from the spec documents they cite, for no gain.
- **Keeping `docs/decisions/` and `docs/adr/` as siblings and moving only the spec set.** Rejected: it is the cheaper migration and it leaves the asymmetry in place, where two trails holding the same kind of artifact are named for a form and an acronym respectively. The repository is one day old; this is the cheapest this change will ever be.
- **Promoting the spec set into `docs/spec/` now.** Rejected: five files do not need a level, and "spec" is the wrong word for a document set that is mostly a product case and an experiment sequence. The growth trigger is recorded above instead.
- **Importing the private scope record as a public scope document.** Rejected: it publishes process history for no reader benefit, duplicates a ladder `beta.md` already owns, and drags material through a sanitization pass it does not need to survive.
- **Creating `docs/guides/` and `docs/retrospectives/` now.** Rejected: no document wants to move into either. The operating knowledge is not homeless — the command ladder, gate contract, commit rules, and release procedure are in `AGENTS.md`, where a contributor must read them anyway, and the reasoning behind each is in this trail. Writing guides now would mean net-new prose that duplicates `AGENTS.md` or hollows it out. Triggers are recorded in Consequences.

## Consequences

- **Every relative link into the moved documents changes, once.** Of the 222 relative link destinations the repository carried, 108 are rewritten and 114 are untouched. The untouched majority is why this is cheaper than it looks: sibling links inside both trails keep their bare names. Among the rewritten, the trails' links up to the spec set keep their exact prefix and change only in case. [scripts/check-links.sh](../../../scripts/check-links.sh) verifies that destinations exist, but not that a link's display text or a sentence of prose naming a path was updated, so the migration also required a hand sweep — for stale display text, and across non-Markdown files for a Python constant that reads a record by path, a registry path in a checker, two Rust doc comments, one test assertion message, and comments in five configuration files.
- **The routing test between the two trails now has one canonical home plus one deliberate restatement, down from two co-equal statements.** `docs/decisions/README.md` holds it; `AGENTS.md` repeats the sentence because it is the contract a contributor reads before committing and should not send them elsewhere for it. The doc set's reading order is duplicated the same way and for the same reason, across `README.md` (a stranger must see it without a hop), `AGENTS.md`, and `docs/README.md` (canonical). Both duplications are accepted, not overlooked: they are short, they are checked against each other, and the alternative costs the reader more than the drift risk costs the corpus.
- **`docs/roadmap.md` is the first document here that is designed to go stale.** Nothing mechanical keeps it current; if it drifts, it is worse than absent, because it claims to be canonical. The mitigation is that it is short and that the milestone it names is the one being worked on.
- **Milestone status has one canonical home only once the maintainers' planning record defers to it.** Until then, two records could disagree, and this record cannot make that edit.
- **`README.md` ships inside the release archive and its documentation links point outside it.** That was true before this change and remains true; the links were repository-relative to the root before and are repository-relative to `docs/` now, and neither resolves from an extracted tarball. Making them absolute URLs would fix the tarball and trade a checked link for an unchecked one, so it was considered and not done.
- **Growth triggers, recorded so the next level is not added by reflex.** `docs/guides/` when a procedure is a repeatable *how* rather than a *why*, needs materially more than a paragraph, and documents something deployed rather than planned — split by domain only once both kinds exist. The class most likely to trigger it first is the admin runbook: applying the repository rulesets, rotating the CodeScene credential, publishing a draft release. `AGENTS.md` is written for contributors, and those procedures have a different audience. `docs/retrospectives/` at the first real retrospective, naturally the beta verdict — the last rung, M11 since [the daily-driver record](2026-08-08-the-beta-is-the-daily-driver.md) lengthened the ladder, and written as M8 here when the ladder had nine rungs. `docs/spec/` when the spec set passes roughly eight documents. `docs/roadmap/` with `scopes/` when a second scope opens.
- **The repository root is meaningfully smaller.** Five documents left it and one configuration file joined it, which is the trade this record makes: the root now holds the entry points a stranger or a tool expects there, and nothing that was only there for want of a better home.
