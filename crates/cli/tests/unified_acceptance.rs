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
    ref_diff_repository, shallow_clone, static_architecture_repository, worktree_change_repository,
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
        let result = Invocation::new(arguments).run(repository.path());
        result.success();
        assert_golden(golden, &result.stdout);
    }
    let languages = GeneratedRepository::new("main");
    copy_language_truth_files(&languages);
    let directory = Invocation::new(["src", "--all"]).run(languages.path());
    directory.success();
    assert_golden("unified-directory.terminal.txt", &directory.stdout);
}

#[cfg(unix)]
#[test]
fn failures_and_recoverable_coverage_obey_status_and_stream_contracts() {
    let repository = coverage_failure_repository();
    let facts: Value = repository.facts("coverage-failures.json");
    let success = Invocation::new(["--json"]).run(repository.path());
    success.success();
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
        ("codebase terminal", vec!["--all"], [1, 28, 8, 0, 3, 8, 10]),
        ("codebase JSON", vec!["--json"], [1, 28, 8, 0, 3, 8, 10]),
        (
            "worktree diff terminal",
            vec!["diff", "main", "--all", "--history", "36500d"],
            [1, 28, 8, 22, 7, 15, 28],
        ),
        (
            "worktree diff JSON",
            vec!["diff", "main", "--json", "--history", "36500d"],
            [1, 28, 8, 22, 7, 15, 28],
        ),
    ] {
        let normal = Invocation::new(arguments.clone()).run(repository.path());
        normal.success();
        let measured = Invocation::new(arguments).evidence().run(repository.path());
        assert_eq!(
            measured.status.code(),
            Some(0),
            "{}",
            measured.stderr_text()
        );
        assert_eq!(measured.stdout, normal.stdout);
        let stderr = measured.stderr_text();
        let line = stderr
            .lines()
            .find_map(|line| line.strip_prefix("smackdebt evidence stats: "))
            .expect("evidence stats line");
        let stats: Value = serde_json::from_str(line).unwrap();
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
            assert_eq!(stats[field], expected, "{name}: {field}");
        }
        assert_eq!(stats["renderer_entries"], 1);
        assert_eq!(stats["render_renderer_entries"], 1);
        for field in [
            "render_inventory_walks",
            "render_inventory_visits",
            "render_source_reads",
            "render_object_reads",
            "render_git_processes",
            "render_parser_visits",
            "render_algorithm_passes",
        ] {
            assert_eq!(stats[field], 0, "{field}");
        }
        if measured.stdout.starts_with(b"{") {
            checked_json(&measured.stdout);
        }
    }
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
        serde_json::from_str(include_str!("../../../schemas/report-v2.schema.json")).unwrap();
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(&report)
        .unwrap();
    assert_index_integrity(&report);
    report
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
}

fn assert_index_integrity(report: &Value) {
    let paths = report["paths"].as_array().unwrap().len();
    let scopes = report["scopes"].as_array().unwrap().len();
    let files = report["files"].as_array().unwrap().len();
    let packages = report["package_graph"].as_array().unwrap().len();
    let file_edges = report["dependency_edges"].as_array().unwrap().len();
    for scope in report["scopes"].as_array().unwrap() {
        assert!((scope["path"].as_u64().unwrap() as usize) < paths);
        for child in scope["children"].as_array().unwrap() {
            assert!((child.as_u64().unwrap() as usize) < scopes);
        }
    }
    for file in report["files"].as_array().unwrap() {
        assert!((file["path"].as_u64().unwrap() as usize) < paths);
        assert!((file["scope"].as_u64().unwrap() as usize) < scopes);
        if let Some(package) = file["package"].as_u64() {
            assert!(
                (package as usize) < packages,
                "file {} points to package {package}, but there are {packages} package rows",
                file["id"]
            );
        }
    }
    for edge in report["dependency_edges"].as_array().unwrap() {
        assert!((edge["source"].as_u64().unwrap() as usize) < files);
        assert!((edge["target"].as_u64().unwrap() as usize) < files);
    }
    for finding in report["architecture_findings"].as_array().unwrap() {
        for edge in finding["witness_edges"].as_array().unwrap() {
            assert!((edge.as_u64().unwrap() as usize) < file_edges);
        }
    }
    for coupling in report["change_coupling"].as_array().unwrap() {
        assert!((coupling["left"].as_u64().unwrap() as usize) < packages);
        assert!((coupling["right"].as_u64().unwrap() as usize) < packages);
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
