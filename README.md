# Smackdebt

**Find the code that hurts, then see whether your change made it better.**

Smackdebt turns source metrics and Git history into two short reports:

- How bad is this codebase, and where are the hotspots?
- How did this worktree change compared with a Git ref?

Run it without configuration. Start at the repository summary, then pass one of
the reported paths to inspect a package, directory, file, or function.

Every report says its answer out loud in words, so the same bytes serve a human
at a terminal and a script or a model reading a pipe.

## Install

```console
cargo install --path crates/cli
```

Smackdebt runs locally and does not upload source code or analysis data.

## Check a codebase

Run `smackdebt` from anywhere inside a repository:

```console
$ smackdebt

smackdebt · repository root
This code fights back.
12 high · 94 watch · 1,686 checked
worst: crates/api/src/checkout.rs — hot AND complex

AREAS
  crates/api · 4 high · 8 watch
  packages/web · 2 high · 9 watch
  crates/core · 1 high · 6 watch

FINDINGS
high process_checkout · function
        crates/api/src/checkout.rs:42
        cognitive 31 · cyclomatic 14 · statements 126 · hot (14 commits)

ARCHITECTURE
high package dependency cycle
        crates/api/src/routes.rs
        → crates/core/src/orders.rs
        → crates/api/src/routes.rs
watch crates/api → crates/core
        depends on less stable code · instability 1/4 → 2/3 · 3 imports

HISTORY
watch crates/api ↔ packages/web changed together in 8 of 10 commits · 80% · no code dependency
watch one contributor made 34 of 36 commits to crates/api

WARNINGS
warning 29 imports could not be followed.

next: smackdebt crates/api
```

The report opens with a verdict block: the selected scope, the sentence for its
tier, the counts behind that sentence with every count labeled by its word, and
the single worst thing with its repository-relative path and the reason it is
worst.

The codebase tiers are fixed. Their identifiers are the stable vocabulary for an
integration; their sentences are what a person reads:

| Tier | Sentence |
| --- | --- |
| `empty` | Nothing was checked. |
| `clean` | Clean. Ship it. |
| `solid` | Solid, with rough edges. |
| `worn` | Worn in the usual places. |
| `fights_back` | This code fights back. |
| `lost` | The code is winning. |

Smackdebt combines fixed code-health limits with recent activity. A complex
function that changes often rises to the top and says `hot (n commits)`. A
complex function in quiet code still appears as debt, but does not outrank
active risks just because it is large.

`AREAS` appears when debt spans several child paths and shows at most five. The
first path is the next useful place to inspect, and `next:` names the exact
command. Empty `AREAS`, `ARCHITECTURE`, `HISTORY`, and `WARNINGS` sections stay
out of the way.

Pass a reported path to progressively inspect the next level. Repository,
package, directory, and file scopes retain the same repository-relative paths:

```console
$ smackdebt crates/api/src/checkout.rs

smackdebt · crates/api/src/checkout.rs
This code fights back.
1 high · 1 watch · 18 checked
worst: crates/api/src/checkout.rs — hot AND complex

FINDINGS
high process_checkout · function
        crates/api/src/checkout.rs:42
        cognitive 31 · cyclomatic 14 · statements 126 · hot (14 commits)
```

The default terminal view shows at most five affected areas and three findings.
Use `--all` for all useful debt without those limits. `--all` is not an export:
raw dependency edges, references outside the repository, churn totals,
cyclomatic-one values, weak coupling, and healthy rows never appear in any
terminal view. JSON keeps every one of them, so `--all --json` is rejected.

The default history window is 90 days. It governs every history-derived number,
including activity, churn, change coupling, and contributor concentration:

```console
smackdebt --history 180d
smackdebt --jobs 4
```

## Check a change

Compare the current worktree with the branch it came from:

```console
$ smackdebt diff

smackdebt diff · repository root
Better here, worse there.
worse 3 (source, architecture) · better 1 (source) · changed 0
worst: crates/api/src/routes.rs — package dependency cycle

AREAS
  crates/api · worse 2 · better 1
  packages/web · worse 1

FINDINGS
worse process_checkout · function
        crates/api/src/checkout.rs:42
        cognitive 14 → 19 · cyclomatic 9 → 12 · statements 42 → 57
better groupOrders · function
        packages/web/src/orders.ts:18
        cognitive 28 → 7 · cyclomatic 17 → 5 · statements 91 → 38
worse GraphEditor.vue · closure
        packages/web/src/GraphEditor.vue:1177
        added

ARCHITECTURE
worse package dependency cycle introduced
        crates/api
        → crates/core
        → crates/api
```

