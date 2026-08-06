# Collectors

A collector is an identity, not a device. Two processes on one machine are two collectors; one
process moved to another machine is still one collector.

## Register

    POST /v2/collectors
    { "name": "bench-1", "metadata": { "site": "north" } }

Answers the identity and its credential. The credential is shown once. Names are unique within
a region and are never reused after retirement — a reused name would silently join two devices'
histories in the ledger.

## List

    GET /v2/collectors?state=active

`state` is `active`, `retired`, or absent for both. The listing is paginated and the page cursor
is opaque.

## Retire

    DELETE /v2/collectors/{name}

Stops the identity being accepted. It does not remove anything from the ledger; retention alone
decides how long readings live, as [reference/limits.md](reference/limits.md) says.

## Errors

`admin.name-taken` and `ingest.unknown-collector` are the two you will meet; both are in
[errors](reference/api/errors.md).

## See also

- [streams](reference/api/v2/streams.md), which every registered collector then opens.
- [guides/integration/overview.md](guides/integration/overview.md) for which door to use.

A collector is an identity and not a device, as [the glossary](glossary.md) insists at length.
