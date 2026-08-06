---
type: decision
title: Freeze the envelope schema for a major version
summary: No field is added, removed, or reinterpreted inside a major version.
decided_on: 2026-02-09
alternatives:
  - Additive-only evolution with ignored unknown fields
  - Versioned envelopes negotiated at handshake
reversibility: one_way
tags:
  - area/ingest
updated: 2026-02-09
mentions: reference/api/v2/schemas/envelope.md
---

# Freeze the envelope schema for a major version

**Decided 2026-02-09. Reversibility: one way.**

The envelope in
[reference/api/v2/schemas/envelope.md](reference/api/v2/schemas/envelope.md) does not change
inside 3.x. An unknown field is a refusal, not something ignored.

## Why

Collectors live on hardware and are upgraded on hardware's schedule — years, not weeks. A
schema that evolves under them means every gateway must accept every historical shape forever,
which is the same commitment as freezing but without anyone having written it down.

Refusing unknown fields rather than ignoring them is the other half. An ignored field is an
integration that appears to work and silently drops the thing the integrator most cared about;
we would rather fail at the first envelope.

## What was rejected

**Additive-only with ignored unknowns.** Rejected for the reason above.

**Negotiated envelope versions.** Rejected on the fleet's arithmetic: a negotiated version is
one more thing that can be wrong on fifty thousand devices, and the negotiation itself becomes a
compatibility surface.

## What it costs

Anything genuinely new waits for 4.x, or goes in `metadata`, which is stored and never
interpreted. Two proposals have already gone into `metadata` and neither has come out.
