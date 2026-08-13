use std::fs;
use std::path::Path;
use std::process::Command;

use assert_cmd::cargo::cargo_bin_cmd;

const SOURCE_ENGINE_FIXTURE: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/source-engine");

#[test]
fn serial_and_parallel_codebase_output_match() {
    let project = fixture();
    let serial = run(["--jobs", "1", project.path().to_str().unwrap()]);
    let parallel = run(["--jobs", "4", project.path().to_str().unwrap()]);
    assert_eq!(serial, parallel);
    let text = String::from_utf8(serial).unwrap();
    assert!(text.contains("QUALITY"));
    assert!(text.contains("No child areas need attention"));
}

#[test]
fn serial_and_parallel_json_match_and_follow_schema_two() {
    let project = fixture();
    let path = project.path().to_str().unwrap();
    let serial = run(["--json", "--jobs", "1", path]);
    let parallel = run(["--json", "--jobs", "4", path]);
    assert_eq!(serial, parallel);
    let report: serde_json::Value = serde_json::from_slice(&serial).unwrap();
    assert_eq!(report["schema_version"], 2);
    assert_eq!(report["mode"], "codebase");
    assert!(report["findings"].is_array());
    validate_schema(&report);
    assert_index_integrity(&report);
}

#[test]
fn mixed_language_parallel_cutover_preserves_terminal_and_json_bytes() {
    let project = mixed_fixture(100);
    let path = project.path().to_str().unwrap();
    assert_eq!(
        run(["--jobs", "1", "--color", "never", path]),
        run(["--jobs", "4", "--color", "never", path])
    );
    assert_eq!(
        run(["--json", "--jobs", "1", path]),
        run(["--json", "--jobs", "4", path])
    );
}

#[test]
fn diff_reports_metric_changes_from_the_worktree() {
    let project = fixture();
    git(project.path(), ["init", "-b", "main"]);
    git(project.path(), ["config", "user.name", "Smackdebt Test"]);
    git(
        project.path(),
        ["config", "user.email", "smackdebt@example.invalid"],
    );
    git(project.path(), ["add", "."]);
    git(project.path(), ["commit", "-m", "test: initial source"]);
    fs::write(
        project.path().join("src/work.rb"),
        "def work(items)\n  items.each do |item|\n    if item.ready?\n      if item.large?\n        ship(item)\n      end\n    end\n  end\nend\n",
    )
    .unwrap();

    let output = run([
        "diff",
        "main",
        project.path().to_str().unwrap(),
        "--json",
        "--jobs",
        "1",
    ]);
    let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(report["mode"], "diff");
    assert!(!report["comparisons"].as_array().unwrap().is_empty());
    validate_schema(&report);
    assert_index_integrity(&report);
    assert!(!report["comparisons"].as_array().unwrap().is_empty());
}

#[test]
fn invalid_project_config_uses_argument_exit_code() {
    let project = fixture();
    fs::write(
        project.path().join(".smackdebt.toml"),
        "[thresholds.cognitive]\nwatch = 25\nhigh = 10\n",
    )
    .unwrap();
    cargo_bin_cmd!("smackdebt")
        .arg(project.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains("high greater than watch"));
}

#[test]
fn terminal_color_can_be_forced_or_disabled_through_a_pipe() {
    let project = fixture();
    let path = project.path().to_str().unwrap();
    let colored = run(["--color", "always", path]);
    assert!(colored.windows(2).any(|bytes| bytes == b"\x1b["));

    let plain = run(["--color", "never", path]);
    assert!(!plain.windows(2).any(|bytes| bytes == b"\x1b["));
    assert_eq!(strip_ansi(&colored), plain);
}

#[test]
fn explicit_color_is_rejected_for_json() {
    let project = fixture();
    cargo_bin_cmd!("smackdebt")
        .args(["--json", "--color", "always"])
        .arg(project.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains("cannot be used with '--color"));
}

