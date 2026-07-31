# TypeScript scaffold boundary

- Status: accepted (amended 2026-07-30 — see [Amendments](#amendments))
- Date: 2026-07-30

## Context

The architecture pillar is one implementation with idiomatic bindings: the TypeScript SDK, due at M6, wraps the Rust core rather than reimplementing vault behavior (see [architecture.md](../../architecture.md) and [beta.md](../../beta.md)). A binding that starts as "just a little TypeScript logic while the core catches up" becomes a parallel implementation nobody decided to build. The repository should state the boundary from day one — before there is any TypeScript to be tempted by.

## Decision

- **`bindings/typescript/` exists at M1 as a reserved scaffold**: a `package.json` and a `README.md`, nothing else. No source files, no dependencies, no build scripts that pretend to do anything.
- **`package.json` declares** name `dogtag`, version `0.1.0-beta.0` (the workspace version), license Apache-2.0, and `"private": true` — npm refuses to publish a private package, so accidental publication before M6 is mechanically impossible, not merely discouraged.
- **The README states the boundary**: this package holds no vault semantics; it will wrap the same Rust core (napi-style native binding — the exact mechanism is an M6 decision), sharing fixtures, diagnostic identifiers, and compatibility rules with it; until M6 nothing here is installable.
- **No npm name-reservation publish.** The `dogtag` name was verified unclaimed on npm on 2026-07-30 and is left unclaimed; no placeholder package is published to hold it.

### Alternatives considered

- **Publishing a `0.0.1` placeholder to npm to reserve the name.** Rejected: npm policy disfavors squatting and reserves the right to reclaim placeholder packages, an empty package is a worse first impression than an unclaimed name, and a placeholder invites `npm install dogtag` to succeed while delivering nothing — the exact confusion `"private": true` exists to prevent.
- **Omitting the directory entirely until M6.** Rejected: the scaffold is the architectural statement. Its README is where "bindings hold no semantics" lives in the tree, visible to anyone who goes looking for the TypeScript story before M6.
- **Scaffolding the napi-rs toolchain now.** Rejected: it would decide the binding mechanism months before M6, the milestone that owns that decision, and add build complexity with no behavior behind it.

## Consequences

- The npm name remains claimable by others until M6 publishes. Accepted risk, same stance as crates.io (see the [workspace and naming ADR](2026-07-30-workspace-and-naming.md)).
- `"private": true` must be removed as a deliberate act at M6 — publication cannot happen by accident, and the flag's removal is a natural anchor for the M6 publish checklist.
- The scaffold's version rides along with workspace version bumps, one more file to touch — a small price for one version number everywhere.

## Amendments

The Decision above stands as written; this later record changes part of it, and the original text is left intact so the change is legible.

- **2026-07-30 — the scaffold holds a third file, `bunfig.toml`.** The Decision says "a `package.json` and a `README.md`, nothing else". It now also carries a Bun configuration setting a seven-day package quarantine (`minimumReleaseAge`), because that file has to exist *before* the first `bun install` to have ever applied — configuring it at M6, alongside the first dependency, would leave exactly the first resolution unprotected. It adds no source, no dependency, and no build script, so the boundary the Decision is actually about is unchanged: the scaffold still holds no semantics and is still not installable. Recorded in the [supply chain and vulnerability policy](2026-07-30-supply-chain-and-vulnerability-policy.md).
