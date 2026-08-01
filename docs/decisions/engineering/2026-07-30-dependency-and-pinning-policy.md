# Dependency and pinning policy

- Status: accepted (amended 2026-07-31 — see [Amendments](#amendments))
- Date: 2026-07-30

## Context

The beta promises a small, auditable supply chain: a single static binary users pipe a shell script to install, built by automation the maintainers must be able to trust. Every dependency and every CI action is attack surface. At M1 the workspace needs almost nothing (the CLI needs an argument parser; the conformance harness needs TOML deserialization), which is the right moment to fix the policy rather than accrete one.

## Decision

- **Minimal dependency set.** M1 direct dependencies are exactly: `clap` (derive) in the CLI; `serde` + `toml` in the conformance harness; nothing in the SDK crate. No convenience crates (`anyhow`, `assert_cmd`, …) until a milestone actually needs them — integration tests use the standard library.
- **Pin via manifest + lockfile + `--locked`.** Manifests state the exact current version with cargo's default caret semantics (e.g. `clap = "4.6.4"`); `Cargo.lock` is committed; CI and release builds pass `--locked`. Together these give byte-for-byte dependency reproducibility while keeping the manifests conventional for the day the crates publish.
- **GitHub Actions pinned to full commit SHAs**, each with a `# vX.Y.Z` comment naming the release the SHA corresponds to. No floating tags. Workflows use only the built-in `GITHUB_TOKEN`, with least-privilege `permissions` blocks.
- **Toolchain pinned exactly** (`1.97.1` in `rust-toolchain.toml`), so local, CI, and release builds compile with the same compiler.
- **Update stream: Dependabot, human-merged.** `.github/dependabot.yml` enables Dependabot for the `github-actions` and `cargo` ecosystems on a weekly schedule. It only opens PRs; every bump lands through a human-reviewed PR, and nothing merges automatically — automation proposes, a maintainer disposes. Pins are also bumped deliberately outside that stream — at each milestone review, or immediately on a security advisory — in commits that touch only the pins. Dependabot is not an alert channel: the maintainers still watch advisories (RustSec, GitHub advisories) directly, because a weekly PR cadence is too slow for an active exploit.

### Alternatives considered

- **Exact `=` requirements in the manifests.** Rejected: redundant with the committed lockfile plus `--locked`, and hostile to downstream resolution once the crates publish — `=` pins in a published library force unification conflicts on consumers.
- **Floating action tags (`actions/checkout@v4`).** Rejected: tags are mutable; a moved tag is the standard mechanism of CI supply-chain compromise. The `# vX.Y.Z` comment keeps SHA pins human-readable.
- **Not committing `Cargo.lock`** (the old library convention). Rejected: the workspace ships a binary and a release pipeline whose reproducibility is the point; current cargo guidance favors committing lockfiles anyway.
- **No auto-update bots.** Rejected: pins rot. SHA-pinned actions and exact dependency versions go silently stale, and milestone-cadence sweeps alone would leave known fixes unapplied for weeks at a time. Weekly Dependabot PRs surface the drift while human review keeps unreviewed automated bumps — themselves a supply-chain vector — out of the tree; the small dependency set keeps the PR volume from drowning a solo-maintained beta.

## Consequences

- Reproducible builds: same compiler, same dependency graph, same action code on every run.
- Update latency is the accepted cost — fixes land only when a human reviews and merges the weekly Dependabot PRs or makes an advisory-triggered bump, never automatically. The advisory-triggered exception is the fast path, and it depends on the maintainers actually watching advisories (RustSec, GitHub advisories) rather than waiting on the bot.
- SHA-pinned actions are unreadable without their comments; keeping comment and SHA in sync is a review obligation.
- Adding any new dependency is a visible, reviewable act — which is exactly the friction intended.

## Amendments

The Decision above stands as written; these later records change parts of it, and the original text is left intact so the change is legible.

- **2026-07-30 — workflows no longer use only `GITHUB_TOKEN`.** The Decision's least-privilege bullet says workflows use only the built-in token. The Code Health gate introduced by [code health and coverage gates](2026-07-30-code-health-and-coverage-gates.md) needs `CS_ACCESS_TOKEN`, a read-scoped CodeScene analysis token and the repository's first and only secret. The least-privilege intent is unchanged — [workflow security and repository rules](2026-07-30-workflow-security-and-repository-rules.md) records how the secret is kept away from fork-controlled execution.
- **2026-07-30 — one action pin does not carry a `# vX.Y.Z` comment.** `dtolnay/rust-toolchain` is pinned to a commit on its rolling `master` branch, which publishes no version tags, so its comment names the branch and date (`# master 2026-07-16`) instead. The rule is otherwise unchanged, and is now mechanically enforced by zizmor rather than by review.
- **2026-07-31 — the SDK crate is no longer dependency-free, and the conformance harness depends on the SDK.** The Decision's minimal-set bullet fixes the M1 direct dependencies as `clap` in the CLI, `serde` + `toml` in the conformance harness, and **nothing in the SDK crate**. M2 changes that, because M2 is the milestone that reads files. `crates/dogtag` gains **`toml`**: both assets [the contract record](2026-07-31-vault-contract-and-installation-record.md) fixes are TOML, and the parser needs spans, so input parsing deliberately does not use `#[derive(Deserialize)]` — it walks `toml::de::DeTable` by hand so that every key *and* value carries a span, so that a malformed asset yields every diagnostic rather than the first, and so that key legality can be scoped to the version the asset declares. It gains **`serde_json`**, because the structured output of both M2 commands is an SDK-owned rendering ([the surfaces record](2026-07-31-m2-surfaces-and-the-sdk-boundary.md)) and hand-rolled JSON escaping inside the semantic kernel would be a correctness liability bought for nothing. It gains **`serde`** with its derive, for those output types only. The conformance harness gains **`dogtag`**, because M2 graduates its scenarios to executable and they run against the SDK's public API.

  The cost is named rather than absorbed. `serde` and `toml` were already workspace dependencies but were **not in the shipped binary's closure**, because the SDK's `[dependencies]` was empty and everything a user installs arrived through the CLI. This change moves TOML and JSON into that closure, which is exactly the condition [the supply-chain policy](2026-07-30-supply-chain-and-vulnerability-policy.md) set as its SBOM trigger; [the M2 release record](2026-07-31-m2-release-and-cutover.md) executes it. Neither is restated here. The pinning half of the Decision is unchanged: the new requirements state their exact current versions in the manifests, `Cargo.lock` is committed, and `--locked` still governs CI and release builds.
