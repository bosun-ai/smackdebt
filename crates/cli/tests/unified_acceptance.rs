mod support;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use serde_json::Value;

#[cfg(unix)]
use support::coverage_failure_repository;
use support::{
    GeneratedRepository, Invocation, copy_language_truth_files, evolution_repository,
    ref_diff_repository, shallow_clone, source_role_repository, static_architecture_repository,
    worktree_change_repository,
};

#[derive(Debug, Deserialize)]
struct LanguageFacts {
    files: Vec<LanguageFileFacts>,
}

#[derive(Debug, Deserialize)]
struct LanguageFileFacts {
    path: String,
    units: Vec<(String, u64, u64, u64)>,
}

const PRIVATE_JSON_KEYS: &[&str] = &[
    "source_text",
    "commit_message",
    "author",
    "authors",
    "author_name",
    "author_address",
    "author_email",
    "author_identity",
    "raw_author",
    "raw_author_name",
    "raw_author_address",
    "raw_author_email",
    "address",
    "email",
    "email_address",
    "contributor_id",
    "contributor_ids",
    "contributor_identity",
    "contributor_identities",
    "contributor_name",
    "contributor_address",
    "contributor_email",
];

#[test]
fn every_supported_language_matches_the_public_fact_manifest() {
    let repository = GeneratedRepository::new("main");
    copy_language_truth_files(&repository);
    let facts: LanguageFacts = repository.facts("all-languages.json");
    let result = Invocation::new(["--json"]).run(repository.path());
    result.success();
    let report = checked_json(&result.stdout);
    let paths = report["paths"].as_array().unwrap();
    let files = report["files"].as_array().unwrap();
    let findings = report["findings"].as_array().unwrap();

    for expected_file in facts.files {
        let file = files
            .iter()
            .find(|file| {
                let path = file["path"].as_u64().unwrap() as usize;
                paths[path] == expected_file.path
            })
            .unwrap_or_else(|| panic!("missing {}", expected_file.path));
        let file_id = file["id"].as_u64().unwrap();
        let actual: Vec<_> = findings
            .iter()
            .filter(|finding| finding["file"] == file_id)
            .map(|finding| {
                (
                    finding["name"].as_str().unwrap().to_owned(),
                    finding["measurements"]["cognitive_complexity"]
                        .as_u64()
                        .unwrap(),
                    finding["measurements"]["cyclomatic_complexity"]
                        .as_u64()
                        .unwrap(),
                    finding["measurements"]["logical_lines"].as_u64().unwrap(),
                )
            })
            .collect();
        assert_eq!(actual, expected_file.units, "{}", expected_file.path);
    }
}

#[test]
fn every_source_role_matches_the_public_fact_manifest() {
    let repository = source_role_repository();
    let facts: Value = repository.facts("source-roles.json");
    let result = Invocation::new(["--json"]).run(repository.path());
    result.success();
    let automatic = Invocation::new(["--json"])
        .automatic_workers()
        .run(repository.path());
    assert_eq!(result, automatic);
    let report = checked_json(&result.stdout);
    for expected in facts["files"].as_array().unwrap() {
        let file = report["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|file| {
                let path = file["path"].as_u64().unwrap() as usize;
                report["paths"][path] == expected["path"]
            })
            .unwrap_or_else(|| panic!("missing role fixture {}", expected["path"]));
        assert_eq!(file["role"], expected["role"]);
    }
    let root = report["root"].as_u64().unwrap() as usize;
    let health = report["scopes"][root]["health"].as_u64().unwrap() as usize;
    assert_eq!(report["health"][health]["high"], facts["verdict_findings"]);
    assert_eq!(report["findings"].as_array().unwrap().len(), 6);
    assert_golden("unified-source-roles.json", &result.stdout);
}

