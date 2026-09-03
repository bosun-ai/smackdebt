// The hermetic pinning and the edge-row invariant are shared with the unified
// suite; only those helpers are mapped in, because this suite builds its
// commands directly.
#[path = "support/edges.rs"]
mod edges;
#[path = "support/hermetic.rs"]
mod support;

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use edges::assert_no_dependency_edge_rows;
use support::hermetic_env;

const SOURCE_ENGINE_FIXTURE: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/source-engine");
const STATIC_ARCHITECTURE_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/static-architecture"
);

#[test]
fn serial_and_parallel_codebase_output_match() {
    let project = fixture();
    let serial = run(["--jobs", "1", project.path().to_str().unwrap()]);
    let parallel = run(["--jobs", "4", project.path().to_str().unwrap()]);
    assert_eq!(serial, parallel);
    let text = String::from_utf8(serial).unwrap();
    assert!(text.starts_with("smackdebt · "), "{text}");
    assert!(text.contains(" high · "), "{text}");
    assert!(text.contains(" checked"), "{text}");
    assert!(!text.contains("AREAS"));
    assert!(!text.contains("FINDINGS"));
}

#[test]
fn serial_and_parallel_json_match_and_follow_schema_four() {
    let project = fixture();
    let path = project.path().to_str().unwrap();
    let serial = run(["--json", "--jobs", "1", path]);
    let parallel = run(["--json", "--jobs", "4", path]);
    assert_eq!(serial, parallel);
    let report: serde_json::Value = serde_json::from_slice(&serial).unwrap();
    assert_eq!(report["schema_version"], 4);
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
fn anonymous_diff_matching_is_safe_and_worker_output_is_equal() {
    let project = anonymous_diff_fixture();
    let terminal = |jobs| {
        let output = smackdebt()
            .current_dir(project.path())
            .env("COLUMNS", "120")
            .args(["diff", "HEAD", "--all", "--color", "never", "--jobs", jobs])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    };
    let serial_terminal = terminal("1");
    let automatic_terminal = terminal("4");
    assert_eq!(serial_terminal, automatic_terminal);
    let text = String::from_utf8(serial_terminal).unwrap();
    assert_eq!(
        text.matches("1 file has anonymous units that could not be matched safely.")
            .count(),
        1,
        "{text}"
    );

    let machine = |jobs| {
        let output = smackdebt()
            .current_dir(project.path())
            .args(["diff", "HEAD", "--json", "--jobs", jobs])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    };
    let serial_json = machine("1");
    let automatic_json = machine("4");
    assert_eq!(serial_json, automatic_json);
    let report: serde_json::Value = serde_json::from_slice(&serial_json).unwrap();
    validate_schema(&report);
    assert_index_integrity(&report);
    let kinds: Vec<_> = report["comparisons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds.iter().filter(|kind| **kind == "ambiguous").count(), 1);
    assert_eq!(kinds.iter().filter(|kind| **kind == "added").count(), 2);
    assert_eq!(kinds.iter().filter(|kind| **kind == "removed").count(), 3);
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == "metric_changed")
            .count(),
        1
    );
    assert_eq!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|value| value["kind"] == "ambiguous_identity")
            .count(),
        1
    );
    let json_text = String::from_utf8(serial_json).unwrap();
    assert!(!json_text.contains("binding:"));
    assert!(!json_text.contains("argument:"));
    assert!(
        report["comparisons"]
            .as_array()
            .unwrap()
            .iter()
            .all(|value| value["name"] != "steady"),
        "the unchanged moved callback must disappear"
    );
}

#[test]
fn repeated_declared_names_do_not_emit_an_anonymous_warning() {
    let project = named_duplicate_fixture();
    let terminal = smackdebt()
        .current_dir(project.path())
        .args(["diff", "HEAD", "--all", "--color", "never"])
        .output()
        .unwrap();
    assert!(terminal.status.success());
    let terminal = String::from_utf8(terminal.stdout).unwrap();
    assert!(!terminal.contains("anonymous units"), "{terminal}");

    let output = smackdebt()
        .current_dir(project.path())
        .args(["diff", "HEAD", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["comparisons"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|value| value["kind"] == "ambiguous")
            .count(),
        1
    );
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .all(|value| value["kind"] != "ambiguous_identity")
    );
}

#[test]
fn added_and_removed_diff_cards_state_the_measurements_of_the_side_that_exists() {
    let project = one_sided_diff_fixture();
    let output = smackdebt()
        .current_dir(project.path())
        .env("COLUMNS", "120")
        .args(["diff", "HEAD", "--all", "--color", "never"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let terminal = String::from_utf8(output.stdout).unwrap();
    // An added unit has an after side only, so the card states it absolutely.
    assert!(
        terminal.contains(
            "        added · cognitive 3 · cyclomatic 3 · statements 2 · nesting 2 · parameters 1\n"
        ),
        "{terminal}"
    );
    // A removed unit has a before side only, stated the same way.
    assert!(
        terminal.contains(
            "        removed · cognitive 3 · cyclomatic 3 · statements 2 · nesting 2 · parameters 2\n"
        ),
        "{terminal}"
    );
    // The card carries facts, never arithmetic, for a side that does not exist.
    assert!(!terminal.contains('→'), "{terminal}");
}

#[test]
fn invalid_project_config_uses_argument_exit_code() {
    let project = fixture();
    fs::write(
        project.path().join(".smackdebt.toml"),
        "[thresholds.cognitive]\nwatch = 25\nhigh = 10\n",
    )
    .unwrap();
    smackdebt()
        .arg(project.path())
        .assert()
        .code(2)
        .stderr(predicates::str::contains("high greater than watch"));
}

#[test]
fn same_level_source_role_conflict_has_exact_argument_failure_streams() {
    let project = fixture();
    fs::write(
        project.path().join("conflict.js"),
        "export function work() { return 1; }\n",
    )
    .unwrap();
    fs::write(
        project.path().join(".smackdebt.toml"),
        "[source_roles]\ntest = ['*.js']\nfixture = ['conflict.js']\n",
    )
    .unwrap();

    smackdebt()
        .arg(project.path())
        .assert()
        .code(2)
        .stdout("")
        .stderr(
            "smackdebt: source roles conflict for conflict.js: test, fixture; update .smackdebt.toml\n",
        );
}

const GATE_BASELINE_HEADERS: &str = "# smackdebt gate baseline v1\npath\tsignal\thigh\twatch\n";

/// One High-cognitive JavaScript unit under a lowered threshold, so the
/// observed gate snapshot carries exactly one cognitive key.
fn gate_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("work.js"),
        "export function work(x) {\n  if (x) {\n    if (x > 1) {\n      return 2;\n    }\n  }\n  return 1;\n}\n",
    )
    .unwrap();
    fs::write(
        project.path().join(".smackdebt.toml"),
        "[thresholds.cognitive]\nwatch = 1\nhigh = 2\n",
    )
    .unwrap();
    project
}

#[test]
fn a_missing_gate_baseline_has_exact_argument_failure_streams() {
    let project = gate_fixture();
    let baseline = project.path().join(".smackdebt-baseline.tsv");
    smackdebt()
        .arg("gate")
        .arg(project.path())
        .assert()
        .code(2)
        .stdout("")
        .stderr(format!(
            "smackdebt: baseline not found: {}\n",
            baseline.display()
        ));
}

#[test]
fn a_clean_gate_states_zero_totals_and_succeeds() {
    let project = gate_fixture();
    let baseline = project.path().join(".smackdebt-baseline.tsv");
    fs::write(
        &baseline,
        format!("{GATE_BASELINE_HEADERS}work.js\tcognitive\t1\t0\n"),
    )
    .unwrap();
    smackdebt()
        .arg("gate")
        .arg(project.path())
        .assert()
        .code(0)
        .stdout(format!(
            "GATE  {}\n\n0 regressions · 0 improvements\n",
            baseline.display()
        ))
        .stderr("");
}

#[test]
fn a_regressed_gate_names_the_offending_row_and_exits_three() {
    let project = gate_fixture();
    let baseline = project.path().join(".smackdebt-baseline.tsv");
    fs::write(&baseline, GATE_BASELINE_HEADERS).unwrap();
    smackdebt()
        .arg("gate")
        .arg(project.path())
        .assert()
        .code(3)
        .stdout(format!(
            concat!(
                "GATE  {}\n",
                "\n",
                "  worse  work.js · cognitive · high 0 → 1\n",
                "\n",
                "1 regression · 0 improvements\n",
                "next: smackdebt gate --update\n",
            ),
            baseline.display()
        ))
        .stderr("");
}

#[test]
fn a_looser_gate_baseline_reports_improvements_and_stays_untouched() {
    let project = gate_fixture();
    let baseline = project.path().join(".smackdebt-baseline.tsv");
    let written =
        format!("{GATE_BASELINE_HEADERS}gone.js\tcognitive\t1\t0\nwork.js\tcognitive\t1\t0\n");
    fs::write(&baseline, &written).unwrap();
    smackdebt()
        .arg("gate")
        .arg(project.path())
        .assert()
        .code(0)
        .stdout(format!(
            concat!(
                "GATE  {}\n",
                "\n",
                "  better gone.js · cognitive · high 1 → 0\n",
                "\n",
                "0 regressions · 1 improvement\n",
            ),
            baseline.display()
        ))
        .stderr("");
    assert_eq!(fs::read_to_string(&baseline).unwrap(), written);
}

#[test]
fn a_baseline_update_is_byte_stable_and_idempotent() {
    let project = gate_fixture();
    let baseline = project.path().join(".smackdebt-baseline.tsv");
    smackdebt()
        .arg("gate")
        .arg("--update")
        .arg(project.path())
        .assert()
        .code(0)
        .stdout("")
        .stderr("");
    let written = fs::read_to_string(&baseline).unwrap();
    assert_eq!(
        written,
        format!("{GATE_BASELINE_HEADERS}work.js\tcognitive\t1\t0\n")
    );
    smackdebt()
        .arg("gate")
        .arg("--update")
        .arg(project.path())
        .assert()
        .code(0)
        .stdout("")
        .stderr("");
    assert_eq!(fs::read_to_string(&baseline).unwrap(), written);
    // The written baseline immediately gates its own tree clean.
    smackdebt()
        .arg("gate")
        .arg(project.path())
        .assert()
        .code(0)
        .stdout(format!(
            "GATE  {}\n\n0 regressions · 0 improvements\n",
            baseline.display()
        ))
        .stderr("");
}

#[test]
fn a_regressed_gate_json_result_follows_its_checked_schema() {
    let project = gate_fixture();
    let baseline = project.path().join(".smackdebt-baseline.tsv");
    fs::write(&baseline, GATE_BASELINE_HEADERS).unwrap();
    let output = smackdebt()
        .arg("gate")
        .arg("--json")
        .arg(project.path())
        .assert()
        .code(3)
        .stderr("")
        .get_output()
        .stdout
        .clone();
    let result: serde_json::Value = serde_json::from_slice(&output).unwrap();
    validate_gate_schema(&result);
    assert_eq!(result["schema_version"], 1);
    assert_eq!(result["status"], "regressed");
    assert_eq!(result["baseline"], baseline.display().to_string());
    let regressions = result["regressions"].as_array().unwrap();
    assert_eq!(regressions.len(), 1);
    assert_eq!(regressions[0]["path"], "work.js");
    assert_eq!(regressions[0]["signal"], "cognitive");
    assert_eq!(regressions[0]["baseline_high"], 0);
    assert_eq!(regressions[0]["high"], 1);
    assert_eq!(regressions[0]["baseline_watch"], 0);
    assert_eq!(regressions[0]["watch"], 0);
    assert!(result["improvements"].as_array().unwrap().is_empty());
    assert_eq!(result["totals"]["regressions"], 1);
    assert_eq!(result["totals"]["improvements"], 0);
}

#[test]
fn a_clean_gate_json_result_states_its_status_and_exits_zero() {
    let project = gate_fixture();
    let baseline = project.path().join(".smackdebt-baseline.tsv");
    fs::write(
        &baseline,
        format!("{GATE_BASELINE_HEADERS}work.js\tcognitive\t1\t0\n"),
    )
    .unwrap();
    let output = smackdebt()
        .arg("gate")
        .arg("--json")
        .arg(project.path())
        .assert()
        .code(0)
        .stderr("")
        .get_output()
        .stdout
        .clone();
    let result: serde_json::Value = serde_json::from_slice(&output).unwrap();
    validate_gate_schema(&result);
    assert_eq!(result["status"], "clean");
    assert!(result["regressions"].as_array().unwrap().is_empty());
    assert!(result["improvements"].as_array().unwrap().is_empty());
}

#[test]
fn a_malformed_gate_baseline_never_passes() {
    let project = gate_fixture();
    let baseline = project.path().join(".smackdebt-baseline.tsv");
    fs::write(
        &baseline,
        format!("{GATE_BASELINE_HEADERS}work.js\tcoupling\t1\t0\n"),
    )
    .unwrap();
    smackdebt()
        .arg("gate")
        .arg(project.path())
        .assert()
        .code(2)
        .stdout("")
        .stderr("smackdebt: baseline line 3: unknown signal 'coupling'\n");
}

