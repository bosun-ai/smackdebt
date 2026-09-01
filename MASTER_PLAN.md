# Trustworthy Improvement Guidance

## Summary

Make Smackdebt's output truthful before making it broader. Fix misleading scope
and coverage, incorrect anonymous-unit comparisons, generated-asset noise,
incomplete graph evidence, and diff wording. Preserve legitimate zero-result
metrics and real Rust dependency cycles.

The current metrics wave passes `just check`, but remains uncommitted and
contains documentation conflicts plus missing diff-suppression evidence. Review
and finish it before starting new work.

## Execution and Review Protocol

- Stage only `MASTER_PLAN.md` first and commit it as
  `docs: add trustworthy guidance master plan`.
- Use one `gpt-5.6-sol` medium worker at a time. Workers may edit their assigned
  stage but must not commit or touch unrelated work.
- After each worker finishes, use a separate read-only reviewer to challenge
  correctness and UX.
- The main agent independently:
  1. Reviews the complete diff.
  2. Checks claims against the relevant source and Git change.
  3. Runs focused tests.
  4. Builds the release binary and tests real repositories.
  5. Runs the complete gate.
  6. Commits only if behavior is correct.
- Return incorrect or unclear work to the same worker with concrete findings.
  Do not advance with unresolved findings.
- Keep implementation stages sequential to avoid shared-worktree conflicts.

## Implementation Stages

### 1. Finish the Current Reach and Leakage Wave

- Correct documentation that still says reach, core, and leakage never appear
  in diffs. Clarify that amplification remains codebase-only.
- Add diff-specific graph evidence carrying separate current and base status
  plus suppressed reach, core, and leakage counts.
- Count candidate comparisons withheld by incomplete evidence before filtering
  them out. Terminal warnings must identify whether evidence was incomplete
  before, after, or on both sides.
- Do not lower thresholds to force findings. A zero comparison count is valid
  when values are unchanged, below the published floor, or visibly suppressed.
- Do not broadly suppress Rust cycles. Inspect every tested witness and retain
  genuine sibling or cross-module `uses` cycles; correct only demonstrably wrong
  resolution or direct ownership wiring.
- Re-run root and diff reports on Smackdebt, Fluyt, Netdisco, and the marketing
  site.
- Run `rtk just check`, review the whole existing wave, then commit as
  `feat(analysis): compare trusted graph movement`.

### 2. Specify the UX Corrections

- Create a focused OpenSpec change named
  `make-improvement-guidance-trustworthy`.
- Specify scope errors, unsupported Astro coverage, generated JavaScript
  classification, anonymous-unit matching, representative mixed diffs, neutral
  verdict wording, JSON additions, and acceptance examples.
- Pass `rtk openspec validate --all --strict` before product-code edits.
- Commit as `docs: specify trustworthy improvement guidance`.

### 3. Make Scope and Coverage Truthful

- Recognize `.astro` as source and add `Language::Astro` as explicitly
  unsupported. Do not add an Astro parser in this change.
- Retain changed Astro files in codebase and diff inventories so coverage and
  graph trust reflect them.
- Remove fallback from an explicitly selected path to repository root.
- Use these exact outcomes:
  - Supported or recognized-unsupported file: report that file.
  - Directory containing source: report that directory.
  - Directory without source: exit 1 with
    `smackdebt: no source files found under: <path>`.
  - Existing non-source file: exit 1 with
    `smackdebt: not a source file: <path>`.
  - Missing path: retain `smackdebt: path not found: <path>`.
- Qualify every report with incomplete source coverage, without a percentage
  threshold:
  - `Not all source was checked.`
  - `<analyzed> of <selected> source files were analyzed.`
- Keep the grouped warning explaining the unsupported language count.
- Add `selected_files` and `analyzed_files` to the JSON qualifier while
  retaining existing byte-share fields.
- Commit as `fix(project): report unsupported selected source`.

### 4. Match Moved Anonymous Units Safely

- Add a private analysis-owned match key separate from display identity.
- Keep declared functions and methods matched by declared identity.
- Let language adapters provide stable semantic anchors:
  - assignment or binding name;
  - enclosing `computed`, `watch`, or callback call plus argument position;
  - stable neighboring literal when relevant;
  - Ruby example or context description.
- For anonymous units without an anchor, use a deterministic digest of their
  exact syntax bytes only when unique within the file.
