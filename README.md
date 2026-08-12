# Smackdebt

**Find the code that hurts, then see whether your change made it better.**

Smackdebt turns source metrics and Git history into two short reports:

- How bad is this codebase, and where are the hotspots?
- How did this worktree change compared with a Git ref?

Run it without configuration. Start at the repository summary, then pass one of
the reported paths to inspect a package, directory, file, or function.

## Install

```console
cargo install smackdebt
```

Smackdebt runs locally and does not upload source code or analysis data.

## Check a codebase

Run `smackdebt` from anywhere inside a repository:

```console
$ smackdebt

smackdebt  .
2 packages · 184 files · 29,418 lines · 96% analyzed

Health
  high    7 functions
  watch  23 functions
  healthy 1,146 functions

Hotspots                                           health  touches/90d
  crates/api/src/checkout.rs                       high             14
    process_checkout              cognitive 31 · cyclomatic 18 · 126 lines
  packages/web/src/routes/orders.ts                 high              9
    loadOrders                    cognitive 27 · cyclomatic 12 · 84 lines
  crates/core/src/pricing/                          watch            18
    4 functions need attention

Explore
  smackdebt crates/api/src/checkout.rs
```

Smackdebt combines fixed code-health limits with recent activity. A complex
function that changes often rises to the top. A complex function in quiet code
still appears as debt, but does not outrank active risks just because it is
large.

Pass a reported path to see its next level:

```console
$ smackdebt crates/api/src/checkout.rs

smackdebt  crates/api/src/checkout.rs
Rust · 612 lines · 18 functions · 14 touches/90d

Functions                                           health  reason
  process_checkout                88–213             high    cognitive 31
  apply_discounts                241–319            watch    58 lines
  reserve_stock                  401–438          healthy    cognitive 4
```

The default history window is 90 days. It affects hotspot priority only:

```console
smackdebt --history 180d
```

## Check a change

Compare the current worktree with the branch it came from:

```console
$ smackdebt diff

smackdebt diff  origin/main...worktree
12 files changed · 8 analyzed · 4 unchanged by code metrics

Health change
  +1 high · -2 watch · 3 improved · 1 regressed

Regressions
  crates/api/src/checkout.rs
    process_checkout              watch → high
    cognitive 14 → 19 · cyclomatic 9 → 12 · 42 → 57 lines

Improvements
  packages/web/src/orders.ts
    groupOrders                   high → healthy
    cognitive 28 → 7 · cyclomatic 17 → 5 · 91 → 38 lines

Explore
  smackdebt diff origin/main crates/api/src/checkout.rs
```

With no ref, Smackdebt tries `origin/HEAD`, `main`, then `master`. It compares
from the merge base through the current worktree, including committed, staged,
unstaged, and non-ignored untracked changes.

Choose a ref or limit the report to a path when needed:

```console
smackdebt diff main
smackdebt diff main crates/api
```

The diff report matches named functions and code containers across both sides.
When a match is unclear, it reports the file change and explains why it could
not produce a symbol-level comparison.

## Read the ratings

Smackdebt reports separate signals instead of hiding them in one score.

| Signal | Watch | High |
| --- | ---: | ---: |
| Cognitive complexity | 15 | 25 |
| Cyclomatic complexity | 11 | 21 |
| Logical lines in a function | 50 | 100 |

A function takes its highest signal rating. Package, directory, and file views
show counts of healthy, watch, and high functions. They do not average one bad
function into a reassuring project score.

Recent activity comes from distinct non-merge commits that touched a file in
the selected history window. Smackdebt uses activity to order code that already
needs attention; activity does not change the code-health rating.

## Discovery

Smackdebt finds the repository root, supported source files, ignored paths, and
project packages. It recognizes packages from common manifests, including:

- `Cargo.toml`
- `package.json`
- `pyproject.toml`
- `pom.xml`
- Gradle settings and build files
- `CMakeLists.txt`

Files belong to their nearest package. Repositories without a known manifest
get one root package. Git ignore rules apply by default, along with common
generated and dependency directories.

Files that cannot be parsed stay visible in the coverage summary. Smackdebt
does not quietly count them as healthy.

## Languages

The first source engine uses
[`rust-code-analysis`](https://github.com/mozilla/rust-code-analysis) and
supports:

- C and C++
- Java
- JavaScript and JSX
- Python
- Rust
- TypeScript and TSX

Language support sits behind a small analysis interface. New engines can add a
language without changing Git analysis, health policy, aggregation, or output.

## JSON

Use `--json` for CI scripts, editors, and other tools:

```console
smackdebt --json
smackdebt diff main --json
```

JSON contains the same report as the terminal view. The top-level object starts
with `schema_version: 1` and includes the mode, scope, coverage, health,
activity, children, findings, diagnostics, and diff data when present.

Finding debt does not fail the command. Exit codes describe whether Smackdebt
could produce a report:

| Code | Meaning |
| ---: | --- |
| 0 | Report produced |
| 1 | Analysis could not produce a report |
| 2 | Invalid arguments or configuration |

Policy gates belong to a later release.

## Configuration

Configuration is optional. Add `.smackdebt.toml` at the repository root when
the defaults do not fit the project:

```toml
history = "180d"
exclude = ["vendor/**", "fixtures/generated/**"]

[thresholds.cognitive]
watch = 15
high = 25

[thresholds.cyclomatic]
watch = 11
high = 21

[thresholds.function_lines]
watch = 50
high = 100
```

Command-line values override the project file. `NO_COLOR` disables terminal
color, and redirected output stays readable without ANSI escape codes.

## Design

Read [ARCHITECTURE.md](ARCHITECTURE.md) for the domain model, data flow,
aggregation rules, Git behavior, and language extension path. OpenSpec records
each implementation change under [`openspec/changes`](openspec/changes).

## Credits

Smackdebt builds on Mozilla's
[`rust-code-analysis`](https://github.com/mozilla/rust-code-analysis), released
under the Mozilla Public License 2.0. It supplies the syntax-aware source
metrics; Smackdebt supplies repository discovery, health policy, Git context,
aggregation, comparison, and reports.
