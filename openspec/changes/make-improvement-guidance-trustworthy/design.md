## Context

The failures addressed here cross several existing seams, but each has one
owner:

- Discovery decides whether a path is source and assigns its source role.
- Languages decide what a unit means and can name syntax context without
  exposing parser nodes.
- Analysis owns identity matching, verdict values, and selection.
- Project composition keeps an explicit scope exact and reuses source reads.
- Output renders completed facts and never repairs analysis decisions.

The change keeps those responsibilities. It does not solve misleading output by
adding renderer guesses or a second inventory.

## Goals and non-goals

### Goals

- Make every successful report answer the path the user selected.
- Make every selected source file visible as analyzed, unsupported, or failed.
- Pair anonymous units only when syntax evidence makes the pairing safe.
- Keep generated assets available for inspection without allowing them to crowd
  out authored debt.
- Make a short mixed diff visibly support its conclusion.
- Keep human language neutral, short, and exact.

### Non-goals

- No Astro parser.
- No health, generated-density, reach, core, leakage, or cycle threshold change.
- No matching by measurements, line, ordinal position, or fuzzy similarity.
- No blanket generated rule for `public`, `share`, or `assets` directories.
- No source digest in reports, logs, diagnostics, serialization, or a readable
  cross-crate accessor.
- No omitted-row notice and no new diff section.
- No schema-version increase.

## Decisions

### Explicit selection is resolved once

The CLI resolves the user path to one of four outcomes before report
composition:

1. A supported or recognized-unsupported source file is the selected file.
2. A directory containing at least one recognized source file is the selected
   directory.
3. An existing non-source file fails with
   `smackdebt: not a source file: <path>`.
4. A directory containing no recognized source fails with
   `smackdebt: no source files found under: <path>`.

A missing path keeps `smackdebt: path not found: <path>`. Each message retains
the user-provided path and ends in one newline. None writes stdout or appends
help. There is no fallback from an explicit path to the repository root.

### Astro is recognized and unsupported

`.astro` joins the compiled language identity as `Astro`, serializes as
`astro`, and has no analyzer dispatch. Discovery retains it as source in root,
directory, file, ref-diff, and worktree-diff inventories. Analysis produces an
unsupported-language diagnostic and no unit facts. This makes coverage and
graph evidence honest without pretending to understand the document.

### Coverage qualification uses file counts

The completed scope coverage already owns selected and analyzed file counts.
`analyzed_files` retains its accepted meaning: clean, recovered, and context
files count as analyzed; unsupported and failed files do not. A qualifier exists
exactly when `analyzed_files < selected_files`, including a one-file gap.

The qualifier owns two exact sentences and both integers:

- `Not all source was checked.`
- `<analyzed> of <selected> source files were analyzed.`

Terminal prints both lines in the verdict block. JSON exposes `sentence`,
`detail`, `selected_files`, and `analyzed_files`. Existing byte-share and largest
unsupported-language facts may remain in the qualifier for compatibility, but
they no longer decide whether it exists. The grouped warning still names the
unsupported or failed file count and its cause; it is supporting detail, not a
replacement for the qualifier.

Repository share requires two measured totals. A sub-scope rendered from a
completed repository report may state its High count against the measured root
High count when selected source also exists outside that sub-scope. When the
sub-scope and root selected-file totals are equal, it omits the share because
the denominator adds no information. A fresh explicit file or directory report
uses limited discovery, so it also omits repository share instead of treating
the selected inventory as the whole repository. Qualifier lines still precede
a share when a completed report provides both facts.

### Anonymous match evidence crosses one narrow inward seam

Display identity and match identity serve different jobs. Display identity may
continue to use a line-based anonymous name because it helps locate a unit in
one version. Matching uses an analysis-owned opaque value with these variants:

- declared identity for a named function or method;
- a language-supplied semantic anchor for an anonymous unit;
- a 256-bit BLAKE3 digest and byte length of the unit's exact syntax bytes;
- no safe key.

The value is public only at the private workspace crate seam because
`smackdebt-languages` depends inward on `smackdebt-analysis` and must construct
it. Its fields and read access stay private to analysis. Language adapters write
it through narrow constructors and attach it to `UnitFact`; project and output
cannot interpret it. The API snapshot records the deliberate seam, and no
dependency points from analysis back to languages.

Language adapters supply anchors while they already traverse syntax. Supported
anchors are an assignment or binding name; the enclosing `computed`, `watch`,
or callback call together with argument position; a stable neighboring literal
when that literal identifies the callback; and a Ruby example or context
description. An anchor includes its enclosing declared container and unit kind
so the same local label in two methods does not collide.

Within one file comparison, matching proceeds in three passes:

1. Pair a declared identity only when it occurs once on each side.
2. Pair a semantic anchor only when it occurs once on each side among remaining
   units.
3. Pair a syntax digest and byte length only when they occur once on each side
   among remaining units.

The fingerprint is deterministic over exact syntax bytes, not file path, line
span, measurements, rating, or traversal position. Exact syntax is not retained:
doing so for every unit would make report memory follow repository size instead
of active workers. A fingerprint bucket pairs only when exactly one unit exists
on each side. Any repeated bucket on either side is ambiguous, even when counts
match, so collision-like evidence never drives a many-to-many guess. A moved and
edited callback pairs only through a semantic anchor; a fingerprint mismatch
never becomes a fuzzy match.

