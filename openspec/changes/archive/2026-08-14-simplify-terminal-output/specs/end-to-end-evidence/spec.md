## MODIFIED Requirements

### Requirement: Terminal evidence is exact and stable
The system SHALL compare committed exact terminal bytes for codebase, diff,
package, directory, and file views at widths 120, 80, and 50. It SHALL prove
exact glyph code points and one-cell width, rejection of U+EC3F, alignment and
truncation, `--color always`, `--color never`, `NO_COLOR`, redirect behavior,
glyph-only ANSI placement, and equality after ANSI stripping.

#### Scenario: Terminal width changes
- **WHEN** each named report view is rendered at each required width
- **THEN** every result exactly matches reviewed bytes, preserves relevant facts, recognizable locations, glyphs, and commands, and every ANSI-stripped line fits the requested Unicode display width
- **AND** direct writer instrumentation records zero unexpected-line safety shortenings for the reviewed 50-column reports and deliberate long-identity fixture, while a synthetic overflow records use of that safety path

#### Scenario: Styling policy changes
- **WHEN** forced, disabled, automatic, `NO_COLOR`, and redirected modes are compared
- **THEN** only the required glyphs receive the exact colors and stripping ANSI reproduces plain bytes

#### Scenario: Glyph vocabulary is audited
- **WHEN** public results are scanned by scalar value and display width
- **THEN** every required glyph is exact and one cell, U+EC3F and redundant status words are absent, and Changed text stays normal

### Requirement: Public command evidence covers every revised decision
Exact acceptance SHALL cover codebase, diff, package, directory, and file flows
for section relevance, five-area and three-finding limits, rank, architecture
witnesses, actionable history, detailed and path relationships, grouped
warnings, simple help and errors, commit singular/plural labels, direct
relationship wording, closed diff cycle paths, readable edge changes, and the
glyph-plus-command discover line. It SHALL also cover explained and weak
coupling evidence in detailed and path views plus direct introduced and removed
coupling outcomes in diff output.
JSON, status, stderr, ranking, serial/automatic bytes, work totals, analysis,
allocation, and performance evidence SHALL prove unchanged behavior.

#### Scenario: Human output is audited
- **WHEN** snapshots, help, errors, warnings, and README examples are checked
- **THEN** required sections and simple phrases appear while removed labels, severity words, processing facts, and forbidden phrases do not

#### Scenario: Machine and analysis output is audited
- **WHEN** revised terminal flows run through the complete public matrix
- **THEN** JSON bytes, exit behavior, rank, work counts, analysis facts, allocations, and resource evidence remain unchanged

### Requirement: Three workload families have privacy-safe acceptance
After public proof passes, aggregate read-only review SHALL cover self, a
private mixed application, and a private Rust workspace. Committed evidence
SHALL name only workload family and outcome categories and SHALL contain no
private path, source, Git identity, history, or raw terminal output.

#### Scenario: Self is reviewed
- **WHEN** default output is inspected
- **THEN** it leads with important debt and omits empty optional sections

#### Scenario: Private mixed application is reviewed
- **WHEN** default output is inspected
- **THEN** it leads with primary application findings and generated Rails schema remains outside default debt

#### Scenario: Private Rust workspace is reviewed
- **WHEN** default output is inspected
- **THEN** it leads with useful Rust findings and weak history and graph facts are absent
