## MODIFIED Requirements

### Requirement: Inventory and current source reads are single pass
Codebase analysis SHALL perform one ignore-aware inventory walk and SHALL read
each selected current source file at most once. Discovery SHALL produce stable
path order before parallel source work begins. Astro recognition SHALL add no
content read. The generated JavaScript content rule SHALL consume the source
buffer already selected for analysis and SHALL NOT reopen the file.

Anonymous semantic anchors SHALL be collected during the existing language
traversal. The fixed-size syntax fingerprint SHALL be computed from the unit
slice while its existing active source buffer is available. Exact syntax SHALL
not be retained in `UnitFact`, `FileAnalysis`, a report, or a cross-worker table.
Matching SHALL add no parser traversal, filesystem read, Git process, or
whole-repository source retention, and source memory SHALL continue to follow
active workers.

#### Scenario: Large repository is scanned
- **WHEN** a report covers many supported and unsupported source files
- **THEN** instrumented tests observe one inventory visit, no read for recognition alone, and at most one source read per current file

#### Scenario: A large JavaScript file is classified
- **WHEN** its existing source buffer meets the generated content rule
- **THEN** the role is decided without another file read or parser traversal

#### Scenario: Anonymous units are compared
- **WHEN** a diff uses semantic anchors and fixed-size syntax fingerprints
- **THEN** instrumented work counts add no inventory, source read, Git process, parser visit, or retained source bytes after the active worker finishes

### Requirement: Explicit paths limit codebase work
Inside a Git repository, an explicit codebase path SHALL preserve
repository-relative identity while discovery and source reads stay limited to
the selected directory subtree or selected file. Package ancestor inspection
SHALL visit each distinct ancestor directory at most once. An explicit path
that yields no recognized source SHALL fail after its limited discovery and
SHALL NOT trigger a second repository-root walk or source read.

#### Scenario: User inspects one nested directory
- **WHEN** the repository contains source outside the selected directory
- **THEN** instrumentation observes no source reads outside the selection and displayed paths retain their repository-relative prefix

#### Scenario: User inspects a source-free directory
- **WHEN** the selected directory contains no recognized source
- **THEN** instrumentation observes one limited discovery, no repository fallback, and no source read outside the selected subtree