#[test]
fn configured_source_role_overrides_the_generic_path_role() {
    let project = fixture();
    fs::create_dir_all(project.path().join("tests")).unwrap();
    fs::write(
        project.path().join("tests/work.js"),
        "export function work() { return 1; }\n",
    )
    .unwrap();
    fs::write(
        project.path().join(".smackdebt.toml"),
        "[source_roles]\nexample = ['tests/work.js']\n",
    )
    .unwrap();

    let output = run(["--json", project.path().to_str().unwrap()]);
    let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let paths = report["paths"].as_array().unwrap();
    let file = report["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|file| paths[file["path"].as_u64().unwrap() as usize] == "tests/work.js")
        .unwrap();
    assert_eq!(file["role"], "example");
}

#[test]
fn rails_schema_findings_stay_visible_without_entering_default_verdicts() {
    let project = tempfile::tempdir().unwrap();
    fs::create_dir_all(project.path().join("db")).unwrap();
    fs::create_dir_all(project.path().join("app/models")).unwrap();
    fs::write(
        project.path().join("Gemfile"),
        "source 'https://example.invalid'\n",
    )
    .unwrap();
    fs::write(
        project.path().join(".smackdebt.toml"),
        "[thresholds.cognitive]\nwatch = 1\nhigh = 2\n[thresholds.cyclomatic]\nwatch = 1\nhigh = 2\n[thresholds.function_lines]\nwatch = 1\nhigh = 2\n",
    )
    .unwrap();
    let schema = "def generated_schema(one, two)\n  if one\n    if two\n      create_table(:items)\n    end\n  end\nend\n";
    fs::write(project.path().join("db/schema.rb"), schema).unwrap();
    fs::write(
        project.path().join("app/models/schema.rb"),
        "def user_schema; :kept; end\n",
    )
    .unwrap();

    let json = run_in(project.path(), ["--json", "--jobs", "1"]);
    let report: serde_json::Value = serde_json::from_slice(&json).unwrap();
    let paths = report["paths"].as_array().unwrap();
    let file_for = |path: &str| {
        report["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|file| paths[file["path"].as_u64().unwrap() as usize] == path)
            .unwrap()
    };
    let generated = file_for("db/schema.rb");
    let user = file_for("app/models/schema.rb");
    assert_eq!(generated["role"], "generated");
    let generated_health = &report["health"][generated["health"].as_u64().unwrap() as usize];
    assert_eq!(generated_health["watch"], 0);
    assert_eq!(generated_health["high"], 0);
    assert_eq!(generated["coverage"]["context_files"], 1);
    assert_eq!(user["role"], "primary");
    let user_health = &report["health"][user["health"].as_u64().unwrap() as usize];
    assert!(
        report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| {
                finding["name"] == "generated_schema"
                    && finding["rating"] == "high"
                    && finding["role"] == "generated"
            })
    );
    let root_health = &report["health"][report["scopes"][0]["health"].as_u64().unwrap() as usize];
    for field in ["healthy", "watch", "high"] {
        assert_eq!(root_health[field], user_health[field], "{field}");
    }

    let default = String::from_utf8(run_in(project.path(), ["--color", "never"])).unwrap();
    assert!(!default.contains("db/schema.rb"));
    let detailed =
        String::from_utf8(run_in(project.path(), ["--all", "--color", "never"])).unwrap();
    assert!(detailed.contains("db/schema.rb"));
    assert!(detailed.contains("generated"));

    git(project.path(), ["init", "-b", "main"]);
    git(project.path(), ["config", "user.name", "Smackdebt Test"]);
    git(
        project.path(),
        ["config", "user.email", "smackdebt@example.invalid"],
    );
    git(project.path(), ["add", "."]);
    git(project.path(), ["commit", "-m", "test: schema base"]);
    fs::write(
        project.path().join("db/schema.rb"),
        schema.replace("create_table(:items)", "create_table(:orders)"),
    )
    .unwrap();
    let diff: serde_json::Value = serde_json::from_slice(&run_in(
        project.path(),
        ["diff", "main", "--json", "--jobs", "1"],
    ))
    .unwrap();
    assert!(diff["comparisons"].as_array().unwrap().is_empty());
    assert_eq!(diff["scopes"][0]["diff"]["total"], 0);
    assert!(diff["findings"].as_array().unwrap().iter().any(|finding| {
        finding["name"] == "generated_schema" && finding["role"] == "generated"
    }));
}

#[test]
fn terminal_color_can_be_forced_or_disabled_through_a_pipe() {
    let project = fixture();
    let path = project.path().to_str().unwrap();
    let colored = run(["--color", "always", path]);
    assert!(colored.windows(2).any(|bytes| bytes == b"\x1b["));

    let plain = run(["--color", "never", path]);
    assert!(!plain.windows(2).any(|bytes| bytes == b"\x1b["));
    // Forcing color also forces decoration, so the plain report is what
    // remains once the glyphs, the tier bar, and the styling are removed.
    assert_eq!(strip_decorations(&strip_ansi(&colored)), plain);
    assert!(private_use_codepoints(&plain).is_empty());
}

#[test]
fn explicit_color_is_rejected_for_json() {
    let project = fixture();
    smackdebt()
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

#[test]
fn the_global_gitignore_excludes_candidates() {
    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("kept.js"), "export const kept = 1;\n").unwrap();
    fs::write(
        project.path().join("machine-wide-ignored.js"),
        "export const dropped = 1;\n",
    )
    .unwrap();
    // Every child process shares one pinned home; the global gitignore lives
    // at git/ignore under it. The pattern names one file unique to this test.
    let home = hermetic_env(&mut Command::new("git"));
    fs::create_dir_all(home.join("git")).unwrap();
    fs::write(home.join("git/ignore"), "machine-wide-ignored.js\n").unwrap();

    let output = run_in(project.path(), ["--json", "--jobs", "1"]);
    let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let paths: Vec<&str> = report["paths"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|path| path.as_str())
        .collect();
    assert!(paths.contains(&"kept.js"), "{paths:?}");
    assert!(!paths.contains(&"machine-wide-ignored.js"), "{paths:?}");
}

#[test]
fn ancestor_ignore_files_never_swallow_the_committed_source_engine_fixture() {
    // The committed fixture lives inside this workspace, so with git ignore
    // semantics every ancestor ignore file up to the workspace root applies
    // to it. This exact evidence fails loudly if a future edit to the
    // workspace `.gitignore` starts matching fixture files.
    let output = run(["--json", "--jobs", "1", SOURCE_ENGINE_FIXTURE]);
    let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let scope = &report["scopes"][report["selected_scope"].as_u64().unwrap() as usize];
    assert_eq!(
        report["paths"][scope["path"].as_u64().unwrap() as usize],
        "crates/cli/tests/fixtures/source-engine"
    );
    assert_eq!(scope["coverage"]["selected_files"], 1);
    assert_eq!(scope["coverage"]["analyzed_files"], 1);
}

#[test]
fn nested_git_checkouts_are_pruned_and_disclosed() {
    let project = nested_checkout_fixture();

    // Default output discloses the pruned checkouts without detail flags.
    let terminal = run_in(project.path(), ["--jobs", "1", "--color", "never"]);
    assert_snapshot(
        "nested-checkout.terminal.txt",
        &terminal,
        include_bytes!("snapshots/nested-checkout.terminal.txt"),
    );
    let text = String::from_utf8(terminal).unwrap();
    assert!(
        text.contains("2 nested repositories were not analyzed."),
        "{text}"
    );

    let json = run_in(project.path(), ["--json", "--jobs", "1"]);
    let report: serde_json::Value = serde_json::from_slice(&json).unwrap();
    validate_schema(&report);
    let messages: Vec<&str> = report["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|diagnostic| diagnostic["kind"] == "nested_repository")
        .map(|diagnostic| diagnostic["message"].as_str().unwrap())
        .collect();
    assert_eq!(
        messages,
        [
            "clone is a nested repository",
            "worktree is a nested repository"
        ]
    );
    let paths: Vec<&str> = report["paths"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|path| path.as_str())
        .collect();
    assert!(paths.contains(&"kept.js"), "{paths:?}");
    assert!(!paths.iter().any(|path| path.contains("lost")), "{paths:?}");
}

#[test]
fn a_mostly_unsupported_selection_qualifies_its_verdict() {
    let project = unsupported_share_fixture();

    // The qualifier row renders in default output, directly under the tier
    // sentence, without any detail flag.
    let terminal = run_in(project.path(), ["--jobs", "1", "--color", "never"]);
    assert_snapshot(
        "unsupported-share.terminal.txt",
        &terminal,
        include_bytes!("snapshots/unsupported-share.terminal.txt"),
    );
    let text = String::from_utf8(terminal).unwrap();
    assert!(text.contains("Not all source was checked."), "{text}");
    assert!(
        text.contains("1 of 2 source files were analyzed."),
        "{text}"
    );

    let json = run_in(project.path(), ["--json", "--jobs", "1"]);
    let report: serde_json::Value = serde_json::from_slice(&json).unwrap();
    validate_schema(&report);
    let qualifier = &report["verdict"]["qualifier"];
    assert_eq!(qualifier["sentence"], "Not all source was checked.");
    assert_eq!(qualifier["detail"], "1 of 2 source files were analyzed.");
    assert_eq!(qualifier["selected_files"], 2);
    assert_eq!(qualifier["analyzed_files"], 1);
    assert_eq!(qualifier["share_permille"], 625);
    assert_eq!(qualifier["largest_language"], "Go");
    let coverage = &report["scopes"][0]["coverage"];
    assert_eq!(coverage["selected_bytes"], 104);
    assert_eq!(coverage["unsupported_bytes"], 65);
    // The qualifier informs the reader only; the tier is still selected from
    // the checked units alone.
    assert_eq!(report["verdict"]["tier"], "clean");
}

#[test]
fn fully_analyzed_coverage_leaves_the_verdict_head_unqualified() {
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("kept.js"),
        "export function kept() {\n  return 1;\n}\n",
    )
    .unwrap();
    let json = run_in(project.path(), ["--json", "--jobs", "1"]);
    let report: serde_json::Value = serde_json::from_slice(&json).unwrap();
    validate_schema(&report);
    assert!(
        report["verdict"].get("qualifier").is_none(),
        "{}",
        report["verdict"]
    );
}

fn explicit_scope_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    fs::create_dir_all(project.path().join("src")).unwrap();
    fs::create_dir(project.path().join("empty")).unwrap();
    fs::write(
        project.path().join("src/good.js"),
        "export function good() { return 1; }\n",
    )
    .unwrap();
    fs::write(
        project.path().join("src/page.astro"),
        "---\nconst title = 'Hello';\n---\n<h1>{title}</h1>\n",
    )
    .unwrap();
    fs::write(project.path().join("README.txt"), "notes\n").unwrap();
    git(project.path(), ["init", "-q"]);
    project
}

#[test]
fn explicit_source_scopes_are_exact_and_absolute_input_keeps_repository_identity() {
    let project = explicit_scope_fixture();
    for (path, expected_kind, selected, analyzed) in [
        ("src/good.js", "file", 1, 1),
        ("src/page.astro", "file", 1, 0),
        ("src", "directory", 2, 1),
    ] {
        let output = smackdebt()
            .current_dir(project.path())
            .args([path, "--json", "--jobs", "1"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}: {}",
            path,
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        validate_schema(&report);
        let scope = &report["scopes"][report["selected_scope"].as_u64().unwrap() as usize];
        assert_eq!(scope["kind"], expected_kind, "{path}");
        assert_eq!(
            report["paths"][scope["path"].as_u64().unwrap() as usize],
            path
        );
        assert_eq!(scope["coverage"]["selected_files"], selected);
        assert_eq!(scope["coverage"]["analyzed_files"], analyzed);
    }

    let absolute = project.path().join("src/good.js");
    let absolute_output = smackdebt()
        .current_dir(project.path())
        .args([absolute.as_os_str(), std::ffi::OsStr::new("--json")])
        .output()
        .unwrap();
    assert!(absolute_output.status.success());
    let absolute_report: serde_json::Value =
        serde_json::from_slice(&absolute_output.stdout).unwrap();
    let selected = absolute_report["selected_scope"].as_u64().unwrap() as usize;
    let scope = &absolute_report["scopes"][selected];
    assert_eq!(
        absolute_report["paths"][scope["path"].as_u64().unwrap() as usize],
        "src/good.js"
    );
    assert!(
        !String::from_utf8_lossy(&absolute_output.stdout)
            .contains(project.path().to_str().unwrap())
    );
}

#[test]
fn modified_astro_diff_retains_coverage_and_counts_one_unsupported_file() {
    let project = explicit_scope_fixture();
    let astro = smackdebt()
        .current_dir(project.path())
        .args(["src/page.astro", "--json", "--jobs", "1"])
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&astro.stdout).unwrap();
    assert_eq!(report["files"][0]["language"], "astro");
    assert_eq!(report["verdict"]["qualifier"]["selected_files"], 1);
    assert_eq!(report["verdict"]["qualifier"]["analyzed_files"], 0);
    assert_eq!(
        report["verdict"]["qualifier"]["detail"],
        "0 of 1 source files were analyzed."
    );

    git(project.path(), ["config", "user.name", "Smackdebt Test"]);
    git(
        project.path(),
        ["config", "user.email", "smackdebt@example.invalid"],
    );
    git(project.path(), ["add", "."]);
    git(project.path(), ["commit", "-qm", "test: add source"]);
    fs::write(
        project.path().join("src/page.astro"),
        "---\nconst title = 'Changed';\n---\n<h1>{title}</h1>\n",
    )
    .unwrap();
    let diff = smackdebt()
        .current_dir(project.path())
        .args(["diff", "HEAD", "--json", "--jobs", "1"])
        .output()
        .unwrap();
    assert!(
        diff.status.success(),
        "{}",
        String::from_utf8_lossy(&diff.stderr)
    );
    let diff: serde_json::Value = serde_json::from_slice(&diff.stdout).unwrap();
    validate_schema(&diff);
    assert!(
        diff["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file["language"] == "astro")
    );
    assert_eq!(diff["scopes"][0]["coverage"]["selected_files"], 1);
    assert_eq!(diff["scopes"][0]["coverage"]["analyzed_files"], 0);
    let diff_terminal = String::from_utf8(run_in(
        project.path(),
        ["diff", "HEAD", "--color", "never", "--jobs", "1"],
    ))
    .unwrap();
    assert!(
        diff_terminal.contains("warning 1 source file uses an unsupported language."),
        "{diff_terminal}"
    );
    assert!(!diff_terminal.contains("2 source files"), "{diff_terminal}");
}

#[test]
fn invalid_and_source_free_paths_return_exact_errors_without_repository_fallback() {
    let project = explicit_scope_fixture();
    for (path, message) in [
        ("empty", "smackdebt: no source files found under: empty\n"),
        ("README.txt", "smackdebt: not a source file: README.txt\n"),
        ("missing", "smackdebt: path not found: missing\n"),
    ] {
        let output = smackdebt()
            .current_dir(project.path())
            .arg(path)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{path}");
        assert!(output.stdout.is_empty(), "{path}");
        assert_eq!(String::from_utf8(output.stderr).unwrap(), message, "{path}");
    }
}

/// Limited explicit analysis knows the selected High count but does not read
/// the rest of the repository to invent a denominator.
#[test]
fn fresh_explicit_scopes_omit_unmeasured_repository_share() {
    let packages = problem_pattern_fixture();
    let root = packages.path().to_str().unwrap().to_owned();
    let god = packages.path().join("god");
    let god = god.to_str().unwrap();

    let root_terminal = String::from_utf8(run(["--color", "never", &root])).unwrap();
    assert!(
        !root_terminal.contains("live here."),
        "the repository root frames nothing: {root_terminal}"
    );
    let root_report: serde_json::Value = serde_json::from_slice(&run(["--json", &root])).unwrap();
    validate_schema(&root_report);
    assert!(
        root_report["verdict"].get("share").is_none(),
        "{}",
        root_report["verdict"]
    );

    // A limited explicit report does not pretend its retained root is the
    // complete repository, so it carries no repository share.
    let package_terminal = String::from_utf8(run(["--color", "never", god])).unwrap();
    let package_lines: Vec<&str> = package_terminal.lines().collect();
    assert_eq!(package_lines[0], "smackdebt · god");
    assert_eq!(package_lines[1], "  Worn in the usual places.");
    assert!(package_lines[2].contains(" high · "), "{package_terminal}");
    let package_report: serde_json::Value = serde_json::from_slice(&run(["--json", god])).unwrap();
    validate_schema(&package_report);
    assert!(package_report["verdict"].get("share").is_none());
    // The selected result remains truthful without claiming the measured root
    // count belongs to this separate limited report.
    assert_eq!(package_report["summary"]["high"], 3);
    assert_eq!(root_report["summary"]["high"], 4);
    assert_eq!(package_report["verdict"]["tier"], "worn");

    // Other limited directories likewise avoid a repository-wide claim.
    let split = split_debt_fixture();
    let messy = split.path().join("messy");
    let messy = messy.to_str().unwrap();
    let clean = split.path().join("clean");
    let clean = clean.to_str().unwrap();
    let messy_terminal = String::from_utf8(run(["--color", "never", messy])).unwrap();
    assert!(!messy_terminal.contains("live here."), "{messy_terminal}");
    let messy_report: serde_json::Value = serde_json::from_slice(&run(["--json", messy])).unwrap();
    validate_schema(&messy_report);
    assert!(messy_report["verdict"].get("share").is_none());
    let clean_report: serde_json::Value = serde_json::from_slice(&run(["--json", clean])).unwrap();
    validate_schema(&clean_report);
    assert!(clean_report["verdict"].get("share").is_none());
}

/// A repository without High debt has no fraction to divide, so no sub-scope
/// states a zero-of-zero sentence.
#[test]
fn a_repository_without_high_debt_frames_no_sub_scope() {
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/kept.js"),
        "export function kept() {\n  return 1;\n}\n",
    )
    .unwrap();
    let terminal = String::from_utf8(run_in(
        project.path(),
        ["--color", "never", "--jobs", "1", "src"],
    ))
    .unwrap();
    assert!(!terminal.contains("live here."), "{terminal}");
    let report: serde_json::Value =
        serde_json::from_slice(&run_in(project.path(), ["--json", "--jobs", "1", "src"])).unwrap();
    validate_schema(&report);
    assert_eq!(report["summary"]["high"], 0);
    assert!(
        report["verdict"].get("share").is_none(),
        "{}",
        report["verdict"]
    );
}

