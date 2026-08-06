# Streaming ingest

One long-lived connection per collector, carrying an ordered sequence of envelopes. This is the
door almost everything should use.

## Connecting

Open the stream, present the collector's credential, and wait for the gateway's opening frame.
The frame tells you the last sequence number the ledger holds for this collector, which is how a
reconnecting collector knows where to resume without asking a second question.

## Sending

Send envelopes in sequence order. Gaps are permitted — a collector that was off does not have to
invent readings — but reordering is not: an envelope numbered below one already accepted is
refused, not buffered.

## Acknowledgements

The gateway acknowledges in batches, and an acknowledgement means durable. Hold every
unacknowledged envelope. A collector that drops on send and waits for an acknowledgement it
already threw away is the single most common integration bug against this gateway.

## Backpressure on a stream

Backpressure arrives as a pause in acknowledgements, then an explicit refusal frame with a retry
hint. Stop sending, wait the hint, resume from the last acknowledgement. Do not reconnect —
reconnecting during backpressure adds a handshake to a gateway that is already behind. The
operator's half of this is [backpressure](backpressure).

## The loop, written out

Almost every integration bug against this gateway is a variation on getting this wrong, so here
it is in full.

1. **Open the stream and read the opening frame.** Do not send anything first. The frame carries
   `last_sequence`, and it is authoritative — your own idea of where you got to is not.
2. **Discard everything at or below `last_sequence`.** Those are durable. Holding them costs
   memory and resending them costs a round trip each.
3. **Send from `last_sequence + 1`, in order.** Keep every sent envelope until it is
   acknowledged.
4. **On an acknowledgement, release up to and including its sequence number.** Acknowledgements
   are batched, so one acknowledgement usually releases several.
5. **On a refusal frame, stop sending.** Read the identifier. If it is `ingest.backpressure`,
   wait the hint and go to step 3 — *not* to step 1.
6. **On a disconnection, go to step 1.** Never to step 3: the frame you did not read is the only
   thing that tells you what survived.

### The three ways this goes wrong

**Releasing on send rather than on acknowledgement.** The envelope is gone from your buffer and
was never written. Nothing will ever tell you; the reading simply is not there.

**Reconnecting on backpressure.** A gateway that is behind gets a handshake it did not need,
and the collector's own queue grows while it does. Wait the hint.

**Renumbering after a gap.** If you skipped sequence numbers because the device was off, send
the numbers you actually have. The ledger tolerates gaps. It does not tolerate two different
envelopes under one number, and it resolves that case by keeping the first — silently, as
[envelope](reference/api/v2/schemas/envelope.md) explains.

### Sizing your buffer

Hold at least the acknowledgement interval times your send rate, doubled. Below that, a normal
batching delay looks to you like a stall and you will be tempted to reconnect. The interval is
not configurable and is currently about 200 ms.

## Limits

Per-collector rate and burst are in
[reference/api/v2/limits.md](reference/api/v2/limits.md#per-collector). A second stream for the
same collector is refused rather than load-balanced.
