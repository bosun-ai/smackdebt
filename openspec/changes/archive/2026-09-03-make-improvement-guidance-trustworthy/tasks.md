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

- [x] 2.1 Add the opaque analysis-owned match-evidence type at the private
      language seam, update its API snapshot, and keep display identity unchanged.
- [x] 2.2 Supply tested semantic anchors from JavaScript, TypeScript, Vue, and
      Ruby adapters without exposing syntax nodes or digests.
- [x] 2.3 Implement declared, unique-anchor, and unique fingerprint matching in
      that order, with no metric, line, ordinal, fuzzy, or retained-source fallback.
- [x] 2.4 Retain only shared collision groups as ambiguous machine comparisons;
      keep one-sided groups Added or Removed and emit one scope warning counting
      each affected file once without moving the verdict.
- [x] 2.5 Prove unchanged moved callbacks disappear, moved-and-edited callbacks
      become one comparison, genuine additions and removals stay one-sided,
      repeated-current-only and repeated-base-only groups stay one-sided,
      two-to-one groups remain ambiguous, and worker policies produce equal bytes.
- [x] 2.6 Build the release binary and verify the recorded Smackdebt closures,
      Fluyt `WorkflowRunMiniMap.vue`, and both `GraphEditor.vue` revisions in
      terminal and JSON before committing this slice.

      **Completion note (2026-09-01).** The final release build passed the full
      `rtk just check` gate with no debt-ratchet regressions. Generated policy,
      language, project, and CLI tests prove safe pairs, one-sided repeats,
      shared collisions, one warning per file, private JSON, and equal serial
      and parallel bytes. The Smackdebt `d4e78ba` review retained 190 Added, 21
      Removed, and 64 ambiguous closure groups across the current 29-file diff;
      16 files carried the grouped warning; jobs 1 and jobs 4 JSON both hashed
      to `1be49d292685fa404e4187c731b61365d744dccb10020ac78b716828a0c26d4b`.
      `WorkflowRunMiniMap.vue` is
      unchanged from `master`, so terminal and JSON retained no source
      comparison. The exact `GraphEditor.vue` revisions changed from the
      recorded 125 Added, 95 Removed, and 20 MetricChanged rows to 27 Added, 1
      Removed, 20 MetricChanged, and 11 ambiguous groups. Its one file warning
      appeared once, its terminal and JSON conclusions agreed, and jobs 1 and
      jobs 4 JSON both hashed to
      `4205f47d8bdd270be062a189130441cd275e6e290e4fe06f33baa3517abe4376`.
      Temporary revision source and output were removed after aggregate review.

## 3. Generated JavaScript context

- [x] 3.1 Add exact `.js`, `.mjs`, and `.cjs` generated filename fixtures,
      prove explicit configuration wins, and prove similar authored names and
      small `.ts` bundle names follow the remaining rules.
- [x] 3.2 Apply the 65,536-byte and 512-byte-per-nonempty-line rule from the
      existing source read with exact edge fixtures.
- [x] 3.3 Keep generated files in machine and file-detail views while excluding
      them from verdicts, root worst-offender selection, default problems, and
      codebase navigation.
- [x] 3.4 Prove a large ordinary multiline file and a small authored one-line
      file remain primary, and prove common directory names add no role alone.
- [x] 3.5 Build the release binary and verify Netdisco root and `diff HEAD~1`
      keep authored Rust and JavaScript visible while bundled assets no longer
      own the default report before committing this slice.

      **Completion note (2026-09-01).** Built the release binary and ran
      Netdisco root and `diff HEAD~1` in terminal and JSON; all four invocations
      exited 0. Root coverage was 90 selected and 90 analyzed, with no
      unsupported or failed files and 14 generated files. The graph was
      incomplete and withheld 2 reach facts. `swagger-ui-bundle.js` was
      generated and no longer appeared as a worst offender, default problem, or
      navigation target. Authored Rust was the first worst offender, authored
      JavaScript remained in default problems, and navigation selected authored
      JavaScript. The diff selected no source files, emitted no comparisons,
      reported no debt change, and had incomplete current and base graph status
      with zero withheld reach, core, or leakage comparisons. No private source
      or complete private output was retained.

## 4. Neutral and representative diffs

- [x] 4.1 Replace the four analysis-owned diff sentences with the exact neutral
      sentences and retain tier identifiers and movement rules.
- [x] 4.2 Apply the exact view-wide direction, family, subject, line, kind, and
      identity key; reserve one default witness per present direction, then fill
      from that same key.
