# Error identifiers

Every refusal the gateway makes carries one of these. The identifier is stable across the major
version; the sentence beside it is not.

## Ingest

| Identifier | Meaning |
| --- | --- |
| `ingest.unknown-collector` | The identity is not registered, or has been retired. |
| `ingest.sequence-regressed` | An envelope numbered below one already accepted. |
| `ingest.clock-outside-skew` | The envelope's instant is outside the configured skew. |
| `ingest.envelope-malformed` | The frame did not parse; see [envelope](reference/api/v2/schemas/envelope.md). |
| `ingest.backpressure` | Accepted nothing; retry after the hinted interval. |
| `ingest.duplicate-stream` | A second stream for a collector that already has one. |

## Admin

| Identifier | Meaning |
| --- | --- |
| `admin.name-taken` | Registration with a name already in the fleet. |
| `admin.not-quiesced` | An operation that requires a quiesced gateway. |
| `admin.reload-unreloadable` | A changed key that a reload cannot pick up. |

## Startup

| Identifier | Meaning |
| --- | --- |
| `start.ledger-path-missing` | No `[ledger] path`. |
| `start.ledger-foreign-region` | The directory holds another region's ledger. |
| `start.unknown-key` | A configuration key this version does not read. |
| `start.limit-exceeded` | A configured value above a hard limit in [reference/limits.md](reference/limits.md). |

## Reading these

`ingest.backpressure` is the only one on this page that is not a defect. Everything else means
something is wrong with the caller, the configuration, or the disk. The operator-side reading of
backpressure is [backpressure](backpressure).

The identifiers here name things defined in [the glossary](glossary.md) — collector, envelope,
stream, ledger — and the start-up ones are grouped [above](#startup).
