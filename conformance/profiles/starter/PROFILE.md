# Fixture profile: `starter`

**Stands for:** a fresh install.
**Corpus:** scheduled, built at M2 (the milestone that defines the committed vault-contract format and the initialization profile).

## Distinguishing axes

- **The initialization profile's own defaults, unmodified.** This fixture is exactly what a fresh install stamps — no hand-tuning on top. That makes it double duty: a conformance profile *and* a standing test that the product's own defaults conform to the product's own rules. If a scenario passes everywhere but here, initialization is shipping a corpus the validator rejects.
- **Lifecycle where the ordinary state is a named value.** The contract names a lifecycle axis whose ordinary state is an explicit named value on every note. This is the opposite encoding from `dense`, deliberately: the two profiles differ on the sharpest axis available, and if the same scenario can filter by the declared axis in both without either vocabulary reaching the core, the configuration seam is real.

## What the fixture is

The contract and starter notes produced by initialization, committed verbatim. Because the fixture *is* the default output, it is regenerated (and its diff reviewed) whenever the initialization profile changes.

## Why it is not built yet

The committed vault-contract format and the initialization profile that stamps it are M2 decisions. This fixture cannot exist before the command that produces it; the specification lands now, the corpus lands with initialization.
