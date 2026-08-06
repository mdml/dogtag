---
type: component
title: Gateway
summary: The four parts of a running gateway, and how each one fails.
aliases:
  - quillon-gateway
  - the gateway
owner: platform
tags:
  - area/ledger
updated: 2026-07-14
depends-on: internals/ledger/overview.md
mentions: "[the single-writer decision](decisions/0001-single-writer-ledger.md)"
---

# Gateway architecture

Four parts, in the order an envelope meets them.

## 1. The listener

Terminates TLS, authenticates the collector, and frames envelopes. Stateless; a listener that
dies drops connections and nothing else. Collectors reconnect and resume from the opening frame
described in [streams](reference/api/v2/streams.md).

## 2. The accept queue

Bounded, in memory, not durable. Its depth is the number operators watch first. Everything about
why it is bounded is in [backpressure](backpressure); the short version is that an unbounded
queue converts a disk problem into a data-loss problem.

**Failure mode:** a process death loses the queue. This is why
[queue-drain.md](guides/operations/runbooks/queue-drain.md) forbids a restart as a way to hurry
a drain.

## 3. The writer

One per region. Appends to the open segment, seals it at size or age, and acknowledges only
after the write returns. The single-writer choice is
[decisions/0001-single-writer-ledger.md](decisions/0001-single-writer-ledger.md).

**Failure mode:** a torn write at the tail of the open segment. Detected at start and truncated;
[recovery](internals/ledger/recovery.md) has the procedure and the reasoning.

## 4. The compactor

Rewrites sealed segments to drop readings past retention, then removes the originals. Never
touches the open segment. See [compaction](internals/ledger/compaction.md).

**Failure mode:** falling behind, which looks like disk filling. It is the fourth of the four
numbers in [guides/operations/overview.md](guides/operations/overview.md).

## What is not here

No replication, no consensus, no automatic failover — see
[decisions/0004-regional-failover-is-manual.md](decisions/0004-regional-failover-is-manual.md).
A region is a whole system, and two regions are two systems that happen to be operated together.

## Where an envelope's time goes

Measured on the reference hardware, at the median, for an envelope carrying ten readings:

| Stage | Median | Notes |
| --- | --- | --- |
| TLS and framing | 0.4 ms | Dominated by the handshake only on the first envelope of a stream. |
| Authentication | 0.1 ms | The collector's identity is cached for the life of the stream. |
| Queue insert | 0.02 ms | Constant; the queue is a ring. |
| Segment append | 1.9 ms | Sequential write plus the fsync that makes it durable. |
| Acknowledgement | 0.1 ms | Batched, so the per-envelope cost is amortized. |

The append dominates, and it is meant to. Every design choice on this page is a choice to keep
the durable write on the critical path rather than behind it, and everything that looks slow
about the gateway is that decision showing through. The alternative — acknowledge from the
queue, write later — was the 2.x behaviour, and it meant an acknowledgement did not mean what
anyone thought it meant.

## Concurrency

The listener is concurrent, the queue is a single-producer-multiple-consumer ring with one
consumer, and the writer is a single thread. There is no lock on the append path: the writer
owns the open segment outright for the whole of its life, which is what makes the
[single-writer decision](decisions/0001-single-writer-ledger.md) worth the write ceiling it
costs.

The compactor runs on its own thread and shares nothing with the writer except the directory.
It never opens the segment the writer holds. That is checked at the top of every cycle rather
than assumed, because the check is one stat call and the failure it prevents is a corrupted
ledger.

## Memory

Three things hold memory in proportion to the fleet, and nothing else does:

- The accept queue, bounded by `queue_depth` — see
  [configuration](reference/configuration.md#accept).
- One buffer per open stream, sized to the largest permitted envelope.
- The collector identity cache, one small entry per registered collector.

At fifty thousand collectors — the hard limit in [reference/limits.md](reference/limits.md) —
those come to roughly two gigabytes, and the gateway will not start with less than three.

## What a restart costs

A restart drops every stream, discards the accept queue, and then runs start-up recovery over
the open segment. Collectors reconnect over a spread interval, so the reconnect storm is a ramp
rather than a wall, but the queue will still spike for a minute or two. This is why
[queue-drain.md](guides/operations/runbooks/queue-drain.md) treats a restart as the wrong tool
and why [upgrading](guides/upgrading.md) upgrades one gateway per region at a time.

## What is deliberately absent

- **No read path through the gateway.** Taps read the ledger's files. A read that went through
  the writer's process would contend with the append, and there is no version of that trade
  that favours the read.
- **No in-process retention policy.** Retention is the compactor's, and the compactor is a
  separate loop precisely so that a slow retention policy cannot slow an append.
- **No cross-region anything.** See
  [decisions/0004-regional-failover-is-manual.md](decisions/0004-regional-failover-is-manual.md).

Every part named here has a glossary entry: see [the glossary](glossary.md).
