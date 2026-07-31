# Code health and coverage gates

- Status: accepted
- Date: 2026-07-30

## Context

M1's gate was formatting, clippy, and tests — necessary, but it measures nothing about design quality or test depth, and those are exactly the properties that erode invisibly in a repository maintained largely by agents. Self-assessed quality is worthless in that setting: an agent (or a rushed human) will sincerely report "clean, well tested" over code that is neither, so every gate in this contract must be mechanical, absolute, and computed by a tool rather than claimed by a contributor. The semantic kernel (`crates/dogtag`) carries a stricter bar than the shells around it because the kernel is the product (see [architecture.md](../../architecture.md)). These decisions were made against the 2026-07-30 state of the tooling; the pinned versions are the record of that state.

## Decision

### CodeScene: every supported file scores 10.0, on stock rules

Every CodeScene-supported source file must score exactly 10.0 Code Health, always. Scoring uses **stock CodeScene rules only**: no rules file, no threshold customization, ever. A file reaches 10.0 by being refactored, and the change is recorded as a refactor. The moment thresholds become configurable, the gate measures the configuration instead of the code.

Verifying that invariant from scratch means one network round trip per file, which is too slow to run on every commit and gets slower as the repository grows. So the invariant is **held inductively** rather than re-derived: the floor was established once — every file scored 10.0 in the change that introduced this contract — and `cs delta` gates every change after it. `scripts/codescene-gate.sh` carries both halves:

| Invocation | What it checks | Where it runs |
| --- | --- | --- |
| `codescene-gate.sh` | every supported file scores 10.0 | `just gate`, `just codescene` |
| `--branch [BASE]` | delta against a base ref | pre-push hook, `just codescene-branch` |
| `--staged` | delta over staged changes | pre-commit hook, `just codescene-staged` |
| `--files P...` | the named paths | `just codescene-files`, ad hoc |

Delta is sufficient for the inductive step because it fails on both ways the floor can break: a file whose score dropped, and a **new** file born below 10.0 (verified against CLI 1.0.36 — a newly added file scoring 8.67 exits nonzero with no prior score to compare against). What delta cannot see is a file that no change touched.

### Where this is enforced: locally, not in CI

**Code Health is a maintainer-local invariant, not a GitHub-enforced required check.** There is no CodeScene job, no `CS_ACCESS_TOKEN` secret, and no CodeScene context in either ruleset. `just gate` runs the full sweep against the pinned CLI and is fail-closed without a token; the git hooks run deltas; the maintainer runs `just gate` before merging, and that run is the gate.

This is a real weakening of the merge rules and is chosen deliberately, because the alternative had no honest form. A required check needs a credential, and a forked pull request cannot have one — GitHub correctly withholds secrets from fork-triggered runs. That left exactly two behaviours, and both were worse than moving the gate:

- **Fail the job on forks.** Every external contribution fails a required check for a reason that says nothing about its code, and can never be merged from its own branch. Tenable while there are no external contributors, which is precisely the moment the cost is invisible.
- **Pass the job unmeasured on forks.** A required check that goes green without checking anything, on exactly the contributions nobody has reviewed yet. This one is not tenable at all.

Requiring every contributor to hold a CodeScene account was the third option, and it prices a paid third-party account into sending a patch.

So the enforcement moved to where the credential already is. What the repository gives up is stated plainly rather than softened: **nothing mechanical stops a merge that skipped the sweep.** The rulesets cannot require it, and a maintainer who forgets `just gate` has no backstop. Three things reduce that exposure without pretending to close it — the pre-push delta, which is the last automatic signal before a change leaves the machine; the hooks' refusal to ever print a false pass; and the CLI-bump rule below.

### The floor is re-established whenever the CLI moves

Losing the daily CI sweep loses the thing that kept the induction's base case current. CodeScene's rules evolve between CLI versions, so a bump can drop an untouched file below 10.0 — and a delta cannot see it, because no change touched that file.

