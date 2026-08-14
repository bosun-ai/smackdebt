## Why

The terminal report repeats counts, labels, and processing facts until the most
important debt is hard to see. The first release needs a short human report
whose icons and sections carry one clear meaning while the complete JSON report
stays unchanged.

## What Changes

- Replace the current arrows and dots with one exact Nerd Font glyph per human
  meaning and color only that glyph.
- Reduce the default report to relevant `QUALITY`, `AREAS`, `FINDINGS`,
  `ARCHITECTURE`, and `HISTORY` sections.
- Remove severity words, repeated labels, repeated ratios, healthy counts,
  processing totals, and weak architecture or history facts from human output.
- Keep `--all` focused on useful findings and relationships, with path views
  retaining relevant incoming and outgoing relationships.
- Simplify help, errors, warnings, and README examples without changing JSON,
  analysis, ranking, exit codes, work counts, or resource behavior.
- Prove exact output at three widths, exact glyphs and styling, unchanged JSON
  and analysis behavior, and privacy-safe outcomes on three workload families.

## Capabilities

### New Capabilities

- `terminal-output`: Defines section relevance, exact glyphs, glyph-only color,
  simple human text, detail behavior, and warning grouping.

### Modified Capabilities

- `progressive-exploration`: Replaces large tables and complete terminal dumps
  with short relevant areas, findings, relationships, and drill guidance.
- `product-documentation`: Makes examples and terminal guidance match the short
  human report.
- `end-to-end-evidence`: Expands exact width, glyph, styling, text, unchanged
  machine behavior, and workload review proof.
- `release-readiness`: Requires reviewed clean evidence for the revised human
  interface before release.

## Impact

This changes human terminal, help, error, warning, documentation, and snapshot
bytes. It affects `smackdebt-output`, CLI text, README examples, acceptance
fixtures, and release evidence. JSON schema version 3 and all analysis facts,
ranking, scheduling, and resource contracts remain unchanged. No icon option,
emoji mode, ASCII fallback, or terminal configuration is added.
