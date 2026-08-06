# Reading

One measurement, inside an [envelope](reference/api/v2/schemas/envelope.md).

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `metric` | string | yes | Dotted, lowercase, stable. |
| `value` | number | yes | No units in the number. |
| `unit` | string | yes | Part of the identity. |
| `taken_at` | instant | no | Defaults to the envelope's `sent_at`. |

## The unit is part of the identity

`inlet.temperature` in `celsius` and `inlet.temperature` in `kelvin` are two metrics, not one
metric with two spellings. The gateway will store both and nothing will ever reconcile them, so
decide the unit once per metric and never change it. [The glossary](glossary.md) says the same
thing in one sentence.

## Naming

Dotted segments, lowercase, no version numbers, no collector name. The collector is already the
envelope's identity, so `bench-1.inlet.temperature` says it twice and makes the metric
unqueryable across the fleet.

## Instants

`taken_at` may precede `sent_at` by any amount — a collector that was offline for a day sends a
day of history at once, and that is the intended shape. It may not *follow* `sent_at`, which is
refused as `ingest.clock-outside-skew`.
