## Context

One completed report already contains ranked source, architecture, history, and
diff facts. The terminal renderer currently prints several summaries of those
same facts, decorates icons and words together, and exposes processing evidence
that belongs in JSON or diagnostics. The change is a presentation reduction;
it must not reclassify or rerank analysis facts.

## Goals / Non-Goals

**Goals:**

- Make the leading human output answer what needs attention and what to inspect
  next.
- Give each status one exact, one-cell glyph and color only the glyph.
- Keep default output relevant and `--all` useful without showing healthy rows
  or internal processing facts.
- Use simple help, error, warning, and README language.
- Preserve JSON bytes, analysis behavior, ordering, work counts, allocation
  behavior, and serial/automatic equality.

**Non-Goals:**

- Changing ratings, rank, scope selection, graph or history algorithms, JSON,
  exit codes, or public resource limits.
- Adding an icon selector, emoji, an ASCII fallback, or editing a user's
  terminal configuration.
- Making terminal output a complete data export.

## Decisions

### One private glyph vocabulary

The output crate owns one private glyph value for each meaning:

| Meaning | Glyph | Code point | Glyph color |
| --- | --- | --- | --- |
| High | `` | U+F024 | red |
| Watch | `` | U+F0EB | ANSI-256 208 orange |
| Discover | `` | U+F46B | cyan |
| Worse | `` | U+F062 | red |
| Better | `` | U+F063 | green |
| Changed | `` | U+F111 | normal |
| Warning | `` | U+F071 | ANSI-256 208 orange |

The renderer writes a glyph with its role, resets styling immediately, then
writes the remaining text normally. Severity and direction words do not repeat
the glyph meaning. U+EC3F is rejected. Tests assert exact scalar values,
one-cell display width, ANSI placement, and plain output after stripping ANSI.

### Relevance decides section presence

`QUALITY` always appears. `AREAS` appears only when several affected child
areas exist and shows at most five. `FINDINGS` shows the first three existing
ranked findings. `ARCHITECTURE` appears only for architecture findings and
shows cycle witnesses in the default view. `HISTORY` appears only for
actionable history findings, with at most three ordered by shared commits
descending, similarity descending, then stable package names and IDs. Each
actionable row states shared commits, union commits, similarity, and that no
code dependency exists. The discover line is only the discover glyph and the
next command.

Repeated bars, summary percentages and ratios, healthy counts, severity words,
edge totals, weak pairs, history processing totals, and omitted-row bookkeeping
are removed from default output. Actionable history keeps its short evidence
ratio. `--all` removes count limits but still shows only useful findings and
relevant relationships. It does not restore healthy rows or internal processing
facts. A path view keeps relevant incoming and outgoing relationships. Human
activity uses `commit` or `commits`. Relationship rows omit primary/trusted,
show non-primary/advisory only when useful, and use direct import, ownership,
external, unmatched, multiple-match, cycle-path, and edge-change wording.
Detailed and path coupling rows always state shared commits, union commits,
similarity, and whether a code dependency exists. Diff coupling comparisons
name both packages and say whether they now or no longer change together
without a code dependency.

### Human diagnostics summarize first

Incomplete history is `History is incomplete.` Rename gaps are `Some renamed
files could not be matched.` Dependency resolution uses correct singular or
plural sentences, such as `3 imports could not be matched.` and `1 import
matched more than one file.` Repeated parser and file warnings are grouped in
the default report. `--all` and a selected path expose file detail.

Help and CLI errors use the same short vocabulary. They avoid implementation
names and explain the next user action where one exists.

### Machine and analysis behavior stay fixed

Terminal filtering reads existing presentation facts. It does not change the
report, JSON serialization, finding rank, analysis policy, worker scheduling,
or project work. Exact JSON snapshots, serial/automatic bytes, live work
counters, allocation checks, and performance digests detect accidental change.

### Evidence is public first and private only in aggregate

Generated repositories cover codebase, diff, package, directory, and file
views at widths 120, 80, and 50, plus color modes, redirect behavior, glyphs,
alignment, truncation, help, errors, and warnings. Read-only private review
records only outcome categories for self, a mixed application, and a Rust
workspace. Raw private output, paths, source, identities, and history are never
committed.

Every human line fits the resolved Unicode display width after ANSI removal.
At 50 columns, changed measurements; relationship identity, counts, status, and
evidence; closed cycle witnesses; finding context; coupling identity and
evidence; and activity facts use short indented lines. Each long identity uses
middle truncation so its beginning and end remain visible without dropping
related facts. Test-only writer instrumentation proves reviewed narrow flows do
not reach the unexpected-line safety shortening.

## Risks / Trade-offs

- **Private-use glyphs depend on a Nerd Font.** The interface deliberately
  requires those glyphs and documents that requirement; no second icon mode is
  introduced.
- **Short output can hide context.** `--all`, path drill, and JSON retain the
  useful detail at the appropriate depth.
- **Unicode alignment can drift.** Exact code-point and display-width tests run
  at all three layout widths.
- **Presentation edits could trigger analysis work.** Existing live work and
  serial/automatic checks remain required.

## Migration Plan

1. Accept and strict-validate the OpenSpec contract.
2. Add the private glyph and relevance policy at the terminal seam.
3. Simplify help, errors, warnings, and documentation.
4. Update public exact snapshots and unchanged-behavior checks.
5. Review the three workload families without retaining raw output.
6. After the reviewed implementation commit, record clean release evidence.

Rollback restores the prior renderer and human snapshots; JSON and analysis
need no migration.
