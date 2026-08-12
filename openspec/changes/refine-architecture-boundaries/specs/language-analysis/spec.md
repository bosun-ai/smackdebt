## ADDED Requirements

### Requirement: Language dispatch is compiled and private
Smackdebt SHALL use one private language identifier and one compile-time dispatch
from a source file to a concrete analyzer. It SHALL NOT load analyzer plugins,
dynamic libraries, or user-provided code.

#### Scenario: Supported file is analyzed
- **WHEN** discovery supplies a path with a verified supported language
- **THEN** the registry selects one compiled analyzer without exposing parser or plugin interfaces

### Requirement: Every analyzer returns Smackdebt facts
Every analyzer SHALL return the same Smackdebt file-analysis values containing
language, line coverage, parse status, and a flat unit list. No upstream syntax
node or metric type SHALL cross the language crate boundary.

#### Scenario: Project consumes different language engines
- **WHEN** one file uses the temporary upstream adapter and another uses an owned analyzer
- **THEN** project orchestration receives the same Smackdebt file-analysis type from both

### Requirement: Units expose only rated measurements
Each unit SHALL expose its name, container identity, kind, original source span,
cognitive complexity, cyclomatic complexity, and exclusive logical lines.
Smackdebt SHALL NOT carry upstream Halstead or maintainability structures into
analysis policy or reports.

#### Scenario: Health policy rates a unit
- **WHEN** a language analyzer produces a unit
- **THEN** health policy can explain its rating entirely from the three retained measurements

### Requirement: Upstream support is pinned and verified
The temporary adapter SHALL pin `rust-code-analysis` revision
`37e5d83c056c8cbf827223d5814a93c5218df1a9`. Initial upstream-backed support
SHALL include C/C++, Java, JavaScript/JSX, Python, Rust, and TypeScript/TSX only
after nested-unit fixtures verify meaningful required measurements.

#### Scenario: Verified upstream language is listed
- **WHEN** product documentation lists an upstream-backed language as supported
- **THEN** its fixtures prove non-empty cognitive, cyclomatic, and line behavior for representative control flow

#### Scenario: Kotlin file is discovered
- **WHEN** a Kotlin file is encountered while the pinned engine still has empty required implementations
- **THEN** the file is reported as unsupported coverage rather than analyzed as healthy

### Requirement: Smackdebt bypasses the upstream directory runner
The temporary adapter SHALL call the pinned library for one already-selected
file at a time. It SHALL NOT use the upstream filesystem walker, worker threads,
channels, callbacks, terminal output, or file-output machinery.

#### Scenario: Project analyzes an upstream-backed file
- **WHEN** a project worker supplies an owned source buffer
- **THEN** the adapter moves it into one per-file upstream call and immediately reduces the result to Smackdebt facts

### Requirement: Metric extraction follows metric meaning
The adapter SHALL read direct cognitive and cyclomatic values for each rated
unit. It SHALL derive exclusive additive line values by removing direct child
totals. It SHALL NOT use one subtract-children rule for every metric or use
upstream metric merging for repository aggregation.

#### Scenario: Function contains a nested function
- **WHEN** the upstream metric tree reports a function containing another rated unit
- **THEN** each unit is rated once, the parent's line value excludes the child, and direct complexity follows verified upstream semantics

### Requirement: Ruby analysis is owned
The first usable release SHALL analyze Ruby without routing Ruby source through
`rust-code-analysis`. Ruby classes and modules SHALL be containers; methods,
singleton methods, and lambdas SHALL be units.

#### Scenario: Ruby method contains nested control flow
- **WHEN** an owned Ruby analyzer visits a method containing conditions and loops
- **THEN** it emits one method unit with direct cognitive, cyclomatic, and logical-line measurements and original source lines

#### Scenario: Ruby method contains a lambda
- **WHEN** a method contains a lambda
- **THEN** the lambda is emitted as its own unit and its direct work is not counted again as parent work

### Requirement: Vue analysis covers script and template
The first usable release SHALL analyze Vue single-file components as documents.
It SHALL delegate script and script-setup regions to JavaScript or TypeScript
analysis and SHALL emit a synthetic template unit for template control flow and
inline handlers.

#### Scenario: Vue component has a TypeScript setup script
- **WHEN** a Vue component contains a script-setup region with TypeScript language metadata
- **THEN** its units are analyzed as TypeScript and retain line spans from the original Vue file

#### Scenario: Vue template has branches and loops
- **WHEN** a Vue template contains conditions, loops, conditional expressions, logical branches, or inline handlers
- **THEN** the template unit exposes the contributing measurements and original template span

#### Scenario: Vue component contains styles
- **WHEN** a Vue component contains one or more style regions
- **THEN** those lines contribute to file coverage but do not create debt units

### Requirement: Upstream replacement is language-local
After Ruby and Vue, real usage and profiles SHALL select each next owned language
implementation. A replacement SHALL add compatibility fixtures and performance
evidence before switching one registry entry, without requiring changes to
project, Git, analysis, report, or output crates.

#### Scenario: Owned analyzer replaces an upstream implementation
- **WHEN** the owned analyzer satisfies the language's compatibility fixtures and performance gate
- **THEN** one private registry entry switches while higher-level acceptance output remains unchanged

### Requirement: Unsupported and failed files remain visible
Smackdebt SHALL produce coverage diagnostics for unsupported languages,
unreadable files, oversized files, and parser failures. It MUST NOT treat those
files as healthy source.

#### Scenario: One file cannot be analyzed
- **WHEN** one selected file is unsupported or fails analysis
- **THEN** a codebase report still succeeds with that path, reason, and excluded coverage recorded