#[test]
fn static_architecture_codebase_snapshots_are_reviewed() {
    let project = static_architecture_fixture();
    let terminal = run_in(project.path(), ["--jobs", "1", "--color", "never"]);
    assert_snapshot(
        "static-architecture.terminal.txt",
        &terminal,
        include_bytes!("snapshots/static-architecture.terminal.txt"),
    );
    let detailed = run_in(project.path(), ["--all", "--jobs", "1", "--color", "never"]);
    assert_snapshot(
        "static-architecture-all.terminal.txt",
        &detailed,
        include_bytes!("snapshots/static-architecture-all.terminal.txt"),
    );
    let json = run_in(project.path(), ["--json", "--jobs", "1"]);
    assert_snapshot(
        "static-architecture.json",
        &json,
        include_bytes!("snapshots/static-architecture.json"),
    );
    let report: serde_json::Value = serde_json::from_slice(&json).unwrap();
    validate_schema(&report);
    assert_index_integrity(&report);
    assert_eq!(report["architecture_findings"].as_array().unwrap().len(), 2);
    assert_eq!(report["dependency_coverage"]["external"], 1);
    assert_eq!(report["dependency_coverage"]["unresolved"], 1);
    assert_eq!(report["dependency_coverage"]["ambiguous"], 1);
    assert_eq!(
        report["dependency_coverage"]["module_ownership_relations"],
        1
    );
    let partition_total: u64 = [
        "resolved_internal_uses",
        "unresolved_internal_uses",
        "ambiguous_internal_uses",
        "external_uses",
        "unresolved_package_uses",
        "module_ownership_relations",
        "context_relations",
    ]
    .iter()
    .map(|field| report["dependency_coverage"][field].as_u64().unwrap())
    .sum();
    assert_eq!(
        partition_total,
        report["dependency_coverage"]["total"].as_u64().unwrap()
    );
    assert!(
        report["package_graph"]
            .as_array()
            .unwrap()
            .iter()
            .any(|package| package["fan_in"] == 1 && package["fan_out"] == 1)
    );
    // `--all` shows all useful debt; raw dependency edges live in JSON only,
    // which every terminal result in this suite carries as an invariant.
    let detailed_text = String::from_utf8(detailed).unwrap();
    assert!(!detailed_text.contains("primary/trusted"));
}

