# Envelope

The outer frame. Frozen for the whole of 3.x; the argument is in
[decisions/0003-freeze-the-envelope-schema.md](decisions/0003-freeze-the-envelope-schema.md).

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `collector` | string | yes | The registered name. |
| `sequence` | integer | yes | Monotonic per collector. Deduplication key. |
| `sent_at` | instant | yes | The collector's clock. Checked against the skew budget. |
| `readings` | list | yes | One or more, each shaped as [reading](reference/api/v2/schemas/reading.md). |
| `metadata` | mapping | no | Opaque; stored and never interpreted. |

## Frozen means frozen

No field is added, removed, or given a new meaning inside 3.x. A field the gateway does not know
is a refusal — `ingest.envelope-malformed` — rather than something ignored, because an ignored
field is an integration that appears to work and silently drops data.

## Size

An envelope carries at most 1 000 readings and at most 1 MiB over the webhook door. Both are in
[reference/api/v2/limits.md](reference/api/v2/limits.md).

## Deduplication

`collector` plus `sequence` is the identity. Resending an identical envelope is free; resending
a *different* envelope under a sequence number already accepted keeps the first one and answers
success, which is the one place the gateway prefers silence to a complaint.
