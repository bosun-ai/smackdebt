## 0. Pre-change evidence

- [x] 0.1 Before task 1.1, run the current release binary in terminal and JSON
      mode over the complete review matrix: Smackdebt root and `diff d4e78ba`;
      Fluyt root and `diff master`, including `WorkflowRunMiniMap.vue`;
      `bow/src/components/graph-editor/GraphEditor.vue` at current
      `f42f3ae8e76241e06254b7787a87a2f198fb70c2` against base
      `02caca0d3bf05599e1232130cf9265cc9926c9b3`; marketing root, Astro file,
      Astro directory, and `diff HEAD`; Netdisco root and `diff HEAD~1`;
      Swiftide root and `diff HEAD~1`; and Parity `diff HEAD~1`. Record each
      command, report digest, selected/analyzed/unsupported/failed counts, graph
      status, emitted and suppressed architecture comparisons, visible
      directions, footer presence, and exit status in a completion note directly
      beneath this task. Record aggregates and already-emitted paths only; copy
      no source, contributor identity, raw Git history, secret, or complete
      private terminal or JSON output. The observed GraphEditor starting point
      is 125 Added, 95 Removed, and 20 MetricChanged closure rows.

      **Completion note (2026-09-01).** Built `target/release/smackdebt` at
      `26c0336`. Every command below ran once as terminal output and once with
      `--json`; the digest is SHA-256 of the complete JSON bytes. Coverage is
      selected/analyzed/unsupported/failed. Graph is current/base for diffs and
      current for codebase reports. Suppression is propagation/core/leakage.
      Comparison counts are source/emitted package-cycle/propagation/core/
      leakage/history rows.
      Every invocation exited 0.

      | Case and command | JSON digest | Coverage | Graph; suppression | Comparisons | Terminal |
      | --- | --- | --- | --- | --- | --- |
      | Smackdebt: `smackdebt`, `smackdebt diff d4e78ba` | `441da3e26b6861d989a351b731ef94592169e8616fed704b6bef8ceae50ccf90`, `1da573177195a4005f853767d8a24fa51bf403fbf838bd20984d062aff63cd21` | 119/119/0/0, 20/20/0/0 | incomplete; 1/1/0, incomplete/incomplete; 0/0/0 | 0/0/0/0/0/0, 1465/0/0/0/0/0 | worn; mixed 4 worse, 5 better, 6 changed; footer present |
      | Fluyt: `smackdebt`, `smackdebt diff master` | `cab3d3e0aff0823fe89c29809aedd58cc903192bcb4f56d815da62458d797d02`, `fafccc7de0baa9a5cb2e3be010901b9b2e344b247df83b5d3b0acdb7f295b0e0` | 1877/1877/0/0, 23/23/0/0 | incomplete; 4/0/0, incomplete/incomplete; 1/0/0 | 0/0/0/0/0/0, 261/0/0/0/0/0 | fights back; mixed 1 worse, 1 better; footer present |
      | GraphEditor detached `f42f3ae8e...` versus `02caca0d3...`: `smackdebt diff 02caca0d3bf05599e1232130cf9265cc9926c9b3` | `cfde35c92821933254c1cd0f8c64d650b2ed01a806ac4f941b9e84d77e919abe` | 24/24/0/0 | incomplete/incomplete; 2/0/0 | 842/0/0/0/0/0 | mixed 7 worse, 2 better, 2 changed; footer present; the named file has 125 Added, 95 Removed, and 20 MetricChanged closure rows |
      | Marketing: `smackdebt`, `smackdebt src/pages/demo.astro`, `smackdebt src/pages`, `smackdebt diff HEAD` | `89a3283b59fd0e582ad510e571222b336906ffefbb2532e0dc4f1b1946dd0be5`, `3a16549e0c53ebf5c5eb4e920894c30b0257bbeb190aa4ed91e0a072f88e28ac`, `ab3a9e07eb3b55b7319fbec171b852e78b014f7011c6b84be95222cf7f02d91e`, `5363603bfa69e37ec7700f5d2df9ad72fe2e7a6e7b0b859cb820d4930cdd2069` | 2/2/0/0, 2/2/0/0, 2/2/0/0, 0/0/0/0 | complete; 0/0/0 for all codebase runs, complete/complete; 0/0/0 | all comparison counts zero | root, explicit Astro file, and Astro directory all return the same worn repository result and `next:` target; diff says no debt changed with no footer |
      | Netdisco: `smackdebt`, `smackdebt diff HEAD~1` | `29571f9bfb2e6a98d1195f50b0901a1fc2c38f3c404a44e27fca412e2f658264`, `890a9aa5a332779807f4dff399c03511d67f42cd37232cc6c91fa42c3ebe7dbd` | 90/90/0/0, 0/0/0/0 | incomplete; 2/0/0, incomplete/incomplete; 0/0/0 | all comparison counts zero | root fights back and navigates to `share/public/swagger-ui/swagger-ui-bundle.js`; diff says no debt changed with no footer |
      | Swiftide: `smackdebt`, `smackdebt diff HEAD~1` | `cc940aa1f1366dae64c0d121b430d7ce53f695722a971fbbe5a893577745f11b`, `de51733c3dfd68e0bb8bea3abdea29cc8cf160b913be191ee6fa8ebbf03cc315` | 207/207/0/0, 3/3/0/0 | incomplete; 3/0/0, incomplete/incomplete; 0/0/0 | 0/0/0/0/0/0, 58/0/0/0/0/0 | worn; no debt changed with 1 changed source row and no footer |
      | Clean detached Parity: `smackdebt diff HEAD~1` | `70375eeb0f8af0542f8a592c9760df58b8ff78028ca2cc33fcf9091c45c4bb24` | 7/7/0/0 | incomplete/incomplete; 0/0/0 | 259/0/0/0/0/0 | no debt changed with 2 changed source rows and no footer |

      The Fluyt `diff master` report contained no comparison for
      `WorkflowRunMiniMap.vue` in either checkout. The detached worktrees kept
      dirty user files out of the GraphEditor and Parity measurements. No source,
      contributor identity, raw history, or complete private output was retained.

