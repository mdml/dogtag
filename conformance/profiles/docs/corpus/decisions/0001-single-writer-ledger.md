---
type: decision
title: A single-writer ledger
summary: One writer per region, and growth by adding regions rather than shards.
decided_on: 2025-11-04
alternatives:
  - Shard by tenant, as 0000 proposed
  - Multi-writer with a consensus log
  - Single writer with a hot standby
reversibility: hard
tags:
  - area/ledger
updated: 2026-02-18
part-of: internals/architecture.md
supersedes: decisions/superseded/0000-shard-by-tenant.md
---

# A single-writer ledger

**Decided 2025-11-04. Reversibility: hard.**

Each region has exactly one process that appends to the ledger. Growth is by adding regions.

## Why

The ordering guarantee we sell is per-stream, and a single writer gives it for free. Every
alternative buys throughput by making ordering a distributed problem, and then spends the rest
of its life explaining to users why a reading arrived out of order.

The second reason is smaller and turned out to matter more: a single writer makes recovery a
file-level operation. [recovery](internals/ledger/recovery.md) is four pages because there is
one writer; with consensus it would be a subsystem.

## What was rejected

**Shard by tenant** — the previous decision,
[decisions/superseded/0000-shard-by-tenant.md](decisions/superseded/0000-shard-by-tenant.md).
Rejected because tenancy is not knowable at ingest: a collector belongs to a site, and sites
move between tenants.

**Multi-writer with a consensus log.** Rejected on operational cost. It would remove the manual
failover in
[decisions/0004-regional-failover-is-manual.md](decisions/0004-regional-failover-is-manual.md)
and add a quorum every operator would then have to understand.

**Hot standby.** Rejected as the worst of both: the complexity of two writers and the throughput
of one.

## The alternatives, at length

The three were costed against the same fleet: fifty thousand collectors, one reading per
collector per minute, a ninety-day retention window.

### Shard by tenant

The proposal in [decisions/superseded/0000-shard-by-tenant.md](decisions/superseded/0000-shard-by-tenant.md).
Its appeal was operational: a tenant that grew could be moved to its own hardware without a
migration, and per-tenant retention became a configuration key rather than a feature.

It failed on a fact about the domain rather than on any engineering ground. **Tenancy is not a
property of a collector.** A collector belongs to a site — a building, a vehicle, a mast — and
sites are reassigned between tenants during the working life of the hardware. A reading already
written into a tenant's shard cannot follow the site to its new tenant, because the shard key is
part of the path and sealed segments are never rewritten. Every workable version of the idea
ended in a retroactive rewrite across sealed segments, which is the one operation the ledger
does not have and will not grow.

### Multi-writer with a consensus log

Two or more writers per region, ordering agreed through a log. This is the option that actually
lifts the write ceiling, and it was costed seriously.

Rejected on operational surface, in three parts. It converts every hardware fault into a quorum
question that an operator must be able to reason about at three in the morning. It makes
[recovery](internals/ledger/recovery.md) a distributed-systems problem rather than a truncation
— today's procedure is four pages and ends with `truncate`. And it buys throughput this fleet
does not need: the measured ceiling of one writer is roughly four times the modelled peak, and
the modelled peak already includes a reconnect storm.

The ceiling is the thing to revisit if that stops being true. It is a hard reversal — a
multi-writer ledger is not a configuration of a single-writer one — which is why this record is
marked hard rather than moderate.

### Single writer with a hot standby

A second process, following the first, ready to take over. Rejected as the worst available
combination: the operational complexity of two writers, the throughput of one, and a failover
that must decide whether the primary is dead or merely unreachable — the same undecidable
question [decisions/0004-regional-failover-is-manual.md](decisions/0004-regional-failover-is-manual.md)
declines to answer automatically at the regional level, asked once per second instead of once
per incident.

## What it costs

A write ceiling per region, which shows up to operators as
[backpressure](backpressure) and to planners as the arithmetic in
[capacity-planning.md](guides/operations/capacity-planning.md).

It also fixes the shape of growth: regions, not nodes. Every operator-facing consequence in this
tree — the manual failover, the two-ledger reconciliation, the per-region hard limits — descends
from this page.
