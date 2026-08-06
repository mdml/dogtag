---
type: reference_page
title: Deployment limits
summary: What one gateway and one ledger will carry, and the constants used to size them.
applies_to: 3.1
tags:
  - area/operations
updated: 2026-07-14
describes: internals/ledger/overview.md
---

# Deployment limits

What one gateway will carry. For what one *caller* is allowed, you want the other limits page,
[reference/api/v2/limits.md](reference/api/v2/limits.md).

## Hard limits

| Limit | Value | Consequence of exceeding |
| --- | --- | --- |
| Collectors per region | 50 000 | Registration refused. |
| Open streams per gateway | 50 000 | Connection refused at handshake. |
| Segment size | 1 GiB | Configuration refused at start. |
| Retention window | 400 days | Configuration refused at start. |

## Sizing

The constants the arithmetic in
[capacity-planning.md](guides/operations/capacity-planning.md) uses:

- 96 bytes per reading on disk, after framing is amortized.
- 1.15 compaction headroom multiplier.
- One sealed segment per hour per 3 000 collectors, at one reading per collector per minute.

These are measured on the reference hardware and restated whenever they move by more than five
per cent. They are not a promise.

## Soft limits

Nothing enforces these; exceeding them simply hurts.

- More than about 200 metrics per collector makes the tap in
  [reference/cli/quillon-tap.md](reference/cli/quillon-tap.md) unpleasant to read.
- Retention longer than 90 days on spinning disk makes compaction the dominant cost. See
  [compaction](internals/ledger/compaction.md).

## Vocabulary

Collector, segment and retention window are defined in [the glossary](glossary.md). The hard
limits are the ones a start-up refusal enforces, [above](#hard-limits).
