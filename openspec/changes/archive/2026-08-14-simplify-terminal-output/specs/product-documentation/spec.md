## MODIFIED Requirements

### Requirement: README provides complete command examples
The README SHALL document installation, codebase analysis, path drill-down, ref
comparison, history configuration, and JSON output with realistic commands and
sample reports that use the exact short terminal sections and glyphs.

#### Scenario: User follows the codebase example
- **WHEN** a user reads the codebase analysis section
- **THEN** the README shows a no-argument command, relevant output, and one glyph-plus-command line for deeper inspection

#### Scenario: User follows the diff example
- **WHEN** a user reads the ref comparison section
- **THEN** the README shows default-ref discovery, relevant Worse, Better, and Changed findings, and path drill-down without internal report terms

### Requirement: README documents terminal presentation controls
Product documentation SHALL explain the Nerd Font glyph requirement,
`--color`, `NO_COLOR`, width behavior, Unicode redirected output, `--all`, and
JSON as the complete machine-readable view. It SHALL state that there is no
icon option, emoji mode, or ASCII fallback.

#### Scenario: User runs Smackdebt in automation
- **WHEN** the user reads output guidance
- **THEN** the README explains how to force or disable ANSI glyph styling and that JSON remains unstyled

### Requirement: Documented command examples are checked
The README SHALL mark runnable console examples and associate each with a named
public generated fixture, expected status, and exact relevant output fragments.

#### Scenario: Documentation tests run
- **WHEN** documentation validation reads runnable README examples
- **THEN** it executes them through the built CLI and matches status, sections, glyphs, commands, stdout, and stderr in order

### Requirement: Documentation covers the unified result
The README SHALL show how one command answers code and architecture questions
through separate `QUALITY`, `FINDINGS`, `ARCHITECTURE`, and `HISTORY` sections,
with empty optional sections omitted.

#### Scenario: A user reads the main examples
- **WHEN** the user follows documented default, path, and diff examples
- **THEN** the examples lead with relevant findings, omit empty sections and internal processing facts, and do not present a combined score