- Match in this order: declared identity, unique semantic anchor, unique syntax
  digest.
- Never match by measurements, line number, or source ordinal.
- Keep line-based names only for display.
- When safe pairing is impossible, retain an ambiguous machine comparison and
  show one grouped file warning. Do not guess or move the verdict.
- Prove moved-and-edited callbacks become one comparison, unchanged moved
  callbacks disappear, and genuine additions remain additions.
- Commit as `fix(analysis): match moved anonymous units`.

### 5. Stop Generated JavaScript Owning the Report

- Classify these JavaScript-family names as generated:
  - `*.min.js`, `*.min.mjs`, `*.min.cjs`
  - `*.bundle.js`, `*.bundle.mjs`, `*.bundle.cjs`
  - `*-bundle.js`, with matching module suffixes
- Also classify a JavaScript-family file as generated when it is at least 64
  KiB and averages at least 512 bytes per nonempty physical line. Reuse the
  existing source read.
- Keep generated files in JSON and file inspection, but exclude them from
  verdicts, root worst-offender selection, default problems, and codebase
  navigation.
- Do not classify all `public`, `share`, or `assets` directories as generated.
- Prove large ordinary multiline JavaScript and small authored one-line files
  remain primary.
- Commit as `fix(discovery): classify generated javascript assets`.

### 6. Make Diff Conclusions Neutral and Representative

- Keep current movement policy and tier IDs.
- Replace verdict sentences with:
  - `No debt changed.`
  - `Debt decreased.`
  - `Debt increased.`
  - `Debt increased in some places and decreased in others.`
- For default mixed output, reserve one visible witness for every nonzero
  direction before filling remaining slots by existing rank.
- Apply that selection across source, architecture, and history so visible rows
  support the verdict.
- Preserve Worse, Better, Changed ordering.
- Respect explicit `--top 1`; never exceed the requested limit.
- Do not add omitted-row notices.
- Keep the existing diff footer exactly
  `inspect directories and files for more details`.
- Commit as `fix(output): make diff conclusions neutral and representative`.

### 7. Real-Repository Proof and Closeout

Run one newly built release binary and compare it with saved pre-stage output:

- Smackdebt root and `diff d4e78ba`: identical moved closures must not appear as
  added and removed.
- Fluyt root and `diff master`, including `WorkflowRunMiniMap.vue`: changed
  callbacks must be paired correctly.
- The two recorded Fluyt revisions around `GraphEditor.vue`: the moved-and-edited
  callback must become one changed comparison.
- Marketing root, Astro file, Astro directory, and `diff HEAD`: scope must remain
  exact and Astro must appear as unsupported coverage.
- Netdisco root and `diff HEAD~1`: bundled assets must not own the default
  report; authored Rust and JavaScript remain visible.
- Swiftide root and `diff HEAD~1`: precise method findings remain stable.
- Parity `diff HEAD~1`: the documentation-only commit remains
  `No debt changed.`

For every run, inspect terminal and JSON and record selected, analyzed,
unsupported, and failed files; graph status; emitted and suppressed architecture
comparisons; visible directions; and exit status.

Then run:

```console
rtk cargo fmt --all -- --check
rtk cargo clippy --workspace --all-targets --all-features -- -D warnings
rtk cargo test --workspace --all-features
rtk openspec validate --all --strict
rtk git diff --check
rtk just check
```

After all code and documentation commits leave a clean tree:

```console
rtk just release-baselines /Users/timonv/projects/smackdebt /Users/timonv/projects/fluyt /Users/timonv/projects/swiftide --accept-report-change
```

Review every baseline and workload-output change, complete the deferred
performance task only when the evidence is valid, and commit as
`perf: refresh release evidence`. Archive completed OpenSpec changes only after
every task and validation passes, then commit the archive separately.

## Public Contract Changes and Assumptions

- JSON schema version 4 gains `astro`, file-count coverage fields, and separate
  current/base comparison graph evidence. No schema-version increase.
- Diff tier IDs and movement rules remain unchanged; sentence bytes
  intentionally change.
- Explicit invalid scopes now exit 1 instead of silently returning repository
  data.
- Anonymous match keys remain private and never expose source digests.
- No Astro analyzer, threshold reduction, broad Rust-cycle suppression, or
  automatic Git push is included.
- Existing unrelated work remains untouched.
