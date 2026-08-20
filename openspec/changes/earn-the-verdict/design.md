## Context

Smackdebt already walks once, streams one history process, and owns a frozen
verdict vocabulary. The 2026-08-20 evaluation showed the inputs to that verdict
are not yet trustworthy: the hand-rolled ignore engine approximates git, nested
checkouts leak into analysis, the history window is applied after streaming
everything, grouped Rust imports collapse, and tier policy ignores evidence
volume and absolute debt mass. The repository itself lacks the license, CI,
and gate its own README implies.

> **Constants under review**
>
> The following constants are proposed values awaiting user review during
> implementation; each is flagged where it lands:
>
> - Tier volume terms (slice 11): `DENSITY_EVIDENCE_UNITS = 200`,
>   `SMALL_SCOPE_HIGH_UNITS = 10`, `VOLUME_FIGHTS_BACK_HIGH = 100`,
>   `VOLUME_LOST_HIGH = 1000`.
> - Unsupported-share threshold (slice 12):
>   `UNSUPPORTED_QUALIFIER_PERMILLE = 100` (10% of selected bytes).
> - Gate signal list and baseline filename (slice 14): the ten time-invariant
>   signal ids and `.smackdebt-baseline.tsv`.
> - `parents(true)` (slice 4, D3): ancestor ignore files apply when a subpath
>   of a repository is analyzed.

## Goals / Non-Goals

**Goals:**

- Make discovery byte-faithful to git ignore semantics and blind to nested
  checkouts.
- Make every history-derived number describe the stated window, cheaply, and
  disclose an empty window.
- Restore item-level dependency fidelity for grouped Rust imports.
- Weigh evidence volume and absolute debt mass in the codebase tier and
  qualify the verdict when coverage is thin.
- Ship the promised ratchet gate and run it on smackdebt itself.
- Give the repository a license, CI, and reviewable JSON snapshots.

**Non-Goals:**

- No escape hatch for the nested-checkout prune in this wave.
- No async I/O, no persistent cache, no second walk or extra Git process.
- No change to the frozen tier ids or sentences.
- No line-count reads for unsupported mass; bytes come free from walk
  metadata.
- History-derived signals never enter the gate.

## Decisions

### Discovery rides the `ignore` crate, serially

The walk uses `ignore::Walk` — never the parallel walker, because
`smackdebt-project` owns the only Rayon pool and serial/parallel byte equality
depends on discovery's deterministic order. The builder is pinned:
`hidden(false)`, `ignore(false)` (no `.ignore`/`.rgignore`), `git_ignore`,
`git_global`, `git_exclude` on, `require_git(false)`, `parents(true)`,
`follow_links(false)`, per-directory file-name sort, and a `filter_entry` for
dependency directories, configuration excludes, and nested checkouts. Key
decisions:

- **D1 `require_git(false)`**: `.gitignore` is honored with or without a
  repository, preserving existing behavior for bare directories.
- **D3 `parents(true)`**: analyzing a subpath honors ancestor ignore files.
  Hazard: acceptance fixtures live inside this workspace, so the workspace
  root `.gitignore` applies to them; a guard test asserts the exact selected
  count for the source-engine fixture so a future root-`.gitignore` edit fails
  loudly.
- **D6** `.smackdebt.toml` `exclude` patterns layer through a gitignore
  builder rooted at the analyzed root — not override semantics — so anchored
  patterns anchor and `!` negations work. This is a behavior fix over the old
  hand-rolled matcher.
- **D7** the hardcoded dependency-directory skip list (`.git`, `.hg`, `.svn`,
  `target`, `node_modules`, `vendor`) applies regardless of gitignore content
  and wins over `!` negations, because the README promises dependency
  directories are excluded by default.
- **D8** visited-entry counts are redefined as "entries the walk yielded";
  pruned subtrees are never yielded. Evidence arrays are updated by hand with
  an explanation per delta.
- The per-directory name sort is load-bearing for manifest recording (first
  manifest in a directory wins), and the final repository-relative sort is
  kept — byte-identical serial and parallel output depends on it.

Named bug this fixes: the hand-rolled parser stripped leading `/`, so anchored
patterns matched at any depth.

### Nested checkouts are pruned, and disclosed

Any directory below the analyzed root containing a `.git` entry — directory
(submodule, clone) or file (linked worktree) — is pruned in `filter_entry` and
recorded as a `nested_repository` diagnostic. The `.git`-file case is the one
neither the old walker nor the `ignore` crate handles. One `exists()` check
per directory. There is no opt-out in this wave; a future
`include_nested_checkouts` option is out of scope.

### The window moves into git, on landed dates

`--since` is passed to the streamed history process in epoch form, and the
parsed timestamp switches from author date to committer date so the process
filter and the in-process boundary compare the same instant — otherwise
rebased or cherry-picked commits silently vanish. Window semantics become
"when the change landed", the right meaning for recent activity. The
in-process window check is retained as a defensive boundary filter; the
excluded-by-window counter now counts boundary rejects (normally zero), and
stream summary counts describe the windowed set. No schema change.

