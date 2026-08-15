## Context

Smackdebt has two audiences and one output. A human reads it in a terminal; an
LLM or a script reads the same bytes through a pipe. Today the report optimizes
only for the first: severity is a private-use glyph, direction is column
position, and detail is quantity. The field runs show the cost — an unlabeled
` 1 · 204` verdict, a truncated cycle witness, unnamed closures, an error that
prints an OS errno, and an `--all` that answers "everything" instead of "all the
debt that matters".

`add-verdict-policy` has already produced the answer as completed facts. This
change only decides how those facts are written, and the renderer keeps doing
zero analysis.

## Goals / Non-Goals

**Goals:**

- Make words carry every meaning so piped output is self-describing.
- Lead with the verdict in both modes.
- Never make the reader guess a count, a direction, or a family.
- Keep every fact that a row exists to show, at every width.
- Make diff findings actionable: location, movement, and a human identity.
- Make the three common input failures exact and safe.

**Non-Goals:**

- Computing anything in the renderer.
- Changing report facts, rank, analysis policy, exit classes, or JSON bytes.
- Adding an icon option, an emoji mode, a theme, or any new terminal setting
  beyond the existing color resolution.

## Decisions

### Words carry meaning; glyphs decorate them

The human vocabulary is `high`, `watch`, `worse`, `better`, `changed`,
`warning`, and `next:`. Every severity, direction, and diagnostic is stated with
its word.

Terminal options gain a decoration flag resolved beside color from the existing
terminal detection. With decorations, a glyph and the tier-colored `▌` bar may
appear, and a glyph is always adjacent to the word it decorates — never instead
of it. Without decorations, the same report is written with words only.

**This reverses the accepted rule that human output never repeats a severity
word beside a glyph.** That rule was written when the terminal was the only
audience. The reversal is explicit in the delta so the change is reviewable, and
the old rule's intent — no duplicated meaning — is preserved by the adjacency
rule: one word, optionally decorated, never two spellings of the same fact.

`--color never` produces fully plain output. Piped output contains no codepoint
in U+E000–U+F8FF, asserted by scanning every public flow, so a private-use glyph
can never reach a machine consumer.

### The verdict block replaces the header, quality, and change lines

The report opens with the scope, the tier sentence, labeled counts, and the
worst offender:

```text
smackdebt · repository root
▌ This code fights back.
12 high · 94 watch · 1,686 checked
worst: parser/engine.rs — hot AND complex
```

The bar is decoration; the sentence, counts, and worst offender are words. The
diff form labels every count with its word and names the family that moved, so
`worse 1 (architecture) · better 0 · changed 0` replaces ` 1 · 204`. A zero
count is printed. Nothing is positional.

A clean diff prints the verdict line only. No history context, no warnings, no
trailing sections after `No debt changed.`

### Sections

- `AREAS`: at most five debt-bearing children with word-labeled counts.
- `FINDINGS`: ranked rows carrying `· hot (n commits)` when the file is a
  hotspot, with `path:line` and measurements.
- `ARCHITECTURE`: cycle findings whose witnesses stack across lines rather than
  truncating with `…`, plus stable-dependency rows printed with integer
  operands as fractions.
- `HISTORY`: one row per package pair and knowledge-concentration rows as counts.
- `WARNINGS`: grouped, one sentence per kind, such as `29 imports could not be
  followed.`
- The discover line is `next: smackdebt <path>`.

### Diff findings become cards

Each diff finding shows `path:line`, and every changed measurement is written as
before and after. Anonymous units get human identities derived from their file
and kind — `GraphEditor.vue · closure`, `filename · template` — so
`<closure 1177>` never reaches a reader.

### `--all` means all useful debt

`--all` removes count limits on debt, not on data. Raw dependency edges,
standard-library externals, churn dumps, cyclomatic-1 rows, weak coupling, and
healthy rows leave the terminal permanently. JSON remains the complete view, so
nothing is lost — it moves to the machine contract.

### Width is decided per row

Report-level width tiers are replaced by per-row content-aware writing: a row
measures its own content, stays aligned when it fits, and stacks its facts on
indented lines when it does not. Measurements, counts, cycle witnesses, history
evidence, dependency state, commands, and identities are never silently
clipped. A cycle witness is never `…`-truncated; it stacks.

### Errors name the fixable value

- Missing path: status 1, empty stdout, stderr exactly
  `smackdebt: path not found: <user-path>\n`, using the original argument, with
  no absolute path and no operating-system text.
- Missing Git ref: status 1, empty stdout, stderr exactly
  `smackdebt: Git ref not found: <ref>\n`, with no Git command, status, or fatal
  output.
- `--all --json`: status 2, empty stdout, stderr exactly
  `smackdebt: --all cannot be used with --json\n`.

None append usage or help.

### The README is an executable contract

The README's examples, vocabulary, and the now-false "no ASCII fallback"
paragraph change atomically with the fixtures and the documentation tests, since
words-only output is exactly the fallback that paragraph denied.

## Risks / Trade-offs

- **Every terminal snapshot moves.** Roughly thirty files regenerate; each is
  reviewed line by line and the field repositories are re-read as the product
  review.
- **Words cost horizontal space.** Content-aware stacking absorbs the cost and
  the 50-column evidence proves nothing is clipped.
- **Reversing an accepted requirement.** It is stated as an explicit
  modification with its reason, not quietly dropped.
- **Permanent removal of terminal rows.** JSON keeps every removed row, and the
  README states where the complete view lives.
- **Human identities are synthesized.** They are derived from file and kind
  only, never invented names, and the machine report keeps the original
  identity.

## Migration Plan

1. Accept and strict-validate this change before implementation.
2. Resolve decorations beside color and add the vocabulary and private-use
   assertions first, so the audience contract is proven before layout moves.
3. Write the verdict block from completed verdict facts through presentation.
4. Rework sections, diff cards, and `--all` filtering.
5. Replace width tiers with per-row content-aware writing and prove zero
   silently clipped facts at widths 120, 100, 80, and 50.
6. Implement the three exact errors.
7. Rewrite the README with its checked examples in the same change.
8. Regenerate every terminal snapshot and review each file.
9. Re-run the release binary on smackdebt, swiftide, and fluyt; swiftide `diff
   HEAD~15` must lead with a one-line verdict instead of 204 changed rows.
10. Archive this change before implementing `adopt-report-schema-v4`.

Rollback restores the previous terminal and error snapshots. Report data,
analysis, and JSON require no migration.
