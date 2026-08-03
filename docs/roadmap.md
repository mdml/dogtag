# Roadmap

> **The canonical public record of milestone status.** The ladder itself — what each of the nine milestones delivers, and the cutover rule every milestone from M2 on carries — is defined in [beta.md](beta.md#milestones). This document says where the work is now and what the finished milestones produced.

## Now — M3, read and validate

The next installed prerelease, nominally `0.1.0-beta.2`, reads and validates a real corpus: the public document model, `check`, `list`, and `show`, over the first `contract_version` bump.

- [x] **Decision packet closed 2026-08-03.** Six records carry it, beginning with [the public document model](decisions/engineering/2026-08-03-m3-document-model.md); the acceptance criteria below are fixed in [the M3 release and cutover record](decisions/engineering/2026-08-03-m3-release-and-cutover.md).
- [ ] The document model and the three surfaces implemented; all 24 conformance scenarios executable and green on `dense` and `starter` — the nine M3 scenarios graduated and five added — with the matrix distinguishing a pair that ran from one pending on an unbuilt corpus.
- [ ] `contract_version = 2` — the tag vocabulary and the `record` kind — with the supported range widening to `1..=2` only in the change that carries the per-version key sets and default tables, per [contract version 2](decisions/engineering/2026-08-03-contract-version-2.md).
- [ ] Both fixture corpora at version 2 with notes: `dense`'s derived numerically under [the extended privacy gate](decisions/engineering/2026-08-03-m3-fixtures-and-conformance.md) with its receipts recorded, `starter`'s as the normative `init` output, and every new floor harness-enforced in the change that raises it.
- [ ] The scripted fixture smoke sequence green before the tag; the prerelease published, and installed from the public release with verified attestations on macOS and Linux.
- [ ] The three-part cutover complete, each part with seven days of parallel running and its incumbent retired: the scheduled corpus lint onto `check --strict`, reading and listing onto `list` and `show`, and schema explanation onto `contract explain`.

## Next — M4, lexical retrieval

Defined in [the ladder](beta.md#milestones); its acceptance criteria land here when it becomes the active rung.

## Shipped

| Milestone | Date | Receipt |
| --- | --- | --- |
| **M0** — beta contract and extraction packet | 2026-07-30 | the decisions now carried by [beta.md](beta.md), [architecture.md](architecture.md), [strategy.md](strategy.md), and the [product decision records](decisions/product/README.md) |
| **M1** — clean repository and empty release | 2026-07-30 | this repository, and `v0.1.0-beta.0` published as four macOS and Linux archives with checksums and build-provenance attestations, installed and verified from the public release |
| **M2** — open and diagnose | 2026-08-03 | `v0.1.0-beta.1` published as four macOS and Linux archives with checksums, provenance attestations, and per-target SBOMs, installed and verified from the public release on both platforms; `dogtag doctor` and `dogtag contract explain` live over the `dense` and `starter` fixture contracts, ten scenarios executable and green. Cutover receipt: **nothing moved** — `doctor` proved purely additive, since the vault's configuration is enforced at commit time and no scheduled configuration check ever existed; the obligation rolled forward into M3's three-part cutover under the roll-forward clause in [beta.md](beta.md#required-properties). See [the release and cutover record's amendments](decisions/engineering/2026-07-31-m2-release-and-cutover.md#amendments) and [the kind-lattice record](decisions/engineering/2026-08-03-the-kind-lattice-against-a-real-corpus.md). |

## What this document is not

- **Not the ladder.** [beta.md](beta.md) defines the nine milestones, the beta's promise, its required properties, and the ship test that ends it.
- **Not the experiment sequence.** [strategy.md](strategy.md) carries E0 through E4 and the stage gates that decide whether the beta graduates.
- **Not a changelog.** Release notes are generated from commit subjects at each tag.

Why this repository is canonical for milestone status: [documentation architecture and roadmap ownership](decisions/engineering/2026-07-30-documentation-architecture.md).
