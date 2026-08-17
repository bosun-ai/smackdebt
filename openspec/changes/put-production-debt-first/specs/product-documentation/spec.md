## MODIFIED Requirements

### Requirement: README documents exact rank and labels
The README SHALL list the rank sequence exactly as rating, role class, hot
state, signals at that rating, total triggered signals, cognitive complexity,
cyclomatic complexity, logical lines, activity, path, and span. It SHALL name
that measurement `logical lines`, matching the architecture documentation and
the machine contract, and SHALL NOT call it `statements` in the rank sequence.
It SHALL state that primary
source precedes non-primary source at equal rating and that hot state decides
next, that hot means a rated file whose windowed touch count reaches the minimum
touch count, that non-primary source remains visible below primary source and is
still named as the worst offender when nothing else is rated, that unit kind and
non-primary role are shown, and that terminal `repository root` maps to machine
path `.`.

#### Scenario: Two findings have the same rating
- **WHEN** a user wants to understand their order
- **THEN** documentation provides every comparison key in order, beginning with role class and hot state before either signal count

#### Scenario: Test debt appears below production debt
- **WHEN** a user compares a primary and a test finding with the same rating
- **THEN** documentation explains the role class key and states that test debt stays visible

#### Scenario: A user compares the documented rank with the machine contract
- **WHEN** the user reads the README rank sequence beside the serialized measurement names
- **THEN** both call the measurement logical lines and the README rank sequence does not say `statements`
