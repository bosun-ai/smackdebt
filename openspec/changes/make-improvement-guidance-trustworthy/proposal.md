## Why

Smackdebt can currently answer a different question from the one the user
asked. An explicit file or directory with no supported source can silently
fall back to the repository, an Astro file can disappear from coverage, and a
moved anonymous callback can be reported as one addition and one removal.
Generated JavaScript can then occupy the short default report, while a mixed
diff may show only the worse half of its own conclusion. Those failures make
otherwise useful measurements hard to trust.

The next change makes the existing output truthful and useful before adding
more metrics. It keeps the current health thresholds, comparison directions,
and graph policies. It changes selection, identity, coverage, and wording so a
short report answers the selected scope and shows evidence for its conclusion.

## What Changes

- Explicit paths remain exact. A recognized source file is reported even when
  its language is unsupported. An existing directory with no recognized source
  and an existing non-source file fail with short, exact errors instead of
  returning repository results.
- `.astro` becomes a recognized source extension and an explicitly unsupported
  language. Astro source remains in codebase and diff inventories, coverage,
  diagnostics, file inspection, and graph-trust decisions. This change adds no
  Astro parser.
- Every report whose selected source was not all analyzed carries
  `Not all source was checked.` and the exact sentence
  `<analyzed> of <selected> source files were analyzed.` There is no percentage
  threshold. JSON keeps the existing coverage totals and adds both file counts
  to the qualifier.
- Anonymous units gain private, language-supplied match keys separate from
  display identity. Declared identity wins, then a unique semantic anchor,
  then a unique digest of exact syntax bytes. Line number, measurements, and
  source order never decide a match. Unsafe groups stay ambiguous and produce
  one aggregate warning per rendered scope, counting each affected file once.
- JavaScript files with the stated `.js`, `.mjs`, or `.cjs` generated names, or
  JavaScript, JSX, TypeScript, and TSX files with both the stated size and
  line-density evidence, receive the existing `generated` role after explicit
  configuration and language markers but before generic rules. They
  remain inspectable context but cannot own the verdict, default problem list,
  root worst offender, or default navigation.
- Diff conclusions use neutral sentences. A default mixed view reserves a row
  for each present direction before filling the remaining limit by the exact
  view-wide direction, family, subject, line, kind, and identity key across
  source, architecture, and history. Explicit limits are never exceeded.
- Every non-empty diff continues to end with exactly
  `inspect directories and files for more details` and never regains a `next:`
  line.
- JSON schema version 4 gains `astro` in the language vocabulary and
  `selected_files` and `analyzed_files` in the qualifier. Match keys and syntax
  digests remain private.
- Public fixtures and real-repository runs prove the behavior in terminal and
  JSON output before the change is closed.

## Capabilities

### Modified Capabilities

- `progressive-exploration`: exact explicit scopes and coverage wording.
- `language-analysis`: visible unsupported Astro source and safe anonymous-unit
  matching.
- `source-signal-quality`: generated JavaScript classification and its display
  participation.
- `verdict-policy`: unconditional incomplete-coverage qualification and neutral
  diff sentences.
- `terminal-output`: exact scope errors, grouped match warnings, representative
  mixed diffs, and the unchanged diff footer.
- `report-schema-v4`: Astro and qualifier file counts, with private match keys.
- `analysis-performance`: generated classification reuses the selected source
  read and matching adds no file or Git work.
- `end-to-end-evidence`: generated fixture and real-repository proof for every
  changed decision.
- `product-documentation`: exact scope, coverage, generated-source, matching,
  and diff behavior.

### Referenced Without Change

- `metric-semantics`: all five rated measurements and their thresholds stay
  unchanged.
- `architecture-analysis`: graph construction, trust rules, and real dependency
  cycles stay unchanged.
- `debt-ratchet`: generated and ambiguous source remain outside debt movement
  under the accepted selection rule.
- `hotspot-analysis` and `problem-clustering`: existing ranks remain the fill
  order after representative rows are reserved.

## Impact

- Affects `smackdebt-discovery` for source recognition and filename roles,
  `smackdebt-languages` for Astro identity and private anonymous anchors,
  `smackdebt-analysis` for matching, qualifier values, and frozen sentences,
  `smackdebt-project` for exact scopes and source-read composition,
  `smackdebt-output` for terminal and JSON presentation, and `smackdebt` for
  exact scope failures.
- `schemas/report-v4.schema.json`, terminal snapshots, JSON snapshots, README
  examples, architecture documentation, API snapshots, and release evidence
  change intentionally with the behavior they describe.
- JSON remains schema version 4. Existing members retain their meaning.
- No parser support, metric threshold change, broad directory exclusion,
  graph-policy change, new Git process, or automatic push is included.
