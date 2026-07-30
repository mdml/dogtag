# Workflow security and repository rules

- Status: accepted
- Date: 2026-07-30

## Context

The dependency graph is small and audited ([Supply chain and vulnerability policy](2026-07-30-supply-chain-and-vulnerability-policy.md)); the code has mechanical quality floors ([Code health and coverage gates](2026-07-30-code-health-and-coverage-gates.md)). Both of those defend the *contents* of the repository. Neither defends the machinery that runs over those contents: a workflow with an over-broad token, a checkout that leaves credentials on disk for a later step to reuse, or a branch anyone can force-push is a shorter path to a compromised release than any dependency in the lockfile. The tj-actions compromise (CVE-2025-30066) that motivated SHA pinning in the [dependency and pinning policy](2026-07-30-dependency-and-pinning-policy.md) worked exactly this way — through CI, not through a library.

This repository is solo-maintained and largely agent-driven, which changes what the threat model should optimize for. The realistic adversary is not a targeted attacker; it is the maintainer (or an agent acting as the maintainer) doing something fast and irreversible on a Friday, and a supply-chain compromise arriving through an action nobody re-reads. Rules that can be waived by the person most likely to need waiving them are decoration. So the posture below is enforced by a linter that fails the build and by rulesets with **no bypass actors** — including for the admin.

The workflow-hardening rules were previously conventions honored by whoever wrote the last workflow. This record makes them lint-enforced, and records the repository-level rulesets in a form that can be reviewed in a diff and re-applied from this file.

## Decision

### zizmor 1.28.0 lints every workflow

Static analysis of the GitHub Actions workflows runs in CI via `zizmorcore/zizmor-action`, pinned at `6fc4b006235f201fdab3722e17240ab420d580e5` (v0.6.1), configured with:

- `version: 1.28.0` — the analyzer itself pinned, not just the action wrapping it.
- `advanced-security: false` — **the setting that makes the job actually fail.** In its default SARIF mode the action always exits 0 and uploads findings to code scanning, so whether a finding blocks anything depends on separate code-scanning ruleset wiring. With `advanced-security: false`, zizmor's own exit codes propagate and a finding fails the job directly.
- `persona: regular` — the default persona: real findings, not the auditor persona's exhaustive "everything that could theoretically matter" output.
- `min-severity: low` — on a three-workflow repository there is no volume argument for filtering, and the low-severity findings are exactly the pin-hygiene ones this record cares about.
- `online-audits: false`.

The job is `Workflow security (zizmor)` in [ci.yml](../../.github/workflows/ci.yml). Locally `just zizmor` runs the same analyzer version with flags mirroring those inputs, so a workflow edit can be checked before it reaches CI.

**The `online-audits: false` trade is real and worth stating.** Disabling them makes every verdict deterministic and independent of live GitHub API state — the same commit lints the same way today, in a rerun next month, and on a machine with no token. The cost is three audits that are online-only and genuinely useful: impostor-commit detection (a SHA that is not reachable from the ref it claims), known-vulnerable-actions, and typosquat detection on `uses:` values. Those are precisely the audits that would catch a tj-actions-shaped attack, and they are off. The mitigations are the SHA pins themselves plus Dependabot; the reason to accept the gap is that a security gate whose verdict depends on unauthenticated GitHub API availability is a gate that fails open or fails randomly, and neither is acceptable for a required check.

A second, more concrete reason: `ref-version-mismatch` runs online in the regular persona and evaluates whether a pin's comment agrees with the ref it names. `dtolnay/rust-toolchain` is pinned to a **master-branch SHA** — the action publishes no release tags to pin against — and is commented `# master 2026-07-16` rather than with a version. Any comment that named a version there would be a mismatch by construction. This is the one place the repository's pin-comment convention bends, it is documented here rather than in a lint suppression, and it is the first thing to re-evaluate if the online audits are ever turned on.

### Workflow posture, now lint-enforced rather than remembered

The rules below applied before this record; the difference is that they are now checked mechanically on every change rather than held in whoever's head wrote the workflow.

