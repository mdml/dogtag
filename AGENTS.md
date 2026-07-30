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

Recipes live in the [justfile](justfile); `just` alone lists them. `just install-dev-tools` installs the pinned Rust toolchains and cargo tools, and prints instructions for the two it cannot install safely on its own (osv-scanner, whose packaging is platform-specific, and lefthook). It assumes `jq` and `python3` are already present.

- `just check` — format check, clippy with warnings as errors, full test suite, docs with warnings as errors, and the offline policy checks. Deterministic and offline; **run it before handing off work.**
- `just gate` — everything CI enforces that can run locally: `check` plus coverage, MSRV, `cargo-deny`, OSV, zizmor, and Code Health. Needs the network and a CodeScene token, and takes minutes rather than seconds. CI remains authoritative.
- `just fmt` / `just test` / `just build` — the individual steps.
- `just conformance` — run the conformance harness and print the scenario × profile matrix.
- `just coverage` — measure coverage and enforce the thresholds and ratchet baseline.
- `just msrv` — build and test against the declared MSRV floor.
- `just deny` / `just osv` / `just zizmor` — Rust advisories, licenses, bans and sources; cross-ecosystem vulnerability scan; workflow security lint.
- `just codescene` — Code Health of every supported file. Narrower and faster: `just codescene-staged` before a commit, `just codescene-branch [BASE]` for the whole branch, `just codescene-files <paths>` for specific files.
- `just semver` — API-compatibility check against the last release tag. Advisory until the first non-prerelease tag; see the ADR for why a blocking gate would be meaningless before then.
- `just commits [RANGE]` / `just hooks` / `just notes` — validate commit messages, install the git hooks, preview the next release's notes.
- `just dist` — release-build the CLI and package a host-target archive into `dist/`, using the same script as the release pipeline.
- `just install-local` — rehearse `install.sh` end-to-end against the locally packaged `dist/`.
- `just links` — offline check that relative Markdown links resolve, via the dependency-free `scripts/check-links.sh` (the same script the CI links job runs).

## Quality and security gates

These are contractual, not aspirational — each one is enforced by a tool, and none can be satisfied by asserting it. The reasoning, the rejected alternatives, and the activation triggers for gates that are not live yet live in four records: [code health and coverage](docs/adr/2026-07-30-code-health-and-coverage-gates.md), [supply chain and vulnerabilities](docs/adr/2026-07-30-supply-chain-and-vulnerability-policy.md), [workflow security and repository rules](docs/adr/2026-07-30-workflow-security-and-repository-rules.md), and [commit convention and release notes](docs/adr/2026-07-30-commit-convention-and-release-notes.md).

- **Code Health 10.0, on stock rules.** Every CodeScene-supported file scores exactly 10.0. When a file falls short, refactor it — never adjust a threshold or add a rules file. There is no rules configuration in this repository and adding one is out of bounds.
- **Coverage floors with a ratchet.** ≥95% line and ≥90% branch overall, 100% line in the semantic kernel, and never below the committed baseline in [coverage-baseline.toml](coverage-baseline.toml). Raising the baseline is routine; lowering it or a threshold needs an ADR. Coverage-exclusion attributes are not allowed — restructure or test the code instead.
- **Unsafe code is forbidden**, mechanically, in every crate and every test target. Introducing any requires its own ADR, an isolated module, documented safety invariants, targeted tests, Miri where applicable, and independent review.
- **Everything is pinned.** Dependencies, toolchains, actions (full commit SHAs), and tools. Tool versions live in [tools.toml](tools.toml) and `just check` fails if a workflow drifts from it. Dependency updates are proposed by Dependabot and merged by a human — never automatically.
- **Security suppressions are data, and they expire.** Any ignore in any security tool needs an entry in [docs/security/exceptions.toml](docs/security/exceptions.toml) carrying a rationale, an owner, an expiry date, and a link to its record. Unregistered suppressions and stale entries both fail CI. Comments in a tool config are not an exception process.
- **The package quarantine holds.** New TypeScript dependencies must be seven days old; the standing exclusion list stays empty, and an urgent security release is a one-off, recorded override rather than a permanent hole.
- **Tags come only from green commits, and published releases are immutable.** A bad release is never edited or deleted — the fix ships forward under a new version, with a security advisory when users must act. This one is enforced by repository rulesets rather than by anything in the tree: the payloads are checked in at [.github/rulesets/](.github/rulesets/), and the rules hold exactly once an admin has applied them.

## Commits

Conventional Commit subjects, present tense, one coherent change per commit. The permitted types are `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `build`, `ci`, `chore`, and `revert`; scopes are optional; a breaking change is marked with `!` before the colon, a `BREAKING CHANGE:` footer, or both.

This is enforced, not merely asked for. `just hooks` installs a `commit-msg` hook that checks the message as you write it, and the `Commit messages` CI job re-checks every commit a pull request introduces — so a bypassed hook still cannot merge. `just commits [RANGE]` runs the same check by hand. **Pull requests merge by rebase only**, so the commits CI validated are exactly the commits that land on `main`; squash merging would synthesize a subject from the pull-request title that no gate ever saw. Tidy your branch before merging, not during. Release notes are generated from these subjects (see the [commit convention ADR](docs/adr/2026-07-30-commit-convention-and-release-notes.md)), so a careless subject becomes user-visible text; `just notes` previews what the next tag would publish.

## Decision records — which one to write

- **PDR** (`docs/decisions/`) — a product stance: how dogtag behaves for its users, written timelessly and for any PKM. See [docs/decisions/README.md](docs/decisions/README.md).
- **ADR** (`docs/adr/`) — a build decision: this repository's layout, toolchain, dependencies, pipelines, policy. See [docs/adr/README.md](docs/adr/README.md).

Rule of thumb: if the decision would still matter to someone reimplementing dogtag from the docs alone, it is a PDR; if it only matters to someone working in this repository, it is an ADR. Both trails supersede — they never delete.

## Releases

Pushing a tag `v<version>` runs the release workflow: it builds the target matrix, packages archives with checksums via `scripts/package.sh` (the same script `just dist` uses), and creates a **draft** GitHub release. Publishing the draft is always a human act — no automation ever publishes. The workflow fails if the tag does not match the workspace package version, so bump the version first, tag second, and only on a commit whose required checks have passed. Once published, a release and its tag are immutable; a problem in a shipped version is fixed forward under a new version, never by moving a tag. The repository rulesets that enforce all of this mechanically — required checks before merge, immutable `v*` tags — are recorded in the [workflow security ADR](docs/adr/2026-07-30-workflow-security-and-repository-rules.md) and are applied by a repository admin; until they are applied, these are conventions, not guarantees.
