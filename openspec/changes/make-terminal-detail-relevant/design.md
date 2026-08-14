## Context

The verdict change defines a clear `QUALITY` result, exact area counts, real-gap
warnings, and content-aware writing. It deliberately leaves detailed selection
to this change. Current main requirements still treat `--all` and path drill as
ways to expose raw static relationships and contextual history. Those facts are
valid report and JSON data, but they make human output longer without adding a
debt decision.

This change is a presentation-selection reduction. It reads existing scope,
finding, architecture-finding, evolution-finding, diagnostic, and child links.
It does not change how any fact is produced, classified, ranked, stored, or
serialized.

## Goals / Non-Goals

**Goals:**

- Give repository, package, directory, and file views the same relevance rules.
- Make default output short and make `--all` mean all useful debt rather than
  all retained facts.
- Keep path selection as scope selection, not an implicit detail mode.
- Trust each unique typed link once in its owning section.
- Select through existing links in one pass without project work or large
  clones.
- Preserve complete JSON, serial/automatic equality, and measured resources.

**Non-Goals:**

- Changing analysis, verdict inclusion, role/trust classification, rank, report
  tables, JSON, diagnostics, or CLI flags.
- Removing raw architecture, weak history, activity, concentration, or healthy
  aggregates from report and JSON.
- Redesigning diff direction, comparison language, or diff section structure;
  that belongs to the third terminal change.
- Defining new default directory file listing beyond the verdict change's
  grouped warning behavior.

## Decisions

### One default relevance policy applies at every scope

Repository, package, directory, and file selections apply the same rules to
their existing scope links:

- `QUALITY` follows the accepted verdict change.
- `AREAS` appears only for several debt-bearing child areas and shows at most
  five in existing stable order.
- `FINDINGS` shows at most three existing ranked source findings linked to the
  scope.
- `ARCHITECTURE` shows at most three linked architecture findings, each with a
  closed stored witness whose final node returns to its first node.
- `HISTORY` shows at most three linked actionable evolution findings.
- Real source gaps and the grouped architecture resolution row follow the
  accepted verdict change.
- At most one discover command points to the first displayed debt-bearing child.

An optional section with no selected row is absent. A selected path changes the
scope and applies this same policy; it never enables raw relationship, weak
history, package activity, concentration, or healthy rows.

### `--all` removes only useful-debt limits

`--all` removes the five-area and three-finding limits for:

- debt-bearing child areas;
- existing scope-linked retained Watch and High source findings, including
  non-primary role and advisory qualifiers already attached to a selected
  finding;
- linked architecture findings and their closed stored witnesses;
- linked actionable evolution findings; and
- real failed/recovered diagnostics required by the verdict change.

Role and trust are display qualifiers on an already selected source finding.
They do not enter verdict arithmetic or create a new inclusion rule. `--all`
does not show healthy rows.

No human view, including `--all` or a selected path, prints raw resolved edges,
ownership, external references, unmatched references, ambiguous references,
weak coupling observations, explained coupling observations, package/file
activity rows, contributor concentration, history coverage fields, or history
processing facts. The single grouped architecture warning remains available.

A source finding's existing commit count is useful finding evidence and can be
shown. A closed cycle witness and an actionable coupling finding's shared
commits, union commits, and similarity remain useful evidence. The actionable
coupling row ends exactly with `not linked in code`; the old final phrase is
absent.

### Human history selects actionable finding IDs only

Terminal history selection iterates the selected scope's existing actionable
evolution-finding IDs in stable presentation order. Default takes the first
three; `--all` takes every linked actionable ID. It does not scan coupling,
activity, concentration, or history-coverage tables for extra terminal rows.
Weak and explained observations remain JSON-only even under `--all` and path
selection.

### Owning link lists are unique before rendering

Analysis/report aggregation owns unique typed IDs within each scope's child,
source-finding, architecture-finding, evolution-finding, and diagnostic link
list. Index integrity audits reject a duplicate ID inside one owning list. The
terminal trusts the completed lists and visits each linked ID once. It does not
create a de-dup set, second collection, identity map, or silent data rewrite.

Accepted current fixtures must already satisfy this invariant, so strengthening
the audit preserves their exact JSON bytes. A negative report/index integrity
test inserts a duplicate scope link and fails before rendering. A fact linked
once in different ancestor scopes can appear once in each separately rendered
scope. Within one rendered scope and section, every linked typed ID appears
once. Empty headings, empty tables, omitted-row bookkeeping, and repeated
discover links are absent.

History makes the same ownership explicit: only the unique actionable
evolution-finding ID list is selected. Descriptive package activity and context
tables are never selected, so indistinguishable package/activity rows cannot
be introduced and then hidden by renderer de-duplication.

### Selection borrows existing links in one pass

Default selection keeps only its small display limits. `--all` streams or
borrows the complete relevant linked slice. Neither mode builds a second report,
de-dup set, identity map, or selected-ID collection; clones large path/finding
collections; scans unrelated global tables for every scope; or performs
filesystem, Git, parser, analysis, or worker work. Each applicable link is
visited once, preventing N+1 per-row lookup patterns.

### Public proof uses dense scope fixtures

Generated dense fixtures cover repository, package, directory, and file views
with default and `--all` at widths 120, 100, 80, and 50. Exact snapshots prove
limits, closed witnesses, actionable history evidence, qualifiers, warnings,
one discover command, no empty sections, one row per completed unique link, and
absence of every excluded family and the old dependency phrase.

For the reviewed dense default public codebase and selected package, directory,
and file path fixtures, results at widths 120, 100, and 80 have at most 60
nonempty lines; width 50 has at most 100. `--all` snapshots remain exact at
every width and keep the excluded-family audit, but they have no nonempty-line
budget because every linked useful-debt row must remain visible.

Serial and automatic terminal bytes match. JSON version-3 bytes validate exact
schema, indexes, privacy, and completeness. Instrumentation proves rendering
does no project work; allocation and resource profiles do not regress.

Read-only review covers self, a private mixed application, and a private Rust
workspace. Committed review data contains only workload family and aggregate
outcomes, never raw private output, names, paths, source, identities, or history.

## Risks / Trade-offs

- **Useful detail is narrower than retained data.** Help and README point users
  to complete JSON for relationships, weak observations, and context.
- **`--all` no longer means every fact.** Its explicit product meaning is all
  useful debt, which matches the human decision flow.
- **Scope views can look similar.** Their value is the selected scope and linked
  facts; consistent rules make drill behavior predictable.
- **Default fixture budgets could be mistaken for all-detail limits.** Tests
  apply budgets only to default codebase/path results; `--all` has no line cap
  and removes every useful-debt limit.

## Migration Plan

1. Accept and strict-validate this change while the other terminal changes can
   also be authored.
2. Wait until `make-terminal-verdict-clear` is reviewed and archived.
3. Add pure selection, owning-link integrity, one-pass, and no-work tests.
4. Implement default and `--all` selection through existing report links.
5. Update help, README, architecture guidance, and guarded exact snapshots.
6. Run public unchanged-machine/resource proof and aggregate workload review.
7. Review and archive this change before implementing
   `make-diff-output-debt-focused`.
8. Keep `prepare-first-release` blocked until all three terminal changes are
   archived.

Rollback restores the earlier terminal selection and snapshots. Report, JSON,
and analysis data require no migration.
