# Documentation architecture and roadmap ownership

- Status: accepted
- Date: 2026-07-30

## Context

[Repository layout](2026-07-30-repository-layout.md) put the six product documents at the repository root and explicitly rejected nesting them, on the reasoning that the set is the product spec rather than auxiliary documentation and that `README.md` has to sit at the root regardless. That reasoning was sound for six documents. It stops being sound at the volume M2 adds — an implementation and contract-format record set — because the root would become the place where every document that has nowhere better to go accumulates, and a path would carry no information at all.

There is a second reason to fix this now rather than later. Dogtag is built to read and reason over Markdown corpora. Its own repository is the first corpus anyone will point it at, and a corpus whose paths carry no semantic role is a poor demonstration of the product's premise.

The corpus has **two orthogonal axes**, and choosing which one to cut on first is the whole decision:

- **Form** — a narrative spec document, a decision record, a temporal plan, an operating procedure. These differ in how they are written, how they age, and how they are read.
- **Domain** — about Dogtag, or about this repository. This is the axis the two decision-trail READMEs already maintain, with a written routing test: *would it still matter to someone reimplementing dogtag from the docs alone?*

Cutting on domain first forces the five spec documents to pick a domain side, and `ARCHITECTURE.md` is where that forcing hurts: it is unambiguously technical, unambiguously about Dogtag rather than about this repository, and cited by nine of the ten product decision records as the canonical home for substrate facts. Any tree that requires adjudicating it has picked the wrong first cut.

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

- **The spec set is flat under `docs/`.** `architecture.md` sits with `abstractions.md` and `beta.md` because it is the same kind of document, which is what makes the domain question stop being a question. The set stays flat until it passes roughly eight documents; structure does not get another level before volume justifies it.
- **The two decision trails are grouped and named for their domain.** `docs/decisions/product/` and `docs/decisions/engineering/` replace `docs/decisions/` and `docs/adr/`, which named the same kind of artifact two different ways. Filenames and the two naming conventions are unchanged: engineering records keep `YYYY-MM-DD-slug.md` because they are dated events, product records keep stable topic slugs because they are living positions. `docs/decisions/README.md` becomes the single home of the routing rule; each trail's README links to it instead of restating it, and [AGENTS.md](../../../AGENTS.md) keeps a short routing note that points there.
- **Temporal material is separated from durable material by path.** `docs/roadmap.md` is the only document in the tree that is expected to be false in a month. It names the active milestone and its acceptance criteria and records completion receipts; it does not restate the milestone ladder, which `beta.md` owns. It becomes `docs/roadmap/now.md` plus `docs/roadmap/scopes/` when a second scope opens.
- **A filename must be self-describing without its directory.** Dogtag surfaces bare names in indexes and search results, where `overview.md` and `case.md` are unreadable and `product.md` is not. This rule is why the `docs/product.md` stutter that the rejected `docs/product/product.md` would have carried is not worth renaming away from, and why the registry below is `security-exceptions.toml` rather than `exceptions.toml`.
- **Names are lowercase inside `docs/`, uppercase at the root.** GitHub's special-casing of uppercase names applies at the root and `.github/` only, so uppercase inside `docs/` buys nothing and would clash with the twenty-three files already there. `README.md`, `AGENTS.md`, `CLAUDE.md`, and `LICENSE` keep uppercase at the root, where the convention is load-bearing. `conformance/profiles/*/PROFILE.md` keeps uppercase: it is a sidecar paired with `PROFILE.toml`, a different convention.
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

- **Every relative link into the moved documents changes, once.** Roughly a hundred and forty of the repository's link destinations are rewritten; about eighty-five are untouched, because sibling links inside both trails keep their bare names and the trails' links up to the spec set keep their exact prefix. [scripts/check-links.sh](../../../scripts/check-links.sh) verifies that destinations exist but not that prose naming a path was updated, so the migration also required a sweep across non-Markdown files — a Python constant that reads a record by path, a registry path in a checker, and comments in six tool configs.
- **The routing rule between the two trails now has one home instead of three.** That is the point, and the cost is one more file whose only job is to hold a rule.
- **`docs/roadmap.md` is the first document here that is designed to go stale.** Nothing mechanical keeps it current; if it drifts, it is worse than absent, because it claims to be canonical. The mitigation is that it is short and that the milestone it names is the one being worked on.
- **The private planning record must be handed off in a matching change.** Until it is, two records claim authority over milestone status. This ADR cannot make that edit, and a proposed diff accompanies the migration for separate review.
- **`README.md` ships inside the release archive and its documentation links point outside it.** That was true before this change and remains true; the links were repository-relative to the root before and are repository-relative to `docs/` now, and neither resolves from an extracted tarball. Making them absolute URLs would fix the tarball and trade a checked link for an unchecked one, so it was considered and not done.
- **Growth triggers, recorded so the next level is not added by reflex.** `docs/guides/` when a procedure is a repeatable *how* rather than a *why*, needs materially more than a paragraph, and has a live integration behind it — split by domain only once both kinds exist. `docs/retrospectives/` at the first real retrospective, naturally the M8 beta verdict. `docs/spec/` when the spec set passes roughly eight documents. `docs/roadmap/` with `scopes/` when a second scope opens.
- **The repository root is meaningfully smaller.** Five documents left it and one configuration file joined it, which is the trade this record makes: the root now holds the entry points a stranger or a tool expects there, and nothing that was only there for want of a better home.
