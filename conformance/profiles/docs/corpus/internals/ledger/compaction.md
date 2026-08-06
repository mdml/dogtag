# Compaction

Compaction removes readings that have fallen out of the retention window. It never edits a
segment: it writes a new one and deletes the originals afterwards.

## The cycle

1. Pick sealed segments whose newest reading is older than the window.
2. Write a replacement holding whatever in them is still inside it — usually nothing, which
   makes the replacement empty and the step free.
3. Fsync the replacement.
4. Delete the originals, unless a tap holds one open.

Step 4 is why a forgotten `--follow` in
[reference/cli/quillon-tap.md](reference/cli/quillon-tap.md) fills a disk.

## Lag

Compaction lag is the age of the oldest segment that should have been compacted and has not
been. Sustained lag means the disk has no headroom for step 2, which is the second of the three
causes in [backpressure](backpressure).

## Why write-then-delete

Because the alternative — delete-then-write — has a window in which the readings exist nowhere.
The cost is the 1.15 headroom multiplier in
[capacity-planning.md](guides/operations/capacity-planning.md), and it is the reason a full disk
stops compaction instead of being fixed by it.

## Interaction with recovery

Compaction does not run until start-up recovery has finished. A truncated tail and a compaction
pass at the same time would be two things editing the ledger's shape at once, which the
single-writer rule exists to prevent; see [recovery](internals/ledger/recovery.md).

## Vocabulary

Segment, sealed, retention window: [the glossary](glossary.md).