## 1. Scope and coverage

- [x] 1.1 Add `.astro` as recognized, explicitly unsupported source and pin its
      codebase and diff inventory behavior with discovery and language fixtures.
- [x] 1.2 Remove explicit-path fallback and add exact supported file,
      unsupported file, source directory, empty directory, non-source file, and
      missing-path acceptance cases.
- [x] 1.3 Make every incomplete file selection carry both exact qualifier
      sentences and add its selected and analyzed counts to JSON version 4.
- [x] 1.4 Update schema, index checks, API snapshots, terminal snapshots, README,
      and architecture documentation for the verified behavior.
- [x] 1.5 Run focused discovery, language, project, output, and CLI tests, build a
      release binary, and verify marketing root, Astro file, Astro directory,
      and diff output in terminal and JSON before committing this slice.

## 2. Anonymous-unit matching

- [ ] 2.1 Add the opaque analysis-owned match-evidence type at the private
      language seam, update its API snapshot, and keep display identity unchanged.
- [ ] 2.2 Supply tested semantic anchors from JavaScript, TypeScript, Vue, and
      Ruby adapters without exposing syntax nodes or digests.
- [ ] 2.3 Implement declared, unique-anchor, and unique fingerprint matching in
      that order, with no metric, line, ordinal, fuzzy, or retained-source fallback.
- [ ] 2.4 Retain only shared collision groups as ambiguous machine comparisons;
      keep one-sided groups Added or Removed and emit one scope warning counting
      each affected file once without moving the verdict.
- [ ] 2.5 Prove unchanged moved callbacks disappear, moved-and-edited callbacks
      become one comparison, genuine additions and removals stay one-sided,
      repeated-current-only and repeated-base-only groups stay one-sided,
      two-to-one groups remain ambiguous, and worker policies produce equal bytes.
- [ ] 2.6 Build the release binary and verify the recorded Smackdebt closures,
      Fluyt `WorkflowRunMiniMap.vue`, and both `GraphEditor.vue` revisions in
      terminal and JSON before committing this slice.

## 3. Generated JavaScript context

- [ ] 3.1 Add exact `.js`, `.mjs`, and `.cjs` generated filename fixtures,
      prove explicit configuration wins, and prove similar authored names and
      small `.ts` bundle names follow the remaining rules.
- [ ] 3.2 Apply the 65,536-byte and 512-byte-per-nonempty-line rule from the
      existing source read with exact edge fixtures.
- [ ] 3.3 Keep generated files in machine and file-detail views while excluding
      them from verdicts, root worst-offender selection, default problems, and
      codebase navigation.
- [ ] 3.4 Prove a large ordinary multiline file and a small authored one-line
      file remain primary, and prove common directory names add no role alone.
- [ ] 3.5 Build the release binary and verify Netdisco root and `diff HEAD~1`
      keep authored Rust and JavaScript visible while bundled assets no longer
      own the default report before committing this slice.

## 4. Neutral and representative diffs

- [ ] 4.1 Replace the four analysis-owned diff sentences with the exact neutral
      sentences and retain tier identifiers and movement rules.
- [ ] 4.2 Apply the exact view-wide direction, family, subject, line, kind, and
      identity key; reserve one default witness per present direction, then fill
      from that same key.
- [ ] 4.3 Prove section ordering, default mixed representation, literal
      `--top 1`, `--all`, fully checked verdict-only no-debt output,
      comparison-trust-warning no-debt output, documentation-only no-debt with
      retained current history context, and the exact unchanged diff footer.
- [ ] 4.4 Update terminal and JSON snapshots, README examples, schema examples,
      and documentation tests together.
- [ ] 4.5 Build the release binary and verify mixed Fluyt output and Parity's
      documentation-only diff in terminal and JSON before committing this slice.

## 5. Real-repository proof and closeout

- [ ] 5.1 Consume the task 0.1 completion note and run one newly built release
      binary on Smackdebt root and `diff d4e78ba`, Fluyt root and
      `diff master`, GraphEditor current
      `f42f3ae8e76241e06254b7787a87a2f198fb70c2` against base
      `02caca0d3bf05599e1232130cf9265cc9926c9b3`, marketing root,
      Astro file, Astro directory and `diff HEAD`, Netdisco root and
      `diff HEAD~1`, Swiftide root and `diff HEAD~1`, and Parity
      `diff HEAD~1`.
- [ ] 5.2 For every run record selected, analyzed, unsupported, and failed file
      counts; graph status; emitted and suppressed architecture comparisons;
      visible directions; terminal footer when present; and exit status. Review
      that terminal and JSON agree and that no private source enters fixtures.
- [ ] 5.3 Run format, clippy, workspace tests, strict OpenSpec validation, diff
      checks, API snapshots, dependency-direction checks, allocation checks,
      acceptance snapshots, and `rtk just check`.
- [ ] 5.4 Run release baselines only after the complete correctness gate passes,
      review every output and workload change, and complete the deferred release
      task only when its evidence is valid.
- [ ] 5.5 Update product and architecture documentation to match verified bytes,
      then archive this change only after every task is complete.
