## Why

The 2026-08-15 field runs on smackdebt, swiftide, and fluyt showed that the
report never says its answer out loud, and that its human-only decoration makes
it unreadable for the second audience it already has — an LLM reading piped
output.

- The verdict is Nerd-Font private-use glyphs and bare numbers. Piped into
  another program it is private-use codepoints and unlabeled integers.
- swiftide `diff HEAD~15` renders ` 1 · 204`: worse 1, better 0 silently
  omitted, changed 204 mostly healthy units. fluyt `diff HEAD~30` renders `33 ·
  16 · 4,138`. Positional columns force the reader to guess.
- A clean diff prints `No changes` and then dumps unchanged history couplings
  and warnings anyway.
- Diff findings read `<closure 1177> · added` and `GenericOpenAI<C>::<closure
  100> · added` — no line number, no measurements, nothing actionable.
- Cycle witnesses truncate with `…` at default width (`…crates/analysis…`), so
  the one piece of evidence a cycle finding exists to show is unreadable.
- Errors leak internals: `smackdebt: could not read
  /Users/timonv/projects/smackdebt/does/not/exist: No such file or directory (os
  error 2)`.
- `--all` prints raw edges, standard-library externals, churn dumps,
  cyclomatic-1 rows, weak coupling, and healthy rows: a data dump instead of all
  the useful debt.

This change supersedes all three retired terminal changes —
`make-terminal-verdict-clear`, `make-terminal-detail-relevant`, and
`make-diff-output-debt-focused` — which were archived without merging their
deltas. They kept glyph-only severity and presentation-owned arithmetic; this
change makes words carry the meaning and glyphs decorate them.

## What Changes

- Severity and direction are words: `high`, `watch`, `worse`, `better`,
  `changed`, `warning`, and `next:`. This reverses the accepted requirement that
  human output never repeats a severity word beside a glyph.
- Glyphs and the tier-colored bar become decoration resolved from terminal
  detection, always adjacent to the word they decorate. `--color never` produces
  fully plain output, and piped output contains no codepoint in U+E000–U+F8FF.
- A verdict block replaces the header, quality, and change lines: the scope, the
  tier sentence, labeled counts, and the worst offender with its resolved path
  and reason.
- Every diff count is labeled with its word and zero counts are printed rather
  than omitted. A clean diff prints the verdict line only.
- Diff findings become actionable: `path:line`, before and after values for every
  changed measurement, and human identities for anonymous units.
- `ARCHITECTURE` adds stable-dependency rows with integer operands and stacks
  cycle witnesses instead of truncating them. `HISTORY` adds concentration rows
  and shows one row per pair. `WARNINGS` are grouped. `AREAS` shows at most five
  debt-bearing children with word-labeled counts. `FINDINGS` gain `· hot (n
  commits)`.
- `--all` shows all useful debt only. Raw edges, standard-library externals,
  churn dumps, cyclomatic-1 rows, weak coupling, and healthy rows leave the
  terminal permanently; JSON stays complete.
- Width handling becomes content-aware per row — aligned when it fits, stacked
  when it does not — replacing report-level width tiers. Nothing is silently
  clipped.
- Missing paths, missing Git refs, and the `--all --json` conflict get exact
  one-line stderr bytes with no operating-system text.
- The README is rewritten with its examples, including removing the "no ASCII
  fallback" commitment, which this change makes false.

## Capabilities

### Modified Capabilities

- `terminal-output`: Word vocabulary, decoration policy, verdict block, labeled
  counts, grouped warnings, actionable diff findings, and exact errors.
- `progressive-exploration`: Content-aware per-row layout, `--all` as all useful
  debt, and clean-diff behavior.
- `product-documentation`: README rewrite including the removed ASCII-fallback
  commitment.
- `architecture-documentation`: Decoration resolution and the presentation
  boundary.
- `end-to-end-evidence`: Piped-versus-tty bytes, private-use-free piped output,
  per-tier verdict blocks, exact error bytes, and stacking evidence.
- `release-readiness`: Keeps publication blocked through the five v-next
  changes.

## Dependency Order

The v-next set is implemented, reviewed, and archived in this order:

1. `resolve-workspace-dependencies`
2. `deepen-debt-signals`
3. `add-verdict-policy`
4. `redesign-terminal-report` (this change)
5. `adopt-report-schema-v4`

This change renders the verdict and signals produced by the three earlier
changes and performs no analysis of its own. `prepare-first-release` stays
blocked until all five are archived.

## Impact

This changes human terminal bytes, error bytes, README content, and every
terminal snapshot. It affects the output crate, CLI terminal resolution and
argument failures, documentation, and acceptance fixtures. It does not change
report facts, analysis policy, rank, exit classes, stream placement, or JSON
version 3 bytes; the schema change belongs to `adopt-report-schema-v4`.
