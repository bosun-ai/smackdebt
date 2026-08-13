# Report schema evidence

`report-v2.schema.json` describes the complete public JSON report. Every
acceptance JSON result is validated against it before semantic assertions and
exact byte comparison. Tests also audit index references between paths, scopes,
files, findings, dependency edges, package rows, history rows, and comparisons.

Changing a field, enum, table, or index relationship requires an intentional
schema change, updated product documentation, focused semantic assertions, and
reviewed JSON results. Contributor names, addresses, and temporary identity
indexes are not report fields.
