---
type: guide
title: Upgrading
summary: What changes between minor versions, and what never does.
tags:
  - area/install
  - audience/operator
needs_review: true
reviewed_on: 2026-05-19
updated: 2026-07-14
---

# Upgrading

The compatibility promise is narrow and worth stating before the steps: **the envelope schema
does not change within a major version, and the ledger format does not change at all.** Anything
else may.

## Before you start

Read the release notes for every version you are skipping, not only the one you are landing on.
[3.1](releases/2026-07-quillon-3-1.md) removed the sidecar protocol that
[3.0](releases/2026-05-quillon-3-0.md) had merely deprecated, and a fleet that jumped from 2.9
straight to 3.1 would meet that removal without ever seeing the warning.

## The order

1. Upgrade one gateway in the region and leave it for a full compaction cycle.
2. Watch the queue depth and the segment seal rate. Both are in
   [guides/operations/overview.md](guides/operations/overview.md).
3. Upgrade the rest of the region.
4. Upgrade collectors last. A new gateway accepts an old collector within the major version;
   the reverse is not true.

## Rollback

Downgrade the gateway binary and restart. The ledger is forward-compatible by construction —
segments are sealed and never rewritten — so a downgraded gateway reads everything a newer one
wrote. What does not roll back is configuration: a key introduced in the newer version is
unknown to the older one, and an unknown key is a refusal to start rather than a warning. Keep
the old configuration file.

## What to do if it goes wrong mid-upgrade

If the gateway is up but refusing traffic, that is backpressure and not a failed upgrade; see
[backpressure](backpressure). If it will not start at all, the message will be one of the
startup refusals in [errors](reference/api/errors.md).

If you are new to the tree, [the top-level README](README.md) says where everything else is,
and the rollback story is [above](#rollback).
