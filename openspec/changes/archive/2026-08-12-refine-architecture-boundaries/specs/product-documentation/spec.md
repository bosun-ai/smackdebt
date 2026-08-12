## ADDED Requirements

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
