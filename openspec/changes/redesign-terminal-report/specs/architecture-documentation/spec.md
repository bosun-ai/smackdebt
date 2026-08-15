## MODIFIED Requirements

### Requirement: Architecture documents the terminal presentation boundary
`ARCHITECTURE.md` SHALL describe borrowed presentation rows, CLI-owned width, color, and
decoration resolution, per-row content-aware writing, semantic styling, and
unchanged JSON serialization. It SHALL state that the verdict, its sentence, its
counts, and the worst offender are completed analysis facts that the renderer
prints without deriving, and that the renderer performs no filesystem, Git,
parser, or analysis work.

#### Scenario: Engineer changes terminal layout
- **WHEN** rendering or display policy changes
- **THEN** the architecture guide identifies whether the change belongs in report facts, verdict policy, presentation selection, CLI policy, or per-row writing

#### Scenario: Engineer adds a decorated element
- **WHEN** a new glyph or bar is proposed
- **THEN** the guide requires an adjacent word that carries the meaning and undecorated output that remains complete
