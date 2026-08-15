mod support;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use serde_json::Value;
use unicode_width::UnicodeWidthStr;

#[cfg(unix)]
use support::coverage_failure_repository;
use support::{
    GeneratedRepository, Invocation, copy_language_truth_files, evolution_repository,
    ref_diff_repository, shallow_clone, source_role_repository, static_architecture_repository,
    workspace_manifest_repository, worktree_change_repository,
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
fn the_selected_history_window_bounds_churn_coupling_and_concentration() {
    let repository = evolution_repository();
    let windowed = Invocation::new(["--json"]).run(repository.path());
    windowed.success();
    let windowed = checked_json(&windowed.stdout);
    let complete = Invocation::new(["--json", "--history", "36500d"]).run(repository.path());
    complete.success();
    let complete = checked_json(&complete.stdout);

    // The generated repository commits are older than the default window, so a
    // default run streams the same commits and derives no history facts.
    assert_eq!(
        windowed["history_coverage"]["commits"],
        complete["history_coverage"]["commits"]
    );
    assert!(complete["history_coverage"]["mapped_eligible_changes"].as_u64() > Some(0));
    assert_eq!(windowed["history_coverage"]["mapped_eligible_changes"], 0);
    assert!(!complete["change_coupling"].as_array().unwrap().is_empty());
    assert!(windowed["change_coupling"].as_array().unwrap().is_empty());
    assert!(
        !complete["contributor_concentration"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        windowed["contributor_concentration"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let touches = |report: &Value| {
        report["file_history"]
            .as_array()
            .unwrap()
            .iter()
            .map(|history| history["touches"].as_u64().unwrap())
            .sum::<u64>()
    };
    assert!(touches(&complete) > 0);
    assert_eq!(touches(&windowed), 0);
    let activity = |report: &Value| {
        report["activity"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|entry| !entry["touches"].is_null())
            .count()
    };
    assert!(activity(&complete) > 0);
    assert_eq!(activity(&windowed), 0);
}

#[test]
fn unified_codebase_terminal_and_json_are_exact_and_deterministic() {
    let repository = worktree_change_repository();
    let concise = Invocation::new(["--history", "36500d"]).run(repository.path());
    concise.success();
    assert_golden("unified-codebase-concise.terminal.txt", &concise.stdout);
    let serial_terminal = Invocation::new(["--all", "--history", "36500d"]).run(repository.path());
    serial_terminal.success();
    let parallel_terminal = Invocation::new(["--all", "--history", "36500d"])
        .automatic_workers()
        .run(repository.path());
    parallel_terminal.success();
    assert_eq!(serial_terminal, parallel_terminal);
    assert_golden("unified-codebase-120.terminal.txt", &serial_terminal.stdout);

    let serial_json = Invocation::new(["--json", "--history", "36500d"]).run(repository.path());
    serial_json.success();
    let parallel_json = Invocation::new(["--json", "--history", "36500d"])
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
    let concise = Invocation::new(["diff", "main", "--history", "36500d"]).run(repository.path());
    concise.success();
    assert_golden(
        "unified-worktree-diff-concise.terminal.txt",
        &concise.stdout,
    );
    let terminal =
        Invocation::new(["diff", "main", "--all", "--history", "36500d"]).run(repository.path());
    terminal.success();
    let parallel_terminal = Invocation::new(["diff", "main", "--all", "--history", "36500d"])
        .automatic_workers()
        .run(repository.path());
    parallel_terminal.success();
    assert_eq!(terminal, parallel_terminal);
    assert_golden("unified-worktree-diff.terminal.txt", &terminal.stdout);
    for width in [80, 50] {
        let result = Invocation::new(["diff", "main", "--all", "--history", "36500d"])
            .columns(width)
            .run(repository.path());
        result.success();
        assert_golden(
            &format!("unified-worktree-diff-{width}.terminal.txt"),
            &result.stdout,
        );
    }
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
        let result = Invocation::new(["--all", "--history", "36500d"])
            .columns(width)
            .run(repository.path());
        result.success();
        assert_golden(
            &format!("unified-codebase-{width}.terminal.txt"),
            &result.stdout,
        );
    }
    let narrow_plain = Invocation::new(["--all", "--history", "36500d"])
        .columns(50)
        .run(repository.path());
    let narrow_colored = Invocation::new(["--all", "--history", "36500d"])
        .columns(50)
        .color("always")
        .run(repository.path());
    narrow_plain.success();
    narrow_colored.success();
    assert_eq!(strip_ansi(&narrow_colored.stdout), narrow_plain.stdout);
    assert_glyph_only_ansi(&narrow_colored.stdout);
    assert_max_display_width(&narrow_colored.stdout, 50, "colored codebase");
    let plain = Invocation::new(["--all", "--history", "36500d"]).run(repository.path());
    let colored = Invocation::new(["--all", "--history", "36500d"])
        .color("always")
        .run(repository.path());
    plain.success();
    colored.success();
    assert_eq!(strip_ansi(&colored.stdout), plain.stdout);
    assert_glyph_only_ansi(&colored.stdout);
    let colored_diff = Invocation::new(["diff", "main", "--all", "--history", "36500d"])
        .color("always")
        .run(repository.path());
    colored_diff.success();
    assert_glyph_only_ansi(&colored_diff.stdout);
    let colored_text = String::from_utf8_lossy(&colored.stdout);
    let colored_diff_text = String::from_utf8_lossy(&colored_diff.stdout);
    for glyph in ['', '', ''] {
        assert!(colored_text.contains(glyph));
    }
    for glyph in ['', '', ''] {
        assert!(colored_diff_text.contains(glyph));
    }
    assert!(!colored_diff_text.contains("\u{1b}[31m"));
    assert!(!colored_diff_text.contains("\u{1b}[32m"));
    assert!(!colored_diff_text.contains("\u{1b}[38;5;208m"));
    let redirected = Invocation::new(["--all", "--history", "36500d"])
        .automatic_color()
        .without_no_color()
        .run(repository.path());
    let no_color = Invocation::new(["--all", "--history", "36500d"])
        .automatic_color()
        .run(repository.path());
    redirected.success();
    no_color.success();
    assert_eq!(redirected.stdout, plain.stdout);
    assert_eq!(no_color.stdout, plain.stdout);
    assert!(!redirected.stdout.windows(2).any(|bytes| bytes == b"\x1b["));
    assert!(!no_color.stdout.windows(2).any(|bytes| bytes == b"\x1b["));

    for (arguments, stem, wide_golden) in [
        (
            vec!["a", "--all", "--history", "36500d"],
            "unified-package",
            "unified-package.terminal.txt",
        ),
        (
            vec!["a/main.js", "--all", "--history", "36500d"],
            "unified-file",
            "unified-file.terminal.txt",
        ),
    ] {
        for width in [120, 80, 50] {
            let result = Invocation::new(arguments.clone())
                .columns(width)
                .run(repository.path());
            result.success();
            let automatic = Invocation::new(arguments.clone())
                .columns(width)
                .automatic_workers()
                .run(repository.path());
            assert_eq!(result, automatic);
            let golden = if width == 120 {
                wide_golden.to_owned()
            } else {
                format!("{stem}-{width}.terminal.txt")
            };
            assert_golden(&golden, &result.stdout);
        }
    }
    for (arguments, golden) in [
        (
            vec!["a", "--json", "--history", "36500d"],
            "unified-package.json",
        ),
        (
            vec!["a/main.js", "--json", "--history", "36500d"],
            "unified-file.json",
        ),
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
    for width in [120, 80, 50] {
        let directory = Invocation::new(["src", "--all"])
            .columns(width)
            .run(languages.path());
        directory.success();
        let automatic_directory = Invocation::new(["src", "--all"])
            .columns(width)
            .automatic_workers()
            .run(languages.path());
        assert_eq!(directory, automatic_directory);
        let golden = if width == 120 {
            "unified-directory.terminal.txt".to_owned()
        } else {
            format!("unified-directory-{width}.terminal.txt")
        };
        assert_golden(&golden, &directory.stdout);
    }
    let directory_json = Invocation::new(["src", "--json"]).run(languages.path());
    directory_json.success();
    let automatic_directory_json = Invocation::new(["src", "--json"])
        .automatic_workers()
        .run(languages.path());
    assert_eq!(directory_json, automatic_directory_json);
    checked_json(&directory_json.stdout);
    assert_golden("unified-directory.json", &directory_json.stdout);
}

#[test]
fn every_fifty_column_snapshot_respects_unicode_display_width() {
    if std::env::var_os("SMACKDEBT_UPDATE_CASE").is_some() {
        return;
    }
    let snapshots = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots");
    let mut checked = 0;
    for entry in fs::read_dir(snapshots).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.ends_with("-50.terminal.txt") {
            continue;
        }
        assert_max_display_width(&fs::read(entry.path()).unwrap(), 50, &name);
        checked += 1;
    }
    assert_eq!(checked, 6, "expected every reviewed 50-column view");
}

fn assert_max_display_width(bytes: &[u8], width: usize, name: &str) {
    let plain = strip_ansi(bytes);
    let plain = String::from_utf8(plain).unwrap();
    for (index, line) in plain.lines().enumerate() {
        assert!(
            UnicodeWidthStr::width(line) <= width,
            "{name}:{} is {} columns: {line}",
            index + 1,
            UnicodeWidthStr::width(line)
        );
    }
}

#[cfg(unix)]
#[test]
fn failures_and_recoverable_coverage_obey_status_and_stream_contracts() {
    let repository = coverage_failure_repository();
    let facts: Value = repository.facts("coverage-failures.json");
    let success = Invocation::new(["--json", "--history", "36500d"]).run(repository.path());
    success.success();
    let automatic_success = Invocation::new(["--json", "--history", "36500d"])
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
    assert_short_terminal_text(&bad_ref.stderr);

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
    assert_short_terminal_text(&bad_argument.stderr);

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
    assert_short_terminal_text(&diff.stderr);
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
        assert_short_terminal_text(&result.stdout);
        if argument == "--help" {
            assert_eq!(
                String::from_utf8_lossy(&result.stdout),
                concat!(
                    "Find costly code and see whether a change made it better\n",
                    "\n",
                    "Usage: smackdebt [OPTIONS] [PATH] [COMMAND]\n",
                    "\n",
                    "Commands:\n",
                    "  diff  Compare your current work with a Git ref\n",
                    "  help  Print this message or the help of the given subcommand(s)\n",
                    "\n",
                    "Arguments:\n",
                    "  [PATH]  Show one path\n",
                    "\n",
                    "Options:\n",
                    "      --json               Write the complete JSON report\n",
                    "      --jobs <JOBS>        Number of workers to use\n",
                    "      --history <HISTORY>  Recent activity window, such as 90d\n",
                    "      --all                Show all useful terminal detail\n",
                    "      --color <COLOR>      Glyph color: auto, always, or never [possible values: auto, always, never]\n",
                    "  -h, --help               Print help\n",
                    "  -V, --version            Print version\n",
                )
            );
        }
    }
}

#[test]
fn readme_console_examples_use_the_simple_terminal_vocabulary() {
    let readme =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../README.md")).unwrap();
    let mut console = String::new();
    let mut inside = false;
    for line in readme.lines() {
        if line == "```console" {
            inside = true;
            continue;
        }
        if inside && line == "```" {
            assert_short_terminal_text(console.as_bytes());
            console.clear();
            inside = false;
            continue;
        }
        if inside {
            console.push_str(line);
            console.push('\n');
        }
    }
    assert!(!inside, "README console block is not closed");
    for required in [
        "QUALITY",
        "AREAS",
        "FINDINGS",
        "ARCHITECTURE",
        "changed together in 8 of 10 commits · 80% · no code dependency",
        "source → target · 1 import",
        "source owns target",
        "label activity as `commit` or `commits`",
        " smackdebt",
    ] {
        assert!(readme.contains(required), "README is missing {required}");
    }
    for removed in [
        "change together without a dependency",
        "touches count distinct commits",
        "external uses · primary/trusted",
        "unresolved uses · primary/trusted",
        "coupling finding",
    ] {
        assert!(!readme.contains(removed), "README contains {removed}");
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
        assert_short_terminal_text(&result.stdout);
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
fn selected_binary_contains_the_requested_evidence_feature() {
    let repository = GeneratedRepository::new("main");
    repository.write("main.js", b"export function small() { return 1; }\n");
    let result = Invocation::new(["--json"])
        .evidence()
        .run(repository.path());
    assert_eq!(result.status.code(), Some(0), "{}", result.stderr_text());
    checked_json(&result.stdout);
    let stats = evidence_stats(&result);
    assert_eq!(stats["renderer_entries"], 1);
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

#[test]
fn declared_manifest_names_make_the_workspace_graph_and_coupling_true() {
    let repository = workspace_manifest_repository();
    let result = Invocation::new(["--json", "--history", "36500d"]).run(repository.path());
    result.success();
    let automatic = Invocation::new(["--json", "--history", "36500d"])
        .automatic_workers()
        .run(repository.path());
    assert_eq!(result, automatic, "serial and parallel runs must agree");
    let report = checked_json(&result.stdout);

    let package = |path: &str| {
        report["packages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|value| value["path"] == path)
            .unwrap_or_else(|| panic!("missing package {path}"))["id"]
            .as_u64()
            .unwrap()
    };
    let edges: HashSet<(u64, u64)> = report["package_edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| {
            (
                edge["source"].as_u64().unwrap(),
                edge["target"].as_u64().unwrap(),
            )
        })
        .collect();
    for (source, target) in [
        ("crates/core", "crates/renamed"),
        ("crates/renamed", "crates/core"),
        ("web", "ui"),
        ("pyapp", "py"),
        ("rbapp", "rb"),
    ] {
        assert!(
            edges.contains(&(package(source), package(target))),
            "{source} -> {target} missing from {edges:?}"
        );
    }
    let core = report["package_graph"]
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["package"].as_u64() == Some(package("crates/core")))
        .unwrap();
    assert_eq!(core["fan_in"], 1);
    assert_eq!(core["fan_out"], 1);
    let cycle = report["architecture_findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| finding["kind"] == "package_cycle")
        .expect("manifest names reveal the package cycle");
    let cycle_packages: HashSet<u64> = cycle["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_u64().unwrap())
        .collect();
    assert_eq!(
        cycle_packages,
        HashSet::from([package("crates/core"), package("crates/renamed")])
    );

    let external = strings(&report["external_dependencies"], "target");
    for absent in [
        "pub",
        "acme_core",
        "acme_core::core",
        "renamed_lib",
        "renamed_lib::renamed",
        "@acme/ui/button",
        "acme_py",
        "acme_rb",
    ] {
        assert!(!external.contains(absent), "{absent} is not external");
    }
    let ambiguous: Vec<_> = report["resolution_diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|value| value["kind"] == "ambiguous")
        .map(|value| value["target"].as_str().unwrap())
        .collect();
    assert_eq!(ambiguous, ["acme_dup::thing"]);
    assert!(
        report["resolution_diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|value| value["target"] != "pub")
    );

    let mut pairs = Vec::new();
    for row in report["change_coupling"].as_array().unwrap() {
        let left = row["left"].as_u64().unwrap();
        let right = row["right"].as_u64().unwrap();
        assert_ne!(
            left,
            package("."),
            "a scope cannot couple with its own tree"
        );
        assert_ne!(right, package("."));
        pairs.push((left.min(right), left.max(right)));
    }
    let unique: HashSet<_> = pairs.iter().copied().collect();
    assert_eq!(pairs.len(), unique.len(), "one row per package pair");
    assert!(unique.contains(&(
        package("crates/core").min(package("crates/renamed")),
        package("crates/core").max(package("crates/renamed"))
    )));

    let terminal = Invocation::new(["--all", "--history", "36500d"]).run(repository.path());
    terminal.success();
    let text = String::from_utf8(terminal.stdout).unwrap();
    let coupling_lines: Vec<_> = text.lines().filter(|line| line.contains(" ↔ ")).collect();
    assert_eq!(
        coupling_lines.len(),
        unique.len(),
        "{coupling_lines:?} must render each retained pair once"
    );
    assert!(
        coupling_lines
            .iter()
            .any(|line| line.contains("crates/core ↔ crates/renamed")
                && line.contains("code dependency exists")),
        "{coupling_lines:?}"
    );
    assert!(
        coupling_lines
            .iter()
            .any(|line| line.contains("rb ↔ ui") && line.contains("no code dependency")),
        "{coupling_lines:?}"
    );
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
    if name.ends_with(".terminal.txt") {
        assert_short_terminal_text(actual);
    }
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

fn assert_short_terminal_text(bytes: &[u8]) {
    let text = String::from_utf8_lossy(bytes);
    for removed in [
        "▲",
        "▼",
        "●",
        "→ Explore",
        " HIGH",
        " WATCH",
        " WORSE",
        " BETTER",
        " CHANGED",
        "healthy",
        "complete local stream",
        "eligible mapping",
        "retained units",
        "retained package pairs",
        " touches",
        " touch ",
        "primary/trusted",
        " · uses · ",
        " references",
        "  witness",
        "unresolved uses",
        "ambiguous uses",
        "coupling finding",
        "DEBT BY AREA",
        "CHANGE BY AREA",
        "TOP FINDINGS",
        "TOP CHANGES",
        "\u{ec3f}",
    ] {
        assert!(
            !text.contains(removed),
            "terminal contains removed text {removed}"
        );
    }
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

fn assert_glyph_only_ansi(value: &[u8]) {
    let text = std::str::from_utf8(value).unwrap();
    let mut remainder = text;
    while let Some(start) = remainder.find("\x1b[") {
        remainder = &remainder[start..];
        let style_end = remainder.find('m').expect("ANSI style terminator") + 1;
        let style = &remainder[..style_end];
        remainder = &remainder[style_end..];
        let glyph = remainder.chars().next().expect("styled glyph");
        assert!(
            [
                '\u{f024}', '\u{f0eb}', '\u{f46b}', '\u{f062}', '\u{f063}', '\u{f071}'
            ]
            .contains(&glyph),
            "styled non-status text {glyph:?}"
        );
        let expected_style = match glyph {
            '' | '' => "\u{1b}[31m",
            '' | '' => "\u{1b}[38;5;208m",
            '' => "\u{1b}[36m",
            '' => "\u{1b}[32m",
            _ => unreachable!(),
        };
        assert_eq!(style, expected_style, "wrong style for {glyph:?}");
        remainder = &remainder[glyph.len_utf8()..];
        assert!(
            remainder.starts_with("\x1b["),
            "glyph style was not reset immediately"
        );
        let reset_end = remainder.find('m').expect("ANSI reset terminator") + 1;
        remainder = &remainder[reset_end..];
    }
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