Every diff count is labeled with its word and printed even when it is zero, and
the families that moved are named beside the count. Each changed finding shows
`path:line` and each changed measurement as a before and after value. A unit
without a source name of its own is written as its file and its kind, such as
`GraphEditor.vue · closure`, so no generated internal identity reaches a reader.

The diff tiers are fixed in the same way:

| Tier | Sentence |
| --- | --- |
| `no_debt_change` | No debt changed. |
| `better` | You made it better. |
| `worse` | You made it worse. |
| `mixed` | Better here, worse there. |

Adding or deleting healthy code moves no debt. A diff that moves no debt prints
the verdict block and nothing else:

```console
$ smackdebt diff

smackdebt diff · repository root
No debt changed.
worse 0 · better 0 · changed 0
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

## Read the report

Severity, direction, and diagnostics are words: `high`, `watch`, `worse`,
`better`, `changed`, `warning`, and `next:`. Every meaning is readable from the
words alone.

In a terminal, each word may be decorated with a one-cell Nerd Font glyph, and
the verdict sentence with a tier-colored `▌` bar. Decoration is resolved from
the same terminal detection as color and has no option of its own: there is no
icon option, no emoji mode, and no theme setting. A glyph always sits beside the
word it decorates and never replaces it.

Piped output is the same report in words: no glyph, no bar, no ANSI, and no
codepoint in the private-use range U+E000–U+F8FF. That makes the output safe to
read with another program without a font or a terminal.

`--color never` produces fully plain output; `--color always` keeps decoration
and styling through a pipe; `NO_COLOR` removes styling while a terminal keeps
its glyphs. Styling reaches decoration only, so removing ANSI sequences yields
the plain bytes exactly.

`COLUMNS` overrides the detected width, and redirected output defaults to 100
columns. Each row chooses its own shape: it stays on one line when its content
fits and stacks its facts on indented lines when it does not. Nothing is
silently clipped at any width — measurements, counts, cycle witnesses, history
evidence, commands, and identities all survive, and a long path continues on the
next line instead of being shortened.

## Read the ratings

Smackdebt reports separate signals instead of hiding them in one score.

| Signal | Watch | High |
| --- | ---: | ---: |
| Cognitive complexity | 15 | 25 |
| Cyclomatic complexity | 11 | 21 |
| Statements in a function | 50 | 100 |
| Maximum nesting depth | 4 | 7 |
| Declared parameters | 6 | 9 |

A function takes its highest signal rating. Terminal views focus on functions
that need attention. JSON retains healthy, watch, and high counts for every
scope. Neither view averages one bad function into a reassuring project score.

Cognitive complexity starts at zero. A control-flow break adds one plus its
nesting depth. Alternatives and labeled jumps add one. A run of `&&` or `and`
adds one, and changing that run to `||` or `or` adds another. A separately
rated closure or nested function does not increase its parent's value.
Recursion is not inferred from syntax alone.

Maximum nesting depth is the deepest level reached inside one rated unit,
counted from the same nesting events cognitive complexity uses. Parameter count
is the number of parameters a unit declares. A separately rated closure reports
its own depth and does not deepen its parent.

Cyclomatic complexity starts at one and adds one for each independent decision.
Because it starts at one, a cyclomatic value of one is never printed.
Statements are counted by syntax, not physical lines: `a(); b();` counts as
two, while one statement spread over several lines counts as one. Blank lines,
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

Smackdebt does not guess when identity is unclear. Unmatched and ambiguous
references are counted in one grouped warning sentence, their per-file detail
stays in `--all` and path views, and JSON retains their locations and reasons.

A reference written against the name a package declares for itself resolves
inside the repository. When no repository path matches a reference, Smackdebt
compares its first segment with the names packages declare in their manifests:
`Cargo.toml` `[package] name` with a `[lib] name` override, `package.json`
`name` including a scoped `@scope/name`, `pyproject.toml` `[project] name`, and
a gemspec name. Rust treats hyphens and underscores as the same character; other
ecosystems compare declared names exactly. The reference becomes an internal
`uses` edge only when exactly one package in the repository declares that name.
When two packages declare the same name, the reference stays unresolved with its
diagnostic instead of guessing. Smackdebt reads declared names only: it never
executes build configuration and never emulates lockfiles, resolver algorithms,
workspace inheritance, or version constraints.

A cycle crossing packages is High. A file cycle contained in one package is
Watch. A cycle prints its closed witness one step per line, so the evidence is
never shortened away. Fan-in is the number of packages that depend on a package;
fan-out is the number it depends on. Instability is `fan-out / (fan-in +
fan-out)` and is printed as its exact integer fraction, so a package that
depends on less stable code reads `instability 1/4 → 2/3`.

Code and architecture results remain separate. Lower source complexity does not
cancel an introduced package cycle. In diff output an introduced cycle is
worse, a removed cycle is better, and an ordinary edge change is changed.

Pass a package, directory, or file path to inspect its incoming and outgoing
relationships. Incoming references remain visible even when their source is
outside the selected path:

```console
smackdebt crates/core
smackdebt --all crates/core
```

Human relationship rows stay direct: `source → target · 1 import`,
`source owns target`, `could not be matched`, or `matched more than one file`.
Non-primary roles and advisory evidence appear only when they change how the row
should be read.

Static analysis does not provide compiler type resolution, runtime tracing, or
executed build configuration. Macros, generated paths, runtime imports, and
unsupported aliases can therefore remain unresolved.

## Read the history evidence

The `HISTORY` section uses locally available non-merge Git history inside the
selected `--history` window. The window governs every history-derived number:
activity, churn, change coupling, and contributor concentration all describe the
same commits. History keeps its evidence separate from current code health and
the static dependency graph:

- activity counts distinct commits that changed a current file or package;
- churn reports textual lines added and deleted;
- change coupling reports packages that changed in the same commits;
- contributor count reports how many normalized contributors changed a
  package;
- top-contributor share reports the largest contributor's package commits over
  all contributor commits for that package.

Coupling uses Jaccard similarity: shared commits divided by commits touching
either package. Default findings require at least three shared commits and 20%
similarity. Weaker observations stay in JSON only. Recurrent coupling without a
static dependency in either direction is Watch because it can reveal a missing
or unclear package relationship. Coupling that matches a static edge remains
descriptive and appears in `--all` and path views. Knowledge concentration is a
Watch row stating counts only, such as
`one contributor made 34 of 36 commits to crates/api`.

Default `HISTORY` shows at most three actionable rows. It orders coupling by
shared commits, then similarity, then stable package identity. Each row includes
the shared and total commit counts, similarity, and whether a code dependency
exists. Churn totals stay in JSON; the terminal reports activity where it
changes a decision, as `hot (n commits)` on a finding.

A package pair is reported once. Source role and trust variants are aggregated
into that one row, and per-role history stays in JSON. A scope and its own
ancestor never form a pair, because commits they share are structural rather
than hidden coupling. `no code dependency` means no trusted eligible `uses`
relation exists in either direction, including relations resolved through a
declared manifest name.

Smackdebt follows detected renames back from files that still exist and assigns
their history to the files' current packages. It does not reconstruct removed
files or old package layouts. Binary changes count as commits without invented
line totals. Shallow, partial, empty, or unavailable history is stated in the
report while source and static architecture results remain usable.

Contributor names, addresses, and internal identities stop before the report.

## Checked command examples

These short examples run against generated public repositories in the release
evidence. Each comment declares the exact exit status, empty stderr, and the
stable stdout fragments that must appear in the stated order.

<!-- smackdebt-example fixture=evolution status=0 stderr=empty stdout=smackdebt_·_repository_root|checked|HISTORY|_a_↔_b -->
```console
smackdebt --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=worktree-change status=0 stderr=empty stdout=smackdebt_diff_·_repository_root|worse|better|changed|AREAS|FINDINGS|ARCHITECTURE -->
```console
smackdebt diff main --color never --jobs 1 --history 36500d
```
Terminal and JSON output contain only aggregate contributor counts and
concentration operands. History and all other analysis stay on the local
machine.

