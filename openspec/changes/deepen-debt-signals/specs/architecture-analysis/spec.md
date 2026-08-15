## ADDED Requirements

### Requirement: Stable dependency violations are Watch findings
Analysis SHALL create a Watch architecture finding when a package depends on a
package that is less stable than itself and the depending package has at least 2
references into the depended-on package. A package is less stable than another
when its instability, unique fan-out over unique neighbor total, is greater.
Comparison SHALL use integer
cross-multiplication of the degree operands and SHALL NOT compare floating-point
instability values: for a depending package with unique fan-out `outA` and
neighbor total `totalA` and a depended-on package with `outB` and `totalB`, a
violation exists only when `outB * totalA` is greater than `outA * totalB`.
Equal cross products SHALL NOT be a violation. A package with no neighbors SHALL
NOT participate. The finding SHALL retain both packages' exact degree operands
and the reference count. Degree and instability SHALL remain descriptive facts
outside this finding.

#### Scenario: A stable package depends on an unstable one
- **WHEN** a package with lower instability has 3 references into a package with higher instability
- **THEN** one Watch architecture finding retains both packages' fan-in, fan-out, and the reference count

#### Scenario: The dependency is incidental
- **WHEN** the same relationship exists with exactly 1 reference
- **THEN** no finding is created and the degree facts remain descriptive

#### Scenario: Instability is equal
- **WHEN** the integer cross products of the two packages are equal
- **THEN** no violation exists

### Requirement: Orphan files are descriptive facts
Analysis SHALL identify an orphan file as a supported primary file with zero
incoming dependencies in the verdict graph that is not an entry file. Entry
files SHALL be exempt, recognized by conventional entry filename and by a
manifest-declared entry. Orphan facts SHALL be descriptive: they SHALL NOT be
rated, SHALL NOT create a finding, and SHALL NOT affect any verdict. Orphan
facts SHALL be derived from existing dependency degree over the file graph
without a new traversal, and SHALL be ordered by data-stable keys.

#### Scenario: Nothing depends on a primary file
- **WHEN** a supported primary file has fan-in zero and is not an entry file
- **THEN** it is recorded as an orphan file without a rating or a finding

#### Scenario: An entry file has no incoming dependency
- **WHEN** a package entry file has fan-in zero
- **THEN** it is not an orphan file

#### Scenario: A non-primary file has no incoming dependency
- **WHEN** a test or fixture file has fan-in zero
- **THEN** it is not recorded as an orphan file
