---
type: runbook
title: Drain the accept queue
summary: The gateway is accepting but not writing; drain it without losing an envelope.
severity: urgent
steps:
  - Confirm the queue is actually growing
  - Stop accepting new connections
  - Let the queue drain to zero
  - Find out why
  - Resume
tags:
  - area/operations
updated: 2026-07-02
part-of: guides/operations/overview.md
---

# Drain the accept queue

**Severity: urgent.** An envelope in the accept queue is one the gateway has taken and not yet
made durable. Losing the process loses them.

## 1. Confirm

    quillon fleet status --watch

Queue depth climbing for more than two consecutive compaction cycles is the condition this
runbook is for. A single spike is not; see [backpressure](backpressure).

## 2. Stop accepting

    quillon fleet quiesce

Collectors are refused with a retry hint and will hold their envelopes. Nothing is lost by
quiescing; the whole point of the retry hint is that this is safe.

## 3. Wait for zero

The queue drains at the ledger's write rate and no faster. Do not restart the gateway to hurry
it — a restart discards the queue, which is the one thing this runbook exists to prevent.

## 4. Diagnose

By now the disk is the suspect. Check free space, then compaction lag. The three usual causes
are in [backpressure](backpressure) and the arithmetic for the third is in
[capacity-planning.md](guides/operations/capacity-planning.md).

## 5. Resume

    quillon fleet resume

Watch the first two minutes. Collectors return all at once, so the queue will spike; if it does
not fall again, you have not fixed anything and should quiesce again rather than hope.
