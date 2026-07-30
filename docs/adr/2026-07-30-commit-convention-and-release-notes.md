# Commit convention and release notes

- Status: accepted
- Date: 2026-07-30

## Context

The repository has declared Conventional Commits since its first commit, and its history has followed the convention — but nothing checked it. A convention that lives only in `AGENTS.md` holds exactly as long as everyone reading it remembers, which in a repository maintained largely by agents is not a property to rely on. The cost of drift is not aesthetic: release notes, and eventually version selection, are supposed to be *derived* from the history, and a derivation is only as trustworthy as the data it reads.

The related weakness is on the way out. `gh release create --generate-notes` produces a flat list of pull-request titles, which discards the structure the commit format exists to create — a reader gets "Update things (#4)" where the history actually knows the change was a breaking API rename.

## Decision

### A Rust-native validator, in this workspace

`tools/commit-lint` is a small crate — standard library only, per the [dependency policy](2026-07-30-dependency-and-pinning-policy.md) — implementing the subset of [Conventional Commits 1.0.0](https://www.conventionalcommits.org) this repository uses: the closed type list `feat, fix, docs, test, refactor, perf, build, ci, chore, revert`, optional scopes, and both breaking-change forms (the `!` marker and the `BREAKING CHANGE:` footer). It has two modes, matching the two moments the rule is enforced:

```text
commit-lint <path>            validate one message file (the commit-msg hook)
commit-lint --range <RANGE>   validate every authored commit in a git range (CI)
```

It deliberately implements **nothing the specification does not mandate**. No subject casing rule, no trailing-punctuation rule, no line-length rule. A validator that rejects a legitimate message is how contributors learn to reach for `--no-verify`, and the marginal value of enforcing sentence case does not survive that trade.

Merge commits are skipped: a merge has no authored subject of its own to hold to a standard. `fixup!` and `squash!` commits are rejected, since they are by construction not meant to land.

Placing it in `tools/` rather than `crates/` follows the layout ADR's "code by role" principle — `crates/` is the product, `conformance/` is the harness, and repository tooling is a third role. It is `publish = false` and outside the release build, so it ships to nobody, but it is a full workspace member and therefore held to every gate in the [code health and coverage ADR](2026-07-30-code-health-and-coverage-gates.md): Code Health 10.0, the coverage floors, `forbid(unsafe_code)`, and the MSRV.

### Two enforcement points, only one of which is binding

- **Locally**, [lefthook.yml](../../lefthook.yml) runs the validator as a `commit-msg` hook, alongside format, clippy, and a staged Code Health delta. Installed with `just hooks`.
- **In CI**, the `Commit messages` job validates every commit the pull request introduces, as a required check.

The CI check is the enforcement boundary and the hook is a convenience — a distinction worth stating because it is easy to get backwards. Hooks are not installed on a fresh clone, and `--no-verify` always exists; a rule enforced only by a hook is a rule enforced only against people who were not in a hurry. Both run the same binary built from this workspace, so the local and CI verdicts cannot drift apart.

There is a third thing the pair does not cover on its own, and it decides a repository setting: **the validator checks the commits a pull request contains, so any merge strategy that synthesizes a new commit puts an unvalidated message on `main`.** Squash merging does exactly that, deriving a subject from the pull-request title — which the validator never sees, and from which the release notes are then generated. The [workflow security ADR](2026-07-30-workflow-security-and-repository-rules.md) therefore restricts merges to rebase, so the commits CI validated are the commits that land. Recorded here as well because the reason lives in this decision, not that one: without it, the invariant would hold for every commit except the ones that actually reach `main`.

The CI job derives its range from `github.event.pull_request.base.sha` on a pull request, falling back to `github.event.before` on a push and to the newest commit otherwise. The pull-request case is the one that matters; the others exist so the job is never silently vacuous.

### git-cliff for deterministic release notes

[cliff.toml](../../cliff.toml) (git-cliff 2.13.1, pinned in [tools.toml](../../tools.toml)) generates the draft release body at release time from the tag range. Grouping: breaking changes first, then features, bug fixes, performance, documentation, refactoring, and reverts. `test`, `ci`, `build`, and `chore` are excluded — real work, but not news to someone deciding whether to install a version.

Two properties are load-bearing. **Grouping is by declared type only**: a body-text pattern (matching, say, "security" anywhere in a message) was rejected because it silently reclassifies commits on a word, and a reader cannot tell why something landed where it did. **Unconventional commits are dropped rather than guessed at**, which is safe only because `commit-lint` makes them unmergeable — the two settings are a pair, and weakening the validator would silently start losing commits from the notes.

Breaking changes appear twice: once under their own heading with the `BREAKING CHANGE:` description, and again under their type. The breaking section says what breaks; the type section says what the change was.

**No committed `CHANGELOG.md`.** The notes are built from the tag range at release time. A committed changelog means every release mutates a file describing history, which is a different decision — about whether the repository carries a second, hand-maintainable copy of what git already knows — and it has not been made.

The draft-only release path is untouched: the workflow still creates a `--draft --prerelease` release, and publishing remains a human act.

### Alternatives considered

- **commitlint (Node).** Rejected: it would introduce Node tooling, an `npm`/`bun` dependency tree, and a second package ecosystem into a repository whose entire supply-chain posture is built around a small, auditable Rust dependency set — all to check a string against a grammar that is 200 lines of Rust with no dependencies. The [Bun quarantine](2026-07-30-supply-chain-and-vulnerability-policy.md) exists precisely because adding a JavaScript dependency tree is a decision with consequences.
- **A shell script with a regex.** Rejected: it would be shorter, but it is untestable at the level the rest of the repository is tested, it has no natural home for the error messages that make a rejection actionable, and it would not be covered by the Code Health or coverage gates.
- **Enforcing the convention only in the hook.** Rejected: see above — that is enforcement against the conscientious.
- **Enforcing only in CI, no hook.** Rejected on feedback latency: discovering a malformed message after a push means an interactive rebase rather than an amend.
- **Keeping `--generate-notes`.** Rejected: it discards the structure the commit convention exists to produce.
- **Committing a `CHANGELOG.md` now.** Deferred rather than rejected — see above.
- **`cocogitto` or `convco`** (Rust-native, would cover both validation and changelog). Rejected for the validator: adopting a tool to enforce a closed 10-type list means inheriting its configuration surface and its opinions about versioning, where the in-repo validator is auditable in one file. git-cliff was still chosen for notes because templating release notes well is genuinely more work than parsing a subject line.

## Consequences

- The commit convention is now mechanically binding, and a malformed message blocks a merge. Rewriting a pushed branch's history to fix a subject is a real cost, paid by whoever gets it wrong.
- Git's default revert message (`Revert "..."`) does not conform, so reverts must be re-worded as `revert: ...`. This is the one place the validator routinely fights a git default.
- The release notes are only as good as the commit subjects, which moves editorial weight onto the moment of committing. That is the intended trade — the subject is written when the context is freshest — but it means a careless subject is now visible to users rather than only to `git log`.
- `tools/commit-lint` is a fourth workspace member: more to build in every CI job, and its coverage counts toward the global floor.
- **The coverage baseline's branch figure was lowered**, from 98.07% to 97.72%, which the [code health and coverage ADR](2026-07-30-code-health-and-coverage-gates.md) permits only with a record — this is that record. Adding a crate enlarges the denominator: over the same change line coverage *rose* (98.90% → 99.02%), and the two branches now uncovered are in the new crate, not newly abandoned in the old one. Both figures remain far above the thresholds (95% line, 90% branch), which did not move and are the actual contract. The ratchet caught this automatically, which is the system working — the number was re-derived from a measured run, never hand-adjusted to make a build pass.
- git-cliff and lefthook are two more pinned tools to track and bump. lefthook is local-only and never runs in CI, so it stays out of the CI trust base; git-cliff runs in the release job, where it can influence only the notes text and not the artifacts.
- Notes are generated at release time rather than stored, so regenerating them for an old tag depends on `cliff.toml` at the commit being read. Since published releases are immutable, the note text is fixed once the draft is published regardless.