- **Every third-party action pinned to a full commit SHA** with a comment naming what the SHA corresponds to — normally `# vX.Y.Z`, with the `dtolnay/rust-toolchain` branch-and-date exception noted above. This is the [dependency and pinning policy](2026-07-30-dependency-and-pinning-policy.md) rule, now enforced instead of reviewed.
- **`persist-credentials: false` on every checkout.** The default leaves a credential in `.git/config` for the rest of the job, reachable by every subsequent step including third-party ones. Nothing in this repository needs it — the release job authenticates `gh` explicitly.
- **Workflow-level `permissions: {}`**, with job-level elevation only where a job genuinely needs it. All three workflows — [ci.yml](../../.github/workflows/ci.yml), [security.yml](../../.github/workflows/security.yml), [release.yml](../../.github/workflows/release.yml) — declare `permissions: {}` at the top, so a job that forgets to declare its own gets nothing rather than inheriting something. Every CI job then takes `contents: read`; the OSV jobs add `security-events: write` for SARIF upload; only the release job holds `contents: write`, `id-token: write`, and `attestations: write`.
- **No secrets and no write tokens reachable from forked pull requests.** Every PR trigger is `pull_request`, never `pull_request_target`; there is no workflow in which fork-controlled content executes with a token that can write anything.
- **Release execution only from `v*` tags in the canonical repository.** [release.yml](../../.github/workflows/release.yml) has no `pull_request` and no `workflow_dispatch` trigger, so there is no path to its `contents: write` token other than pushing a matching tag, and its jobs are guarded by `if: github.repository == 'mdml/dogtag'`. The guard protects nothing about *this* repository's token — a fork's run uses the fork's own token — but it means no fork can mint archives that look like official dogtag releases by tagging its copy. Given that the artifacts are the thing users pipe into a shell, the asymmetry favors the guard.

### Branch ruleset on `main`

**Status: decided here, applied separately.** Rulesets live in repository settings, not in the tree, so nothing in this change enables them — a token with repository-administration scope has to POST them. Until that happens the rules below describe intent, not enforcement, and the surrounding gates are only as binding as the discipline of whoever pushes. The payload is recorded here so it is reviewable in a diff, re-appliable after an accident, and honest about which half of the contract is code and which half is configuration:

```json
{
  "name": "main",
  "target": "branch",
  "enforcement": "active",
  "bypass_actors": [],
  "conditions": { "ref_name": { "include": ["refs/heads/main"], "exclude": [] } },
  "rules": [
    {
      "type": "pull_request",
      "parameters": {
        "required_approving_review_count": 0,
        "dismiss_stale_reviews_on_push": false,
        "require_code_owner_review": false,
        "require_last_push_approval": false,
        "required_review_thread_resolution": false,
        "allowed_merge_methods": ["rebase"]
      }
    },
    {
      "type": "required_status_checks",
      "parameters": {
        "strict_required_status_checks_policy": true,
        "do_not_enforce_on_create": false,
        "required_status_checks": [
          { "context": "Format, lint, test (Linux)" },
          { "context": "Commit messages" },
          { "context": "Test (macOS arm64)" },
          { "context": "MSRV (Rust 1.85)" },
          { "context": "Coverage thresholds" },
          { "context": "CodeScene Code Health" },
          { "context": "Workflow security (zizmor)" },
          { "context": "Release build check (x86_64 musl)" },
          { "context": "Markdown link integrity" },
          { "context": "cargo-deny" },
          { "context": "osv-pr / osv-scan" }
        ]
      }
    },
    { "type": "required_linear_history" },
    { "type": "non_fast_forward" },
    { "type": "deletion" }
  ]
}
```

Four things in that payload are decisions rather than defaults:

