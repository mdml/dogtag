# Working in this repository

Canonical instructions for anyone — human or agent — contributing here. `CLAUDE.md` is a pointer to this file; if the two ever diverge, this file wins.

## What this repository is

Dogtag is a personal knowledge management SDK designed for AI agents: they configure it, co-author the notes, and keep the vault maintained. The kernel model is four abstractions — note, type, property, relationship — with everything else declared configuration; the SDK (`crates/dogtag`) is the product, and every surface (CLI today; TUI, MCP server, and language bindings later) is a consumer of its public API. This repository holds the Rust workspace, the conformance harness, the reserved TypeScript binding scaffold, and the product documentation set. Reading order for the doc set: [README.md](README.md) (what it is and the first hour), [product.md](docs/product.md) (the full case), [abstractions.md](docs/abstractions.md) (domain concepts), [architecture.md](docs/architecture.md) (SDK architecture), [beta.md](docs/beta.md) (the first release contract, including the milestone ladder), [strategy.md](docs/strategy.md) (the experiment sequence). [docs/roadmap.md](docs/roadmap.md) names the milestone currently in flight; [docs/README.md](docs/README.md) indexes the whole corpus.

## Load-bearing boundaries

Three rules carry the architecture. Do not trade any of them for convenience — if one is genuinely in the way, that is a decision record, not a workaround.

1. **The SDK is the kernel; the CLI consumes only its public API.** All vault behavior lives in `crates/dogtag`. If `crates/dogtag-cli` needs something, the SDK grows public surface — never a private backchannel, a copied constant, or a reimplementation. (This goes as far as the version string: the CLI reports `dogtag::version()` and carries no version text of its own.)
2. **No behavior ahead of its milestone.** Features land at the milestone that owns them (see [beta.md](docs/beta.md); [roadmap.md](docs/roadmap.md) names the rung in flight). M5 has landed exactly one mutation, `capture`, and the fence sits immediately after it: no triage verbs, no edits, no `init`, no `import`, no `migrate`, **no second write path**, no persistent index, no TypeScript binding source, no MCP server — nothing later-milestone lands early, and narrow interfaces and pending fixtures are the ceiling for foreshadowing. Two writes that might look like natural companions are scheduled elsewhere on purpose: `AGENTS.md` generation waits for M6, where the MCP server gives the agent contract its consumer, and `init` is an M7 packet question. The one-mutation scope is load-bearing rather than decorative — the plan, collision, and atomicity decisions a multi-file mutation needs are deliberately unmade, so the first verb that needs them owns making them.
3. **Conformance has no waivers.** Every scenario runs against every fixture profile; there is no profile-specific skip, allowlist, or waiver mechanism, and the scenario format deliberately has no field that could name a profile. Do not add one. See [conformance/README.md](conformance/README.md).

A fourth rule guards the future surfaces: **bindings hold no semantics.** `bindings/typescript` (and any later binding) wraps the one Rust core; it never reimplements vault behavior, and until its milestone it contains no source at all.

## Commands

Recipes live in the [justfile](justfile); `just` alone lists them. `just install-dev-tools` installs the pinned Rust toolchains and cargo tools, and prints instructions for the two it cannot install safely on its own (osv-scanner, whose packaging is platform-specific, and lefthook). It assumes `jq` and `python3` are already present.

Four commands carry the day-to-day work, as a ladder. Each is a strict superset of the one above it, so climbing a rung adds checks and never re-runs the ones below differently.

| Command | When | What it costs |
| --- | --- | --- |
| `just fast` | while implementing | 2s warm, offline. **Not a merge signal** |
| `just check` | before handing work off | 5s warm, offline and deterministic |
| `just gate` | before opening or updating a pull request, **and before merging anyone else's** | ~3m30s warm and growing with the suite (Code Health dominates it, one round trip per file); needs the network, the pinned tools, and `CS_ACCESS_TOKEN` |
| `just gate-verbose` | when you want the evidence | identical run, every gate's full output |

