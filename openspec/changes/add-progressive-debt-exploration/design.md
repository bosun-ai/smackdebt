## Context

The report already stores flat scopes with parent and child indexes, aggregate
health, coverage, and retained Watch and High findings. Codebase output ignores
the child scopes and prints up to ten findings across the whole selection. Diff
output creates only repository and file scopes, and a comparison has no owning
file index. An explicit codebase path also becomes a new inventory root, so its
displayed paths no longer match a root report.

The first public release has not been frozen. This is the right point to make
progressive exploration part of the CLI and JSON contracts. The model must also
support a later interactive terminal interface without coupling core policy to
terminal state.

## Goals / Non-Goals

**Goals:**

- Let users see where debt or worktree change is concentrated before inspecting
  individual units.
- Keep every count and percentage traceable to report facts.
- Use one hierarchy and one aggregation policy for terminal, JSON, and future
  renderers.
- Preserve stable repository-relative scope identity across root and path-limited
  invocations.
- Keep a path-limited invocation proportional to the selected area.
- Preserve deterministic output and the existing memory and process contracts.

**Non-Goals:**

- Build an interactive terminal interface, web view, or editor integration.
- Add a combined debt score, estimated remediation time, or hidden weighting.
- Persist reports or add a source-analysis cache.
- Add historical trends, architecture components, ownership, coupling, or policy
  gates.
- Change health thresholds, measurements, activity collection, or diff matching.

## Decisions

### Keep navigation state outside the report

`Report` owns the complete facts produced by one invocation. `ProjectReport`
also identifies the initial selected scope. Renderers accept a report and a
scope index, so moving to another scope in the same report is a pure read. The
report does not carry a mutable cursor, expanded-row state, terminal width, or
presentation limits.

A root invocation contains the full project hierarchy. A future interactive
terminal interface can retain that report and move between scope indexes without
filesystem, Git, parser, or worker activity. A path-limited invocation contains
only its selected area and ancestors required to preserve identity.

### Own each report path once

The finished report owns one flat table of repository-relative paths. Scopes and
files refer to paths by typed index instead of each owning another string.
Discovery remains responsible for validating repository-relative paths and
package roots. Project construction transfers each selected identity once at the
adapter edge; analysis policy never reads filesystem path types.

Repository, package, directory, and file scopes stay in one flat table. Findings
remain stored once. Comparisons gain an owning file index and remain stored once.
Scopes hold indexes for descendant findings and comparisons so renderers do not
rescan global tables.

### Build the same hierarchy for codebase and diff

Both modes use repository, package, directory, and file scopes. Codebase package
roots come from the existing inventory. Diff collects candidate package roots by
inspecting each distinct ancestor directory of changed paths at most once. It
also retains changed manifest paths before non-source paths are filtered, so a
deleted or renamed manifest can still identify the former package directory.

Current files and rename destinations use their worktree path. Deleted files use
their previous path. Every file belongs to its nearest discovered package root;
when none exists, it belongs to the repository package.

One shared post-order pass aggregates coverage, health, finding indexes,
comparison indexes, and diff outcome counts. Child scopes partition their
parent, so displayed shares do not double count nested work.

### Preserve repository identity for selected paths

Inside Git, an explicit path resolves against the repository root. Discovery
walks only the selected directory, or reads only the selected file, while scope
paths retain the repository-relative prefix. Package detection inspects the
selection's ancestor directories once so the nearest enclosing package remains
visible. Outside Git, the selected directory acts as the identity root as it does
today.

An existing selected directory with no supported source still produces a valid
selected scope with zero totals. A missing path remains an invocation error.
Shares in a path-limited report use that selected scope as the denominator; they
do not claim project-wide percentages.

### Make codebase distribution exact and severity-led

For a selected codebase scope, each row represents one child after display-only
single-child skipping. Columns show High, Watch, Healthy when width permits, and
debt share. Debt share is:

`(child.high + child.watch) / (selected.high + selected.watch)`