#[test]
fn static_architecture_fact_manifest_matches_public_json() {
    let repository = static_architecture_repository();
    let facts: Value = repository.facts("static-architecture.json");
    let result = Invocation::new(["--json"]).run(repository.path());
    result.success();
    let report = checked_json(&result.stdout);
    for field in ["internal", "external", "unresolved", "ambiguous"] {
        assert_eq!(
            report["dependency_coverage"][field], facts[field],
            "{field}"
        );
    }
    assert_eq!(
        report["architecture_findings"].as_array().unwrap().len() as u64,
        facts["architecture_findings"]
    );
    let kinds: HashSet<_> = report["architecture_findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| finding["kind"].as_str().unwrap())
        .collect();
    for kind in facts["required_kinds"].as_array().unwrap() {
        assert!(kinds.contains(kind.as_str().unwrap()));
    }
}

#[test]
fn evolution_fact_manifest_matches_public_json_and_keeps_identity_private() {
    let repository = evolution_repository();
    let facts: Value = repository.facts("evolution.json");
    let result = Invocation::new(["--json", "--history", "36500d"]).run(repository.path());
    result.success();
    let report = checked_json(&result.stdout);
    assert_eq!(report["history_coverage"]["commits"], facts["commits"]);
    let coupling = &report["change_coupling"][0];
    assert_eq!(coupling["shared_commits"], facts["shared_commits"]);
    assert_eq!(coupling["union_commits"], facts["union_commits"]);
    assert_eq!(coupling["shared_commits"], facts["similarity_numerator"]);
    assert_eq!(coupling["union_commits"], facts["similarity_denominator"]);
    assert_eq!(
        report["history_coverage"]["uncounted_changes"],
        facts["binary_changes_without_line_totals"]
    );
    assert_private_values_absent(&facts, &result.stdout);
}

#[test]
fn unified_codebase_terminal_and_json_are_exact_and_deterministic() {
    let repository = worktree_change_repository();
    let concise = Invocation::new(std::iter::empty::<&str>()).run(repository.path());
    concise.success();
    assert_golden("unified-codebase-concise.terminal.txt", &concise.stdout);
    let serial_terminal = Invocation::new(["--all"]).run(repository.path());
    serial_terminal.success();
    let parallel_terminal = Invocation::new(["--all"])
        .automatic_workers()
        .run(repository.path());
    parallel_terminal.success();
    assert_eq!(serial_terminal, parallel_terminal);
    assert_golden("unified-codebase-120.terminal.txt", &serial_terminal.stdout);

    let serial_json = Invocation::new(["--json"]).run(repository.path());
    serial_json.success();
    let parallel_json = Invocation::new(["--json"])
        .automatic_workers()
        .run(repository.path());
    parallel_json.success();
    assert_eq!(serial_json, parallel_json);
    let report = checked_json(&serial_json.stdout);
    assert_unified_facts(&report);
    assert_golden("unified-codebase.json", &serial_json.stdout);
}

#[test]
fn worktree_diff_reports_the_declared_mixed_change_outcomes_once() {
    let repository = worktree_change_repository();
    let facts: Value = repository.facts("worktree-change.json");
    let terminal =
        Invocation::new(["diff", "main", "--all", "--history", "36500d"]).run(repository.path());
    terminal.success();
    let parallel_terminal = Invocation::new(["diff", "main", "--all", "--history", "36500d"])
        .automatic_workers()
        .run(repository.path());
    parallel_terminal.success();
    assert_eq!(terminal, parallel_terminal);
    assert_golden("unified-worktree-diff.terminal.txt", &terminal.stdout);
    let json =
        Invocation::new(["diff", "main", "--json", "--history", "36500d"]).run(repository.path());
    json.success();
    let parallel_json = Invocation::new(["diff", "main", "--json", "--history", "36500d"])
        .automatic_workers()
        .run(repository.path());
    parallel_json.success();
    assert_eq!(json, parallel_json);
    let report = checked_json(&json.stdout);
    assert_golden("unified-worktree-diff.json", &json.stdout);
    assert!(
        report["packages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|package| package["path"] == "gone" && package["presence"] == "base_only")
    );
    assert!(
        report["evolutionary_findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    let comparison_directions = strings(&report["comparisons"], "direction");
    for direction in facts["comparison_directions"].as_array().unwrap() {
        assert!(comparison_directions.contains(direction.as_str().unwrap()));
    }
    let architecture_directions = strings(&report["architecture_comparisons"], "direction");
    for direction in facts["architecture_directions"].as_array().unwrap() {
        assert!(architecture_directions.contains(direction.as_str().unwrap()));
    }
    assert!(
        report["evolutionary_comparisons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|comparison| comparison["direction"] == facts["evolution_direction"])
    );
    let comparison_kinds = strings(&report["comparisons"], "kind");
    assert!(comparison_kinds.contains("added"));
    assert!(comparison_kinds.contains("removed"));
    let paths = report["paths"].as_array().unwrap();
    for expected_path in ["e/new.js", "f/deleted.js", "new/untracked.js"] {
        assert!(paths.iter().any(|path| path == expected_path));
    }
    assert_eq!(facts["changes"].as_array().unwrap().len(), 4);
    for private in [
        "First Fixture",
        "first@example.invalid",
        "Second Fixture",
        "second@example.invalid",
    ] {
        assert!(
            !terminal
                .stdout
                .windows(private.len())
                .any(|value| value == private.as_bytes())
        );
        assert!(
            !json
                .stdout
                .windows(private.len())
                .any(|value| value == private.as_bytes())
        );
    }
}

#[test]
fn committed_ref_diff_has_exact_terminal_and_json_with_a_clean_worktree() {
    let repository = ref_diff_repository();
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repository.path())
        .output()
        .unwrap();
    assert!(status.status.success());
    assert!(status.stdout.is_empty(), "fixture worktree must be clean");

    let terminal =
        Invocation::new(["diff", "main~1", "--all", "--history", "36500d"]).run(repository.path());
    terminal.success();
    let parallel_terminal = Invocation::new(["diff", "main~1", "--all", "--history", "36500d"])
        .automatic_workers()
        .run(repository.path());
    assert_eq!(terminal, parallel_terminal);
    assert_golden("unified-ref-diff.terminal.txt", &terminal.stdout);

    let json =
        Invocation::new(["diff", "main~1", "--json", "--history", "36500d"]).run(repository.path());
    json.success();
    let parallel_json = Invocation::new(["diff", "main~1", "--json", "--history", "36500d"])
        .automatic_workers()
        .run(repository.path());
    assert_eq!(json, parallel_json);
    let report = checked_json(&json.stdout);
    assert_eq!(report["mode"], "diff");
    assert!(!report["comparisons"].as_array().unwrap().is_empty());
    assert!(
        !report["architecture_comparisons"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        report["packages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|package| package["path"] == "gone" && package["presence"] == "base_only")
    );
    assert_golden("unified-ref-diff.json", &json.stdout);
}

#[test]
fn terminal_width_color_and_path_drills_have_exact_public_bytes() {
    let repository = worktree_change_repository();
    for width in [120, 80, 50] {
        let result = Invocation::new(["--all"])
            .columns(width)
            .run(repository.path());
        result.success();
        assert_golden(
            &format!("unified-codebase-{width}.terminal.txt"),
            &result.stdout,
        );
    }
    let plain = Invocation::new(["--all"]).run(repository.path());
    let colored = Invocation::new(["--all"])
        .color("always")
        .run(repository.path());
    plain.success();
    colored.success();
    assert_eq!(strip_ansi(&colored.stdout), plain.stdout);

    for (arguments, golden) in [
        (vec!["a", "--all"], "unified-package.terminal.txt"),
        (vec!["a/main.js", "--all"], "unified-file.terminal.txt"),
    ] {
        let result = Invocation::new(arguments.clone()).run(repository.path());
        result.success();
        let automatic = Invocation::new(arguments)
            .automatic_workers()
            .run(repository.path());
        assert_eq!(result, automatic);
        assert_golden(golden, &result.stdout);
    }
    for (arguments, golden) in [
        (vec!["a", "--json"], "unified-package.json"),
        (vec!["a/main.js", "--json"], "unified-file.json"),
    ] {
        let result = Invocation::new(arguments.clone()).run(repository.path());
        result.success();
        let automatic = Invocation::new(arguments)
            .automatic_workers()
            .run(repository.path());
        assert_eq!(result, automatic);
        checked_json(&result.stdout);
        assert_golden(golden, &result.stdout);
    }
    let languages = GeneratedRepository::new("main");
    copy_language_truth_files(&languages);
    let directory = Invocation::new(["src", "--all"]).run(languages.path());
    directory.success();
    let automatic_directory = Invocation::new(["src", "--all"])
        .automatic_workers()
        .run(languages.path());
    assert_eq!(directory, automatic_directory);
    assert_golden("unified-directory.terminal.txt", &directory.stdout);
    let directory_json = Invocation::new(["src", "--json"]).run(languages.path());
    directory_json.success();
    let automatic_directory_json = Invocation::new(["src", "--json"])
        .automatic_workers()
        .run(languages.path());
    assert_eq!(directory_json, automatic_directory_json);
    checked_json(&directory_json.stdout);
    assert_golden("unified-directory.json", &directory_json.stdout);
}

#[cfg(unix)]
#[test]
fn failures_and_recoverable_coverage_obey_status_and_stream_contracts() {
    let repository = coverage_failure_repository();
    let facts: Value = repository.facts("coverage-failures.json");
    let success = Invocation::new(["--json"]).run(repository.path());
    success.success();
    let automatic_success = Invocation::new(["--json"])
        .automatic_workers()
        .run(repository.path());
    assert_eq!(success, automatic_success);
    let report = checked_json(&success.stdout);
    assert_golden("unified-coverage-failures.json", &success.stdout);
    let root = report["root"].as_u64().unwrap() as usize;
    let coverage = &report["scopes"][root]["coverage"];
    for field in [
        "selected_files",
        "analyzed_files",
        "unsupported_files",
        "failed_files",
    ] {
        assert_eq!(coverage[field], facts[field], "{field}");
    }
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| { diagnostic["kind"] == facts["failed_diagnostic_kind"] })
    );

    let complete_history = evolution_repository();
    let shallow = shallow_clone(&complete_history);
    let incomplete = Invocation::new(["--json", "--history", "36500d"]).run(shallow.path());
    incomplete.success();
    let automatic_incomplete = Invocation::new(["--json", "--history", "36500d"])
        .automatic_workers()
        .run(shallow.path());
    assert_eq!(incomplete, automatic_incomplete);
    let incomplete_report = checked_json(&incomplete.stdout);
    assert_golden("unified-incomplete-history.json", &incomplete.stdout);
    assert_eq!(
        incomplete_report["history_coverage"]["availability"],
        facts["history_availability"]
    );
    assert_eq!(
        incomplete_report["history_coverage"]["reason"],
        facts["history_reason"]
    );
    assert_eq!(
        incomplete_report["history_coverage"]["commits"],
        facts["history_commits"]
    );

    let bad_ref = Invocation::new(["diff", "does-not-exist", "--json"]).run(repository.path());
    assert_eq!(
        bad_ref.status.code(),
        facts["expected_invalid_ref_exit"]
            .as_i64()
            .map(|v| v as i32)
    );
    assert!(bad_ref.stdout.is_empty());
    assert_eq!(bad_ref.stderr_text(), facts["expected_invalid_ref_stderr"]);

    let bad_argument = Invocation::new(["--jobs", "0"]).run(repository.path());
    assert_eq!(
        bad_argument.status.code(),
        facts["expected_invalid_argument_exit"]
            .as_i64()
            .map(|v| v as i32)
    );
    assert!(bad_argument.stdout.is_empty());
    assert_eq!(
        bad_argument.stderr_text(),
        facts["expected_invalid_argument_stderr"]
    );

    let outside_git = tempfile::tempdir().unwrap();
    fs::write(
        outside_git.path().join("main.js"),
        "export const value = 1;\n",
    )
    .unwrap();
    let codebase = Invocation::new(["--json"]).run(outside_git.path());
    assert_eq!(
        codebase.status.code(),
        facts["expected_outside_git_codebase_exit"]
            .as_i64()
            .map(|v| v as i32)
    );
    assert!(codebase.stderr.is_empty());
    let report = checked_json(&codebase.stdout);
    assert_eq!(report["history_coverage"]["availability"], "unavailable");
    let diff = Invocation::new(["diff", "main", "--json"]).run(outside_git.path());
    assert_eq!(
        diff.status.code(),
        facts["expected_outside_git_diff_exit"]
            .as_i64()
            .map(|v| v as i32)
    );
    assert!(diff.stdout.is_empty());
    assert_eq!(
        diff.stderr_text(),
        facts["expected_outside_git_diff_stderr"]
    );
}

#[test]
fn normal_tests_cannot_rewrite_golden_files_without_a_selected_case() {
    if std::env::var_os("SMACKDEBT_UPDATE_CASE").is_some() {
        assert_eq!(
            std::env::var_os("SMACKDEBT_UPDATE_COMMAND").as_deref(),
            Some(std::ffi::OsStr::new("1"))
        );
    }
}

#[test]
fn committed_results_do_not_contain_generated_contributor_identities() {
    let snapshots = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots");
    for entry in fs::read_dir(snapshots).unwrap() {
        let entry = entry.unwrap();
        let bytes = fs::read(entry.path()).unwrap();
        for private in [
            "Alice Example",
            "alice@example.invalid",
            "Alias Person",
            "alias@example.invalid",
            "Bob Example",
            "bob@example.invalid",
            "First Fixture",
            "first@example.invalid",
            "Second Fixture",
            "second@example.invalid",
            "Review Fixture",
            "review@example.invalid",
            "Coverage Fixture",
            "coverage@example.invalid",
        ] {
            assert!(
                !bytes
                    .windows(private.len())
                    .any(|value| value == private.as_bytes()),
                "{} contains {private}",
                entry.path().display()
            );
        }
    }
}

#[test]
fn help_and_version_use_the_success_stream_contract() {
    let repository = GeneratedRepository::new("main");
    for argument in ["--help", "--version"] {
        let result = Invocation::new([argument]).run(repository.path());
        result.success();
        assert!(!result.stdout.is_empty());
    }
}

#[test]
fn executable_readme_examples_match_named_public_fixtures() {
    let readme =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../README.md")).unwrap();
    let examples = readme_examples(&readme);
    assert_eq!(examples.len(), 2);
    for example in examples {
        let repository = match example.fixture.as_str() {
            "evolution" => evolution_repository(),
            "worktree-change" => worktree_change_repository(),
            other => panic!("unknown README fixture {other}"),
        };
        let arguments = example
            .command
            .split_whitespace()
            .skip(1)
            .collect::<Vec<_>>();
        let result = Invocation::new(arguments).run(repository.path());
        assert_eq!(result.status.code(), Some(example.status));
        assert_eq!(example.stderr, "empty");
        assert!(result.stderr.is_empty(), "{}", result.stderr_text());
        let output = String::from_utf8(result.stdout).unwrap();
        let mut remainder = output.as_str();
        for fragment in example.stdout_fragments {
            let visible = fragment.replace('_', " ");
            let position = remainder
                .find(&visible)
                .unwrap_or_else(|| panic!("missing ordered stdout fragment {visible}"));
            remainder = &remainder[position + visible.len()..];
        }
    }
}

#[cfg(feature = "evidence-stats")]
#[test]
fn composition_work_counts_are_visible_without_changing_report_bytes() {
    let repository = worktree_change_repository();
    for (name, arguments, expected) in [
        ("codebase terminal", vec!["--all"], [1, 29, 8, 0, 3, 8, 10]),
        ("codebase JSON", vec!["--json"], [1, 29, 8, 0, 3, 8, 10]),
        (
            "worktree diff terminal",
            vec!["diff", "main", "--all", "--history", "36500d"],
            [1, 29, 8, 24, 7, 15, 28],
        ),
        (
            "worktree diff JSON",
            vec!["diff", "main", "--json", "--history", "36500d"],
            [1, 29, 8, 24, 7, 15, 28],
        ),
        (
            "package terminal",
            vec!["a", "--all"],
            [1, 29, 8, 0, 3, 8, 10],
        ),
        ("package JSON", vec!["a", "--json"], [1, 29, 8, 0, 3, 8, 10]),
        (
            "file terminal",
            vec!["a/main.js", "--all"],
            [1, 29, 8, 0, 3, 8, 10],
        ),
        (
            "file JSON",
            vec!["a/main.js", "--json"],
            [1, 29, 8, 0, 3, 8, 10],
        ),
    ] {
        assert_evidence_flow(name, arguments, repository.path(), expected);
    }

    let reference = ref_diff_repository();
    for (name, arguments) in [
        (
            "clean ref diff terminal",
            vec!["diff", "main~1", "--all", "--history", "36500d"],
        ),
        (
            "clean ref diff JSON",
            vec!["diff", "main~1", "--json", "--history", "36500d"],
        ),
    ] {
        assert_evidence_flow(name, arguments, reference.path(), [1, 29, 8, 24, 7, 15, 28]);
    }

    let languages = GeneratedRepository::new("main");
    copy_language_truth_files(&languages);
    for (name, arguments) in [
        ("directory terminal", vec!["src", "--all"]),
        ("directory JSON", vec!["src", "--json"]),
    ] {
        assert_evidence_flow(name, arguments, languages.path(), [1, 15, 11, 0, 3, 11, 13]);
    }

    let roles = source_role_repository();
    assert_evidence_flow(
        "source roles JSON",
        vec!["--json"],
        roles.path(),
        [1, 14, 6, 0, 3, 6, 8],
    );
}

#[cfg(feature = "evidence-stats")]
fn assert_evidence_flow(name: &str, arguments: Vec<&str>, repository: &Path, expected: [usize; 7]) {
    let normal = Invocation::new(arguments.clone()).run(repository);
    normal.success();
    let automatic = Invocation::new(arguments.clone())
        .automatic_workers()
        .run(repository);
    assert_eq!(normal, automatic, "{name}: normal worker policies");

    let measured = Invocation::new(arguments.clone())
        .evidence()
        .run(repository);
    let automatic_measured = Invocation::new(arguments)
        .automatic_workers()
        .evidence()
        .run(repository);
    for result in [&measured, &automatic_measured] {
        assert_eq!(result.status.code(), Some(0), "{}", result.stderr_text());
        assert_eq!(result.stdout, normal.stdout, "{name}: report bytes");
        if result.stdout.starts_with(b"{") {
            checked_json(&result.stdout);
        }
    }
    let measured_stats = evidence_stats(&measured);
    let automatic_stats = evidence_stats(&automatic_measured);
    assert_eq!(measured_stats, automatic_stats, "{name}: work totals");
    for (field, expected) in [
        "inventory_walks",
        "inventory_visits",
        "source_reads",
        "object_reads",
        "git_processes",
        "parser_visits",
        "algorithm_passes",
    ]
    .into_iter()
    .zip(expected)
    {
        assert_eq!(measured_stats[field], expected, "{name}: {field}");
    }
    assert_eq!(measured_stats["renderer_entries"], 1, "{name}");
    assert_eq!(measured_stats["render_renderer_entries"], 1, "{name}");
    for field in [
        "render_inventory_walks",
        "render_inventory_visits",
        "render_source_reads",
        "render_object_reads",
        "render_git_processes",
        "render_parser_visits",
        "render_algorithm_passes",
    ] {
        assert_eq!(measured_stats[field], 0, "{name}: {field}");
    }
}

#[cfg(feature = "evidence-stats")]
fn evidence_stats(result: &support::ProcessResult) -> Value {
    let stderr = result.stderr_text();
    let mut lines = stderr.lines();
    let line = lines
        .next()
        .and_then(|line| line.strip_prefix("smackdebt evidence stats: "))
        .expect("one evidence stats line");
    assert!(
        lines.next().is_none(),
        "unexpected evidence stderr: {stderr}"
    );
    serde_json::from_str(line).unwrap()
}

#[test]
#[ignore = "release smoke installs locked crates and is run by the release evidence command"]
fn installed_command_runs_outside_the_workspace() {
    let repository = evolution_repository();
    let prefix = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new("cargo")
        .args([
            "install",
            "--locked",
            "--path",
            "crates/cli",
            "--root",
            prefix.path().to_str().unwrap(),
        ])
        .env("CARGO_TARGET_DIR", target.path())
        .current_dir(&workspace)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let command = prefix.path().join("bin/smackdebt");
    for arguments in [vec!["--help"], vec!["--version"]] {
        let output = Command::new(&command).args(arguments).output().unwrap();
        assert_eq!(output.status.code(), Some(0));
        assert!(!output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
    for arguments in [
        vec!["--color", "never", "--jobs", "1"],
        vec!["--json", "--jobs", "1"],
    ] {
        let output = Command::new(&command)
            .args(arguments)
            .current_dir(repository.path())
            .env("COLUMNS", "120")
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0));
        assert!(!output.stdout.is_empty());
        assert!(output.stderr.is_empty());
        if output.stdout.starts_with(b"{") {
            checked_json(&output.stdout);
        }
    }
}

fn checked_json(bytes: &[u8]) -> Value {
    let report: Value = serde_json::from_slice(bytes).unwrap();
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/report-v3.schema.json")).unwrap();
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(&report)
        .unwrap();
    assert_index_integrity(&report);
    assert_recursive_privacy(&report);
    report
}

fn assert_recursive_privacy(value: &Value) {
    match value {
        Value::Object(fields) => {
            for (name, value) in fields {
                assert!(
                    !PRIVATE_JSON_KEYS.contains(&name.as_str()),
                    "private field {name} entered JSON"
                );
                assert_recursive_privacy(value);
            }
        }
        Value::Array(values) => values.iter().for_each(assert_recursive_privacy),
        Value::String(value) => assert!(
            !std::path::Path::new(value).is_absolute(),
            "absolute path entered JSON: {value}"
        ),
        _ => {}
    }
}

#[test]
fn recursive_runtime_privacy_rejects_a_nested_identity_field() {
    let value = serde_json::json!({"outer": [{"contributor_email": "private@example.invalid"}]});
    assert!(std::panic::catch_unwind(|| assert_recursive_privacy(&value)).is_err());
}

fn assert_unified_facts(report: &Value) {
    assert!(!report["findings"].as_array().unwrap().is_empty());
    assert!(
        !report["architecture_findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(!report["change_coupling"].as_array().unwrap().is_empty());
    assert!(
        report["evolutionary_findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let empty = report["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|package| package["path"] == "f")
        .expect("empty current package");
    assert_eq!(empty["presence"], "current");
    let scope = empty["scope"].as_u64().unwrap() as usize;
    assert!(
        report["scopes"][scope]["children"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

fn assert_index_integrity(report: &Value) {
    let paths = report["paths"].as_array().unwrap().len();
    let scopes = report["scopes"].as_array().unwrap().len();
    let files = report["files"].as_array().unwrap().len();
    let packages = report["packages"].as_array().unwrap().len();
    let file_edges = report["dependency_edges"].as_array().unwrap().len();
    let findings = report["findings"].as_array().unwrap().len();
    let diagnostics = report["diagnostics"].as_array().unwrap().len();
    let comparisons = report["comparisons"].as_array().unwrap().len();
    let health = report["health"].as_array().unwrap().len();
    let activity = report["activity"].as_array().unwrap().len();
    let architecture_findings = report["architecture_findings"].as_array().unwrap().len();
    let architecture_comparisons = report["architecture_comparisons"].as_array().unwrap().len();
    let evolutionary_findings = report["evolutionary_findings"].as_array().unwrap().len();
    let evolutionary_comparisons = report["evolutionary_comparisons"].as_array().unwrap().len();
    for field in ["root", "selected_scope"] {
        if let Some(scope) = report[field].as_u64() {
            assert!((scope as usize) < scopes, "{field} points outside scopes");
        }
    }
    for table in [
        "packages",
        "scopes",
        "files",
        "findings",
        "diagnostics",
        "comparisons",
        "health",
        "activity",
        "dependency_edges",
        "package_edges",
        "architecture_findings",
        "architecture_comparisons",
        "evolutionary_findings",
        "evolutionary_comparisons",
    ] {
        for (id, row) in report[table].as_array().unwrap().iter().enumerate() {
            assert_eq!(row["id"].as_u64(), Some(id as u64), "{table} id {id}");
        }
    }
    for (id, package) in report["packages"].as_array().unwrap().iter().enumerate() {
        assert_eq!(package["id"].as_u64(), Some(id as u64));
        let scope = package["scope"].as_u64().unwrap() as usize;
        assert!(scope < scopes);
        let path = package["path"].as_str().unwrap();
        assert!(path == "." || !std::path::Path::new(path).is_absolute());
        let scope_row = &report["scopes"][scope];
        assert_eq!(scope_row["kind"], "package");
        let scope_path = scope_row["path"].as_u64().unwrap() as usize;
        assert_eq!(report["paths"][scope_path], path);
    }
    for scope in report["scopes"].as_array().unwrap() {
        if let Some(path) = scope["path"].as_u64() {
            assert!((path as usize) < paths);
        }
        if let Some(parent) = scope["parent"].as_u64() {
            assert!((parent as usize) < scopes);
        }
        for child in scope["children"].as_array().unwrap() {
            assert!((child.as_u64().unwrap() as usize) < scopes);
        }
        for (field, limit) in [
            ("findings", findings),
            ("comparisons", comparisons),
            ("architecture_findings", architecture_findings),
            ("architecture_comparisons", architecture_comparisons),
            ("evolutionary_findings", evolutionary_findings),
            ("evolutionary_comparisons", evolutionary_comparisons),
        ] {
            for id in scope[field].as_array().unwrap() {
                assert!((id.as_u64().unwrap() as usize) < limit, "scope {field}");
            }
        }
        assert!((scope["health"].as_u64().unwrap() as usize) < health);
    }
    for file in report["files"].as_array().unwrap() {
        assert!((file["path"].as_u64().unwrap() as usize) < paths);
        assert!((file["scope"].as_u64().unwrap() as usize) < scopes);
        let package = file["package"]
            .as_u64()
            .expect("file has one package owner");
        assert!(
            (package as usize) < packages,
            "file {} points to package {package}, but there are {packages} package rows",
            file["id"]
        );
        assert!((file["health"].as_u64().unwrap() as usize) < health);
        assert!((file["activity"].as_u64().unwrap() as usize) < activity);
        match file["parse_outcome"].as_str() {
            Some("parsed") => assert_eq!(file["trust"], "trusted"),
            Some("recovered") => assert_eq!(file["trust"], "advisory"),
            Some("failed") => assert_eq!(file["trust"], "failed"),
            None => {}
            Some(value) => panic!("unknown parse outcome {value}"),
        }
    }
    for finding in report["findings"].as_array().unwrap() {
        let file = finding["file"].as_u64().unwrap() as usize;
        assert!(file < files);
        assert_eq!(finding["role"], report["files"][file]["role"]);
        assert_eq!(finding["trust"], report["files"][file]["trust"]);
    }
    for diagnostic in report["diagnostics"].as_array().unwrap() {
        if let Some(file) = diagnostic["file"].as_u64() {
            assert!((file as usize) < files);
        }
    }
    assert_eq!(diagnostics, report["diagnostics"].as_array().unwrap().len());
    for comparison in report["comparisons"].as_array().unwrap() {
        if let Some(file) = comparison["file"].as_u64() {
            assert!((file as usize) < files);
        }
    }
    for row in report["activity"].as_array().unwrap() {
        assert!((row["file"].as_u64().unwrap() as usize) < files);
    }
    for edge in report["dependency_edges"].as_array().unwrap() {
        let source = edge["source"].as_u64().unwrap() as usize;
        assert!(source < files);
        assert!((edge["target"].as_u64().unwrap() as usize) < files);
        assert_eq!(edge["role"], report["files"][source]["role"]);
        assert_eq!(edge["trust"], report["files"][source]["trust"]);
    }
    for dependency in report["external_dependencies"].as_array().unwrap() {
        let file = dependency["file"].as_u64().unwrap() as usize;
        assert!(file < files);
        assert_eq!(dependency["role"], report["files"][file]["role"]);
        assert_eq!(dependency["trust"], report["files"][file]["trust"]);
    }
    for diagnostic in report["resolution_diagnostics"].as_array().unwrap() {
        let file = diagnostic["file"].as_u64().unwrap() as usize;
        assert!(file < files);
        assert_eq!(diagnostic["role"], report["files"][file]["role"]);
        assert_eq!(diagnostic["trust"], report["files"][file]["trust"]);
    }
    for finding in report["architecture_findings"].as_array().unwrap() {
        for package in finding["packages"].as_array().unwrap() {
            assert!((package.as_u64().unwrap() as usize) < packages);
        }
        for file in finding["files"].as_array().unwrap() {
            assert!((file.as_u64().unwrap() as usize) < files);
        }
        for edge in finding["witness_edges"].as_array().unwrap() {
            let edge = edge.as_u64().unwrap() as usize;
            assert!(edge < file_edges);
            assert_eq!(report["dependency_edges"][edge]["relation"], "uses");
            assert_eq!(report["dependency_edges"][edge]["trust"], "trusted");
        }
    }
    for edge in report["package_edges"].as_array().unwrap() {
        assert!((edge["source"].as_u64().unwrap() as usize) < packages);
        assert!((edge["target"].as_u64().unwrap() as usize) < packages);
        for file_edge in edge["file_edges"].as_array().unwrap() {
            let file_edge = file_edge.as_u64().unwrap() as usize;
            assert!(file_edge < file_edges);
            assert_eq!(report["dependency_edges"][file_edge]["relation"], "uses");
            assert_eq!(report["dependency_edges"][file_edge]["trust"], "trusted");
        }
    }
    for comparison in report["architecture_comparisons"].as_array().unwrap() {
        for package in comparison["packages"].as_array().unwrap() {
            assert!((package.as_u64().unwrap() as usize) < packages);
        }
        for package in comparison["witness"].as_array().unwrap() {
            assert!((package.as_u64().unwrap() as usize) < packages);
        }
        for file in comparison["files"].as_array().unwrap() {
            assert!((file.as_u64().unwrap() as usize) < files);
        }
    }
    for row in report["package_graph"].as_array().unwrap() {
        assert!((row["package"].as_u64().unwrap() as usize) < packages);
    }
    for row in report["package_history"].as_array().unwrap() {
        assert!((row["package"].as_u64().unwrap() as usize) < packages);
    }
    for row in report["file_history"].as_array().unwrap() {
        let file = row["file"].as_u64().unwrap() as usize;
        assert!(file < files);
        assert_eq!(row["role"], report["files"][file]["role"]);
        assert_eq!(row["trust"], report["files"][file]["trust"]);
    }
    for row in report["contributor_concentration"].as_array().unwrap() {
        assert!((row["package"].as_u64().unwrap() as usize) < packages);
    }
    for coupling in report["change_coupling"].as_array().unwrap() {
        let left = coupling["left"].as_u64().unwrap();
        let right = coupling["right"].as_u64().unwrap();
        assert!((left as usize) < packages);
        assert!((right as usize) < packages);
        assert!(left < right);
        assert!(
            coupling["shared_commits"].as_u64().unwrap()
                <= coupling["union_commits"].as_u64().unwrap()
        );
    }
    for finding in report["evolutionary_findings"].as_array().unwrap() {
        let left = finding["left"].as_u64().unwrap();
        let right = finding["right"].as_u64().unwrap();
        assert!((left as usize) < packages);
        assert!((right as usize) < packages);
        assert!(left < right);
        assert!(finding["shared_commits"].as_u64().unwrap() >= 3);
        assert!(finding["similarity"].as_f64().unwrap() >= 0.2);
        assert!(
            finding["shared_commits"].as_u64().unwrap()
                <= finding["union_commits"].as_u64().unwrap()
        );
    }
    for comparison in report["evolutionary_comparisons"].as_array().unwrap() {
        let left = comparison["left"].as_u64().unwrap();
        let right = comparison["right"].as_u64().unwrap();
        assert!((left as usize) < packages);
        assert!((right as usize) < packages);
        assert!(left < right);
        assert!(comparison["shared_commits"].as_u64().unwrap() >= 3);
        assert!(comparison["similarity"].as_f64().unwrap() >= 0.2);
        assert!(
            comparison["shared_commits"].as_u64().unwrap()
                <= comparison["union_commits"].as_u64().unwrap()
        );
    }
}

fn assert_private_values_absent(facts: &Value, bytes: &[u8]) {
    for private in facts["private_values"].as_array().unwrap() {
        let private = private.as_str().unwrap().as_bytes();
        assert!(!bytes.windows(private.len()).any(|window| window == private));
    }
}

fn strings<'a>(values: &'a Value, field: &str) -> HashSet<&'a str> {
    values
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|value| value[field].as_str())
        .collect()
}

fn assert_golden(name: &str, actual: &[u8]) {
    let path = golden_path(name);
    let selected = std::env::var("SMACKDEBT_UPDATE_CASE").ok();
    if selected.as_deref() == Some(name) || selected.as_deref() == Some("all") {
        assert_eq!(
            std::env::var_os("SMACKDEBT_UPDATE_COMMAND").as_deref(),
            Some(std::ffi::OsStr::new("1")),
            "golden updates require the named developer command"
        );
        fs::write(&path, actual).unwrap();
        eprintln!("updated {}", path.display());
        return;
    }
    let expected =
        fs::read(&path).unwrap_or_else(|error| panic!("read golden {}: {error}", path.display()));
    assert_eq!(actual, expected, "golden {name} changed");
}

fn golden_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(name)
}

fn strip_ansi(value: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(value.len());
    let mut index = 0;
    while index < value.len() {
        if value[index..].starts_with(b"\x1b[") {
            index += 2;
            while index < value.len() && !(b'@'..=b'~').contains(&value[index]) {
                index += 1;
            }
            index += usize::from(index < value.len());
        } else {
            result.push(value[index]);
            index += 1;
        }
    }
    result
}

#[derive(Debug)]
struct ReadmeExample {
    fixture: String,
    status: i32,
    stderr: String,
    stdout_fragments: Vec<String>,
    command: String,
}

fn readme_examples(readme: &str) -> Vec<ReadmeExample> {
    let mut examples = Vec::new();
    let mut lines = readme.lines();
    while let Some(line) = lines.next() {
        let Some(metadata) = line
            .strip_prefix("<!-- smackdebt-example ")
            .and_then(|line| line.strip_suffix(" -->"))
        else {
            continue;
        };
        let mut fixture = None;
        let mut status = None;
        let mut stderr = None;
        let mut stdout_fragments = None;
        for field in metadata.split_whitespace() {
            let (name, value) = field.split_once('=').unwrap();
            match name {
                "fixture" => fixture = Some(value.to_owned()),
                "status" => status = Some(value.parse().unwrap()),
                "stderr" => stderr = Some(value.to_owned()),
                "stdout" => {
                    stdout_fragments = Some(value.split('|').map(str::to_owned).collect());
                }
                _ => panic!("unknown README example field {name}"),
            }
        }
        assert_eq!(lines.next(), Some("```console"));
        let command = lines.next().expect("README example command").to_owned();
        assert_eq!(lines.next(), Some("```"));
        examples.push(ReadmeExample {
            fixture: fixture.expect("fixture"),
            status: status.expect("status"),
            stderr: stderr.expect("stderr"),
            stdout_fragments: stdout_fragments.expect("stdout"),
            command,
        });
    }
    examples
}
