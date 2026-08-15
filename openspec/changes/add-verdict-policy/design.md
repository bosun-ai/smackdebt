## Context

The report already contains everything a verdict needs: rated units, High and
Watch counts, architecture findings, ranked findings with resolved paths, and
three families of comparisons. What it lacks is the single statement those facts
support. Presentation cannot own that statement, because two renderers would
then have to agree on arithmetic, and machine consumers would have to re-derive
it from counts.

The plan freezes the tier ids as the machine contract and treats the sentences
as tunable copy. Analysis owns both, so terminal and JSON print identical bytes.

## Goals / Non-Goals

**Goals:**

- One tier per report, chosen by integer arithmetic over existing counts.
- One diff tier reconciled across source, architecture, and history.
- A named worst offender with a resolved path and a stated reason.
- An explicit membership rule for what counts as human debt movement in a diff.
- Scope-local verdicts so a path view answers about that path.

**Non-Goals:**

- Rendering the verdict; `redesign-terminal-report` owns that.
- Serializing the verdict; `adopt-report-schema-v4` owns that.
- A numeric score, grade, or index. The tier is an enum, not a number.
- Any configuration surface. Tier boundaries are frozen policy.

## Decisions

### Codebase tiers and their sentences

Tier ids are frozen; sentences are tunable copy owned by analysis:

| id | sentence |
| --- | --- |
| `empty` | `Nothing was checked.` |
| `clean` | `Clean. Ship it.` |
| `solid` | `Solid, with rough edges.` |
| `worn` | `Worn in the usual places.` |
| `fights_back` | `This code fights back.` |
| `lost` | `The code is winning.` |

### Tier mapping is integer permille

Selection runs in order:

1. checked units is 0 → `empty`.
2. High is 0 and Watch is 0 → `clean`.
3. High is 0 → `solid`.
4. `high * 1000 / checked` is at most 10 (High is at most 1% of units) → `worn`.
5. That permille value is at most 50 (at most 5%) → `fights_back`.
6. Otherwise → `lost`.

The permille value uses integer division of `high * 1000` by checked units, so
no floating-point value is produced anywhere in the path. A boundary value
belongs to the lower tier: exactly 1% is `worn`, exactly 5% is `fights_back`.

### Architecture escalation floors the tier

Package dependency cycles are structural debt that a unit-count ratio cannot
see. After the mapping above, at least one High architecture finding floors the
tier at `worn`, and at least three floors it at `fights_back`. A floor raises a
lower tier and never lowers a higher one, so a `lost` codebase stays `lost` and
a `clean` codebase with one High architecture finding becomes `worn`.

### Diff tiers and reconciliation

Diff tier ids are frozen with their sentences: `no_debt_change` `No debt
changed.`, `better` `You made it better.`, `worse` `You made it worse.`, `mixed`
`Better here, worse there.`

The tier is decided by the debt-diff membership below, reconciled across all
three comparison families at once — source comparisons, architecture findings
introduced or removed, and evolutionary findings introduced or removed. Any
worse member makes the change worse; any better member makes it better; both
make it mixed; neither makes it `no_debt_change`. The contradiction case is
explicit: no source comparison moved but a package cycle was introduced, and the
diff is `worse`.

The facts behind the tier name the family that moved and label every count with
its word, so a report can state `worse 1 (architecture) · changed 0` rather than
a positional ` 1 · 204`. A zero count is printed, never omitted.

### DebtDiffSelection defines human debt movement

Analysis owns a per-scope typed-ID list of the comparisons and findings that
move a verdict:

- Source comparisons: Regressed, Improved, Added at Watch or High, Removed at
  Watch or High, and MetricChanged while the unit is rated.
- Architecture findings introduced or removed.
- Evolutionary findings introduced or removed.

Everything else stays JSON-only and never reaches the verdict or the human
report: healthy units added or removed (swiftide's 204 and fluyt's 4,138),
unchanged comparisons, ambiguous comparisons, and comparisons whose file is
fixture or generated source. The lists are typed IDs, not copies, and are
consumed by the verdict, by presentation, and by index-integrity audits, which
reject duplicate IDs inside one scope's selection.

### Worst offender

The worst offender is the first entry of the hot- and role-aware finding rank
for the selected scope, carrying its resolved path string. Its reason is `hot
AND complex` when the finding belongs to a hotspot file, otherwise `most
complex`. When the scope has no ranked source finding but has a package cycle,
the worst offender is that cycle's first witness with reason `package dependency
cycle`. When neither exists, there is no worst offender and no line is produced.

### Scope-local verdicts

The root verdict is completed while the report is built. A verdict for any other
selected scope is a pure function of the completed report and that scope, so a
path view answers about the path and no renderer performs analysis to get it.

## Risks / Trade-offs

- **Tier constants invite bikeshedding.** Ids are frozen as the machine
  contract and every boundary has an exact pure test; sentences can change
  without breaking consumers.
- **Escalation can surprise.** A clean-looking codebase with one High
  architecture finding reads `worn`; the facts line names the architecture
  family so the reason is visible.
- **Membership rules hide data from the verdict.** Everything excluded stays in
  JSON; only the human answer is narrowed.
- **Percentage-shaped thresholds on tiny repositories.** With few units a single
  High finding crosses 5% immediately; `empty` and `clean` cover the degenerate
  cases and the tier stays a statement about density, not size.

## Migration Plan

1. Accept and strict-validate this change before implementation.
2. Add pure tests for every tier boundary, including permille edges at exactly
   1% and 5%, both escalation floors, and floors that must not lower a tier.
3. Add the diff truth table including the contradiction case and the
   all-neutral case.
4. Add `DebtDiffSelection` with index-integrity audits rejecting duplicate IDs.
5. Compute the root verdict during report construction and expose the pure
   scope verdict function.
6. Prove serial and parallel runs produce identical verdicts and selections.
7. Archive this change before implementing `redesign-terminal-report`.

Rollback removes the verdict and selection from the report; nothing serialized
changes in this change, so no consumer migration is required.
