# Dogtag product strategy

> **Status: beta hypothesis map, 2026-07-29; M0 packet folded in 2026-07-30.** Evidence and decisions accumulate in the maintainers' planning record; the product repository's ADR trail is `docs/adr/`; this document owns the experiment sequence beyond the build.

## Strategic premise

Dogtag enters through an operational layer for structured, versioned knowledge: agents and applications can safely understand and change a vault without recreating its schema, identity, relationships, provenance, validation, or migration rules.

The first product is local software. Dogtag hosts documentation and release artifacts, not users' vaults. The SDK is primary; CLI, MCP, TUI, webhooks, CI jobs, and agent loops are consumers.

## Riskiest hypotheses

1. **Recurring value:** agent-operated maintenance produces enough repeated value to justify setup and changed habits.
2. **Portability:** one semantic kernel can support materially different vaults through configuration rather than product forks.
3. **Embeddability:** developers can build useful workflows directly with the SDK instead of reproducing vault semantics or parsing CLI output.
4. **Adoption:** an existing Markdown corpus can gain an operational layer without surrendering its structure.
5. **Trust:** preview, provenance, validation, scope, and recovery make agent mutation acceptable for important notes.
6. **Wedge:** one initial audience and job communicates and retains materially better than a general "PKM SDK" offer.

## Experiment ladder

### E0 — two-vault released beta

Two maintainers install the same prerelease and use it against independently configured vaults through both CLI and embedded SDK operation.

Evidence: setup time, config differences, manual interventions, rejected mutations, upgrade failures, recurring workflows, and places where a personal convention masquerades as an invariant.

Gate: the [beta ship test](BETA.md#ship-test) passes.

### E1 — assisted existing-vault adoption

Three to five technical Markdown and agentic-CLI users bring existing corpora. A guided session installs Dogtag, maps the corpus to configuration, and leaves one recurring read workflow and one safe write workflow.

The first participant is already identified and already operates a sophisticated non-Dogtag knowledge system of their own — a domain record taxonomy with immutable source originals, evidence trails, and its own agent contract. That combination makes them an E1 subject rather than an E0 maintainer: E0 tests whether founders can run the product on vaults they built for it, and adoption of an existing corpus is a materially different question. Folding them into E0 would fuse the two, so that a failure could not be attributed to either the SDK or the guided-adoption process. Their system's shape is the source for the `records` fixture profile, which exists so that their onboarding is rehearsed against a fixture before it is attempted against their corpus.

Evidence: time to first value, content accepted without destructive normalization, interventions, trust failures, one- and four-week retention, and configuration unique to each vault.

Gate: at least two non-founder vaults retain a recurring workflow and recover from ordinary failures without maintainer surgery.

### E2 — focused starter use case

Test one opinionated starting configuration against the broad bring-your-own-vault offer: research memory, repository-adjacent engineering knowledge, meeting/decision memory, or another job selected from E1 evidence.

Evidence: onboarding time, activation, repeated use, referral language, and whether the focused promise is easier to understand and retain.

Gate: one wedge shows materially stronger pull than the general offer.

### E3 — technically sponsored user

A technical sponsor establishes the vault; the beneficiary primarily captures and asks questions conversationally.

Gate: the beneficiary receives recurring value without learning repository operations, while the sponsor can diagnose and recover the system.

### E4 — small shared vault

Two or more people operate one vault through different editing and agent surfaces.

Gate: scoped writes, attribution, synchronization, and conflict recovery preserve trust under real concurrent use.

## Stage gates

| Stage | Evidence required |
| --- | --- |
| Extract product repository | Product decisions closed; beta contract, minimum domain, and first fixture defined |
| First prerelease | Installable artifact and versioned release path exist, even before useful operations |
| Founder beta | E0 ship test passes |
| Private beta | E1 retention and recovery gate passes |
| Focused beta | E2 identifies a stronger wedge |
| Public release | Unassisted install, migration, compatibility, privacy, documentation, and support expectations are stable |

## Distribution posture

- `dogtag.dev` is owned and is the canonical product and documentation domain.
- An existing logo is the starting brand asset; the beta does not reopen identity design.
- The site begins as install, concepts, SDK guides, compatibility, troubleshooting, and release notes, and fronts the install script.
- Product marketing is intentionally light until E1/E2 clarify the audience and promise.
- Release artifacts are hosted for direct installation even though the product has no hosted runtime.
- **The beta is Apache-2.0 from a public repository.** The licensing choice is deliberately the permissive one despite closing off the paid branch of the product question earlier than the evidence requires: an SDK whose entire premise is that other people embed it is not helped by a license its intended audience has to think about, and the audiences most worth reaching are the ones whose employers vet licenses. The paid question, if it returns, returns as something built beside the SDK rather than as a retraction of it.
- **Release automation is rented, and reversibly so.** Tagged builds run on hosted runners, which are free for public repositories and would cost single-digit dollars across a whole beta even if they weren't. Self-hosting the whole pipeline is a live fallback — a self-hosted forge already runs, and a build runner registered on a maintainer's own macOS machine covers the one platform a Linux host cannot cross-compile for. The fallback is deliberately not stood up in advance, because a young single-crate repository is cheap to move and standing up unused infrastructure is not evidence about anything. Triggers to move: hosted CI stops being free for public repositories, the terms change unacceptably, or the beta verdict favors a fully self-hosted posture.

## Signals to change course

- Conformance scenarios cannot be expressed against more than one fixture profile without special-casing. This is the leading indicator of the next signal, and it arrives long before a real vault does — which is the reason the no-waiver rule exists rather than an exemption list.
- Two real vaults require semantic forks rather than configuration.
- Embedded consumers cannot use the SDK without depending on CLI-only behavior.
- Mutation remains too risky despite preview, validation, provenance, and recovery.
- Assisted users value search or file organization once but do not retain an operational workflow.
- Maintaining bindings and compatibility costs more than the small semantic kernel earns.
