# Rotate the ingest certificate

**Severity: routine.** Do this before expiry, not after. Collectors pin the issuer, not the
leaf, so a rotation within the same issuer is invisible to them.

## Before

- The new leaf and its chain, on disk, readable by the gateway user.
- A maintenance window is *not* required. This rotation does not drop connections.

## Steps

1. Place the new pair beside the old one. The paths are configuration keys, listed in
   [reference/configuration.md](reference/configuration.md).
2. Point the keys at the new pair.
3. Reload:

        quillon fleet reload

   Reload rereads the certificate and nothing else. It does not reread the ledger path, and a
   changed ledger path in the same edit will be silently ignored until a restart — which is the
   most common way this goes wrong.

4. Confirm the served leaf changed:

        quillon fleet status --tls

## If the issuer changed

Then collectors will not connect, because the pin is on the issuer. That is a fleet-wide
collector configuration change and a much larger operation than this page; treat it as a
migration and stage it the way [upgrading](guides/upgrading.md) stages a version.

## Afterwards

Remove the old pair once `status --tls` has shown the new one for a full day. Leaving both in
place is how a rotation gets reverted by an unrelated restart.