The renderer rounds to the nearest whole percent. A zero denominator produces
`0%`. Rows sort by High descending, Watch descending, then repository-relative
path ascending. Exact counts remain authoritative when rounded shares do not sum
to 100%.

The renderer passes through structural scopes while the current display scope
has exactly one child and is not a file. It prints the passed path as a
breadcrumb, then lists the first scope with several children or the resulting
file. A package rooted at `.` is display-only structure and does not produce an
empty row or drill command.

The default shows at most ten rows. Debt-bearing children come first. Healthy-
only children fill unused rows. When rows remain, the report states separate
counts for omitted debt-bearing and healthy-only children. `--all` removes the
row limit.

Non-file views show the three highest-ranked retained findings within the
selected scope, using the existing health, activity, measurement, path, and span
order. File views show every retained finding, grouped by container when one is
present, with a copyable repository-relative `path:line` location. `--all` shows
all retained findings in non-file views too.

The `Explore` command selects the first displayed debt-bearing child. It is
omitted when no deeper debt-bearing scope exists.

### Summarize diff outcomes without losing detail

Every retained comparison contributes to exactly one displayed direction:

- `worse`: `Regressed`, or `Added` with an after rating of Watch or High;
- `better`: `Improved`, or `Removed` with a before rating of Watch or High;
- `changed`: `MetricChanged`, `Ambiguous`, healthy `Added`, or healthy `Removed`.

Directory rows show Worse, Better, Changed, and share. Share is the child's sum
of those three counts divided by the selected scope's sum. It uses nearest whole
percent and `0%` for a zero denominator. Rows sort by Worse descending, Better
descending, Changed descending, then path ascending.

The selected-scope detail lists up to three comparisons by direction priority:
Worse, Better, then Changed, with stable path and source order inside each
direction. File views and `--all` expose every retained comparison. Existing
detailed kinds, before and after measurements, and rating transitions remain
available in file detail and JSON.

### Extend JSON version 1 additively

JSON stays a complete, untruncated serialization of the report. It gains:

- top-level `selected_scope`, containing a scope index;
- a top-level path table and a path index on each scope and file;
- finding indexes and comparison indexes on scopes;
- Worse, Better, and Changed counts on scopes;
- an owning `file` index and derived `direction` on every comparison.

Existing keys, types, and meanings remain unchanged, including existing path and
name strings. The new indexed fields are additive. Terminal-only `--all` cannot
be combined with `--json`, because JSON is already complete.

### Keep display policy in output

Analysis owns exact facts and the three-way diff direction policy. Project owns
path resolution, hierarchy construction, package assignment, and the initial
scope. Output owns single-child skipping, row limits, ranking for display,
percentage formatting, width adaptation, breadcrumbs, and drill commands. The
CLI owns argument conflicts and passes the initial scope to output.

## Risks / Trade-offs

- **More scope links increase report memory**: Store typed indexes, reserve from
  child counts before aggregation, and measure generated large reports.
- **Path-table additions overlap existing string fields**: Keep old JSON fields
  for version 1 compatibility while removing duplicate ownership from the Rust
  report model through borrowed serializer views.
- **Diff package roots can change with manifests**: Include current ancestor
  manifests plus changed current and previous manifest paths, and cover delete
  and rename cases.
- **Smart skipping can hide structural levels**: Print the passed breadcrumb and
  keep the complete scope tree in JSON.
- **Rounded shares may not total 100%**: Treat exact counts as the source of truth
  and document integer rounding.
- **A path-limited report cannot navigate to siblings**: State that shares apply
  to the selection; root analysis remains the path for whole-project navigation.

## Migration Plan

1. Add core path, selected-scope, comparison ownership, and aggregate values.
2. Unify codebase and diff hierarchy construction and selected-path identity.
3. Add progressive terminal rendering and `--all` validation.
4. Extend JSON version 1 and update API and acceptance snapshots.
5. Update product and architecture documentation and establish release evidence.

No stored data migration is needed. If implementation evidence invalidates the
design, revert this private change before the first-release compatibility notes
are frozen.

## Open Questions

None.
