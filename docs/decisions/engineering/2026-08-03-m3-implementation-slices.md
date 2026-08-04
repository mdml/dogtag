# The M3 implementation slices and the harness-trial flags

- Status: accepted (amended 2026-08-04 — see [Amendments](#amendments))
- Date: 2026-08-03

## Context

The M3 decision packet is frozen across five records; this one turns it into bounded implementation contracts. Two process changes from the M2 retrospective bind the shape (scope-14 log, 2026-08-02): reviews run as bounded background passes that return findings, with the interactive session adjudicating rather than conducting; and M3's slices double as the bounded trial tasks for an external harness-comparison trial run from the maintainers' toolchain — the same slice implemented three times through three coding harnesses on separate branches, each lane unassisted, with this repository's deterministic gates (`just gate`, the conformance matrix, the coverage ratchet) as the shared judge, and only one branch merging. Trial slices must therefore be representative rather than critical-path, sized under roughly an hour, with stable contracts and gate-checkable acceptance.

## Decision

### The eleven slices

Each slice is a bounded contract: its scope is what its packet record decides, its non-goals are everything else, and its acceptance is the gate plus the scenarios it makes green. Dependency order:

1. **Version machinery.** Per-version key sets and default tables; supported range `1..=2`; the `supported` classification and its info diagnostic live; version-1 resolution provably unchanged. ([contract-version-2](2026-08-03-contract-version-2.md), acceptance criterion 7.)
2. **Tags construct.** `[tags]`, `[[type.tag-namespace]]`, their validity rules, the catch-all rule, `contract explain` rendering across Markdown, JSON, and provenance.
3. **Record kind.** Parsing, validation, `list` of `record`, rendering. Depends on 1; independent of 2.
4. **Document model core.** Traversal, the YAML-subset frontmatter parser, typing and catch-all binding, kind validation, the `note.*` diagnostics. ([m3-document-model](2026-08-03-m3-document-model.md).)
5. **Link resolution.** The name index, typed-link extraction per dialect, bare-name/path-qualified/ambiguous resolution, the `link.*` diagnostics.
6. **`check`.** Corpus walk, aggregation, report shape, exit semantics. ([m3-surfaces](2026-08-03-m3-surfaces-check-list-show.md).)
7. **`list`.** The four filters, both renderings.
8. **`show`.** Reference resolution, both renderings.
9. **Fixtures.** `dense`'s contract to version 2 with namespaces and records; the numeric note derivation and roughly forty authored notes; `starter`'s notes. The counting and the read-back are maintainer-only acts under the privacy gate; authoring from the numeric artifact is delegable. ([m3-fixtures-and-conformance](2026-08-03-m3-fixtures-and-conformance.md).)
10. **Conformance.** Graduate the nine, add the five, extend the derivation machinery to corpus transformations with their tests. One change with slice 9's notes, per the one-act rule.
11. **Release.** The scripted smoke sequence, tag, publish, verified installs, receipts. ([m3-release-and-cutover](2026-08-03-m3-release-and-cutover.md).)

Slices 1–5 are the critical path; 6–8 parallelize once 4 and 5 merge; 9–10 are one change once 1–3 fix the format and 4–8 the behavior; 11 closes.

### The trial flags

**Slices 7 (`list`), 8 (`show`), and the smoke-sequence script from slice 11 carry the trial flag.** All three are bounded, hour-scale, off the critical path, and land after the document model merges, so their contracts are stable when the lanes start; each has unambiguous acceptance (scenarios plus the gate for 7 and 8; the release record's criterion 10 for the script). Per the trial protocol: each flagged slice runs once per harness on its own branch, no cross-harness review or repair, gate evidence recorded per lane, one branch merges. `check` was the near-alternative and stays in the ordinary lane — bigger than an hour and closer to the critical path; the `dense` note-authoring step is delegable under the privacy gate but is not representative of the coding work the trial measures.

### Alternatives considered

- **Coarser slices** (one version-2-constructs slice; one surfaces slice). Rejected: slices stop being hour-scale, and the trial loses its candidates.
- **Flagging `check` instead of the smoke script.** Rejected as above, narrowly — revisit if the trial wants a third code slice and the schedule has slack.
- **Flagging note authoring.** Rejected: creative-writing work would measure the wrong capability for the harness decision the trial serves.

## Consequences

- **Three slices are implemented three times and two implementations of each are discarded.** That cost belongs to the trial, not to M3, and is why the flags sit off the critical path.
- **The flagged slices' packet records are their specifications**, so any ambiguity a lane hits is a packet defect worth recording — the trial doubles as a legibility test of this packet.
- **Slice 9's privacy-gated steps cannot be fully delegated**, and the slice list says which half is whose, so the boundary is never negotiated mid-implementation.

## Amendments

The Decision above stands as written; these later entries change parts of it, and the original text is left intact so the change is legible.

- **2026-08-04 — the fixture-contract migration belongs to slice 1, and a lane brief may not fence what the packet requires.** Slices 1–5 ran; every lane independently found that widening the supported range while the committed fixtures declare version 1 turns fourteen green scenario pairs red, so the migration this record filed under slice 9 executes with the version machinery instead ([the fixtures record](2026-08-03-m3-fixtures-and-conformance.md#amendments) carries the reasoning). The launch brief's out-of-scope list said "fixtures" anyway — a fence contradicting the packet it was meant to protect — and the lanes rightly resolved the contradiction toward the packet while reporting it. The rule this teaches is recorded for the trial fan-out and every later brief: **the packet is the specification and a brief is scaffolding; where they conflict, the packet wins and the conflict is a finding.** The slices 1–5 build also validated the process shape itself: ten adversarial verify passes returned twenty-six confirmed findings including four blocking, and all seventy-six packet-defect reports arrived through the report-don't-guess channel rather than as silent guesses — the M2 retrospective's failure mode, not repeated.
