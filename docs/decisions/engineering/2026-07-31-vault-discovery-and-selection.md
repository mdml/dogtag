# Vault-root discovery and vault selection

- Status: accepted (amended 2026-08-01 — see [Amendments](#amendments))
- Date: 2026-07-31

## Context

M2 delivers vault-root discovery ([beta.md](../../beta.md#milestones)). Discovery is the operation every later one depends on: if it resolves the wrong root, every subsequent read, diagnostic, and eventually every write acts on the wrong corpus while reporting success. It also has to satisfy two standing positions that pull in different directions. [architecture.md](../../architecture.md) makes *identity is the path* a load-bearing invariant, and the [settings PDR](../product/settings.md) rejects a singleton current vault outright — *"current-directory discovery is a useful CLI convenience but a poor SDK foundation"* — while the CLI's most common invocation is exactly current-directory discovery.

## Decision

### The sentinel and the search

- **The sentinel is the file `.dogtag/contract.toml`**, not the `.dogtag/` directory.
- **Discovery walks upward from an explicit starting directory to the filesystem root**, stopping at the first directory holding the sentinel. There is no other boundary: not `.git`, not `$HOME`, not a mount point.
- **A directory holding `.dogtag/` but no `.dogtag/contract.toml` halts the walk with an error.** It is a broken vault root, not a non-root.
- **Finding nothing is a deterministic error** naming the starting directory and the sentinel that was sought, under an identifier distinct from the broken-root case, so a caller can tell "you are not in a vault" from "your vault is damaged."

Halting on an incomplete root is the decision with the sharpest failure mode behind it. If the walk continued, a half-initialized vault would silently delegate to its parent, and every operation would act on the wrong corpus while reporting success — the worst outcome available here. Continuing costs nothing visible; halting costs a confusing error in the rare case where a stray `.dogtag/` sits above a real vault, and that error names the exact directory.

Refusing `.git` as a boundary follows from `architecture.md` putting the version layer behind a seam: git is *one implementation* of it, and making `.git` a discovery boundary would hardcode git into the kernel's most basic operation and refuse to find a vault that has no remote yet — which the README explicitly permits. `$HOME` is arbitrary and fails for a vault under `/data` or a mounted share. One rule with no exceptions is also the only version that stays explainable.

### Symlinks

**The starting path is canonicalized before the walk, the walk is physical, and the resolved root is reported canonically.** Every path the SDK emits is relative to that canonical root.

This follows from path identity rather than from taste. A vault reachable at both `/data/vaults/work` and `~/work` would otherwise give one note two identities depending on the route taken, and every link, diagnostic, and eventual index key would fork along with it. When the resolved root differs from what was requested, `doctor` emits an info-severity diagnostic saying so — the surprise is real and the answer is to make it visible rather than to avoid it.

Symlinks *inside* a vault — a note that is a link, a link escaping the root — are corpus-traversal questions that M2 does not reach, and are deferred to the milestone that enumerates notes.

### Nested vaults

**Nearest root wins, and the walk always continues to the filesystem root.** When an ancestor directory also holds a contract, the resolved root is still the nearest one, and discovery emits a warning-severity diagnostic naming the ancestor. Nesting is legal — a vendored vault, a vault checked out inside another — so it is deterministic and visible rather than refused.

Completing the walk after the sentinel is found is what makes that warning possible at all, and it is stated here because the alternative is worse: if `doctor` performed a second private walk to find the ancestor, there would be two discovery implementations, and every other consumer would silently lose the warning.

### Trust

A discovered contract is trusted exactly as far as the directory tree it was found in. With no boundary on the walk, that tree is not always the user's.

The accidental case is mild — running inside an unrelated repository nested under a vault finds the vault, which the resolved-root line makes visible. The adversarial case is not, and it is live at M2 rather than deferred to the mutation milestone. `contract explain` renders the contract as **instructions an agent follows**; every string value in a valid contract is free text. So a `.dogtag/contract.toml` planted in any ancestor of where dogtag runs — in an extracted archive or cloned repository whose top level carries one, in a world-writable directory on a shared host, on a network mount — becomes attacker-authored text presented to an agent as the vault's rules. Fatal unknown keys bound the structure an attacker can inject; they bound nothing about the content.

Nothing here is a write or an exfiltration at M2: `doctor` is read-only and opens no note. The blast radius is whatever the consuming agent is permitted to do. Two mitigations, both cheap and neither requiring a boundary:

- **A warning-severity diagnostic when the resolved root is outside the user's home directory or is group- or world-writable.** It costs one `stat`, changes no resolution rule, and preserves the one-rule-no-exceptions property.
- **`contract explain`'s Markdown names the resolved root in its preamble**, so an agent consuming the rendering receives the provenance with the instructions. Printing the root to a terminal protects a human reading it; it does nothing for an agent reading piped output.

The walk also crosses mount boundaries by design, so a hanging network mount above the working directory stalls discovery — an availability cost on exactly the network-hosted vault this record declines to exclude.

### Selection

Vault selection is the CLI's, and resolves in one order: **`--vault`, then `DOGTAG_VAULT`, then upward discovery from the current directory.** Every command reports the root it resolved and how.

`--vault` and `DOGTAG_VAULT` take either a registered name or a path, distinguished **syntactically, with no fallback between them**:

- An argument containing a path separator, or beginning with `.`, `/`, or `~`, is **always a path**. It is used exactly, never searched upward: explicit means explicit.
- Any other argument is **always a registered name**, looked up in the installation record's registry. An unregistered name is an error; it never falls back to being a path.

The no-fallback rule is what makes this deterministic. *Path-if-it-exists-else-name* would resolve `--vault work` differently depending on whether a `work` directory happens to sit in the current directory, so the same command would select different vaults from different places. *Name-first-else-path* would let registering a vault named `docs` silently change what `--vault docs` means in every existing script on the machine — a machine-local file reinterpreting an invocation, which the settings PDR forbids. With no fallback, the argument's own syntax fixes its meaning, and registering a vault can never change what an existing invocation resolves to.

Two supporting rules follow. Registry names are kebab-case and contain no path separators, and **duplicate names are a load error on the installation record** rather than a silently-resolved ambiguity — which is also what forecloses shadowing an existing name by appending an entry, not merely a determinism nicety. A registered name whose path is absent or is not a vault root is an error citing the registry entry as related evidence — never a fall-through to discovery. Registry paths are absolute, with no tilde or environment expansion, so a registry entry cannot resolve differently from different directories.

**Registration never implies selection.** There is no default vault and no "current vault" state, so multiple registered vaults cannot produce ambiguity: the registry answers only questions that name an entry.

### The SDK/CLI boundary

**Every SDK entry point here is a pure function of explicit arguments.** None reads an environment variable, consults the current directory, or holds process-global vault state. The CLI resolves argv, environment, and the current directory into explicit arguments before calling in.

Three entry points, not one, because one cannot express the decisions above:

- **`discover(start)`** walks upward and returns the resolved root **together with the diagnostics discovery itself produced** — the symlink-resolution note and the nested-vault warning both arise on a *successful* discovery, so a bare success value has nowhere to carry them.
- **`root_at(path)`** verifies that an exact path is a vault root, without walking. This is what `--vault <path>` calls; without it the CLI would either walk (violating explicit-means-exact) or perform the sentinel check itself (putting kernel knowledge in a consumer).
- **`resolve_registered(name, record)`** maps a registry name to a path against an explicitly supplied installation record. Registry resolution lives in the SDK rather than the CLI because its failures — an unknown name, an entry whose path is not a vault root, duplicate names — are `installation.*` diagnostics, and only the SDK may mint an identifier outside the `ext.` namespace. Taking the record as an argument keeps the no-ambient-state rule intact: the CLI still decides *which* record, using `XDG_CONFIG_HOME`.

**`VaultRoot` is an opaque type, not an alias for a path.** It renders as a path and is compared as one, but callers do not reconstruct it from the rendered string. That costs nothing now and preserves the option of carrying a held directory handle later — the difference between re-resolving a write target from a string and writing through the handle that was verified, which is the gap between canonicalization and use that only matters once there are writes. Retrofitting it after `VaultRoot` is public API would be a breaking change.

This is what keeps *"the SDK is the kernel"* true for the one operation most tempted toward ambient state, and it is what makes discovery testable exhaustively against synthetic trees rather than against whatever directory a test happens to run in.

### Alternatives considered

- **Treating `.dogtag/` as the sentinel.** Rejected: an empty or half-written `.dogtag/` would then shadow a real parent vault, or — if it did not — the same directory would mean two different things depending on its contents.
- **Continuing the walk past an incomplete root.** Rejected as reasoning above: silent delegation to the wrong corpus.
- **Stopping at a `.git` directory.** A familiar mental model, and it prevents cross-repository surprises. Rejected: it hardcodes one version layer into discovery and refuses vaults that are not repositories.
- **Stopping at `$HOME`.** Rejected: arbitrary, and simply inapplicable to vaults outside it.
- **Walking the logical path and preserving the symlink route.** Friendlier output. Rejected: it gives one note two identities.
- **Canonicalizing only the reported root while walking logically.** Rejected: the search and the result would describe different trees, which makes the resulting diagnostics unexplainable.
- **Refusing nested vaults outright.** Rejected: nesting is legal, and refusing it would make a vendored vault unreadable to serve a tidiness preference.
- **`--vault` as a path only, with the registry never selecting.** This was the initial recommendation. Rejected once the no-fallback rule removed the ambiguity that motivated it: with syntax fixing meaning, a registered name is unambiguous, useful, and free.
- **A sigil for registry names (`--vault @work`).** Rejected: it imposes ceremony on the common case and `@work` is itself a legal relative path, so it needs its own escape rule anyway.
- **Separate `--vault` and `--vault-name` flags.** Rejected: two flags for one concept, and every consumer must handle both.
- **Falling back to a registered default when outside any vault.** Rejected: it reintroduces the singleton current vault the settings PDR ruled out, and makes provenance ambiguous the moment results are aggregated.

## Consequences

- **From inside an unrelated repository nested under a vault, discovery finds the vault.** That is the accepted cost of having no boundary; the mitigation is that every command prints the root it resolved, which makes the surprise a one-line read rather than a mystery.
- **A user who organizes vaults through symlinks sees paths they did not type.** The info diagnostic explains it once per invocation, which is noise for anyone who set it up deliberately.
- **`--vault work` cannot mean the directory `./work`.** It must be written `./work`. The error for an unregistered bare name has to teach this, because the mistake is easy and the correction is not guessable.
- **Discovery is exhaustively testable**, since it takes an explicit path. Every branch — nested, incomplete, absent, symlinked, competing — is reachable against a synthetic tree, which matters directly for the 100% kernel coverage floor. The no-vault-found case is the exception that needs care: because the walk has no boundary, "a directory with no vault above it" is a property of the *machine*, not of a fixture. The conformance harness therefore needs a root it controls, the way `XDG_CONFIG_HOME` already gives it a hermetic installation record; otherwise that scenario's outcome depends on whose checkout it runs in, which is one developer's directory layout reaching a conformance result.
- **The adversarial cases above are acknowledged rather than closed.** The two mitigations reduce a planted contract to something visible, not to something impossible, and no boundary rule would close it either — this record's whole argument is that every candidate boundary is worse. Anyone extending discovery should treat the trust sentence as the constraint, not the walk.
- **The registry gains a real job at M2** (name resolution) beyond being reported, which means the installation record's parse and validation rules are load-bearing from the first release that reads it.

## Amendments

The Decision above stands as written; these later records change parts of it, and the original text is left intact so the change is legible.

- **2026-08-01 — halting on an incomplete root applies only while no root has been resolved.** The Decision states without qualification both that a directory holding `.dogtag/` but no contract halts the walk, and that the walk always continues to the filesystem root. For a broken root sitting *above* an already-resolved one the two demand opposite behaviour, and the implementer had to choose. The choice: halt only while nothing has been resolved; above a resolved root, an incomplete directory changes nothing and the nearest root still wins. This is the reading that keeps a stray `.dogtag/` in a shared ancestor from denying service to every vault beneath it. The same rule governs a directory the filesystem refuses to probe, which no record mentioned at all and which reports `discovery.path-unreadable`.

- **2026-08-01 — an empty vault selector is refused, from either source.** Neither this record nor any other says what `--vault ""` or `DOGTAG_VAULT=` means. The CLI answered the same empty value two ways: the variable read as unset and fell through to discovery, the flag became a registry name and errored. The silent one was the dangerous one — an unfilled slot in a CI or cron template made `dogtag doctor --strict` inspect whatever vault the working directory sat in and exit 0, which is the wrong-vault resolution `--strict` exists to catch. An empty selector from either source is now a usage refusal, exit 2: it names no vault, so there is nothing to diagnose. `NO_COLOR` keeps the empty-as-unset convention it is named for.

- **2026-08-01 — the purity invariant needs the caller to pass an absolute path, and now says so.** The Decision says every SDK entry point here is a pure function of explicit arguments and reads no ambient state. `fs::canonicalize` resolves a relative argument against the *process* working directory, so the invariant held only for callers that already passed absolute paths. The CLI now joins a relative `--vault` onto the current directory it resolved. The invariant as stated is a requirement on the caller, and any embedder passing a relative path is reading process state whether it means to or not.

- **2026-08-01 — `VaultRoot`'s opacity does not preserve the option it was bought for.** The Decision forbids reconstructing a `VaultRoot` from its rendered string, to keep open the option of carrying a held directory handle later. The prohibition holds — there is no `Display`, `FromStr`, `From` or serde impl. But the type exposes `path()` and `contract_path()`, and every consumer re-resolves from a raw path, so introducing a handle later would change no consumer's behaviour and the breaking change the opacity was meant to avoid is still owed. The rule closes a door nobody was walking through.

- **2026-08-01 — the trust analysis does not cover a planted *incomplete* root.** The Trust section reasons carefully about a `.dogtag/contract.toml` planted in an ancestor and supplies two mitigations. Planting a bare `.dogtag/` is strictly cheaper — one `mkdir`, no file, no valid TOML — and under the halt rule it stops every run beneath it. Because discovery resolves no root, `inspect_root_trust` never runs, so neither trust warning fires to say the halting directory is world-writable. The diagnostic names the directory, which is all that stands between the reader and a mystery.

- **2026-08-01 — a vault selected by name reports no registry entry when the entry's path is not canonical.** Registry paths are absolute and unexpanded; the resolved root is canonical. The report compares the two lexically, so with a symlink anywhere in a registered path `doctor --vault work` succeeds, prints the canonical root, and reports `installation.entry = null` — a run resolved *through* an entry it then says does not exist.

- **2026-08-01 — `resolve_registered` takes an `Installation`, not an `InstallationRecord`.** The signature recorded in [the surfaces record](2026-07-31-m2-surfaces-and-the-sdk-boundary.md) cannot express the absent and unusable states, which are exactly the states its refusals must distinguish. The deviation is right; the recorded signature is what was wrong.

- **2026-08-01 — `XDG_CONFIG_HOME` is not how the conformance harness stays hermetic.** This record and [the vault contract record](2026-07-31-vault-contract-and-installation-record.md) both credit the variable with giving the harness hermetic runs. The harness sets no environment variable at all; it passes explicit paths into a temporary tree, which is a stronger mechanism than the one recorded. The claim is stale rather than wrong about the outcome.
