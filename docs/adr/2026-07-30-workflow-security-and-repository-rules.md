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

**Ten contexts** are required on a pull request: eight from [ci.yml](../../.github/workflows/ci.yml) (which include the coverage and MSRV gates from the [code health ADR](2026-07-30-code-health-and-coverage-gates.md)), plus `cargo-deny` and the pull-request OSV scan from [security.yml](../../.github/workflows/security.yml). Code Health is deliberately not among them — see [the code health ADR](2026-07-30-code-health-and-coverage-gates.md) for why it is enforced locally instead. A job that declares a `name` is required under that name; a job that does not is required under its id. The OSV jobs call reusable workflows, whose contexts render as `caller-job-id / callee-job-name`: both upstream workflows define a single job with the id `osv-scan` and no display name (read from the pinned commit `9a49870`, not guessed), so the caller ids `osv-pr` and `osv-full` produce `osv-pr / osv-scan` and `osv-full / osv-scan`.

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

**Ten contexts** again, and the tag list differs from the branch list in exactly one place — not cosmetically: it requires `osv-full / osv-scan` where the branch ruleset requires `osv-pr / osv-scan`. The contexts a tag rule can demand are those present on the *tagged commit*, and that commit reached `main` through a push — which runs the full recursive vulnerability scan, not the pull-request diff scan. The same asymmetry works in this repository's favor for Code Health: the push-to-`main` run scores every file rather than the changed ones, so a release tag is gated on a full measurement of both properties rather than an incremental one.

**Honest caveat: this half is not empirically confirmed.** Status-checks-gating-tag-creation is documented-implied rather than documented — GitHub's ruleset documentation uses "branch or tag" wording throughout, and the existence of a `do_not_enforce_on_create` parameter only makes sense if creation is otherwise enforced — but it was not tested, because the maintainer's current token lacks the repository-administration scope needed to create a test ruleset. If the API rejects a `required_status_checks` rule on a tag target, the fallback is a creation-restriction rule limiting who may create `v*` tags. The immutability rules stand either way; only the green-commit gate is contingent.

### Immutable releases

The two mechanisms cover different objects, and the difference decides what `v0.1.0-beta.0` does and does not get. The tag ruleset protects the *ref*; GitHub's **immutable releases** setting protects the *assets* — a published release's uploaded archives can otherwise be deleted or replaced while the tag stays exactly where it was. Both are wanted, and the setting is not a ruleset, so it is enabled separately:

```bash
gh api -X PUT -H 'X-GitHub-Api-Version: 2026-03-10' repos/mdml/dogtag/immutable-releases
```

**The asset guarantee is not retroactive; the tag guarantee is.** Immutable releases applies to releases published after it is enabled, so `v0.1.0-beta.0`'s uploaded archives remain replaceable by anyone with write access. Its *tag*, though, is a ref like any other: the `refs/tags/v*` ruleset matches it the moment the ruleset is created, and its `update`, `deletion`, and `non_fast_forward` rules then bind that existing tag exactly as they bind future ones. So after provisioning, the already-published release is in a mixed state — the commit it points at is pinned, the bytes hanging off it are not — and saying "immutable releases are on" would describe only the half that is not true of it.

What actually protects the existing release's bytes is the attestation, and the distinction matters because the two artifacts look similar and are not:

- The `.sha256` sidecars and the aggregate `sha256.sum` are stored **beside the assets, on the same release, with the same write permissions**. They detect ordinary corruption — a truncated download, a bit flip, a CDN serving something stale — which is what `install.sh` uses them for. They are not evidence against deliberate replacement: whoever can replace an asset can replace its sidecar in the same breath, and the pair will agree.
- The **Sigstore-backed build provenance** is recorded independently of the release: in Sigstore's transparency log and GitHub's attestation store, signed through the workflow's OIDC identity rather than by anyone holding repository credentials. It binds the artifact's digest to the workflow, the commit, and the ref that produced it. Replacing an asset does not produce a matching attestation, and one cannot be minted after the fact without that workflow identity — which is precisely the property a mutable sidecar lacks.

Verified for the published release on 2026-07-30: `gh attestation verify dogtag-x86_64-unknown-linux-musl.tar.gz --repo mdml/dogtag` succeeds against `https://github.com/mdml/dogtag/.github/workflows/release.yml@refs/tags/v0.1.0-beta.0`, source digest `55985cb`, issuer `token.actions.githubusercontent.com`. That command — not the sidecar — is the answer to "is this the artifact the release workflow built".

### Provisioning order