- `just fast` — format check, clippy with warnings as errors, the test suite, commit-message validation, and the cheap policy checks (tool pins, security exceptions).
- `just check` — `fast` plus docs with warnings as errors, link integrity, and the remaining deterministic offline checks (ruleset payloads, gate-table parity, the runner's own tests).
- `just gate` — `check` plus coverage, MSRV, `cargo-deny`, OSV, zizmor, and Code Health. For every gate except Code Health, CI and the repository rulesets remain authoritative and a green `gate` is evidence rather than permission. **Code Health is the exception: `just gate` is the only place it is enforced at all** (see below). A missing tool or token fails **that gate only**, with the command that fixes it: without `CS_ACCESS_TOKEN` the other nineteen still run, `codescene` fails, and the suite exits nonzero. It is never a skip — a Code Health gate that goes green without measuring is worse than none.
- `just gate-verbose` — the same gates, the same thresholds, the same exit codes, with each gate's output printed as it runs. Verbosity changes **rendering only**; [scripts/test_gate.py](scripts/test_gate.py) is what holds that claim up.

Every gate in those suites is declared once, in [scripts/gate.py](scripts/gate.py), as the exact command CI runs — `just gates` prints that table, and `scripts/check-gate-parity.py` fails `just check` if a command drifts from its workflow, if a step loses an environment variable CI sets, if a required status check loses its local counterpart, or if a repository script starts running in CI with nothing local behind it. The gaps run both ways and both are recorded rather than left as absences: two required checks have no local counterpart (`Test (macOS arm64)`, and the musl release build, which `just dist` rehearses), and one local gate has no required check (`codescene` — the parity checker additionally fails if any CodeScene marker reappears in a workflow, so restoring the job and restoring its required context cannot come apart).

Narrower recipes, for when the ladder is more than you need. The expensive ones each have a `-verbose` twin:

- `just fmt` (writes) / `just build` / `just conformance` / `just smoke` — individual steps that are not gates; `smoke` is the scripted fixture sequence that must be green before every release tag.
- `just test` / `just coverage` / `just msrv` / `just deny` / `just osv` / `just zizmor` / `just links` / `just codescene`, each with a `-verbose` twin.
- `just codescene-staged` before a commit, `just codescene-branch [BASE]` for the whole branch, `just codescene-files <paths>` for specific files. These are deltas and already print their findings in full, so they need no verbose twin.
- `just semver` — API-compatibility check against the last release tag. Advisory until the first non-prerelease tag; see the ADR for why a blocking gate would be meaningless before then.
- `just commits [RANGE]` / `just hooks` / `just notes` — validate an explicit commit range, install the git hooks, preview the next release's notes.
- `just dist` — release-build the CLI, package a host-target archive into `dist/`, and generate its SBOM, using the same scripts as the release pipeline.
- `just install-local` — rehearse `install.sh` end-to-end against the locally packaged `dist/`.

Three honest limits. The commit check — in `just fast` and by default in `just commits` — resolves `origin/main..HEAD`, while CI validates exactly the commits a pull request introduces, so a stale `origin/main` moves the range under you; it reports `skip`, never `pass`, when that range is empty. `just gate` cannot run the macOS suite or the musl release build; those pass only in CI. And `just commits <range>` and the `commit-msg` hook invoke the validator directly rather than through the gate table, so they are the two commands parity does not cover. The [gate ergonomics ADR](docs/decisions/engineering/2026-07-30-gate-ergonomics-and-the-command-ladder.md) records the four places local and CI deliberately differ, and what the parity checker cannot see.

## Quality and security gates

These are contractual, not aspirational — each one is enforced by a tool, and none can be satisfied by asserting it. The reasoning, the rejected alternatives, and the activation triggers for gates that are not live yet live in four records: [code health and coverage](docs/decisions/engineering/2026-07-30-code-health-and-coverage-gates.md), [supply chain and vulnerabilities](docs/decisions/engineering/2026-07-30-supply-chain-and-vulnerability-policy.md), [workflow security and repository rules](docs/decisions/engineering/2026-07-30-workflow-security-and-repository-rules.md), and [commit convention and release notes](docs/decisions/engineering/2026-07-30-commit-convention-and-release-notes.md).

- **Code Health 10.0, on stock rules — enforced locally, not by CI.** Every CodeScene-supported file scores exactly 10.0. When a file falls short, refactor it — never adjust a threshold or add a rules file. There is no rules configuration in this repository and adding one is out of bounds.

  This one is **not** a required status check and cannot be: it needs a CodeScene credential, and a forked pull request cannot have one, which leaves only failing every external contribution or passing it unmeasured. So it lives in `just gate`, which is fail-closed without a token, plus a pre-commit delta and a pre-push branch delta. **A maintainer must run `just gate` locally before merging any contribution, including their own** — nothing mechanical stops a merge that skipped it. Contributors need no CodeScene account: the hooks print a conspicuous `CODE HEALTH NOT MEASURED` notice and decline, never a false pass. Bumping the pinned CodeScene CLI requires a full `just codescene` sweep and moving `swept_at` in [tools.toml](tools.toml) in the same commit, because new CLI rules can drop an untouched file below 10.0 where no delta would ever see it. See the [code health ADR](docs/decisions/engineering/2026-07-30-code-health-and-coverage-gates.md).
- **Coverage floors with a ratchet.** ≥95% line and ≥90% branch overall, 100% line in the semantic kernel, and never below the committed baseline in [coverage-baseline.toml](coverage-baseline.toml). Raising the baseline is routine; lowering it or a threshold needs an ADR. Coverage-exclusion attributes are not allowed — restructure or test the code instead.
- **Unsafe code is forbidden**, mechanically, in every crate and every test target. Introducing any requires its own ADR, an isolated module, documented safety invariants, targeted tests, Miri where applicable, and independent review.
- **Everything is pinned.** Dependencies, toolchains, actions (full commit SHAs), and tools. Tool versions live in [tools.toml](tools.toml) and `just check` fails if a workflow drifts from it. Dependency updates are proposed by Dependabot and merged by a human — never automatically.
- **Security suppressions are data, and they expire.** Any ignore in any security tool needs an entry in [security-exceptions.toml](security-exceptions.toml) carrying a rationale, an owner, an expiry date, and a link to its record. Unregistered suppressions and stale entries both fail CI. Comments in a tool config are not an exception process.
- **The package quarantine holds.** New TypeScript dependencies must be seven days old; the standing exclusion list stays empty, and an urgent security release is a one-off, recorded override rather than a permanent hole.
- **Tags come only from green commits, and published releases are immutable.** A bad release is never edited or deleted — the fix ships forward under a new version, with a security advisory when users must act. This one is enforced by repository rulesets rather than by anything in the tree: the payloads are checked in at [.github/rulesets/](.github/rulesets/), and the rules hold exactly once an admin has applied them.

## Commits

Conventional Commit subjects, present tense, one coherent change per commit. The permitted types are `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `build`, `ci`, `chore`, and `revert`; scopes are optional; a breaking change is marked with `!` before the colon, a `BREAKING CHANGE:` footer, or both.

This is enforced, not merely asked for. `just hooks` installs a `commit-msg` hook that checks the message as you write it, and the `Commit messages` CI job re-checks every commit a pull request introduces — so a bypassed hook still cannot merge. `just commits [RANGE]` runs the same check by hand. **Pull requests merge by rebase only**, so the commits CI validated are exactly the commits that land on `main`; squash merging would synthesize a subject from the pull-request title that no gate ever saw. Tidy your branch before merging, not during. Release notes are generated from these subjects (see the [commit convention ADR](docs/decisions/engineering/2026-07-30-commit-convention-and-release-notes.md)), so a careless subject becomes user-visible text; `just notes` previews what the next tag would publish.

## Decision records — which one to write

- **PDR** (`docs/decisions/product/`) — a product stance: how dogtag behaves for its users, written timelessly and for any PKM. See [docs/decisions/product/README.md](docs/decisions/product/README.md).
- **ADR** (`docs/decisions/engineering/`) — a build decision: this repository's layout, toolchain, dependencies, pipelines, policy. See [docs/decisions/engineering/README.md](docs/decisions/engineering/README.md).

Rule of thumb: if the decision would still matter to someone reimplementing dogtag from the docs alone, it is a PDR; if it only matters to someone working in this repository, it is an ADR — [docs/decisions/README.md](docs/decisions/README.md) is the canonical statement of that test, and of the documents it routes besides decisions. Both trails supersede — they never delete.

## Releases

Pushing a tag `v<version>` runs the release workflow: it builds the target matrix, packages archives with checksums via `scripts/package.sh` and a per-target CycloneDX SBOM via `scripts/sbom.sh` (the same scripts `just dist` uses), attests each archive's build provenance and its SBOM, and creates a **draft** GitHub release. Publishing the draft is always a human act — no automation ever publishes. The workflow fails if the tag does not match the workspace package version, so bump the version first, tag second, and only on a commit whose required checks have passed. Once published, a release and its tag are immutable; a problem in a shipped version is fixed forward under a new version, never by moving a tag. The repository rulesets that enforce all of this mechanically — required checks before merge, immutable `v*` tags — are recorded in the [workflow security ADR](docs/decisions/engineering/2026-07-30-workflow-security-and-repository-rules.md) and are applied by a repository admin; until they are applied, these are conventions, not guarantees.
