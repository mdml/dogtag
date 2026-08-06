# Capacity planning

Enough arithmetic to size a disk, and no more.

## The one equation

    bytes/day = collectors x readings/collector/day x 96 bytes x 1.15

Ninety-six bytes is the settled per-reading cost on disk after the envelope framing is
amortized; the 1.15 is compaction headroom. Both are measured rather than derived, and both are
restated in [reference/limits.md](reference/limits.md#sizing) whenever they move.

## Retention

Retention is a duration, not a size. The compactor removes a sealed segment once every reading
in it is older than the window and no tap holds it open. A shorter window reclaims disk on the
next cycle rather than immediately, which surprises people at least once per deployment; see
[compaction](internals/ledger/compaction.md).

## Headroom

Keep a third of the disk free. The compactor writes a new segment before it removes the old
ones, so a full disk stops compaction, and stopped compaction is how a disk that was merely
nearly full becomes a disk that is entirely full.

## Growing

The gateway is a single writer by design, so you grow by adding regions, not nodes. What that
costs you operationally is a manual failover story, which is written down in
[regional-failover.md](guides/operations/runbooks/regional-failover.md).

## Vocabulary

Retention is a duration and a segment is a file; both are in [the glossary](glossary.md).