Diff reports show history as existing context for changed files and packages.
Historical values are not labelled better or worse. When a worktree dependency
changes the finding, the terminal says the packages `now change together
without a code dependency` or `no longer change together without a code
dependency`. The retained history values do not change.

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

The object answers the common question first, so `smackdebt --json | head`
is useful on its own and no consumer joins a table to learn whether the code is
in trouble:

```json
{
  "schema_version": 4,
  "mode": "codebase",
  "verdict": { "tier": "worn", "sentence": "Worn in the usual places.", "mode": "codebase" },
  "summary": {
    "checked": 2166, "high": 17, "watch": 58, "high_architecture": 0,
    "debt_diff": { "worse": 0, "better": 0, "changed": 0, "total": 0 },
    "worst": [
      { "path": "crates/project/src/project.rs", "name": "analyze_diff",
        "container": null, "unit_kind": "function", "reason": "hot_and_complex" }
    ]
  }
}
```

`verdict.tier` is the frozen id and `verdict.sentence` is the exact sentence the
terminal prints for it. In a diff the tier and sentence are the diff answer and
`summary.debt_diff` states each count with its word, including zero counts.
`summary.worst` names at most three offenders with real repository-relative
path strings. Every value in the head also exists in a table, and both come from
the same completed report.

Behind the head, JSON contains the complete report, including everything the
terminal leaves out: raw dependency edges, references outside the repository,
churn totals, weak coupling, hotspots, size findings, orphan files, and healthy
counts. One package table owns stable package IDs, repository-relative paths,
current or base-only presence, and the name a manifest declares; machine path
`.` stays unchanged even though terminal output calls it `repository root`.
Files expose SourceRole, parse outcome, and trust. Recovered Watch and High
facts remain advisory in JSON and `--all` without entering health, default
findings, architecture verdicts, coupling, or diff verdicts.

