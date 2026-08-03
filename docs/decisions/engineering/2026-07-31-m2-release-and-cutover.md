# The M2 prerelease, its acceptance criteria, and the cutover

- Status: accepted (amended 2026-08-01, 2026-08-03 — see [Amendments](#amendments))
- Date: 2026-07-31

## Context

Every milestone from M2 on ships as an installable prerelease and names one real workflow that moves from the incumbent personal tooling onto installed Dogtag and does not move back ([beta.md](../../beta.md#required-properties)). M2's named cutover is the daily vault health check. The release path itself is already built and proven by M1 ([release pipeline and artifacts](2026-07-30-release-pipeline-and-artifacts.md)), so what M2 has to decide is the version, what "done" means, and what evidence licenses the cutover.

This is being decided while GitHub Actions has an unexplained runner-assignment backlog for this repository, so post-merge workflows queue without runners. That is a temporary infrastructure state, but it determines what the implementation milestone may do while it persists, and the answer belongs in the record rather than in someone's memory.

## Decision

### Version

**`0.1.0-beta.1`.** The beta is one `0.1.0` line and each milestone increments the prerelease counter, as M1 established with `0.1.0-beta.0`. It keeps the minor axis free for the beta's own graduation and needs no change to the release workflow's tag-equals-version guard.

Spending the minor axis on milestone numbering (`0.2.0-beta.0`) was rejected: the beta would then have no obvious version to graduate into, and every subsequent milestone would need a fresh convention.

### Acceptance criteria

M2 is done when all of the following hold:

1. The M2 conformance scenarios are `executable` and green against the `dense` and `starter` fixtures, and the printed matrix distinguishes a pair that *ran* from one skipped because its corpus is unbuilt. The first draft of this criterion said only that the matrix shows the complete cross product with no cell unaccounted for — which is satisfied today by a matrix rendering `docs` and `records` identically to a scenario nobody has written, and would let half-coverage read as full. The distinction the harness already computes must reach the rendering before this criterion means anything.
2. `just gate` passes locally in full, including Code Health 10.0 on every supported file, the coverage ratchet, MSRV, `cargo-deny`, OSV, and zizmor.
3. The coverage baseline in `coverage-baseline.toml` is **raised in the same commit** that adds kernel code, and every file under the kernel paths holds 100% line coverage. M2 is where `crates/dogtag/src/` stops being a version string, so this is the first milestone at which the kernel floor genuinely binds.
4. Every required GitHub check has executed and passed on the merge commit — and the repository rulesets in [the workflow-security record](2026-07-30-workflow-security-and-repository-rules.md) have actually been applied. That record states it decides the rules but that applying them needs administrative scope the maintainer's token does not carry, so until they exist this criterion is self-attested discipline rather than a mechanism. **Provisioning them is a precondition of the tag, not a parallel task.**
5. The tag `v0.1.0-beta.1` is pushed only from a commit whose required checks passed, and the draft release it produces is inspected and published by a human.
6. Installation is verified **from the public release**, not from a checkout, in a sequence that binds the attested bytes to the installed bytes: download the archive once; verify it with `gh attestation verify --repo mdml/dogtag --signer-workflow mdml/dogtag/.github/workflows/release.yml`; confirm its `sha256` equals the published sidecar; then install *that verified file* by pointing `DOGTAG_DOWNLOAD_BASE` at it, on macOS and on Linux; then prove life with `dogtag version` and `dogtag doctor` against a fixture vault.

   The order matters. `install.sh` performs no attestation check — its only integrity check is the sidecar, which comes from the same release under the same write permissions, and the supply-chain policy is explicit that *"whoever can swap an asset can swap its sidecar and leave the pair agreeing."* `install.sh` also deletes its download directory on exit, so a separately downloaded archive would be a different file from the one installed, and three independent acts over three downloads would not chain into a verified installation. Naming `--signer-workflow` rather than only `--repo` is what constrains *which* workflow minted the attestation; without it, any workflow in the repository holding the attestation permissions would satisfy the check.

   The same archive now carries a second attestation, and checking it is a second command rather than a longer one: **`gh attestation verify <archive> --repo mdml/dogtag --signer-workflow mdml/dogtag/.github/workflows/release.yml --predicate-type https://cyclonedx.org/bom`**, whose output is the target's CycloneDX SBOM. The flag is not optional and not cosmetic — `gh attestation verify` defaults to the SLSA provenance predicate, so the command without it re-verifies the provenance and exits zero having never looked at the SBOM. [The supply-chain policy](2026-07-30-supply-chain-and-vulnerability-policy.md) fixed both the flag and the moment it becomes due ("the day the SBOM ships"); this is that day, so it is written here rather than decided here.
7. The cutover evidence below is complete.

### SBOM and provenance

**The M2 release ships an SBOM, because the standing trigger fires here.** [The supply-chain policy](2026-07-30-supply-chain-and-vulnerability-policy.md) already set the condition — *the first prerelease whose dependency closure differs from the M1 set* — and pre-decided the tooling. This record executes that decision rather than making a new one.

It was very nearly missed. The first draft of this section deferred the SBOM behind a freshly invented trigger ("a consumer or policy that actually requires one, or the first non-prerelease tag") without citing or superseding the standing one, on the reasoning that `serde` and `toml` are already workspace dependencies so the contract format adds nothing. That reasoning was **true of the workspace and false of the shipped artifact**, which is precisely the distinction the supply-chain policy drew: the SDK crate's `[dependencies]` is empty today, and every dependency in the installed binary comes through the CLI. Parsing the contract with spans moves `serde`, `toml`, and their tree into the SDK, and structured output adds a JSON dependency that `Cargo.lock` does not currently contain at all — taking the shipped closure from roughly seventeen crates to the mid-twenties. That is the trigger condition, met squarely.

The supply-chain policy predicted this failure in its own words: *"If the trigger condition passes unnoticed — a dependency added without anyone re-reading this ADR — the deferral silently becomes an omission."* It passed unnoticed, and was caught by review rather than by anything mechanical, which is worth recording as evidence about how well a named trigger with no owner actually works.

So: the M2 implementation adds SBOM generation to the release path alongside what M1 already ships — `cargo auditable`'s embedded dependency list, SLSA build-provenance attestations, per-asset `sha256` sidecars, and the aggregate `sha256.sum` — with its tool pinned in `tools.toml`, its workflow step declared, and gate parity updated in the same commit.

**Measured 2026-07-31, once the SBOM existed to count with.** The estimate above — roughly seventeen crates growing to the mid-twenties — was low. The generated CycloneDX document for the shipped binary holds **34 components**: the `dogtag` library plus 33 third-party crates, where the same document over the pre-M2 tree holds 18, the library plus the seventeen the estimate counted. The closure very nearly doubled rather than growing by half, which meets the trigger more decisively than predicted rather than differently, so the figure moves and nothing decided here does.

### The cutover

**What moves is the vault configuration health check, not corpus linting.** `dogtag doctor` at M2 reads the vault root, the contract, and the installation record, and opens no note ([the M2 surfaces](2026-07-31-m2-surfaces-and-the-sdk-boundary.md)). Corpus checks move at M3 with `check`. The roadmap receipt names which half moved, so the ladder's does-not-move-back rule is credited for what actually happened.

**Evidence required, in order:**

1. The real vault carries a hand-authored `.dogtag/contract.toml` that loads with zero error diagnostics. This is a private-repository act and is not performed here.
2. `0.1.0-beta.1` is installed from the public release and verified as above.
3. **Seven consecutive days of parallel running**: the incumbent configuration check and `dogtag doctor --strict` both run, and every discrepancy is triaged to either a dogtag defect or a deliberate scope difference. Defects are fixed forward before the cutover completes — and because criterion 6 requires verification from the public release, a fix ships as `0.1.0-beta.2` rather than as a locally patched binary. **A fix that changes what `doctor` reports restarts the seven days; one that does not, does not.** That rule is written down because it is the criterion that costs calendar time, which is exactly where schedule pressure lands, and "does the clock restart" should not be decided in the moment by whoever wants it not to.

   Diagnostic output from a private vault is private: triage happens against `--format json` locally, and pasted output is treated the way the [fixture privacy gate](2026-07-31-m2-fixtures-and-the-privacy-gate.md) treats vocabulary, because diagnostics quote the corpus's own names.
4. The incumbent configuration check is removed from the schedule and does not return.
5. `docs/roadmap.md` records the receipt: that the vault configuration health check runs on installed dogtag, and the date. **No path, no schema content, no vocabulary, no hostname.**

A single successful run was rejected as evidence: it cannot show the check keeps working, and *does not move back* is the property the cutover rule actually tests. Cutting over while keeping the incumbent as a live fallback was rejected for the same reason — keeping the old path warm is precisely what makes moving back cheap.

### Publication gating while required checks cannot run

**While required GitHub checks cannot execute, implementation proceeds locally to completion and stops hard at the merge gate.** This section is conditional and is spent the moment required checks run green on a merge commit; it is written in the present tense about a transient infrastructure state, and `docs/roadmap.md` — the one document licensed to go stale — is where that state is tracked. While the condition holds:

- Permitted: implementing, testing, running the full local gate including Code Health, committing on a feature branch, and reviewing.
- Blocked unconditionally: pushing, merging, tagging, publishing, bumping the version, and weakening, skipping, or reinterpreting any gate.

There is no time-based escape and no documented exception. A local gate is evidence; a required check is permission, and the two are not interchangeable no matter how long the backlog lasts. An exception that expires after N days was rejected explicitly: it would establish that a required check is waivable under schedule pressure, which is the erosion path every gate record in this trail was written to prevent. Pausing implementation entirely was also rejected — `just gate` already covers everything except the macOS suite and the musl release build, so work done under it is verified work, and stalling a milestone for an unrelated infrastructure fault buys nothing.

### Alternatives considered

- **`0.2.0-beta.0`.** Rejected as above.
- **Deferring the SBOM behind a new trigger.** Rejected once the standing trigger was found: restating an existing policy with different terms, without citing or superseding it, is the one move this trail's discipline forbids. Superseding it deliberately remained available — the argument would have had to be that a fifty-percent growth in the shipped closure no longer warrants an SBOM — and was not made because it is not believable.
- **A single clean run as cutover evidence.** Rejected as above.
- **Cutting over with the incumbent retained as a fallback.** Rejected as above.
- **A documented publication exception after a fixed backlog period.** Rejected as above.
- **Pausing M2 implementation until the backlog clears.** Rejected as above.

## Consequences

- **M2 cannot be declared done for at least seven days after the prerelease is installed**, regardless of how quickly the code lands. That is calendar time bought deliberately, and it is the first milestone where the cutover rule costs schedule rather than effort.
- **The cutover's first precondition is work in a private repository** that this trail cannot verify or cite. The public record has the receipt and the date; the evidence behind it stays private, which is a deliberate asymmetry and a limit on what the public record can prove.
- **The coverage ratchet becomes a genuine constraint at M2.** Every branch in the kernel needs a test, including the compatibility branches no real vault reaches — which is why the version gate is a pure function over an injectable range.
- **The SBOM adds a pinned tool and a release-workflow step to a milestone already carrying the largest implementation load of the beta.** That cost is the price of the standing policy being honored rather than quietly restated, and it is smaller than the cost of discovering at the beta verdict that the trigger fired six milestones ago.
- **A named trigger with no owner nearly failed on its first use.** It was caught by an independent review pass, not by any gate, which is worth remembering the next time a decision is deferred behind a condition nobody is scheduled to re-read.
- **If the Actions backlog persists, M2 stalls at the merge gate with completed, locally verified work.** That is the intended behavior and it is uncomfortable by design; the discomfort is what keeps the alternative from looking reasonable.

## Amendments

The Decision above stands as written; these later records change parts of it, and the original text is left intact so the change is legible.

- **2026-08-01 — the shipped closure is 31 components, not 34, and the generator alone could not have told us.** The figure recorded above was read off `cargo cyclonedx`'s output, which builds its component list from `cargo metadata`'s resolve graph. That graph is filtered by platform but not by feature, so `toml`'s optional `preserve_order` contributed `indexmap`, `equivalent`, and `hashbrown` to a document describing a binary that links none of them. Three of the 34 were phantoms. The word *measured* in the paragraph above was earned against the generator, not against the artifact — the same conflation this record warns about two paragraphs earlier when it distinguishes what is true of the workspace from what is true of the shipped artifact. `scripts/sbom.sh` now reconciles the generated document against the closure `cargo tree` resolves for the same target and fails if the two disagree in the direction a filter cannot fix. The trigger conclusion is unchanged and nothing decided here moves: the closure still very nearly doubled, from 18 to 31.
- **2026-08-01 — the SBOM assets carry `sha256` sidecars and join the aggregate.** [The release pipeline record](2026-07-30-release-pipeline-and-artifacts.md) documents `sha256.sum` as the one file that checks everything. The SBOM shipped as an attestation *predicate*, which means `gh attestation verify` reads the copy in the transparency log and never looks at the published asset, so the four `.cdx.json` files sat beside the archives with nothing covering them. Each now gets the sidecar every other published asset gets, and the aggregate is built from every sidecar rather than only the archives'.
- **2026-08-01 — the release path is rehearsed in CI, not only locally.** `scripts/package.sh` and `scripts/sbom.sh` were covered by `just dist` and by nothing a workflow ran, so their first CI execution would have been the `v0.1.0-beta.1` tag itself — on four targets, two of which nothing local builds, with a burned version number as the cost of a failure, `v*` tags being immutable. The `release-build-check` job now runs both scripts on its one target, and `scripts/check-gate-parity.py` records them so the rehearsal cannot quietly lapse.

- **2026-08-03 — cutover evidence steps 3 and 4 are inapplicable, because the incumbent they name does not exist.** Both are written against an *incumbent configuration check*: step 3 runs it daily beside `dogtag doctor --strict` for seven days, step 4 removes it from the schedule. Neither is performable. The founder vault has never had a scheduled configuration check — its configuration is enforced continuously, at commit time, by the schema that defines it, so there was nothing to put on a schedule and nothing to take off one. The incumbent tooling's scheduled work is corpus linting, which this record already excludes from M2 by name. Running `doctor` alone for seven days and recording step 3 as passed would report a comparison that never happened, so the two steps are marked **inapplicable rather than satisfied**, and the seven-day clock is not started.

  **The consequence reaches past this record, and is the more important half.** [beta.md](../../beta.md#required-properties) requires each milestone from M2 on to name one real workflow that moves onto installed Dogtag and does not move back. M2's named workflow is the daily vault health check — but `doctor` did not *replace* a configuration check, it is the first one this vault has ever had. It is purely additive, and nothing moved, so there is no does-not-move-back property for M2 to demonstrate. M3 is the first milestone where a workflow genuinely leaves the incumbent. **Whether that means M2's dogfooding obligation is unmet or was mis-specified is not this amendment's to decide** — it is a `beta.md` question, raised here because this is where it surfaced. What is decided is that M2 does not claim evidence it does not have.

  This is the dogfooding rule working rather than failing: the criterion did not survive contact with a real vault, and the only thing that could have discovered that is the attempt to satisfy it.

- **2026-08-03 — criterion 6 overstates what the SBOM check prints.** The criterion says the `--predicate-type https://cyclonedx.org/bom` command's *"output is the target's CycloneDX SBOM."* It is not: `gh attestation verify` prints a success line on a terminal and nothing at all when redirected, and never renders the predicate. Reading the SBOM requires `--format json` and the `.verificationResult.statement.predicate` path. The criterion's substance is untouched — the flag is neither optional nor cosmetic, and a bogus predicate type still fails with a 404, so a clean exit does prove the CycloneDX attestation was found and verified. Only the sentence about output is wrong, and it cost a founder-machine verification run an unnecessary round of doubt about whether the check had passed.
