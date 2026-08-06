# Fail over to another region

**Severity: emergency.** This is a manual procedure on purpose. The gateway does not fail over
by itself, and the reasoning — that an automatic failover between two append-only ledgers can
only ever guess which one is authoritative — is in
[decisions/0004-regional-failover-is-manual.md](decisions/0004-regional-failover-is-manual.md).

Read the whole page before starting step 1.

## What you are about to do

Move a fleet of collectors from region A to region B. Region B has its own ledger, which does
not contain region A's readings and never will. You are choosing to continue collecting rather
than to preserve continuity of the record.

## Steps

1. **Decide, and write down the time.** Everything after this is reconstructable only if you
   know the instant it began.
2. **Stop region A's ingest listener** if it is reachable. A half-reachable region A that some
   collectors can still see is the worst outcome available.
3. **Retarget the fleet.** Collectors take their gateway address from their own configuration;
   how you push that is your fleet management, not this gateway's.
4. **Confirm arrivals in region B.**

        quillon fleet status --region b

5. **Leave region A alone.** Do not delete its ledger. It is the only copy of everything it
   accepted before step 2.

## Afterwards

The two ledgers are now siblings with a gap between them. Reconciling them is an offline job
against sealed segments, described in [recovery](internals/ledger/recovery.md). Do not attempt
it while either region is accepting traffic.
