## MODIFIED Requirements

### Requirement: README explains ratings through defined metric rules
The README SHALL publish each default health threshold, explain cognitive
complexity, cyclomatic complexity, and logical statements through exact
cross-language examples, explain how Git activity affects ordering, and state
that Smackdebt does not calculate one repository score.

#### Scenario: User evaluates a finding
- **WHEN** a user reads a High or Watch finding
- **THEN** the README provides enough information to reproduce its rating from defined source measurements

### Requirement: README lists only fully proven language support
The README SHALL list C, C++, Java, JavaScript, JSX, Python, Rust, TypeScript,
TSX, Ruby, and Vue only after their concrete tree-sitter implementations pass
exact unit, metric, recovery, span, nested-unit, serial/parallel, and performance
fixtures. It SHALL NOT describe any supported language as upstream-backed.

#### Scenario: User checks a mixed repository
- **WHEN** the user reads the language section
- **THEN** it explains that every listed language uses the same defined measurement algorithms through language-specific syntax translation

### Requirement: README documents JSON schema version 2
The README SHALL document `schema_version: 2`, flat indexed source facts,
complete retained Watch and High findings, complete comparisons, coverage and
diagnostics, and aggregate healthy counts. It SHALL NOT present schema version 1
as a supported output before first release.

#### Scenario: Integration author chooses JSON output
- **WHEN** the author reads the JSON section
- **THEN** the documented version, fields, completeness, and checked schema match executable black-box output
