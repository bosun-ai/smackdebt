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

ARCHITECTURE
142 internal · 86 external · 3 unresolved · 1 ambiguous
▲ 1 high · ● 2 watch · 142 internal edges
▲ HIGH  package dependency cycle
        crates/api/src/routes.rs → crates/core/src/orders.rs → crates/api/src/routes.rs

EVOLUTION
184 files · 3,812 commits · 127,440 lines of churn
● 1 unexplained coupling · 12 retained package pairs
● WATCH  packages change together without a static dependency
         crates/api ↔ packages/web · 18/31 shared commits · 58% similarity

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

ARCHITECTURE CHANGE
▲ WORSE 1 · ▼ BETTER 0 · ● CHANGED 2
▲ WORSE  package dependency cycle introduced
        crates/api → crates/core → crates/api
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

Cognitive complexity starts at zero. A control-flow break adds one plus its
nesting depth. Alternatives and labeled jumps add one. A run of `&&` or `and`
adds one, and changing that run to `||` or `or` adds another. A separately
rated closure or nested function does not increase its parent's value.
Recursion is not inferred from syntax alone.

Cyclomatic complexity starts at one and adds one for each independent decision.
Logical lines count statements, not physical lines: `a(); b();` counts as two,
while one statement spread over several lines counts as one. Blank lines,
comments, wrappers, markup, and nested rated units do not count.

These equivalent functions each have cognitive complexity 1, cyclomatic
complexity 2, and two logical statements:

```rust
fn choose(ready: bool) -> i32 {
    if ready { return 1; }
    0
}
```

```ruby
def choose(ready)
  return 1 if ready
  return 0
end
```

Recent activity comes from distinct non-merge commits that touched a file in
the selected history window. Smackdebt uses activity to order code that already
needs attention; activity does not change the code-health rating.

## Read the architecture

The same command also reports static dependency health. Smackdebt extracts
imports, includes, modules, and requires during the source parse, then resolves
them against discovered repository files:

- `internal` references identify exactly one file in the repository;
- `external` references name code outside the repository;
- `unresolved` references are dynamic or malformed;
- `ambiguous` references match several possible internal files.

Smackdebt does not guess when identity is unclear. The unresolved and ambiguous
counts stay visible in terminal output, and JSON retains their locations and
reasons.

A cycle crossing packages is High. A file cycle contained in one package is
Watch. Fan-in is the number of packages that depend on a package; fan-out is the
number it depends on. Instability is `fan-out / (fan-in + fan-out)` and is absent
for an isolated package. Degree and instability describe graph shape and do not
receive a health label.

Code and architecture results remain separate. Lower source complexity does not
cancel an introduced package cycle. In diff output an introduced cycle is
Worse, a removed cycle is Better, and an ordinary edge change is Changed.

Pass a package, directory, or file path to inspect its incoming and outgoing
relationships. Incoming references remain visible even when their source is
outside the selected path:

```console
smackdebt crates/core
smackdebt --all crates/core
```

Static analysis does not provide compiler type resolution, runtime tracing, or
executed build configuration. Macros, generated paths, runtime imports, and
unsupported aliases can therefore remain unresolved.

## Read the evolution evidence

The `EVOLUTION` section uses all locally available non-merge Git history. The
`--history` option still limits only the recent activity used to order source
findings. Evolution keeps its evidence separate from current code health and
the static dependency graph:

- touches count distinct commits that changed a current file or package;
- churn reports textual lines added and deleted;
- change coupling reports packages that changed in the same commits;
- contributor count reports how many normalized contributors touched a
  package;
- top-contributor share reports the largest contributor's package touches over
  all contributor touches for that package.

Coupling uses Jaccard similarity: shared commits divided by commits touching
either package. Default findings require at least three shared commits and 20%
similarity. Weaker observations remain available in JSON and `--all` output.
Recurrent coupling without a static dependency in either direction is Watch
because it can reveal a missing or unclear package relationship. Coupling that
matches a static edge remains descriptive. Churn, contributor count, and
contributor concentration do not receive health labels.

