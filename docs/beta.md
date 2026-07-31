# Dogtag SDK beta

> **Status: scope contract, 2026-07-29; M0 packet folded in 2026-07-30.** This document defines the first externally installable experiment. Milestone status and completion receipts are in [roadmap.md](roadmap.md); the maintainers' planning record keeps the private context behind them. The product repository's ADR trail is `docs/decisions/engineering/`.

## Promise

Two people with independently configured vaults can install the same released Dogtag SDK, operate both vaults through the CLI, embed the same semantics in a TypeScript workflow, and upgrade to a fix without manual vault repair.

The beta proves the complete product lifecycle with a deliberately small operational kernel:

1. install a released artifact without a product source checkout;
2. discover, configure, inspect, and validate a vault;
3. read it through stable SDK operations and their CLI binding;
4. make one planned, scoped, validated write;
5. embed the same operation model in a non-CLI workflow;
6. upgrade and preserve compatibility.

## In scope

- A clean Dogtag product repository with no personal vault history.
- A public Rust core SDK, licensed Apache-2.0.
- A CLI implemented entirely through the public SDK.
- A TypeScript binding backed by the same core.
- Vault discovery and layered, validated configuration: one committed vault contract, one local installation record.
- Structured diagnostics and machine-readable results.
- `version`, `doctor`, `check`, `list`, `show`, lexical `search`, and `contract explain` — the last rendering the resolved contract as agent-readable instructions so they cannot drift from it.
- One plan/validate/apply mutation, initially `capture`.
- Sanitized golden fixtures representing two different vault configurations, with two further profiles specified and scheduled.
- Versioned macOS and Linux prerelease artifacts, checksums, an installer, and upgrade inspection.
- A minimal documentation site at `dogtag.dev`.
- One real dynamic workflow built through the TypeScript SDK.

## Deferred

- Python bindings, until the first foreign-language binding and conformance pattern settle.
- TUI and MCP as product surfaces; both remain intended SDK consumers, and the beta's embedded workflow being an MCP server does not promote MCP to a product surface.
- Skills as committed vault markdown, and any starter skill pack; the generated agent contract ships without them.
- Saved views, named workflows, integration bindings, and secret references as configuration assets.
- Semantic search and richer graph queries.
- General import, schema interviews, and migration from every Markdown system.
- Working-tree dialect materialization.
- Self-mutating upgrades.
- Frontmatter-aware merge and multi-writer conflict UX.
- A hosted vault service, accounts, telemetry, or a polished marketing site.

## Required properties

- CLI behavior comes from the public SDK; the CLI contains no independent vault semantics.
- Rust and TypeScript behavior share fixtures, diagnostic identifiers, and compatibility rules.
- The core interprets only notes, types, properties, and relationships. Lifecycle, flags, and write policy reach it as declarations, and it enforces the declared shape without knowing any corpus's vocabulary.
- Behavior binds to declared type capabilities, never to type names. A contract that declares no catch-all, or more than one, fails to load.
- A corpus with repeated note names opens and reads. Ambiguity is reported against the unresolvable reference, never as a corpus-level error.
- Every mutation identifies its intended files, supplies actor/provenance, previews its effect, validates the result, and preserves a recovery path.
- Dynamic workflows decide when and why to compose operations; the SDK deterministically enforces identity, scope, ownership, validation, and compatibility.
- Every conformance scenario runs against every fixture profile. A scenario expressible against only one profile fails the harness and is triaged as either an incomplete configuration model or a personal convention mistaken for an invariant. There are no waivers.
- Both founder vaults use installed release artifacts for the beta surface. Running from the product checkout is development, not dogfooding.
- Each milestone names one real workflow that moves from the incumbent personal tooling onto installed Dogtag and does not move back. Capability that nothing depends on produces no evidence.

## Ship test

The beta ships when:

