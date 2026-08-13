## Context

Terminal rendering currently mixes selection, ranking, omission, formatting,
and writing in the same functions. Width only affects one detail label, color
cannot be controlled, and package summary counting recursively treats
non-package scopes as packages. The report domain already retains every fact
needed for a better view, so analysis and JSON do not need to change.

## Decisions

### Build private borrowed presentation rows

The output crate will create private rows for the selected summary, child
areas, findings, comparisons, diagnostics, and the next drill command. Rows
borrow report values and contain display decisions such as ordering and omitted
counts. Width renderers consume those rows without scanning the report again.

The interface stays private. It prepares a clean seam for a later interactive
renderer without promising a Rust API or introducing terminal state now.

### Use three width tiers

The resolved terminal width selects one layout:

- 100 columns or more: aligned full tables with twelve-cell bars;
- 70 through 99 columns: compact aligned tables with eight-cell bars;
- fewer than 70 columns: stacked area cards with ten-cell bars.

Area labels may be shortened in the middle to preserve their useful beginning
and ending. Source locations and drill commands remain whole and copyable.
Unicode display width, rather than byte or character count, controls padding.

### Make exact facts primary

The codebase summary shows the attention count and rate, High, Watch, and
Healthy totals, plus analyzed coverage. A neutral bar visualizes the attention
rate without inventing a score. Codebase area rows show selected-scope debt
share and local attention rate. Diff rows show their share of all retained
changes. Fractional block characters keep small non-zero values visible.

Large integers use thousands separators. Table zeroes use an en dash. Text
with a zero count, including excluded-file noise and activity, is omitted when
it adds no information.

### Use sparse semantic styling

The terminal uses a small symbol and color vocabulary:

- `▲ HIGH` and `▲ WORSE` use red;
- `● WATCH` and `● CHANGED` use yellow;
- `▼ BETTER` uses green;
- `!` marks coverage gaps;
- `→ Explore` uses cyan.

Headings and secondary facts use emphasis or dim styling without backgrounds
or boxes. ANSI styling is optional; symbols and layout remain in redirected
plain text. Removing ANSI sequences from styled output must reproduce the
plain output byte for byte.

### Resolve color and width in the CLI

`--color` accepts `auto`, `always`, or `never` and defaults to automatic
behavior. Automatic color requires a terminal and the absence of `NO_COLOR`.
Always and never are explicit overrides. Supplying `--color` with `--json` is
an argument error, and JSON never receives style choices.

`COLUMNS` is the explicit width override. Otherwise a connected terminal uses
its reported width and a redirected stream uses 100 columns. The CLI passes
resolved width and color values through `TerminalOptions`; the output crate
does not read process state.

### Count packages by scope meaning

Repository summaries count descendant package scopes. A package, directory, or
file selection reports one package when it has a package ancestor, otherwise
zero. Files and directories are never counted as fallback packages.

## Risks

- Rich output can become noisy. Styling is limited to semantic roles, and plain
  output has identical wording and layout.
- Narrow output can hide relationships. Stacked cards retain every exact count
  and percentage shown in wider layouts.
- Unicode widths vary in unusual terminals. The renderer uses measured display
  width and tests its chosen symbols, bars, and shortened labels.
- New layout snapshots can make intentional wording changes broad. Pure row
  selection tests remain separate from rendering snapshots so failures point to
  either policy or presentation.