### Coupling wording gains a transitive link, findings do not

Analysis computes a per-pair link classification — direct, indirect via a
named first intermediate on the shortest package path (checked in both
directions), or none — once at report build. Output renders direct pairs
unchanged, indirect pairs as `no direct dependency` plus `linked via
<package>`, and unreachable pairs as `no code dependency` unchanged. Finding
creation is deliberately unchanged: a transitive link does not suppress the
Watch coupling finding, and this is stated in the spec so nobody "fixes" it
later.

### Tier mapping gains a cap and floors, all integer

Two `const fn` terms join the existing max-floor combination beside the
architecture floor: a small-scope cap (fewer than 200 checked units and fewer
than 10 High caps the density-mapped tier at `worn`; Empty/Clean/Solid are
untouched) and volume floors (100 High floors at `fights_back`, 1000 High at
`lost`). Floors raise, never lower. Worked outcomes: 1 High in 5 checked reads
`worn` (was `lost`); 545 High in 102k reads `fights_back` (was `worn`); every
accepted boundary scenario at 1000 checked is unchanged.

### Unsupported mass is measured in bytes

Line counts would add file reads constrained by the performance evidence, so
the qualifier keys on byte share: `size_bytes` comes free from walk metadata,
coverage gains selected and unsupported byte totals, and above a 10% permille
share the verdict carries the frozen sentence `Not all source was checked.`
plus a fact naming the share and the largest unsupported language. The
qualifier never moves the tier.

### The gate is three crates in their lanes

Pure policy (frozen signal ids, snapshot, comparison) in analysis; TSV
baseline read/write in the CLI (the config module is the precedent for
CLI-owned project files); words in output. The baseline is a committed,
sorted, tab-separated file with zero-zero rows omitted; the parser rejects
unknown signals, duplicates, and out-of-order rows with exit 2, never a
silent pass. The ten ratcheted signals are all time-invariant; history-derived
signals (coupling, concentration) are excluded by rule, not omission, because
they roll with wall-clock time and would make `just check` flaky. Any
regression exits 3; improvements are reported, never auto-applied — `--update`
is the only writer, so `just check` never dirties the tree.

### Hermetic acceptance environment

The `ignore` crate resolves the global gitignore from `$XDG_CONFIG_HOME` and
`$HOME` (it does not read `GIT_CONFIG_GLOBAL`), so acceptance child processes
and fixture git commands pin `HOME`, `XDG_CONFIG_HOME`, `GIT_CONFIG_GLOBAL`,
and `GIT_CONFIG_SYSTEM` through one shared helper. This lands before the
discovery rewrite and must move zero snapshot bytes; the positive
leak-proof evidence (a pinned global ignore provably excluding a file) lands
with the rewrite.

### Reviewable snapshots via textconv, not pretty JSON

End-to-end evidence requires real CLI stdout bytes, so pretty-printing `--json`
for reviewers would force a product change and full regeneration. Instead
`.gitattributes` marks JSON snapshots `diff=json` and a documented textconv
renders structured diffs — zero product change, reversible.

### CI runs the one gate that already exists

One workflow runs `just check` on push and pull request, on Linux and macOS,
from a full-depth checkout (self-analysis and the gate need history), with
every installed tool being one `just check` already needs. The repository has
no remote yet; the workflow is inert until one is created manually.

## Risks / Trade-offs

- **Snapshot churn volume**: grouped-use resolution and tier terms touch most
  committed snapshots. Mitigation: strictly sequential slices, per-case
  regeneration, textconv first, every commit body states the expected diff
  shape.
- **Ancestor-ignore hazard**: `parents(true)` makes the workspace root
  `.gitignore` apply to in-repo fixtures. Mitigation: exact-count guard test.
- **Date-semantics slip**: fixtures set author equal to committer, so tests
  cannot catch a missed `%ct` swap; the spec delta states the semantics
  explicitly.
- **Grouped-use fan-out**: new edges may raise smackdebt's own verdict —
  correct behavior; it lands in the self-baseline, not as a regression.
- **New dependency tree**: the `ignore` crate's tree is license-checked inside
  the discovery slice via `just licenses`.
- **Gate flakiness would be fatal**: prevented by the time-invariant signal
  rule.

## Migration Plan

1. Author and strict-validate this change (slice 0).
2. Ops first: license, dead check, textconv, CI, hermetic environment
   (slices 1–3).
3. Discovery rewrite, nested-checkout exclusion, grouped-use fix, windowed
   history, disclosure sentences, coupling wording, warning breakdown
   (slices 4–10, strictly sequential where snapshots churn).
4. Verdict quality: tier terms, unsupported qualifier, `--top`
   (slices 11–13).
5. The gate, then self-gating with the committed baseline wired into
   `just check` (slices 14–15; every analysis-changing slice lands first so
   the baseline is fresh on arrival).
6. Archive (slice 16).

Rollback before slice 15 is per-slice revert; after slice 15 a revert must
also regenerate the committed baseline.
