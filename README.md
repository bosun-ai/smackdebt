# Smackdebt

**Find the code that hurts, then see whether your change made it better.**

Smackdebt turns source metrics and Git history into two short reports:

- How bad is this codebase, and where are the hotspots?
- How did this worktree change compared with a Git ref?

Run it without configuration. Start at the repository summary, then pass one of
the reported paths to inspect a package, directory, file, or function.

## Install

```console
cargo install --path crates/cli
```

Smackdebt runs locally and does not upload source code or analysis data.

## Check a codebase

Run `smackdebt` from anywhere inside a repository:

```console
$ smackdebt

smackdebt · codebase
.
2 packages · 184 files · 29,418 lines

QUALITY  3% need attention
▍░░░░░░░░░░░  30 / 1,176 code units
▲ 7 high · ● 23 watch · 1,146 healthy
! 7 of 184 source files excluded from analysis

DEBT BY AREA
  area          high  watch   share  rate
  crates/api        4      8     40%  █▏░░░░░░░░░░  9%
  packages/web      2      9     37%  █▍░░░░░░░░░░  11%
  crates/core       1      6     23%  █▏░░░░░░░░░░  9%
  … 4 quiet areas hidden (use --all)

TOP FINDINGS
▲ HIGH  process_checkout
        crates/api/src/checkout.rs:42
        cognitive 31 · lines 126 · 14 touches

→ Explore: smackdebt crates/api
```

Smackdebt combines fixed code-health limits with recent activity. A complex
function that changes often rises to the top. A complex function in quiet code
still appears as debt, but does not outrank active risks just because it is
large.

`share` answers how much of the selected scope's debt lives in an area. `rate`
answers how much of that area's rated code needs attention. Together they
separate large contributors from smaller areas where debt is concentrated.
Healthy-only areas stay out of the default view; `--all` shows the full shape.

Pass a reported path to progressively inspect the next level. Repository,
package, directory, and file scopes retain the same repository-relative paths:

```console
$ smackdebt crates/api/src/checkout.rs

smackdebt · codebase
crates/api/src/checkout.rs
1 package · 1 file · 612 lines

QUALITY  11% need attention
█▍░░░░░░░░░░  2 / 18 code units
▲ 1 high · ● 1 watch · 16 healthy
✓ all 1 source file analyzed

TOP FINDINGS
▲ HIGH  process_checkout
        crates/api/src/checkout.rs:42
        cognitive 31 · lines 126 · 14 touches
```

The default terminal view shows up to ten areas and three details. Use
`--all` for every terminal row and detail. JSON is always complete, so
`--all --json` is rejected.

The default history window is 90 days. It affects hotspot priority only:

```console
smackdebt --history 180d
smackdebt --jobs 4
```

## Check a change

Compare the current worktree with the branch it came from:

```console
$ smackdebt diff

smackdebt · diff
.
8 source files changed · 8 analyzed

CHANGE  8 retained units
▲ WORSE 3 · ▼ BETTER 1 · ● CHANGED 4

CHANGE BY AREA
  area          worse  better  changed  share
  crates/api        2       1        1  ██████░░░░░░  50%
  packages/web      1       –        1  ███░░░░░░░░░  25%

TOP CHANGES
▲ WORSE  process_checkout
        crates/api/src/checkout.rs
        cognitive 14 → 19 · cyclomatic 9 → 12 · lines 42 → 57
▼ BETTER  groupOrders
        packages/web/src/orders.ts
        cognitive 28 → 7 · cyclomatic 17 → 5 · lines 91 → 38
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
- `Gemfile` and `*.gemspec`

Several recognized manifests in one directory describe one package with
several ecosystem markers. Files belong to their nearest package. Repositories
without a known manifest get one root package. Git ignore rules apply by
default, along with common generated and dependency directories.

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
- Ruby
- Vue single-file components, including script and template regions

Kotlin files remain visible as unsupported coverage; they are not counted as
healthy. Language dispatch is compiled into the binary. An owned engine can
replace one upstream-backed language without changing Git analysis, health
policy, aggregation, or output.

## JSON

Use `--json` for CI scripts, editors, and other tools:

```console
smackdebt --json
smackdebt diff main --json
```

JSON contains the same report as the terminal view. The top-level object starts
with `schema_version: 1` and includes mode, root, selected scope, a flat path
table, flat scopes, files, findings, diagnostics, comparisons, and the health
summary. Scope entries carry parent and child indexes, finding and comparison
links, coverage, health, and Worse/Better/Changed counts. Files and comparisons
carry indexed path and file ownership where available.
It retains every `watch` and `high` finding while healthy units are represented
by aggregate counts.

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

Command-line values override the project file. Color defaults to automatic:
`NO_COLOR` or `--color never` disables ANSI styling, while `--color always`
keeps it when output is piped. `COLUMNS` overrides detected width. Reports use
full, compact, or stacked layouts as space changes; redirected output defaults
to 100 columns and keeps readable Unicode symbols and bars. JSON is always
unstyled, and an explicit `--color` cannot be combined with `--json`.

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
