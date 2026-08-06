# `quillon tap`

Read the ledger back. A tap is read-only and holds a segment open while it runs, which is why a
forgotten tap can stop the compactor reclaiming disk.

    quillon tap [--collector NAME] [--metric NAME] [--from POSITION] [--follow]

## Positions

- `earliest` — the oldest reading still inside the retention window.
- `latest` — the next reading to arrive; useful only with `--follow`.
- An instant — the first reading at or after it.

## Output

One reading per line: instant, collector, metric, value, unit. The unit is printed because it is
part of the metric's identity, as [the glossary](glossary.md) insists.

## Following

`--follow` streams as readings land. It is the fastest way to answer "is anything arriving at
all", which is step 6 of [first-gateway.md](guides/first-gateway.md).

## The cost

A tap reads sealed segments directly. Against a long retention window and a broad filter it will
read the whole ledger, so filter by collector when you can. Never leave `--follow` running in a
detached session; see the disk warning in
[capacity-planning.md](guides/operations/capacity-planning.md).