- [x] 4.3 Prove section ordering, default mixed representation, literal
      `--top 1`, `--all`, fully checked verdict-only no-debt output,
      comparison-trust-warning no-debt output, documentation-only no-debt with
      retained current history context, and the exact unchanged diff footer.
- [x] 4.4 Update terminal and JSON snapshots, README examples, schema examples,
      and documentation tests together.
- [x] 4.5 Build the release binary and verify mixed Fluyt output and Parity's
      documentation-only diff in terminal and JSON before committing this slice.

      **Completion note (2026-09-03).** The final release build passed
      `rtk just gate` with zero regressions and seven improvements, followed by
      the complete `rtk just check` gate. Focused analysis, output, CLI, and
      executable README tests also passed. Mixed selection uses one typed key
      across source, architecture, and history; its default view retains one
      Worse, Better, and Changed witness, while `--top 1` remains one row and
      `--all` retains useful detail. Comparison-confidence warnings now read
      analysis-owned suppression facts for qualifying co-change pairs whose
      explanation changed while history was incomplete or affected by rename
      gaps. Negative coverage proves an ordinary dependency change with no
      qualifying co-change evidence creates no warning.

      On the final Fluyt checkout, `smackdebt diff master` exited 0 in terminal
      and JSON modes. Coverage was 63/63/0/0, graph status was incomplete on
      both sides, suppression was 1/0/0, and comparison counts were
      342/0/1/0/0/0. The terminal showed mixed totals of 4 worse, 2 better, and
      2 changed, retained one visible row for every direction, and ended with
      the exact detail footer. Jobs 1 and jobs 4 JSON both hashed to
      `f5912e32fe0fcf79a38dcaff09a9dc8f00a45350bb9a6271e06fe8cd56fda6b8`.

      In a clean detached Parity worktree at
      `b30f99a8fe29aa386358aafc83d78d64ca88e8bf`, `smackdebt diff HEAD~1`
      exited 0 and printed only the no-debt verdict block with no footer.
      Coverage and every comparison count were zero; graph status was
      incomplete on both sides with 0/0/0 suppression. Jobs 1 and jobs 4 JSON
      both hashed to
      `4cb84719299eec6afd2dd1a05fdf0fde5b8c6000f0e8ccbaff07f231869a4a20`.
      The temporary worktree was removed, and no private source or complete
      private output was retained.

- [x] 4.6 Put primary application problem cards before no-source and
      non-primary cards at the same rating while keeping rating first; prove
      the policy with focused ranking tests, regenerate and review the affected
      committed terminal and JSON bytes, and verify root guidance on Smackdebt,
      Fluyt, Swiftide, and Netdisco with one release binary.

      **Completion note (2026-09-03).** Rating remains the first problem-rank
      key. At equal rating, primary-source cards now precede relationship-only
      cards and non-primary-source cards. Focused tests prove all three classes,
      stable ordering, and that High non-primary debt still precedes Watch
      primary debt. The committed pattern and unified snapshots changed only in
      card order and the resulting root navigation target; normalized JSON was
      otherwise identical. One release binary made Fluyt lead with
      `bow/src/util/manifests/step-form.ts`, whose card claims 13 trusted primary
      findings, while the High benchmark card remained default-visible lower in
      the table and still preceded Watch primary cards. Smackdebt and Swiftide
      retained useful leading findings, and Netdisco retained authored
      JavaScript first. The workload review passed all six outcomes. Independent
      review and the final `rtk just check` both passed with zero debt-gate
      regressions.

## 5. Real-repository proof and closeout

- [x] 5.1 Consume the task 0.1 completion note and run one newly built release
      binary on Smackdebt root and `diff d4e78ba`, Fluyt root and
      `diff master`, GraphEditor current
      `f42f3ae8e76241e06254b7787a87a2f198fb70c2` against base
      `02caca0d3bf05599e1232130cf9265cc9926c9b3`, marketing root,
      Astro file, Astro directory and `diff HEAD`, Netdisco root and
      `diff HEAD~1`, Swiftide root and `diff HEAD~1`, and Parity
      `diff HEAD~1`.
- [x] 5.2 For every run record selected, analyzed, unsupported, and failed file
      counts; graph status; emitted and suppressed architecture comparisons;
      visible directions; terminal footer when present; and exit status. Review
      that terminal and JSON agree and that no private source enters fixtures.
