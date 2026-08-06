# Operating a gateway

What to watch, and what to do when it moves.

## The four numbers

- **Queue depth** — envelopes accepted but not yet durable. Should hover near zero and spike
  during compaction. A sustained climb is [backpressure](backpressure).
- **Segment seal rate** — sealed segments per hour. Tells you how fast the ledger is growing and
  therefore when you will run out of disk; see
  [capacity-planning.md](guides/operations/capacity-planning.md).
- **Rejected envelopes** — should be zero. A nonzero rate is almost always an unregistered
  collector or a clock outside the accepted skew.
- **Compaction lag** — how far behind the compactor is. Explained in
  [compaction](internals/ledger/compaction.md).

## When something moves

The runbooks are the answer to "it moved, now what":

- [Draining the queue](guides/operations/runbooks/queue-drain.md)
- [Rotating a certificate](guides/operations/runbooks/certificate-rotation.md)
- [Failing over to another region](guides/operations/runbooks/regional-failover.md)

Each is written to be followed by someone who did not write it, at three in the morning. If one
of them is not, fix it: [writing a runbook](guides/operations/runbooks/writing-a-runbook.md) says
what the shape is.

## What not to do

Do not delete a segment to reclaim disk. Segments are the ledger; the compactor is the only
thing that may remove one, and it will not remove one a tap could still be reading. If you are
out of disk, you are in [capacity planning](guides/operations/capacity-planning.md) territory,
and the answer is a bigger disk or a shorter retention window.

## Vocabulary

Segment, generation, tap, compaction lag: all four are in [the glossary](glossary.md), and this
page uses them in exactly that sense. The numbers themselves are listed once,
[above](#the-four-numbers).
