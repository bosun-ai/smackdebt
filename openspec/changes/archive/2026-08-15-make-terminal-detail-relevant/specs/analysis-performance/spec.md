## MODIFIED Requirements

### Requirement: Output streams from one report
Terminal and JSON renderers SHALL write directly to `io::Write` from one
borrowed completed report. Terminal selection SHALL borrow or stream existing
scope links and selected facts without a second report model or large path,
finding, relationship, or history clones. It SHALL NOT allocate a de-dup set,
second selected-ID collection, or identity mapping. JSON SHALL stream every retained fact
without using the terminal shortlist. Analysis SHALL NOT depend on Serde.

#### Scenario: Dense all-detail scope is emitted
- **WHEN** terminal `--all` and JSON render the same completed report
- **THEN** terminal streams each linked useful-debt ID once, JSON streams complete facts, and neither clones the report nor allocates a renderer de-dup structure

### Requirement: Scope rendering performs no project work
Rendering a retained repository, package, directory, or file scope SHALL
perform no inventory walk, source read, Git operation, parsing, health
assessment, classification, or worker scheduling. Terminal selection SHALL
visit each applicable existing child, source-finding, architecture-finding,
evolution-finding, and diagnostic link once and SHALL NOT scan an unrelated
global table again for every displayed row. Rendering SHALL trust unique owning
lists and SHALL NOT rewrite or hide duplicate report data.

#### Scenario: One report renders every scope and detail mode
- **WHEN** an instrumented test renders default and `--all` repository, package, directory, and file scopes
- **THEN** project-work counters remain unchanged and link-visit evidence shows one selection pass per rendered scope

### Requirement: Progressive links preserve measured report limits
Progressive links SHALL preserve measured report limits. Finding links,
comparison links, architecture-finding links,
evolution-finding links, diagnostic links, path indexes, and diff counts SHALL
use reserved flat storage. Analysis/report aggregation SHALL place each typed
ID at most once in each owning scope link list, and index audits SHALL reject a
duplicate. Default terminal selection SHALL retain only its small display
limits. `--all` SHALL borrow or stream the relevant linked slice without a large
clone, de-dup set, second collection, or identity mapping. Generated
large-repository workloads SHALL measure allocation count and peak memory, and
SHALL reject repeated per-row global-table scans.

#### Scenario: Large hierarchy is selected for all useful debt
- **WHEN** one scope contains many linked findings and child areas
- **THEN** report aggregation links each typed ID once per owning scope list and terminal selection visits each applicable link once without repeated lookup work, a large clone, or de-dup allocation

#### Scenario: A scope owning list repeats an ID
- **WHEN** report/index integrity is checked
- **THEN** the audit fails before rendering and no renderer allocation or mapping hides the duplicate

#### Scenario: Accepted current fixtures are audited
- **WHEN** their owning lists pass uniqueness checks
- **THEN** JSON bytes remain exact because no report data is rewritten