Ambiguity requires plausible candidates on both sides. A shared anchor or
fingerprint bucket with one unit on each side pairs. A shared bucket with a
repeated side, including two-to-one, one-to-two, and many-to-many, emits one
existing `ambiguous` comparison for that collision group and one
`ambiguous_identity` diagnostic for the file. A bucket present only before
produces one Removed row per unit, even when repeated. A bucket present only
after produces one Added row per unit, even when repeated. A unit with no safe
key likewise stays one-sided unless shared match evidence exists. Human output
emits one aggregate warning per rendered scope, counting each affected file
once, and does not guess a direction or measurements.

### Generated JavaScript has two narrow rules and one precedence level

The filename rule applies case-sensitively only to files whose suffix identifies
them as JavaScript through `.js`, `.mjs`, or `.cjs`:

- `*.min.js`, `*.min.mjs`, `*.min.cjs`;
- `*.bundle.js`, `*.bundle.mjs`, `*.bundle.cjs`;
- `*-bundle.js`, `*-bundle.mjs`, `*-bundle.cjs`.

The content rule applies to JavaScript, JSX, TypeScript, and TSX source,
including `.js`, `.mjs`, `.cjs`, `.jsx`, `.ts`, and `.tsx`. Vue remains a
document and is outside this rule. A file is generated
when its source is at least 65,536 bytes and its total byte length is at least
512 times its count of nonempty physical lines. A file with no nonempty line
does not match. This integer comparison avoids rounding and division. The
project applies it from the source buffer already read for analysis.

Role precedence is explicit configuration, language-owned generated markers,
these JavaScript filename and content rules, existing generic filename and path
rules, test-declared fallback, then primary. Explicit configuration therefore
wins. The new level can produce only `generated`, so it creates no same-level
role conflict. The content decision is final before report facts, health,
history, or graph policy consume the role.

The two rules add a source role; they do not remove the file. Generated files
remain in files, coverage, diagnostics, machine comparisons, `--all`, and an
explicit file view. They do not enter verdict health, the root worst offender,
default problem cards, or the codebase `next:` target. Directory names alone do
not trigger this role.

### Mixed selection is one view-wide decision with one exact key

Presentation collects rows linked by the selected scope's `DebtDiffSelection`
across source, architecture, and evolutionary comparison families. Their
view-wide key is direction (Worse, Better, Changed), family in terminal section
order (source, architecture, history), repository-relative subject path or path
pair, start line with an absent line last, family kind in its enum order, then
comparison identity.

The default and explicit limit are one count over those comparison rows, not a
separate count per section. When both Worse and Better are present, default
selection reserves the first row by this key for Worse, Better, and Changed when
that direction exists, then fills remaining slots from the same key without
duplicates. A reserved row stays in its owning section. Rendering uses the same
direction and stable-subject order inside each section. Current-state history
context that is not a diff comparison keeps its accepted section-local policy
and does not consume the comparison limit.

The default comparison limit remains three. `--all` needs no reservation
because nothing is truncated. An explicit `--top <n>` applies the view-wide key
without reservation and never emits more than `n` comparison rows. This keeps
`--top 1` literal and deterministic.

No hidden-row count or omission sentence is added. A no-debt diff is
verdict-only unless comparison trust is narrowed by incomplete source coverage,
anonymous-match ambiguity, a suppressed graph comparison, or rename or history
availability evidence that affects the selected comparison. Those facts expand
the report only enough to render the qualifier and applicable warning rows,
then end with exactly `inspect directories and files for more details`.
Unrelated current-state areas, history findings, concentration, coupling, and
other context never pierce the verdict-only form. A documentation-only diff
therefore remains exactly `No debt changed.` even when the current repository
retains history context.

### Neutral sentences are analysis-owned

The diff tier identifiers and movement policy remain unchanged. Their frozen
sentences become:

| Tier | Sentence |
| --- | --- |
| `no_debt_change` | `No debt changed.` |
| `better` | `Debt decreased.` |
| `worse` | `Debt increased.` |
| `mixed` | `Debt increased in some places and decreased in others.` |

Terminal and JSON consume the same completed verdict bytes.

## Alternatives rejected

- Treating Astro as a non-source file keeps coverage falsely complete.
- Parsing Astro as HTML or JavaScript would invent support without fixtures for
  document regions and original spans.
- Matching anonymous units by line, metric tuple, or source order creates a
  convincing but unsafe diff after insertions and moves.
- Fuzzy syntax similarity can pair two different callbacks and is harder to
  explain than an ambiguity warning.
- Ignoring common asset directories hides authored code and breaks explicit
  path inspection.
- Raising the default row limit without reserving directions still allows one
  direction to occupy the screen.
- Renderer-side sentence or row repair would split terminal and JSON truth.

## Validation plan

- Pure tests pin scope classification, qualifier creation, match-pass order,
  collision behavior, generated filename and content limits, representative
  selection, and exact verdict sentences.
- Language fixtures pin anchors for JavaScript/TypeScript/Vue callbacks and Ruby
  examples, plus unchanged-move and moved-and-edited cases.
- Black-box generated repositories pin exact stderr, terminal, JSON, schema,
  serial/automatic equality, explicit Astro paths, generated context, ambiguity
  warnings, mixed output, `--top 1`, and the footer.
- One newly built release binary is compared with saved pre-change output on
  Smackdebt, Fluyt, the marketing repository, Netdisco, Swiftide, and Parity.
  Review records contain only aggregate counts, paths already present in tool
  output, directions, graph evidence, exit status, and report digests; no source
  content is copied into fixtures.
