# Checked command examples

These short examples run against generated public repositories in the acceptance
evidence. Each comment declares the exact exit status, empty stderr, and the
stable stdout fragments that must appear in the stated order.

<!-- smackdebt-example fixture=evolution status=0 stderr=empty stdout=smackdebt_·_repository_root|checked|PROBLEMS|packages_change_together_·_a_↔_b -->
```console
smackdebt --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=worktree-change status=0 stderr=empty stdout=smackdebt_diff_·_repository_root|Debt_increased_in_some_places_and_decreased_in_others.|worse|better|changed|AREAS|FINDINGS -->
```console
smackdebt diff main --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=worktree-change status=0 stderr=empty stdout=Debt_increased_in_some_places_and_decreased_in_others.|worse_b|better_a|changed_c|next:_smackdebt_diff_main_b/main.js -->
```console
smackdebt diff main --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=worktree-change status=0 stderr=empty stdout=Debt_increased.|worse_package_dependency_cycle_introduced -->
```console
smackdebt diff main c --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=worktree-change status=0 stderr=empty stdout=Debt_decreased.|better_a|next:_smackdebt_diff_main_a/main.js -->
```console
smackdebt diff main a --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=worktree-change status=0 stderr=empty stdout=No_debt_changed. -->
```console
smackdebt diff main new/untracked.js --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=comparison-trust-warning status=0 stderr=empty stdout=No_debt_changed.|Not_all_source_was_checked.|0_of_1_source_files_were_analyzed.|WARNINGS|1_source_file_uses_an_unsupported_language -->
```console
smackdebt diff main --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=comparison-trust-warning status=0 stderr=empty stdout=smackdebt_·_page.astro|Not_all_source_was_checked.|0_of_1_source_files_were_analyzed.|1_source_file_uses_an_unsupported_language -->
```console
smackdebt page.astro --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=comparison-trust-warning status=1 stderr=smackdebt:_path_not_found:_does/not/exist stdout=empty -->
```console
smackdebt does/not/exist --color never
```

<!-- smackdebt-example fixture=comparison-trust-warning status=1 stderr=smackdebt:_no_source_files_found_under:_docs stdout=empty -->
```console
smackdebt docs --color never
```

<!-- smackdebt-example fixture=comparison-trust-warning status=1 stderr=smackdebt:_not_a_source_file:_README.txt stdout=empty -->
```console
smackdebt README.txt --color never
```

<!-- smackdebt-example fixture=generated-javascript status=0 stderr=empty stdout=No_debt_changed.|1_file_has_anonymous_units_that_could_not_be_matched_safely -->
```console
smackdebt diff HEAD~1 bundles/collision.bundle.js --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=generated-javascript status=0 stderr=empty stdout=FINDINGS|transitions/from-generated.js -->
```console
smackdebt diff HEAD~1 transitions/from-generated.js --all --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=worktree-change status=0 stderr=empty stdout=Debt_increased_in_some_places_and_decreased_in_others.|FINDINGS|worse_b|next:_smackdebt_diff_main_b/main.js -->
```console
smackdebt diff main --top 1 --color never --jobs 1 --history 36500d
```

<!-- smackdebt-example fixture=worktree-change status=0 stderr=empty stdout=Debt_increased_in_some_places_and_decreased_in_others.|FINDINGS|ARCHITECTURE|HISTORY|next:_smackdebt_diff_main_b/main.js -->
```console
smackdebt diff main --all --color never --jobs 1 --history 36500d
```
Terminal and JSON output contain only aggregate contributor counts and
concentration operands. History and all other analysis stay on the local
machine.

Diff reports show history as existing context for changed files and packages.
Historical values are not labelled better or worse. When a worktree dependency
changes the finding, the terminal says the packages `now change together
without a code dependency` or `no longer change together without a code
dependency`. The retained history values do not change.


## Selected scopes

<!-- smackdebt-example fixture=generated-javascript status=0 stderr=empty stdout=smackdebt_·_bundles/vendor.min.js|0_high|PROBLEMS|generated -->
```console
smackdebt bundles/vendor.min.js --color never
```

<!-- smackdebt-example fixture=source-roles status=0 stderr=empty stdout=smackdebt_·_share/jquery.plugin.js|0_high|PROBLEMS|vendored -->
```console
smackdebt share/jquery.plugin.js --color never
```
