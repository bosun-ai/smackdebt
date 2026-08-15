## Why

Smackdebt exists to answer "is this code shit?" and today it never answers. The
2026-08-15 field runs on smackdebt, swiftide, and fluyt show the gap directly:

- Codebase output ends in bare counts. The reader has to convert `12 high · 94
  watch · 1,686 checked` into a judgement, and two readers convert it
  differently. An LLM consuming the same output has nothing stable to key on.
- Diff output is worse. swiftide `diff HEAD~15` prints ` 1 · 204`: worse 1,
  better 0 silently omitted, changed 204 — and the 204 are mostly healthy added
  or removed units. fluyt `diff HEAD~30` prints `33 · 16 · 4,138`. Neither line
  says whether the change made the code better or worse.
- Nothing reconciles the three comparison families. A diff whose source
  comparisons are all neutral but which introduces a package cycle has no way to
  come out worse.
- No report names the single worst thing in the codebase, although the ranked
  findings already know it.

This change supersedes the verdict parts of the retired
`make-terminal-verdict-clear` and `make-diff-output-debt-focused` changes. Those
kept the answer in presentation arithmetic; this one puts it in analysis, where
terminal and JSON can print identical bytes and the tier id is a stable contract
for machine consumers.

## What Changes

- A new pure verdict policy in analysis owns codebase tiers, diff tiers, the
  sentence for each tier, the counts behind them, and the worst offender.
- Codebase tier ids are frozen: `empty`, `clean`, `solid`, `worn`,
  `fights_back`, `lost`. Their sentences are `Nothing was checked.`, `Clean.
  Ship it.`, `Solid, with rough edges.`, `Worn in the usual places.`, `This code
  fights back.`, and `The code is winning.`
- Tier selection uses integer permille arithmetic over High count and checked
  units. Package cycles escalate: at least one High architecture finding floors
  the tier at `worn`, at least three floors it at `fights_back`.
- Diff tier ids are frozen: `no_debt_change`, `better`, `worse`, `mixed`, with
  sentences `No debt changed.`, `You made it better.`, `You made it worse.`, and
  `Better here, worse there.`
- The diff tier reconciles source, architecture, and history comparisons, and
  the facts behind it name the family that moved and label every count with its
  word, including zero counts.
- `DebtDiffSelection` becomes analysis-owned: the typed comparison and finding
  identities that count as human debt movement, per scope. Healthy added or
  removed units, unchanged and ambiguous comparisons, and fixture or generated
  source stay in JSON and never move a verdict.
- Every selected scope has its own verdict computed from that scope's facts.
- The worst offender is the first ranked finding with its resolved path, falling
  back to the first package-cycle witness, with reasons `hot AND complex`, `most
  complex`, and `package dependency cycle`.

## Capabilities

### New Capabilities

- `verdict-policy`: Owns codebase and diff tiers, their sentences, integer tier
  mapping, escalation floors, debt-diff membership, and worst-offender
  selection.

### Modified Capabilities

- `progressive-exploration`: Every selected scope exposes its own verdict.
- `end-to-end-evidence`: Tier boundaries, the contradiction case, and
  worst-offender fallback are proven end to end.
- `release-readiness`: Keeps publication blocked through the five v-next
  changes.

## Dependency Order

The v-next set is implemented, reviewed, and archived in this order:

1. `resolve-workspace-dependencies`
2. `deepen-debt-signals`
3. `add-verdict-policy` (this change)
4. `redesign-terminal-report`
5. `adopt-report-schema-v4`

This change consumes the hot and role-aware rank and the architecture findings
produced by the two earlier changes. `redesign-terminal-report` renders the
verdict and `adopt-report-schema-v4` serializes it, so neither can precede it.
`prepare-first-release` stays blocked until all five are archived.

## Impact

This adds a completed verdict, its counts, its debt-diff membership, and its
worst offender to the report. It affects report construction, comparison
selection, and index-integrity audits. It does not change terminal presentation
or JSON serialization in this change; those land in the two following changes.
No configuration is introduced: tier ids and thresholds are frozen policy.
