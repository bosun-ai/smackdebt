## 1. Architecture contract

- [x] 1.1 Preserve and pass characterization tests for CLI, terminal, JSON, selected scope, and serial/parallel behavior
- [x] 1.2 Add tested entry-module and compiler-visible API checks
- [x] 1.3 Enable the workspace lint that rejects unreachable public items

## 2. Analysis and languages

- [x] 2.1 Split analysis into private source, health, comparison, and report modules
- [x] 2.2 Add analysis-owned report construction with shared paths and immutable completed reports
- [x] 2.3 Remove unused analysis facts and public report mutation methods
- [x] 2.4 Split languages into dispatch, upstream, Ruby, and Vue modules behind one analyzer interface
- [x] 2.5 Pass focused analysis and language tests

## 3. Infrastructure and project

- [x] 3.1 Separate ignore policy from discovery and use the analysis-owned package identity through a small inventory interface
- [x] 3.2 Return sorted unique changes through a small repository interface and hide status parsing and process details
- [x] 3.3 Separate public project requests from private codebase and diff execution
- [x] 3.4 Share hierarchy construction across project flows, move aggregation into analysis, and remove complete report-table clones
- [x] 3.5 Preserve one walk, one current-source read, fixed Git process counts, and stable parallel output

## 4. Output and CLI

- [x] 4.1 Separate JSON serialization from terminal presentation and keep terminal helpers private
- [x] 4.2 Resolve report paths through the shared path table while preserving JSON schema version 1
- [x] 4.3 Split CLI arguments, configuration, terminal policy, and application execution from `main.rs`
- [x] 4.4 Pass focused output and CLI acceptance tests

## 5. Enforcement and documentation

- [x] 5.1 Reduce every `lib.rs` and `mod.rs` to allowed wiring and enable the entry-module gate
- [x] 5.2 Refresh compiler-visible API snapshots and dependency-direction expectations
- [x] 5.3 Update architecture documentation with module placement and ownership rules
- [x] 5.4 Run complete formatting, lint, test, architecture, license, performance, OpenSpec, and diff checks
