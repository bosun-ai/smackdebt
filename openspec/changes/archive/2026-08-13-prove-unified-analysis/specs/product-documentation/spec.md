## ADDED Requirements

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

