## ADDED Requirements

### Requirement: Architecture documentation assigns signal ownership
Architecture documentation SHALL identify owners for SourceRole precedence,
language-generated markers, generic rules, parser trust, package identity,
static relation kind, orthogonal evidence, history fields, finding policy,
ranking, and rendering.

#### Scenario: A language adds a generated marker
- **WHEN** a maintainer consults the guide
- **THEN** it directs syntax recognition to that language implementation and shared precedence to policy code

### Requirement: Architecture documentation traces package and relation flow
Architecture documentation SHALL trace recognized roots into stable package
IDs and trace `uses` and `module_ownership` with separate role, trust, span, and
resolution evidence through project composition, analysis, JSON, and terminal
views.

#### Scenario: A base-only package owns advisory relations
- **WHEN** a maintainer follows its indexes
- **THEN** the guide explains package presence, stable identity, and why advisory relations do not enter verdict graphs

### Requirement: Architecture documentation separates analysis and presentation
Architecture documentation SHALL place rank keys, verdict inclusion, advisory
state, and de-duplication before rendering. It SHALL keep filesystem, Git,
parser, analysis, classification, and trust work out of output code.

#### Scenario: Default terminal removes an edge row
- **WHEN** presentation policy changes
- **THEN** retained JSON relations and analysis facts remain unchanged
