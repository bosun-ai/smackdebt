## Why

The progressive report contains the right facts, but its terminal form still
reads like debug output. Columns drift, small percentages disappear, large
counts are hard to scan, and the same layout is forced into every terminal
width. Important findings compete with routine coverage text instead of guiding
the reader from project health to the next useful detail.

Smackdebt should communicate code quality in seconds to both people and tools.
Its terminal output needs a calm visual hierarchy while redirected output must
remain plain, stable, and readable.

## What Changes

- Introduce a private presentation layer that selects and ranks borrowed report
  values once before width-specific rendering.
- Render codebase and diff reports as a restrained developer dashboard with a
  clear header, summary, distribution, leading detail, coverage notes, and one
  copyable drill command.
- Add full, compact, and stacked terminal layouts with Unicode-aware alignment,
  visible fractional bars, formatted counts, and concise singular or plural
  text.
- Add `--color auto|always|never`, honor `NO_COLOR` in automatic mode, and keep
  JSON free from terminal styling.
- Resolve width in the CLI from `COLUMNS`, the connected terminal, or a stable
  redirected default. The output crate receives resolved choices and performs
  no environment or terminal inspection.
- Correct package totals so repository summaries count package scopes while a
  selected directory or file reports its nearest package.
- Keep JSON schema version 1 and all report facts unchanged.
- Update product and architecture documentation with verified examples and the
  presentation boundary.

## Impact

- Affected specs: `progressive-exploration`, `workspace-architecture`,
  `product-documentation`, `architecture-documentation`
- Affected crates: `smackdebt-output`, `smackdebt`
- New small dependencies: `anstyle`, `unicode-width`, and `terminal_size`
- No interactive terminal interface is added in this change; the private
  presentation rows make that a later renderer choice.
