# The M5 release and the capture cutover

- Status: accepted
- Date: 2026-08-07

## Context

M5's prerelease, nominally `0.1.0-beta.4`, ships safe mutation: the write transaction and `capture` ([the surfaces record](2026-08-07-m5-capture-and-the-write-transaction.md)), contract version 3 ([the version record](2026-08-07-contract-version-3.md)), and the write scenario set ([the fixtures record](2026-08-07-m5-fixtures-and-conformance.md)). The cutover rule from [beta.md](../../beta.md#required-properties) applies as amended: one real workflow moves onto the installed binary with seven days of parallel running, an obligation with no incumbent rolls forward with an honest receipt, and — the rule M3's part one added — **a record that names an incumbent owes that naming a verification at packet time, not at cutover time.**

This packet was frozen 2026-08-07, while M3's three cutover clocks (earliest close 2026-08-12) and M4's search clock (earliest close 2026-08-14) were still running — the M4 precedent applied deliberately: a packet freezes when its decisions are ready; implementation follows the packet.

## Decision

### Acceptance criteria

1. The write transaction and `capture` implemented per the surfaces record: structural exemption, plan-as-value with `--preview`, commit-at-birth with pathspec scope where dogtag owns the commit path, guest mode writing uncommitted, transaction-verdict exit semantics, the `write.*` area minted with its enum review, every write through the verified `VaultRoot` handle.
2. `contract_version = 3` — the `[capture]` and birth-state seats — with the supported range widening to `1..=3` only in the change that carries the per-version key sets and default tables, per the version record. The trailer pair's keys fixed in the same review as the `write.*` enum.
3. The M5 scenario set executable and green on `dense`, `starter`, and `docs`, with `starter` migrated to version 3 and the other two deliberately held at version 2, per the fixtures record; the matrix distinguishing ran from pending throughout; every floor the changes raise harness-enforced in the changes that raise them.
4. The JSON report schema takes its one tick for the milestone when the first M5 report shape lands, per the standing per-milestone rule.
5. The scripted smoke sequence extended with capture steps over the fixture corpora — including a preview, a capture, and a read-back of the captured note — green before the tag.
6. The prerelease published from a passing tag and installed from the public release with verified attestations on macOS and Linux, the established verification sequence unchanged.
7. The capture cutover complete: one part, seven days of parallel running, the incumbent path retired, the named residue recorded — as specified below.

### The cutover: the agent-skill capture path

**The incumbent is verified as of packet time.** The founder's real capture workflow is the agent-skill path: telling an agent to ingest material — routinely from `~/files/inbox` — whereupon the `import-attachment` and `new-note` skills author vault notes through their own file-writing instructions. Verified live on 2026-08-07: the inbox held 21 items awaiting exactly this treatment, and both skills carry the inbox in their instructions. This is the packet-time verification the roll-forward clause's amendment demands, recorded here so this milestone cannot discover an absent incumbent the way M2 and M3 did.

Day one repoints the skills' **note-minting step** onto `dogtag capture` against the installed binary: where a skill today authors a new unfiled or stub note with its own Write-tool instructions, it instead invokes `capture` and continues from the structured result. Triage during the parallel week follows the established evidence design: the incumbent authoring path remains available, `--format json` results diffed against what the incumbent would have written for the same input, receipts without vocabulary, a report-changing fix restarting only this clock.

After seven clean days the skills' own note-authoring instructions for the capture step retire — the skills keep their judgment about *what* to capture; they lose their private mechanism for *how*.

**Named residue, carried forward:** binary attachment handling — moving or copying the file itself into the vault's attachment layout — stays with the incumbent skills. `import` is a distinct, deliberately unscheduled verb ([the M2 fixtures record](2026-07-31-m2-fixtures-and-the-privacy-gate.md) records its openness), and capture writes notes, not blobs. The residue is named with its destination honestly unknown: it belongs to whatever milestone schedules `import`, and the receipt must say so rather than imply capture absorbed it.

The cutover receipt also records the founder-vault capture latency at cutover time (the transaction includes a full validation pass, so the M4 latency evidence and its one-second trigger logic carry over to the write path).

### Scheduling the other homeless writes

Two standing open items close by naming destinations, not by riding this milestone: **AGENTS.md generation is scheduled at M6**, where the MCP server gives the agent contract its consumer, discharging the obligation [the M2 surfaces record](2026-07-31-m2-surfaces-and-the-sdk-boundary.md) parked at "the milestone that performs writes" — the write machinery lands here, but the file's reader arrives at M6, and a generated contract nothing consumes would be capability producing no evidence. **`init` is named as an M7 packet question**, where second-vault onboarding is the workflow that would exercise it and `starter`'s corpus is already the normative output waiting. M5 itself ships one mutation, and the scope contract means that sentence.

