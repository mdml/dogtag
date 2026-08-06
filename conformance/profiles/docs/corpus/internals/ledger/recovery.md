# Recovery

What a gateway does when it starts against a ledger that was not shut down cleanly, and what a
human does when two regions have diverged.

## Start-up recovery

Automatic, and short, because segments are append-only.

1. Find the segment with no trailer. There is at most one; more than one is corruption and the
   gateway refuses to start.
2. Scan it forward, framing envelopes, until framing fails.
3. Truncate at the last complete envelope.
4. Reopen it for append.

A partly-written envelope at the tail was never acknowledged — the writer acknowledges after the
write returns, per [internals/architecture.md](internals/architecture.md) — so truncating it
loses nothing anybody was told was durable. The collector still holds it and will resend.

## Divergence between regions

Not automatic, and not short. It arises after
[regional-failover.md](guides/operations/runbooks/regional-failover.md), when two ledgers each
hold readings the other does not.

There is no merge. The two ledgers stay separate, and reconciliation means exporting sealed
segments from one and reading them alongside the other's — offline, with both regions quiesced,
using the collector-plus-sequence identity from
[envelope](reference/api/v2/schemas/envelope.md) to detect the overlap.

Do not attempt this while either region accepts traffic. The export is a consistent read only of
sealed segments, and the open segment is by definition still moving.

## What cannot be recovered

The accept queue. Everything in it at the moment of a process death is gone, and the collectors
that sent it were never told otherwise, so they still hold it — provided they follow
[streaming-ingest.md](guides/integration/streaming-ingest.md) and keep unacknowledged envelopes.

## Vocabulary

Segment, trailer, generation and accept queue are defined in [the glossary](glossary.md) and in
[internals/architecture.md](internals/architecture.md#2-the-accept-queue).
