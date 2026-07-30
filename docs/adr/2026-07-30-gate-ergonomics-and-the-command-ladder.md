# Gate ergonomics and the command ladder

- Status: accepted
- Date: 2026-07-30

## Context

The quality and security contract ([code health and coverage](2026-07-30-code-health-and-coverage-gates.md), [supply chain](2026-07-30-supply-chain-and-vulnerability-policy.md), [workflow security](2026-07-30-workflow-security-and-repository-rules.md), [commit convention](2026-07-30-commit-convention-and-release-notes.md)) arrived as a set of gates and two recipes: `just check` for the offline ones, `just gate` for everything CI enforces that can run locally. Both worked. Neither was pleasant to run, and one of them was quietly wrong.

The unpleasantness was output volume. Every gate printed everything it had — an evidence table, a per-file score list, thirty-four licence notes — so a green run buried the one number a reader wanted under a screen of proof that nothing was wrong. That is a bad trade in a repository maintained largely by agents, where a gate's output is context spent.

The wrongness was drift. The same commands were written out in four places — the justfile, `ci.yml`, `security.yml`, `lefthook.yml` — and nothing compared them. `just gate` already ran `zizmor` over a narrower path than CI did, ran a different OSV scanner version with different pass/fail semantics, and validated a commit range CI never uses. `AGENTS.md` described `gate` as "everything CI enforces that can run locally" while omitting `links` from its own list of what `gate` runs. None of this was noticed by anything.

One measurement reframed the problem. The whole suite, warm, is **32 seconds — and 27 of them are the CodeScene sweep's one network round trip per file.** Everything else together is under five seconds: 2,458 lines of Rust, four packages, 37 locked dependencies. So the ergonomic problem was never wall-clock. It was output volume and honesty.

## Decision

### A ladder of four commands, defined by scope and authority rather than by speed

```
just fast          while implementing            0.9s warm, offline
just check         before handing work off       1.9s warm, offline, deterministic
just gate          before opening a pull request  32s warm; network, pinned tools, CS_ACCESS_TOKEN
just gate-verbose  when you want the evidence    identical run, full output
```

From a cleaned target directory those become 7.1s, 8.4s and 47.5s — the compile dominates, and it is shared, which is the other reason every suite runs byte-identical commands: climbing a rung is a cargo cache hit rather than a rebuild.

Each is a strict superset of the one above. `fast` is not meaningfully faster than `check` on this repository today — the five steps `check` adds cost about a second between them — and pretending otherwise would be a lie a contributor discovers in a week. The rungs are worth having for what they *mean*: `fast` is the loop you run on every edit and is explicitly not a merge signal; `check` is what you owe a reader before handing work over; `gate` is what you owe a pull request. The final line of every run says which suite it was and what authority it carries, because `fast` already runs part of the work behind two required checks — `Format, lint, test (Linux)` and `Commit messages` — and would otherwise read as permission to merge. A run of individually named steps is labelled `steps` rather than borrowing a suite's name, so `just coverage` cannot sign off as though it were `just gate`.

The rungs will separate as the repository grows. Recording that they have not separated *yet* is the point of writing the numbers down.

### Every gate is declared once, as the command CI runs

[scripts/gate.py](../../scripts/gate.py) holds a table of steps. Each step is an id, the **exact command string** CI runs, an optional environment, and the name of an extractor for its metric. The suites are literal tuples — `CHECK = FAST + (…)`, `GATE = CHECK + (…)` — so the nesting is a fact about the data rather than a convention someone maintains, and `just gates` prints the table.

Declaring the command rather than a recipe name is what makes the next part possible.

### check-gate-parity.py binds the table to the workflows

The repository already refused to let a comment vouch for a version: `scripts/check-tool-pins.py` reads the declaration that actually runs. [scripts/check-gate-parity.py](../../scripts/check-gate-parity.py) extends that reasoning from versions to commands. It runs in `just check` and in CI, and enforces five rules:

