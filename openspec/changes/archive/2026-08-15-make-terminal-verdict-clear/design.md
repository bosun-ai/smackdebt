## Context

The archived short-output change removed much of the old report, but the
remaining summary still asks the reader to translate rated-unit totals into a
verdict. Area percentages and bars compete with exact status counts. Coverage
also mixes genuine incomplete analysis with fixture/generated role policy,
while a last-resort width writer can hide the end of a fact-bearing row. The
report and JSON already retain the needed facts, so this change stays at
presentation and input-error seams.

This change is first in a three-change terminal sequence. It establishes a
clear verdict and fact-preserving layout. `make-terminal-detail-relevant` owns
which detailed facts appear. `make-diff-output-debt-focused` owns the later diff
story. `prepare-first-release` remains blocked until all three are archived.

## Goals / Non-Goals

**Goals:**

- Let a user read overall attention share and exact High and Watch counts
  without calculation.
- Keep area choice driven by exact counts in stable severity-led order.
- Distinguish real incomplete analysis from deliberate source-role policy.
- Preserve every useful fact at resolved widths by choosing row shape from its
  content.
- Make three common input failures direct, exact, and safe.
- Prove that JSON, analysis, work, status classes, and stream placement do not
  change.

**Non-Goals:**

- Changing health thresholds, finding rank, report fields, JSON version 3,
  source roles, parser trust, architecture resolution, or diff policy.
- Deciding the detailed relationship list or removing detail families; that is
  the next change.
- Redesigning diff findings or direction; that is the third change.
- Adding a second renderer model, terminal setting, or new command option.

## Decisions

### QUALITY is a two-line verdict

For a non-empty checked selection, `QUALITY` writes exactly these two content
lines. The checked count uses the existing grouped-integer presentation, such
as `1,686 checked`:

```text
<grouped-checked> checked · <attention-percent-to-one-decimal>% need attention
<High glyph> <high-count> · <Watch glyph> <watch-count>
```

`checked` is the existing rated-unit total. Attention count is the existing
High plus Watch total. No report field is added. Presentation arithmetic widens
both existing counts to an unsigned integer type that safely holds
`attention * 1000`, divides that scaled numerator by checked count, and rounds
from quotient and remainder. It increments the tenths value when twice the
remainder is greater than or equal to the checked count. This produces a
nonnegative nearest tenth without floating-point behavior; an exact halfway
value advances to the next tenth, so 1 of 16 is `6.3%`.
Counts and the percentage are presentation derived from existing report facts
and do not enter the report or JSON.

When checked is greater than zero and attention is zero, the first line uses the
grouped checked count and `0.0%`; the second is exactly `No findings.` and empty
glyph counts are not printed. When checked is zero, `QUALITY` has the sole
content line `Nothing was checked.` It prints no percentage, findings sentence,
or glyph count. The old `rated units`, `N need attention`, and duplicate icon
total do not appear.

### Areas show decisions, not presentation ratios

Codebase area rows retain exact High and Watch counts and their existing stable
severity-led order. Diff area rows retain their exact status counts and stable
order. Human area rows do not print local rate, debt share, change share,
percentages, or bars. The underlying counts, shares, rates, ordering facts, and
JSON version-3 values remain unchanged.

### Coverage means analysis was genuinely incomplete

A terminal coverage gap exists only for a selected source read failure, a
failed parse, or a recovered/advisory parse excluded from health. Fixture and
generated exclusions are policy, not failed coverage, and do not produce a
warning. Primary, test, example, and benchmark source remain part of the
default verdict; only fixture and generated roles receive this policy exclusion.

Default root, package, and directory views group each real gap class instead of
repeating one sentence for every fact. `--all` names every relevant failed or
recovered file once. An explicitly selected affected file names that file.
This change does not alter default directory file-list detail; that decision
belongs to `make-terminal-detail-relevant`. This change also does not alter
retained diagnostics, advisory facts, health indexes, or JSON.

The architecture resolution warning remains separate from source coverage.
Unmatched and ambiguous imports produce one grouped human row: `<Warning glyph>
1 import could not be followed.` or `<Warning glyph> <count> imports could not
be followed.` It does not expose command, status, parser, or other
implementation text. This decision does not choose or describe the raw
relationship rows in detailed output.

### Rows choose their shape from content

The renderer does not select a fixed table solely because width equals 120, 80,
or 50. Each row measures its visible content, keeps a compact form when it fits,
and otherwise stacks related facts on indented lines. Paths and long identities
shorten in the middle when needed so the beginning and end remain recognizable.

Measurements, exact counts, cycle closure, history commit evidence, dependency
state, commands, finding identity, and comparison identity are never silently
clipped. The final writer guard remains only for unexpected synthetic input.
Reviewed default codebase, path, and diff flows at widths 120, 100, 80, and 50
must record zero safety-fallback shortenings. A synthetic overflow test alone
exercises that guard.

### Input errors name the fixable value

The three revised failures are exact:

- A missing selected path exits with status 1, writes no stdout, and writes
  exactly `smackdebt: path not found: <user-path>\n` to stderr.
- A missing Git ref exits with status 1, writes no stdout, and writes exactly
  `smackdebt: Git ref not found: <ref>\n` to stderr.
- `--all --json` exits with status 2, writes no stdout, and writes exactly
  `smackdebt: --all cannot be used with --json\n` to stderr.

No failure appends usage or help. User values are rendered from the original
argument rather than a resolved private path or subprocess diagnostic.

### Proof stays public, with private review in aggregate

Exact public fixtures cover default codebase, path, and diff output at widths
120, 100, 80, and 50; empty, zero-attention, and non-zero verdicts; grouped
checked digits; integer-safe halfway-up arithmetic; area ordering; real and
fixture/generated exclusions; the exact architecture warning; color equality;
and all three errors. JSON version-3 bytes, serial and automatic terminal/JSON bytes,
status, streams, rank, work counters, allocations, and resource profiles remain
unchanged.

Read-only review covers self, a private mixed application, and a private Rust
workspace. Committed review material stores only workload family and aggregate
outcomes. It never stores raw private output, names, paths, source, identities,
or history.

## Risks / Trade-offs

- **A percentage can look more precise than the policy.** One decimal digit is
  explicitly presentation arithmetic over exact counts, not a new score.
- **Removing area shares changes a familiar scan aid.** Exact status counts and
  stable order give the next drill choice without competing denominators; JSON
  retains every distribution fact.
- **Content-aware writing has more row cases.** One shared measurement and
  stacking policy plus exact width fixtures keeps those cases reviewable.
- **Grouped warnings expose less default context.** `--all`, path selection, and
  JSON retain relevant file facts without repeating summary sentences.

## Migration Plan

1. Accept and strict-validate this change before implementation.
2. Add pure verdict, coverage, warning, row-shape, and error policy tests.
3. Update terminal and CLI writing without changing the completed report.
4. Update docs and guarded exact public snapshots.
5. Run unchanged-machine, work, resource, and aggregate workload proof.
6. The other two changes can already be authored; archive this change before
   implementing `make-terminal-detail-relevant`.
7. Archive the detail change before implementing
   `make-diff-output-debt-focused`.
8. Resume `prepare-first-release` only after all three are archived.

Rollback restores the prior terminal and error snapshots. Report data, JSON,
and analysis require no migration.