- **`required_approving_review_count: 0`.** A solo maintainer cannot approve their own pull request, so any positive number makes the repository unmergeable. The gate here is deliberately the required checks, not a second human — there is no second human. The pull-request rule still earns its place: it forces every change through a ref that CI runs against before it can reach `main`.
- **`require_last_push_approval: false`** for the same reason. With it on and zero required reviewers, the last pusher's own commit would need an approval that only the last pusher could give.
- **`strict_required_status_checks_policy: true`** — branches must be up to date with `main` before merging. On a low-traffic repository the re-run cost is small, and it closes the semantic-conflict gap where two independently-green PRs break `main` together.
- **`bypass_actors: []`.** The rules bind the admin. An emergency requires visibly disabling the ruleset, which is a deliberate, attributable act — rulesets carry audit-history endpoints that record exactly that.
- **`allowed_merge_methods: ["rebase"]`.** Two reasons, and the second is the one that matters.

  The small one: the repository allows merge commits, which `required_linear_history` would reject at merge time — the button is offered, the merge fails, and the reason is a rule two screens away. Constraining the methods in the rule that requires the pull request keeps the two consistent.

  The load-bearing one: **squash merging would put commits on `main` that no gate ever validated.** The `Commit messages` job checks the commits a pull request *contains*. A squash merge discards those and synthesizes a new one whose subject GitHub derives from the pull-request title, defaulting to `<PR title> (#N)` — a message the validator never saw, landing directly on `main`, from which the release notes are then generated. The invariant "every commit on `main` is a Conventional Commit" would hold for everything except the only commits that actually get there.

  Rebase-only closes it by construction: the exact commits CI validated are the commits that become `main` history. Nothing is generated, so nothing escapes.

  Retaining squash was considered and rejected as strictly worse. Making it safe needs *two* mechanisms that must agree — a PR-title validator, plus a GitHub `commit_message_pattern` metadata rule to catch a title edited after the check ran — and that pairing is only equivalent to the rebase case if GitHub's squash-subject template is never customized, if the repository never enables the "default to commit title for single-commit PRs" setting (which silently changes where the subject comes from), and if a maintainer never overrides the subject in the merge dialog, which is a free-text box the metadata rule is the sole defense against. Worse, `commit_message_pattern` is an organization-Enterprise rule and is unavailable on this personal repository, so on this repo the pairing cannot be assembled at all: the PR-title check alone leaves the merge-dialog override unguarded. Rebase-only needs no second mechanism and has no such matrix.

  The cost, recorded plainly: a messy branch cannot be tidied at merge time. Local history has to be worth landing before the merge button is pressed, which is more work for the author and is the intended direction of the pressure — the commits are the release notes.

**Eleven contexts** are required on a pull request: nine from [ci.yml](../../.github/workflows/ci.yml) (which include the code-quality gates from the [code health ADR](2026-07-30-code-health-and-coverage-gates.md)), plus `cargo-deny` and the pull-request OSV scan from [security.yml](../../.github/workflows/security.yml). A job that declares a `name` is required under that name; a job that does not is required under its id. The OSV jobs call reusable workflows, whose contexts render as `caller-job-id / callee-job-name`: both upstream workflows define a single job with the id `osv-scan` and no display name (read from the pinned commit `9a49870`, not guessed), so the caller ids `osv-pr` and `osv-full` produce `osv-pr / osv-scan` and `osv-full / osv-scan`.

**These strings are derived, not observed.** They come from reading the workflow definitions and GitHub's documented rendering rules; no pull request has run on this branch yet, so none of them has been seen in a real check run. Applying the ruleset before that happens risks a permanently-blocking rule from a single wrong string — a required context that never reports cannot be satisfied. The provisioning order below therefore pushes the branch first, reads the rendered names off the actual run, and only then creates the rulesets.

Only `osv-pr / osv-scan` is required here. `osv-full / osv-scan` is deliberately absent: it is conditioned on push, schedule, and manual dispatch, so it never runs on a pull request. This is the general rule for the list — **a required context must be one that always runs on a pull request** — and it is the trap to remember when adding jobs. The list is otherwise open: it grows with the jobs, and the [code health ADR](2026-07-30-code-health-and-coverage-gates.md) already names two that activate later (cargo-semver-checks at the first stable tag, the TypeScript gates when the binding gains source).

The contexts are also fragile in one specific way worth naming: they are display names, so **renaming a CI job silently un-requires it** until the ruleset is updated. The names are chosen to be stable, and a job rename is a ruleset change in the same commit.

### Tag ruleset on `refs/tags/v*`

