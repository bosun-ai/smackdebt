use std::fs;
use std::path::Path;
use std::process::Command;

use assert_cmd::cargo::cargo_bin_cmd;

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
fn serial_and_parallel_json_match_and_follow_schema_one() {
    let project = fixture();
    let path = project.path().to_str().unwrap();
    let serial = run(["--json", "--jobs", "1", path]);
    let parallel = run(["--json", "--jobs", "4", path]);
    assert_eq!(serial, parallel);
    let report: serde_json::Value = serde_json::from_slice(&serial).unwrap();
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["mode"], "codebase");
    assert!(report["findings"].is_array());
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
