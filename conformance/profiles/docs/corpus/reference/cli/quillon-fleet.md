# `quillon fleet`

Fleet and lifecycle operations against one gateway.

## `register`

    quillon fleet register --name NAME [--metadata KEY=VALUE]...

Creates a collector identity and prints its credential once. The credential is not stored in a
form the gateway can print again; losing it means re-registering. The identity model is in
[collectors](reference/api/v2/collectors.md).

## `status`

    quillon fleet status [--watch] [--tls] [--region NAME]

Queue depth, segment seal rate, rejected envelopes, compaction lag — the four numbers
[guides/operations/overview.md](guides/operations/overview.md) says to watch. `--watch` redraws
every second.

## `quiesce` and `resume`

    quillon fleet quiesce
    quillon fleet resume

Stop and start accepting. Quiescing refuses collectors with a retry hint and loses nothing;
[queue-drain.md](guides/operations/runbooks/queue-drain.md) is the procedure these two exist for.

## `reload`

    quillon fleet reload

Rereads exactly the keys marked reloadable in
[configuration](reference/configuration.md#listen). Everything else is ignored until a restart,
silently — a wart that has been logged and not yet fixed.

## `retire`

    quillon fleet retire --name NAME

Marks a collector as gone. Its readings stay in the ledger; retirement only stops the identity
being accepted again.
