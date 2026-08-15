> Superseded on 2026-08-15 by the v-next change set (`resolve-workspace-dependencies`, `deepen-debt-signals`, `add-verdict-policy`, `redesign-terminal-report`, `adopt-report-schema-v4`); archived with `--skip-specs` at 0% implementation, so none of its deltas entered accepted specs.

## Why

After the verdict and detail changes, diff output still treats every retained
comparison as a human result. Healthy-only movement, unchanged facts, uncertain
matches, ordinary graph changes, and unchanged history context compete with
actual debt movement. A diff should answer whether the selected work made debt
worse or better and where to inspect it, while JSON remains complete.

## What Changes

- Select human source comparisons only when debt regressed, improved, was added
  or removed, or a measurement changed while either side was Watch or High.
- Keep unchanged, ambiguous, healthy-only, fixture, generated, and recovered
  comparisons in JSON; exclude fixture/generated/recovered comparisons from
  diff default, diff `--all`, and diff path while preserving codebase behavior;
  count each unique selected changed file once across read failure,
  failed/recovered parse, and ambiguous unit match, then show one exact grouped
  changed-file warning.
- Show architecture and history diff rows only for introduced or removed
  findings, as Worse or Better. Keep unchanged facts and ordinary Changed
  context in JSON.
- Make default diff `QUALITY`, source `AREAS`, and source discover guidance use
  only trusted source debt counts. Limit source debt-bearing areas to five and
  each source, architecture, and history detail group to three. Architecture-
  only and history-only results use their sections without source summary
  sections. `--all` removes only those human-debt limits, and a path changes
  scope only.
- Add one private, nonserialized, analysis-owned `DebtDiffSelection` per scope
  with three unique typed ID lists: trusted source debt comparisons,
  introduced/removed architecture findings, and introduced/removed evolution
  findings. `DebtDiffCounts` counts only source IDs. Recovered comparisons
  remain JSON-only in both human modes.
  Index audits reject repeated IDs; terminal reads the completed selection
  without trust policy or scans.
- Give unnamed source units useful human identities as `container · kind` or
  `filename · kind`, including `filename · template` for Vue, with `path:line`
  on the next line and unchanged JSON identity fields.
- Preserve every comparison, JSON version-3 bytes and fields, direction and
  count facts, analysis results, CLI flags, work, allocation, and resource
  behavior.

## Capabilities

### Modified Capabilities

- `terminal-output`: Defines debt-focused diff sections, exact empty states,
  warnings, identities, measurements, locations, and human limits.
- `progressive-exploration`: Defines human debt direction, counts, scope
  behavior, stable order, and meaningful diff detail.
- `source-signal-quality`: Defines which source comparisons are human debt and
  keeps uncertain or healthy-only comparisons out of verdict rows.
- `architecture-analysis`: Limits human architecture diff to introduced and
  removed findings while retaining complete graph comparisons in JSON.
- `evolutionary-analysis`: Limits human history diff to introduced and removed
  findings while retaining history as machine context.
- `workspace-architecture`: Assigns each private per-scope trusted selection
  object and its count value to analysis/report assembly.
- `report-schema-v3`: Extends index integrity proof while preserving exact JSON
  version-3 bytes and fields.
- `analysis-performance`: Requires one-pass borrowed rendering from completed
  links without global scans, second collections, or project work.
- `product-documentation`: Documents debt-focused diff output, `--all`, paths,
  empty states, and complete JSON.
- `architecture-documentation`: Documents comparison selection ownership and
  the private presentation link seam.
- `end-to-end-evidence`: Adds exact scope, mode, width, identity, no-debt,
  unchanged-machine, work, and privacy-safe workload proof.
- `release-readiness`: Keeps release preparation blocked until this final
  terminal change is reviewed and archived.

## Dependency Order

All three terminal changes can be authored now. Implement, review, and archive
`make-terminal-verdict-clear`, then `make-terminal-detail-relevant`, then this
change. Archive this change before resuming `prepare-first-release`.

## Impact

Human diff terminal bytes, help, README, architecture guidance, and exact
snapshots change. Analysis/report assembly gains one private nonserialized
selection object per scope. No CLI flag, JSON field, JSON version,
retained comparison, comparison direction, public report fact, analysis rule,
exit class, worker policy, or resource limit changes.
