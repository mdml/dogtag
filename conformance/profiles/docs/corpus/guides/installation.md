---
type: guide
title: Install the gateway
summary: Packages, containers, and the single-node case.
tags:
  - area/install
  - audience/operator
reviewed_on: 2026-06-02
updated: 2026-06-02
---

# Install the gateway

Three supported shapes: a system package, a container image, and a single static binary for the
one-node case. All three carry the same gateway; they differ in what supervises it.

## System package

Packages are published to [the package repository](https://packages.quillon.example/stable) for
the two most recent stable releases of the supported distributions.
The package installs the binary, a service unit, and an empty configuration file at
`/etc/quillon/gateway.toml`. It does not start the service — a gateway with no configured ledger
path will refuse to start, and failing at install time reads like a broken package.

## Container image

The image expects the ledger on a mounted volume. Everything else is passed as environment, and
every environment variable maps to exactly one configuration key; the mapping is in
[reference/configuration.md](reference/configuration.md).

Give the volume its own disk if you can. The ledger's write pattern is sequential and its
compaction pattern is not, and sharing a spindle with anything else is the most common cause of
the backpressure described in [backpressure](backpressure).

## Single binary

For a laboratory bench or a single-site deployment, the static binary needs only a writable
directory:

    quillon-gateway --ledger ./ledger --listen 0.0.0.0:9700

There is no supervisor, no log rotation and no upgrade path in this shape. It is for trying
things out; do not run a fleet on it.

## Afterwards

Go on to [first-gateway.md](guides/first-gateway.md), which takes an installed gateway and gets
a reading into the ledger. If you are replacing an existing deployment, read
[upgrading](guides/upgrading.md) first — the sidecar protocol was removed in 3.1 and a 2.x
collector will not connect.
