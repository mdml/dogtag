# Decisions

One decision per page, numbered, dated, and never edited after acceptance. A decision that turns
out wrong is superseded by a new one; the old page moves to `superseded/` and keeps its number.

## Accepted

- [0001 — a single-writer ledger](decisions/0001-single-writer-ledger.md)
- [0002 — drop the agent sidecar](decisions/0002-drop-the-agent-sidecar.md)
- [0003 — freeze the envelope schema](decisions/0003-freeze-the-envelope-schema.md)
- [0004 — regional failover is manual](decisions/0004-regional-failover-is-manual.md)

## Superseded

- [0000 — shard by tenant](decisions/superseded/0000-shard-by-tenant.md), superseded by 0001.

## Writing one

State the decision in the title, in the past tense, as a thing that was decided. Record what was
considered and rejected — the alternatives are the part future readers need, because the chosen
option is visible in the code and the rejected ones are not.

Link the pages the decision constrains. [internals/architecture.md](internals/architecture.md)
is usually one of them.

The placement rule these pages obey — the folder is the classification — is stated in
[CONTRIBUTING.md](CONTRIBUTING.md).
