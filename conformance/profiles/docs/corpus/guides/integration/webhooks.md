# Webhooks

One HTTP request carries one envelope. Simple, and the wrong choice above a few readings a
minute — see [guides/integration/overview.md](guides/integration/overview.md) for the trade.

## The request

`POST` the envelope as the body. The envelope's shape is in
[envelope](reference/api/v2/schemas/envelope.md); nothing about it changes because it arrived
over HTTP.

Sign the request. The signature covers the raw body and the timestamp header, and a request
whose timestamp is outside the accepted skew is refused before the body is read.

## The response

- `202` — accepted and durable. The gateway does not answer 202 until the write returns.
- `409` — this sequence number is already in the ledger. Your retry worked the first time.
- `429` — backpressure. Wait the hinted interval and send the same body again.
- `4xx` otherwise — the envelope is wrong; see [errors](reference/api/errors.md).

## Retries

You own them. Resend the identical body with the identical sequence number; the ledger
deduplicates, so an over-eager retry costs a round trip and nothing else. Do not renumber on
retry — a renumbered retry is a second reading of the same measurement, and nothing downstream
can tell that it is not.

## What webhooks do not get

Ordering. A stream is ordered per collector; a sequence of independent HTTP requests is not, and
the gateway will not hold one back waiting for a gap to be filled.
