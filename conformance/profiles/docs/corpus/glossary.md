# Glossary

The words below mean exactly this throughout the documentation. Where a page uses one loosely,
that page is wrong.

**Collector** — a process on a device that reads sensors and speaks the ingest protocol. One
collector may carry many readings. Registered through
[reference/api/v2/collectors.md](reference/api/v2/collectors.md).

**Envelope** — the outer frame a collector sends: collector identity, sequence number, clock,
and a payload of readings. Frozen since 3.0; see
[reference/api/v2/schemas/envelope.md](reference/api/v2/schemas/envelope.md) and the decision
that froze it, [decisions/0003-freeze-the-envelope-schema.md](decisions/0003-freeze-the-envelope-schema.md).

**Reading** — one measurement: a metric name, a value, a unit, and the instant it was taken.
The unit is part of the identity, so `celsius` and `kelvin` are two metrics.

**Stream** — one collector's ordered sequence of envelopes. Streams are per-collector and never
merged; the ordering guarantee is per-stream only.

**Ledger** — the append-only store every envelope lands in. It is the system of record; nothing
downstream reads a device. See [internals/ledger/overview.md](internals/ledger/overview.md).

**Segment** — one file of the ledger. Segments are sealed at a size or an age and then never
written again, which is what makes [compaction](internals/ledger/compaction.md) safe.

**Region** — a deployment of the whole gateway with its own ledger. Regions do not replicate to
each other automatically; see
[decisions/0004-regional-failover-is-manual.md](decisions/0004-regional-failover-is-manual.md).

**Backpressure** — the gateway's refusal to accept faster than it can durably write. It is a
normal operating state, not a fault; [backpressure](backpressure) says what to do about it.

**Tap** — a read-only subscription to the ledger, used for debugging. See
[reference/cli/quillon-tap.md](reference/cli/quillon-tap.md).
