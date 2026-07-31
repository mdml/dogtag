# Vault-root discovery and vault selection

- Status: accepted
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

**Nearest root wins.** When an ancestor directory also holds a contract, `doctor` reports it as a warning-severity diagnostic. Nesting is legal — a vendored vault, a vault checked out inside another — so it is deterministic and visible rather than refused.

### Selection

Vault selection is the CLI's, and resolves in one order: **`--vault`, then `DOGTAG_VAULT`, then upward discovery from the current directory.** Every command reports the root it resolved and how.

`--vault` and `DOGTAG_VAULT` take either a registered name or a path, distinguished **syntactically, with no fallback between them**:

- An argument containing a path separator, or beginning with `.`, `/`, or `~`, is **always a path**. It is used exactly, never searched upward: explicit means explicit.
- Any other argument is **always a registered name**, looked up in the installation record's registry. An unregistered name is an error; it never falls back to being a path.

The no-fallback rule is what makes this deterministic. *Path-if-it-exists-else-name* would resolve `--vault work` differently depending on whether a `work` directory happens to sit in the current directory, so the same command would select different vaults from different places. *Name-first-else-path* would let registering a vault named `docs` silently change what `--vault docs` means in every existing script on the machine — a machine-local file reinterpreting an invocation, which the settings PDR forbids. With no fallback, the argument's own syntax fixes its meaning, and registering a vault can never change what an existing invocation resolves to.

Two supporting rules follow. Registry names are kebab-case and contain no path separators, and **duplicate names are a load error on the installation record** rather than a silently-resolved ambiguity. A registered name whose path is absent or is not a vault root is an error citing the registry entry as related evidence — never a fall-through to discovery.

**Registration never implies selection.** There is no default vault and no "current vault" state, so multiple registered vaults cannot produce ambiguity: the registry answers only questions that name an entry.

### The SDK/CLI boundary

**The SDK's discovery entry point is a pure function of an explicit starting path.** It reads no environment variable, consults no current directory, and holds no process-global vault state. The CLI resolves argv, environment, and the current directory into an explicit path, and resolves registry names into paths, before calling in.

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
- **Discovery is exhaustively testable**, since it takes an explicit path. Every branch — nested, incomplete, absent, symlinked, competing — is reachable against a synthetic tree, which matters directly for the 100% kernel coverage floor.
- **The registry gains a real job at M2** (name resolution) beyond being reported, which means the installation record's parse and validation rules are load-bearing from the first release that reads it.