Static relations identify `uses` or `module_ownership` independently from
role, trust, resolution, and source spans. History rows expose role-aware churn,
coupling operands, and contributor concentration without contributor identity.
Hotspots, size findings, orphan files, stable-dependency findings, and
knowledge-concentration findings each own a table and state their own `kind`.

Every serialized value is an integer or a string. Coupling similarity and
concentration share are published as their integer operands — shared and union
commits, numerator and denominator — rather than as ratios, so a consumer
derives any ratio at whatever precision it wants and no floating-point value
appears anywhere in the object.

Terminal limits never remove JSON facts. The checked schema is
[`schemas/report-v4.schema.json`](schemas/report-v4.schema.json); no earlier
schema is emitted.

Source findings are ordered by rating, role class, hot state, count of signals
at that rating, total triggered signals, cognitive complexity, cyclomatic
complexity, logical lines, activity, path, and span. Role class places primary
source before non-primary source at equal rating, and non-primary source stays
visible below it rather than being removed. Nothing is filtered on role, so a
repository whose only rated debt is non-primary still names it as the worst
offender rather than reporting nothing. Hot state decides next, so among
production findings of the same rating the file being edited comes first. Hot
means a rated file whose touch count inside the selected history window reaches
the minimum touch count, five by default. Findings show unit kind and any
non-primary role.

Finding debt does not fail the command. Exit codes describe whether Smackdebt
could produce a report:

| Code | Meaning |
| ---: | --- |
| 0 | Report produced |
| 1 | Analysis could not produce a report |
| 2 | Invalid arguments or configuration |

Three common mistakes get one exact line on standard error, an empty standard
output, and no usage tail:

```console
smackdebt: path not found: does/not/exist
smackdebt: Git ref not found: no-such-ref
smackdebt: --all cannot be used with --json
```

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

[thresholds.file_lines]
watch = 400
high = 800

[thresholds.container_lines]
watch = 300
high = 600

[thresholds.nesting]
watch = 4
high = 7

[thresholds.parameters]
watch = 6
high = 9

[hotspots]
minimum_touches = 5
```

Command-line values override the project file. JSON is always unstyled and
undecorated, and an explicit `--color` cannot be combined with `--json`.

## Design

Read [ARCHITECTURE.md](ARCHITECTURE.md) for the domain model, data flow,
aggregation rules, Git behavior, and language extension path. OpenSpec records
each implementation change under [`openspec/changes`](openspec/changes).

## Credits

Smackdebt uses tree-sitter and its language grammars for syntax trees. Smackdebt
owns the measurement rules, repository discovery, health policy, Git context,
aggregation, comparison, and reports.
