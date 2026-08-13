## ADDED Requirements

### Requirement: Architecture documents the terminal presentation boundary
`ARCHITECTURE.md` SHALL describe borrowed presentation rows, CLI-owned width and
color resolution, width-specific renderers, semantic styling, and unchanged JSON
serialization.

#### Scenario: Engineer changes terminal layout
- **WHEN** rendering or display policy changes
- **THEN** the architecture guide identifies whether the change belongs in report facts, presentation selection, CLI policy, or width-specific writing