- [x] 5.3 Run format, clippy, workspace tests, strict OpenSpec validation, diff
      checks, API snapshots, dependency-direction checks, allocation checks,
      acceptance snapshots, and `rtk just check`.

      **Completion note (2026-09-03).** Built one fresh release binary from
      `1218040625a1d0c6a55b4f9ae7b05523e30b03dd`; its SHA-256 was
      `9ee555c46c00b37601275ac0069ed4a36ab76d60db278d7e0d6bfbd4d68e9018`.
      Every matrix command ran once with plain terminal output and once with
      `--json --jobs 1`; every invocation exited 0. The digest is SHA-256 of
      the complete JSON bytes. Coverage is selected/analyzed/unsupported/failed.
      Graph is current/base for diffs and current for codebase reports.
      Suppression is propagation/core/leakage. Comparison counts are
      source/emitted package-cycle/propagation/core/leakage/history rows.

      | Case and command | JSON digest | Coverage | Graph; suppression | Comparisons | Terminal and review |
      | --- | --- | --- | --- | --- | --- |
      | Smackdebt `1218040`: `smackdebt`, `smackdebt diff d4e78ba` | `2fbef48026adfef627ff75cfb4fad712aa898100827479dd3357e0b7d1898bfd`, `4e8040f5049ff9d4503c6345e1ddc47404159b9846604d7a037bdd6e7e39f792` | 120/120/0/0, 31/31/0/0 | incomplete; 1/1/0, incomplete/incomplete; 0/0/0 | 0/0/0/0/0/0, 766/0/0/0/0/0 | root is worn; diff is mixed 3 worse, 7 better, 12 changed, visibly shows one row in each direction, and has the footer. Closure rows are 237 Added, 26 Removed, and 71 ambiguous across the later 31-file diff; 18 files carry the grouped warning, so unsafe matches stay visible rather than being guessed. |
      | Fluyt `ea3aceb` with its current worktree: `smackdebt`, `smackdebt diff master` | `f2b74c5e5c692b78eaa95b1b6101e1a974c61cea9b45795659f64e703985a646`, `4393ae01305fdfda29b882a678fc207b992a208646a4f6d76a9677eee8f2e039` | 1887/1887/0/0, 63/63/0/0 | incomplete; 4/0/0, incomplete/incomplete; 1/0/0 | 0/0/0/0/0/0, 342/0/1/0/0/0 | root fights back; diff is mixed 4 worse, 2 better, 2 changed, visibly shows every direction, and has the footer. `WorkflowRunMiniMap.vue` has no comparison because it is unchanged from `master`. The 21-file anonymous warning is honest but still leaves a broad drill-down. |
      | Clean detached GraphEditor `f42f3ae8e...`: `smackdebt diff 02caca0d3bf05599e1232130cf9265cc9926c9b3` | `36e2b0e8f4583105abedf47aace27c98f196bd67f358f614ecb84fa2ab9a914f` | 24/24/0/0 | incomplete/incomplete; 2/0/0 | 259/0/1/0/0/0 | worse 6, better 0, changed 2; three worse rows are visible and the footer is present. `GraphEditor.vue` has 27 Added, 1 Removed, 20 MetricChanged, and 11 ambiguous groups, replacing the pre-change 125 Added and 95 Removed while retaining the 20 changed rows. |
      | Marketing `85bfabe` with its current worktree: `smackdebt`, `smackdebt src/pages/demo.astro`, `smackdebt src/pages`, `smackdebt diff HEAD` | `4e39b8cbc7862084e8ae21ca23095bb7512f1cdd11e84cf40f84ca51a830c312`, `c407e58656a5f1b4a47b868ae1b4f1c3450a200a387dbe16db0b2e6a161cad08`, `7225ebdadbcb9e39f821f0151528a042319adc1219ccb2b94a2f48e999eec14b`, `fb1971d15922786a2179aae42221d4f07a07acbfe871b8b2ab7bdb406f3ad111` | 117/2/115/0, 1/0/1/0, 27/0/27/0, 3/0/3/0 | incomplete; 0/0/0, incomplete; 0/0/0, incomplete; 0/0/0, incomplete/incomplete; 0/0/0 | all zero | root, file, and directory retain their exact scopes and state their exact incomplete coverage. Diff says no debt changed, explains 0 of 3 analyzed, and has the footer. The result is truthful but offers little debt guidance until Astro has analysis support. |
      | Netdisco `fe5eeae`: `smackdebt`, `smackdebt diff HEAD~1` | `a581d23843625c99f6ffdc78f6805ca0f7ca0f5b08fa202e29d009f534e010a8`, `cf0e3a6365184600f285f5c631e1e56259cdd2f3f8bf193ef1cd444de83dc092` | 90/90/0/0, 0/0/0/0 | incomplete; 2/0/0, incomplete/incomplete; 0/0/0 | all zero | root fights back; the named bundle is one of 14 generated files and is absent from default output. Authored Rust is worst, and authored JavaScript remains in problems and navigation. Large public JavaScript still fills much of the short report, so project role configuration may still be needed. Diff is verdict-only with no footer. |
      | Swiftide `aadfb7b`: `smackdebt`, `smackdebt diff HEAD~1` | `cc940aa1f1366dae64c0d121b430d7ce53f695722a971fbbe5a893577745f11b`, `7bf8c2b7958b5e57c5885d3b6a355677990a294e2a10176d23bac66e399767d0` | 207/207/0/0, 3/3/0/0 | incomplete; 3/0/0, incomplete/incomplete; 0/0/0 | 0/0/0/0/0/0, 14/0/0/0/0/0 | root is worn. Diff says no debt changed, retains precise `complete`, `complete_stream`, and `prompt` method comparisons in JSON, and shows one anonymous-match warning plus the footer. The default warning preserves trust but requires a file drill-down to act on it. |
      | Clean detached Parity `b30f99a8fe29aa386358aafc83d78d64ca88e8bf`: `smackdebt diff HEAD~1` | `39af894a4ae315f04a839a8ce2702fb188b0cd1ed29b75cd12afba3ff20bd809` | 0/0/0/0 | incomplete/incomplete; 0/0/0 | all zero | the documentation-only commit is exactly the no-debt verdict form with no footer. |

      Jobs 1 and jobs 4 produced byte-identical terminal and JSON output for
      Smackdebt `diff d4e78ba`, Fluyt `diff master`, the detached GraphEditor
      comparison, and the detached Parity comparison. Repository movement since
      task 0.1 is recorded as observed state rather than treated as a product
      failure. Terminal conclusions and counts agreed with JSON in every case.
      No private source, contributor identity, raw history, secret, or complete
      private terminal or JSON output was retained. Both temporary worktrees
      were removed after review.

      The explicit format, clippy, workspace-test, strict OpenSpec, diff,
      architecture, API-snapshot, dependency-direction, allocation, and
      acceptance checks passed. Workspace tests reported 649 passed and 1
      ignored; the focused allocation check passed; both API snapshot modes
      passed; and the final `rtk just check` passed, including acceptance
      evidence, performance checks, and the debt gate. Release baselines were
      deliberately not run.
