# Streams

One stream per collector, ordered, long-lived.

## Opening

    GET /v2/streams/{collector}
    Upgrade: quillon/2

The gateway answers an **opening frame** before anything else:

| Field | Meaning |
| --- | --- |
| `last_sequence` | The highest sequence number in the ledger for this collector. |
| `region` | The region name, so a misconfigured collector notices immediately. |
| `skew_budget` | The accepted clock skew, in seconds. |

Resume from `last_sequence + 1`. A collector that ignores the opening frame and resends from its
own idea of the position will meet `ingest.sequence-regressed`.

## Sending

Envelopes, in order, as framed in [envelope](reference/api/v2/schemas/envelope.md). Gaps are
allowed; regressions are not.

## Acknowledgements

Batched, and durable. See the collector-author's guidance in
[streaming-ingest.md](guides/integration/streaming-ingest.md) — particularly the part about
holding unacknowledged envelopes.

## Refusal frames

A refusal frame carries an identifier from [errors](reference/api/errors.md) and, for
`ingest.backpressure`, a retry hint in milliseconds.

## One stream only

A second stream for the same collector is refused with `ingest.duplicate-stream`. Per-collector
limits are in [reference/api/v2/limits.md](reference/api/v2/limits.md), and they are per
collector precisely so that opening more connections cannot buy throughput.

Stream, envelope and sequence are all defined in [the glossary](glossary.md).
