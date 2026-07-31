# The M2 prerelease, its acceptance criteria, and the cutover

- Status: accepted
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

1. The M2 conformance scenarios are `executable` and green against the `dense` and `starter` fixtures, and the printed matrix shows the complete cross product with no cell unaccounted for.
2. `just gate` passes locally in full, including Code Health 10.0 on every supported file, the coverage ratchet, MSRV, `cargo-deny`, OSV, and zizmor.
3. The coverage baseline in `coverage-baseline.toml` is **raised in the same commit** that adds kernel code, and every file under the kernel paths holds 100% line coverage. M2 is where `crates/dogtag/src/` stops being a version string, so this is the first milestone at which the kernel floor genuinely binds.
4. Every required GitHub check has executed and passed on the merge commit.
5. The tag `v0.1.0-beta.1` is pushed only from a commit whose required checks passed, and the draft release it produces is inspected and published by a human.
6. Installation is verified **from the public release**, not from a checkout: `install.sh` on macOS and on Linux, the per-asset `sha256` sidecar verified, the SLSA provenance attestation verified with `gh attestation verify`, and proof of life through `dogtag version` and `dogtag doctor` against a fixture vault. Running from the product checkout is development, not dogfooding.
7. The cutover evidence below is complete.

### SBOM and provenance

**No new tooling.** The M2 release ships what M1 already ships: `cargo auditable`'s embedded dependency list, so a binary can be audited directly rather than by trusting a manifest elsewhere; SLSA build-provenance attestations linking every asset to its workflow run and commit; per-asset `sha256` sidecars; and the aggregate `sha256.sum`.

A CycloneDX or SPDX SBOM is deferred behind a named trigger: **a consumer or policy that actually requires one, or the first non-prerelease tag, whichever comes first.** Adding it now would mean a new `tools.toml` pin, a release-workflow step, gate-parity work, and another third-party binary in the release path — for a beta whose two users are both founders, and duplicating data `cargo auditable` already embeds in the artifact.

### The cutover

**What moves is the vault configuration health check, not corpus linting.** `dogtag doctor` at M2 reads the vault root, the contract, and the installation record, and opens no note ([the M2 surfaces](2026-07-31-m2-surfaces-and-the-sdk-boundary.md)). Corpus checks move at M3 with `check`. The roadmap receipt names which half moved, so the ladder's does-not-move-back rule is credited for what actually happened.

**Evidence required, in order:**

1. The real vault carries a hand-authored `.dogtag/contract.toml` that loads with zero error diagnostics. This is a private-repository act and is not performed here.
2. `0.1.0-beta.1` is installed from the public release and verified as above.
3. **Seven consecutive days of parallel running**: the incumbent configuration check and `dogtag doctor` both run, and every discrepancy is triaged to either a dogtag defect or a deliberate scope difference. Defects are fixed forward before the cutover completes.
4. The incumbent configuration check is removed from the schedule and does not return.
5. `docs/roadmap.md` records the receipt: that the vault configuration health check runs on installed dogtag, and the date. **No path, no schema content, no vocabulary, no hostname.**

A single successful run was rejected as evidence: it cannot show the check keeps working, and *does not move back* is the property the cutover rule actually tests. Cutting over while keeping the incumbent as a live fallback was rejected for the same reason — keeping the old path warm is precisely what makes moving back cheap.

### Publication gating while required checks cannot run

**Implementation proceeds locally to completion and stops hard at the merge gate.** While required GitHub checks cannot execute:

- Permitted: implementing, testing, running the full local gate including Code Health, committing on a feature branch, and reviewing.
- Blocked unconditionally: pushing, merging, tagging, publishing, bumping the version, and weakening, skipping, or reinterpreting any gate.

There is no time-based escape and no documented exception. A local gate is evidence; a required check is permission, and the two are not interchangeable no matter how long the backlog lasts. An exception that expires after N days was rejected explicitly: it would establish that a required check is waivable under schedule pressure, which is the erosion path every gate record in this trail was written to prevent. Pausing implementation entirely was also rejected — `just gate` already covers everything except the macOS suite and the musl release build, so work done under it is verified work, and stalling a milestone for an unrelated infrastructure fault buys nothing.

### Alternatives considered

- **`0.2.0-beta.0`.** Rejected as above.
- **Adding a CycloneDX SBOM at M2.** Rejected as above.
- **Tying the SBOM trigger to the first crates.io publish.** Rejected: it adds a second condition to track that overlaps the first-non-prerelease one.
- **A single clean run as cutover evidence.** Rejected as above.
- **Cutting over with the incumbent retained as a fallback.** Rejected as above.
- **A documented publication exception after a fixed backlog period.** Rejected as above.
- **Pausing M2 implementation until the backlog clears.** Rejected as above.

## Consequences

- **M2 cannot be declared done for at least seven days after the prerelease is installed**, regardless of how quickly the code lands. That is calendar time bought deliberately, and it is the first milestone where the cutover rule costs schedule rather than effort.
- **The cutover's first precondition is work in a private repository** that this trail cannot verify or cite. The public record has the receipt and the date; the evidence behind it stays private, which is a deliberate asymmetry and a limit on what the public record can prove.
- **The coverage ratchet becomes a genuine constraint at M2.** Every branch in the kernel needs a test, including the compatibility branches no real vault reaches — which is why the version gate is a pure function over an injectable range.
- **Deferring the SBOM means an enterprise-shaped request would arrive unmet.** The trigger is recorded so the answer is "here is when we add it" rather than "we never considered it."
- **If the Actions backlog persists, M2 stalls at the merge gate with completed, locally verified work.** That is the intended behavior and it is uncomfortable by design; the discomfort is what keeps the alternative from looking reasonable.