The order is load-bearing, and it is load-bearing twice. The rulesets depend on context names that do not exist until the branch has run, and that run is only authoritative if the secrets it needs were already in place — so the branch ruleset is created once the pull request is green and its ten names have been read off a real run, which is early enough that the first merge goes *through* the rule rather than around it. The *tag* ruleset then depends on a name no pull request can produce, so it cannot be created until a commit has reached `main` and its own run has finished. Each is read back afterwards rather than assumed from an exit status.

1. **No secrets to install.** The inventory is empty (below), so provisioning starts at the pull request. Code Health is enforced by the maintainer's local `just gate`, not by a required check, and installing a `CS_ACCESS_TOKEN` for Actions or Dependabot would be provisioning a credential nothing reads.

2. **Push the branch and open a pull request.** This produces the authoritative run. If any job did start before step 1 landed, rerun it rather than reasoning about which failures were spurious: `gh run rerun <run-id> --failed`.

3. **Require the pull request to be fully green**, every check, before going further. Two distinct things depend on it. The context names in the payloads are *derived* from the workflow definitions and GitHub's documented rendering rules, not observed, and a single wrong string becomes a required check that can never report — blocking every future merge until someone edits the ruleset. And a required check that has never passed once is a rule adopted on faith. Read the rendered names off the green run and reconcile them against the payloads:

   ```bash
   gh pr checks <n> --json name,state --jq '.[] | "\(.state)  \(.name)"'
   ```

   Expect ten passing contexts matching [.github/rulesets/main-branch.json](../../.github/rulesets/main-branch.json). Reconcile before creating anything; it costs one command and is the cheapest moment to find a typo.

   **Create the branch ruleset here**, once those names are reconciled and before the merge — everything it requires has now been observed, and applying it first means the very first merge goes through the rule rather than around it:

   ```bash
   gh api -X POST repos/mdml/dogtag/rulesets --input .github/rulesets/main-branch.json
   ```

   Only the *tag* ruleset has to wait, and step 4 explains why.

4. **Merge by rebase, and let every workflow `main` triggers finish.** The *tag* payload cannot be reconciled against a pull request at all: it requires `osv-full / osv-scan`, and that job is conditioned on push, schedule and dispatch, so its rendered name does not exist until a commit lands on `main`.

   Two workflows fire on that push — CI and Security — so watch the runs for the merged commit rather than the most recent one, and read the conclusions rather than assuming them:

   ```bash
   sha="$(git rev-parse main)"
   for id in $(gh run list --commit "$sha" --json databaseId --jq '.[].databaseId'); do
     gh run watch "$id"
   done
   gh api repos/mdml/dogtag/commits/"$sha"/check-runs \
     --jq '.check_runs[] | "\(.conclusion // "pending")  \(.name)"'
   ```

   Expect every check green and none pending, and expect the names to match [.github/rulesets/release-tags.json](../../.github/rulesets/release-tags.json) — ten again, differing from the branch list in exactly one place: `osv-full / osv-scan` where the pull request produced `osv-pr / osv-scan`. Reconcile that name here, for the same reason the branch names were reconciled in step 3.

5. **Create the tag ruleset**, staged disabled:

   ```bash
   jq '.enforcement = "disabled"' .github/rulesets/release-tags.json \
     | gh api -X POST repos/mdml/dogtag/rulesets --input -
   ```

   The tag ruleset is staged disabled deliberately: its `required_status_checks` rule on a tag target is documented-implied rather than confirmed (above). Inspect it, then activate it with the checked-in payload, which carries `"enforcement": "active"`:

   ```bash
   gh api -X PUT repos/mdml/dogtag/rulesets/<tag-ruleset-id> \
     --input .github/rulesets/release-tags.json
   ```

   Rollback for either is `gh api -X DELETE repos/mdml/dogtag/rulesets/<id>`, with `gh api repos/mdml/dogtag/rulesets` listing the ids.

6. **Enable immutable releases** (above). Independent of the rest.