- both vaults pass the shared read and validation scenarios, and no scenario is profile-scoped;
- both users install the same prerelease independently;
- both complete a real CLI read workflow and a real CLI write workflow;
- one real TypeScript workflow operates a vault without shelling out to the CLI;
- an older beta upgrades to the final beta candidate on both installations;
- every workflow moved onto Dogtag by the per-milestone cutover rule is still there;
- known limitations and recovery instructions are published at `dogtag.dev`;
- neither vault has to adopt the other's schema or personal conventions.

## Milestones

The beta ships in nine milestones, each an installable vertical slice published as a prerelease. Which one is active, and what the finished ones produced, is [roadmap.md](roadmap.md); this section defines what each one delivers. Every milestone from M2 on names one real workflow that moves from the incumbent personal tooling onto installed Dogtag and does not move back — the cutover rule in Required properties.

- **M0 — beta contract and extraction packet.** Shipped: the decisions this document carries.
- **M1 — clean repository and empty release.** The product repository, Rust workspace, CLI and TypeScript scaffolds, conformance harness, release automation, and the doc set; `dogtag version` published and installed as `0.1.0-beta.0`.
- **M2 — open and diagnose.** Vault-root discovery, configuration loading, capability validation, compatibility checks, structured diagnostics, `dogtag doctor`, and `dogtag contract explain`. The committed vault-contract format is decided here; the `dense` and `starter` fixture corpora land with it. Cutover: the daily vault health check.
- **M3 — read and validate.** The public document model plus `check`, `list`, and `show`, against the shared conformance scenarios written at M0. Cutover: reading and listing notes.
- **M4 — lexical retrieval.** Basic search and entity lookup over the common model; the `docs` fixture profile is built here, where a folder-organized markdown-link corpus stresses retrieval hardest. Cutover: search.
- **M5 — safe mutation.** Plan/validate/apply transaction with one `capture` operation: explicit file scope, actor/provenance, preview, post-write validation, structured result, recovery. Cutover: one capture path.
- **M6 — TypeScript SDK and dynamic workflow.** The binding backed by the Rust core, then an SDK-backed MCP server (`search`, `show`, `capture`) deployed alongside the maintainers' existing infrastructure.
- **M7 — second-vault onboarding.** Install the released artifact, configure the second founder vault, complete one read and one write workflow without a product source checkout.
- **M8 — upgrade and beta verdict.** Upgrade both installations from older prereleases, run the complete conformance and workflow test, publish limitations and recovery documentation, and decide whether E0 graduates to assisted adoption ([strategy.md](strategy.md)).

## Fixture profiles

Four profiles, each standing for one audience in [product.md](product.md)'s persona table — `starter` for the fresh install every persona begins at — and together spreading across every axis the configuration seam claims to absorb: type taxonomy, capability assignment, property requirements, predicate vocabulary, lifecycle encoding, name resolution, and dialect. Two are built for the first release that can load a vault contract; two are specified now and built when the hypothesis they serve comes up. All four ship as specifications from M1; each corpus lands at its named milestone, because the committed contract format the corpora depend on is an M2 decision (the reasoning is recorded in [the conformance harness ADR](decisions/engineering/2026-07-30-conformance-harness-shape.md)).

| Profile | Stands for | Distinguishing axes | Corpus lands |
| --- | --- | --- | --- |
| `dense` | the PKM enthusiast with an established corpus | many types, several identity-bearing, wikilink dialect, lifecycle where the ordinary state is absence | M2 |
| `starter` | a fresh install | the initialization profile's own defaults, lifecycle where the ordinary state is a named value | M2 |
| `docs` | the dev team | repository documentation shapes, folder organization, markdown-link dialect, most files carrying no frontmatter, repeated basenames | M4 |
| `records` | the decision maker | a dense domain record taxonomy, immutable originals under closed-write, evidence trails | before assisted adoption (E1) |

`dense` and `starter` differ on the sharpest axis available — whether a corpus spends a named value on its ordinary lifecycle state or leaves it absent. If the same scenario can filter by the life axis in both without either vocabulary reaching the core, the seam is real.
