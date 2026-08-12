# product-documentation Specification

## Purpose
TBD - created by archiving change add-product-foundation. Update Purpose after archive.
## Requirements
### Requirement: README explains the product through its two user questions
The project SHALL provide a root `README.md` that explains codebase health and ref comparison in simple language.

#### Scenario: New user reads the introduction
- **WHEN** a user opens the repository README
- **THEN** the introduction states the two questions Smackdebt answers and explains progressive path discovery

### Requirement: README provides complete command examples
The README SHALL document installation, codebase analysis, path drill-down, ref comparison, history configuration, and JSON output with realistic commands and sample reports.

#### Scenario: User follows the codebase example
- **WHEN** a user reads the codebase analysis section
- **THEN** the README shows a no-argument command, concise output, and the next command for deeper inspection

#### Scenario: User follows the diff example
- **WHEN** a user reads the ref comparison section
- **THEN** the README shows default-ref discovery, worktree comparison, regressions, improvements, and path drill-down

### Requirement: README explains ratings without false precision
The README SHALL publish each default health threshold, explain how Git activity affects hotspot order, and state that Smackdebt does not calculate one repository score.

#### Scenario: User evaluates a finding
- **WHEN** a user reads a high or watch finding
- **THEN** the README provides enough information to trace the rating to cognitive complexity, cyclomatic complexity, or function size

### Requirement: README states support and limits
The README SHALL list supported languages, package discovery inputs, failure exit codes, optional configuration, privacy behavior, upstream attribution, and links to architecture and OpenSpec artifacts.

#### Scenario: User checks whether a repository is supported
- **WHEN** a user reads the support sections
- **THEN** the README identifies the initial languages and explains how unsupported or failed files affect coverage

