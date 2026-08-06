# The command-line tools

Two commands, one for the fleet and one for the ledger.

- [reference/cli/quillon-fleet.md](reference/cli/quillon-fleet.md) — register collectors,
  quiesce, resume, reload, and ask what the gateway thinks is happening.
- [reference/cli/quillon-tap.md](reference/cli/quillon-tap.md) — read the ledger back.

Both talk to the admin listener, whose address is the `admin` key in
[configuration](reference/configuration.md). Neither has a way to write a reading; the only door
into the ledger is ingest, which is deliberate.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Did the thing. |
| 1 | Refused; the reason is on stderr. |
| 2 | Could not reach the admin listener. |

Anything that refuses prints one of the identifiers in
[errors](reference/api/errors.md), so a script can branch on the identifier rather than on the
prose.

## Vocabulary

Both commands print collector names and metric identities; both are defined in
[the glossary](glossary.md).
