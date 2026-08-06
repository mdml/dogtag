---
type: decision
title: Shard the ledger by tenant
summary: Superseded. Tenancy is not knowable at ingest time.
decided_on: 2025-08-12
alternatives:
  - Shard by collector hash
  - A single writer per region
reversibility: hard
state: superseded
tags:
  - area/ledger
updated: 2025-11-04
---

# Shard the ledger by tenant

**Decided 2025-08-12. Superseded 2025-11-04 by
[decisions/0001-single-writer-ledger.md](decisions/0001-single-writer-ledger.md).**

The original plan: one ledger shard per tenant, so a large tenant could be moved to its own
hardware without touching anyone else's readings.

## Why it was decided

It made the largest customer's growth somebody else's problem, and it made per-tenant retention
a configuration rather than a feature.

## Why it was wrong

Tenancy is not knowable at ingest. A collector belongs to a *site*; sites are reassigned between
tenants during the life of the hardware, and a reading already written into a tenant's shard
cannot follow. The shard key would have had to be rewritten retroactively across sealed
segments, which the ledger's whole design forbids — see
[internals/ledger/overview.md](internals/ledger/overview.md).

## What it is kept for

The rejection reasoning, which 0001 cites, and the sizing note: the per-tenant arithmetic here
was the first version of what is now in
[capacity-planning.md](guides/operations/capacity-planning.md).
