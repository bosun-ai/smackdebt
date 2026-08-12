## 1. Accepted contract and report values

- [ ] 1.1 Add and strictly validate the progressive-exploration, architecture,
  performance, and documentation deltas before implementation
- [ ] 1.2 Add typed report-path identity, selected-scope output, comparison file
  ownership, scope comparison links, and three-way diff counts to pure analysis
  values
- [ ] 1.3 Implement and test the pure diff-direction policy for every comparison
  kind and healthy, Watch, and High addition or removal
- [ ] 1.4 Extend reserved post-order aggregation for finding links, comparison
  links, health, coverage, and diff counts without duplicate ownership
- [ ] 1.5 Update analysis API snapshots and allocation tests for the intentional
  report contract change

## 2. Shared hierarchy and selection

- [ ] 2.1 Extract one hierarchy builder used by codebase and diff reports for
  repository, package, directory, and file scopes
- [ ] 2.2 Preserve repository-relative path identity while limiting explicit
  codebase path discovery and source reads to the selected subtree or file
- [ ] 2.3 Retain the initial selected scope, including a zero-total scope for an
  existing directory with no supported source
- [ ] 2.4 Add diff package discovery from distinct changed-path ancestors and
  changed current or previous manifest paths without per-file repeated I/O
- [ ] 2.5 Cover nested and root packages, selected subtrees, empty selections,
  rename destinations, deletions, and deleted or renamed manifests

## 3. Progressive terminal reports

- [ ] 3.1 Render codebase child rows with exact health counts, selected-scope
  shares, severity-first order, smart single-child skipping, and breadcrumbs
- [ ] 3.2 Render diff child rows with Worse, Better, Changed, selected-scope
  shares, and deterministic direction-first detail
- [ ] 3.3 Apply the ten-row and three-detail defaults, debt-bearing priority,
  healthy-only fill, exact omitted counts, and the first useful drill command
- [ ] 3.4 Render complete file findings or comparisons with containers and
  copyable repository-relative source locations
- [ ] 3.5 Add terminal-only `--all`, reject `--all --json`, and retain useful
  narrow-terminal output

## 4. JSON, documentation, and complete proof

- [ ] 4.1 Add the version 1 selected-scope, indexed path, scope-link, diff-count,
  comparison-file, and direction fields without changing existing JSON fields
- [ ] 4.2 Add black-box terminal and JSON snapshots for repository, package,
  directory, file, empty, no-debt, coverage-failure, truncated, `--all`, and diff
  flows
- [ ] 4.3 Prove serial and parallel output equality and prove that rendering
  several scopes from one root report performs no discovery, source, Git, parser,
  or worker work
- [ ] 4.4 Re-run source-read, inventory, Git-process, allocation, memory, API,
  dependency, license, and performance checks affected by the larger report
- [ ] 4.5 Update README and architecture examples to match verified behavior,
  archive this change, then resume first-release compatibility evidence
- [ ] 4.6 Run formatting, warning-free workspace linting, all tests, strict
  OpenSpec validation, acceptance snapshots, and `git diff --check`
