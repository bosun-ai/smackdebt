use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use assert_cmd::cargo::cargo_bin_cmd;

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
    assert!(text.contains("QUALITY"));
    assert!(text.contains("No child areas need attention"));
}

#[test]
fn serial_and_parallel_json_match_and_follow_schema_three() {
    let project = fixture();
    let path = project.path().to_str().unwrap();
    let serial = run(["--json", "--jobs", "1", path]);
    let parallel = run(["--json", "--jobs", "4", path]);
    assert_eq!(serial, parallel);
    let report: serde_json::Value = serde_json::from_slice(&serial).unwrap();
    assert_eq!(report["schema_version"], 3);
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

    cargo_bin_cmd!("smackdebt")
        .arg(project.path())
        .assert()
        .code(2)
        .stdout("")
        .stderr("smackdebt: source role conflict for conflict.js: test, fixture\n");
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
}

#[test]
fn architecture_path_drill_keeps_incoming_edges_and_omits_unrelated_regions() {
    let project = static_architecture_fixture();
    git(project.path(), ["init", "-b", "main"]);
    let output = run_in(project.path(), ["app", "--all", "--color", "never"]);
    let text = String::from_utf8(output).unwrap();
    assert!(text.contains("core/main.js → app/main.js"));
    assert!(!text.contains("native/src/helper.rs"));
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
    assert_eq!(report["change_coupling"][0]["similarity"], 0.5);
    assert_eq!(report["change_coupling"].as_array().unwrap().len(), 1);
    assert_eq!(report["evolutionary_findings"].as_array().unwrap().len(), 1);
    let terminal_text = String::from_utf8(serial_terminal.clone()).unwrap();
    assert_eq!(terminal_text.matches("3/6 shared commits").count(), 1);
    assert_eq!(terminal_text.matches("  a · 5 touches").count(), 1);
    assert_eq!(terminal_text.matches("80% top share").count(), 1);
    assert!(!terminal_text.contains("coupling a ↔ c"));
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
    assert!(!default_terminal.contains("coupling a ↔ b"));
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
    assert!(detailed_terminal.contains("coupling a ↔ b · 3/6 shared commits"));
    assert!(detailed_terminal.contains("static use"));
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
    assert!(!default.contains("coupling a ↔ b"));
    assert!(!default.contains("coupling a ↔ c"));
    let detailed = String::from_utf8(run_in(
        project.path(),
        ["--all", "--color", "never", "--history", "36500d"],
    ))
    .unwrap();
    assert!(detailed.contains("coupling a ↔ b · 3/3 shared commits"));
    assert!(detailed.contains("coupling a ↔ c · 3/3 shared commits"));
    assert!(detailed.contains("primary/trusted ↔ generated/trusted"));
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
    assert!(
        report["change_coupling"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["left_role"] == "primary"
                && row["right_role"] == "primary"
                && row["shared_commits"] == 3
                && row["union_commits"] == 3)
    );
    assert!(
        report["change_coupling"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["left_role"] == "generated"
                && row["right_role"] == "generated"
                && row["shared_commits"] == 8
                && row["union_commits"] == 8)
    );
    assert_eq!(report["evolutionary_findings"][0]["shared_commits"], 3);
    assert_eq!(report["evolutionary_findings"][0]["union_commits"], 3);

    let default = String::from_utf8(run_in(
        project.path(),
        ["--color", "never", "--history", "36500d"],
    ))
    .unwrap();
    assert!(default.contains("eligible mapping 6/25 changes · 24% · 3 commits"));
    assert!(default.contains("a · 3 touches"));
    assert!(default.contains("1 contributors · 100% top share"));
    assert!(!default.contains("generated/trusted"));
    assert!(!default.contains("a · 8 touches"));

    let detailed = String::from_utf8(run_in(
        project.path(),
        ["--all", "--color", "never", "--history", "36500d"],
    ))
    .unwrap();
    assert!(detailed.contains("a · 8 touches"));
    assert!(detailed.contains("8 contributors · 13% top share · generated/trusted"));
    assert!(detailed.contains("generated/trusted ↔ generated/trusted"));
    assert_eq!(detailed.matches("3/3 shared commits").count(), 1);
    assert!(detailed.contains("● WATCH  coupling a ↔ b · 3/3 shared commits"));
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
    assert!(terminal.contains("complete local stream · 3 commits"));
    assert!(terminal.contains("eligible mapping 0/9 changes · 0% · 0 commits"));
    assert!(!terminal.contains("● WATCH"));
}

