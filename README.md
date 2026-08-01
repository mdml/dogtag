# Dogtag

> **Status: beta product draft (2026-07-29).** This README is written as if the product shipped — readme-driven development makes it true. Start here for what it is and the first hour; [product.md](docs/product.md) carries the full case, [abstractions.md](docs/abstractions.md) the domain concepts, [architecture.md](docs/architecture.md) the SDK architecture, [beta.md](docs/beta.md) the first release contract, and [strategy.md](docs/strategy.md) the experiment sequence.

> **Beta status.** The published release, `0.1.0-beta.0`, is the empty vertical slice: it proves the complete release path end to end, and the only command it carries is `dogtag version`. Install it with `curl -fsSL https://raw.githubusercontent.com/mdml/dogtag/main/install.sh | sh` (`dogtag.dev` will front the same script). Commands land in the repository before they land in a release, so what is installable today is behind what is implemented — the rest of this document set describes the product under construction, readme-driven development, with the milestone ladder in [beta.md](docs/beta.md) and where the work actually stands in [roadmap.md](docs/roadmap.md). The Getting started below is the destination, not the current state.

Dogtag is a personal knowledge management SDK designed for AI agents and applications: they configure it, co-author the notes, and keep the vault maintained. The CLI, MCP server, TUI, webhooks, CI jobs, and agent loops are consumers of the same public Rust, TypeScript, Python, and later bindings—not separate homes of vault behavior. The full case — why now, who it's for, what's deliberately absent — is in [product.md](docs/product.md). The whole documentation set is indexed in [docs/](docs/README.md), including both decision trails; contributor and agent instructions are in [AGENTS.md](AGENTS.md).

## Getting started

Getting started is one binary and one conversation. `brew install dogtag` (or the install script) drops a single static binary; `dogtag init` stamps a vault with [Portent](https://portent.md/) defaults — eight object types plus a Dogtag-added ninth, `Source` (immutable captures: transcripts, exports, originals — agents read them, never rewrite them), two relationship types, a captured → organized → archived lifecycle — while `dogtag import` adopts an existing folder of markdown instead, the agent classifying and linking what it can and asking about the rest. The contract is a committed config file, and customizing it is a conversation: tell your agent to rename a type, add one, or trim the relationship vocabulary, and it edits the config, validated by the same contract the tool enforces everywhere. (You could edit it by hand; almost no one ever will.) Connecting a git remote is recommended, not required — any host works; the privacy posture is yours. Starter kits and an agent-led schema interview come later; v1 never makes a new user design a schema before their first note. Within the hour: first capture, first triage, first search.

## What a vault looks like

A conforming vault on disk (file names illustrative):

```
my-vault/
  <notes, in whatever folders the user likes — folders carry no semantics>
  .dogtag/
    contract.toml   # the one committed asset: types, properties, relationships,
                    # capabilities, lifecycle, write policy, dialect, version
  skills/           # travel-with-vault agent verbs
  AGENTS.md         # generated agent contract — regenerated from the contract, not hand-edited
  .index/           # derived search index + caches, gitignored
```

One committed file, not two: the *canonical* dialect travels inside the contract so every collaborator reads the corpus the same way. A machine-local installation record, which never leaves your machine and is never committed, carries what genuinely differs per machine — including which editor dialect gets materialized, so teammates sharing a vault can each keep their favored editor. `AGENTS.md` and `.index/` above are generated and derived respectively; neither is written by the current release. The format is decided in [the vault contract record](docs/decisions/engineering/2026-07-31-vault-contract-and-installation-record.md).

## Verifying a release

Every release publishes, per target, a `.tar.gz`, its `.sha256` sidecar, and a CycloneDX SBOM with its own sidecar, plus one aggregate `sha256.sum` covering all of them. Checksums first:

```sh
sha256sum -c sha256.sum
```

Both attestations are minted by [the release workflow](.github/workflows/release.yml) and bound to this repository. Verify the build provenance:

```sh
gh attestation verify dogtag-x86_64-unknown-linux-musl.tar.gz \
  --repo mdml/dogtag \
  --signer-workflow mdml/dogtag/.github/workflows/release.yml
```

The SBOM is attested separately, as a predicate over the same archive. **`--predicate-type` is required**: without it the command verifies the provenance attestation, exits 0, and tells you nothing about the SBOM — a success that looks exactly like the one you wanted.

```sh
gh attestation verify dogtag-x86_64-unknown-linux-musl.tar.gz \
  --repo mdml/dogtag \
  --signer-workflow mdml/dogtag/.github/workflows/release.yml \
  --predicate-type https://cyclonedx.org/bom
```

The SBOM describes one binary on one target and is reconciled against the closure the build actually resolves, so it names what the binary links and nothing more. The rules are in [the supply-chain policy](docs/decisions/engineering/2026-07-30-supply-chain-and-vulnerability-policy.md) and [the release pipeline record](docs/decisions/engineering/2026-07-30-release-pipeline-and-artifacts.md).
