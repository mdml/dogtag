# Working in this repository

Canonical instructions for anyone — human or agent — contributing here. `CLAUDE.md` is a pointer to this file; if the two ever diverge, this file wins.

## What this repository is

Dogtag is a personal knowledge management SDK designed for AI agents: they configure it, co-author the notes, and keep the vault maintained. The kernel model is four abstractions — note, type, property, relationship — with everything else declared configuration; the SDK (`crates/dogtag`) is the product, and every surface (CLI today; TUI, MCP server, and language bindings later) is a consumer of its public API. This repository holds the Rust workspace, the conformance harness, the reserved TypeScript binding scaffold, and the product documentation set. Reading order for the doc set: [README.md](README.md) (what it is and the first hour), [PRODUCT.md](PRODUCT.md) (the full case), [ABSTRACTIONS.md](ABSTRACTIONS.md) (domain concepts), [ARCHITECTURE.md](ARCHITECTURE.md) (SDK architecture), [BETA.md](BETA.md) (the first release contract, including the milestone ladder), [STRATEGY.md](STRATEGY.md) (the experiment sequence).

## Load-bearing boundaries

Three rules carry the architecture. Do not trade any of them for convenience — if one is genuinely in the way, that is a decision record, not a workaround.

1. **The SDK is the kernel; the CLI consumes only its public API.** All vault behavior lives in `crates/dogtag`. If `crates/dogtag-cli` needs something, the SDK grows public surface — never a private backchannel, a copied constant, or a reimplementation. (This goes as far as the version string: the CLI reports `dogtag::version()` and carries no version text of its own.)
2. **No behavior ahead of its milestone.** Features land at the milestone that owns them (see [BETA.md](BETA.md)). Do not implement vault discovery, configuration loading, contract validation, diagnostics, or any later-milestone behavior early — narrow interfaces and pending fixtures are the ceiling for foreshadowing.
3. **Conformance has no waivers.** Every scenario runs against every fixture profile; there is no profile-specific skip, allowlist, or waiver mechanism, and the scenario format deliberately has no field that could name a profile. Do not add one. See [conformance/README.md](conformance/README.md).

A fourth rule guards the future surfaces: **bindings hold no semantics.** `bindings/typescript` (and any later binding) wraps the one Rust core; it never reimplements vault behavior, and until its milestone it contains no source at all.

## Commands

Recipes live in the [justfile](justfile); `just` alone lists them.

- `just check` — format check, clippy with warnings as errors, full test suite. This is the CI gate; run it before handing off work.
- `just fmt` / `just test` / `just build` — the individual steps.
- `just conformance` — run the conformance harness and print the scenario × profile matrix.
- `just dist` — release-build the CLI and package a host-target archive into `dist/`, using the same script as the release pipeline.
- `just install-local` — rehearse `install.sh` end-to-end against the locally packaged `dist/`.
- `just links` — offline check that relative Markdown links resolve, via the dependency-free `scripts/check-links.sh` (the same script the CI links job runs).

## Commits

Conventional commit subjects (`feat:`, `fix:`, `docs:`, `test:`, `chore:`, `ci:`, …), present tense, one coherent change per commit.

## Decision records — which one to write

- **PDR** (`docs/decisions/`) — a product stance: how dogtag behaves for its users, written timelessly and for any PKM. See [docs/decisions/README.md](docs/decisions/README.md).
- **ADR** (`docs/adr/`) — a build decision: this repository's layout, toolchain, dependencies, pipelines, policy. See [docs/adr/README.md](docs/adr/README.md).

Rule of thumb: if the decision would still matter to someone reimplementing dogtag from the docs alone, it is a PDR; if it only matters to someone working in this repository, it is an ADR. Both trails supersede — they never delete.

## Releases

Pushing a tag `v<version>` runs the release workflow: it builds the target matrix, packages archives with checksums via `scripts/package.sh` (the same script `just dist` uses), and creates a **draft** GitHub release. Publishing the draft is always a human act — no automation ever publishes. The workflow fails if the tag does not match the workspace package version, so bump the version first, tag second.
