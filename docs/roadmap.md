# Roadmap

> **The canonical public record of milestone status.** The twelve-rung ladder and the cutover rule are defined in [beta.md](beta.md#milestones). This document says where the work is now and what the finished milestones produced.

## Now — M6, TypeScript SDK and dynamic workflow

The next installed prerelease, nominally `0.1.0-beta.5`, publishes the napi-rs TypeScript binding and deploys an SDK-backed MCP server over one configured vault.

- [x] **Decision packet closed 2026-08-12.** Four records carry it: [the TypeScript binding and npm package](decisions/engineering/2026-08-12-m6-typescript-binding-and-package.md), [the MCP server and inherited residue](decisions/engineering/2026-08-12-m6-mcp-server-and-residue.md), [agent-contract generation](decisions/engineering/2026-08-12-m6-agent-contract-generation.md), and [fixtures, release, and acceptance](decisions/engineering/2026-08-12-m6-fixtures-release-and-acceptance.md).
- [ ] Publish the napi-rs TypeScript SDK and its platform packages at the same prerelease version as Rust and the CLI; verify every claimed platform/runtime pair.
- [ ] Run every applicable semantic scenario through Rust and TypeScript over `dense`, `starter`, and `docs`, including derived version-1 and version-2 variants; no waivers.
- [ ] Ship the four-tool MCP server — `search`, `show`, `capture`, `tags` — over stdio and authenticated HTTP, one verified vault per process.
- [ ] Ship `dogtag contract generate-agents`, commit generated fixture contracts, and enforce non-drift on the founder vault.
- [ ] Keep `contract_version = 3`; move the CLI JSON report schema exactly once, 4 to 5.
- [ ] Extend smoke through npm installation, direct TypeScript operations, agent-contract generation, and both MCP transports; pass independent `just gate` 20/20 and `just smoke`.
- [ ] Publish and verify `0.1.0-beta.5` on macOS and Linux, including native npm package selection.
- [ ] Replace the incumbent connector's search/show/tags mechanism; retire its FTS database, reindex scheduling, and `pkm tags`; record fzf as TUI residue.
- [ ] Establish real MCP capture use and close M5's rolled-forward obligation with the honest no-incumbent receipt; attachment handling remains M9 residue.
- [ ] Record local and deployed MCP search medians; open the persistent-index decision if either median exceeds one second.

## Next — M7, typed creation and triage

Defined in [the ladder](beta.md#milestones); its acceptance criteria land here when M6 closes.

## Shipped

| Milestone | Date | Receipt |
| --- | --- | --- |
| **M0** — beta contract and extraction packet | 2026-07-30 | the decisions now carried by [beta.md](beta.md), [architecture.md](architecture.md), [strategy.md](strategy.md), and the [product decision records](decisions/product/README.md) |
| **M1** — clean repository and empty release | 2026-07-30 | this repository, and `v0.1.0-beta.0` published as four macOS and Linux archives with checksums and build-provenance attestations, installed and verified from the public release |
| **M2** — open and diagnose | 2026-08-03 | `v0.1.0-beta.1` published as four macOS and Linux archives with checksums, provenance attestations, and per-target SBOMs, installed and verified from the public release on both platforms; `dogtag doctor` and `dogtag contract explain` live over the `dense` and `starter` fixture contracts, ten scenarios executable and green. Cutover receipt: **nothing moved** — `doctor` proved purely additive, since the vault's configuration is enforced at commit time and no scheduled configuration check ever existed; the obligation rolled forward into M3's three-part cutover under the roll-forward clause in [beta.md](beta.md#required-properties). See [the release and cutover record's amendments](decisions/engineering/2026-07-31-m2-release-and-cutover.md#amendments) and [the kind-lattice record](decisions/engineering/2026-08-03-the-kind-lattice-against-a-real-corpus.md). |
| **M3** — read and validate | 2026-08-12 | `v0.1.0-beta.2`; document model, `check`, `list`, `show`, contract version 2, 24-scenario release matrix; reading/listing and schema explanation moved to installed Dogtag, while scheduled check was established fresh because no incumbent schedule existed |
| **M4** — lexical retrieval | 2026-08-12 | `v0.1.0-beta.3`; `search`, `find`, `docs` corpus, 36-scenario matrix; daily retrieval moved to installed Dogtag, with connector FTS and `pkm tags` assigned to M6, richer resolution retained as named residue, and fzf assigned to the TUI milestone |
| **M5** — safe mutation | 2026-08-08 | `v0.1.0-beta.4`; capture transaction, contract version 3, 46-scenario matrix; criteria 1–6 complete and the mis-specified capture cutover rolled to M6, where capture gains a real consumer |

## What this document is not

- **Not the ladder.** [beta.md](beta.md) defines the milestones, promise, required properties, and ship test.
- **Not the experiment sequence.** [strategy.md](strategy.md) carries E0 through E4.
- **Not a changelog.** Release notes are generated from commit subjects at each tag.

Why this repository is canonical for milestone status: [documentation architecture and roadmap ownership](decisions/engineering/2026-07-30-documentation-architecture.md).
