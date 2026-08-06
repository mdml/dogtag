---
type: decision
title: Regional failover is manual
summary: The gateway never chooses which region is authoritative.
decided_on: 2026-04-21
alternatives:
  - Automatic failover on a health check
  - Automatic failover behind a quorum of observers
reversibility: moderate
tags:
  - area/operations
updated: 2026-06-30
part-of: internals/architecture.md
---

# Regional failover is manual

**Decided 2026-04-21. Reversibility: moderate.**

No automatic failover. A human decides, and the procedure they follow is
[regional-failover.md](guides/operations/runbooks/regional-failover.md).

## Why

Two regions each hold an append-only ledger. When they cannot see each other, nothing in either
one can determine which holds more of the truth — only that they differ. An automatic failover
therefore does not resolve the ambiguity; it picks a side quickly and hides that it picked.

Since the reconciliation afterwards is manual regardless — see
[recovery](internals/ledger/recovery.md) — automation would buy minutes of availability at the
cost of the operator not knowing when the gap began. The runbook's first step is to write down
the time, and that is precisely the information automation destroys.

## What was rejected

**Health-check failover.** Rejected: a health check answers whether a region is reachable from
the checker, which is not the question.

**Quorum of observers.** Rejected as a consensus system in disguise, which
[decisions/0001-single-writer-ledger.md](decisions/0001-single-writer-ledger.md) already declined
for the write path.

## What it costs

Minutes of downtime, and an operator who has read the runbook before the day they need it.
