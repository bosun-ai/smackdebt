## Context

Smackdebt already measures complexity per unit and change activity per file, and
already computes package degree, instability, and contributor concentration. The
field runs showed that the product answers worse than its own data allows: the
two halves of the classic hotspot signal are never multiplied, instability and
concentration are dead weight in JSON, `--history` describes a window that most
history numbers ignore, and test code outranks production code.

`resolve-workspace-dependencies` lands first because instability over an empty
package graph is meaningless. `add-verdict-policy`, `redesign-terminal-report`,
and `adopt-report-schema-v4` consume what this change produces.

## Goals / Non-Goals

**Goals:**

- Make the stated history window govern every history-derived number.
- Cross rated debt with change activity so hot debt is identifiable.
- Rank production debt above non-primary debt at equal rating.
- Turn already-computed instability and concentration into findings.
- Collect nesting, parameter count, and size so later phases can rate them.
- Describe files nothing depends on.

**Non-Goals:**

- Changing terminal sections, labels, or vocabulary; only rank order moves.
- Changing JSON version 3 shape; new tables become serialized in
  `adopt-report-schema-v4`.
- Rating maximum nesting or parameter count in this change.
- Introducing a second history read, a new traversal, or an extra Git process.
- Emitting contributor identity in any form.

## Decisions

### Hotspots become a new capability rather than part of evolutionary analysis

**Decision: create the `hotspot-analysis` capability.**

A hotspot is not a history fact and not a source fact; it is the product of the
two. `evolutionary-analysis` is defined as what Git history can prove about
change behavior, and every requirement in it is expressed over commits, churn,
coupling, and coverage. Folding a signal that depends on unit ratings into that
capability would make its scope "history, plus anything correlated with
history", and the rank keys that hotspots introduce apply to source findings
that have no history at all.

A separate capability keeps three things in one place: the cross-family policy
(rated file × touches), the minimum-touch threshold, and the rank keys that
order hot production debt first. It also gives `add-verdict-policy` a single
capability to cite for the worst-offender reason `hot AND complex`.

The cost is one more capability document and one more archive step. That is
accepted.

### The history window governs every history signal

The `--history` cutoff currently gates activity only. It becomes the single
filter applied when history records are turned into facts, so churn, touches,
coupling, and concentration all describe the same window. History coverage gains
the window length in days and the number of streamed commits excluded by the
window, so a reader can tell "quiet" from "outside the window". Commits excluded
by the window are counted separately from commits excluded for other reasons.

### Hotspot definition

A hotspot exists for a file when the file has at least one rated unit and its
change activity reaches the minimum touch count, default 5. Its strength
operands are the file's maximum unit rating and its touch count, both retained
as integers; no score is invented and no floating-point value is produced. The
minimum touch count is configurable and its default requires no configuration.

### Rank gains a hot key and a role class key

The existing rank sequence is rating, signals at that rating, total triggered
signals, cognitive complexity, cyclomatic complexity, logical lines, activity,
path, span. Two keys are inserted after total triggered signals: first hot
before not hot, then primary role before non-primary role. Everything after
those keys keeps its current order, so ordering stays total and data-stable and
test debt stays visible below production debt rather than disappearing.

### Stable-dependency violations use integer cross-multiplication

Instability is a ratio of unique fan-out to unique fan-in plus fan-out. A
package that depends on a package with a lower instability is depending on
something less stable than itself. Comparison uses cross-multiplication over the
integer degree operands, never a float: for packages A and B with `outA`,
`totalA`, `outB`, `totalB`, the violation condition is `outA * totalB > outB *
totalA`. A violation is a Watch architecture finding only when the depending
package has at least 2 references into the depended-on package, which keeps
single incidental imports out of the report. Both packages' exact degree
operands are retained on the finding.

### Knowledge concentration findings are counts only

A package with at least 10 commits in the window whose top contributor share is
at least 90% produces a Watch evolutionary finding. The finding carries the
package, the contributor count, the numerator, and the denominator. It carries
no name, address, raw author field, or internal contributor identifier, so the
existing privacy rule is unchanged. The evolutionary finding gains a kind
discriminator so unexplained coupling and knowledge concentration remain
distinguishable.

### Nesting and parameters are collected, not rated

Maximum nesting depth is the deepest nesting level reached inside one rated
unit, counted from zero at the unit body and incremented by the same
language-provided nesting events cognitive complexity already uses. Parameter
count is the number of declared parameters of the rated unit, counted once per
declared parameter, including a receiver only when the language declares it
explicitly; a unit that cannot declare parameters reports zero.

Both are exposed as measurements now and rated in `adopt-report-schema-v4`,
where the thresholds and the schema that serializes them land together. Rating
them earlier would produce ratings a reader cannot explain from the serialized
version-3 measurements.

### Size is rated now because its measurements already serialize

File size uses the file's line count against thresholds 400 for Watch and 800
for High. Container size uses the container's exclusive statement total against
300 for Watch and 600 for High. Both thresholds are configurable. Size is rated
in this change because both operands are already present and explainable in
version 3.

### Orphans are descriptive, not rated

An orphan file is a supported primary file with zero incoming dependencies in
the verdict graph that is not an entry file for its package or language. Entry
files are exempt by name and by manifest-declared entry, because being depended
on by nothing is their normal state. Orphans are facts, not findings, so they
never affect a rating or a verdict.

### New tables stay inside the existing flow

Hotspots, size findings, and orphan files are derived from tables the report
already builds, in the passes that already exist. No new file read, no new
traversal, and no new Git process is introduced. Every new table is ordered by
data-stable keys so serial and parallel runs stay byte-identical.

## Risks / Trade-offs

- **Rank changes move existing evidence.** Hot and role keys reorder findings on
  every workload; each regenerated snapshot is reviewed and the field runs are
  re-read.
- **Window-governed history changes existing numbers.** Coupling and
  concentration values move when a window is set; coverage now states the window
  so the change is explainable.
- **Two-stage measurement rollout.** Nesting and parameter count exist unrated
  for one change; the promotion is a named task in `adopt-report-schema-v4`.
- **More findings can crowd the report.** Minimum thresholds — 5 touches, 2
  references, 10 commits and 90% share — keep weak observations descriptive.
- **Measurement arity ripple.** Adding two measurements touches all 11 language
  implementations and their fixtures; it is confined to this change.

## Migration Plan

1. Accept and strict-validate this change before implementation.
2. Apply the history window to churn, coupling, and concentration and add the
   window coverage fields with pure tests.
3. Add hotspot, stable-dependency, knowledge-concentration, size, and orphan
   policy with pure boundary tests at every threshold.
4. Extend rank with the hot and role-class keys and re-prove total ordering.
5. Add nesting and parameter collection with exact fixtures for all 11 supported
   grammars, including Vue script, script-setup, and template regions.
6. Re-prove serial and parallel byte equality, work counts, allocation, and
   performance gates.
7. Regenerate reviewed evidence; JSON version 3 shape must be unchanged.
8. Archive this change before implementing `add-verdict-policy`.

Rollback removes the new tables and rank keys; no data migration is required
because the serialized schema does not change.
