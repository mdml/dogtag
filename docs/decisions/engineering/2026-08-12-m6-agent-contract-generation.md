# M6 generates the vault's `AGENTS.md`

- Status: accepted
- Date: 2026-08-12

## Context

The architecture requires the agent instructions a vault carries to be generated from the resolved contract so prose cannot drift from enforcement. M2 shipped the rendering as `contract explain`; M5 scheduled the on-disk write at M6 because the MCP server is its first consumer. This is a second write path and therefore needs its ownership, overwrite, transaction, and recovery behavior decided rather than borrowed silently from capture.

## Decision

The Rust SDK owns both rendering and a plan/apply operation that writes root `AGENTS.md`. The CLI exposes that operation as `dogtag contract generate-agents [--vault …] [--preview] [--format text|json]`; TypeScript binds the same public operation. The MCP server does not expose generation as a tool and does not write the file on startup. Deployment invokes the operation explicitly.

Generated output carries an unmistakable Dogtag ownership marker plus the contract version and generator version as provenance. Dogtag atomically replaces only a file carrying that marker. An existing unowned `AGENTS.md` is a refusal and is never overwritten. Re-running against unchanged inputs is idempotent and reports no content change.

The plan names exactly `AGENTS.md`, actor, provenance, compatibility impact, diagnostics known before writing, and the preview bytes. Apply writes atomically and validates that the resulting bytes equal the SDK's current rendering. Where Dogtag owns the commit path, it commits only `AGENTS.md` with the established `Dogtag-Actor` and `Dogtag-Provenance` trailers; guest mode leaves the file uncommitted and reports its path. A successful commit is recovery by revert; an uncommitted generation reports the prior owned bytes when replacement occurred, or the created path when it did not, so recovery is concrete without a general undo verb.

The founder vault commits the generated file and adds a gate asserting byte equality with current SDK rendering. Conformance asserts rendering and plan/apply behavior against every fixture profile, including refusal on an unowned collision and idempotent regeneration.

### Alternatives considered

- **Generate automatically at server startup.** Rejected: startup becomes a surprising mutation and authentication/deployment failures become entangled with an unrelated write.
- **Expose an MCP generation tool.** Rejected: agents need the contract as startup context, not a model-selected fourth mutation, and M6's bounded tool surface has no workflow for it.
- **Always overwrite.** Rejected: it destroys human-owned instructions and violates the writer's semantic-touch boundary.
- **Refuse every existing file.** Rejected: a generated artifact could never be refreshed, defeating non-drift.
- **Leave generation uncommitted because it is derived.** Rejected: the file travels with the vault and is the instruction artifact collaborators read; committed derivation plus a drift gate is the existing architecture.
- **Let TypeScript or deployment write rendered bytes.** Rejected: ownership, collision, atomicity, and recovery would leave the kernel and become per-consumer semantics.

## Consequences

- M6 has two mutation operations, capture and agent-contract generation, but only capture is remotely model-controlled.
- The generator establishes the first replace-in-place transaction. Its single-file atomic replace does not decide M8's general edit semantics or multi-file transaction questions.
- A vault with a human-owned root `AGENTS.md` must reconcile or relocate it explicitly before adopting generation.