### Alternatives considered

- **Two parts (interactive capture, then the skills).** Rejected: the founder's interactive capture *is* telling an agent; there is no second lane to run in parallel.
- **Naming `~/files/inbox` file-drops as the incumbent.** Rejected at verification: the drops are input to the agent conversation, not a capture mechanism of their own; the mechanism the skills own is what `capture` displaces.
- **Deferring the cutover to M6.** Rejected: the dogfooding rule again — a write milestone that produces no write evidence is the failure the rule exists to prevent.
- **AGENTS.md generation riding M5.** Rejected: a second write path inside the one-mutation milestone, with no consumer until M6.
- **Waiting for the M3/M4 clocks before freezing.** Rejected: the clocks gate those milestones' receipts, not this packet's decisions — the precedent now twice applied.

## Consequences

- **These criteria land in the public roadmap when M5 becomes the active rung** — from this record, unchanged; the roadmap section is a mirror, not a second source.
- **Three clocks may briefly run at once** (M3's tail, M4's search week, this one when it starts) — the cost of not serializing decisions behind receipts, accepted twice already.
- **The parallel week produces double notes by design** — the incumbent's authored note and capture's, diffed then reconciled — so triage this week includes cleanup, and the receipts must distinguish evidence from litter.
- **The residue leaves `import` visibly homeless a third time.** At some point the honest receipts pile up into a scheduling obligation; this record adds to the pile on purpose.
- **If the parallel week surfaces a fidelity regression against the incumbent's authored notes — lost text, mangled frontmatter — the cutover does not complete**, and the milestone holds open on criterion 7, the designed failure mode.

## Amendments

The Decision above stands as written; these later entries change parts of it, and the original text is left intact so the change is legible.

- **2026-08-08 — the named incumbent does not exist in the shape this record describes.** The Decision has day one repoint a step where "a skill today authors a new unfiled or stub note with its own Write-tool instructions." Verified against the vault repository at cutover time: neither skill does that. `new-note` delegates to `pkm note new` and `import-attachment` to `pkm attachment import`, and both stamp a **typed, fully-shaped note** — fileClass and subtype, frontmatter, required tags, a body template, a humanized `# Title`, a born status — which is filing, the act [the inbox-workflow PDR](../product/inbox-workflow.md) holds separate from capture. `capture` creates an untyped catch-all note with no frontmatter and no template, so repointing that step onto it would not transfer a workflow; it would delete structure from every note the skills mint, which is the fidelity regression this record's own last consequence names as the reason a cutover does not complete.
- **2026-08-08 — the packet-time verification measured the workflow, not the mechanism.** The Decision cites twenty-one items in `~/files/inbox` as evidence. They are binaries — PDFs and images — and they flow through `import-attachment`, which is precisely the residue this record already carves out and leaves with the incumbent. The vault's own `inbox/` directory is empty, and the skills only ever *consume* from it. So the evidence offered for the incumbent pointed at the exclusion. The roll-forward clause's amendment demands a packet-time verification of any named incumbent; this record performed one and it confirmed the wrong thing, which is a lesson about what such a verification has to establish — that the *mechanism* being displaced exists, not merely that the workflow does.
- **2026-08-08 — criterion 7 rolls forward with an honest receipt; M5 ships on criteria 1–6.** The obligation was **mis-specified rather than unmet**, the same diagnosis and the same clause M2's receipt recorded. No parallel week runs: an incumbent that does not exist cannot be run beside anything, and the double notes such a week would produce would be litter rather than evidence. The receipt records what mutation evidence does exist — `capture` exercised by hand against the published `0.1.0-beta.4` on both macOS and Linux, preview writing nothing, the act landing at exit 0 with its warning and recovery, and the corpus still green afterward — and calls it that rather than dressing it as a cutover.
- **2026-08-08 — the capture cutover is scheduled at M6, where capture gains a consumer.** This is the reasoning this record already applied to AGENTS.md generation: a capability whose consumer has not arrived produces no evidence. [The ladder](../../beta.md#milestones) puts an SDK-backed MCP server at M6 exposing `search`, `show`, and **`capture`** — an agent calling capture through a server is the real workflow, and a parallel run against it means something. The residue is unchanged and travels with it: binary attachment handling stays with the incumbent skills, its destination still honestly unknown until some milestone schedules `import`.
