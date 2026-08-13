## 1. Contract

- [x] 1.1 Add and strictly validate the terminal and hierarchy deltas
- [x] 1.2 Add focused tests for package fallback, attention rate, row filtering,
  finding reasons, and drill selection

## 2. Reliable hierarchy

- [x] 2.1 Pass discovery package identity into shared codebase hierarchy
- [x] 2.2 Remove the repeated codebase nearest-root lookup and panic path
- [x] 2.3 Prove a mixed repository with source outside manifest roots succeeds

## 3. Useful progressive output

- [x] 3.1 Add rated-unit and attention-rate summary language
- [x] 3.2 Show debt-bearing child rows by default and summarize quiet areas
- [x] 3.3 Show both selected-scope debt share and local debt rate
- [x] 3.4 Explain each retained finding using the signals that caused its rating
- [x] 3.5 Point `Explore` to the first visible debt-bearing row

## 4. Proof and documentation

- [x] 4.1 Verify root, selected directory, file, no-debt, `--all`, JSON, and diff
  output against fixtures
- [x] 4.2 Verify the complete Fluyt report succeeds and is useful
- [x] 4.3 Update README and architecture examples
- [x] 4.4 Run the complete workspace gate, archive this change, and re-run strict
  OpenSpec validation