#[test]
fn source_engine_terminal_snapshot_is_reviewed() {
    let project = source_engine_fixture();
    let actual = run_in(project.path(), ["--jobs", "1", "--color", "never"]);
    assert_snapshot(
        "source-engine.terminal.txt",
        &actual,
        include_bytes!("snapshots/source-engine.terminal.txt"),
    );
}

#[test]
fn source_engine_json_snapshot_is_reviewed_and_matches_schema() {
    let project = source_engine_fixture();
    let actual = run_in(project.path(), ["--json", "--jobs", "1"]);
    assert_snapshot(
        "source-engine.json",
        &actual,
        include_bytes!("snapshots/source-engine.json"),
    );
    let report: serde_json::Value = serde_json::from_slice(&actual).unwrap();
    validate_schema(&report);
}

fn assert_snapshot(name: &str, actual: &[u8], expected: &[u8]) {
    if std::env::var_os("SMACKDEBT_UPDATE_SNAPSHOTS").is_some() {
        fs::write(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/snapshots")
                .join(name),
            actual,
        )
        .unwrap();
        return;
    }
    assert_eq!(actual, expected, "snapshot {name} changed");
}

fn validate_schema(report: &serde_json::Value) {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../schemas/report-v2.schema.json")).unwrap();
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(report)
        .unwrap();
}

fn assert_index_integrity(report: &serde_json::Value) {
    let paths = report["paths"].as_array().unwrap();
    let scopes = report["scopes"].as_array().unwrap();
    let files = report["files"].as_array().unwrap();
    let findings = report["findings"].as_array().unwrap();
    let comparisons = report["comparisons"].as_array().unwrap();
    let health = report["health"].as_array().unwrap();
    let activity = report["activity"].as_array().unwrap();

    assert!(report.get("summary").is_none());
    for field in ["root", "selected_scope"] {
        if let Some(index) = report[field].as_u64() {
            assert!(
                (index as usize) < scopes.len(),
                "{field} points outside scopes"
            );
        }
    }
    assert_eq!(health.len(), scopes.len() + files.len());
    assert_eq!(activity.len(), files.len());

    for (index, scope) in scopes.iter().enumerate() {
        assert_eq!(scope["id"], index);
        assert!(scope.get("name").is_none());
        if let Some(path) = scope["path"].as_u64() {
            assert!((path as usize) < paths.len());
        }
        assert!((scope["health"].as_u64().unwrap() as usize) < health.len());
        if let Some(parent) = scope["parent"].as_u64() {
            let parent = parent as usize;
            assert!(parent < scopes.len());
            assert!(
                scopes[parent]["children"]
                    .as_array()
                    .unwrap()
                    .contains(&scope["id"])
            );
        }
        for child in scope["children"].as_array().unwrap() {
            let child = child.as_u64().unwrap() as usize;
            assert!(child < scopes.len());
            assert_eq!(scopes[child]["parent"], index);
        }
        for finding in scope["findings"].as_array().unwrap() {
            assert!((finding.as_u64().unwrap() as usize) < findings.len());
        }
        for comparison in scope["comparisons"].as_array().unwrap() {
            assert!((comparison.as_u64().unwrap() as usize) < comparisons.len());
        }
    }
    for (index, file) in files.iter().enumerate() {
        assert_eq!(file["id"], index);
        assert!(file.get("path_id").is_none());
        if let Some(path) = file["path"].as_u64() {
            assert!((path as usize) < paths.len());
        }
        assert!((file["scope"].as_u64().unwrap() as usize) < scopes.len());
        assert!((file["health"].as_u64().unwrap() as usize) < health.len());
        assert!((file["activity"].as_u64().unwrap() as usize) < activity.len());
    }
    for (index, finding) in findings.iter().enumerate() {
        assert_eq!(finding["id"], index);
        assert!((finding["file"].as_u64().unwrap() as usize) < files.len());
        assert!(finding["kind"].is_string());
    }
    for (index, comparison) in comparisons.iter().enumerate() {
        assert_eq!(comparison["id"], index);
        assert!(comparison["unit_kind"].is_string());
        if let Some(file) = comparison["file"].as_u64() {
            assert!((file as usize) < files.len());
        }
    }
    for (index, record) in health.iter().enumerate() {
        assert_eq!(record["id"], index);
    }
    for (index, record) in activity.iter().enumerate() {
        assert_eq!(record["id"], index);
        assert!((record["file"].as_u64().unwrap() as usize) < files.len());
    }
}

