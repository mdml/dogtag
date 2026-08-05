# The M3 prerelease, its acceptance criteria, and the three-part cutover

- Status: accepted
- Date: 2026-08-03

## Context

M3 ships as an installable prerelease and carries the widest cutover of the beta so far. [beta.md](../../beta.md#milestones) names reading and listing notes plus M2's deferred schema-explanation transfer; [the M2 surfaces record](2026-07-31-m2-surfaces-and-the-sdk-boundary.md) already stated that corpus checks move at M3 with `check`; and the M2 release record's 2026-08-03 amendment discovered that the incumbent's *scheduled* work is corpus linting — the one workflow with a real incumbent on a schedule, which the seven-day parallel-run machinery was designed for and never got to exercise. That amendment also raised, and deliberately did not answer, a beta.md question: whether M2's dogfooding obligation is unmet or was mis-specified. This record executes the answer beta.md now carries.

The release path is built and rehearsed; what M3 decides is the version, what "done" means, and what evidence licenses the cutover — the same three questions the [M2 release record](2026-07-31-m2-release-and-cutover.md) answered for its milestone, several of whose criteria are inherited verbatim.

## Decision

### Version

**The next free prerelease increment on the `0.1.0` line — nominally `0.1.0-beta.2`.** If a `beta.1` defect fix ships first, M3 takes the next counter; the convention is M2's, unchanged.

### Acceptance criteria

M3 is done when all of the following hold:

1. **All 24 conformance scenarios are `executable` and green against `dense` and `starter`** — the ten M2 scenarios plus the nine graduated and five added by [the fixtures record](2026-08-03-m3-fixtures-and-conformance.md) — with the printed matrix distinguishing a pair that ran from one pending on an unbuilt corpus.
2. **`just gate` passes locally in full** — Code Health 10.0 on every supported file, the coverage ratchet, MSRV, `cargo-deny`, OSV, zizmor.
3. **The coverage baseline is raised in the same commit that adds kernel code**, and every file under the kernel paths holds 100% line coverage. M3 adds the document model, the traversal, and a YAML-subset parser to the kernel, so this floor binds harder than it ever has.
4. **Every required GitHub check has executed and passed on the merge commit.** The rulesets exist and are active; this criterion is now a mechanism, not self-attestation.
5. **The tag is pushed only from a commit whose required checks passed, and the draft release is inspected and published by a human.**
6. **Installation is verified from the public release on macOS and Linux**, by the M2 sequence: one download, `gh attestation verify --repo mdml/dogtag --signer-workflow mdml/dogtag/.github/workflows/release.yml`, sidecar comparison, install of that verified file via `DOGTAG_DOWNLOAD_BASE`, then life proven with `dogtag version`, `doctor`, and now `check`, `list`, and `show` against a fixture vault. The SBOM attestation check runs with `--predicate-type https://cyclonedx.org/bom`, expecting what the M2 amendment corrected: a success line, not the SBOM; reading the document takes `--format json`.
7. **The supported range widens to `1..=2` only in the change that carries the per-version key sets and per-version default tables** ([the contract-version-2 record](2026-08-03-contract-version-2.md)). Widening without them is the regression the vault-contract amendment named; this criterion is that sentence with a gate behind it.
8. **The new fixture floors are harness-enforced in the same change that raises them** — construct coverage and the three mechanized irregularity floors — per the M2 lesson that a floor nothing reads is a floor that erodes.
9. **The `dense` note-derivation privacy receipts are recorded** as the gate requires: the numeric-artifact check and the read-back, attested in the authoring commit's message.
10. **The scripted smoke sequence over the fixtures exists in the tree, is green, and has run before the tag** — the M3 process change's replacement for interactive manual testing, made a criterion so the replacement cannot quietly not happen.
11. **The cutover evidence below is complete.**

M3's dependency closure grows (a YAML parser enters the shipped binary), and the SBOM path M2 built absorbs it: generation, reconciliation against `cargo tree`, sidecars, and attestation are standing release-path behavior, not a new decision.

### The cutover: three parts

Every part obeys the M2 evidence design: install first, seven consecutive days of parallel running with every discrepancy triaged to a dogtag defect or a deliberate scope difference, defects fixed forward as the next prerelease, **a fix that changes what the surface reports restarts that part's seven days**, then the incumbent retires and does not return. Diagnostic output from a private vault is private; triage happens locally against `--format json`, and the roadmap receipt carries no path, schema content, vocabulary, or hostname. The three clocks run concurrently; none blocks another.

**Part one — the scheduled corpus lint moves to `dogtag check --strict`.** This is the workflow the M2 amendment identified as the incumbent's real scheduled job, and the first cutover in the beta with a genuine incumbent on a schedule — the comparison the seven-day machinery was built for, finally run. The honest caveat is recorded up front: the incumbent lint likely checks things the contract cannot express — value formats and other linter-over-the-SDK territory — so triage may legitimately leave a residue with a reduced incumbent tool. **A residue is a named scope difference, not a fallback**: what moves, and what the receipt credits, is the schema-enforced lint; what remains is enumerated in the private triage and summarized in the receipt as "checks outside the contract's expressive range," without vocabulary. Keeping the *moved* checks warm in the incumbent is what does-not-move-back forbids; keeping checks dogtag cannot perform is not moving back, it is scope, credited precisely — the discipline the M2 record set when it named which half of the health check moved.

**Part two — reading and listing move to `show` and `list`.** The daily reading and listing workflows in the maintainers' toolchain repoint at installed dogtag on day one of the window; seven days of real use; then the incumbent read/list commands for those workflows are retired from the skills and toolchain. Interactive workflows get the same evidence standard as scheduled ones rather than a lighter one — a single verified session is the single-clean-run shape the M2 record already rejected.

**Part three — schema explanation to agents moves to `contract explain`.** The transfer M2 named and could not perform: with the version-2 tag vocabulary, `contract explain` can carry the subtype vocabulary and the per-type required tag prefixes the incumbent prints. **Its precondition is a private act this trail cites and cannot perform: the founder vault migrates its contract to version 2 and declares its tag vocabulary — and, closing the kind-lattice record's gap one, its previously omitted record-valued properties.** Then the note-authoring skills repoint, seven days, and the incumbent schema-printing command retires.

### M2 closes, and the rule that closes it

**beta.md's cutover rule gains the roll-forward clause this packet decided: where a milestone's surface turns out to displace nothing — discoverable only by attempting the cutover — the obligation rolls forward to the next milestone rather than being waived, and the receipt records that nothing moved.** Applied to M2: the verdict is *mis-specified, codified*, not unmet. `doctor` displaced nothing because the founder vault never had a scheduled configuration check; the obligation rolled into this milestone's three parts; M2's remaining roadmap boxes resolve now — the publish-and-install box closed by the verified installs on both platforms, the cutover box marked rolled-forward per the amendment — and M2 ships. M3 becomes the active rung. The alternative, holding M2 open until M3 completes the rolled-forward transfer, was rejected: it makes M2's closure depend on work M2 cannot contain, for a criterion whose own attempt proved it described a workflow that never existed.

### Alternatives considered

- **The two-part cutover, per beta.md's entry as written.** Rejected: `check` would ship additive at its own milestone — the exact shape that just embarrassed M2 — and the one workflow with a real scheduled incumbent would wait a rung for no stated reason. beta.md's entry is corrected to name three parts.
- **A lighter evidence standard for the interactive parts.** Rejected: it is the single-successful-run shape the M2 record rejected, and two standards for one rule is where the weaker one becomes the rule.
- **Retiring the incumbent lint wholesale, residue included.** Rejected: it would abandon real checks to credit a cutover, which is the receipt overclaiming — the thing receipts exist not to do.
- **M2 unmet, held open.** Rejected as above.
- **`0.2.0-beta.0`.** Rejected at M2; nothing has changed.

## Consequences

- **M3 cannot be declared done for at least seven days after the prerelease is installed**, as M2 could not — but M3's clocks can actually start, which M2's never did. Three concurrent clocks mean one report-changing fix can restart one part without restarting the others.
- **The lint cutover may end with a named residue**, and the receipt will credit a partial move precisely. That is honesty about the contract's expressive range, and it seeds the E-series question of whether the linter-over-the-SDK story actually gets built.
- **Part three's precondition is private work** — the founder vault's version-2 migration — that the public record can cite but never verify, the same asymmetry M2's first evidence step carried.
- **The roll-forward clause weakens the cutover rule's rhetoric and strengthens its truthfulness**: a milestone can no longer satisfy the rule by renaming an additive feature a cutover, because the receipt must now say what moved, and "nothing" is a legal answer only when the attempt proved it.
- **M2 ships with a receipt that says nothing moved.** That reads badly and is correct, which is the point of receipts.

## Amendments

The Decision above stands as written; these later entries change parts of it, and the original text is left intact so the change is legible.

- **2026-08-05 — part one's incumbent does not exist, and part one resolves as nothing-moved with the schedule established fresh.** Attempting the cutover discovered that the founder toolchain has no scheduled corpus lint. The M2 release record's 2026-08-03 amendment, which named "the incumbent tooling's scheduled work is corpus linting," was itself mistaken: corpus linting runs at commit time, continuously, exactly as the configuration check before it did, and nothing runs on a schedule. Under the roll-forward clause's own standard the receipt must say what moved, and for part one the honest answer is again "nothing" — there is no incumbent to run beside for seven days, so the comparison the machinery was built for has no second lane. Resolved with the founder: part one's receipt records that nothing moved; a scheduled `dogtag check --strict` over the founder vault is established fresh as an additive act — dogtag is the first thing ever on that schedule — credited as establishment rather than cutover, with no seven-day window because there is nothing to compare against. Parts two and three are untouched and their clocks run as written. The pattern now has two data points — M2's `doctor` and M3's `check` both found no scheduled incumbent, because the incumbent philosophy enforces at commit time — so the next record that names a scheduled incumbent owes that naming a verification at packet time, not at cutover time.