```json
{
  "name": "release-tags",
  "target": "tag",
  "enforcement": "active",
  "bypass_actors": [],
  "conditions": { "ref_name": { "include": ["refs/tags/v*"], "exclude": [] } },
  "rules": [
    { "type": "update" },
    { "type": "deletion" },
    { "type": "non_fast_forward" },
    {
      "type": "required_status_checks",
      "parameters": {
        "strict_required_status_checks_policy": false,
        "do_not_enforce_on_create": false,
        "required_status_checks": [
          { "context": "Format, lint, test (Linux)" },
          { "context": "Commit messages" },
          { "context": "Test (macOS arm64)" },
          { "context": "MSRV (Rust 1.85)" },
          { "context": "Coverage thresholds" },
          { "context": "CodeScene Code Health" },
          { "context": "Workflow security (zizmor)" },
          { "context": "Release build check (x86_64 musl)" },
          { "context": "Markdown link integrity" },
          { "context": "cargo-deny" },
          { "context": "osv-full / osv-scan" }
        ]
      }
    }
  ]
}
```

`update`, `deletion`, and `non_fast_forward` with no bypass actors make a published tag immutable **for everyone, including the admin** — the mechanical half of the immutable-release policy recorded in the [supply chain ADR](2026-07-30-supply-chain-and-vulnerability-policy.md). Creating a new `v*` tag stays allowed; moving or deleting one does not. The ruleset pattern (`v*`) is deliberately broader than the release workflow's trigger (`v[0-9]*`), so a mistyped tag is still immutable.

The `required_status_checks` rule is the interesting one: with `do_not_enforce_on_create: false`, a `v*` tag can only be created on a commit whose required checks already passed. That is the mechanical form of "tags come only from green commits", and it works because CI runs on push to `main`, so every `main` commit carries the check contexts by the time anyone tags it.

**Eleven contexts** again, and the tag list differs from the branch list in exactly one place — not cosmetically: it requires `osv-full / osv-scan` where the branch ruleset requires `osv-pr / osv-scan`. The contexts a tag rule can demand are those present on the *tagged commit*, and that commit reached `main` through a push — which runs the full recursive vulnerability scan, not the pull-request diff scan. The same asymmetry works in this repository's favor for Code Health: the push-to-`main` run scores every file rather than the changed ones, so a release tag is gated on a full measurement of both properties rather than an incremental one.

**Honest caveat: this half is not empirically confirmed.** Status-checks-gating-tag-creation is documented-implied rather than documented — GitHub's ruleset documentation uses "branch or tag" wording throughout, and the existence of a `do_not_enforce_on_create` parameter only makes sense if creation is otherwise enforced — but it was not tested, because the maintainer's current token lacks the repository-administration scope needed to create a test ruleset. If the API rejects a `required_status_checks` rule on a tag target, the fallback is a creation-restriction rule limiting who may create `v*` tags. The immutability rules stand either way; only the green-commit gate is contingent.

### Immutable releases

The tag ruleset protects the *ref*; GitHub's **immutable releases** setting protects the *assets*, which is the half a tag rule cannot reach — a published release's uploaded archives can otherwise be deleted or replaced while the tag stays exactly where it was. Both are needed, and the setting is not a ruleset, so it is enabled separately:

```bash
gh api -X PUT -H 'X-GitHub-Api-Version: 2026-03-10' repos/mdml/dogtag/immutable-releases
```

**It protects only releases published after it is enabled.** The already-published `v0.1.0-beta.0` release does not become immutable retroactively: its assets stay mutable and its tag stays deletable for as long as it exists. That is worth stating rather than assuming, because the natural reading of "immutable releases are on" is that the release list is now immutable, and for the one release this repository has already shipped, it is not. Nothing here can fix that release retroactively — the guarantee begins with the next one, and the honest description of the current state is "every release from the next one onward".

The corollary for anyone verifying `v0.1.0-beta.0`: its `.sha256` sidecars and the aggregate `sha256.sum` are the integrity evidence for that release, not the setting.

### Provisioning order

The order matters, because two of these steps depend on facts that do not exist until the branch has run:

1. **Push the branch and open a pull request.** No admin rights needed. This is what produces the first real check run.
2. **Read the rendered context names off that run** — `gh pr checks <n> --json name` — and reconcile them against the two payloads above. The names there are derived from the workflow definitions and GitHub's documented rendering, not observed; a single wrong string becomes a required check that can never report, which blocks every future merge and can only be undone by editing the ruleset. Reconciling first costs one command.
3. **Add the secrets.** `CS_ACCESS_TOKEN` twice — once for Actions, once for Dependabot, which receives no Actions secrets and would otherwise be unable to pass the Code Health check on its own pin-freshness pull requests.
4. **Create the rulesets**, branch first, then tag. Staging with `"enforcement": "disabled"` and flipping to `active` after inspection is available for the tag ruleset in particular, whose `required_status_checks` rule on a tag target is documented-implied rather than confirmed.
5. **Enable immutable releases** (above). Independent of the rest; do it whenever.

Steps 3 through 5 need a token with repository-administration scope, which the maintainer's current PAT does not carry.

### Secrets

`CS_ACCESS_TOKEN` (CodeScene) is the repository's only secret: a read-scoped analysis token used by the code-health gate. It is never exposed to fork-controlled execution — the CodeScene job holds no write permissions, and no workflow here uses `pull_request_target`. One secret is a number worth keeping: the inventory of "what could leak" is currently short enough to hold in a sentence, and any addition should be argued against that.

### Alternatives considered

- **SARIF-mode zizmor with code-scanning merge protection.** The action's default: findings upload to code scanning, and blocking is configured separately as a code-scanning ruleset. Rejected as indirection — the enforcement lives in a place the repository does not version, the job goes green while findings accumulate, and a lint that reports but cannot fail is a lint people stop reading. `advanced-security: false` puts the failure where the change is.
- **Leaving branch protection to discipline.** Rejected: discipline is what fails at 11pm, and it fails invisibly. Every argument for the mechanical gates in the code-health ADR applies here with more force, because the failure mode is an unrecoverable ref rather than a bad score.
- **Adding an admin bypass actor "just in case".** Rejected, and this is the load-bearing rejection. For a solo repository, the admin's own mistakes are the primary threat the rules exist to catch; a ruleset the admin can bypass provides near-zero protection against that threat while providing full protection against an attacker who does not exist. The emergency case is handled by disabling the ruleset, which is slower, visible, and logged — all three of which are features.
- **Relying on review rather than required checks for tag hygiene.** Rejected: there is nobody to review, and "the maintainer will check that the commit was green before tagging" is exactly the class of invariant this record exists to replace with a mechanism.

## Consequences

- **The solo maintainer must open a pull request for every change**, including one-line documentation fixes, and wait for the full check suite — which includes the network-dependent CodeScene and scanner jobs. This is deliberate friction with a real, daily, cumulative cost, and it will be tempting to disable long before it is tempting to bypass.
- **No bypass means a genuine emergency costs a visible ruleset edit.** Disabling and re-enabling a ruleset is slower than a force-push and leaves a trail. That is the intent; it is still a bad half-hour when it happens.
- **Required-check contexts are referenced by rendered job name.** Renaming a CI job silently breaks the requirement — the ruleset keeps waiting for a context nothing will ever report, and the PR sits pending rather than failing loudly. This is a real foot-gun with no mechanical guard, and it is worse for the reusable-workflow jobs whose contexts are not readable from the caller's YAML at all. Job renames must include a ruleset update in the same change.
- **The tag-creation gate depends on CI having run on the exact tagged commit.** Tagging a commit whose checks never ran, or whose check runs have aged out of GitHub's retention, will be refused — including in the case where a release is being cut from an older commit on purpose.
- **A forked pull request cannot pass the Code Health gate at all.** Secrets are correctly withheld from fork-triggered `pull_request` runs, so the CodeScene CLI cannot authenticate and the job fails with an explanatory error. Passing it unmeasured was considered and rejected: a required check that goes green without checking anything is worse than no check, and it would make the one gate that guards maintainability advisory for exactly the contributions nobody has reviewed yet. The cost is that landing an external contribution takes a deliberate maintainer act — pushing the branch into this repository, where the gate runs for real — and that no fork PR can ever be merged from its own branch. For a beta with no external contributors that is free today, and it is honest rather than convenient when that changes.
- **`online-audits: false` leaves the impostor-commit, known-vulnerable-actions, and typosquat audits unrun.** The pins and Dependabot are the compensating controls, and neither of them detects a SHA that never belonged to the ref it claims.