#[test]
fn default_history_shows_each_package_once_across_eligible_roles() {
    let project = eligible_role_history_fixture();
    let default = String::from_utf8(run_in(
        project.path(),
        ["--color", "never", "--history", "36500d"],
    ))
    .unwrap();
    assert_eq!(default.matches("  a · 2 touches").count(), 1);

    let detailed = String::from_utf8(run_in(
        project.path(),
        ["--all", "--color", "never", "--history", "36500d"],
    ))
    .unwrap();
    assert_eq!(detailed.matches("  a · 2 touches").count(), 2);
    assert!(detailed.contains("primary/trusted"));
    assert!(detailed.contains("test/trusted"));
}

#[test]
fn weak_coupling_stays_in_json_and_detailed_terminal_only() {
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

    let default = String::from_utf8(run_in(
        project.path(),
        ["--color", "never", "--history", "36500d"],
    ))
    .unwrap();
    assert!(!default.contains("coupling a ↔ b"));
    let detailed = String::from_utf8(run_in(
        project.path(),
        ["--all", "--color", "never", "--history", "36500d"],
    ))
    .unwrap();
    assert!(detailed.contains("coupling a ↔ b · 2/4 shared commits"));
}

#[test]
fn selected_package_shows_file_churn_all_coupling_and_omits_unrelated_history() {
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
    assert!(text.contains("file a/main.js · 5 touches"));
    assert!(text.contains("coupling a ↔ b · 3/6 shared commits · 50% similarity · static use"));
    assert!(!text.contains("c/main.js"));
    assert!(!text.contains("  c ·"));
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
    assert!(text.contains("b · 4 touches"));
    assert!(text.contains("file b/main.js · 4 touches"));
    assert!(text.contains("coupling a ↔ b · 3/6 shared commits"));
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
    assert!(terminal.contains("history unavailable: repository has no commits"));
    assert!(terminal.contains("! Git history unavailable: repository has no commits"));
    assert!(!terminal.contains("history incomplete"));
}

#[test]
fn shallow_history_is_reported_as_incomplete() {
    let origin = evolutionary_fixture();
    let checkout = tempfile::tempdir().unwrap();
    let output = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
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
    assert_eq!(report["history_coverage"]["commits"], 1);
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

fn assert_snapshot(name: &str, actual: &[u8], expected: &[u8]) {
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
        serde_json::from_str(include_str!("../../../schemas/report-v3.schema.json")).unwrap();
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
        assert!(finding["similarity"].as_f64().unwrap() >= 0.2);
    }
    for (index, comparison) in evolutionary_comparisons.iter().enumerate() {
        assert_eq!(comparison["id"], index);
        let left = comparison["left"].as_u64().unwrap() as usize;
        let right = comparison["right"].as_u64().unwrap() as usize;
        assert!(left < package_graph.len());
        assert!(right < package_graph.len());
        assert!(left < right);
        assert!(comparison["shared_commits"].as_u64().unwrap() >= 3);
        assert!(comparison["similarity"].as_f64().unwrap() >= 0.2);
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
        serde_json::from_str(include_str!("../../../schemas/report-v3.schema.json")).unwrap();
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
        serde_json::from_str(include_str!("../../../schemas/report-v3.schema.json")).unwrap();
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
        serde_json::from_str(include_str!("../../../schemas/report-v3.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for (table, report) in [
        ("evolutionary_findings", finding_report),
        ("evolutionary_comparisons", comparison_report),
    ] {
        let mut weak_commits = report.clone();
        weak_commits[table][0]["shared_commits"] = 2.into();
        assert!(validator.validate(&weak_commits).is_err(), "{table}");

        let mut weak_similarity = report;
        weak_similarity[table][0]["similarity"] = 0.19.into();
        assert!(validator.validate(&weak_similarity).is_err(), "{table}");
    }
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

fn git<const N: usize>(directory: &Path, arguments: [&str; N]) {
    let status = Command::new("git")
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
    let status = Command::new("git")
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
