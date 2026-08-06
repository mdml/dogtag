# Caller limits

What one caller is allowed. For what a deployment will carry, see
[reference/limits.md](reference/limits.md) — the other page of this name, one level up.

## Per collector

| Limit | Value |
| --- | --- |
| Envelopes per second | 200 |
| Readings per envelope | 1 000 |
| Burst allowance | 5 seconds at 4x |
| Open streams | 1 |

Exceeding the rate is `ingest.backpressure`, not a hard refusal: the gateway would rather slow
you down than lose you. Exceeding readings-per-envelope is a hard refusal, because the frame is
already parsed by the time the count is known.

## Per request

| Limit | Value |
| --- | --- |
| Webhook body | 1 MiB |
| Admin page size | 500 |

## What is not limited

Metrics per collector. There is a soft recommendation in
[reference/limits.md](reference/limits.md#soft-limits) and nothing enforces it.

## See also

[streams](reference/api/v2/streams.md) for where the per-collector stream limit bites, and
[webhooks](guides/integration/webhooks.md) for the body limit.