#[test]
fn long_fact_families_keep_their_meaning_at_fifty_columns() {
    let project = long_responsive_fixture();
    let output = smackdebt()
        .current_dir(project.path())
        .env("COLUMNS", "50")
        .args(["--all", "--color", "never", "--history", "36500d"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_snapshot(
        "long-responsive-50.terminal.txt",
        &output.stdout,
        include_bytes!("snapshots/long-responsive-50.terminal.txt"),
    );
    let terminal = String::from_utf8(output.stdout).unwrap();
    assert_narrow(&terminal, 50);
    for fact in ["method · test", "statements 56"] {
        assert!(terminal.contains(fact), "missing {fact}: {terminal}");
    }
    let lines = terminal.lines().collect::<Vec<_>>();
    assert!(terminal.contains("high circular dependency"), "{terminal}");
    // The card states the witness step by step first and its member count
    // last, and a narrow width stacks a long path over several lines.
    let head = lines
        .iter()
        .position(|line| line.contains("circular dependency"))
        .unwrap_or_else(|| panic!("{terminal}"));
    let members = lines[head..]
        .iter()
        .position(|line| line.trim().ends_with(" in the cycle"))
        .map(|offset| head + offset)
        .unwrap_or_else(|| panic!("{terminal}"));
    assert!(members > head + 1, "{terminal}");
    let witness = lines[head + 1..members]
        .iter()
        .map(|line| line.trim().trim_start_matches("→ ").to_owned())
        .collect::<Vec<_>>()
        .join("");
    assert!(
        witness.starts_with("very-long-source-package-name/"),
        "{terminal}"
    );
    assert!(
        witness.ends_with("very-long-source-package-name/very-long-source-directory-name/very-long-source-file-name.js"),
        "{terminal}"
    );

    // A path view keeps the relationships that explain one package, still at
    // fifty columns and still without losing a fact.
    let path = smackdebt()
        .current_dir(project.path())
        .env("COLUMNS", "50")
        .args([
            "very-long-source-package-name",
            "--all",
            "--color",
            "never",
            "--history",
            "36500d",
        ])
        .output()
        .unwrap();
    assert!(
        path.status.success(),
        "{}",
        String::from_utf8_lossy(&path.stderr)
    );
    assert_snapshot(
        "long-responsive-path-50.terminal.txt",
        &path.stdout,
        include_bytes!("snapshots/long-responsive-path-50.terminal.txt"),
    );
    let path = String::from_utf8(path.stdout).unwrap();
    assert_narrow(&path, 50);
    // `--all` keeps the imports that could not be followed; the resolved uses
    // beside them are graph facts and stay in the machine report.
    for fact in ["could not be matched", "matched more than one file"] {
        assert!(path.contains(fact), "missing {fact}: {path}");
    }
    assert_no_dependency_edge_rows(&path, "long responsive path view");
    // References outside the repository are not debt and never reach a human.
    assert!(!path.contains("external"), "{path}");

    // Module wiring is a relationship too, so no ownership row survives.
    let owned = String::from_utf8(run_in(
        project.path(),
        [
            "very-long-rust-ownership-package-name",
            "--all",
            "--color",
            "never",
        ],
    ))
    .unwrap();
    assert!(!owned.contains(" owns "), "{owned}");
}

/// Every line fits the requested width and none reached the safety shortening.
fn assert_narrow(terminal: &str, width: usize) {
    for line in terminal.lines() {
        assert!(
            unicode_width::UnicodeWidthStr::width(line) <= width,
            "{line}"
        );
        assert!(!line.ends_with('…'), "safety fallback clipped: {line}");
    }
}

#[test]
fn architecture_path_drill_shows_findings_without_edge_rows() {
    let project = static_architecture_fixture();
    git(project.path(), ["init", "-b", "main"]);
    let output = run_in(project.path(), ["app", "--all", "--color", "never"]);
    let text = String::from_utf8(output).unwrap();
    // Sibling source outside the selected path is not read. The local cycle
    // remains visible and the cross-scope import becomes unresolved evidence.
    assert!(!text.contains("core/main.js → app/main.js"), "{text}");
    assert!(!text.contains("app/main.js → core/main.js"), "{text}");
    assert!(
        text.contains("watch circular dependency · app/choice.js"),
        "{text}"
    );
    assert!(!text.contains("        → core/main.js"), "{text}");
    assert!(text.contains("3 imports could not be followed"), "{text}");
    assert!(!text.contains("native/src/helper.rs"));

    // Module wiring is a relationship too: a Rust package states no ownership.
    let native = String::from_utf8(run_in(
        project.path(),
        ["native", "--all", "--color", "never"],
    ))
    .unwrap();
    assert!(!native.contains(" owns "), "{native}");
    assert!(!native.contains("native/src/helper.rs"), "{native}");
}

#[test]
fn selected_path_scopes_import_warning_counts() {
    let project = static_architecture_fixture();
    // Both causes in one scope: the total row carries one fact per cause.
    let app = String::from_utf8(run_in(project.path(), ["app", "--color", "never"])).unwrap();
    assert!(
        app.contains("warning 3 imports could not be followed"),
        "{app}"
    );
    assert!(app.contains("2 named nothing in the repository"), "{app}");
    assert!(app.contains("1 matched more than one file"), "{app}");

    // A single cause states the total and that cause only.
    let core = String::from_utf8(run_in(project.path(), ["core", "--color", "never"])).unwrap();
    assert!(
        core.contains("warning 1 import could not be followed · 1 named nothing in the repository"),
        "{core}"
    );
    assert!(!core.contains("matched more than one file"), "{core}");

    let native = String::from_utf8(run_in(project.path(), ["native", "--color", "never"])).unwrap();
    assert!(!native.contains("could not be followed"), "{native}");
}

#[test]
fn diff_and_path_views_state_findings_without_current_edges() {
    let project = static_architecture_fixture();
    git(project.path(), ["init", "-b", "main"]);
    git(project.path(), ["config", "user.name", "Smackdebt Test"]);
    git(
        project.path(),
        ["config", "user.email", "smackdebt@example.invalid"],
    );
    git(project.path(), ["add", "."]);
    git(project.path(), ["commit", "-m", "test: architecture base"]);
    fs::write(
        project.path().join("app/main.js"),
        "import core from '../core/main';\nimport coreAgain from '../core/main';\n\nexport function app() { return core(coreAgain); }\n",
    )
    .unwrap();

    // Adding one import moves no rated debt, so every diff view is the
    // verdict block and nothing else.
    let root =
        String::from_utf8(run_in(project.path(), ["diff", "main", "--color", "never"])).unwrap();
    assert!(root.contains("No debt changed."), "{root}");
    assert!(!root.contains("ARCHITECTURE"), "{root}");
    for arguments in [
        ["diff", "main", "app", "--color", "never"],
        ["diff", "main", "core", "--color", "never"],
    ] {
        let terminal = String::from_utf8(run_in(project.path(), arguments)).unwrap();
        assert!(terminal.contains("No debt changed."), "{terminal}");
        assert!(!terminal.contains("ARCHITECTURE"), "{terminal}");
        assert!(!terminal.contains("HISTORY"), "{terminal}");
        assert!(!terminal.contains("WARNINGS"), "{terminal}");
    }
    let app = String::from_utf8(run_in(project.path(), ["app", "--color", "never"])).unwrap();
    assert!(!app.contains("app/main.js → core/main.js"), "{app}");
    assert!(
        app.contains("watch circular dependency · app/choice.js"),
        "{app}"
    );
    let core = String::from_utf8(run_in(project.path(), ["core", "--color", "never"])).unwrap();
    assert!(!core.contains("app/main.js → core/main.js"), "{core}");
    assert!(!core.contains("circular dependency"), "{core}");
}

#[test]
fn unresolved_and_ambiguous_relations_reach_a_file_scope_and_all_only() {
    let project = tempfile::tempdir().unwrap();
    fs::create_dir(project.path().join("app")).unwrap();
    fs::write(project.path().join("app/package.json"), "{}\n").unwrap();
    fs::write(
        project.path().join("app/main.js"),
        "import local from './choice';\nimport library from 'external-library';\nconst late = require(moduleName);\nexport function value() { return local + library + late; }\n",
    )
    .unwrap();
    fs::write(project.path().join("app/choice.js"), "export default 1;\n").unwrap();
    fs::write(project.path().join("app/choice.ts"), "export default 2;\n").unwrap();
    // A directory inside the package gives the third scope kind the rule names.
    fs::create_dir(project.path().join("app/inner")).unwrap();
    fs::write(
        project.path().join("app/inner/main.js"),
        "import missing from './missing';\nexport function inner() { return missing; }\n",
    )
    .unwrap();
    git(project.path(), ["init", "-b", "main"]);
    git(project.path(), ["config", "user.name", "Smackdebt Test"]);
    git(
        project.path(),
        ["config", "user.email", "smackdebt@example.invalid"],
    );
    git(project.path(), ["add", "."]);
    git(project.path(), ["commit", "-m", "test: relation-only base"]);
    fs::write(
        project.path().join("app/main.js"),
        "import local from './choice';\nimport library from 'external-library';\nconst late = require(moduleName);\nexport function value() { return local + library + late + 1; }\n",
    )
    .unwrap();

    // A file scope and `--all` state each import that could not be followed;
    // a package and a directory scope keep the grouped sentence alone.
    for (scope, sentence) in [
        ("app", "warning 3 imports could not be followed"),
        ("app/inner", "warning 1 import could not be followed"),
    ] {
        let grouped =
            String::from_utf8(run_in(project.path(), [scope, "--color", "never"])).unwrap();
        assert!(!grouped.contains("\nARCHITECTURE\n"), "{grouped}");
        // The grouped sentence is the whole terminal presence of those
        // imports, so the section states it and nothing per file.
        assert!(grouped.contains("\nWARNINGS\n"), "{grouped}");
        assert!(grouped.contains(sentence), "{grouped}");
        for row in [
            "app/main.js:1 → ./choice",
            "app/main.js:3 → require(",
            "app/inner/main.js:1 → ./missing",
        ] {
            assert!(!grouped.contains(row), "{grouped}");
        }
    }
    // The same directory states its rows once `--all` is asked for.
    let detailed = String::from_utf8(run_in(
        project.path(),
        ["app/inner", "--all", "--color", "never"],
    ))
    .unwrap();
    assert!(
        detailed.contains("app/inner/main.js:1 → ./missing · could not be matched"),
        "{detailed}"
    );

    for (case, arguments) in [
        vec!["app/main.js", "--color", "never"],
        vec!["app", "--all", "--color", "never"],
    ]
    .into_iter()
    .enumerate()
    {
        let output = smackdebt()
            .current_dir(project.path())
            .args(arguments)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "case {case}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let terminal = String::from_utf8(output.stdout).unwrap();
        assert_no_dependency_edge_rows(&terminal, &format!("case {case}"));
        // A per-import row is diagnostic detail, so it joins the grouped
        // sentence it explains rather than a debt section.
        assert!(terminal.contains("\nWARNINGS\n"), "case {case}: {terminal}");
        assert!(
            !terminal.contains("\nARCHITECTURE\n"),
            "case {case}: {terminal}"
        );
        // References outside the repository are not debt, so no external row
        // reaches a human view again.
        assert!(
            !terminal.contains("external-library"),
            "case {case}: {terminal}"
        );
        for relation in [
            "app/main.js:1 → ./choice",
            "app/main.js:3 → require(moduleName)",
        ] {
            assert_eq!(
                terminal.matches(relation).count(),
                1,
                "case {case}: {terminal}"
            );
        }
        // The warning breakdown may repeat a cause's words, so each relation's
        // own status is matched together with its target.
        let choice_status = if case == 0 {
            "./choice · could not be matched"
        } else {
            "./choice · matched more than one file"
        };
        for status in [choice_status, "require(moduleName) · could not be matched"] {
            assert_eq!(
                terminal.matches(status).count(),
                1,
                "case {case}: {terminal}"
            );
        }
    }
}

#[test]
fn static_architecture_diff_snapshot_uses_unchanged_return_edges() {
    let project = static_architecture_fixture();
    fs::write(
        project.path().join("core/main.js"),
        "export default function core(value) {\n  return value;\n}\n",
    )
    .unwrap();
    git(project.path(), ["init", "-b", "main"]);
    git(project.path(), ["config", "user.name", "Smackdebt Test"]);
    git(
        project.path(),
        ["config", "user.email", "smackdebt@example.invalid"],
    );
    git(project.path(), ["add", "."]);
    git(project.path(), ["commit", "-m", "test: acyclic base"]);
    fs::write(
        project.path().join("core/main.js"),
        "import app from '../app/main';\n\nexport default function core(value) {\n  return app(value);\n}\n",
    )
    .unwrap();

    let json = run_in(
        project.path(),
        [
            "diff",
            "main",
            "--json",
            "--jobs",
            "1",
            "--history",
            "36500d",
        ],
    );
    let terminal = run_in(
        project.path(),
        [
            "diff",
            "main",
            "--jobs",
            "1",
            "--color",
            "never",
            "--history",
            "36500d",
        ],
    );
    assert_snapshot(
        "static-architecture-diff.terminal.txt",
        &terminal,
        include_bytes!("snapshots/static-architecture-diff.terminal.txt"),
    );
    assert_snapshot(
        "static-architecture-diff.json",
        &json,
        include_bytes!("snapshots/static-architecture-diff.json"),
    );
    let report: serde_json::Value = serde_json::from_slice(&json).unwrap();
    validate_schema(&report);
    assert!(
        report["architecture_comparisons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|comparison| comparison["kind"] == "cycle_introduced"
                && comparison["direction"] == "worse")
    );
    // The contradiction case: no source comparison moved and the diff is
    // still worse, because the new package cycle is the debt that moved.
    assert!(report["comparisons"].as_array().unwrap().is_empty());
    let text = String::from_utf8(terminal).unwrap();
    assert!(text.contains("Debt increased."), "{text}");
    assert!(
        text.contains("worse 1 (architecture) · better 0 · changed 0"),
        "{text}"
    );
    assert!(
        text.contains("worse package dependency cycle introduced"),
        "{text}"
    );
}

#[test]
fn diff_discloses_current_graph_suppression_without_reporting_reach_movement() {
    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("package.json"), "{}\n").unwrap();
    for index in 0..20 {
        fs::write(
            project.path().join(format!("f{index}.ts")),
            format!("export const f{index} = {index};\n"),
        )
        .unwrap();
    }
    git(project.path(), ["init", "-b", "main"]);
    git(project.path(), ["config", "user.name", "Smackdebt Test"]);
    git(
        project.path(),
        ["config", "user.email", "smackdebt@example.invalid"],
    );
    git(project.path(), ["add", "."]);
    git(project.path(), ["commit", "-m", "test: complete base"]);
    fs::write(
        project.path().join("f1.ts"),
        "import { f0 } from './f0.js';\nimport missing from './missing.js';\nexport const f1 = f0 + missing;\n",
    )
    .unwrap();

    let json = run_in(project.path(), ["diff", "main", "--json", "--jobs", "1"]);
    let terminal = String::from_utf8(run_in(
        project.path(),
        ["diff", "main", "--jobs", "1", "--color", "never"],
    ))
    .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&json).unwrap();
    validate_schema(&report);

    assert_eq!(
        report["diff_graph_evidence"]["current"]["status"],
        "incomplete"
    );
    assert_eq!(report["diff_graph_evidence"]["base"]["status"], "complete");
    assert_eq!(
        report["diff_graph_evidence"]["suppressed_propagation"],
        serde_json::json!({"total": 1, "current": 1, "base": 0})
    );
    assert!(
        report["propagation_comparisons"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(report["summary"]["debt_diff"]["total"], 0);
    assert!(terminal.contains("No debt changed."), "{terminal}");
    assert!(
        terminal.contains(
            "1 architecture comparison hidden because dependency data is incomplete after the change."
        ),
        "{terminal}"
    );
    assert!(!terminal.contains("change reach"), "{terminal}");
}

#[test]
fn static_architecture_removed_cycle_snapshot_is_reviewed() {
    let project = static_architecture_fixture();
    git(project.path(), ["init", "-b", "main"]);
    git(project.path(), ["config", "user.name", "Smackdebt Test"]);
    git(
        project.path(),
        ["config", "user.email", "smackdebt@example.invalid"],
    );
    git(project.path(), ["add", "."]);
    git(project.path(), ["commit", "-m", "test: cyclic base"]);
    fs::write(
        project.path().join("core/main.js"),
        "export default function core(value) {\n  return value;\n}\n",
    )
    .unwrap();

    let json = run_in(
        project.path(),
        [
            "diff",
            "main",
            "--json",
            "--jobs",
            "1",
            "--history",
            "36500d",
        ],
    );
    assert_snapshot(
        "static-architecture-removed-cycle.json",
        &json,
        include_bytes!("snapshots/static-architecture-removed-cycle.json"),
    );
    let report: serde_json::Value = serde_json::from_slice(&json).unwrap();
    validate_schema(&report);
    assert!(
        report["architecture_comparisons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|comparison| comparison["kind"] == "cycle_removed"
                && comparison["direction"] == "better")
    );
}

#[test]
fn evolutionary_analysis_is_exact_private_and_deterministic() {
    let project = evolutionary_fixture();
    let serial_json = run_in(
        project.path(),
        ["--json", "--jobs", "1", "--history", "36500d"],
    );
    let parallel_json = run_in(
        project.path(),
        ["--json", "--jobs", "4", "--history", "36500d"],
    );
    assert_eq!(serial_json, parallel_json);
    let serial_terminal = run_in(
        project.path(),
        ["--jobs", "1", "--color", "never", "--history", "36500d"],
    );
    let parallel_terminal = run_in(
        project.path(),
        ["--jobs", "4", "--color", "never", "--history", "36500d"],
    );
    assert_eq!(serial_terminal, parallel_terminal);
    assert_snapshot(
        "evolutionary-analysis.json",
        &serial_json,
        include_bytes!("snapshots/evolutionary-analysis.json"),
    );
    assert_snapshot(
        "evolutionary-analysis.terminal.txt",
        &serial_terminal,
        include_bytes!("snapshots/evolutionary-analysis.terminal.txt"),
    );
    let report: serde_json::Value = serde_json::from_slice(&serial_json).unwrap();
    validate_schema(&report);
    assert_index_integrity(&report);
    assert_eq!(report["history_coverage"]["availability"], "complete");
    assert_eq!(report["history_coverage"]["commits"], 6);
    assert_eq!(report["change_coupling"][0]["shared_commits"], 3);
    assert_eq!(report["change_coupling"][0]["union_commits"], 6);
    // The operands are published; a consumer derives 3/6 at its own precision.
    assert_eq!(report["change_coupling"][0]["shared_commits"], 3);
    assert_eq!(report["change_coupling"][0]["union_commits"], 6);
    assert_eq!(report["change_coupling"].as_array().unwrap().len(), 1);
    assert_eq!(report["evolutionary_findings"].as_array().unwrap().len(), 1);
    let terminal_text = String::from_utf8(serial_terminal.clone()).unwrap();
    assert_eq!(
        terminal_text
            .matches(
                "  watch packages change together · a ↔ b\n        changed together in 3 of 6 commits · 50% · no code dependency\n"
            )
            .count(),
        1
    );
    assert!(!terminal_text.contains("touches"));
    assert!(!terminal_text.contains("top share"));
    assert!(!terminal_text.contains("a ↔ c"));
    for private_value in [
        "Alice Example",
        "alice@example.invalid",
        "Alias Person",
        "alias@example.invalid",
        "Bob Example",
        "bob@example.invalid",
    ] {
        assert!(
            !serial_json
                .windows(private_value.len())
                .any(|window| window == private_value.as_bytes())
        );
        assert!(
            !serial_terminal
                .windows(private_value.len())
                .any(|window| window == private_value.as_bytes())
        );
    }
}

#[test]
fn an_indirect_dependency_path_is_named_without_suppressing_the_finding() {
    let project = tempfile::tempdir().unwrap();
    git(project.path(), ["init", "-b", "main"]);
    for package in ["a", "b", "c"] {
        fs::create_dir_all(project.path().join(package)).unwrap();
        fs::write(project.path().join(package).join("package.json"), "{}\n").unwrap();
    }
    // The dependency chain runs a → b → c with no direct a → c relation.
    fs::write(
        project.path().join("a/main.js"),
        "import { b } from '../b/main';\nexport const a = b;\n",
    )
    .unwrap();
    fs::write(
        project.path().join("b/main.js"),
        "import { c } from '../c/main';\nexport const b = c;\n",
    )
    .unwrap();
    fs::write(project.path().join("c/main.js"), "export const c = 0;\n").unwrap();
    commit_as(
        project.path(),
        "History Test",
        "history@example.invalid",
        "initial",
    );
    for version in 1..=3 {
        fs::write(
            project.path().join("a/main.js"),
            format!("import {{ b }} from '../b/main';\nexport const a = b + {version};\n"),
        )
        .unwrap();
        fs::write(
            project.path().join("c/main.js"),
            format!("export const c = {version};\n"),
        )
        .unwrap();
        commit_as(
            project.path(),
            "History Test",
            "history@example.invalid",
            &format!("a and c together {version}"),
        );
    }

    let terminal = String::from_utf8(run_in(
        project.path(),
        ["--color", "never", "--history", "36500d"],
    ))
    .unwrap();
    // The Watch finding is created exactly as when no path exists; only the
    // wording distinguishes the indirect link and names the first
    // intermediate on the path.
    assert!(
        terminal.contains(
            "  watch packages change together · a ↔ c\n        changed together in 4 of 4 commits · 100% · no direct dependency · linked via b\n"
        ),
        "{terminal}"
    );
    assert!(!terminal.contains("no code dependency"), "{terminal}");
}

/// Every frozen pattern, every anchor kind, and every evidence kind reaches a
/// committed human view and a committed machine view together.
#[test]
fn every_problem_pattern_reaches_a_committed_terminal_and_machine_view() {
    let project = problem_pattern_fixture();
    let terminal = run_in(
        project.path(),
        [
            "--all",
            "--jobs",
            "1",
            "--color",
            "never",
            "--history",
            "36500d",
        ],
    );
    assert_snapshot(
        "problem-patterns.terminal.txt",
        &terminal,
        include_bytes!("snapshots/problem-patterns.terminal.txt"),
    );
    let json = run_in(
        project.path(),
        ["--json", "--jobs", "1", "--history", "36500d"],
    );
    assert_snapshot(
        "problem-patterns.json",
        &json,
        include_bytes!("snapshots/problem-patterns.json"),
    );
    let report: serde_json::Value = serde_json::from_slice(&json).unwrap();
    validate_schema(&report);
    assert_index_integrity(&report);
    let cards = report["problems"].as_array().unwrap();
    let value = |card: &serde_json::Value, key: &str| card[key].as_str().unwrap().to_owned();
    let patterns: Vec<String> = cards.iter().map(|card| value(card, "pattern")).collect();
    for pattern in ["god_file", "hub", "hot_mess", "bus_risk", "measured"] {
        assert!(patterns.contains(&pattern.to_owned()), "{patterns:?}");
    }
    let anchors: Vec<String> = cards
        .iter()
        .map(|card| value(&card["anchor"], "kind"))
        .collect();
    assert!(anchors.contains(&"package".to_owned()), "{anchors:?}");
    let evidence: Vec<String> = cards
        .iter()
        .flat_map(|card| card["evidence"].as_array().unwrap())
        .map(|fact| value(fact, "kind"))
        .collect();
    for kind in ["fan_in", "fan_out", "hot", "rated_units", "size_findings"] {
        assert!(evidence.contains(&kind.to_owned()), "{evidence:?}");
    }

    let text = String::from_utf8(terminal).unwrap();
    for head in [
        "  high does too much · god/god.js\n",
        "  watch everything depends on this · hub/hub.js\n",
        "  high hot and complex · hot/hot.js\n",
        "  watch one author · hot\n",
    ] {
        assert!(text.contains(head), "{head}: {text}");
    }
    for fact in [
        "        10 files import this\n",
        "        imports 10 files\n",
        "        hot (11 commits)\n",
        "        one contributor made 11 of 11 commits\n",
    ] {
        assert!(text.contains(fact), "{fact}: {text}");
    }
    // A size finding states its subject and its measured value, and a card
    // that heads on one names the file in the identity-then-kind form a
    // finding head takes and states the value alone.
    assert!(
        text.lines()
            .any(|line| line.starts_with("        file · ") && line.ends_with(" lines")),
        "{text}"
    );
    assert!(
        text.contains("  watch god/long.js · file\n        41 lines\n"),
        "{text}"
    );
    // A grouped sentence agrees with its count in its verb and its object.
    assert!(
        text.contains("  warning 2 source files use unsupported languages.\n"),
        "{text}"
    );
    // A warning detail row names its file once: the row states the path and
    // the diagnostic states what happened to it.
    assert!(
        text.contains("  hub/first.go: uses an unsupported language\n"),
        "{text}"
    );
    assert!(!text.contains("hub/first.go: hub/first.go"), "{text}");
    // A recovered file's whole debt is advisory, so its card states that trust
    // and cannot be one of the two names reserved for debt that moves a
    // verdict.
    assert!(
        text.contains("  high recovered · function · advisory · god/recovered.js:1\n"),
        "{text}"
    );

    // The same fixture rendered by default withholds exactly the cards whose
    // debt cannot move a verdict, and states the ones whose debt can.
    let default_view = String::from_utf8(run_in(
        project.path(),
        ["--jobs", "1", "--color", "never", "--history", "36500d"],
    ))
    .unwrap();
    for detail in ["god/recovered.js", "god/long.js"] {
        assert!(!default_view.contains(detail), "{detail}: {default_view}");
    }
    assert!(
        default_view.contains("  high does too much · god/god.js\n"),
        "{default_view}"
    );
    // Every rated unit total reaches a reader with a noun that agrees.
    assert!(text.contains(" rated units\n"), "{text}");
}

#[test]
fn every_actionable_coupling_reaches_a_card_in_the_problem_rank() {
    let project = history_strength_order_fixture();
    let json = run_in(project.path(), ["--json", "--history", "36500d"]);
    let report: serde_json::Value = serde_json::from_slice(&json).unwrap();
    let package_path = |id: u64| report["packages"][id as usize]["path"].as_str().unwrap();
    let stored_pairs = report["evolutionary_findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| {
            (
                package_path(finding["left"].as_u64().unwrap()),
                package_path(finding["right"].as_u64().unwrap()),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        stored_pairs,
        [("a", "b"), ("b", "c"), ("c", "d"), ("d", "e")]
    );

    let terminal = run_in(project.path(), ["--color", "never", "--history", "36500d"]);
    assert_snapshot(
        "history-strength-order.terminal.txt",
        &terminal,
        include_bytes!("snapshots/history-strength-order.terminal.txt"),
    );
    let terminal = String::from_utf8(terminal).unwrap();
    // Every pair is a card ranked against every other problem, so the
    // section-local limit that hid the fourth pair is gone. The four cards
    // tie on every rank key before the anchor, so the anchor package orders
    // them.
    let mut previous = 0;
    for pair in ["a ↔ b", "b ↔ c", "c ↔ d", "d ↔ e"] {
        let at = terminal.find(pair).unwrap_or_else(|| panic!("{terminal}"));
        assert!(at > previous, "{pair}: {terminal}");
        previous = at;
    }
}

#[test]
fn commits_that_landed_outside_a_narrow_window_never_reach_the_report() {
    let project = tempfile::tempdir().unwrap();
    git(project.path(), ["init", "-b", "main"]);
    fs::write(project.path().join("package.json"), "{}\n").unwrap();
    fs::write(project.path().join("main.js"), "export const value = 1;\n").unwrap();
    commit_landed(
        project.path(),
        Some("2000-01-02T03:04:05Z"),
        "far outside every selectable window",
    );
    fs::write(project.path().join("main.js"), "export const value = 2;\n").unwrap();
    commit_landed(project.path(), None, "inside the narrow window");

    let complete = run_in(project.path(), ["--json", "--history", "36500d"]);
    let complete: serde_json::Value = serde_json::from_slice(&complete).unwrap();
    assert_eq!(complete["history_coverage"]["commits"], 2);

    let narrow = run_in(project.path(), ["--json", "--history", "90d"]);
    let narrow: serde_json::Value = serde_json::from_slice(&narrow).unwrap();
    validate_schema(&narrow);
    // The old commit never reaches the report: the streamed set is the
    // windowed set and the boundary check rejects nothing.
    assert_eq!(narrow["history_coverage"]["window_days"], 90);
    assert_eq!(narrow["history_coverage"]["commits"], 1);
    assert_eq!(narrow["history_coverage"]["window_excluded_commits"], 0);
    let touches = |report: &serde_json::Value| {
        report["file_history"]
            .as_array()
            .unwrap()
            .iter()
            .map(|history| history["touches"].as_u64().unwrap())
            .sum::<u64>()
    };
    assert_eq!(touches(&complete), 2);
    assert_eq!(touches(&narrow), 1);
}

#[test]
fn an_empty_history_window_is_disclosed() {
    let project = tempfile::tempdir().unwrap();
    git(project.path(), ["init", "-b", "main"]);
    fs::write(project.path().join("package.json"), "{}\n").unwrap();
    fs::write(
        project.path().join("lib.js"),
        "export function value(item) {\n  return item ? 1 : 2;\n}\n",
    )
    .unwrap();
    fs::write(
        project.path().join("main.js"),
        "import { value } from './lib';\nexport function doubled(item) {\n  return value(item) * 2;\n}\n",
    )
    .unwrap();
    commit_landed(
        project.path(),
        Some("2000-01-02T03:04:05Z"),
        "older than every selectable window",
    );

    let terminal = run_in(
        project.path(),
        ["--color", "never", "--jobs", "1", "--history", "90d"],
    );
    assert_snapshot(
        "empty-history-window.terminal.txt",
        &terminal,
        include_bytes!("snapshots/empty-history-window.terminal.txt"),
    );
    let text = String::from_utf8(terminal).unwrap();
    assert!(text.contains("No commits in the last 90 days."), "{text}");

    let json = run_in(
        project.path(),
        ["--json", "--jobs", "1", "--history", "90d"],
    );
    let report: serde_json::Value = serde_json::from_slice(&json).unwrap();
    validate_schema(&report);
    // The stream is complete; the selected window simply contains nothing.
    assert_eq!(report["history_coverage"]["availability"], "complete");
    assert_eq!(report["history_coverage"]["window_days"], 90);
    assert_eq!(report["history_coverage"]["commits"], 0);
    // Source and static architecture results survive the empty window.
    let root = report["root"].as_u64().unwrap() as usize;
    let coverage = &report["scopes"][root]["coverage"];
    assert_eq!(coverage["selected_files"], 2);
    assert_eq!(coverage["analyzed_files"], 2);
    assert_ne!(report["verdict"]["tier"], "empty");
    assert_eq!(report["dependency_coverage"]["internal"], 1);
}

#[test]
fn diff_uses_history_as_context_and_can_explain_coupling() {
    let project = evolutionary_fixture();
    fs::write(
        project.path().join("b/main.js"),
        "import { a } from '../a/main';\nexport const b = a;\n",
    )
    .unwrap();
    let json = run_in(
        project.path(),
        [
            "diff",
            "main",
            "--json",
            "--jobs",
            "1",
            "--history",
            "36500d",
        ],
    );
    let report: serde_json::Value = serde_json::from_slice(&json).unwrap();
    validate_schema(&report);
    assert_eq!(report["change_coupling"][0]["shared_commits"], 3);
    assert_eq!(report["change_coupling"][0]["union_commits"], 6);
    assert!(
        report["evolutionary_findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        report["evolutionary_comparisons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| { value["kind"] == "finding_removed" && value["direction"] == "better" })
    );
    let default_terminal = String::from_utf8(run_in(
        project.path(),
        ["diff", "main", "--color", "never", "--history", "36500d"],
    ))
    .unwrap();
    assert!(
        default_terminal.contains("Debt increased in some places and decreased in others."),
        "{default_terminal}"
    );
    assert!(
        default_terminal.contains("worse 1 (architecture) · better 1 (evolutionary) · changed 0"),
        "{default_terminal}"
    );
    assert!(
        default_terminal.contains("worse change reaches more code · a · 1 of 3 → 2 of 3"),
        "{default_terminal}"
    );
    assert!(default_terminal.contains("a ↔ b no longer change together without a code dependency"));
    let detailed_terminal = String::from_utf8(run_in(
        project.path(),
        [
            "diff",
            "main",
            "--all",
            "--color",
            "never",
            "--history",
            "36500d",
        ],
    ))
    .unwrap();
    assert!(
        detailed_terminal
            .contains("a ↔ b changed together in 3 of 6 commits · 50% · code dependency exists"),
        "{detailed_terminal}"
    );
    assert!(
        detailed_terminal.contains("a ↔ b no longer change together without a code dependency")
    );
    assert!(!detailed_terminal.contains("coupling finding"));
}

#[test]
fn diff_says_when_packages_now_change_together_without_a_code_dependency() {
    let project = evolutionary_fixture();
    fs::write(
        project.path().join("b/main.js"),
        "import { a } from '../a/main';\nexport const b = a;\n",
    )
    .unwrap();
    commit_as(
        project.path(),
        "History Test",
        "history@example.invalid",
        "explain coupling",
    );
    fs::write(project.path().join("b/main.js"), "export const b = 6;\n").unwrap();

    let terminal = run_in(
        project.path(),
        ["diff", "HEAD", "--color", "never", "--history", "36500d"],
    );
    assert_snapshot(
        "evolutionary-introduced.terminal.txt",
        &terminal,
        include_bytes!("snapshots/evolutionary-introduced.terminal.txt"),
    );
    let terminal = String::from_utf8(terminal).unwrap();
    assert!(terminal.contains("a ↔ b now change together without a code dependency"));
    assert!(!terminal.contains("coupling finding"));
}

#[test]
fn ref_and_worktree_diffs_apply_the_same_coupling_policy() {
    let project = evolutionary_fixture();
    git(project.path(), ["branch", "base"]);
    fs::write(
        project.path().join("b/main.js"),
        "import { a } from '../a/main';\nexport const b = a;\n",
    )
    .unwrap();
    let worktree: serde_json::Value = serde_json::from_slice(&run_in(
        project.path(),
        ["diff", "base", "--json", "--history", "36500d"],
    ))
    .unwrap();
    commit_as(
        project.path(),
        "History Test",
        "history@example.invalid",
        "explain coupling",
    );
    let committed: serde_json::Value = serde_json::from_slice(&run_in(
        project.path(),
        ["diff", "base", "--json", "--history", "36500d"],
    ))
    .unwrap();

    for report in [&worktree, &committed] {
        assert!(
            report["evolutionary_findings"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            report["evolutionary_comparisons"][0]["kind"],
            "finding_removed"
        );
        assert_eq!(report["evolutionary_comparisons"][0]["direction"], "better");
    }
}

#[test]
fn fixture_and_generated_history_stays_descriptive_without_findings() {
    let project = contextual_history_fixture();
    let json = run_in(project.path(), ["--json", "--history", "36500d"]);
    let report: serde_json::Value = serde_json::from_slice(&json).unwrap();
    validate_schema(&report);
    assert_eq!(report["change_coupling"].as_array().unwrap().len(), 3);
    assert!(
        report["evolutionary_findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(report["files"][0]["role"], "primary");
    assert_eq!(report["files"][1]["role"], "fixture");
    assert_eq!(report["files"][2]["role"], "generated");
    assert_eq!(report["file_history"][0]["touches"], 3);
    assert_eq!(report["file_history"][0]["role"], "primary");
    assert_eq!(report["file_history"][0]["trust"], "trusted");
    assert_eq!(report["file_history"][1]["touches"], 3);
    assert_eq!(report["file_history"][1]["role"], "fixture");
    assert_eq!(report["file_history"][2]["touches"], 3);
    assert_eq!(report["file_history"][2]["role"], "generated");
    assert_eq!(report["history_coverage"]["eligible_commits"], 3);
    assert_eq!(report["history_coverage"]["mapped_eligible_changes"], 3);
    assert_eq!(report["history_coverage"]["context_changes"], 6);
    assert_eq!(report["change_coupling"][0]["left_role"], "primary");
    assert_eq!(report["change_coupling"][0]["right_role"], "fixture");

    let default = String::from_utf8(run_in(
        project.path(),
        ["--color", "never", "--history", "36500d"],
    ))
    .unwrap();
    assert!(!default.contains("a ↔ b"), "{default}");
    assert!(!default.contains("a ↔ c"), "{default}");
    // Codebase debt is problem cards, and only an actionable finding makes
    // one, so a pair a fixture or generated role explains has no card at any
    // detail level and stays a complete row in the machine report.
    let detailed = String::from_utf8(run_in(
        project.path(),
        ["--all", "--color", "never", "--history", "36500d"],
    ))
    .unwrap();
    // The substring covers both co-change patterns, so neither the package
    // wording nor the file wording can return with a fixture-explained pair.
    assert!(!detailed.contains("change together"), "{detailed}");
    assert!(!detailed.contains("a ↔ b"), "{detailed}");
    assert!(!detailed.contains("a ↔ c"), "{detailed}");
}

#[test]
fn generated_history_cannot_change_eligible_history_or_concentration() {
    let project = generated_heavy_history_fixture();
    let report: serde_json::Value =
        serde_json::from_slice(&run_in(project.path(), ["--json", "--history", "36500d"])).unwrap();
    validate_schema(&report);
    assert_index_integrity(&report);
    let diff_report: serde_json::Value = serde_json::from_slice(&run_in(
        project.path(),
        ["diff", "HEAD", "--json", "--history", "36500d"],
    ))
    .unwrap();
    for field in [
        "history_coverage",
        "file_history",
        "change_coupling",
        "contributor_concentration",
        "evolutionary_findings",
    ] {
        assert_eq!(
            report[field], diff_report[field],
            "{field} differs in diff mode"
        );
    }
    let nonempty_package_history = |value: &serde_json::Value| {
        value["package_history"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["touches"] != 0)
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(
        nonempty_package_history(&report),
        nonempty_package_history(&diff_report)
    );

    let coverage = &report["history_coverage"];
    assert_eq!(coverage["commits"], 10);
    assert_eq!(coverage["eligible_commits"], 3);
    assert_eq!(coverage["mapped_eligible_changes"], 6);
    assert_eq!(coverage["context_changes"], 16);
    assert_eq!(coverage["excluded_changes"], 3);
    assert_eq!(coverage["textual_changes"], 22);
    assert_eq!(coverage["uncounted_changes"], 0);

    let package_history = report["package_history"].as_array().unwrap();
    for package in 0..=1 {
        let eligible = package_history
            .iter()
            .find(|row| row["package"] == package && row["role"] == "primary")
            .unwrap();
        let context = package_history
            .iter()
            .find(|row| row["package"] == package && row["role"] == "generated")
            .unwrap();
        assert_eq!(eligible["touches"], 3);
        assert_eq!(context["touches"], 8);
    }
    let concentration = report["contributor_concentration"].as_array().unwrap();
    for package in 0..=1 {
        let eligible = concentration
            .iter()
            .find(|row| row["package"] == package && row["role"] == "primary")
            .unwrap();
        let context = concentration
            .iter()
            .find(|row| row["package"] == package && row["role"] == "generated")
            .unwrap();
        assert_eq!(eligible["contributor_count"], 1);
        assert_eq!(eligible["numerator"], 3);
        assert_eq!(eligible["denominator"], 3);
        assert_eq!(context["contributor_count"], 8);
        assert_eq!(context["numerator"], 1);
        assert_eq!(context["denominator"], 8);
    }
    let coupling = report["change_coupling"].as_array().unwrap();
    assert_eq!(coupling.len(), 1, "{coupling:?}");
    assert_eq!(coupling[0]["left_role"], "primary");
    assert_eq!(coupling[0]["right_role"], "primary");
    assert_eq!(coupling[0]["shared_commits"], 10);
    assert_eq!(coupling[0]["union_commits"], 10);
    assert_eq!(report["evolutionary_findings"][0]["shared_commits"], 3);
    assert_eq!(report["evolutionary_findings"][0]["union_commits"], 3);

    let default = String::from_utf8(run_in(
        project.path(),
        ["--color", "never", "--history", "36500d"],
    ))
    .unwrap();
    assert!(!default.contains("eligible mapping"));
    assert!(!default.contains("touches"));
    assert!(!default.contains("top share"));
    assert!(!default.contains("generated/trusted"));
    assert!(!default.contains("a · 8 touches"));

    let detailed = String::from_utf8(run_in(
        project.path(),
        ["--all", "--color", "never", "--history", "36500d"],
    ))
    .unwrap();
    // Churn dumps left the terminal; the actionable coupling finding stayed.
    assert!(!detailed.contains("a · 8 commits"), "{detailed}");
    assert!(!detailed.contains("shared commits"));
    assert!(
        detailed.contains(
            "  watch packages change together · a ↔ b\n        changed together in 3 of 3 commits · 100% · no code dependency\n"
        ),
        "{detailed}"
    );
}

#[test]
fn complete_stream_without_eligible_mapping_cannot_create_findings() {
    let project = generated_only_history_fixture();
    let report: serde_json::Value =
        serde_json::from_slice(&run_in(project.path(), ["--json", "--history", "36500d"])).unwrap();
    assert_eq!(report["history_coverage"]["availability"], "complete");
    assert_eq!(report["history_coverage"]["eligible_commits"], 0);
    assert_eq!(report["history_coverage"]["mapped_eligible_changes"], 0);
    assert_eq!(report["history_coverage"]["context_changes"], 6);
    assert!(
        report["evolutionary_findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(report["change_coupling"][0]["shared_commits"], 3);

    let terminal = String::from_utf8(run_in(
        project.path(),
        ["--all", "--color", "never", "--history", "36500d"],
    ))
    .unwrap();
    assert!(!terminal.contains("complete local stream"));
    assert!(!terminal.contains("eligible mapping"));
    assert!(!terminal.contains(''));
}

#[test]
fn package_churn_stays_in_json_instead_of_repeating_in_the_terminal() {
    let project = eligible_role_history_fixture();
    let default = String::from_utf8(run_in(
        project.path(),
        ["--color", "never", "--history", "36500d"],
    ))
    .unwrap();
    assert!(!default.contains("HISTORY"));

    let detailed = String::from_utf8(run_in(
        project.path(),
        ["--all", "--color", "never", "--history", "36500d"],
    ))
    .unwrap();
    assert!(!detailed.contains("a · 2 commits"), "{detailed}");
    let report: serde_json::Value =
        serde_json::from_slice(&run_in(project.path(), ["--json", "--history", "36500d"])).unwrap();
    assert_eq!(
        report["package_history"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["touches"] == 2)
            .count(),
        2
    );
}

#[test]
fn weak_coupling_stays_in_json_only() {
    let project = weak_history_fixture();
    let report: serde_json::Value =
        serde_json::from_slice(&run_in(project.path(), ["--json", "--history", "36500d"])).unwrap();
    assert_eq!(report["change_coupling"].as_array().unwrap().len(), 1);
    assert_eq!(report["change_coupling"][0]["shared_commits"], 2);
    assert_eq!(report["change_coupling"][0]["union_commits"], 4);
    assert!(
        report["evolutionary_findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    for terminal in [
        run_in(project.path(), ["--color", "never", "--history", "36500d"]),
        run_in(
            project.path(),
            ["--all", "--color", "never", "--history", "36500d"],
        ),
        run_in(
            project.path(),
            ["a", "--all", "--color", "never", "--history", "36500d"],
        ),
    ] {
        let terminal = String::from_utf8(terminal).unwrap();
        assert!(!terminal.contains("a ↔ b"), "{terminal}");
    }
}

#[test]
fn a_selected_package_states_actionable_history_only() {
    let project = evolutionary_fixture();
    fs::write(
        project.path().join("b/main.js"),
        "import { a } from '../a/main';\nexport const b = a;\n",
    )
    .unwrap();
    let terminal = run_in(
        project.path(),
        ["a", "--all", "--color", "never", "--history", "36500d"],
    );
    assert_snapshot(
        "evolutionary-package.terminal.txt",
        &terminal,
        include_bytes!("snapshots/evolutionary-package.terminal.txt"),
    );
    let text = String::from_utf8(terminal).unwrap();
    assert!(!text.contains("a/main.js · 5 commits"), "{text}");
    // Codebase debt is problem cards, and a pair a code dependency explains
    // is not a finding, so nothing states it here; the complete pair table
    // stays in the machine report.
    assert!(!text.contains("a ↔ b"), "{text}");
    assert!(!text.contains("c/main.js"));
    assert!(!text.contains("  c ·"));
    let report: serde_json::Value = serde_json::from_slice(&run_in(
        project.path(),
        ["a", "--json", "--history", "36500d"],
    ))
    .unwrap();
    assert!(report["change_coupling"].as_array().unwrap().is_empty());
}

#[test]
fn selected_diff_package_shows_only_relevant_evolution_context() {
    let project = evolutionary_fixture();
    fs::write(
        project.path().join("b/main.js"),
        "import { a } from '../a/main';\nexport const b = a;\n",
    )
    .unwrap();
    let terminal = run_in(
        project.path(),
        [
            "diff",
            "main",
            "b",
            "--all",
            "--color",
            "never",
            "--history",
            "36500d",
        ],
    );
    assert_snapshot(
        "evolutionary-diff-package.terminal.txt",
        &terminal,
        include_bytes!("snapshots/evolutionary-diff-package.terminal.txt"),
    );
    let text = String::from_utf8(terminal).unwrap();
    assert!(!text.contains("b · 4 commits"), "{text}");
    assert!(!text.contains("b/main.js · 4 commits"), "{text}");
    assert!(
        text.contains("a ↔ b changed together in 3 of 6 commits · 50% · code dependency exists"),
        "{text}"
    );
    assert!(text.contains("a ↔ b no longer change together without a code dependency"));
    assert!(!text.contains("c/main.js"));
    assert!(!text.contains("  c ·"));
}

#[test]
fn empty_git_history_is_unavailable_in_coverage_terminal_and_diagnostics() {
    let project = fixture();
    git(project.path(), ["init", "-b", "main"]);
    let json = run_in(project.path(), ["--json", "--jobs", "1"]);
    let report: serde_json::Value = serde_json::from_slice(&json).unwrap();
    validate_schema(&report);
    assert_eq!(report["history_coverage"]["availability"], "unavailable");
    assert_eq!(
        report["history_coverage"]["reason"],
        "repository has no commits"
    );
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| {
                diagnostic["message"] == "Git history unavailable: repository has no commits"
            })
    );
    let terminal = String::from_utf8(run_in(project.path(), ["--color", "never"])).unwrap();
    assert!(terminal.contains("WARNINGS"));
    assert!(terminal.contains("History is unavailable."));
    assert!(!terminal.contains("History is incomplete."));
}

#[test]
fn shallow_history_is_reported_as_incomplete() {
    let checkout = shallow_evolutionary_fixture();
    let json = run_in(
        checkout.path(),
        ["--json", "--jobs", "1", "--history", "36500d"],
    );
    let report: serde_json::Value = serde_json::from_slice(&json).unwrap();
    validate_schema(&report);
    assert_eq!(report["history_coverage"]["availability"], "incomplete");
    assert_eq!(
        report["history_coverage"]["reason"],
        "repository history is shallow"
    );
    assert_eq!(report["history_coverage"]["commits"], 6);
    assert!(report["history_coverage"].get("revision").is_some());
    assert!(report["history_coverage"].get("newest_timestamp").is_some());
    assert!(report["history_coverage"].get("oldest_timestamp").is_some());
    assert!(report["history_coverage"].get("textual_changes").is_some());
    assert!(
        report["history_coverage"]
            .get("uncounted_changes")
            .is_some()
    );
    assert!(report["history_coverage"].get("eligible_commits").is_some());
    assert!(
        report["history_coverage"]
            .get("mapped_eligible_changes")
            .is_some()
    );
    assert!(report["history_coverage"].get("context_changes").is_some());
    assert!(report["history_coverage"].get("excluded_changes").is_some());
    assert!(report["history_coverage"].get("rename_gaps").is_some());
    assert!(
        report["evolutionary_findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(!report["files"].as_array().unwrap().is_empty());
}

#[test]
fn changed_dependency_with_shallow_history_shows_only_the_relevant_trust_warning() {
    let checkout = shallow_evolutionary_fixture();
    fs::write(
        checkout.path().join("b/main.test.js"),
        "import { a } from '../a/main';\nexport const expected = a;\n",
    )
    .unwrap();

    let terminal = String::from_utf8(run_in(
        checkout.path(),
        ["diff", "HEAD", "--color", "never", "--history", "36500d"],
    ))
    .unwrap();
    assert!(terminal.contains("No debt changed."), "{terminal}");
    assert!(
        terminal.contains("warning History is incomplete."),
        "{terminal}"
    );
    assert!(!terminal.contains("HISTORY"), "{terminal}");
    assert!(!terminal.contains("changed together"), "{terminal}");
    assert!(
        terminal.ends_with("  inspect directories and files for more details\n"),
        "{terminal}"
    );
}

/// A path-selected machine head answers the question the terminal answers for
/// the same invocation, so `--json | head` and the human report never
/// disagree about which scope was asked about.
#[test]
fn a_path_selected_json_head_answers_the_selected_scope_like_the_terminal() {
    let project = split_debt_fixture();
    let clean = project.path().join("clean");
    let clean = clean.to_str().unwrap();
    let repository: serde_json::Value =
        serde_json::from_slice(&run(["--json", project.path().to_str().unwrap()])).unwrap();
    let selected: serde_json::Value = serde_json::from_slice(&run(["--json", clean])).unwrap();
    let terminal = String::from_utf8(run(["--color", "never", "--all", clean])).unwrap();

    assert_ne!(
        selected["verdict"]["tier"], repository["verdict"]["tier"],
        "the head answers the selected scope, not the repository"
    );
    assert!(
        terminal.contains(selected["verdict"]["sentence"].as_str().unwrap()),
        "{terminal}"
    );
    let summary = &selected["summary"];
    let counts = format!(
        "{} high · {} watch · {} checked",
        summary["high"].as_u64().unwrap(),
        summary["watch"].as_u64().unwrap(),
        summary["checked"].as_u64().unwrap()
    );
    assert!(terminal.contains(&counts), "{terminal}");
    match summary["worst"].as_array().unwrap().first() {
        Some(worst) => assert!(
            terminal.contains(&format!("worst: {}", worst["path"].as_str().unwrap())),
            "{terminal}"
        ),
        None => assert!(!terminal.contains("worst: "), "{terminal}"),
    }
    validate_schema(&selected);
    assert_index_integrity(&selected);
}

/// A reader that stops reading, which is what `| head` is, ends the run
/// quietly the way every other Unix tool ends it.
#[test]
fn a_report_piped_into_head_is_quiet_and_successful() {
    // The report has to outgrow one buffered write, which is when the reader
    // has already left by the time the next write happens.
    let project = tangled_fixture(400);
    let binary = assert_cmd::cargo::cargo_bin("smackdebt");
    let mut shell = Command::new("sh");
    // The pipeline inherits the pinned environment through the shell.
    hermetic_env(&mut shell);
    let output = shell
        .arg("-c")
        .arg(format!(
            "'{}' --all '{}' | head -1",
            binary.display(),
            project.path().display()
        ))
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout).starts_with("smackdebt · "),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

fn assert_snapshot(name: &str, actual: &[u8], expected: &[u8]) {
    if name.ends_with(".terminal.txt") {
        assert_no_dependency_edge_rows(&String::from_utf8_lossy(actual), name);
    }
    if std::env::var_os("SMACKDEBT_UPDATE_SNAPSHOTS").is_some() {
        assert_eq!(
            std::env::var_os("SMACKDEBT_UPDATE_COMMAND").as_deref(),
            Some(std::ffi::OsStr::new("1")),
            "snapshot updates require the named developer command"
        );
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
        serde_json::from_str(include_str!("../../../schemas/report-v4.schema.json")).unwrap();
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(report)
        .unwrap();
}

fn validate_gate_schema(result: &serde_json::Value) {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../schemas/gate-v1.schema.json")).unwrap();
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(result)
        .unwrap();
}

/// The denormalized head duplicates table facts on purpose, so every value in
/// it is compared with the table it came from.
fn assert_head_agrees_with_tables(report: &serde_json::Value) {
    let mode = report["mode"].as_str().unwrap();
    assert_eq!(report["verdict"]["mode"], mode);
    assert!(!report["verdict"]["tier"].as_str().unwrap().is_empty());
    assert!(
        report["verdict"]["sentence"]
            .as_str()
            .unwrap()
            .ends_with('.')
    );
    let summary = &report["summary"];
    // The head answers the selected scope, which is the repository unless a
    // path was asked about.
    let Some(answered) = report["selected_scope"]
        .as_u64()
        .or_else(|| report["root"].as_u64())
        .map(|scope| scope as usize)
    else {
        assert_eq!(summary["checked"], 0);
        return;
    };
    let health = report["scopes"][answered]["health"].as_u64().unwrap() as usize;
    let coverage = &report["scopes"][answered]["coverage"];
    match report["verdict"].get("qualifier") {
        Some(qualifier) => {
            assert_eq!(qualifier["selected_files"], coverage["selected_files"]);
            assert_eq!(qualifier["analyzed_files"], coverage["analyzed_files"]);
            assert!(
                coverage["analyzed_files"].as_u64().unwrap()
                    < coverage["selected_files"].as_u64().unwrap()
            );
        }
        None => assert_eq!(coverage["analyzed_files"], coverage["selected_files"]),
    }
    let counts = &report["health"][health];
    assert_eq!(summary["high"], counts["high"]);
    assert_eq!(summary["watch"], counts["watch"]);
    assert_eq!(
        summary["checked"].as_u64().unwrap(),
        counts["healthy"].as_u64().unwrap()
            + counts["watch"].as_u64().unwrap()
            + counts["high"].as_u64().unwrap()
    );
    let architecture = report["scopes"][answered]["architecture_findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|id| {
            report["architecture_findings"][id.as_u64().unwrap() as usize]["rating"] == "high"
        })
        .count() as u64;
    assert_eq!(summary["high_architecture"].as_u64(), Some(architecture));
    let paths: Vec<&str> = report["paths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|path| path.as_str().unwrap())
        .collect();
    let worst = summary["worst"].as_array().unwrap();
    assert!(worst.len() <= 3);
    for offender in worst {
        let path = offender["path"].as_str().unwrap();
        assert!(
            paths.contains(&path),
            "worst path {path} is not a real path"
        );
    }
}

fn assert_index_integrity(report: &serde_json::Value) {
    let paths = report["paths"].as_array().unwrap();
    let scopes = report["scopes"].as_array().unwrap();
    let files = report["files"].as_array().unwrap();
    let findings = report["findings"].as_array().unwrap();
    let comparisons = report["comparisons"].as_array().unwrap();
    let health = report["health"].as_array().unwrap();
    let activity = report["activity"].as_array().unwrap();
    let dependency_edges = report["dependency_edges"].as_array().unwrap();
    let package_edges = report["package_edges"].as_array().unwrap();
    let architecture_findings = report["architecture_findings"].as_array().unwrap();
    let architecture_comparisons = report["architecture_comparisons"].as_array().unwrap();
    let file_history = report["file_history"].as_array().unwrap();
    let package_history = report["package_history"].as_array().unwrap();
    let change_coupling = report["change_coupling"].as_array().unwrap();
    let contributor_concentration = report["contributor_concentration"].as_array().unwrap();
    let evolutionary_findings = report["evolutionary_findings"].as_array().unwrap();
    let evolutionary_comparisons = report["evolutionary_comparisons"].as_array().unwrap();
    let package_graph = report["package_graph"].as_array().unwrap();
    let external_dependencies = report["external_dependencies"].as_array().unwrap();
    let resolution_diagnostics = report["resolution_diagnostics"].as_array().unwrap();

    assert_head_agrees_with_tables(report);
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
        for finding in scope["architecture_findings"].as_array().unwrap() {
            assert!((finding.as_u64().unwrap() as usize) < architecture_findings.len());
        }
        for comparison in scope["architecture_comparisons"].as_array().unwrap() {
            assert!((comparison.as_u64().unwrap() as usize) < architecture_comparisons.len());
        }
        for finding in scope["evolutionary_findings"].as_array().unwrap() {
            assert!((finding.as_u64().unwrap() as usize) < evolutionary_findings.len());
        }
        for comparison in scope["evolutionary_comparisons"].as_array().unwrap() {
            assert!((comparison.as_u64().unwrap() as usize) < evolutionary_comparisons.len());
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
        if let Some(package) = file["package"].as_u64() {
            assert!((package as usize) < package_graph.len());
        }
    }
    for (index, finding) in findings.iter().enumerate() {
        assert_eq!(finding["id"], index);
        assert!((finding["file"].as_u64().unwrap() as usize) < files.len());
        assert!(finding["kind"].is_string());
    }
    for (index, comparison) in comparisons.iter().enumerate() {
        assert_eq!(comparison["id"], index);
        assert!(comparison["unit_kind"].is_string());
        assert!(matches!(
            comparison["participation"].as_str(),
            Some("verdict" | "context")
        ));
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
    for (index, edge) in dependency_edges.iter().enumerate() {
        assert_eq!(edge["id"], index);
        assert!((edge["source"].as_u64().unwrap() as usize) < files.len());
        assert!((edge["target"].as_u64().unwrap() as usize) < files.len());
    }
    for (index, edge) in package_edges.iter().enumerate() {
        assert_eq!(edge["id"], index);
        assert!((edge["source"].as_u64().unwrap() as usize) < package_graph.len());
        assert!((edge["target"].as_u64().unwrap() as usize) < package_graph.len());
        for file_edge in edge["file_edges"].as_array().unwrap() {
            assert!((file_edge.as_u64().unwrap() as usize) < dependency_edges.len());
        }
    }
    for (index, finding) in architecture_findings.iter().enumerate() {
        assert_eq!(finding["id"], index);
        for file in finding["files"].as_array().unwrap() {
            assert!((file.as_u64().unwrap() as usize) < files.len());
        }
        for package in finding["packages"].as_array().unwrap() {
            assert!((package.as_u64().unwrap() as usize) < package_graph.len());
        }
        for edge in finding["witness_edges"].as_array().unwrap() {
            assert!((edge.as_u64().unwrap() as usize) < dependency_edges.len());
        }
    }
    for (index, comparison) in architecture_comparisons.iter().enumerate() {
        assert_eq!(comparison["id"], index);
        for field in ["packages", "witness"] {
            for package in comparison[field].as_array().unwrap() {
                assert!((package.as_u64().unwrap() as usize) < package_graph.len());
            }
        }
        for file in comparison["files"].as_array().unwrap() {
            assert!((file.as_u64().unwrap() as usize) < files.len());
        }
    }
    for (index, measurement) in package_graph.iter().enumerate() {
        assert_eq!(measurement["package"], index);
    }
    assert_eq!(file_history.len(), files.len());
    let mut files_with_history = HashSet::new();
    for history in file_history {
        let file = history["file"].as_u64().unwrap() as usize;
        assert!(file < files.len());
        assert!(
            files_with_history.insert(file),
            "duplicate file history row"
        );
    }
    assert_eq!(files_with_history.len(), files.len());

    let mut packages_with_history = HashSet::new();
    let mut package_history_keys = HashSet::new();
    for history in package_history {
        let package = history["package"].as_u64().unwrap() as usize;
        assert!(package < package_graph.len());
        let role = history["role"].as_str().unwrap();
        let trust = history["trust"].as_str().unwrap();
        assert!(
            package_history_keys.insert((package, role, trust)),
            "duplicate package history evidence row"
        );
        packages_with_history.insert(package);
    }
    assert_eq!(packages_with_history.len(), package_graph.len());

    let mut coupling_pairs = HashSet::new();
    for coupling in change_coupling {
        let left = coupling["left"].as_u64().unwrap() as usize;
        let right = coupling["right"].as_u64().unwrap() as usize;
        assert!(left < package_graph.len());
        assert!(right < package_graph.len());
        assert!(left < right, "coupling pair identity is not stable");
        let left_role = coupling["left_role"].as_str().unwrap();
        let left_trust = coupling["left_trust"].as_str().unwrap();
        let right_role = coupling["right_role"].as_str().unwrap();
        let right_trust = coupling["right_trust"].as_str().unwrap();
        assert!(
            coupling_pairs.insert((left, right, left_role, left_trust, right_role, right_trust,)),
            "duplicate coupling evidence pair"
        );
    }

    // A retained file pair names the lower file identity first, crosses a
    // directory boundary, and compares two counts drawn from one population.
    let mut previous_pair = None;
    for pair in report["file_change_coupling"].as_array().unwrap() {
        let left = pair["left"].as_u64().unwrap();
        let right = pair["right"].as_u64().unwrap();
        assert!((left as usize) < files.len(), "pair left index is invalid");
        assert!(
            (right as usize) < files.len(),
            "pair right index is invalid"
        );
        assert!(left < right, "a pair names the lower file identity first");
        let key = Some((left, right));
        assert!(previous_pair < key, "pairs are ordered by file identity");
        previous_pair = key;
        let shared = pair["shared_commits"].as_u64().unwrap();
        let union = pair["union_commits"].as_u64().unwrap();
        assert!(shared >= 3, "a retained pair clears the support floor");
        assert!(shared <= union, "shared commits are part of the union");
        assert!(shared * 10 >= union, "a retained pair clears one tenth");
        assert!(
            pair["distance"].as_u64().unwrap() >= 1,
            "a same-directory pair is never stored"
        );
        assert!(pair.get("similarity").is_none(), "no ratio is serialized");
    }

    let mut concentration_packages = HashSet::new();
    for concentration in contributor_concentration {
        let package = concentration["package"].as_u64().unwrap() as usize;
        assert!(package < package_graph.len());
        let role = concentration["role"].as_str().unwrap();
        let trust = concentration["trust"].as_str().unwrap();
        assert!(
            concentration_packages.insert((package, role, trust)),
            "duplicate concentration evidence row"
        );
    }

    for (index, finding) in evolutionary_findings.iter().enumerate() {
        assert_eq!(finding["id"], index);
        let left = finding["left"].as_u64().unwrap() as usize;
        let right = finding["right"].as_u64().unwrap() as usize;
        assert!(left < package_graph.len());
        assert!(right < package_graph.len());
        assert!(left < right);
        assert!(finding["shared_commits"].as_u64().unwrap() >= 3);
        assert_eq!(finding["kind"], "unexplained_coupling");
    }
    for (index, comparison) in evolutionary_comparisons.iter().enumerate() {
        assert_eq!(comparison["id"], index);
        let left = comparison["left"].as_u64().unwrap() as usize;
        let right = comparison["right"].as_u64().unwrap() as usize;
        assert!(left < package_graph.len());
        assert!(right < package_graph.len());
        assert!(left < right);
        assert!(comparison["shared_commits"].as_u64().unwrap() >= 3);
    }
    for dependency in external_dependencies {
        assert!((dependency["file"].as_u64().unwrap() as usize) < files.len());
    }
    for diagnostic in resolution_diagnostics {
        assert!((diagnostic["file"].as_u64().unwrap() as usize) < files.len());
    }
}

#[test]
fn index_audit_rejects_a_nested_architecture_reference() {
    let project = static_architecture_fixture();
    let bytes = run(["--json", project.path().to_str().unwrap()]);
    let mut report: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(
        !report["architecture_findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        !report["architecture_findings"][0]["witness_edges"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    report["architecture_findings"][0]["witness_edges"][0] = 999_999.into();
    assert!(std::panic::catch_unwind(|| assert_index_integrity(&report)).is_err());
}

#[test]
fn index_audit_rejects_a_nested_evolution_reference() {
    let project = evolutionary_fixture();
    let bytes = run([
        "--json",
        "--history",
        "36500d",
        project.path().to_str().unwrap(),
    ]);
    let mut report: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(
        !report["evolutionary_findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    report["evolutionary_findings"][0]["right"] = 999_999.into();
    assert!(std::panic::catch_unwind(|| assert_index_integrity(&report)).is_err());
}

#[test]
fn report_schema_has_no_contributor_identity_fields_or_tables() {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../schemas/report-v4.schema.json")).unwrap();
    assert_schema_omits_identity_keys(&schema);
}

fn assert_schema_omits_identity_keys(value: &serde_json::Value) {
    const FORBIDDEN_KEYS: &[&str] = &[
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
    match value {
        serde_json::Value::Object(object) => {
            for (key, child) in object {
                assert!(
                    !FORBIDDEN_KEYS.contains(&key.as_str()),
                    "schema exposes contributor identity key {key}"
                );
                assert_schema_omits_identity_keys(child);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                assert_schema_omits_identity_keys(item);
            }
        }
        _ => {}
    }
}

#[test]
fn json_schema_rejects_nested_field_type_drift() {
    let project = fixture();
    let bytes = run(["--json", project.path().to_str().unwrap()]);
    let mut report: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    report["scopes"][0]["coverage"]["selected_files"] = "one".into();
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../schemas/report-v4.schema.json")).unwrap();
    assert!(
        jsonschema::validator_for(&schema)
            .unwrap()
            .validate(&report)
            .is_err()
    );
}

#[test]
fn json_schema_rejects_weak_evolution_verdict_thresholds() {
    let project = evolutionary_fixture();
    let finding_report: serde_json::Value =
        serde_json::from_slice(&run_in(project.path(), ["--json", "--history", "36500d"])).unwrap();
    assert!(
        !finding_report["evolutionary_findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    fs::write(
        project.path().join("b/main.js"),
        "import { a } from '../a/main';\nexport const b = a;\n",
    )
    .unwrap();
    let comparison_report: serde_json::Value = serde_json::from_slice(&run_in(
        project.path(),
        ["diff", "main", "--json", "--history", "36500d"],
    ))
    .unwrap();
    assert!(
        !comparison_report["evolutionary_comparisons"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../schemas/report-v4.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for (table, report) in [
        ("evolutionary_findings", finding_report),
        ("evolutionary_comparisons", comparison_report),
    ] {
        let mut weak_commits = report.clone();
        weak_commits[table][0]["shared_commits"] = 2.into();
        assert!(validator.validate(&weak_commits).is_err(), "{table}");

        // Version 4 publishes operands only, so a re-added float is rejected.
        let mut float_operand = report;
        float_operand[table][0]["similarity"] = 0.19.into();
        assert!(validator.validate(&float_operand).is_err(), "{table}");
    }
}

/// Every private-use codepoint, which may never reach a machine consumer.
fn private_use_codepoints(value: &[u8]) -> Vec<char> {
    String::from_utf8_lossy(value)
        .chars()
        .filter(|character| ('\u{e000}'..='\u{f8ff}').contains(character))
        .collect()
}

/// Removes each decoration, restoring the cells it filled.
fn strip_decorations(value: &[u8]) -> Vec<u8> {
    String::from_utf8_lossy(value)
        .chars()
        .map(|character| {
            // A decoration fills the two cells an undecorated report leaves
            // blank, so removing it restores those two spaces.
            if ('\u{e000}'..='\u{f8ff}').contains(&character) || character == '\u{258c}' {
                ' '
            } else {
                character
            }
        })
        .collect::<String>()
        .into_bytes()
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

/// The built command with its environment pinned to the hermetic home.
fn smackdebt() -> assert_cmd::Command {
    let mut command = Command::new(assert_cmd::cargo::cargo_bin("smackdebt"));
    hermetic_env(&mut command);
    assert_cmd::Command::from_std(command)
}

fn run<const N: usize>(arguments: [&str; N]) -> Vec<u8> {
    let stdout = smackdebt().args(arguments).output().unwrap().stdout;
    // Every human result this suite produces carries the invariant, so a flow
    // without a committed result cannot reintroduce edge rows either.
    if !arguments.contains(&"--json") {
        assert_no_dependency_edge_rows(
            &String::from_utf8_lossy(&stdout),
            &format!("{arguments:?}"),
        );
    }
    stdout
}

fn run_in<const N: usize>(directory: &Path, arguments: [&str; N]) -> Vec<u8> {
    let stdout = smackdebt()
        .current_dir(directory)
        .args(arguments)
        .output()
        .unwrap()
        .stdout;
    // Every human result this suite produces carries the invariant, so a flow
    // without a committed result cannot reintroduce edge rows either.
    if !arguments.contains(&"--json") {
        assert_no_dependency_edge_rows(
            &String::from_utf8_lossy(&stdout),
            &format!("{arguments:?}"),
        );
    }
    stdout
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

/// One kept candidate beside both nested-checkout shapes: an embedded clone
/// with a `.git` directory and a linked worktree with a `.git` file.
fn nested_checkout_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("kept.js"), "export const kept = 1;\n").unwrap();
    fs::create_dir_all(project.path().join("clone/.git")).unwrap();
    fs::write(
        project.path().join("clone/lost.js"),
        "export const lost = 1;\n",
    )
    .unwrap();
    fs::create_dir_all(project.path().join("worktree")).unwrap();
    fs::write(
        project.path().join("worktree/.git"),
        "gitdir: /elsewhere/.git/worktrees/pr-48\n",
    )
    .unwrap();
    fs::write(
        project.path().join("worktree/lost.js"),
        "export const lost = 1;\n",
    )
    .unwrap();
    project
}

/// One supported JavaScript file of 39 bytes beside one unsupported Go file
/// of 65 bytes, so the unsupported byte share is exactly 625 permille.
fn unsupported_share_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("kept.js"),
        "export function kept() {\n  return 1;\n}\n",
    )
    .unwrap();
    fs::write(
        project.path().join("main.go"),
        "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"debt\")\n}\n",
    )
    .unwrap();
    project
}

fn static_architecture_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    for path in [
        "app/package.json",
        "app/main.js",
        "app/choice.js",
        "app/choice.ts",
        "app/helper.js",
        "core/package.json",
        "core/main.js",
        "native/Cargo.toml",
        "native/src/lib.rs",
        "native/src/helper.rs",
    ] {
        let source = Path::new(STATIC_ARCHITECTURE_FIXTURE).join(path);
        let target = project.path().join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(source, target).unwrap();
    }
    project
}

fn long_responsive_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    let source_package = "very-long-source-package-name";
    let source_directory = "very-long-source-directory-name";
    let source_file = "very-long-source-file-name.js";
    let target_package = "very-long-target-package-name";
    let target_directory = "very-long-target-directory-name";
    let target_file = "very-long-target-file-name.js";
    let source_path = format!("{source_package}/{source_directory}/{source_file}");
    let target_path = format!("{target_package}/{target_directory}/{target_file}");
    for package in [source_package, target_package] {
        fs::create_dir_all(project.path().join(package)).unwrap();
        fs::write(project.path().join(package).join("package.json"), "{}\n").unwrap();
    }
    fs::create_dir_all(project.path().join(source_package).join(source_directory)).unwrap();
    fs::create_dir_all(project.path().join(target_package).join(target_directory)).unwrap();
    let mut source = format!(
        "import target from '../../{target_package}/{target_directory}/{target_file}';\n\
         import choice from './choice';\n\
         import library from 'extremely-long-external-library-name';\n\
         const dynamicValue = require(dynamicModuleName);\n\
         export class ExtremelyLongContainerNameThatMustRemainRecognizable {{\n\
           extremelyLongMethodNameThatMustRemainRecognizable(input) {{\n"
    );
    for index in 0..55 {
        source.push_str(&format!("    const value{index} = input + {index};\n"));
    }
    source.push_str("    return target(choice + library + dynamicValue + value54);\n  }\n}\n");
    fs::write(project.path().join(&source_path), &source).unwrap();
    fs::write(
        project.path().join(&target_path),
        format!(
            "import source from '../../{source_package}/{source_directory}/{source_file}';\nexport default source;\n"
        ),
    )
    .unwrap();
    fs::write(
        project
            .path()
            .join(source_package)
            .join(source_directory)
            .join("choice.js"),
        // The long source file is configured as test source, so the primary
        // half of the package cycle has to come from a primary file: a verdict
        // graph carries primary relations only.
        format!(
            "import target from '../../{target_package}/{target_directory}/{target_file}';\nexport default target;\n"
        ),
    )
    .unwrap();
    fs::write(
        project
            .path()
            .join(source_package)
            .join(source_directory)
            .join("choice.ts"),
        "export default 2;\n",
    )
    .unwrap();
    let rust_package = "very-long-rust-ownership-package-name";
    fs::create_dir_all(project.path().join(rust_package).join("src")).unwrap();
    fs::write(
        project.path().join(rust_package).join("Cargo.toml"),
        "[package]\nname = 'long-owned'\nversion = '0.1.0'\n",
    )
    .unwrap();
    fs::write(
        project.path().join(rust_package).join("src/lib.rs"),
        "mod extremely_long_owned_module_name_that_stays_visible;\n",
    )
    .unwrap();
    fs::write(
        project
            .path()
            .join(rust_package)
            .join("src/extremely_long_owned_module_name_that_stays_visible.rs"),
        "pub fn owned() {}\n",
    )
    .unwrap();
    fs::write(
        project.path().join(".smackdebt.toml"),
        format!("[source_roles]\ntest = ['{source_path}']\n"),
    )
    .unwrap();
    git(project.path(), ["init", "-b", "main"]);
    commit_as(
        project.path(),
        "Responsive Test",
        "responsive@example.invalid",
        "initial",
    );
    source.push_str("// second activity commit\n");
    fs::write(project.path().join(&source_path), source).unwrap();
    commit_as(
        project.path(),
        "Responsive Test",
        "responsive@example.invalid",
        "activity",
    );
    project
}

fn git<const N: usize>(directory: &Path, arguments: [&str; N]) {
    let mut command = Command::new("git");
    hermetic_env(&mut command);
    let status = command
        .args(arguments)
        .env("GIT_AUTHOR_DATE", "2026-08-01T12:00:00Z")
        .env("GIT_COMMITTER_DATE", "2026-08-01T12:00:00Z")
        .current_dir(directory)
        .status()
        .unwrap();
    assert!(status.success());
}

fn evolutionary_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    git(project.path(), ["init", "-b", "main"]);
    fs::create_dir_all(project.path().join("a")).unwrap();
    fs::create_dir_all(project.path().join("b")).unwrap();
    fs::create_dir_all(project.path().join("c")).unwrap();
    fs::write(project.path().join("a/package.json"), "{}\n").unwrap();
    fs::write(project.path().join("b/package.json"), "{}\n").unwrap();
    fs::write(project.path().join("c/package.json"), "{}\n").unwrap();
    fs::write(project.path().join("a/old.js"), "export const a = 1;\n").unwrap();
    fs::write(project.path().join("b/main.js"), "export const b = 1;\n").unwrap();
    fs::write(project.path().join("c/main.js"), "export const c = 1;\n").unwrap();
    commit_as(
        project.path(),
        "Alice Example",
        "alice@example.invalid",
        "initial",
    );
    fs::rename(
        project.path().join("a/old.js"),
        project.path().join("a/main.js"),
    )
    .unwrap();
    fs::write(project.path().join("b/main.js"), "export const b = 2;\n").unwrap();
    commit_as(
        project.path(),
        "Alias Person",
        "alias@example.invalid",
        "rename together",
    );
    fs::write(
        project.path().join(".mailmap"),
        "Alice Example <alice@example.invalid> Alias Person <alias@example.invalid>\n",
    )
    .unwrap();
    fs::write(project.path().join("a/main.js"), "export const a = 3;\n").unwrap();
    fs::write(project.path().join("b/main.js"), "export const b = 3;\n").unwrap();
    commit_as(
        project.path(),
        "Bob Example",
        "bob@example.invalid",
        "together again",
    );
    fs::write(project.path().join("a/main.js"), "export const a = 4;\n").unwrap();
    commit_as(
        project.path(),
        "Alice Example",
        "alice@example.invalid",
        "a only one",
    );
    fs::write(project.path().join("a/main.js"), "export const a = 5;\n").unwrap();
    commit_as(
        project.path(),
        "Alice Example",
        "alice@example.invalid",
        "a only two",
    );
    fs::write(project.path().join("b/main.js"), "export const b = 6;\n").unwrap();
    commit_as(
        project.path(),
        "Bob Example",
        "bob@example.invalid",
        "b only",
    );
    project
}

fn shallow_evolutionary_fixture() -> tempfile::TempDir {
    let origin = evolutionary_fixture();
    fs::write(origin.path().join("a/main.js"), "export const a = 7;\n").unwrap();
    fs::write(origin.path().join("b/main.js"), "export const b = 7;\n").unwrap();
    commit_as(
        origin.path(),
        "Both Example",
        "both@example.invalid",
        "together at head",
    );
    let checkout = tempfile::tempdir().unwrap();
    let mut clone = Command::new("git");
    hermetic_env(&mut clone);
    let output = clone
        .args([
            "clone",
            "--depth",
            "6",
            &format!("file://{}", origin.path().display()),
            ".",
        ])
        .current_dir(checkout.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    checkout
}

/// A repository holding one file per frozen file pattern plus the history one
/// package needs to concentrate its knowledge.
///
/// Package `god` does too much and is broad both ways, package `hub` is one
/// widely imported file beside the nine that import it, and package `hot`
/// carries every commit after the first, which makes its one file a hotspot
/// and its package a single-author package.
fn problem_pattern_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    git(project.path(), ["init", "-b", "main"]);
    fs::write(
        project.path().join(".smackdebt.toml"),
        "[thresholds]\ncognitive = { watch = 2, high = 4 }\ncyclomatic = { watch = 2, high = 4 }\nfunction_lines = { watch = 20, high = 40 }\nfile_lines = { watch = 30, high = 200 }\n\n[hotspots]\nminimum_touches = 3\n",
    )
    .unwrap();
    for package in ["god", "hub", "hot"] {
        fs::create_dir_all(project.path().join(package)).unwrap();
        fs::write(
            project.path().join(package).join("package.json"),
            format!("{{\"name\":\"{package}\",\"private\":true}}\n"),
        )
        .unwrap();
    }
    // One widely imported file with one Watch unit, and the nine files of its
    // own package that import it.
    fs::write(
        project.path().join("hub/hub.js"),
        "export function pick(value) {\n  if (value) {\n    return 1;\n  }\n  return 0;\n}\n",
    )
    .unwrap();
    for index in 0..9 {
        fs::write(
            project.path().join(format!("hub/user-{index}.js")),
            format!(
                "import {{ pick }} from './hub';\n\nexport const user{index} = pick({index});\n"
            ),
        )
        .unwrap();
    }
    // Ten imports make the concentrated file broad without its length.
    let mut god = (0..9)
        .map(|index| format!("import user{index} from '../hub/user-{index}';\n"))
        .collect::<String>();
    god.push_str("import { pick } from '../hub/hub';\n\n");
    for name in ["first", "second", "third"] {
        god.push_str(&format!(
            "export function {name}(value) {{\n  if (value > 1) {{\n    if (value > 2) {{\n      if (value > 3) {{\n        return pick(value);\n      }}\n    }}\n  }}\n  return 0;\n}}\n\n"
        ));
    }
    god.push_str(
        "export const total = user0 + user1 + user2 + user3 + user4 + user5 + user6 + user7 + user8;\n",
    );
    fs::write(project.path().join("god/god.js"), god).unwrap();
    // One ordinary file no named pattern claims, which the fallback measures.
    fs::write(
        project.path().join("god/plain.js"),
        "export function plain(value) {\n  if (value) {\n    return 2;\n  }\n  return 0;\n}\n",
    )
    .unwrap();
    // One file no grammar can parse cleanly, so its findings are advisory:
    // they cannot move the verdict and their card is detail rather than
    // default.
    fs::write(
        project.path().join("god/recovered.js"),
        "export function recovered(value) {\n  if (value > 1) {\n    if (value > 2) {\n      if (value > 3) {\n        return 1;\n      }\n    }\n  }\n  return 0;\n}\n\nexport function unterminated(\n",
    )
    .unwrap();
    // One file whose only debt is its length, so its card heads on its size
    // finding rather than on a claimed source finding.
    let mut long = (0..40)
        .map(|line| format!("// a long file states its length and nothing else, line {line}\n"))
        .collect::<String>();
    long.push_str("export const length = 1;\n");
    fs::write(project.path().join("god/long.js"), long).unwrap();
    let churn = |version: u32| {
        format!(
            "export function churn(value) {{\n  if (value > {version}) {{\n    if (value > 2) {{\n      if (value > 3) {{\n        return {version};\n      }}\n    }}\n  }}\n  return 0;\n}}\n"
        )
    };
    fs::write(project.path().join("hot/hot.js"), churn(1)).unwrap();
    // Two files no grammar reads, so the grouped sentence that names them
    // states a plural subject with a plural verb and a plural object.
    for name in ["first", "second"] {
        fs::write(
            project.path().join(format!("hub/{name}.go")),
            format!("package hub\n\nfunc {name}() int {{ return 1 }}\n"),
        )
        .unwrap();
    }
    commit_as(
        project.path(),
        "Pattern Test",
        "pattern@example.invalid",
        "initial",
    );
    // Every later commit touches one package only, so nothing couples and one
    // author owns that package's whole history.
    for version in 2..=11 {
        fs::write(project.path().join("hot/hot.js"), churn(version)).unwrap();
        commit_as(
            project.path(),
            "Pattern Test",
            "pattern@example.invalid",
            &format!("churn {version}"),
        );
    }
    project
}

fn history_strength_order_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    git(project.path(), ["init", "-b", "main"]);
    for package in ["a", "b", "c", "d", "e"] {
        fs::create_dir_all(project.path().join(package)).unwrap();
        fs::write(project.path().join(package).join("package.json"), "{}\n").unwrap();
        fs::write(
            project.path().join(package).join("main.js"),
            format!("export const {package} = 0;\n"),
        )
        .unwrap();
    }
    commit_as(
        project.path(),
        "History Test",
        "history@example.invalid",
        "initial",
    );
    for (left, right, commits) in [("a", "b", 3), ("b", "c", 6), ("c", "d", 4), ("d", "e", 3)] {
        for version in 1..=commits {
            for package in [left, right] {
                fs::write(
                    project.path().join(package).join("main.js"),
                    format!("export const {package} = '{left}-{right}-{version}';\n"),
                )
                .unwrap();
            }
            commit_as(
                project.path(),
                "History Test",
                "history@example.invalid",
                &format!("{left} {right} {version}"),
            );
        }
    }
    project
}

fn contextual_history_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    git(project.path(), ["init", "-b", "main"]);
    for package in ["a", "b", "c"] {
        fs::create_dir_all(project.path().join(package)).unwrap();
        fs::write(project.path().join(package).join("package.json"), "{}\n").unwrap();
        fs::write(
            project.path().join(package).join("main.js"),
            format!("export const {package} = 1;\n"),
        )
        .unwrap();
    }
    fs::write(
        project.path().join(".smackdebt.toml"),
        "[source_roles]\nfixture = ['b/main.js']\ngenerated = ['c/main.js']\n",
    )
    .unwrap();
    commit_as(
        project.path(),
        "History Test",
        "history@example.invalid",
        "initial",
    );
    for version in 2..=3 {
        for package in ["a", "b", "c"] {
            fs::write(
                project.path().join(package).join("main.js"),
                format!("export const {package} = {version};\n"),
            )
            .unwrap();
        }
        commit_as(
            project.path(),
            "History Test",
            "history@example.invalid",
            &format!("version {version}"),
        );
    }
    project
}

fn generated_heavy_history_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    git(project.path(), ["init", "-b", "main"]);
    for package in ["a", "b"] {
        fs::create_dir_all(project.path().join(package)).unwrap();
        fs::write(project.path().join(package).join("package.json"), "{}\n").unwrap();
        fs::write(
            project.path().join(package).join("main.js"),
            format!("export const {package} = 1;\n"),
        )
        .unwrap();
        fs::write(
            project.path().join(package).join("generated.js"),
            format!("export const generated_{package} = 1;\n"),
        )
        .unwrap();
    }
    fs::write(
        project.path().join(".smackdebt.toml"),
        "[source_roles]\ngenerated = ['a/generated.js', 'b/generated.js']\n",
    )
    .unwrap();
    commit_as(
        project.path(),
        "Eligible History",
        "eligible@example.invalid",
        "initial eligible and context",
    );
    for version in 2..=3 {
        for package in ["a", "b"] {
            fs::write(
                project.path().join(package).join("main.js"),
                format!("export const {package} = {version};\n"),
            )
            .unwrap();
        }
        commit_as(
            project.path(),
            "Eligible History",
            "eligible@example.invalid",
            &format!("eligible {version}"),
        );
    }
    for version in 2..=8 {
        for package in ["a", "b"] {
            fs::write(
                project.path().join(package).join("generated.js"),
                format!("export const generated_{package} = {version};\n"),
            )
            .unwrap();
        }
        commit_as(
            project.path(),
            &format!("Generated {version}"),
            &format!("generated-{version}@example.invalid"),
            &format!("generated {version}"),
        );
    }
    project
}

fn generated_only_history_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    git(project.path(), ["init", "-b", "main"]);
    for package in ["a", "b"] {
        fs::create_dir_all(project.path().join(package)).unwrap();
        fs::write(project.path().join(package).join("package.json"), "{}\n").unwrap();
        fs::write(
            project.path().join(package).join("generated.js"),
            format!("export const generated_{package} = 1;\n"),
        )
        .unwrap();
    }
    fs::write(
        project.path().join(".smackdebt.toml"),
        "[source_roles]\ngenerated = ['a/generated.js', 'b/generated.js']\n",
    )
    .unwrap();
    commit_as(
        project.path(),
        "Generated History",
        "generated@example.invalid",
        "generated one",
    );
    for version in 2..=3 {
        for package in ["a", "b"] {
            fs::write(
                project.path().join(package).join("generated.js"),
                format!("export const generated_{package} = {version};\n"),
            )
            .unwrap();
        }
        commit_as(
            project.path(),
            "Generated History",
            "generated@example.invalid",
            &format!("generated {version}"),
        );
    }
    project
}

fn eligible_role_history_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    git(project.path(), ["init", "-b", "main"]);
    fs::create_dir_all(project.path().join("a")).unwrap();
    fs::write(project.path().join("a/package.json"), "{}\n").unwrap();
    fs::write(project.path().join("a/main.js"), "export const main = 1;\n").unwrap();
    fs::write(project.path().join("a/test.js"), "export const test = 1;\n").unwrap();
    fs::write(
        project.path().join(".smackdebt.toml"),
        "[source_roles]\ntest = ['a/test.js']\n",
    )
    .unwrap();
    commit_as(
        project.path(),
        "Role History",
        "roles@example.invalid",
        "roles one",
    );
    fs::write(project.path().join("a/main.js"), "export const main = 2;\n").unwrap();
    fs::write(project.path().join("a/test.js"), "export const test = 2;\n").unwrap();
    commit_as(
        project.path(),
        "Role History",
        "roles@example.invalid",
        "roles two",
    );
    project
}

fn weak_history_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    git(project.path(), ["init", "-b", "main"]);
    for package in ["a", "b"] {
        fs::create_dir_all(project.path().join(package)).unwrap();
        fs::write(project.path().join(package).join("package.json"), "{}\n").unwrap();
        fs::write(
            project.path().join(package).join("main.js"),
            format!("export const {package} = 1;\n"),
        )
        .unwrap();
    }
    commit_as(
        project.path(),
        "History Test",
        "history@example.invalid",
        "together one",
    );
    for package in ["a", "b"] {
        fs::write(
            project.path().join(package).join("main.js"),
            format!("export const {package} = 2;\n"),
        )
        .unwrap();
    }
    commit_as(
        project.path(),
        "History Test",
        "history@example.invalid",
        "together two",
    );
    fs::write(project.path().join("a/main.js"), "export const a = 3;\n").unwrap();
    commit_as(
        project.path(),
        "History Test",
        "history@example.invalid",
        "a only",
    );
    fs::write(project.path().join("b/main.js"), "export const b = 3;\n").unwrap();
    commit_as(
        project.path(),
        "History Test",
        "history@example.invalid",
        "b only",
    );
    project
}

fn commit_as(directory: &Path, name: &str, email: &str, message: &str) {
    git(directory, ["add", "-A"]);
    let mut command = Command::new("git");
    hermetic_env(&mut command);
    let status = command
        .args([
            "-c",
            &format!("user.name={name}"),
            "-c",
            &format!("user.email={email}"),
            "commit",
            "-qm",
            message,
        ])
        .env("GIT_AUTHOR_DATE", "2026-08-01T12:00:00Z")
        .env("GIT_COMMITTER_DATE", "2026-08-01T12:00:00Z")
        .current_dir(directory)
        .status()
        .unwrap();
    assert!(status.success());
}

/// Commit with the landed (committer) and authored dates pinned to `date`, or
/// to the wall clock when no date is given, so window fixtures control the
/// instant history filters compare.
fn commit_landed(directory: &Path, date: Option<&str>, message: &str) {
    git(directory, ["add", "-A"]);
    let mut command = Command::new("git");
    hermetic_env(&mut command);
    command
        .args([
            "-c",
            "user.name=History Test",
            "-c",
            "user.email=history@example.invalid",
            "commit",
            "-qm",
            message,
        ])
        .current_dir(directory);
    if let Some(date) = date {
        command
            .env("GIT_AUTHOR_DATE", date)
            .env("GIT_COMMITTER_DATE", date);
    }
    let status = command.status().unwrap();
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

/// A repository whose worktree deletes one rated unit and writes another, so a
/// diff carries exactly one added and one removed comparison.
fn one_sided_diff_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("package.json"),
        "{\"name\":\"one-sided\",\"private\":true}\n",
    )
    .unwrap();
    fs::write(
        project.path().join(".smackdebt.toml"),
        "[thresholds.cognitive]\nwatch = 2\nhigh = 5\n\n[thresholds.cyclomatic]\nwatch = 2\nhigh = 5\n\n[thresholds.function_lines]\nwatch = 2\nhigh = 5\n",
    )
    .unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    fs::write(
        project.path().join("src/gone.js"),
        "export function gone(value, other) {\n  if (value) {\n    if (other) {\n      return value + other;\n    }\n  }\n  return 0;\n}\n",
    )
    .unwrap();
    git(project.path(), ["init", "-b", "main"]);
    git(project.path(), ["config", "user.name", "Smackdebt Test"]);
    git(
        project.path(),
        ["config", "user.email", "smackdebt@example.invalid"],
    );
    git(project.path(), ["add", "."]);
    git(project.path(), ["commit", "-m", "test: one rated unit"]);
    fs::remove_file(project.path().join("src/gone.js")).unwrap();
    fs::write(
        project.path().join("src/fresh.js"),
        "export function fresh(value) {\n  if (value) {\n    if (value > 1) {\n      return value;\n    }\n  }\n  return 0;\n}\n",
    )
    .unwrap();
    project
}

fn anonymous_diff_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("package.json"),
        "{\"name\":\"anonymous-diff\",\"private\":true}\n",
    )
    .unwrap();
    fs::write(
        project.path().join("callbacks.js"),
        "watch('ready', () => work());\nrepeat(() => same());\ngone(() => old());\nold_only(() => retired());\nold_only(() => retired());\nconst steady = () => keep();\n",
    )
    .unwrap();
    for index in 0..100 {
        fs::write(
            project.path().join(format!("stable-{index}.js")),
            format!("export function stable{index}() {{ return {index}; }}\n"),
        )
        .unwrap();
    }
    git(project.path(), ["init", "-b", "main"]);
    git(project.path(), ["config", "user.name", "Smackdebt Test"]);
    git(
        project.path(),
        ["config", "user.email", "smackdebt@example.invalid"],
    );
    git(project.path(), ["add", "."]);
    git(project.path(), ["commit", "-m", "test: anonymous base"]);
    fs::write(
        project.path().join("callbacks.js"),
        "\nwatch('ready', () => { if (ready) work(); });\nrepeat(() => same());\nrepeat(() => same());\nonly(() => new_one());\nonly(() => new_one());\n\nconst steady = () => keep();\n",
    )
    .unwrap();
    project
}

fn named_duplicate_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("duplicate.js"),
        "function same() { return 1; }\nfunction same() { return 2; }\n",
    )
    .unwrap();
    git(project.path(), ["init", "-b", "main"]);
    git(project.path(), ["config", "user.name", "Smackdebt Test"]);
    git(
        project.path(),
        ["config", "user.email", "smackdebt@example.invalid"],
    );
    git(project.path(), ["add", "."]);
    git(project.path(), ["commit", "-m", "test: duplicate names"]);
    fs::write(
        project.path().join("duplicate.js"),
        "function same() { return 1; }\nfunction same() { if (ready) return 2; }\n",
    )
    .unwrap();
    project
}

/// A repository whose debt lives in one area, so a selected clean area and the
/// repository answer differently.
fn split_debt_fixture() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("Gemfile"),
        "source 'https://example.invalid'\n",
    )
    .unwrap();
    fs::create_dir(project.path().join("clean")).unwrap();
    fs::create_dir(project.path().join("messy")).unwrap();
    fs::write(
        project.path().join("clean/ship.rb"),
        "def ship(item)\n  item.ship\nend\n",
    )
    .unwrap();
    fs::write(
        project.path().join("clean/ready.rb"),
        "def ready?(item)\n  item.ready\nend\n",
    )
    .unwrap();
    for index in 0..4 {
        fs::write(
            project.path().join(format!("messy/tangle{index}.rb")),
            tangled_source(index),
        )
        .unwrap();
    }
    // A repository root is what makes a path selection a drill-down into one
    // report rather than a separate walk of a smaller tree.
    git(project.path(), ["init", "-b", "main"]);
    git(project.path(), ["config", "user.name", "Smackdebt Test"]);
    git(
        project.path(),
        ["config", "user.email", "smackdebt@example.invalid"],
    );
    git(project.path(), ["add", "."]);
    git(project.path(), ["commit", "-m", "test: split debt"]);
    project
}

/// A repository whose report outgrows one buffered write, so a reader that
/// stops reading has already left before the last write happens.
fn tangled_fixture(files: usize) -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("Gemfile"),
        "source 'https://example.invalid'\n",
    )
    .unwrap();
    fs::create_dir(project.path().join("src")).unwrap();
    for index in 0..files {
        fs::write(
            project.path().join(format!("src/tangle{index}.rb")),
            tangled_source(index),
        )
        .unwrap();
    }
    project
}

/// One deeply nested Ruby function, which every rated measurement objects to.
fn tangled_source(index: usize) -> String {
    let mut source = format!("def tangle{index}(items)\n");
    for depth in 0..8 {
        source.push_str(&"  ".repeat(depth + 1));
        source.push_str(&format!("if items[{depth}] && items[{depth}].ready?\n"));
    }
    source.push_str(&"  ".repeat(9));
    source.push_str("ship(items)\n");
    for depth in (0..8).rev() {
        source.push_str(&"  ".repeat(depth + 1));
        source.push_str("end\n");
    }
    source.push_str("end\n");
    source
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
