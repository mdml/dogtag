# Fixture profile: `starter`

**Stands for:** a fresh install.
**Corpus:** built at M2 (the milestone that defines the committed vault-contract format and the initialization profile).

## Distinguishing axes

- **The initialization profile's own defaults, unmodified.** This fixture is exactly what a fresh install stamps — no hand-tuning on top. That makes it double duty: a conformance profile *and* a standing test that the product's own defaults conform to the product's own rules. If a scenario passes everywhere but here, initialization is shipping a corpus the validator rejects.
- **Lifecycle where the ordinary state is a named value.** The contract names a lifecycle axis whose ordinary state is an explicit named value on every note that carries the axis. This is the opposite encoding from `dense`, deliberately: the two profiles differ on the sharpest axis available, and if the same scenario can filter by the declared axis in both without either vocabulary reaching the core, the configuration seam is real. The catch-all type does *not* carry the axis, and at contract version 2 it cannot: a named ordinary state requires its axis on every type that declares it, and a version-2 catch-all may declare nothing a note must carry. An untyped note in this corpus therefore has no lifecycle state, and every note that carries one is typed.

## What the fixture is

**The normative initialization profile, authored here.** The direction of authority runs from this fixture outward: it is the definition of what a fresh install must stamp, not a copy of what one happened to produce. When an initialization command lands, a test asserts its output equals this fixture byte for byte, and a change to either is reviewed as a change to both.

That inverts the profile's original wording, which described the fixture as initialization's committed output. No initialization command exists — it appears in no milestone of [beta.md](../../../docs/beta.md#milestones) — and a fixture defined as the output of an unscheduled command could not land at M2 at all. Authoring it normatively keeps the M2 pair intact without adding a vault-creating write to the milestone whose job is opening and diagnosing. The reasoning is in [the M2 fixture and privacy record](../../../docs/decisions/engineering/2026-07-31-m2-fixtures-and-the-privacy-gate.md), which also records the absence of an initialization command as an open question rather than resolving it.

## What it must cover

Exactly one catch-all, at least one identity-bearing type, a lifecycle axis whose ordinary state is an explicit named value, and otherwise the smallest contract that is genuinely useful. It loads with zero diagnostics at any severity — doubly load-bearing here, since this fixture is the standing test that the product's own defaults satisfy the product's own rules.

## What the corpus holds now

`corpus/.dogtag/contract.toml` — the contract a fresh install stamps — and the two notes it stamps beside it: `welcome.md`, untyped, bound by the catch-all and carrying no lifecycle state, linking by bare name to `projects/getting-started.md`, typed and in its ordinary state (`status: active`). Together they demonstrate the profile's sharpest claims in the smallest corpus that can: the catch-all binding, the named-value ordinary encoding on a typed note, and one resolving wikilink. The direction of authority is unchanged — when `init` lands, its output must equal this corpus byte for byte.