[tools.toml](../../../tools.toml) therefore records `swept_at` beside the CodeScene CLI `version`, and `scripts/check-tool-pins.py` fails `just check` unless the two are equal. Bumping the pinned CLI means running a full `just codescene` sweep and moving `swept_at` in the same commit. That is the mechanism that keeps "every supported file scores 10.0" a measured fact rather than a memory of one.

The CLI stays pinned at 1.0.36, fetched by its build SHA (`5f703ce1f9c264701f32c795fa7104467f1e4ab4`) and verified against a sha256 recorded in tools.toml — upstream publishes no checksums, so the repository carries its own. `scripts/install-dev-tools.sh` reads all three fields at install time, so the pin, the URL and the checksum have exactly one copy between them.

### Formatting and lints, now contractual

`cargo fmt --check` exact and `cargo clippy --all-targets` with `-D warnings` were already the practice; this ADR makes them part of the contract rather than a habit.

### Coverage: hard thresholds plus a ratchet

- **cargo-llvm-cov 0.8.7 (pinned)** measures the workspace. Floors: ≥95% line and ≥90% branch globally, and 100% line for every file under `crates/dogtag/src/` — the semantic kernel. The kernel paths list in [coverage-baseline.toml](../../../coverage-baseline.toml) grows as kernel, domain, and configuration modules appear.
- **Branch coverage requires nightly instrumentation** (rust-lang/rust#79649 is still open; `-Z coverage-options` is unstable), so the coverage job — and only the coverage job — runs a pinned nightly (`nightly-2026-07-30`, recorded in coverage-baseline.toml). Everything else builds on the pinned stable 1.97.1.
- **Thresholds and the committed baseline ratchet** are enforced by `scripts/coverage-gate.sh` plus `scripts/coverage_check.py` from the machine-readable JSON export — never from eyeballed terminal output. Raising the baseline is routine; lowering it, or either threshold, requires an ADR. Re-baselining after a coverage-toolchain bump is routine but must be called out in the bump commit.
- **No coverage-exclusion attributes.** Code that is hard to cover gets restructured or tested, not annotated out of the denominator.

### MSRV, tested not declared

`rust-version = "1.85"` in `Cargo.toml`, and a dedicated CI job builds **and tests** the workspace with 1.85.0 — the floor of the declared range — alongside the pinned current toolchain. A declared-but-untested MSRV is a promise nobody checked.

### Documentation warnings are errors

`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` runs in CI and in `just check`.

### Unsafe code is forbidden, with a named exception process

`#![forbid(unsafe_code)]` in every crate root, plus `[workspace.lints.rust] unsafe_code = "forbid"` inherited by all three crates so the ban mechanically covers test targets too. The exception process is policy: any future unsafe requires its own ADR, an isolated module boundary, documented safety invariants, targeted tests, Miri in CI where applicable, and independent review — the forbid is lifted only for the isolated module, never workspace-wide.

### API compatibility: advisory now, blocking at first stable tag

cargo-semver-checks 0.49.0 is pinned and wired as `just semver` (`--package dogtag --baseline-rev v0.1.0-beta.0 --release-type patch`). It is deliberately **not** a blocking CI gate yet, for two reasons worth stating plainly: prerelease→prerelease comparisons classify as an actual Major bump, so every lint is skipped — a blocking gate today would be green-but-meaningless; and forcing `--release-type patch` as a gate mode would forbid legitimate beta API evolution. The activation trigger is precise: the first non-prerelease release tag becomes the baseline, and the CI job becomes required at that moment. Until then, advisory runs use the recipe.

### Test-depth gates that activate with the surfaces they test

- **Property tests are mandatory** for parsers, emitters, round trips, path resolution, and configuration provenance *as those surfaces land* — the first such surface arrives with M2's contract loading. There are no M1 property tests, deliberately: a property test over a version string would be ceremony, and this ADR records the triggers, not the ceremony.
- **Mutation testing is mandatory on the semantic kernel** before each prerelease once the kernel has substantive behavior — from the first M2 prerelease onward. The tool is chosen at activation (cargo-mutants is the likely candidate).
- **TypeScript gates** — formatting, linting, tests, and `tsc --noEmit` — become mandatory in the same commit that first introduces binding source (M6 or earlier). The scaffold stays source-free until then, consistent with the [TypeScript scaffold boundary ADR](2026-07-30-typescript-scaffold-boundary.md).

### Alternatives considered

- **`cs delta`-only gating, with no full scan at all.** Rejected, but narrowly: delta is the per-change gate precisely because it is cheap and catches both degradation and new sub-10 files. What it cannot do is notice a file nobody touched — so a delta-only repository silently inherits whatever the floor was on the day the tool last changed its rules. The full sweep in `just gate`, plus the `swept_at` rule that forces one on every CLI bump, is what makes the induction's base case a measured fact rather than a memory.
- **Full-scan-only gating, on every commit.** Rejected for cost: one network round trip per supported file, paid on every push, growing with the repository, to re-derive a property the previous run already established. The layering above buys the same guarantee for a fraction of the calls.
- **Grep-based or self-reported quality metrics.** Rejected: fakeable, and in an agent-maintained repository "fakeable" means "will eventually be faked", with no malice required.
- **MCP-only CodeScene enforcement.** Rejected: the CodeScene MCP server serves interactive review well, but it is not scriptable from a gate, produces no exit status a runner can read, and spends agent context on output the summary line replaces. Local enforcement means the pinned CLI through `scripts/codescene-gate.sh` — never the MCP.
- **Keeping the CI job and accepting the fork behaviour.** Rejected above, and it is the decision this record most expects to be revisited. If GitHub ever offers a way to run a credentialed check on fork content safely, or if CodeScene offers per-repository rather than per-user authentication, the required check should come back and this ADR should be superseded rather than quietly ignored.
- **cargo-tarpaulin for coverage.** Rejected: branch coverage is literally marked "NOT IMPLEMENTED" in its feature table as of 0.37.0, and the branch threshold is half the contract.
- **Waiting for stable branch coverage.** Rejected: rust-lang/rust#79649 has no timeline; the wait is unbounded and the interim would be line-only.
- **Line-only coverage.** Rejected: line coverage misses exactly the conditional paths the conformance harness and the future contract-validation code are full of; a 95% line number can hide entire untested branches.

## Consequences

- **Code Health can be skipped by a maintainer who forgets `just gate`**, and no rule prevents the merge. This is the cost of the move out of CI, and it is the one consequence in this record with no mechanical mitigation. The pre-push delta and the `swept_at` rule narrow it; neither closes it.
- A codescene.io outage no longer blocks merges — it blocks `just gate`, which is the maintainer's problem rather than the repository's. In exchange the repository now has **no secrets at all** (see the [workflow security ADR](2026-07-30-workflow-security-and-repository-rules.md)), and a forked pull request can run every required check.
- **Contributors need no CodeScene account.** The hooks decline loudly when no token is configured — a conspicuous `CODE HEALTH NOT MEASURED` notice, never a pass — so an external contributor gets a working hook set without a paid third-party account, and never a false green.
- The nightly coverage toolchain is a second toolchain to keep pinned and bumped, and coverage numbers may shift slightly at bump time — hence the rule that re-baselining lands in the bump commit, where the cause is visible.
- `scripts/coverage_check.py` adds a python3-stdlib dependency to the local gate — accepted over fragile bash TOML parsing; ubuntu runners and the dev machine both ship 3.11+.
- `just check` stays offline and deterministic; the full `just gate` is network-dependent and slower. CI remains authoritative. How those recipes are invoked, rendered, and held to the workflows they mirror is the [gate ergonomics ADR](2026-07-30-gate-ergonomics-and-the-command-ladder.md); the thresholds and rules recorded here are unchanged by it.
- 10.0-on-stock-rules is a hard line: some legitimate designs will take real refactoring effort to satisfy a stock rule that a threshold tweak would have waved through. Accepted — the first tweak is the end of the metric.
- The 100%-line kernel rule means no kernel module can land without its tests in the same change; kernel PRs get bigger, and that is the point.