7. **Read all three back**, because "the command exited 0" is not the same claim as "the rule is in force with the contents intended". GitHub can normalize a payload on the way in, and a ruleset created `disabled` looks identical to an active one in a list that omits enforcement.

   ```bash
   gh api repos/mdml/dogtag/rulesets --jq '.[] | "\(.id)  \(.name)  \(.target)  \(.enforcement)"'
   gh api repos/mdml/dogtag/rulesets/<id> --jq '{enforcement, bypass_actors,
     contexts: [.rules[] | select(.type == "required_status_checks")
                | .parameters.required_status_checks[].context]}'
   ```

   For each of the two rulesets, confirm `enforcement` is `active`, `bypass_actors` is empty, and the context list is the ten strings from its payload — the branch one ending `osv-pr / osv-scan`, the tag one ending `osv-full / osv-scan`. For immutable releases, `gh api repos/mdml/dogtag/immutable-releases` should succeed; **that read-back is documented-implied rather than confirmed**, like the setting's own endpoint, so if it does not answer `GET`, confirm it under Settings → General instead and record which method was used.

The payloads are checked in rather than pasted from a transcript so the thing reviewed is the thing posted; `scripts/check-ruleset-payloads.py` holds them to the copies quoted in this record, and to their context counts, on every run of `just check`. Steps 1, 5, 6 and 7 need a token with repository-administration scope, which the maintainer's current PAT does not carry.

### Secrets

**The repository has no secrets.** `grep -r 'secrets\.' .github/workflows` returns nothing, and `scripts/test_hooks.py` asserts it stays that way.

It had one — `CS_ACCESS_TOKEN`, for the Code Health job — and losing it is the main thing the [code health ADR](2026-07-30-code-health-and-coverage-gates.md)'s move to local enforcement bought. An empty inventory is a stronger property than a short one: there is no credential to leak to a compromised action, none to scope, none to rotate, and no asymmetry between what a fork's run can do and what the canonical repository's can. Anything that would add the first secret back should be argued against that, and against the fork problem that made this one untenable.

### Alternatives considered

- **SARIF-mode zizmor with code-scanning merge protection.** The action's default: findings upload to code scanning, and blocking is configured separately as a code-scanning ruleset. Rejected as indirection — the enforcement lives in a place the repository does not version, the job goes green while findings accumulate, and a lint that reports but cannot fail is a lint people stop reading. `advanced-security: false` puts the failure where the change is.
- **Leaving branch protection to discipline.** Rejected: discipline is what fails at 11pm, and it fails invisibly. Every argument for the mechanical gates in the code-health ADR applies here with more force, because the failure mode is an unrecoverable ref rather than a bad score.
- **Adding an admin bypass actor "just in case".** Rejected, and this is the load-bearing rejection. For a solo repository, the admin's own mistakes are the primary threat the rules exist to catch; a ruleset the admin can bypass provides near-zero protection against that threat while providing full protection against an attacker who does not exist. The emergency case is handled by disabling the ruleset, which is slower, visible, and logged — all three of which are features.
- **Relying on review rather than required checks for tag hygiene.** Rejected: there is nobody to review, and "the maintainer will check that the commit was green before tagging" is exactly the class of invariant this record exists to replace with a mechanism.

## Consequences

- **The solo maintainer must open a pull request for every change**, including one-line documentation fixes, and wait for the full check suite — which includes the network-dependent scanner jobs. This is deliberate friction with a real, daily, cumulative cost, and it will be tempting to disable long before it is tempting to bypass.
- **No bypass means a genuine emergency costs a visible ruleset edit.** Disabling and re-enabling a ruleset is slower than a force-push and leaves a trail. That is the intent; it is still a bad half-hour when it happens.
- **Required-check contexts are referenced by rendered job name.** Renaming a CI job silently breaks the requirement — the ruleset keeps waiting for a context nothing will ever report, and the PR sits pending rather than failing loudly. This is a real foot-gun with no mechanical guard, and it is worse for the reusable-workflow jobs whose contexts are not readable from the caller's YAML at all. Job renames must include a ruleset update in the same change.
- **The tag-creation gate depends on CI having run on the exact tagged commit.** Tagging a commit whose checks never ran, or whose check runs have aged out of GitHub's retention, will be refused — including in the case where a release is being cut from an older commit on purpose.
- **A forked pull request can now run every required check.** This is the change the [code health ADR](2026-07-30-code-health-and-coverage-gates.md) bought by moving Code Health out of CI: previously the CodeScene job could not authenticate on a fork run, so an external contribution either failed a required check for a reason that said nothing about the code, or would have had to pass unmeasured. Neither was acceptable, and the fork case has no third option while the gate needs a per-contributor credential. The cost moved rather than vanished — Code Health is now something the maintainer must run locally before merging, and nothing mechanical stops a merge that skipped it.
- **`online-audits: false` leaves the impostor-commit, known-vulnerable-actions, and typosquat audits unrun.** The pins and Dependabot are the compensating controls, and neither of them detects a SHA that never belonged to the ref it claims.
