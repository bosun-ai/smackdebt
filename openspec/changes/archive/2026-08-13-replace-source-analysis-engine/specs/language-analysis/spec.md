## MODIFIED Requirements

### Requirement: Language dispatch is compiled and private
Smackdebt SHALL use one private language identifier, one private generic
`Language` trait, concrete compiled implementations, and static dispatch from a
source file to a generic analyzer. It SHALL NOT store trait objects or load
analyzer plugins, dynamic libraries, or user-provided code. The trait and parser
types SHALL NOT cross the language crate boundary.

#### Scenario: Supported file is analyzed
- **WHEN** discovery supplies a path with a verified supported language
- **THEN** compiled dispatch invokes the concrete language implementation through the generic engine and project receives only Smackdebt facts

#### Scenario: Another language is added
- **WHEN** its concrete trait implementation and exact fixtures pass
- **THEN** one private dispatch entry enables it without changing generic algorithms or another language module

### Requirement: Every analyzer returns Smackdebt facts
Every language implementation SHALL return the same analysis-owned file values
containing language, line coverage, parse status, and a flat unit list. No
tree-sitter syntax node, query, parser, or grammar-specific value SHALL cross the
language crate boundary. The implementation SHALL construct those facts without
an intermediate model that repeats their fields.

#### Scenario: Project consumes different language implementations
- **WHEN** one file is Rust and another is Ruby
- **THEN** project orchestration receives the same Smackdebt file-analysis type from both without mapping between duplicate analysis structures

### Requirement: Units expose only rated source measurements
Each unit SHALL expose its name, container identity, kind, original source span,
cognitive complexity, cyclomatic complexity, and exclusive logical lines.
Smackdebt SHALL NOT carry parser nodes or unused metric suites into analysis
policy or reports.

#### Scenario: Health policy rates a unit
- **WHEN** a language implementation produces a unit
- **THEN** health policy can explain its rating entirely from the three defined measurements

### Requirement: Ruby analysis uses the shared source engine
Ruby SHALL implement the private language contract with tree-sitter. Ruby
classes and modules SHALL be containers; methods, singleton methods, lambdas,
and separately rated closures SHALL be units. Modifiers and block forms SHALL
contribute through the same shared metric algorithms used by other languages.

#### Scenario: Ruby method contains nested control flow
- **WHEN** its syntax contains conditions, modifiers, loops, and boolean decisions
- **THEN** the Ruby implementation emits semantic events and the shared algorithms produce its exact measurements

#### Scenario: Ruby method contains a closure
- **WHEN** the closure is a separately rated unit
- **THEN** both units retain original Ruby spans and the closure body is not counted in the method measurements

### Requirement: Vue analysis uses document and injected language trees
Vue SHALL implement the private language contract as a multi-language document.
It SHALL parse script and script-setup regions with the owned JavaScript or
TypeScript implementations, emit a template unit from template syntax, retain
original file spans, and count style regions as covered source without rating
them.

#### Scenario: Vue component has a TypeScript setup script
- **WHEN** the document grammar identifies a TypeScript script-setup region
- **THEN** the TypeScript implementation analyzes that included source range once and emitted units use original Vue positions

#### Scenario: Template has control flow and an event reference
- **WHEN** a template contains a condition, loop, boolean decision, and plain event-handler reference
- **THEN** control-flow syntax contributes defined events while the plain handler reference does not create another branch

### Requirement: Supported languages use one owned engine
C, C++, Java, JavaScript, JSX, Python, Rust, TypeScript, TSX, Ruby, and Vue SHALL
use concrete implementations of the private language contract and the shared
metric algorithms. A language SHALL be enabled only after its exact unit,
metric, recovery, span, nested-unit, serial/parallel, and performance fixtures
pass. Kotlin SHALL remain unsupported until it meets the same requirements.

#### Scenario: Supported language is listed
- **WHEN** product documentation names a supported language
- **THEN** its compiled implementation and exact fixtures are active without routing source through `rust-code-analysis`

#### Scenario: Kotlin file is discovered
- **WHEN** Kotlin has not passed the complete language proof
- **THEN** the file remains an unsupported coverage item and never contributes a healthy unit

## REMOVED Requirements

### Requirement: Upstream support is pinned and verified
**Reason**: Every supported language moves to the owned generic tree-sitter
engine, so the pinned upstream metric engine is no longer part of the product.

**Migration**: Keep the pinned adapter only while incomplete language entries
move within this change, then remove its dependency and fixtures.

### Requirement: Smackdebt bypasses the upstream directory runner
**Reason**: Removing the upstream engine also removes its directory runner and
per-file adapter.

**Migration**: Preserve project-owned discovery, scheduling, and output while
each registry entry moves to the owned engine.

### Requirement: Metric extraction follows metric meaning
**Reason**: The new `metric-semantics` capability defines direct traversal and
exact algorithm meaning instead of extracting completed upstream metric trees.

**Migration**: Replace subtract-child extraction fixtures with semantic event
and nested-unit traversal fixtures.

### Requirement: Upstream replacement is language-local
**Reason**: This change completes the upstream replacement for all supported
languages.

**Migration**: Retain the one-language-at-a-time implementation order inside
this change and remove the temporary adapter after the final switch.
