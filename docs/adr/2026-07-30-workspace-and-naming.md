# Workspace structure and naming

- Status: accepted
- Date: 2026-07-30

## Context

The architecture makes one boundary load-bearing: the SDK is the product and the CLI is only a consumer of its public API (see [ARCHITECTURE.md](../../ARCHITECTURE.md)). The repository layout has to make that boundary structural rather than aspirational, and the first release (`0.1.0-beta.0`, the empty vertical slice) has to fix the names users and toolchains will see from now on.

## Decision

- **Cargo virtual workspace** with `resolver = "3"` (explicit — virtual workspace roots do not infer it) and two members at M1: `crates/dogtag` and `crates/dogtag-cli`, plus the non-published `conformance/harness`.
- **Names**: the SDK library crate is `dogtag`; the CLI crate is `dogtag-cli` with its binary named `dogtag` — users type `dogtag`, the crate namespace keeps SDK and CLI distinct.
- **Workspace inheritance**: `version`, `edition`, `license`, `repository`, and `rust-version` live once in `[workspace.package]` and every crate inherits them; shared dependency versions live in `[workspace.dependencies]`. The version is `0.1.0-beta.0` everywhere, including the npm scaffold — one number, bumped in one place.
- **Toolchain pin**: `rust-toolchain.toml` pins stable `1.97.1` with `clippy` and `rustfmt` components; edition `2024`; MSRV `rust-version = "1.85"` (the floor set by the pinned clap line).
- **Name reservation stance**: `dogtag` was verified unclaimed on both crates.io and npm on 2026-07-30. Nothing is published to either registry at M1 — no placeholder crates or packages are squatted to hold the names.

### Alternatives considered

- **A single crate with both `lib` and `bin` targets.** Rejected: it erases the SDK/CLI boundary at exactly the layer where it must be enforced — a binary in the same crate can reach non-public items, and the discipline "the CLI consumes only the public API" would become a convention instead of a compile error.
- **Naming the binary `dogtag-cli`.** Rejected: the installed command is the product's most-typed name; `dogtag` is the brand and the docs already promise it.
- **Publishing placeholder packages to reserve the names.** Rejected: both registries discourage squatting, an empty package is a worse first impression than an unclaimed name, and the beta's install path is the release archive, not the registries.
- **Floating on latest stable instead of pinning the toolchain.** Rejected: reproducibility across contributors, CI, and release builds is worth the small chore of bumping the pin deliberately (see the [pinning policy ADR](2026-07-30-dependency-and-pinning-policy.md)).

## Consequences

- The version exists in one place; the CLI cannot drift from the SDK because it reports `dogtag::version()` and carries no version text of its own.
- Registry publication is deferred, so the names remain claimable by others until first publish. Accepted risk; the mitigation is simply not waiting longer than the milestone plan requires.
- Three items to remember at first publish — documented here so they land in the crates.io publish checklist rather than as surprises: a plain `"0.1"` requirement will not match a prerelease, so early consumers of the crate must require `=0.1.0-beta.0` (or the then-current prerelease) explicitly; `dogtag-cli`'s dependency on the SDK crate is path-only today, and crates.io strips `path` keys, so it needs an explicit version requirement — in the exact prerelease form (`=0.1.0-beta.0`) while versions are prereleases — before publish; and the published packages should include the LICENSE file, which lives once at the repository root and is not inside any member crate's directory.
- Toolchain and edition bumps are deliberate, visible commits.
