# Fixture profile: `starter`

**Stands for:** a fresh install.
**Corpus:** scheduled, built at M2 (the milestone that defines the committed vault-contract format and the initialization profile).

## Distinguishing axes

- **The initialization profile's own defaults, unmodified.** This fixture is exactly what a fresh install stamps — no hand-tuning on top. That makes it double duty: a conformance profile *and* a standing test that the product's own defaults conform to the product's own rules. If a scenario passes everywhere but here, initialization is shipping a corpus the validator rejects.
- **Lifecycle where the ordinary state is a named value.** The contract names a lifecycle axis whose ordinary state is an explicit named value on every note. This is the opposite encoding from `dense`, deliberately: the two profiles differ on the sharpest axis available, and if the same scenario can filter by the declared axis in both without either vocabulary reaching the core, the configuration seam is real.

## What the fixture is

**The normative initialization profile, authored here.** The direction of authority runs from this fixture outward: it is the definition of what a fresh install must stamp, not a copy of what one happened to produce. When an initialization command lands, a test asserts its output equals this fixture byte for byte, and a change to either is reviewed as a change to both.

That inverts the profile's original wording, which described the fixture as initialization's committed output. No initialization command exists — it appears in no milestone of [beta.md](../../../docs/beta.md#milestones) — and a fixture defined as the output of an unscheduled command could not land at M2 at all. Authoring it normatively keeps the M2 pair intact without adding a vault-creating write to the milestone whose job is opening and diagnosing. The reasoning is in [the M2 fixture and privacy record](../../../docs/decisions/engineering/2026-07-31-m2-fixtures-and-the-privacy-gate.md), which also records the absence of an initialization command as an open question rather than resolving it.

## What it must cover

Exactly one catch-all, at least one identity-bearing type, a lifecycle axis whose ordinary state is an explicit named value, and otherwise the smallest contract that is genuinely useful. It loads with zero diagnostics at any severity — doubly load-bearing here, since this fixture is the standing test that the product's own defaults satisfy the product's own rules.

## Why it is not built yet

The committed vault-contract format is an M2 decision. The contract lands with the format it must be written in; the starter notes land with the document model at M3.
