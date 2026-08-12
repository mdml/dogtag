# The M6 TypeScript binding and npm package

- Status: accepted
- Date: 2026-08-12

## Context

M6 delivers the first foreign-language SDK binding. The binding must operate a vault through the Rust core without invoking the CLI, hold no vault semantics of its own, and be installable by the TypeScript workflow that proves the SDK architecture. The reserved scaffold names a native napi-style direction but deliberately left the mechanism and publication decision to this packet.

Portability matters, so WebAssembly/WASI was considered seriously rather than treated as a synonym for “not native.” The current core reads and writes the host filesystem and implements commit-at-birth by spawning `git`. A WASI build can receive a preopened vault directory, but process spawning and the current filesystem and trust behavior do not follow automatically. Full parity would require a host interface for filesystem, Git, environment, permissions, and trust checks, or a deliberate split into platform-neutral semantics and substrate adapters. M6 has no evidence that either shape preserves the one Rust implementation more cheaply than a native binding.

## Decision

### Native binding, published at M6

The TypeScript SDK wraps the Rust core through **napi-rs**. The public `dogtag` npm package ships platform prebuilds through platform-specific optional packages; consumers install the root package and do not compile Rust or download a binary during installation.

The M6 native matrix is explicit: **macOS x64 and arm64; Linux glibc x64 and arm64.** Node.js 22 and the repository's pinned Bun release are blocking runtime tests on each claimed platform available to the release matrix. The hosted workflow's Linux x64 glibc/Bun pair is additionally exercised by deployment. Linux musl remains a CLI release target and is not an npm-binding claim at M6; neither are Windows, browsers, edge isolates, Deno, or other Node/Bun versions. A platform or runtime joins only with a real blocking runtime test, never because napi-rs accepts its name or Node-API suggests ABI compatibility.

M6 deliberately removes `"private": true` and publishes the binding at the same prerelease version as the Rust workspace and CLI, nominally `0.1.0-beta.5`. Package-name availability is rechecked immediately before the first publish because the scaffold's 2026-07-30 observation is evidence about that date, not a reservation.

The binding exposes an idiomatic TypeScript API over the public SDK operations M6 consumes: opening a verified vault, search, show, capture planning and application, tag enumeration, and agent-contract rendering and generation, with the shared diagnostic and result types. It does not mechanically export every Rust item, and it does not reduce the SDK to MCP handlers. TypeScript adapts ownership, errors, and value representation; it never parses a contract, traverses notes, interprets diagnostics, or reconstructs a result.

### Conformance crosses the language boundary

Every applicable language-neutral conformance scenario runs through both the Rust adapter and a TypeScript adapter against the same fixture transformations. There are no binding-specific scenario waivers. Harness mechanics that test Rust implementation details remain Rust tests rather than being mislabeled language-neutral scenarios. Equivalence covers semantic fields, diagnostic identifiers and severities, compatibility classification, and mutation results; it does not require TypeScript values to serialize as the CLI's JSON document.

### WASI is a named follow-up, not a fallback claim

The public API boundary must not preclude a later WASI substrate, but M6 publishes no WASI artifact and makes no browser, edge, Bun-WASI, or runtime-neutral claim. The next binding-mechanism packet may reopen WASI when it can name how filesystem authority, commit-at-birth, trust checks, and runtime testing remain equivalent. A partial read-only fallback hidden behind the same package name is rejected: `capture` silently losing commit behavior would be portability by subtraction.

### Alternatives considered

- **WASM/WASI as the M6 binding.** Rejected for this milestone: the artifact is portable, but the kernel's host behavior is not automatically portable; M6 would acquire a second substrate adapter and its equivalence proof before the first binding has shipped.
- **Native primary with an automatic WASI fallback.** Rejected: unsupported platforms would receive a behaviorally narrower SDK under the same package contract.
- **Neon.** Rejected: viable, but it provides no advantage over napi-rs for the selected prebuild/package model and requires more handwritten bridge surface.
- **A CLI subprocess.** Rejected by the beta contract and the SDK boundary: it proves command orchestration, not embedding.
- **Keeping the package private.** Rejected: an in-tree-only binding does not prove the installable TypeScript workflow the beta promises.
- **Building native code during installation.** Rejected: it makes Rust a consumer prerequisite and weakens release verification.
- **Claiming the CLI's musl matrix for npm.** Rejected: native-addon loading is a separate support claim; M6 has no binding runtime evidence on musl.

## Consequences

- Native publication is a multi-package release and therefore not atomic; the release workflow must publish platform packages before the root package and must not reuse a burned version after a partial publish.
- The support claim is narrower than Rust's target vocabulary and is defined by blocking runtime tests, not by what napi-rs can parse.
- A future WASI binding may require an SDK-internal substrate seam. That work is not smuggled into M6, but the TypeScript API is kept independent of Node-specific path or handle types so the question remains open.
