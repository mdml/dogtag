# Integrating with the gateway

Two ways in, and the choice between them is a rate question.

| Way in | Use when | Page |
| --- | --- | --- |
| Webhooks | fewer than a few readings a minute, bursty, no long-lived connection possible | [webhooks](guides/integration/webhooks.md) |
| Streaming ingest | everything else | [streaming-ingest.md](guides/integration/streaming-ingest.md) |

Both deliver the same envelope, and the ledger cannot tell them apart afterwards. What differs
is who holds the retry: with the stream, the collector does; with a webhook, you do.

## Before either

Register the collector. An unregistered identity is refused at both doors — see
[collectors](reference/api/v2/collectors.md) — and the refusal is deliberate: a fleet that
accepts unknown devices has no fleet inventory.

## Rate limits

Both doors share the per-collector limits in
[reference/api/v2/limits.md](reference/api/v2/limits.md). They are per collector, not per
connection, so opening a second stream buys nothing.

## Vocabulary

Collector, envelope, reading and stream mean what [the glossary](glossary.md) says they mean.
The distinction between a collector and a device catches out most first integrations.
