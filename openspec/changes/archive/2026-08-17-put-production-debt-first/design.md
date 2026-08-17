## Context

`FindingRank` is a field-ordered `Ord`-derived struct in
`crates/analysis/src/report.rs`. Its declared field order *is* the accepted
rank, and today that order is rating, signals at rating, triggered signals, hot,
role class, then the measurement and identity keys. The last two of the leading
four are near-unique integers over a real repository, so the comparison almost
always terminates before `hot` or `role_class` is read. The field runs show the
consequence: a test unit with one more triggered signal outranks the production
function the reader came for.

The diff card path is `changed_measurements` in `crates/output/src/output.rs`,
which early-returns a single word for `Added`, `Removed`, and `Ambiguous`.
`Comparison` already populates exactly one side for added and removed units
(`crates/analysis/src/comparison.rs`), so the evidence is present and discarded
at the last step.

## Goals / Non-Goals

**Goals:**

- Make the headline and the first FINDINGS rows name production debt whenever
  production debt of that rating exists.
- Let heat decide among production findings instead of being unreachable.
- Give an added or removed diff card the evidence for why it is debt.

**Non-Goals:**

- Filtering, hiding, or down-rating non-primary findings.
- Changing ratings, signals, measurements, verdict tiers, counts, or which
  findings exist.
- Changing the architecture, history, or coverage families. Dev-dependency and
  module-ownership graph defects belong to `classify-production-architecture`.
- Computing anything new in the renderer.

## Decisions

### The rank order is rating, role class, hot, then the signal counts

The accepted order becomes exactly:

1. `rating`
2. `role_class` — primary source before every other role
3. `hot` — hot before not hot
4. `signals_at_rating`
5. `triggered_signals`
6. `cognitive_complexity`
7. `cyclomatic_complexity`
8. `logical_lines`
9. `activity`
10. `path`
11. `start_line`, `end_line` (the span)

Rating stays first because severity is the product's primary claim. Role class
comes next because a reader asking "what should I fix" is asking about code that
ships. Hot comes third because among code that ships, the code being edited is
the code worth fixing. The signal counts move below them: they are the reason
the finding *has* its rating, not a reason to prefer one finding of that rating
over another, and their near-uniqueness is precisely what made the two intent
keys dead.

Implementation is a field reorder in the `FindingRank` declaration and its
constructor. `Ord` is derived, so the declaration is the policy; nothing else
computes an order.

### No worst-offender filter is added

An obvious alternative is to restrict the `worst:` line and the FINDINGS head to
primary-role findings. It is rejected. The rank alone already produces the
desired outcome — a primary finding of equal or higher rating always sorts above
a non-primary one — and a filter would introduce a second, divergent policy for
"what may be named" beside "how findings order".

A filter would also break the repository whose only rated findings are
non-primary — a test-fixture repository, a benchmark suite, a repository whose
production code is clean. Such a repository must still name a worst offender;
silence there reads as "no debt", which is false. The existing
`affects_verdict()` filter on the worst-offender selection is what bounds
eligibility, and it is retained unchanged. The fallback is stated as an accepted
scenario so a later filter cannot be added without contradicting a requirement.

### Added and removed cards show the present side's absolute measurements

A retained comparison shows movement, so its card writes before and after
values. An added or removed unit has only one side, so its card writes absolute
values:

- `added` shows the after-side measurements.
- `removed` shows the before-side measurements.
- Only nonzero measurements are written, so a small unit's card stays short and
  a zero never poses as evidence.
- The direction word comes first, so the card still reads as a direction even
  when the reader stops at the first fact.
- If every measurement of the present side is zero, the card is the bare
  direction word, exactly as today.

`Ambiguous` is unchanged: it has no trustworthy side, so it keeps its direct
explanatory sentence alone. This is pure formatting of facts the comparison
already carries; the renderer performs no analysis and no arithmetic.

### The README says logical lines

The README's rank sentence currently ends "... cyclomatic complexity,
statements, activity, path, and span". `statements` is not a measurement name
anywhere else in the product; the measurement is logical lines, as
`ARCHITECTURE.md` and the JSON contract already say. The rank rewrite corrects
the sequence and the word in the same edit, so the documented rank matches the
implemented rank exactly.

## Risks / Trade-offs

- **The order-key test can pass while pinning the old order.** Roughly fifteen
  assertions flip direction, and an inequality written from the wrong side still
  passes. Each rewritten assertion states its deciding key in a comment, and a
  dedicated case proves role beats hot and hot beats the signal counts.
- **Hot production debt can now outrank a higher-signal production finding of
  the same rating.** That is the intent: at equal rating the signal count is a
  weaker predictor of what to fix than the fact that the file is being edited.
- **Snapshot churn is ordering-only and easy to wave through.** Every
  regenerated snapshot is reviewed for reordering only, with no row appearing or
  disappearing and no count changing.
- **Added and removed cards get longer.** Nonzero-only filtering and the
  existing per-row content-aware stacking bound the cost, and a diff of trivial
  additions keeps the bare word.

## Migration Plan

1. Accept and strict-validate this change before implementation.
2. Rewrite the rank order tests first, each assertion naming its deciding key,
   including a role-beats-hot case, a hot-beats-signal-counts case, and a
   test-only repository that still names a worst offender. Then reorder the
   `FindingRank` fields and constructor.
3. Update the README and `ARCHITECTURE.md` rank paragraphs in the same change,
   including `statements` becoming logical lines in the README.
4. Write the added and removed card expectation as acceptance evidence first,
   then render the present side's nonzero measurements.
5. Regenerate the affected terminal and JSON snapshots and review each file.
6. Verify against tokio, scikit-learn, opencode, kwaak, and fluyt: every
   `worst:` line names production code, benchmarks rank below production, and
   test debt is still present below it.
7. Archive this change before authoring `classify-production-architecture`.

Rollback restores the previous field order and the previous card bytes. No data,
schema, or stored artifact requires migration.