Smackdebt follows detected renames back from files that still exist and assigns
their history to the files' current packages. It does not reconstruct removed
files or old package layouts. Binary changes count as touches without invented
line totals. Shallow, partial, empty, or unavailable history is stated in the
report while source and static architecture results remain usable.

Contributor names, addresses, and internal identities stop before the report.

## Checked command examples

These short examples run against generated public repositories in the release
evidence. Each comment declares the exact exit status, empty stderr, and the
stable stdout fragments that must appear in the stated order.

<!-- smackdebt-example fixture=evolution status=0 stderr=empty stdout=QUALITY|ARCHITECTURE|EVOLUTION -->
```console
smackdebt --color never --jobs 1
```

<!-- smackdebt-example fixture=worktree-change status=0 stderr=empty stdout=CHANGE|ARCHITECTURE_CHANGE|EVOLUTION_CONTEXT|TOP_CHANGES -->
```console
smackdebt diff main --color never --jobs 1 --history 36500d
```
Terminal and JSON output contain only aggregate contributor counts and
concentration operands. History and all other analysis stay on the local
machine.

Diff reports show evolution as existing context for changed files and packages.
Historical values are not labelled Better or Worse. A worktree dependency can
remove an unexplained-coupling finding, but it does not change the retained
history values.

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
default, along with explicit exclusions and dependency directories. A supported
file is not ignored only because its directory looks generated.

Every selected file has one source role: primary, test, example, benchmark,
fixture, or generated. Classification checks explicit `source_roles`
configuration first, then language-owned generated markers, generic filenames
and paths, and finally primary. Different matches at the same level are an
invalid configuration and exit with status 2. Primary, test, example, and
benchmark source affect default verdicts. Fixture and generated source remain
visible for inspection without affecting those verdicts.

Files that cannot be parsed stay visible in the coverage summary. Smackdebt
does not quietly count them as healthy.

## Languages

One tree-sitter source engine supports:

- C and C++
- Java
- JavaScript and JSX
- Python
- Rust
- TypeScript and TSX
- Ruby
- Vue single-file components, including script and template regions

Kotlin files remain visible as unsupported coverage; they are not counted as
healthy. Language dispatch is compiled into the binary. Each language translates
its own syntax into the same cognitive, cyclomatic, and logical-statement rules.
Exact fixtures check units, recovery, spans, nesting, and measurements before a
language is listed here.

## JSON

Use `--json` for CI scripts, editors, and other tools:

```console
smackdebt --json
smackdebt diff main --json
```

JSON contains the complete report behind the terminal view. The top-level
object starts with `schema_version: 3`. One package table owns stable package
IDs, repository-relative paths, and current or base-only presence; machine path
`.` stays unchanged even though terminal output calls it `repository root`.
Files expose SourceRole, parse outcome, and trust. Recovered Watch and High
facts remain advisory in JSON and `--all` without entering health, default
findings, architecture verdicts, coupling, or diff verdicts.

Static relations identify `uses` or `module_ownership` independently from
role, trust, resolution, and source spans. History rows expose role-aware churn,
coupling operands, and contributor concentration without contributor identity.
Terminal limits never remove JSON facts. The checked schema is
[`schemas/report-v3.schema.json`](schemas/report-v3.schema.json); no earlier
schema is emitted.

Source findings are ordered by rating, count of signals at that rating, total
triggered signals, cognitive complexity, cyclomatic complexity, logical lines,
activity, path, and span. Findings show unit kind and any non-primary role.
Default architecture output shows rated cycle witnesses rather than arbitrary
edge samples. Use `--all` or a path drill to inspect relevant resolved,
unresolved, ambiguous, ownership, and advisory relations.

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

[source_roles]
test = ["spec/**"]
example = ["examples/**"]
benchmark = ["benches/**"]
fixture = ["testdata/**"]
generated = ["src/client/generated.rs"]

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

Smackdebt uses tree-sitter and its language grammars for syntax trees. Smackdebt
owns the measurement rules, repository discovery, health policy, Git context,
aggregation, comparison, and reports.
