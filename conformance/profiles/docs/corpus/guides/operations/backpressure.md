---
type: guide
title: Backpressure
summary: Why the gateway refuses to accept faster than it can write, and what to do about it.
tags:
  - area/operations
  - audience/operator
reviewed_on: 2026-06-30
updated: 2026-06-30
mentions: internals/ledger/overview.md
---

# Backpressure

Backpressure is the gateway declining to accept an envelope it cannot yet promise to have
written. It is a designed state, not a fault, and a fleet that never sees it is provisioned for
its peak rather than its median.

## What a collector sees

A refusal, with a retry hint in it. A well-behaved collector holds the envelope, waits the hint,
and sends it again with the same sequence number; the ledger deduplicates on that number, so a
retry is never a duplicate reading. The refusal code is listed in
[errors](reference/api/errors.md).

## Why it happens

Almost always disk. The gateway acknowledges an envelope only after the segment write returns,
so the acknowledgement rate is bounded by the ledger's write path and nothing else. The three
causes, in the order they actually occur:

1. The ledger shares a disk with something bursty.
2. Compaction is running and the disk has no headroom left for it.
3. The fleet genuinely grew.

## What to do

Confirm which one it is before changing anything. Queue depth that falls to zero between
compaction cycles is cause 2; queue depth that never falls is cause 3.

For cause 1 and cause 2, move the ledger or give it headroom —
[capacity planning](guides/operations/capacity-planning.md) has the arithmetic. For cause 3,
either shorten retention or add a region; the gateway does not shard, which was decided
deliberately in
[decisions/superseded/0000-shard-by-tenant.md](decisions/superseded/0000-shard-by-tenant.md) and
then decided the other way in
[decisions/0001-single-writer-ledger.md](decisions/0001-single-writer-ledger.md).

## What not to do

Do not raise the accept buffer to make the number look better. The buffer is not durable; every
envelope in it is one you have accepted and could still lose.