- [x] 5.4 Run release baselines only after the complete correctness gate passes,
      review every output and workload change, and complete the deferred release
      task only when its evidence is valid.

      Nine release profiles were recorded from clean accepted revision
      `a939eb7a778596eba8cdb1ccbcb765a4e90d03d6` with five stable-work samples
      each. Workload identity, source shape, report digests, resource maxima,
      budgets, and metadata agreed across the evidence; every generated report
      passed correctness review, and all six real-workload outcomes passed
      without retaining private output. Quiet-host rerecording removed isolated
      timing noise from the two largest profiles. The remaining sub-second
      large-diff increase is accepted for its changed work: 401 rather than 202
      object reads, 1,200 parser visits, 1,404 analysis passes, and the current
      graph and improvement-guidance features. Release evidence, performance
      tests, the debt gate, strict OpenSpec validation, diff checks, and the
      complete workspace check passed.
- [x] 5.5 Update product and architecture documentation to match verified bytes,
      then archive this change only after every task is complete.

      **Completion note (2026-09-03).** Audited the README, architecture guide,
      schema, checked examples, and acceptance bytes against one fresh release
      build. Smackdebt and Fluyt root and diff runs agreed in terminal and JSON.
      The architecture guide now distinguishes codebase reach, core, and leakage
      facts from their current/base diff comparisons, qualifies verdict-only
      no-debt output when comparison confidence is reduced, and lists gate exit
      status 3. README examples, exact diff sentences and footer, coverage,
      Astro, generated JavaScript, anonymous matching, graph suppression,
      problem rank, and all nine release profiles already matched. Focused README
      tests, acceptance, the debt gate, strict OpenSpec validation, diff checks,
      and the complete workspace check passed before archival.