#[test]
fn json_schema_rejects_nested_field_type_drift() {
    let project = fixture();
    let bytes = run(["--json", project.path().to_str().unwrap()]);
    let mut report: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    report["scopes"][0]["coverage"]["selected_files"] = "one".into();
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../schemas/report-v2.schema.json")).unwrap();
    assert!(
        jsonschema::validator_for(&schema)
            .unwrap()
            .validate(&report)
            .is_err()
    );
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

fn run<const N: usize>(arguments: [&str; N]) -> Vec<u8> {
    cargo_bin_cmd!("smackdebt")
        .args(arguments)
        .output()
        .unwrap()
        .stdout
}

fn run_in<const N: usize>(directory: &Path, arguments: [&str; N]) -> Vec<u8> {
    cargo_bin_cmd!("smackdebt")
        .current_dir(directory)
        .args(arguments)
        .output()
        .unwrap()
        .stdout
}

fn source_engine_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::copy(
        Path::new(SOURCE_ENGINE_FIXTURE).join("Gemfile"),
        project.path().join("Gemfile"),
    )
    .unwrap();
    fs::copy(
        Path::new(SOURCE_ENGINE_FIXTURE).join("src/work.rb"),
        project.path().join("src/work.rb"),
    )
    .unwrap();
    project
}

fn git<const N: usize>(directory: &Path, arguments: [&str; N]) {
    let status = Command::new("git")
        .args(arguments)
        .current_dir(directory)
        .status()
        .unwrap();
    assert!(status.success());
}

fn fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("Gemfile"),
        "source 'https://example.invalid'\n",
    )
    .unwrap();
    fs::write(
        project.path().join("src/work.rb"),
        "def work(items)\n  items.each { |item| ship(item) if item.ready? }\nend\n",
    )
    .unwrap();
    fs::write(
        project.path().join("src/view.vue"),
        "<template><ul><li v-for=\"item in items\" v-if=\"item.ready\">{{ item.name }}</li></ul></template>\n<script setup lang=\"ts\">\nconst ready = (item) => item.ready\n</script>\n<style>.ready { color: green; }</style>\n",
    )
    .unwrap();
    project
}

fn mixed_fixture(files: usize) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("Gemfile"),
        "source 'https://example.invalid'\n",
    )
    .unwrap();
    let samples = [
        ("rs", "fn work(value: bool) { if value { work(false); } }\n"),
        ("rb", "def work(value)\n  return 1 if value\n  0\nend\n"),
        (
            "js",
            "function work(value) { if (value) return 1; return 0; }\n",
        ),
        (
            "py",
            "def work(value):\n    if value:\n        return 1\n    return 0\n",
        ),
        (
            "java",
            "class Work { int work(boolean value) { if (value) return 1; return 0; } }\n",
        ),
        (
            "c",
            "int work(int value) { if (value) return 1; return 0; }\n",
        ),
        (
            "cpp",
            "int work(bool value) { if (value) return 1; return 0; }\n",
        ),
        (
            "ts",
            "function work(value: boolean): number { if (value) return 1; return 0; }\n",
        ),
        (
            "tsx",
            "function Work({ value }: { value: boolean }) { return value ? <b /> : <i />; }\n",
        ),
        (
            "vue",
            "<template><b v-if=\"value\">yes</b></template>\n<script setup>const value = true</script>\n",
        ),
    ];
    for index in 0..files {
        let (extension, source) = samples[index % samples.len()];
        fs::write(
            project
                .path()
                .join(format!("source-{index:03}.{extension}")),
            source,
        )
        .unwrap();
    }
    project
}