1. Every step in the `gate` suite is a **whole `run:` line** in a workflow, or is listed in `DIVERGENCES` with the reason it cannot be. Fifteen match; four diverge.
2. Every environment variable a step declares is declared on the CI side too, so the docs gate cannot lose `-D warnings` on one side only.
3. Every required status check in `.github/rulesets/main-branch.json` maps to gate steps that exist, or is listed in `NOT_LOCAL` with the reason; each mapped step really runs in the job reporting that context; and every required context is still reported by a job of that name. This is the rule that would have caught `gate`'s missing `links`, and it is what makes "everything CI enforces that can run locally" a checked claim rather than a slogan. The job-name half also turns the fragility the [workflow security ADR](2026-07-30-workflow-security-and-repository-rules.md) names — renaming a job silently un-requires its context — into a failed build rather than a blocked merge.
4. Every repository script a workflow runs is a gate step, or is listed in `CI_ONLY` with the reason. This is the direction that notices a check added inside an already-required job. It is scoped to commands naming `scripts/`, so ordinary shell plumbing needs no entry.
5. No table carries a stale entry — a divergence whose command has since become identical is deleted, not kept, and neither context table may name a check the ruleset stopped requiring.

Matching is whole-line against a parsed `run:` step rather than a substring search of the file, and the difference is not pedantry: a substring search accepts a command sitting in a YAML comment, a CI line that is a strict superset of the local one (`… --locked --offline` satisfying `… --locked`), and a trailing flag added in CI only. All three are exactly the drift the checker exists to catch, and all three are covered by tests that mutate the real workflows.

The one entry in `CI_ONLY` records a genuine asymmetry: on a pull request CI scores the CodeScene *delta* against the base ref, while the local gate runs the full sweep. The local side is the stricter of the two — a delta cannot see a file no change touched — so this is a difference worth having rather than a gap.

