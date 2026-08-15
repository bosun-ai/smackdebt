# Report schema evidence

`report-v4.schema.json` describes the complete public JSON report. Every
acceptance JSON result is validated against it before semantic assertions and
exact byte comparison. Tests also audit index references between paths, scopes,
files, findings, dependency edges, package rows, history rows, comparisons, and
the derived signal tables.

Version 4 opens with a denormalized head. `verdict` states the frozen tier id,
its analysis-owned sentence, and the report mode; `summary` states the checked,
high, watch, and High-architecture counts, the word-labeled debt-diff totals,
and up to three fully resolved worst offenders carrying repository-relative
path strings. The head duplicates facts the tables also carry: acceptance
rebuilds every head value from those tables and fails when they disagree.

Every serialized value is an integer or a string. Coupling similarity and
concentration ratio are not serialized; their integer operands are, so a
consumer derives any ratio at its own precision. Acceptance scans each result
and fails on any floating-point number.

Changing a field, enum, table, or index relationship requires an intentional
schema change, updated product documentation, focused semantic assertions, and
reviewed JSON results. Contributor names, addresses, and temporary identity
indexes are not report fields.
