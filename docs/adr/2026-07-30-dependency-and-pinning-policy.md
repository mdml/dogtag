# Dependency and pinning policy

- Status: accepted
- Date: 2026-07-30

## Context

The beta promises a small, auditable supply chain: a single static binary users pipe a shell script to install, built by automation the maintainers must be able to trust. Every dependency and every CI action is attack surface. At M1 the workspace needs almost nothing (the CLI needs an argument parser; the conformance harness needs TOML deserialization), which is the right moment to fix the policy rather than accrete one.

## Decision

- **Minimal dependency set.** M1 direct dependencies are exactly: `clap` (derive) in the CLI; `serde` + `toml` in the conformance harness; nothing in the SDK crate. No convenience crates (`anyhow`, `assert_cmd`, …) until a milestone actually needs them — integration tests use the standard library.
- **Pin via manifest + lockfile + `--locked`.** Manifests state the exact current version with cargo's default caret semantics (e.g. `clap = "4.6.4"`); `Cargo.lock` is committed; CI and release builds pass `--locked`. Together these give byte-for-byte dependency reproducibility while keeping the manifests conventional for the day the crates publish.
- **GitHub Actions pinned to full commit SHAs**, each with a `# vX.Y.Z` comment naming the release the SHA corresponds to. No floating tags. Workflows use only the built-in `GITHUB_TOKEN`, with least-privilege `permissions` blocks.
- **Toolchain pinned exactly** (`1.97.1` in `rust-toolchain.toml`), so local, CI, and release builds compile with the same compiler.
- **Review cadence.** Pins are bumped deliberately — reviewed at each milestone, or immediately on a security advisory — in commits that touch only the pins. No auto-update bots at this stage; the update stream would drown a solo-maintained beta in noise, and unreviewed automated bumps are themselves a supply-chain vector.

### Alternatives considered

- **Exact `=` requirements in the manifests.** Rejected: redundant with the committed lockfile plus `--locked`, and hostile to downstream resolution once the crates publish — `=` pins in a published library force unification conflicts on consumers.
- **Floating action tags (`actions/checkout@v4`).** Rejected: tags are mutable; a moved tag is the standard mechanism of CI supply-chain compromise. The `# vX.Y.Z` comment keeps SHA pins human-readable.
- **Not committing `Cargo.lock`** (the old library convention). Rejected: the workspace ships a binary and a release pipeline whose reproducibility is the point; current cargo guidance favors committing lockfiles anyway.
- **Dependabot/Renovate from day one.** Deferred, not refused: once the dependency tree or contributor count grows, automated update PRs with human review may beat milestone-cadence sweeps. Revisit when either happens.

## Consequences

- Reproducible builds: same compiler, same dependency graph, same action code on every run.
- Staleness is the accepted cost — security fixes arrive on the review cadence, not automatically. The advisory-triggered exception in the cadence is the mitigation, and it depends on the maintainers actually watching advisories (RustSec, GitHub advisories).
- SHA-pinned actions are unreadable without their comments; keeping comment and SHA in sync is a review obligation.
- Adding any new dependency is a visible, reviewable act — which is exactly the friction intended.
