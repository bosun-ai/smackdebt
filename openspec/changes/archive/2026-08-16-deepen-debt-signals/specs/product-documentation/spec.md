## MODIFIED Requirements

### Requirement: README documents exact rank and labels
The README SHALL list the rank sequence exactly as rating, signals at that
rating, total triggered signals, hot state, role class, cognitive complexity,
cyclomatic complexity, logical lines, activity, path, and span. It SHALL state
that hot means a rated file whose windowed touch count reaches the minimum touch
count, that primary source precedes non-primary source at equal rating while
non-primary source remains visible, that unit kind and non-primary role are
shown, and that terminal `repository root` maps to machine path `.`.

#### Scenario: Two findings have the same rating
- **WHEN** a user wants to understand their order
- **THEN** documentation provides every comparison key in order

#### Scenario: Test debt appears below production debt
- **WHEN** a user compares a primary and a test finding with the same rating
- **THEN** documentation explains the role class key and states that test debt stays visible
