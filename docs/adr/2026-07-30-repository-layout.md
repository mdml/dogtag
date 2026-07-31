# Repository layout

- Status: accepted (amended 2026-07-30 — see [Amendments](#amendments))
- Date: 2026-07-30

## Context

This repository starts life by extraction: the product documentation set (six documents) and ten product decision records were drafted before any code existed, under a readme-driven development approach, and the extraction contract says they move here as the repository's front matter. The repository must simultaneously hold a Rust workspace, a conformance suite whose structure is itself load-bearing (see the [conformance harness ADR](2026-07-30-conformance-harness-shape.md)), a reserved scaffold for a future TypeScript binding, and two distinct decision trails — product stances that predate the repo, and build decisions that start with it.

## Decision

- **The six product documents live at the repository root**: `README.md`, `PRODUCT.md`, `ABSTRACTIONS.md`, `ARCHITECTURE.md`, `BETA.md`, `STRATEGY.md`. The README is the repo front page; the other five are its immediate siblings, cross-linked with plain relative links. At M1 this root set *is* the documentation source; standing up dogtag.dev to front it (site plus hosted install script) is deliberately deferred to a later milestone.
- **Two decision trails under `docs/`**: `docs/decisions/` holds the product decision records (PDRs — timeless product stances, written for any PKM), `docs/adr/` holds this repository's own build decisions. Each has a README stating its conventions.
- **Code by role**: `crates/dogtag` (the SDK — the product kernel) and `crates/dogtag-cli` (a consumer of the SDK's public API) under `crates/`; the conformance harness, scenarios, and fixture-profile specs under `conformance/`; the reserved TypeScript scaffold under `bindings/typescript/`; shared shell logic under `scripts/`; workflows under `.github/workflows/`.
- **Operational files at the root**: `LICENSE`, `AGENTS.md` (canonical repo instructions, with `CLAUDE.md` as a pointer), `justfile`, `install.sh`.

### Alternatives considered

- **A `docs/` subtree for the six documents** (e.g. `docs/product/`). Rejected: the doc set is not auxiliary documentation, it *is* the product spec that precedes the code — and `README.md` has to sit at the root regardless, so nesting the others would split the set and break its internal reading order for the most common entry path (landing on the repo page).
- **A separate documentation repository.** Rejected: it divorces the contract from the implementation it governs, invites drift, and doubles the maintenance surface for a solo-maintained beta. One repo, one clone, one place to read.
- **Flattening the decision trails into one directory.** Rejected: PDRs and ADRs have different audiences, voices, and lifecycles (product stances are written timelessly for any PKM; build decisions are dated repo mechanics). Mixing them would blur exactly the boundary the two READMEs exist to keep sharp.

## Consequences

- The repository root is document-heavy for a code repo. That is deliberate — the docs are the product's front matter — but it means new top-level files need a reason to exist there.
- Future surfaces have obvious homes: a new crate goes under `crates/`, a new binding under `bindings/`, and neither disturbs the root.
- Because the six docs cross-link relatively at the root, moving any of them later is a breaking change to the doc set's link graph and would need its own ADR.

## Amendments

- **2026-07-30 — the product documents moved into `docs/`, and the two decision trails were regrouped.** The first Decision bullet no longer holds: `README.md` remains the repository front page, but `PRODUCT.md`, `ABSTRACTIONS.md`, `ARCHITECTURE.md`, `BETA.md`, and `STRATEGY.md` became `docs/product.md`, `docs/abstractions.md`, `docs/architecture.md`, `docs/beta.md`, and `docs/strategy.md`. The second bullet's two trails survive as a pair but moved and were renamed: `docs/decisions/` and `docs/adr/` became `docs/decisions/product/` and `docs/decisions/engineering/`. The fourth bullet gained `security-exceptions.toml`, which moved to the root from `docs/security/exceptions.toml` as configuration rather than documentation. Code by role is unchanged. The record that changed this — and that supplies the ADR the Consequences section above required before any of the documents could move — is [documentation architecture and roadmap ownership](2026-07-30-documentation-architecture.md), which also answers this record's rejection of a `docs/` subtree on the grounds that the rejection protected a reading order rather than a path.
