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

### Requirement: README reflects private development installation
Before registry publication is authorized, the README SHALL NOT claim that
`cargo install smackdebt` works from the registry. It SHALL document local-path
installation or clearly label registry installation as planned behavior.

#### Scenario: Developer follows current installation instructions
- **WHEN** a developer follows the README before publication
- **THEN** the documented command installs from the checked-out CLI crate without requiring published workspace libraries

### Requirement: README lists only verified language support
The README SHALL list C/C++, Java, JavaScript/JSX, Python, Rust, TypeScript/TSX,
Ruby, and Vue as initial supported languages after their requirements are
implemented. It SHALL distinguish owned Ruby and Vue analysis from the temporary
upstream-backed set and SHALL NOT list Kotlin as supported.

#### Scenario: User checks a mixed Ruby and Vue repository
- **WHEN** the user reads the language section
- **THEN** the README explains that Ruby methods and Vue script and template regions receive source measurements

#### Scenario: User checks Kotlin support
- **WHEN** the user reads the initial language list
- **THEN** Kotlin is absent and unsupported files are described as visible coverage gaps

### Requirement: README states JSON detail retention
The README SHALL state that JSON retains every Watch and High finding and all
scope summaries, while healthy units are represented through aggregate counts.

#### Scenario: Integration author chooses JSON output
- **WHEN** an integration needs all debt findings
- **THEN** the README makes clear that JSON is complete for Watch and High findings but not a full healthy-unit index

### Requirement: README explains package-root grouping
The README SHALL state that co-located manifests form one report package and
that each source file belongs to its nearest package-root directory once.

#### Scenario: Repository mixes ecosystems in one directory
- **WHEN** a user reads discovery behavior for a directory with several manifests
- **THEN** the README explains why the report shows one package rather than duplicate ecosystem packages

### Requirement: README demonstrates debt distribution
The README SHALL show the responsive codebase dashboard with exact child High,
Watch, debt-share, and local-rate values, rate bars, leading findings, and the
next drill command.

#### Scenario: New user follows progressive exploration
- **WHEN** the user reads the codebase example
- **THEN** the example explains the quality summary and moves from repository to child area to file using exact, internally consistent counts

### Requirement: README demonstrates progressive diff exploration
The README SHALL show a responsive diff dashboard with child Worse, Better,
Changed, and share values, share bars, and meaningful unit changes.

#### Scenario: User locates a worktree regression
- **WHEN** the user reads the diff example
- **THEN** the example shows how to identify the affected area and drill to its concise detailed comparison

### Requirement: README explains share and completeness
The README SHALL define codebase and diff share denominators, integer rounding,
path-limited scope, default terminal limits, `--all`, and complete JSON behavior.

#### Scenario: User interprets a percentage
- **WHEN** a displayed child share is rounded
- **THEN** the README directs the user to exact counts and explains why displayed shares may not sum to 100 percent

#### Scenario: User needs every retained result
- **WHEN** the default terminal view omits rows or details
- **THEN** the README explains terminal `--all` and that JSON is always complete

### Requirement: Product examples explain share and rate
Product documentation SHALL show progressive area output and distinguish an
area's share of selected debt from its local attention rate.

#### Scenario: User reads the root example
- **WHEN** the README introduces the codebase report
- **THEN** it explains how contribution and concentration support the next drill decision

### Requirement: README documents terminal presentation controls
Product documentation SHALL explain `--color`, `NO_COLOR`, width behavior,
Unicode redirected output, `--all`, and JSON as the complete machine-readable
view.

#### Scenario: User runs Smackdebt in automation
- **WHEN** the user reads output guidance
- **THEN** the README explains how to force or disable ANSI styling and that JSON remains unstyled

### Requirement: Product documentation explains evolutionary signals

The README SHALL explain churn, package change coupling, contributor count,
contributor concentration, and unexplained-coupling Watch findings in plain
product language.

#### Scenario: A user interprets evolution output

- **WHEN** the user reads the README and an example report
- **THEN** the user can distinguish present code health, static architecture,
  and change-history evidence
- **AND** the user is not told that any descriptive value is automatically bad

### Requirement: Product documentation states privacy and coverage behavior

The README SHALL state that analysis stays local, contributor identities are
not reported, and unavailable or incomplete history is shown explicitly.

#### Scenario: A user analyzes a shallow repository

- **WHEN** the user consults history documentation
- **THEN** the documented output matches the incomplete-history diagnostic
- **AND** it explains which source and architecture results still remain usable

### Requirement: Documented command examples are checked

The README SHALL mark runnable console examples and associate each with a named
public generated fixture and expected status and output.

#### Scenario: Documentation tests run

- **WHEN** the documentation validation command reads runnable README examples
- **THEN** it executes them through the built CLI against their named fixtures
- **AND** observed status and output match the documentation

### Requirement: Documentation covers the unified result

The README SHALL show how one command answers both code-quality and
architecture-quality questions using separate source, static architecture, and
evolution evidence.

#### Scenario: A user reads the main example

- **WHEN** the user follows the documented default and diff examples
- **THEN** the examples show separate findings and coverage for all three
  analysis families
- **AND** they do not present a combined debt score

