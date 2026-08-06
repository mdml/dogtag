---
type: decision
title: Drop the agent sidecar
summary: Collectors speak to the gateway directly; the per-host sidecar is removed.
decided_on: 2026-01-27
alternatives:
  - Keep the sidecar as an optional buffer
  - Reimplement the sidecar as a library
reversibility: moderate
tags:
  - area/ingest
updated: 2026-05-11
mentions: guides/integration/streaming-ingest.md
---

# Drop the agent sidecar

**Decided 2026-01-27. Reversibility: moderate.**

The per-host sidecar that buffered envelopes on behalf of collectors is removed. Collectors open
their own stream.

## Why

The sidecar existed to hold envelopes while the gateway was busy. Once acknowledgements became
durable and backpressure became explicit, the collector could do that itself with a queue it
already had — and doing it in the collector puts the buffer where the data is, rather than one
process boundary away from it.

It also removed an entire class of incident: a sidecar that died with envelopes in it looked to
the collector exactly like a successful send.

## What was rejected

**Keep it as optional.** Rejected because an optional buffer is a supported buffer, and the
failure mode above does not become acceptable by being opt-in.

**Reimplement as a library.** Rejected as a rename. The buffering guidance now lives as prose in
[streaming-ingest.md](guides/integration/streaming-ingest.md), where it can be read by an
implementer in any language.

## Migration

Deprecated in [3.0](releases/2026-05-quillon-3-0.md), removed in
[3.1](releases/2026-07-quillon-3-1.md). A 2.x collector cannot connect to a 3.1 gateway, which
is the one thing [upgrading](guides/upgrading.md) puts in bold.
