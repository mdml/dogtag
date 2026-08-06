# Quillon

Quillon is a fleet telemetry gateway. Collectors on your hardware speak to it over a single
long-lived stream; it batches, deduplicates and writes what they say into an append-only
ledger, and everything downstream reads the ledger rather than the devices.

These are the docs that ship with the gateway. They are organized by what you are trying to
do, not by which team wrote them.

## Start here

- [Install the gateway](guides/installation.md) — packages, containers, and the one-node case.
- [Bring up your first gateway](guides/first-gateway.md) — from a fresh install to a reading
  landing in the ledger, in about twenty minutes.
- [Upgrading](guides/upgrading.md) — what changes between minor versions, and what does not.

## The rest of the tree

- [guides/README.md](guides/README.md) — task-shaped prose: operations, integration, runbooks.
- [reference/README.md](reference/README.md) — configuration keys, limits, the CLI, the HTTP API.
- [decisions/README.md](decisions/README.md) — why the gateway is shaped the way it is.
- [internals/architecture.md](internals/architecture.md) — the parts, and how they fail.
- [contributing/docs-style.md](contributing/docs-style.md) — how to write a page that fits here.

Words that mean something specific in these pages — *collector*, *envelope*, *reading*,
*stream*, *ledger* — are defined once in the [glossary](glossary.md) and used that way
everywhere.

## Versions

These pages describe 3.1. The [3.1 release notes](releases/2026-07-quillon-3-1.md) list what
changed; [3.0](releases/2026-05-quillon-3-0.md) is the version before it, and the last one that
accepted the old sidecar protocol.

## Reporting a problem

Open an issue against the gateway repository. If the gateway is refusing traffic right now,
start with [guides/operations/runbooks/queue-drain.md](guides/operations/runbooks/queue-drain.md)
rather than with an issue.
