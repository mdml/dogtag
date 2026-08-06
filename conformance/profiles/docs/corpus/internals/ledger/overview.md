# The ledger

An append-only sequence of segments. One open segment, many sealed ones, and a rule that a
sealed segment is never written again. Almost everything else about the system follows from that
rule.

## Segments

A segment is a file: a header naming the region and the generation, then framed envelopes in
arrival order, then a trailer written when it is sealed. Sealing happens at `segment_bytes` or
`segment_age`, whichever comes first — both in
[configuration](reference/configuration.md#ledger).

An unsealed segment has no trailer, which is how a start-up detects a gateway that died mid-run.

## Generations

The generation is stamped once, at creation, from the region name. A gateway refuses a ledger
directory whose generation names another region — `start.ledger-foreign-region` in
[errors](reference/api/errors.md) — because merging two regions' segments by copying files is
the failure this check exists to prevent.

## Ordering

Per collector, and only per collector. Two collectors' envelopes interleave in a segment in
whatever order they arrived; nothing in the ledger claims otherwise, and nothing downstream
should assume otherwise. [The glossary](glossary.md) states this in one line.

## Reading

Taps read segments directly and hold them open. That is why a tap can stop the compactor —
[compaction](internals/ledger/compaction.md) will not remove a segment somebody is reading.

## Why append-only

Because it makes [recovery](internals/ledger/recovery.md) a truncation rather than a repair, and
because it is what lets [reference/limits.md](reference/limits.md) promise a retention window
without promising a size.
