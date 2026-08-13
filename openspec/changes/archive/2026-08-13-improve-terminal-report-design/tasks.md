## 1. Contract

- [x] 1.1 Add terminal layout, styling, color, width, and package-count deltas
- [x] 1.2 Strictly validate the change before implementation

## 2. Presentation values

- [x] 2.1 Build private borrowed summary, area, detail, diagnostic, and drill rows
- [x] 2.2 Apply ranking, omission, package counting, and zero-noise policy once
- [x] 2.3 Add Unicode-aware count, percentage, bar, and label formatting

## 3. Responsive terminal rendering

- [x] 3.1 Render the shared header, scope facts, quality or change summary, and coverage
- [x] 3.2 Render full, compact, and stacked codebase area layouts
- [x] 3.3 Render full, compact, and stacked diff area layouts
- [x] 3.4 Render concise finding and comparison cards plus one copyable drill command
- [x] 3.5 Add semantic ANSI styling whose removal exactly matches plain output

## 4. CLI policy

- [x] 4.1 Add and resolve `--color auto|always|never` with `NO_COLOR` behavior
- [x] 4.2 Reject explicit color selection with JSON and keep JSON unstyled
- [x] 4.3 Resolve width from `COLUMNS`, terminal size, or the redirected default

## 5. Proof and documentation

- [x] 5.1 Add plain and colored snapshots across full, compact, and stacked widths
- [x] 5.2 Test color policy, Unicode alignment, bars, count language, omission, and copyable paths
- [x] 5.3 Prove serial and parallel terminal bytes match and JSON values are unchanged
- [x] 5.4 Run representative codebase and diff reports, including Fluyt
- [x] 5.5 Update README and architecture examples
- [x] 5.6 Run the complete workspace gate, archive this change, and re-run strict OpenSpec validation
