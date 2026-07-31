# Roadmap

> **The canonical public record of milestone status.** The ladder itself — what each of the nine milestones delivers, and the cutover rule every milestone from M2 on carries — is defined in [beta.md](beta.md#milestones). This document says where the work is now and what the finished milestones produced.

## Now — M2, open and diagnose

The next installed prerelease, `0.1.0-beta.1`, opens a real vault, explains its resolved contract, and takes over the vault's daily configuration health check.

- [x] **Decision packet closed 2026-07-31.** The committed contract is `.dogtag/contract.toml` (TOML, integer `contract_version`, fatal unknown keys); the local installation record is `~/.config/dogtag/installation.toml`, read and never written at M2. Six records carry it, beginning with [the vault contract and installation record](decisions/engineering/2026-07-31-vault-contract-and-installation-record.md).
- [ ] Implement vault-root discovery and configuration resolution, with per-leaf provenance across the committed contract, the local record, and format defaults.
- [ ] Author the `dense` and `starter` fixture contracts — `dense` from numeric shape alone, with all vocabulary authored, per [the fixture and privacy record](decisions/engineering/2026-07-31-m2-fixtures-and-the-privacy-gate.md).
- [ ] Validate capabilities, the lifecycle declaration, and compatibility with structured diagnostics carrying shared identifiers, and ship `dogtag doctor` and `dogtag contract explain`.
- [ ] Graduate the M2 conformance scenarios against both fixtures, and raise the coverage baseline with the kernel code that lands.
- [ ] Publish and install `0.1.0-beta.1` from the public release, then complete the cutover: seven days of parallel running before the incumbent configuration check is retired.

**Blocked on infrastructure.** GitHub Actions is not assigning runners for this repository, so required checks cannot execute. Work proceeds locally under `just gate`; nothing pushes, merges, tags, or publishes until required checks run and pass. See [the release and cutover record](decisions/engineering/2026-07-31-m2-release-and-cutover.md).

## Next — M3, read and validate

Defined in [the ladder](beta.md#milestones); its acceptance criteria land here when it becomes the active rung.

## Shipped

| Milestone | Date | Receipt |
| --- | --- | --- |
| **M0** — beta contract and extraction packet | 2026-07-30 | the decisions now carried by [beta.md](beta.md), [architecture.md](architecture.md), [strategy.md](strategy.md), and the [product decision records](decisions/product/README.md) |
| **M1** — clean repository and empty release | 2026-07-30 | this repository, and `v0.1.0-beta.0` published as four macOS and Linux archives with checksums and build-provenance attestations, installed and verified from the public release |

## What this document is not

- **Not the ladder.** [beta.md](beta.md) defines the nine milestones, the beta's promise, its required properties, and the ship test that ends it.
- **Not the experiment sequence.** [strategy.md](strategy.md) carries E0 through E4 and the stage gates that decide whether the beta graduates.
- **Not a changelog.** Release notes are generated from commit subjects at each tag.

Why this repository is canonical for milestone status: [documentation architecture and roadmap ownership](decisions/engineering/2026-07-30-documentation-architecture.md).
