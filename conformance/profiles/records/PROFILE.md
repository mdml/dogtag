# Fixture profile: `records`

**Stands for:** the decision maker.
**Corpus:** scheduled, built before assisted adoption begins (pre-E1).

## Distinguishing axes

- **A dense domain record taxonomy.** Many record types specific to one working domain — decisions, filings, correspondence, meeting records — each with its own required properties. Unlike `dense`'s broad personal taxonomy, the density here is vertical: one domain, deeply typed.
- **Immutable originals under closed-write.** Source documents that must never be modified are declared closed-write, so immutability is a policy on a class of types rather than a privileged directory. This is the profile that exercises the closed-write capability for real: scenarios touching write policy must be expressible here without naming any record type.
- **Evidence trails.** Working notes cite the immutable originals they rest on through typed relationships, forming chains from conclusion back to source. This stresses relationship resolution and dangling-reference diagnostics where a broken link is not a cosmetic defect but a broken chain of evidence.

## What the fixture is

A corpus whose shape follows a real decision-records practice: a record taxonomy, closed-write originals, and citation chains. The fixture is built in coordination with its source's author before assisted adoption; attribution is recorded when the corpus lands.

## Why it is not built yet

The committed vault-contract format is an M2 decision, and this profile's hypothesis — that a stranger's record-keeping discipline fits the configuration seam without any personal convention entering the kernel — is exactly what assisted adoption (E1) tests. Building it in coordination with its source, immediately before that experiment, is the point; specifying it now keeps every earlier scenario honest about the axes it must survive.
