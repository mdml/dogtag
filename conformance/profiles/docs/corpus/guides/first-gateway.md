# Bring up your first gateway

About twenty minutes, assuming the gateway is already installed. If it is not, start at
[installation](guides/installation.md).

At the end of this you will have a gateway accepting one collector, and a reading you can find
in the ledger by hand.

## 1. Choose a ledger directory

Anything writable, on a disk you are willing to fill. The gateway creates the directory tree on
first start and refuses to reuse a directory that holds a ledger from a different region.

## 2. Write the smallest configuration that works

    [ledger]
    path = "/var/lib/quillon/ledger"

    [listen]
    ingest = "0.0.0.0:9700"

Everything else has a default, and the defaults are documented in
[reference/configuration.md](reference/configuration.md). Resist the urge to set more of them
now; the ones that matter will announce themselves.

## 3. Start it and watch the first line

The gateway logs one line on a clean start: the region name, the ledger generation, and the
segment it will append to. If it logs anything else, the message is in
[reference/api/errors.md](reference/api/errors.md).

## 4. Register a collector

Collectors are registered before they connect, so an unknown identity is a rejection rather than
a silent new device. Register one with the CLI:

    quillon fleet register --name bench-1

The full command is in [reference/cli/quillon-fleet.md](reference/cli/quillon-fleet.md).

## 5. Send one envelope

Anything that speaks the ingest protocol will do; the shape is in
[envelope](reference/api/v2/schemas/envelope.md). The collector will be told its sequence number
was accepted, and the gateway will have written it before it says so.

## 6. Find it again

    quillon tap --collector bench-1 --from earliest

If the reading is there, the whole path works. If the tap prints nothing, the reading did not
land, and the gateway will have said why — taps never lie about an empty ledger.

## Next

[guides/operations/overview.md](guides/operations/overview.md) is what to do with a gateway that
now has to stay up.

Every word in this page that sounds like jargon is defined in [the glossary](glossary.md).
