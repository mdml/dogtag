# Roadmap

> **The canonical public record of milestone status.** The ladder itself — what each of the nine milestones delivers, and the cutover rule every milestone from M2 on carries — is defined in [beta.md](beta.md#milestones). This document says where the work is now and what the finished milestones produced.

## Now — M3, read and validate

The public document model plus `check`, `list`, and `show`, against the shared conformance scenarios written at M0 — widened 2026-08-03 to carry the first `contract_version` bump (a tag-vocabulary construct and the `record` kind) and M2's deferred schema-explanation cutover, per [the ladder](beta.md#milestones). The decision packet is the next act; acceptance criteria land here when it freezes.

## Shipped

| Milestone | Date | Receipt |
| --- | --- | --- |
| **M0** — beta contract and extraction packet | 2026-07-30 | the decisions now carried by [beta.md](beta.md), [architecture.md](architecture.md), [strategy.md](strategy.md), and the [product decision records](decisions/product/README.md) |
| **M1** — clean repository and empty release | 2026-07-30 | this repository, and `v0.1.0-beta.0` published as four macOS and Linux archives with checksums and build-provenance attestations, installed and verified from the public release |
| **M2** — open and diagnose | 2026-08-03 | `v0.1.0-beta.1` published as four macOS and Linux archives with checksums, provenance attestations, and per-target SBOMs, installed and verified from the public release on both platforms; the cutover was re-pointed to schema explanation and carried into M3, per [the release and cutover record](decisions/engineering/2026-07-31-m2-release-and-cutover.md) and [the kind-lattice record](decisions/engineering/2026-08-03-the-kind-lattice-against-a-real-corpus.md) |

## What this document is not

- **Not the ladder.** [beta.md](beta.md) defines the nine milestones, the beta's promise, its required properties, and the ship test that ends it.
- **Not the experiment sequence.** [strategy.md](strategy.md) carries E0 through E4 and the stage gates that decide whether the beta graduates.
- **Not a changelog.** Release notes are generated from commit subjects at each tag.

Why this repository is canonical for milestone status: [documentation architecture and roadmap ownership](decisions/engineering/2026-07-30-documentation-architecture.md).
