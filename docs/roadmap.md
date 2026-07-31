# Roadmap

> **The canonical public record of milestone status.** The ladder itself — what each of the nine milestones delivers, and the cutover rule every milestone from M2 on carries — is defined in [beta.md](beta.md#milestones). This document says where the work is now and what the finished milestones produced.

## Now — M2, open and diagnose

The next installed prerelease opens a real vault, explains its resolved contract, and takes over the daily vault health check.

- [ ] Decide the committed vault-contract format and its compatibility version, plus the shape of the local installation record.
- [ ] Implement vault-root discovery and layered configuration loading, with per-setting provenance.
- [ ] Build the `dense` and `starter` fixture contracts and corpora, each through the dedicated fixture privacy pass that the [extraction and sanitization record](decisions/engineering/2026-07-30-document-extraction-sanitization.md) requires of a derived contract.
- [ ] Validate capabilities and compatibility with stable structured diagnostics, and ship `dogtag doctor` and `dogtag contract explain`.
- [ ] Publish and install the next prerelease, run it against a real vault, and move the daily health check off the incumbent personal tooling.

## Next — M3, read and validate

The public document model plus `check`, `list`, and `show`, graduating the conformance scenarios written at M0 across both built fixture profiles. Cutover: reading and listing notes.

## Shipped

| Milestone | Date | Receipt |
| --- | --- | --- |
| **M0** — beta contract and extraction packet | 2026-07-30 | the decisions now carried by [beta.md](beta.md), [architecture.md](architecture.md), [strategy.md](strategy.md), and the [product decision records](decisions/product/README.md) |
| **M1** — clean repository and empty release | 2026-07-30 | this repository, and `v0.1.0-beta.0` published as four macOS and Linux archives with checksums and build-provenance attestations, then installed from the public release on both founder machines |

## What this document is not

- **Not the ladder.** [beta.md](beta.md) defines the nine milestones, the beta's promise, its required properties, and the ship test that ends it.
- **Not the experiment sequence.** [strategy.md](strategy.md) carries E0 through E4 and the stage gates that decide whether the beta graduates.
- **Not a changelog.** Release notes are generated from commit subjects at each tag.