The four recorded divergences, each with its reason in the source: **msrv-build** and **msrv-test** (CI sets `RUSTUP_TOOLCHAIN` because `rust-toolchain.toml` outranks `rustup default`; `cargo +1.85.0` does the same job locally), **osv** (CI runs Google's pinned reusable workflow with its own scanner image; the local binary is pinned separately in `tools.toml` and will drift, so a local scan is a pre-flight and CI is the verdict), and **zizmor** (CI passes the pinned action its settings as inputs; the local command mirrors them as flags and scopes the audit to `.github/workflows`).

Two required checks have no local counterpart, and say so: **`Test (macOS arm64)`**, which a Linux developer cannot reproduce, and **`Release build check (x86_64 musl)`**, which needs a cross target that is not available on every supported development platform — `just dist` rehearses the same build where it is.

`commits` is deliberately not a divergence: its *command* is verbatim, and what differs is the range appended at runtime. CI validates exactly the commits a pull request introduces; the local run resolves `origin/main..HEAD`. That is recorded in `gate.prepare_commits` and in AGENTS.md, because a stale `origin/main` moves the range under the developer.

### Summary and verbose differ in rendering, structurally

Both modes resolve the same steps from the same table, run the same argv, in the same working directory, with the same environment and the same **closed stdin**, and capture the same merged output through a pipe. `--verbose` additionally echoes that capture as it arrives. Nothing about which checks run, what they enforce, what scope they cover, or what they exit with can depend on the mode, because the mode reaches only the echo.

The child scripts deliberately gained **no `--verbose` flag of their own**. A flag would have made verbosity a difference in the command, which is exactly what must not vary; keeping the rendering entirely in the runner makes the equivalence structural rather than reviewed. `scripts/test_gate.py` asserts it anyway, including that both modes hand the child an identical environment and a non-tty stdin.

Closing stdin is part of the contract, not an implementation detail: a tool that decides to prompt would otherwise block forever behind a summary line with nothing to show — the one way summary mode could behave differently from verbose, and the worst way, because it presents as a hang rather than a failure.

### Metrics are rendering, and are never invented

Pass and fail come from the child's exit status alone. The metric column is decoration with a rule: it may be absent, and it may never be guessed.

Two sources. Every in-repo checker already ends with one stdout line naming itself and what it verified — `check-links: all N relative markdown links resolve.`, `coverage-check: OK — line 99.03%, branch 97.73%, worst kernel file 100.00%.` That line is the contract, and `test_gate.py` **runs six of the eight checkers and asserts each still prints one**, so a reworded summary fails a test instead of silently emptying a column. The two it does not run need a nightly toolchain and a CodeScene token respectively; `just gate` is what exercises those. Third-party tools get a small anchored extractor each, over ANSI-stripped text — CI sets `CARGO_TERM_COLOR: always`, so an extractor that ignored escapes would work on a developer's machine and find nothing in the one place the evidence is durable.

The metrics are worded to say what was measured rather than what was configured. `coverage-check` reports the *worst measured* kernel file, not the 100% bar it was checked against — printing the rule back would read identically whether the kernel was fully covered or had no files at all. `codescene-gate` counts files at 10.0 separately from files holding no scorable code, and fails if nothing was scored. `cargo-deny` carries its warning count, because it reports `ok` for a check that emitted warnings.

An extractor that finds nothing prints no number and writes `gate: no metric for <step>` to stderr. That keeps degradation visible: when a tool rewords its summary on a version bump, the metric does not quietly vanish forever.

An earlier design had the in-repo scripts write their metric to a file named by `$DOGTAG_GATE_METRIC`. It was rejected: `coverage-gate.sh` `exec`s into `coverage_check.py` and has no process left to write anything, the env var would be inherited by every grandchild, and the write would be a branch that only ever ran on developer machines — a local/CI asymmetry introduced by the very change meant to remove them.

### The run does not stop at the first failure

Every step runs; every step gets a line. Stopping early would leave a summary that reads as complete while lines are simply missing, and the suite is seconds of compute outside the CodeScene sweep. Failing steps print their captured output to **stderr** after the table — in summary mode too, so no one has to rerun verbosely to learn what broke — and the run exits with the *first* failure's status.

That status is translated the way a shell reports it: `subprocess` returns a negative number for a signal death, and `sys.exit` of a negative number is truncated to eight bits, so an untranslated SIGTERM would surface as 241 rather than 143. It is clamped so a failed step can never yield 0.

### A missing prerequisite fails its step, and only its step

`gate` needs the network, pinned tools and `CS_ACCESS_TOKEN`. When one of them is absent, three things have to be true at once, and only one arrangement gives all three.

**It is a failure, not a skip.** A Code Health gate that goes green because nobody had a token measured nothing, and a required check that passes without checking is worse than no check — the same reasoning `ci.yml` already applies to forked pull requests, where the job fails closed rather than passing unmeasured.

**It is scoped to the steps that need it.** Prerequisites are resolved per step, against that step's own list, immediately before it would have run. An absent CodeScene token blocks `codescene` and nothing else; the other eighteen gates have no opinion about it.

**The run still happens.** There is no suite-wide preflight. Aborting before the first step would throw away eighteen gates' worth of real verification to report a problem with the nineteenth, and would leave a contributor without a token unable to get any signal at all from `just gate`. Instead the suite runs to the end, the blocked step fails with one line naming what is absent and the single command that fixes it, and the suite exits nonzero:

```
codescene       fail — CS_ACCESS_TOKEN is not set; export a PAT from https://codescene.io/users/me/pat
gate            fail — 18/19 steps, 4.7s · CI and the repository rulesets remain authoritative
```

A blocked step spawns nothing, so it has no captured output; the message it carries *is* the diagnosis, and the failure dump prints it in both modes rather than falling back to a log that does not exist. A narrow `just codescene` has only the one step, so it fails immediately — the same rule, arriving sooner.

The earlier design aborted the whole suite up front. It was rejected once the behaviour was written out plainly: it converted a partial result into no result, for a credential that gates one of nineteen checks.

### CI keeps its own step lists

CI does **not** call `gate.py`. Job names are required status-check contexts, and collapsing jobs into runner invocations would change those names, which means editing both ruleset payloads, `check-ruleset-payloads.py`, and the JSON quoted in the [workflow security ADR](2026-07-30-workflow-security-and-repository-rules.md) — churning the merge rules to save some duplication. The duplication is instead made safe by the parity checker, which is the cheaper half of the trade.

This also preserves something worth keeping: CI's per-job logs stay full. The summary rendering is a local affordance, and a post-failure investigation in CI still sees the evidence table, the per-file scores, and every licence note.

### Hooks

`commit-msg` is unchanged: the fast Conventional Commit check. `pre-commit` now runs `python3 scripts/gate.py fmt clippy` instead of restating those two command strings a fourth time, plus the staged Code Health delta, still skipped when no token is configured. It deliberately does **not** run `just fast`: a commit hook that also compiles the test suite and walks the commit range stops being a hook people leave installed.

**No pre-push hook.** A long gate that fires on push is a gate people learn to `--no-verify` past, and this repository's enforcement boundary is CI plus the rulesets, not the developer's machine. `just gate` before opening a pull request is the documented step, and it is a step someone chooses.

### Alternatives considered

- **A bash gate runner.** Rejected. The orchestration needs tests, and the repository's existing mechanism for testing a checker is a plain `unittest` script — reachable from Python and awkward from shell. Portability decided it too: macOS ships bash 3.2, and `codescene-gate.sh` was already using the bash-4 `mapfile` (fixed here). Writing the runner in Python also submits it to the repository's own Code Health 10.0 contract, which `.sh` files escape by not being a scored language.
- **Having CI call `gate.py`.** Rejected above. Revisit if the required-context list is ever being edited for another reason.
- **Fail-fast.** Rejected above.
- **A `--keep-going` / `--fail-fast` flag.** Rejected as surface for a choice with one right answer at this size.
- **A TTY progress spinner rewriting a line in place.** Rejected once the profile was measured. Printing the step's name before it runs and its verdict after gives live progress in any stream, needs no TTY detection or second rendering path, and still leaves exactly one line per gate.
- **JSON output from the scanners.** Rejected for every one of them, and the reason is uniform: `osv-scanner --format json` loses the human table *and* moves its progress lines to stderr; `cargo deny --format json` empties stdout entirely; `zizmor --format json` reduces stdout to `[]`. All three would trade the diagnostic view for a metric, and changing a scanner's flags is exactly what summary mode may not do.
- **Parsing `cargo test --format json`.** Not available: libtest's JSON output is nightly-only. The `test result:` line is parsed instead, anchored so a reworded line yields nothing rather than a wrong number.
- **Adding the musl release build to `gate`.** Rejected: it would make `just gate` fail for every macOS developer. Recorded as a `NOT_LOCAL` context instead, with `just dist` named as its rehearsal.
- **Dropping the `fast` rung**, on the grounds that it is within a second of `check`. Rejected, and the measurement is why the question is fair: the rungs earn their place by meaning, not by seconds, and `fast` is the one a developer runs on every edit. The numbers are recorded above so the trade is visible rather than implied; if `check` ever grows a step that costs real time, the rung starts paying for itself in the obvious way.
- **Dropping the `-verbose` twins on the narrow recipes**, letting `just coverage` run its script directly. Rejected: it would make verbosity a property of *which recipe* you ran rather than of one flag, and re-introduce the thing this design most wants to prevent — two ways to invoke the same gate that are not provably the same invocation. The twins are four lines each and all route through one runner.

## Consequences

- **A third declaration of every command now exists**, in `gate.py`. That is a real cost, paid down by `check-gate-parity.py` — which is itself another file to maintain, and which only checks the directions it was told to check. Its known blind spots, stated so nobody has to rediscover them: it cannot notice a new CI job that never becomes a required context; it only claims *repository-script* commands, so a new third-party tool added to an existing job passes unremarked; and it reads `run:` blocks with an indentation scanner rather than a YAML parser, which is adequate for these two workflows and would need replacing before it could be trusted on arbitrary ones.
- **`just test` and the narrow recipes now route through the runner**, which means their output is summarized by default and their live cargo progress is gone. `just test-verbose` restores it. Two commands still restate a gate rather than declaring one: `just commits <range>`, whose whole purpose is an explicit range, and lefthook's `commit-msg` hook, which passes the message file rather than a range. Neither is covered by parity, and both are named here rather than left to be discovered.
- **`coverage_check.py`'s evidence table no longer prints on a passing local `just gate`.** Its docstring said the table exists "so CI logs and local runs show the same evidence", and under summary rendering that is no longer true of the default local run. The docstring has been corrected; `just coverage-verbose` and `just gate-verbose` are the evidence-bearing local invocations, and CI is unaffected.
- **`just gate` is all-or-nothing without a CodeScene token.** Accepted above.
- **Metrics will rot at tool upgrades.** The extractors are anchored regexes over third-party output; a reworded summary loses a number. The design makes that visible (`gate: no metric for …`) rather than silent, and the in-repo half is covered by tests, but nothing stops the rot.
- **The suites are ordered cheap-first**, which matters only for how a failing run reads, since every step runs regardless.
- **`just fast` and `just check` are nearly the same cost today.** Anyone comparing them will notice. The numbers are in this record so the answer is "yes, and here is why the rung exists" rather than an argument.
