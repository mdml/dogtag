# dogtag — TypeScript binding (reserved scaffold)

This directory reserves the home of the dogtag TypeScript binding. **Nothing here is installable yet**, and nothing will be until milestone M6 (see [Beta](../../docs/beta.md)).

## Boundary

- **This package holds no vault semantics.** All behavior — the note / type / property / relationship kernel, contract validation, diagnostics — lives in the Rust core (`crates/dogtag`). TypeScript is a binding, never a parallel implementation.
- At M6 this package will **wrap the same Rust core** as a native binding (napi-style; the exact mechanism is an M6 decision), exposing an idiomatic TypeScript API over the SDK's public surface.
- The binding will **share the conformance fixtures, diagnostic identifiers, and compatibility rules** with the Rust SDK — one implementation, one behavior, verified by the same scenarios.

`"private": true` in `package.json` prevents accidental publication before M6. The `dogtag` name was verified unclaimed on npm (2026-07-30); no placeholder is published.

## Pointers

- [Architecture](../../docs/architecture.md) — "one implementation; idiomatic bindings".
- [Beta](../../docs/beta.md) — milestone plan, including M6.
