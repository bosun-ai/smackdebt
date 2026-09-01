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
use support::edges::assert_no_dependency_edge_rows;
use support::hermetic::hermetic_env;
use support::{
    Commit, GeneratedRepository, Identity, Invocation, amplification_repository,
    bulk_commit_repository, change_leakage_repository, copy_language_truth_files, core_repository,
    deepened_signal_repository, evolution_repository, module_wiring_repository,
    propagation_repository, ref_diff_repository, rust_test_scope_repository, shallow_clone,
    signal_table_repository, source_role_repository, stable_dependency_repository,
    static_architecture_repository, test_scoped_workspace_repository, wide_directory_repository,
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

/// The rows of the problem section, which is the codebase debt body.
fn problem_body(text: &str) -> Vec<&str> {
    text.lines()
        .skip_while(|line| *line != "PROBLEMS")
        .skip(1)
        .take_while(|line| !line.is_empty())
        .collect()
}

/// The card heads of the problem section, which are its rows that are not
/// indented continuations.
fn problem_heads(text: &str) -> Vec<&str> {
    problem_body(text)
        .into_iter()
        .filter(|line| !line.starts_with("        "))
        .collect()
}

/// The verdict block, which is every line before the first blank one.
fn verdict_block(text: &str) -> Vec<&str> {
    text.lines().take_while(|line| !line.is_empty()).collect()
}

/// The verdict block without the facts a verdict states only, which is the
/// scope, the tier sentence, the counts, and the worst offender.
///
/// The propagation and amplification facts are the lines the stated-only rule
/// allows to differ between two trees; everything left here is what it forbids
/// to differ.
fn rated_verdict_lines(text: &str) -> Vec<&str> {
    verdict_block(text)
        .into_iter()
        .filter(|line| {
            !line.contains("can reach")
                && !line.contains("sit in one dependency cycle")
                && !line.contains("A typical change here touches")
        })
        .collect()
}

/// The repository path of one file in a report.
fn file_path(report: &Value, file: u64) -> String {
    let file = &report["files"][file as usize];
    report["paths"][file["path"].as_u64().unwrap() as usize]
        .as_str()
        .unwrap()
        .to_owned()
}

/// The repository path of one package in a report.
fn package_path(report: &Value, package: u64) -> String {
    report["packages"][package as usize]["path"]
        .as_str()
        .unwrap()
        .to_owned()
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
fn a_rust_test_scope_publishes_test_relations_beside_the_primary_ones() {
    let repository = rust_test_scope_repository();
    let result = Invocation::new(["--json"]).run(repository.path());
    result.success();
    let automatic = Invocation::new(["--json"])
        .automatic_workers()
        .run(repository.path());
    assert_eq!(result, automatic);
    let report = checked_json(&result.stdout);
    let mut relations: Vec<_> = report["dependency_edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| {
            (
                file_path(&report, edge["source"].as_u64().unwrap()),
                file_path(&report, edge["target"].as_u64().unwrap()),
                edge["relation"].as_str().unwrap().to_owned(),
                edge["role"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    relations.sort();
    assert_eq!(
        relations,
        [
            (
                "src/lib.rs".to_owned(),
                "src/helper.rs".to_owned(),
                "module_ownership".to_owned(),
                "primary".to_owned()
            ),
            (
                "src/lib.rs".to_owned(),
                "src/helper.rs".to_owned(),
                "uses".to_owned(),
                "primary".to_owned()
            ),
            (
                "src/lib.rs".to_owned(),
                "src/helper.rs".to_owned(),
                "uses".to_owned(),
                "test".to_owned()
            ),
            (
                "src/lib.rs".to_owned(),
                "src/only_tests.rs".to_owned(),
                "module_ownership".to_owned(),
                "test".to_owned()
            ),
            (
                "src/lib.rs".to_owned(),
                "src/only_tests.rs".to_owned(),
                "uses".to_owned(),
                "test".to_owned()
            ),
            (
                "src/lib.rs".to_owned(),
                "src/shipped.rs".to_owned(),
                "module_ownership".to_owned(),
                "primary".to_owned()
            ),
            (
                "src/lib.rs".to_owned(),
                "src/shipped.rs".to_owned(),
                "uses".to_owned(),
                "primary".to_owned()
            ),
        ]
    );
    assert!(report["orphan_files"].as_array().unwrap().is_empty());
    // The build compiles `src/only_tests.rs` only under `test`, so the file
    // itself is test source while its siblings ship.
    let role_of = |path: &str| {
        report["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|file| report["paths"][file["path"].as_u64().unwrap() as usize] == path)
            .unwrap_or_else(|| panic!("missing {path}"))["role"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    assert_eq!(role_of("src/only_tests.rs"), "test");
    assert_eq!(role_of("src/shipped.rs"), "primary");
    assert_eq!(role_of("src/lib.rs"), "primary");
    assert_golden("unified-rust-test-scope.json", &result.stdout);
}

#[test]
fn rust_module_wiring_publishes_its_relations_without_a_file_cycle() {
    let repository = module_wiring_repository();
    let result = Invocation::new(["--json"]).run(repository.path());
    result.success();
    let automatic = Invocation::new(["--json"])
        .automatic_workers()
        .run(repository.path());
    assert_eq!(result, automatic);
    let report = checked_json(&result.stdout);
    let mut relations: Vec<_> = report["dependency_edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| {
            (
                file_path(&report, edge["source"].as_u64().unwrap()),
                file_path(&report, edge["target"].as_u64().unwrap()),
                edge["relation"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    relations.sort();
    assert_eq!(
        relations,
        [
            (
                "src/lib.rs".to_owned(),
                "src/thing/mod.rs".to_owned(),
                "module_ownership".to_owned()
            ),
            (
                "src/lib.rs".to_owned(),
                "src/thing/mod.rs".to_owned(),
                "uses".to_owned()
            ),
            (
                "src/thing/child.rs".to_owned(),
                "src/thing/mod.rs".to_owned(),
                "uses".to_owned()
            ),
            (
                "src/thing/mod.rs".to_owned(),
                "src/thing/child.rs".to_owned(),
                "module_ownership".to_owned()
            ),
            (
                "src/thing/mod.rs".to_owned(),
                "src/thing/child.rs".to_owned(),
                "uses".to_owned()
            ),
        ],
        "the imports that wire a module stay complete in the machine report"
    );
    assert!(
        report["architecture_findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| finding["kind"] != "file_cycle"),
        "imports between a module-owning pair are wiring, not a cycle"
    );
    assert!(report["orphan_files"].as_array().unwrap().is_empty());
    assert_golden("unified-module-wiring.json", &result.stdout);

    let terminal = Invocation::new(["--all"]).run(repository.path());
    terminal.success();
    let text = String::from_utf8(terminal.stdout.clone()).unwrap();
    assert!(!text.contains("cycle"), "{text}");
    assert_golden("unified-module-wiring.terminal.txt", &terminal.stdout);
}

#[test]
fn test_only_return_dependencies_are_context_and_still_explain_coupling() {
    let repository = test_scoped_workspace_repository();
    let result = Invocation::new(["--json", "--history", "36500d"]).run(repository.path());
    result.success();
    let automatic = Invocation::new(["--json", "--history", "36500d"])
        .automatic_workers()
        .run(repository.path());
    assert_eq!(result, automatic);
    let report = checked_json(&result.stdout);

    // The verdict graph carries the shipped direction only.
    let mut package_edges: Vec<_> = report["package_edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| {
            (
                package_path(&report, edge["source"].as_u64().unwrap()),
                package_path(&report, edge["target"].as_u64().unwrap()),
            )
        })
        .collect();
    package_edges.sort();
    assert_eq!(
        package_edges,
        [("crates/alpha".to_owned(), "crates/beta".to_owned())]
    );
    assert!(
        report["architecture_findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| finding["kind"] != "package_cycle"),
        "test code cannot close a package cycle"
    );
    assert!(
        report["stable_dependency_findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    // The return dependencies stay complete as evidence.
    let mut returns: Vec<_> = report["dependency_edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|edge| {
            file_path(&report, edge["target"].as_u64().unwrap()) == "crates/alpha/src/lib.rs"
        })
        .map(|edge| {
            (
                file_path(&report, edge["source"].as_u64().unwrap()),
                edge["role"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    returns.sort();
    assert_eq!(
        returns,
        [
            ("crates/beta/src/lib.rs".to_owned(), "test".to_owned()),
            (
                "crates/beta/tests/integration.rs".to_owned(),
                "test".to_owned()
            ),
            (
                "crates/gamma/tests/integration.rs".to_owned(),
                "test".to_owned()
            ),
        ]
    );

    // Every qualifying coupling pair has a code dependency to explain it,
    // including the two that never enter a verdict graph.
    let coupling: Vec<_> = report["change_coupling"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|pair| pair["shared_commits"].as_u64().unwrap() >= 3)
        .map(|pair| {
            (
                package_path(&report, pair["left"].as_u64().unwrap()),
                package_path(&report, pair["right"].as_u64().unwrap()),
            )
        })
        .collect();
    assert_eq!(
        coupling,
        [
            ("crates/alpha".to_owned(), "crates/beta".to_owned()),
            ("crates/alpha".to_owned(), "crates/gamma".to_owned()),
        ]
    );
    assert!(
        report["evolutionary_findings"]
            .as_array()
            .unwrap()
            .is_empty(),
        "a dev-dependency import is still a code dependency"
    );
    assert_golden("unified-test-scoped-workspace.json", &result.stdout);

    let terminal = Invocation::new(["--all", "--history", "36500d"]).run(repository.path());
    terminal.success();
    let text = String::from_utf8(terminal.stdout.clone()).unwrap();
    assert!(!text.contains("package dependency cycle"), "{text}");
    assert!(!text.contains("no code dependency"), "{text}");
    assert_golden(
        "unified-test-scoped-workspace.terminal.txt",
        &terminal.stdout,
    );
}

#[test]
fn a_diff_over_test_explained_coupling_reports_no_evolutionary_change() {
    let repository = test_scoped_workspace_repository();
    let result =
        Invocation::new(["diff", "main~1", "--json", "--history", "36500d"]).run(repository.path());
    result.success();
    let report = checked_json(&result.stdout);
    assert_eq!(report["mode"], "diff");
    assert!(
        report["evolutionary_comparisons"]
            .as_array()
            .unwrap()
            .is_empty(),
        "coupling explained on both sides cannot change"
    );
    let terminal =
        Invocation::new(["diff", "main~1", "--all", "--history", "36500d"]).run(repository.path());
    terminal.success();
    let text = String::from_utf8(terminal.stdout).unwrap();
    assert!(!text.contains("no code dependency"), "{text}");
}

/// The layered fixture publishes hand-calculated reach operands while the
/// terminal avoids an aggregate without a named place.
///
/// `a` -> `b` -> `c` -> `d` makes `d` reachable from three other packages, so
/// its reach is four of the six packages. `wide` chains twenty modules under a
/// library root that only declares them, so the last module is reached by
/// twenty of that package's twenty-one files.
#[test]
fn a_layered_repository_states_its_package_reach_and_its_file_reach() {
    let repository = propagation_repository();
    let result = Invocation::new(["--json"]).run(repository.path());
    result.success();
    let automatic = Invocation::new(["--json"])
        .automatic_workers()
        .run(repository.path());
    assert_eq!(result, automatic, "serial and parallel runs must agree");
    let report = checked_json(&result.stdout);
    assert_eq!(
        report["verdict"]["reach"]["sentence"],
        "A change in one package can reach 4 of 6 packages."
    );
    assert_eq!(report["verdict"]["reach"]["reached"], 4);
    assert_eq!(report["verdict"]["reach"]["total"], 6);
    assert!(
        report["verdict"]["core_size"].is_null(),
        "a repository without a file cycle states no core"
    );
    let reach_in: Vec<_> = report["package_graph"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                package_path(&report, row["package"].as_u64().unwrap()),
                row["reach_in"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        reach_in,
        [
            ("crates/a".to_owned(), 1),
            ("crates/b".to_owned(), 2),
            ("crates/c".to_owned(), 3),
            ("crates/d".to_owned(), 4),
            ("crates/e".to_owned(), 1),
            ("crates/wide".to_owned(), 1),
        ]
    );
    let closures: Vec<_> = report["package_closures"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                package_path(&report, row["package"].as_u64().unwrap()),
                row["files"].as_u64().unwrap(),
                row["reach"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        closures,
        [("crates/wide".to_owned(), 21, 20)],
        "only a package above the file floor carries a closure row"
    );

    let root = Invocation::new([] as [&str; 0]).run(repository.path());
    root.success();
    let text = String::from_utf8(root.stdout).unwrap();
    assert!(!text.contains("A change in one package"), "{text}");
    assert!(!text.contains("sit in one dependency cycle"), "{text}");

    let package = Invocation::new(["crates/wide"]).run(repository.path());
    package.success();
    let package_text = String::from_utf8(package.stdout).unwrap();
    assert!(
        !package_text.contains("A change here can reach"),
        "{package_text}"
    );
    let scoped = Invocation::new(["crates/wide", "--json"]).run(repository.path());
    scoped.success();
    let scoped_report = checked_json(&scoped.stdout);
    assert_eq!(
        scoped_report["verdict"]["reach"]["sentence"],
        "A change here can reach 20 of 21 files in this package."
    );
    assert_eq!(scoped_report["verdict"]["reach"]["reached"], 20);
    assert_eq!(scoped_report["verdict"]["reach"]["total"], 21);

    // A package below the file floor states nothing, and neither does a
    // directory or a file inside the wide package.
    for scope in ["crates/a", "crates/wide/src", "crates/wide/src/step00.rs"] {
        let narrow = Invocation::new([scope, "--json"]).run(repository.path());
        narrow.success();
        let narrow_report = checked_json(&narrow.stdout);
        assert!(
            narrow_report["verdict"]["reach"].is_null(),
            "{scope} states no reach"
        );
        let rendered = Invocation::new([scope]).run(repository.path());
        rendered.success();
        let rendered_text = String::from_utf8(rendered.stdout).unwrap();
        assert!(!rendered_text.contains("can reach"), "{rendered_text}");
    }

    // Narrow output follows the same rule.
    let narrow = Invocation::new([] as [&str; 0])
        .columns(50)
        .run(repository.path());
    narrow.success();
    let narrow_text = String::from_utf8(narrow.stdout).unwrap();
    assert!(
        !narrow_text.contains("A change in one package"),
        "{narrow_text}"
    );
    let narrow_package = Invocation::new(["crates/wide"])
        .columns(50)
        .run(repository.path());
    narrow_package.success();
    let narrow_package_text = String::from_utf8(narrow_package.stdout).unwrap();
    assert!(
        !narrow_package_text.contains("A change here can reach"),
        "{narrow_package_text}"
    );
}

/// A repository with one package has no cross-package reach to state, so the
/// root states that package's own file reach instead — the repository is the
/// package and no path selects its package scope — and states nothing at all
/// where that package is below the file floor.
#[test]
fn a_single_package_repository_states_its_own_file_reach_or_nothing() {
    let small = rust_test_scope_repository();
    let result = Invocation::new(["--json"]).run(small.path());
    result.success();
    let report = checked_json(&result.stdout);
    assert!(report["verdict"]["reach"].is_null());
    assert!(report["package_closures"].as_array().unwrap().is_empty());
    let terminal = Invocation::new(["--all"]).run(small.path());
    terminal.success();
    let text = String::from_utf8(terminal.stdout).unwrap();
    assert!(!text.contains("can reach"), "{text}");

    // Twenty files in one package, six of them in one cycle: every member of
    // that cycle reaches the other five, so the package reaches six of twenty.
    let wide = core_repository(20, 6);
    let wide_result = Invocation::new(["--json"]).run(wide.path());
    wide_result.success();
    let wide_report = checked_json(&wide_result.stdout);
    assert_eq!(
        wide_report["verdict"]["reach"]["sentence"],
        "A change here can reach 6 of 20 files in this package."
    );
    assert_eq!(wide_report["verdict"]["reach"]["reached"], 6);
    assert_eq!(wide_report["verdict"]["reach"]["total"], 20);
    let wide_terminal = Invocation::new([] as [&str; 0]).run(wide.path());
    wide_terminal.success();
    let wide_text = String::from_utf8(wide_terminal.stdout).unwrap();
    assert!(
        !wide_text.contains("A change here can reach"),
        "{wide_text}"
    );
}

/// The core is stated only when the largest file cycle clears both floors, and
/// the cycle's own findings are the same either way.
#[test]
fn a_core_is_stated_only_when_the_largest_cycle_clears_both_floors() {
    let repository = core_repository(20, 6);
    let result = Invocation::new(["--json"]).run(repository.path());
    result.success();
    let automatic = Invocation::new(["--json"])
        .automatic_workers()
        .run(repository.path());
    assert_eq!(result, automatic, "serial and parallel runs must agree");
    let report = checked_json(&result.stdout);
    assert_eq!(
        report["verdict"]["core_size"]["sentence"],
        "6 of 20 files sit in one dependency cycle."
    );
    assert_eq!(report["verdict"]["core_size"]["core"], 6);
    assert_eq!(report["verdict"]["core_size"]["files"], 20);
    let cycles = report["architecture_findings"].as_array().unwrap().len();
    assert_eq!(cycles, 1, "one component, one finding");
    // Every member of the cycle is a candidate and states the same reach, which
    // excludes the file itself.
    let reaches: Vec<_> = report["file_reach"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["reach"].as_u64().unwrap())
        .collect();
    assert_eq!(reaches, [5; 6]);
    let terminal = Invocation::new([] as [&str; 0]).run(repository.path());
    terminal.success();
    let text = String::from_utf8(terminal.stdout).unwrap();
    assert!(!text.contains("sit in one dependency cycle"), "{text}");
    assert!(text.contains("6 of 20 files are in this cycle"), "{text}");
    assert!(
        text.contains("        a change here reaches 5 files"),
        "{text}"
    );
    let narrow = Invocation::new([] as [&str; 0])
        .columns(50)
        .run(repository.path());
    narrow.success();
    let narrow_text = String::from_utf8(narrow.stdout).unwrap();
    assert!(
        narrow_text.contains("6 of 20 files are in this cycle"),
        "the named cycle evidence fits fifty columns: {narrow_text}"
    );

    // The same twenty files without the cycle state no core and a reach of one
    // rather than six, and every rated line of the block is byte-identical:
    // both facts are stated only.
    let plain = core_repository(20, 0);
    let plain_terminal = Invocation::new([] as [&str; 0]).run(plain.path());
    plain_terminal.success();
    let plain_text = String::from_utf8(plain_terminal.stdout).unwrap();
    assert!(
        !plain_text.contains("sit in one dependency cycle"),
        "{plain_text}"
    );
    assert_eq!(
        rated_verdict_lines(&text),
        rated_verdict_lines(&plain_text),
        "neither propagation fact moves a tier, a count, or the worst offender"
    );

    let small = core_repository(200, 3);
    let small_result = Invocation::new(["--json"]).run(small.path());
    small_result.success();
    let small_report = checked_json(&small_result.stdout);
    assert!(
        small_report["verdict"]["core_size"].is_null(),
        "three of two hundred is below both floors"
    );
    assert_eq!(
        small_report["architecture_findings"]
            .as_array()
            .unwrap()
            .len(),
        1,
        "the cycle keeps its own finding whether or not the core is stated"
    );
    let small_terminal = Invocation::new([] as [&str; 0]).run(small.path());
    small_terminal.success();
    let small_text = String::from_utf8(small_terminal.stdout).unwrap();
    assert!(
        !small_text.contains("sit in one dependency cycle"),
        "{small_text}"
    );
}

/// The bulk-commit guard belongs to file pair accumulation alone, and the two
/// counters that disclose it are machine-report facts no human view states.
///
/// Five ordinary commits change one cross-directory pair. The sixth rewrites
/// both of its files and adds twenty-eight more, so thirty files enter the
/// change graph at once: the pair keeps five shared of five union while churn,
/// touches, and package change coupling all count six commits. Five later
/// commits touch a third package alone, so no pair operand moves and the
/// repository's amplification sample reaches the commit floor.
#[test]
fn a_sweeping_commit_is_counted_everywhere_but_in_the_file_pair_table() {
    let repository = bulk_commit_repository();
    let result = Invocation::new(["--json", "--history", "36500d"]).run(repository.path());
    result.success();
    let automatic = Invocation::new(["--json", "--history", "36500d"])
        .automatic_workers()
        .run(repository.path());
    assert_eq!(result, automatic, "serial and parallel runs must agree");
    let report = checked_json(&result.stdout);

    assert_eq!(report["history_coverage"]["commits"], 11);
    assert_eq!(report["history_coverage"]["bulk_commits"], 1);
    assert_eq!(report["history_coverage"]["declined_pairs"], 0);

    // The other half of the guard: the sweeping commit contributes exactly one
    // amplification observation, of the thirty files it changed. The other ten
    // commits are five of two files and five of three, so the sweep is the
    // eleventh observation and the one that moves the nearest-rank median from
    // two — below the floor, stating nothing — to three.
    let amplification = &report["verdict"]["amplification"];
    assert_eq!(
        amplification["sentence"],
        "A typical change here touches 3 files."
    );
    assert_eq!(amplification["median"], 3);
    assert_eq!(
        amplification["commits"], 11,
        "one observation from the sweeping commit, not thirty and not none"
    );

    let pairs: Vec<_> = report["file_change_coupling"]
        .as_array()
        .unwrap()
        .iter()
        .map(|pair| {
            (
                file_path(&report, pair["left"].as_u64().unwrap()),
                file_path(&report, pair["right"].as_u64().unwrap()),
                pair["shared_commits"].as_u64().unwrap(),
                pair["union_commits"].as_u64().unwrap(),
                pair["distance"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        pairs,
        [(
            "left/src/a.js".to_owned(),
            "right/src/b.js".to_owned(),
            5,
            5,
            4
        )],
        "the sweeping commit is outside both the shared count and its union"
    );

    let touches = |path: &str| {
        report["file_history"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| file_path(&report, row["file"].as_u64().unwrap()) == path)
            .map(|row| row["touches"].as_u64().unwrap())
    };
    assert_eq!(touches("left/src/a.js"), Some(6));
    assert_eq!(touches("right/src/b.js"), Some(6));
    assert_eq!(touches("left/src/bulk/unit00.js"), Some(1));
    let coupling: Vec<_> = report["change_coupling"]
        .as_array()
        .unwrap()
        .iter()
        .map(|pair| {
            (
                package_path(&report, pair["left"].as_u64().unwrap()),
                package_path(&report, pair["right"].as_u64().unwrap()),
                pair["shared_commits"].as_u64().unwrap(),
                pair["union_commits"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        coupling,
        [("left".to_owned(), "right".to_owned(), 6, 6)],
        "the guard never reaches package change coupling"
    );
    let concentration: Vec<_> = report["contributor_concentration"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                package_path(&report, row["package"].as_u64().unwrap()),
                row["contributor_count"].as_u64().unwrap(),
                row["numerator"].as_u64().unwrap(),
                row["denominator"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        concentration,
        [
            ("left".to_owned(), 1, 6, 6),
            ("right".to_owned(), 1, 6, 6),
            ("wide".to_owned(), 1, 5, 5)
        ],
        "the sweeping commit is one of the six commits concentration counts"
    );

    // No human view states either counter, at any scope or detail level.
    for arguments in [
        vec![],
        vec!["--all"],
        vec!["--top", "5"],
        vec!["left"],
        vec!["left/src"],
        vec!["left/src/a.js", "--all"],
    ] {
        let rendered = Invocation::new(arguments.clone()).run(repository.path());
        rendered.success();
        let text = String::from_utf8(rendered.stdout).unwrap();
        for absent in ["bulk commit", "declined", "change together"] {
            assert!(!text.contains(absent), "{arguments:?}: {text}");
        }
    }
}

/// JSON retains typical change size while terminal output avoids presenting it
/// as an action.
///
/// The fixture's root sees ten commits of two files, five of three, and twelve
/// of four, whose nearest-rank median is 3, while `core` and `core/src` see the
/// twelve of four alone and state 4: a scope answers about its own directory
/// rather than about the repository. `edge` has too few commits and `quiet` has
/// a median below the floor, so each states nothing.
#[test]
fn a_scope_states_how_many_files_a_typical_change_there_touches() {
    let repository = amplification_repository();
    let history = ["--json", "--history", "36500d"];
    let result = Invocation::new(history).run(repository.path());
    result.success();
    let automatic = Invocation::new(history)
        .automatic_workers()
        .run(repository.path());
    assert_eq!(result, automatic, "serial and parallel runs must agree");

    let stated = |arguments: &[&str]| {
        let run = Invocation::new(arguments.to_vec()).run(repository.path());
        run.success();
        let report = checked_json(&run.stdout);
        let amplification = &report["verdict"]["amplification"];
        if amplification.is_null() {
            return None;
        }
        Some((
            amplification["sentence"].as_str().unwrap().to_owned(),
            amplification["median"].as_u64().unwrap(),
            amplification["commits"].as_u64().unwrap(),
        ))
    };
    assert_eq!(
        stated(&history),
        Some(("A typical change here touches 3 files.".to_owned(), 3, 27))
    );
    let four = Some(("A typical change here touches 4 files.".to_owned(), 4, 12));
    assert_eq!(stated(&["core", "--json", "--history", "36500d"]), four);
    assert_eq!(stated(&["core/src", "--json", "--history", "36500d"]), four);
    assert_eq!(
        stated(&["core/src/unit0.js", "--json", "--history", "36500d"]),
        None,
        "a file scope would state sample noise as a fact"
    );
    assert_eq!(
        stated(&["edge", "--json", "--history", "36500d"]),
        None,
        "five commits are an anecdote rather than a typical change"
    );
    assert_eq!(
        stated(&["quiet", "--json", "--history", "36500d"]),
        None,
        "a typical change of two files is what a directory is for"
    );

    // Human output omits the aggregate at every scope and width.
    let rendered = |arguments: Vec<&str>| {
        let run = Invocation::new(arguments).run(repository.path());
        run.success();
        String::from_utf8(run.stdout).unwrap()
    };
    let root = rendered(vec!["--history", "36500d"]);
    assert!(!root.contains("A typical change here"), "{root}");
    let directory = rendered(vec!["core/src", "--history", "36500d"]);
    assert!(!directory.contains("A typical change here"), "{directory}");
    let narrow = Invocation::new(["core/src", "--history", "36500d"])
        .columns(50)
        .run(repository.path());
    narrow.success();
    let narrow_text = String::from_utf8(narrow.stdout).unwrap();
    assert!(
        !narrow_text.contains("A typical change here"),
        "{narrow_text}"
    );
    let file = rendered(vec!["core/src/unit0.js", "--history", "36500d"]);
    assert!(!file.contains("A typical change here"), "{file}");

    // The same tree read through the default window states no typical change:
    // the fixture's commits carry fixed dates the ninety-day window excludes, so
    // the stream stays complete and its sample is empty rather than shallow.
    // Every rated line of the block is byte-identical between the two runs,
    // which is what stated-only means.
    let default_run = Invocation::new(["--json"]).run(repository.path());
    default_run.success();
    let default_report = checked_json(&default_run.stdout);
    assert_eq!(
        default_report["history_coverage"]["availability"],
        "complete"
    );
    assert_eq!(default_report["history_coverage"]["commits"], 0);
    assert_eq!(stated(&["--json"]), None);
    let windowed = rendered(vec![]);
    assert!(!windowed.contains("A typical change here"), "{windowed}");
    assert_eq!(
        rated_verdict_lines(&root),
        rated_verdict_lines(&windowed),
        "amplification moves no tier, no count, and no worst offender"
    );
}

/// Both leakage kinds, their hand-calculated operands, and every case that
/// must produce nothing, from one invocation.
///
/// The importer three directories away shares 7 of the interface's 12 commits;
/// the two unlinked pairs share 6 of 9 each across two packages neither of
/// which can reach the other. Those two state the claiming rule from both
/// sides: the model's pair raises a card of its own because its lower-indexed
/// file carries nothing, and the rules file's pair is one more line on the card
/// that file already had — the line that has to name the partner, because
/// nothing else on that card does. Four cases produce nothing: the test that
/// changes with the file it exercises, the two files of one directory, the Rust
/// parent and the module its child declares, and the pair three commits short
/// of the support floor. The last of those is retained, which is what makes it
/// evidence that a retained pair is not a finding.
#[test]
fn the_leakage_fixture_states_both_kinds_and_every_absence() {
    let repository = change_leakage_repository();
    let history = ["--json", "--history", "36500d"];
    let result = Invocation::new(history).run(repository.path());
    result.success();
    let automatic = Invocation::new(history)
        .automatic_workers()
        .run(repository.path());
    assert_eq!(result, automatic, "serial and parallel runs must agree");
    let report = checked_json(&result.stdout);

    let pairs: Vec<_> = report["file_change_coupling"]
        .as_array()
        .unwrap()
        .iter()
        .map(|pair| {
            (
                file_path(&report, pair["left"].as_u64().unwrap()),
                file_path(&report, pair["right"].as_u64().unwrap()),
                pair["shared_commits"].as_u64().unwrap(),
                pair["union_commits"].as_u64().unwrap(),
                pair["distance"].as_u64().unwrap(),
            )
        })
        .collect();
    let expected: Vec<_> = [
        ("app/interface.js", "web/src/follower.js", 7, 12, 3),
        ("data/src/model.js", "data/store/lib/keys.js", 6, 9, 3),
        ("data/src/rules.js", "data/store/lib/cache.js", 6, 9, 3),
        ("edge/lib/b.js", "edge/src/a.js", 3, 3, 2),
        ("wiring/src/a/b/mod.rs", "wiring/src/lib.rs", 6, 6, 2),
    ]
    .into_iter()
    .map(|(left, right, shared, union, distance)| {
        (left.to_owned(), right.to_owned(), shared, union, distance)
    })
    .collect();
    assert_eq!(
        pairs, expected,
        "no pair names the test file and none names two files of one directory"
    );

    let findings: Vec<_> = report["change_leakage_findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| {
            (
                finding["kind"].as_str().unwrap().to_owned(),
                finding["coupling"].as_u64().unwrap(),
                finding["interface"]
                    .as_u64()
                    .map(|file| file_path(&report, file)),
            )
        })
        .collect();
    assert_eq!(
        findings,
        [
            (
                "leaky_interface".to_owned(),
                0,
                Some("app/interface.js".to_owned())
            ),
            ("hidden_coupling".to_owned(), 1, None),
            ("hidden_coupling".to_owned(), 2, None),
        ],
        "the wiring pair is connected by a module declaration and the edge pair is below the support floor"
    );

    // The hybrid claim, in both of its shapes: the interface and the rules file
    // already carry cards, so their leakage findings are one more line on those
    // cards rather than second cards about the same files, while the model's
    // pair — whose lower-indexed file carries nothing — raises a card of its
    // own.
    let cards: Vec<_> = report["problems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|card| {
            (
                card["pattern"].as_str().unwrap().to_owned(),
                card["visibility"].as_str().unwrap().to_owned(),
                serde_json::to_string(&card["claimed"]).unwrap(),
            )
        })
        .collect();
    assert_eq!(
        cards,
        [
            (
                "hot_mess".to_owned(),
                "default".to_owned(),
                "[{\"index\":0,\"table\":\"findings\"},{\"index\":0,\"table\":\"change_leakage_findings\"}]".to_owned()
            ),
            (
                "measured".to_owned(),
                "default".to_owned(),
                "[{\"index\":1,\"table\":\"findings\"},{\"index\":2,\"table\":\"change_leakage_findings\"}]".to_owned()
            ),
            (
                "hidden_coupling".to_owned(),
                "default".to_owned(),
                "[{\"index\":1,\"table\":\"change_leakage_findings\"}]".to_owned()
            ),
        ]
    );

    let terminal = Invocation::new(["--history", "36500d"]).run(repository.path());
    terminal.success();
    let text = String::from_utf8(terminal.stdout).unwrap();
    assert_eq!(
        problem_body(&text),
        [
            "  high hot and complex · app/interface.js",
            "        app/interface.js:1 · function · cognitive 6",
            "        1 rated unit",
            "        web/src/follower.js changed with it in 7 of 12 commits · 58% · 3 directories away",
            // The claimed hidden finding names the partner, because this card's
            // head names one file and nothing else on the card would.
            "  watch rule · function · data/src/rules.js:1",
            "        cognitive 3",
            "        changed with data/store/lib/cache.js in 6 of 9 commits · 67% · no dependency either way · 3 directories away",
            "        hot (9 commits)",
            // The standalone card's head names both files, so its line states
            // the proof instead.
            "  watch change together without a dependency · data/src/model.js ↔ data/store/lib/keys.js",
            "        changed together in 6 of 9 commits · 67% · no dependency either way · 3 directories away",
        ]
    );
    for absent in [
        "interface.test.js",
        "web/src/one.js",
        "wiring/src/lib.rs",
        "edge/",
    ] {
        assert!(!text.contains(absent), "{absent}: {text}");
    }
}

/// The strict launch: the fixture's default view gains one card, every scope
/// still fits one screen, and the pair below every floor reaches the machine
/// report and no human view.
#[test]
fn the_leakage_fixture_gains_one_default_card_and_states_no_weak_pair() {
    /// The slots a codebase view spends on problems, stated once more where
    /// the evidence is read.
    const SCREEN_BUDGET: usize = 24;

    let repository = change_leakage_repository();
    let rendered = |arguments: Vec<&str>| {
        let run = Invocation::new(arguments.clone()).run(repository.path());
        run.success();
        String::from_utf8(run.stdout).unwrap()
    };
    // The fixture's commits carry fixed dates, so a one-day window renders the
    // same tree without history at all.
    for scope in [".", "app", "data", "data/src", "web/src"] {
        let with = rendered(vec![scope, "--history", "36500d"]);
        let without = rendered(vec![scope, "--history", "1d"]);
        let gained = problem_heads(&with).len() - problem_heads(&without).len();
        let expected = usize::from(scope == "." || scope == "data");
        assert_eq!(gained, expected, "{scope}: {with}{without}");
        assert!(
            problem_body(&with).len() <= SCREEN_BUDGET,
            "{scope}: {with}"
        );
    }

    // A retained pair that met no detector's thresholds reaches the machine
    // report and no view at any scope or detail level.
    for arguments in [
        vec!["--history", "36500d"],
        vec!["--all", "--history", "36500d"],
        vec!["--top", "5", "--history", "36500d"],
        vec!["edge", "--history", "36500d"],
        vec!["edge", "--all", "--history", "36500d"],
        vec!["edge/src/a.js", "--history", "36500d"],
        vec!["edge/src/a.js", "--all", "--history", "36500d"],
    ] {
        let text = rendered(arguments.clone());
        let body = problem_body(&text).join("\n");
        // The pair's identity is its two paths, and neither reaches a card,
        // an evidence line, or a scope of its own.
        for absent in ["edge/lib/b.js", "edge/src/a.js"] {
            assert!(!body.contains(absent), "{arguments:?}: {text}");
        }
        if arguments[0] == "edge" {
            assert!(body.is_empty(), "{arguments:?}: {text}");
        }
    }
}

#[test]
fn a_primary_dependency_direction_publishes_a_stable_dependency_finding() {
    let repository = stable_dependency_repository(false);
    let result = Invocation::new(["--json"]).run(repository.path());
    result.success();
    let automatic = Invocation::new(["--json"])
        .automatic_workers()
        .run(repository.path());
    assert_eq!(result, automatic);
    let report = checked_json(&result.stdout);
    let findings: Vec<_> = report["stable_dependency_findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| {
            (
                package_path(&report, finding["source"].as_u64().unwrap()),
                package_path(&report, finding["target"].as_u64().unwrap()),
                finding["references"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        findings,
        [("crates/a".to_owned(), "crates/b".to_owned(), 2)]
    );
    assert_golden("unified-stable-dependency.json", &result.stdout);

    let terminal = Invocation::new(["--all"]).run(repository.path());
    terminal.success();
    let text = String::from_utf8(terminal.stdout.clone()).unwrap();
    assert!(text.contains("depends on less stable code"), "{text}");
    assert_golden("unified-stable-dependency.terminal.txt", &terminal.stdout);

    let scoped = stable_dependency_repository(true);
    let scoped_result = Invocation::new(["--json"]).run(scoped.path());
    scoped_result.success();
    let scoped_report = checked_json(&scoped_result.stdout);
    assert!(
        scoped_report["stable_dependency_findings"]
            .as_array()
            .unwrap()
            .is_empty(),
        "a test-scoped import is not a production dependency direction"
    );
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

    // The generated repository commits are older than the default window, so
    // they never reach a default run's report: the stream is windowed and no
    // boundary reject is counted.
    assert_eq!(windowed["history_coverage"]["commits"], 0);
    assert_eq!(windowed["history_coverage"]["window_excluded_commits"], 0);
    assert!(complete["history_coverage"]["commits"].as_u64() > Some(0));
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
fn hot_production_debt_outranks_equally_rated_cold_and_test_debt() {
    let repository = deepened_signal_repository();
    let result = Invocation::new(["--all", "--history", "36500d"]).run(repository.path());
    result.success();
    let terminal = String::from_utf8(result.stdout.clone()).unwrap();
    let position = |needle: &str| {
        terminal
            .find(needle)
            .unwrap_or_else(|| panic!("missing {needle} in\n{terminal}"))
    };
    // Every file is rated watch, so rating decides nothing here.
    //
    // Role class first: the signal-heavy test file carries three signals at the
    // watch rating where the production files carry one, and its path sorts
    // before theirs, so only the role class key can keep production debt above
    // it. A rank that read the signal counts first would list it at the top.
    assert!(position("src/cold.js") < position("spec/rich.js"));
    assert!(position("src/hot.js") < position("spec/rich.js"));
    // Hot second: `src/rich.js` is production too, so role class ties, and it
    // carries three signals at the watch rating against the hot file's one.
    // Only the hot key can place the hot file first.
    assert!(position("src/hot.js") < position("src/rich.js"));
    // The hot file also carries fewer statements than the cold file, so only
    // the hot rank key can place it first.
    assert!(position("src/hot.js") < position("src/cold.js"));
    // Non-primary debt stays visible below production rather than being
    // removed, in both its signal-heavy and its plain form.
    assert!(position("src/cold.js") < position("spec/cold.js"));

    let json = Invocation::new(["--json", "--history", "36500d"]).run(repository.path());
    json.success();
    let report = checked_json(&json.stdout);
    let ratings: HashSet<_> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| finding["rating"].as_str().unwrap())
        .collect();
    assert_eq!(ratings, HashSet::from(["watch"]));
    // The counterweight really is signal-heavier than the production debt that
    // now outranks it, so the ordering above cannot be an accident of ties.
    // The default watch thresholds, as documented in the README.
    let watch_thresholds = [
        ("cognitive_complexity", 15),
        ("cyclomatic_complexity", 11),
        ("logical_lines", 50),
    ];
    let signals_at_watch = |path: &str| {
        let file = report["files"]
            .as_array()
            .unwrap()
            .iter()
            .position(|file| report["paths"][file["path"].as_u64().unwrap() as usize] == path)
            .unwrap();
        report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|finding| finding["file"].as_u64().unwrap() as usize == file)
            .map(|finding| {
                watch_thresholds
                    .iter()
                    .filter(|(name, watch)| {
                        finding["measurements"][name].as_u64().unwrap() >= *watch
                    })
                    .count()
            })
            .max()
            .unwrap()
    };
    assert_eq!(signals_at_watch("src/rich.js"), 3);
    assert_eq!(signals_at_watch("spec/rich.js"), 3);
    assert_eq!(signals_at_watch("src/hot.js"), 1);
    assert_eq!(signals_at_watch("src/cold.js"), 1);
    let hot = report["files"]
        .as_array()
        .unwrap()
        .iter()
        .find(|file| report["paths"][file["path"].as_u64().unwrap() as usize] == "src/hot.js")
        .unwrap();
    assert_eq!(
        report["activity"][hot["activity"].as_u64().unwrap() as usize]["touches"],
        5
    );
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
fn top_counts_the_problem_cards_a_codebase_view_shows() {
    let repository = worktree_change_repository();
    // A limit of one keeps the single worst card and drops the rest.
    let top_one = Invocation::new(["--top", "1", "--history", "36500d"]).run(repository.path());
    top_one.success();
    let limited = String::from_utf8_lossy(&top_one.stdout).into_owned();
    assert_eq!(problem_heads(&limited).len(), 1, "{limited}");
    assert_eq!(
        problem_heads(&limited)[0],
        "  high circular dependency · c/main.js",
        "{limited}"
    );
    assert!(!limited.contains("renamed"), "{limited}");
    assert!(!limited.contains("b/main.js:1"), "{limited}");
    // A limit above the cards shows them all and adds no filler row; it buys
    // that breadth with the evidence depth its own rung allows, so it is not
    // the default view byte for byte.
    let concise = Invocation::new(["--history", "36500d"]).run(repository.path());
    concise.success();
    let concise_text = String::from_utf8_lossy(&concise.stdout).into_owned();
    let top_ten = Invocation::new(["--top", "10", "--history", "36500d"]).run(repository.path());
    top_ten.success();
    let ten = String::from_utf8_lossy(&top_ten.stdout).into_owned();
    assert_eq!(problem_heads(&ten), problem_heads(&concise_text), "{ten}");
    assert!(
        problem_body(&ten).len() < problem_body(&concise_text).len(),
        "{ten}"
    );

    // Zero and both documented conflicts are rejected before any analysis.
    let zero = Invocation::new(["--top", "0"]).run(repository.path());
    assert_eq!(zero.status.code(), Some(2));
    assert!(zero.stdout.is_empty());
    assert!(
        zero.stderr_text()
            .contains("use a whole number greater than zero"),
        "{}",
        zero.stderr_text()
    );
    for conflict in [["--top", "1", "--json"], ["--top", "1", "--all"]] {
        let rejected = Invocation::new(conflict).run(repository.path());
        assert_eq!(rejected.status.code(), Some(2));
        assert!(rejected.stdout.is_empty());
        assert!(
            rejected.stderr_text().contains("cannot be used with"),
            "{}",
            rejected.stderr_text()
        );
    }
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
    let terminal_text = String::from_utf8_lossy(&terminal.stdout);
    assert!(
        terminal_text.ends_with("  inspect directories and files for more details\n"),
        "{terminal_text}"
    );
    assert!(!terminal_text.contains("next:"), "{terminal_text}");
    let parallel_terminal = Invocation::new(["diff", "main", "--all", "--history", "36500d"])
        .automatic_workers()
        .run(repository.path());
    parallel_terminal.success();
    assert_eq!(terminal, parallel_terminal);
    assert_golden("unified-worktree-diff.terminal.txt", &terminal.stdout);
    // The added unit has an after side only, so its card states that side.
    assert!(
        String::from_utf8(terminal.stdout.clone()).unwrap().contains(
            "        added · cognitive 3 · cyclomatic 3 · statements 2 · nesting 2 · parameters 1\n"
        ),
        "{}",
        String::from_utf8_lossy(&terminal.stdout)
    );
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
    // A diff answers about a change rather than about a tree, so it carries no
    // propagation fact even though its package graph keeps its reach counts.
    assert!(report["verdict"]["reach"].is_null());
    assert!(report["verdict"]["core_size"].is_null());
    assert!(
        report["package_graph"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["reach_in"].as_u64().is_some())
    );
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
    let mut command = Command::new("git");
    hermetic_env(&mut command);
    let status = command
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
    assert_eq!(
        strip_decorations(&strip_ansi(&narrow_colored.stdout)),
        narrow_plain.stdout
    );
    assert_decoration_only_ansi(&narrow_colored.stdout);
    assert_max_display_width(&narrow_colored.stdout, 50, "colored codebase");
    let plain = Invocation::new(["--all", "--history", "36500d"]).run(repository.path());
    let colored = Invocation::new(["--all", "--history", "36500d"])
        .color("always")
        .run(repository.path());
    plain.success();
    colored.success();
    assert_golden("unified-codebase-decorated.terminal.txt", &colored.stdout);
    // A pipe receives the same report stated in words alone.
    assert_eq!(
        strip_decorations(&strip_ansi(&colored.stdout)),
        plain.stdout
    );
    assert_decoration_only_ansi(&colored.stdout);
    let colored_diff = Invocation::new(["diff", "main", "--all", "--history", "36500d"])
        .color("always")
        .run(repository.path());
    colored_diff.success();
    assert_decoration_only_ansi(&colored_diff.stdout);
    let colored_text = String::from_utf8_lossy(&colored.stdout);
    let colored_diff_text = String::from_utf8_lossy(&colored_diff.stdout);
    // Every glyph decorates the word beside it and never replaces it.
    for (glyph, word) in [('', "high"), ('', "watch"), ('', "next:")] {
        assert!(
            colored_text.contains(&format!("{glyph}\u{1b}[0m {word} ")),
            "{word}: {colored_text}"
        );
    }
    for (glyph, word) in [('', "worse"), ('', "better")] {
        assert!(
            colored_diff_text.contains(&format!("{glyph}\u{1b}[0m {word} ")),
            "{word}: {colored_diff_text}"
        );
    }
    assert!(colored_text.contains('\u{258c}'), "{colored_text}");
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

/// The one-screen invariant: zooming in changes which problems fill the
/// budget and never how much is printed.
#[test]
fn every_default_codebase_view_spends_at_most_the_screen_budget() {
    /// The slots a codebase view spends on problems, which is the renderer's
    /// budget stated once more where the evidence is read.
    const SCREEN_BUDGET: usize = 24;

    // A cycle witness is one fact stated one step per line, so its steps
    // beyond the first are exempt from the accounting: the budget counts the
    // evidence item and never its steps.
    let slots = |text: &str| {
        let rows = problem_body(text);
        let steps = rows
            .iter()
            .filter(|line| line.trim_start().starts_with("→ "))
            .count();
        rows.len() - steps
    };

    let languages = GeneratedRepository::new("main");
    copy_language_truth_files(&languages);
    // A directory scope over every supported language, which is the shape the
    // reported regression printed more than a thousand rows for.
    let directory = Invocation::new(["src"]).run(languages.path());
    directory.success();
    let text = String::from_utf8(directory.stdout).unwrap();
    assert!(slots(&text) <= SCREEN_BUDGET, "{text}");
    assert!(!problem_heads(&text).is_empty(), "{text}");

    // A cycle-bearing repository spends the budget the same way while its
    // witnesses render in full, because eliding a witness destroys its
    // meaning rather than shortening it.
    let cycles = static_architecture_repository();
    let cyclic = Invocation::new(["--history", "36500d"]).run(cycles.path());
    cyclic.success();
    let cyclic = String::from_utf8(cyclic.stdout).unwrap();
    let rows = problem_body(&cyclic);
    let heads = problem_heads(&cyclic);
    assert!(!heads.is_empty(), "{cyclic}");
    assert!(rows.len() > heads.len(), "{cyclic}");
    assert!(slots(&cyclic) <= SCREEN_BUDGET, "{cyclic}");
    for head in &heads {
        assert!(head.contains("circular dependency"), "{cyclic}");
    }
    // Every witness closes its cycle and none is shortened.
    let steps: Vec<&str> = rows
        .iter()
        .copied()
        .filter(|line| line.trim_start().starts_with("→ "))
        .collect();
    assert!(steps.len() >= heads.len() * 2, "{cyclic}");
    assert!(!cyclic.contains('\u{2026}'), "{cyclic}");

    // The next command opens the first visible problem exactly.
    let drill = wide_directory_repository();
    let root = Invocation::new(Vec::<&str>::new()).run(drill.path());
    root.success();
    let root = String::from_utf8(root.stdout).unwrap();
    let hint = root
        .lines()
        .find_map(|line| line.trim().strip_prefix("next: smackdebt "))
        .unwrap_or_else(|| panic!("{root}"))
        .to_owned();
    assert!(drill.path().join(&hint).is_file(), "{hint}: {root}");
    assert!(
        problem_heads(&root).iter().any(|line| line.contains(&hint)),
        "{hint}: {root}"
    );
    let target = Invocation::new([hint.as_str()]).run(drill.path());
    target.success();
    let target = String::from_utf8(target.stdout).unwrap();
    assert!(!problem_heads(&target).is_empty(), "{hint}: {target}");

    // One invocation states the same cards and the same evidence at every
    // width, because the budget counts slots rather than rendered lines.
    let facts = |width| {
        let result = Invocation::new(["src"])
            .columns(width)
            .run(languages.path());
        result.success();
        let body = String::from_utf8(result.stdout).unwrap();
        problem_body(&body)
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let wide = facts(120);
    assert_eq!(facts(80), wide);
    assert_eq!(facts(50), wide);
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
    assert_eq!(checked, 7, "expected every reviewed 50-column view");
}

/// Both suites commit their terminal results here, so one scan proves the
/// invariant over every reviewed human view at every scope and detail level.
#[test]
fn no_committed_terminal_result_states_a_dependency_edge_as_a_row() {
    let snapshots = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots");
    let mut checked = 0;
    for entry in fs::read_dir(snapshots).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.ends_with(".terminal.txt") {
            continue;
        }
        let bytes = strip_decorations(&strip_ansi(&fs::read(entry.path()).unwrap()));
        assert_no_dependency_edge_rows(&String::from_utf8_lossy(&bytes), &name);
        checked += 1;
    }
    assert_eq!(checked, 37, "expected every committed terminal view");
}

#[test]
fn every_piped_public_flow_is_words_only() {
    let repository = worktree_change_repository();
    for arguments in [
        vec!["--history", "36500d"],
        vec!["--all", "--history", "36500d"],
        vec!["a", "--all", "--history", "36500d"],
        vec!["a/main.js", "--all", "--history", "36500d"],
        vec!["diff", "main", "--history", "36500d"],
        vec!["diff", "main", "--all", "--history", "36500d"],
        vec!["diff", "main", "b", "--all", "--history", "36500d"],
    ] {
        let result = Invocation::new(arguments.clone()).run(repository.path());
        result.success();
        assert_eq!(
            private_use_codepoints(&result.stdout),
            Vec::<char>::new(),
            "{arguments:?}"
        );
        assert!(
            !result.stdout.windows(2).any(|bytes| bytes == b"\x1b["),
            "{arguments:?}"
        );
        let text = String::from_utf8(result.stdout).unwrap();
        assert!(text.starts_with("smackdebt"), "{arguments:?}: {text}");
        assert!(!text.contains('\u{258c}'), "{arguments:?}: {text}");
        assert!(!text.contains('\u{2026}'), "{arguments:?}: {text}");
    }
}

/// Every private-use codepoint, which may never reach a machine consumer.
fn private_use_codepoints(bytes: &[u8]) -> Vec<char> {
    String::from_utf8_lossy(bytes)
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

    let missing_path = Invocation::new(["does/not/exist"]).run(repository.path());
    assert_eq!(
        missing_path.status.code(),
        facts["expected_missing_path_exit"]
            .as_i64()
            .map(|v| v as i32)
    );
    assert!(missing_path.stdout.is_empty());
    assert_eq!(
        missing_path.stderr_text(),
        facts["expected_missing_path_stderr"]
    );
    assert_short_terminal_text(&missing_path.stderr);

    let all_json = Invocation::new(["--all", "--json"]).run(repository.path());
    assert_eq!(
        all_json.status.code(),
        facts["expected_all_json_exit"].as_i64().map(|v| v as i32)
    );
    assert!(all_json.stdout.is_empty());
    assert_eq!(all_json.stderr_text(), facts["expected_all_json_stderr"]);
    assert_short_terminal_text(&all_json.stderr);

    let diff_all_json = Invocation::new(["diff", "main", "--all", "--json"]).run(repository.path());
    assert_eq!(diff_all_json.status.code(), Some(2));
    assert!(diff_all_json.stdout.is_empty());
    assert_eq!(
        diff_all_json.stderr_text(),
        facts["expected_all_json_stderr"]
    );

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
fn every_codebase_tier_states_its_own_sentence_and_counts() {
    // One generated repository per reachable tier, so the frozen sentences are
    // proven from public bytes rather than from analysis unit tests.
    let empty = GeneratedRepository::new("main");
    empty.write("notes.txt", b"nothing to check\n");
    let clean = GeneratedRepository::new("main");
    clean.write("main.js", b"export function small() { return 1; }\n");
    let solid = GeneratedRepository::new("main");
    solid.write("main.js", watch_source(1).as_bytes());
    let worn = GeneratedRepository::new("main");
    worn.write("main.js", &mixed_source(1, 200));
    // Ten High in 200 checked units is 50 permille with enough density
    // evidence and enough absolute High debt to escape the small-scope cap.
    let fights_back = GeneratedRepository::new("main");
    fights_back.write("main.js", &mixed_source(10, 190));
    // Twelve High in 20 checked units reaches the small-scope High threshold,
    // so the saturated 600 permille selects lost.
    let lost = GeneratedRepository::new("main");
    lost.write("main.js", &mixed_source(12, 8));

    for (repository, tier, sentence) in [
        (&empty, "empty", "Nothing was checked."),
        (&clean, "clean", "Clean. Ship it."),
        (&solid, "solid", "Solid, with rough edges."),
        (&worn, "worn", "Worn in the usual places."),
        (&fights_back, "fights_back", "This code fights back."),
        (&lost, "lost", "The code is winning."),
    ] {
        let result = Invocation::new(["--history", "36500d"]).run(repository.path());
        result.success();
        let text = String::from_utf8(result.stdout).unwrap();
        let mut lines = text.lines();
        assert_eq!(lines.next(), Some("smackdebt · repository root"), "{tier}");
        assert_eq!(
            lines.next(),
            Some(format!("  {sentence}").as_str()),
            "{tier}: {text}"
        );
        let counts = lines.next().unwrap_or_default();
        assert!(
            counts.contains(" high · ")
                && counts.contains(" watch · ")
                && counts.ends_with(" checked"),
            "{tier}: {counts}"
        );
    }
}

#[test]
fn tier_volume_boundaries_hold_at_exactly_one_unit() {
    // The small-scope cap and the volume floors are proven from public bytes
    // at their exact boundaries, one unit either side where the tier moves.
    for (name, high, healthy, sentence) in [
        // One High in five checked units is 200 permille, which used to be
        // lost; the small-scope cap holds it at worn.
        ("tiny scope", 1, 4, "Worn in the usual places."),
        // Nine High at 199 checked units is one unit short of the density
        // evidence threshold, so the 45 permille stays capped at worn.
        (
            "evidence boundary below",
            9,
            190,
            "Worn in the usual places.",
        ),
        // The same debt with one more checked unit has enough evidence.
        ("evidence boundary at", 9, 191, "This code fights back."),
        // 99 High in 10000 checked units is 9 permille and below the volume
        // floor, so density keeps the scope worn.
        ("volume floor below", 99, 9901, "Worn in the usual places."),
        // One more High finding reaches the absolute floor: 100 High is 10
        // permille — worn by density — but fights back on volume alone.
        ("volume floor at", 100, 9900, "This code fights back."),
    ] {
        let repository = GeneratedRepository::new("main");
        repository.write("main.js", &mixed_source(high, healthy));
        let result = Invocation::new(["--history", "36500d"]).run(repository.path());
        result.success();
        let text = String::from_utf8(result.stdout).unwrap();
        let mut lines = text.lines();
        assert_eq!(lines.next(), Some("smackdebt · repository root"), "{name}");
        assert_eq!(
            lines.next(),
            Some(format!("  {sentence}").as_str()),
            "{name}: {text}"
        );
    }
}

/// Source with `high` units and enough healthy units to place the High share
/// in the requested tier band.
fn mixed_source(high: usize, healthy: usize) -> Vec<u8> {
    let mut source = String::new();
    for index in 0..high {
        source.push_str(&high_source(index));
    }
    for index in 0..healthy {
        source.push_str(&format!(
            "export function healthy{index}() {{ return {index}; }}\n"
        ));
    }
    source.into_bytes()
}

fn high_source(index: usize) -> String {
    let mut source = format!("export function heavy{index}(input) {{\n");
    for depth in 0..26 {
        source.push_str(&format!("  if (input > {depth}) {{ input += {depth}; }}\n"));
    }
    source.push_str("  return input;\n}\n");
    source
}

fn watch_source(index: usize) -> String {
    let mut source = format!("export function moderate{index}(input) {{\n");
    for depth in 0..16 {
        source.push_str(&format!("  if (input > {depth}) {{ input += {depth}; }}\n"));
    }
    source.push_str("  return input;\n}\n");
    source
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
            "Signal Fixture",
            "signal@example.invalid",
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
                    "  gate  Check debt against a committed baseline\n",
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
                    "      --top <TOP>          Show up to this many problems or comparisons\n",
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
        "AREAS",
        "PROBLEMS",
        "WARNINGS",
        // The diff report keeps its own three sections this release.
        "FINDINGS",
        "ARCHITECTURE",
        "HISTORY",
        // Every frozen pattern id beside the words the terminal prints for it.
        "| `god_file` | `does too much` |",
        "| `hub` | `everything depends on this`, `depends on many files`, or `change spreads far` |",
        "| `tangle` | `circular dependency` |",
        "| `hot_mess` | `hot and complex` |",
        "| `shotgun_pair` | `packages change together` |",
        "| `leaky_interface` | `importers follow its changes` |",
        "| `hidden_coupling` | `change together without a dependency` |",
        "| `bus_risk` | `one author` |",
        "| `unstable_dependency` | `depends on less stable code` |",
        "| `measured` |",
        "changed together in 33 of 98 commits · 34% · no direct dependency · linked via crates/output",
        "29% · no code dependency",
        "one contributor made 57 of 60 commits",
        // Verdict facts shown in captured output.
        "A change in one package can reach 6 of 12 packages.",
        "A change here can reach 17 of 36 files in this package.",
        "9 of 86 files sit in one dependency cycle.",
        "A typical change here touches 4 files.",
        "a change here reaches 14 files",
        "3 importers follow it",
        "instability 1/4 → 2/3",
        "hot (7 commits)",
        "2 files in the cycle",
        "imports 13 files",
        "11 files import this",
        "GraphEditor.vue · closure",
        "next: smackdebt scripts",
        "worse 0 · better 0 · changed 0",
        "Not all source was checked.",
        "<analyzed> of <selected> source files were analyzed.",
        // Repository share is available only when both totals were measured.
        "A fresh explicit file or directory command inspects only that selection.",
        "A retained sub-scope from a completed root report may carry `verdict.share`",
        "smackdebt: path not found: does/not/exist",
        "smackdebt: no source files found under: docs",
        "smackdebt: not a source file: README.txt",
        "smackdebt: Git ref not found: no-such-ref",
        "smackdebt: --all cannot be used with --json",
        "smackdebt: baseline not found: .smackdebt-baseline.tsv",
        "| 3 | Gate baseline exceeded |",
        "U+E000–U+F8FF",
        "`fights_back`",
        "`no_debt_change`",
    ] {
        assert!(readme.contains(required), "README is missing {required}");
    }
    // `change together without a dependency` was banned while it belonged to a
    // deleted coupling section, to prove the section had not crept back. It is
    // now the human name of the frozen pattern id `hidden_coupling`, so the
    // documentation must carry it and the assertion moved to the required list
    // above: a shipped feature cannot be documented under a ban.
    for removed in [
        "QUALITY",
        "touches count distinct commits",
        "external uses · primary/trusted",
        "unresolved uses · primary/trusted",
        "coupling finding",
        "ASCII fallback",
        "rated units ·",
        "Policy gates belong to a later release.",
        // The edge rows left the human terminal with the section that carried
        // them, so the README may not document them as terminal wording.
        "source → target · 1 import",
        "source owns target",
        "· 1 import",
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
fn explicit_astro_scopes_have_limited_work_evidence() {
    let repository = GeneratedRepository::new("main");
    repository.write("package.json", b"{}\n");
    repository.write("page.astro", b"<h1>Before</h1>\n");
    repository.write("outside.rs", b"fn outside() {}\n");
    repository.commit(Commit {
        message: "base",
        identity: Identity {
            name: "Scope Evidence",
            address: "scope@example.invalid",
        },
        date: "2026-01-01T12:00:00Z",
    });

    let codebase = Invocation::new(["page.astro", "--json"])
        .evidence()
        .run(repository.path());
    assert_eq!(
        codebase.status.code(),
        Some(0),
        "{}",
        codebase.stderr_text()
    );
    let report = checked_json(&codebase.stdout);
    assert_eq!(report["files"].as_array().unwrap().len(), 1);
    assert_eq!(report["files"][0]["language"], "astro");
    assert_eq!(report["files"][0]["trust"], "failed");
    assert_eq!(report["scopes"][0]["coverage"]["unsupported_files"], 1);
    assert!(report["findings"].as_array().unwrap().is_empty());
    assert!(report["dependency_edges"].as_array().unwrap().is_empty());
    let codebase_stats = evidence_stats(&codebase);
    assert_eq!(codebase_stats["inventory_walks"], 1);
    assert_eq!(codebase_stats["source_reads"], 0);
    assert_eq!(codebase_stats["parser_visits"], 0);

    fs::remove_file(repository.path().join("outside.rs")).unwrap();
    repository.commit(Commit {
        message: "remove outside source",
        identity: Identity {
            name: "Scope Evidence",
            address: "scope@example.invalid",
        },
        date: "2026-01-02T12:00:00Z",
    });
    repository.write("page.astro", b"<h1>After</h1>\n");
    let diff = Invocation::new(["diff", "HEAD", "--json"])
        .evidence()
        .run(repository.path());
    assert_eq!(diff.status.code(), Some(0), "{}", diff.stderr_text());
    let report = checked_json(&diff.stdout);
    assert_eq!(report["files"][0]["language"], "astro");
    assert_eq!(report["files"][0]["trust"], "failed");
    assert_eq!(report["scopes"][0]["coverage"]["unsupported_files"], 1);
    assert_eq!(
        report["diff_graph_evidence"]["current"]["status"],
        "incomplete"
    );
    assert_eq!(
        report["diff_graph_evidence"]["base"]["status"],
        "incomplete"
    );
    assert!(report["findings"].as_array().unwrap().is_empty());
    assert!(report["dependency_edges"].as_array().unwrap().is_empty());
    assert_eq!(evidence_stats(&diff)["parser_visits"], 0);
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
            [1, 29, 8, 26, 7, 15, 28],
        ),
        (
            "worktree diff JSON",
            vec!["diff", "main", "--json", "--history", "36500d"],
            [1, 29, 8, 26, 7, 15, 28],
        ),
        (
            "package terminal",
            vec!["a", "--all"],
            [1, 4, 1, 0, 3, 1, 3],
        ),
        ("package JSON", vec!["a", "--json"], [1, 4, 1, 0, 3, 1, 3]),
        (
            "file terminal",
            vec!["a/main.js", "--all"],
            [1, 3, 1, 0, 3, 1, 3],
        ),
        (
            "file JSON",
            vec!["a/main.js", "--json"],
            [1, 3, 1, 0, 3, 1, 3],
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
        assert_evidence_flow(name, arguments, reference.path(), [1, 29, 8, 26, 7, 15, 28]);
    }

    let languages = GeneratedRepository::new("main");
    copy_language_truth_files(&languages);
    for (name, arguments) in [
        ("directory terminal", vec!["src", "--all"]),
        ("directory JSON", vec!["src", "--json"]),
    ] {
        assert_evidence_flow(name, arguments, languages.path(), [1, 13, 11, 0, 3, 11, 13]);
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
        let mut invocation = Command::new(&command);
        hermetic_env(&mut invocation);
        let output = invocation.args(arguments).output().unwrap();
        assert_eq!(output.status.code(), Some(0));
        assert!(!output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
    for arguments in [
        vec!["--color", "never", "--jobs", "1"],
        vec!["--json", "--jobs", "1"],
    ] {
        let mut invocation = Command::new(&command);
        hermetic_env(&mut invocation);
        let output = invocation
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
    // Codebase debt is problem cards, so the terminal states the actionable
    // pair once and a pair a code dependency explains keeps its complete row
    // in the machine report alone.
    let coupling_lines: Vec<_> = text.lines().filter(|line| line.contains(" ↔ ")).collect();
    assert_eq!(coupling_lines.len(), 1, "{coupling_lines:?}");
    assert_eq!(
        coupling_lines[0],
        "  watch packages change together · rb ↔ ui"
    );
    assert!(unique.len() > 1, "{unique:?}");
    assert!(
        text.contains(" · no code dependency\n"),
        "{coupling_lines:?}: {text}"
    );
}

#[test]
fn every_derived_signal_table_carries_its_exact_rows() {
    let repository = signal_table_repository();
    let result = Invocation::new(["--json", "--history", "36500d"]).run(repository.path());
    result.success();
    let parallel = Invocation::new(["--json", "--history", "36500d"])
        .automatic_workers()
        .run(repository.path());
    assert_eq!(result, parallel, "serial and parallel runs must agree");
    let report = checked_json(&result.stdout);

    let hot: Vec<_> = report["hotspots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hotspot| {
            (
                file_path(&report, hotspot["file"].as_u64().unwrap()),
                hotspot["rating"].as_str().unwrap().to_owned(),
                hotspot["touches"].as_u64().unwrap(),
            )
        })
        .collect();
    // A hot file is a rated file that changes often, so a file whose units are
    // all healthy is hot too and states `healthy` as its maximum rating.
    assert_eq!(
        hot,
        [
            ("core/main.js".to_owned(), "watch".to_owned(), 13),
            ("helper/small.js".to_owned(), "healthy".to_owned(), 13),
        ]
    );

    let sizes: HashSet<(String, &str, u64)> = report["size_findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| {
            (
                file_path(&report, finding["file"].as_u64().unwrap()),
                finding["subject"].as_str().unwrap(),
                finding["value"].as_u64().unwrap(),
            )
        })
        .collect();
    assert!(
        sizes
            .iter()
            .any(|(path, subject, _)| path == "core/oversized.js" && *subject == "file"),
        "{sizes:?}"
    );
    assert!(
        sizes
            .iter()
            .any(|(path, subject, _)| path == "core/container.js" && *subject == "container"),
        "{sizes:?}"
    );

    let orphans: HashSet<String> = report["orphan_files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| file_path(&report, file.as_u64().unwrap()))
        .collect();
    assert!(orphans.contains("core/unused.js"), "{orphans:?}");

    let violations: Vec<_> = report["stable_dependency_findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| {
            (
                package_path(&report, finding["source"].as_u64().unwrap()),
                package_path(&report, finding["target"].as_u64().unwrap()),
                finding["source_fan_in"].as_u64().unwrap(),
                finding["source_fan_out"].as_u64().unwrap(),
                finding["target_fan_in"].as_u64().unwrap(),
                finding["target_fan_out"].as_u64().unwrap(),
                finding["references"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        violations,
        [("core".to_owned(), "util".to_owned(), 2, 1, 1, 1, 2)]
    );

    let concentration: Vec<_> = report["knowledge_concentration_findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| {
            (
                package_path(&report, finding["package"].as_u64().unwrap()),
                finding["contributor_count"].as_u64().unwrap(),
                finding["numerator"].as_u64().unwrap(),
                finding["denominator"].as_u64().unwrap(),
            )
        })
        .collect();
    assert!(
        concentration
            .iter()
            .any(
                |(package, contributors, numerator, denominator)| package == "core"
                    && *contributors == 1
                    && numerator == denominator
                    && *denominator >= 10
            ),
        "{concentration:?}"
    );
    // The finding states counts alone; the contributor never reaches output.
    for private in ["Solo Fixture", "solo@example.invalid"] {
        assert!(
            !result
                .stdout
                .windows(private.len())
                .any(|window| window == private.as_bytes())
        );
    }
}

fn checked_json(bytes: &[u8]) -> Value {
    let report: Value = serde_json::from_slice(bytes).unwrap();
    let schema: Value =
        serde_json::from_str(include_str!("../../../schemas/report-v4.schema.json")).unwrap();
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(&report)
        .unwrap();
    assert_index_integrity(&report);
    assert_recursive_privacy(&report);
    assert_no_float_or_boolean_value(&report, "$");
    assert_head_agrees_with_tables(&report);
    report
}

/// Version 4 serializes integers and strings only, so a floating-point value
/// or a boolean anywhere in the object is a contract failure.
///
/// This is why a problem card states its visibility as a string: a boolean
/// would answer one question and then have to be replaced when a third value
/// appears.
fn assert_no_float_or_boolean_value(value: &Value, path: &str) {
    match value {
        Value::Object(fields) => {
            for (name, value) in fields {
                assert_no_float_or_boolean_value(value, &format!("{path}.{name}"));
            }
        }
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                assert_no_float_or_boolean_value(value, &format!("{path}[{index}]"));
            }
        }
        Value::Number(number) => assert!(
            number.is_i64() || number.is_u64(),
            "floating-point value at {path}: {number}"
        ),
        Value::Bool(value) => panic!("boolean value at {path}: {value}"),
        _ => {}
    }
}

/// The denormalized head duplicates table facts on purpose, so acceptance
/// rebuilds every head value from the tables and compares them.
fn assert_head_agrees_with_tables(report: &Value) {
    assert_eq!(report["verdict"]["mode"], report["mode"]);
    assert!(
        report["verdict"]["sentence"]
            .as_str()
            .unwrap()
            .ends_with('.')
    );
    let summary = &report["summary"];
    let Some(answered) = answered_scope(report) else {
        assert_eq!(summary["checked"], 0);
        assert!(summary["worst"].as_array().unwrap().is_empty());
        return;
    };
    let scope = &report["scopes"][answered];
    let counts = &report["health"][scope["health"].as_u64().unwrap() as usize];
    assert_eq!(summary["high"], counts["high"]);
    assert_eq!(summary["watch"], counts["watch"]);
    assert_eq!(
        summary["checked"].as_u64().unwrap(),
        counts["healthy"].as_u64().unwrap()
            + counts["watch"].as_u64().unwrap()
            + counts["high"].as_u64().unwrap()
    );
    let architecture = scope["architecture_findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|id| {
            report["architecture_findings"][id.as_u64().unwrap() as usize]["rating"] == "high"
        })
        .count() as u64;
    assert_eq!(summary["high_architecture"].as_u64(), Some(architecture));

    let paths: HashSet<&str> = report["paths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|path| path.as_str().unwrap())
        .collect();
    let worst = summary["worst"].as_array().unwrap();
    assert!(worst.len() <= 3, "the head names at most three offenders");
    for offender in worst {
        let path = offender["path"].as_str().unwrap();
        assert!(paths.contains(path), "worst path {path} is not a real path");
        if offender["name"].is_null() {
            assert_eq!(offender["reason"], "package_dependency_cycle");
        }
    }
    assert_debt_diff_selection(report);
}

/// The scope the head answers, which is the selected scope when a path was
/// asked about and the repository otherwise.
fn answered_scope(report: &Value) -> Option<usize> {
    report["selected_scope"]
        .as_u64()
        .or_else(|| report["root"].as_u64())
        .map(|scope| scope as usize)
}

/// Rebuilds the debt-diff selection from the serialized tables.
///
/// This proves two things at once: that the head's counts are the counts the
/// tables imply, and that no comparison identity is selected twice, which
/// would count one movement as two.
fn assert_debt_diff_selection(report: &Value) {
    let answered = answered_scope(report).unwrap();
    let scope = &report["scopes"][answered];
    let rated = |value: &Value| value == "watch" || value == "high";
    let mut counts = (0_u64, 0_u64, 0_u64);
    let mut selected = HashSet::new();
    let count = |direction: &Value, counts: &mut (u64, u64, u64)| match direction.as_str().unwrap()
    {
        "worse" => counts.0 += 1,
        "better" => counts.1 += 1,
        other => {
            assert_eq!(other, "changed");
            counts.2 += 1;
        }
    };
    for id in scope["comparisons"].as_array().unwrap() {
        let id = id.as_u64().unwrap() as usize;
        let comparison = &report["comparisons"][id];
        let role = comparison["file"].as_u64().map_or("primary", |file| {
            report["files"][file as usize]["role"].as_str().unwrap()
        });
        if role == "fixture" || role == "generated" {
            continue;
        }
        let before = &comparison["ratings"]["before"];
        let after = &comparison["ratings"]["after"];
        let moves = match comparison["kind"].as_str().unwrap() {
            "regressed" | "improved" => true,
            "added" => rated(after),
            "removed" => rated(before),
            "metric_changed" => rated(before) || rated(after),
            _ => false,
        };
        if !moves {
            continue;
        }
        assert!(
            selected.insert(("source", id)),
            "comparison {id} selected twice"
        );
        count(&comparison["direction"], &mut counts);
    }
    for id in scope["architecture_comparisons"].as_array().unwrap() {
        let id = id.as_u64().unwrap() as usize;
        let comparison = &report["architecture_comparisons"][id];
        if !matches!(
            comparison["kind"].as_str().unwrap(),
            "cycle_introduced" | "cycle_removed"
        ) {
            continue;
        }
        assert!(
            selected.insert(("architecture", id)),
            "architecture comparison {id} selected twice"
        );
        count(&comparison["direction"], &mut counts);
    }
    for (scope_key, table_key, identity) in [
        (
            "propagation_comparisons",
            "propagation_comparisons",
            "propagation",
        ),
        ("core_comparisons", "core_comparisons", "core"),
        (
            "change_leakage_comparisons",
            "change_leakage_comparisons",
            "change_leakage",
        ),
    ] {
        for id in scope[scope_key].as_array().unwrap() {
            let id = id.as_u64().unwrap() as usize;
            assert!(
                selected.insert((identity, id)),
                "{identity} comparison {id} selected twice"
            );
            count(&report[table_key][id]["direction"], &mut counts);
        }
    }
    for id in scope["evolutionary_comparisons"].as_array().unwrap() {
        let id = id.as_u64().unwrap() as usize;
        assert!(
            selected.insert(("evolutionary", id)),
            "evolutionary comparison {id} selected twice"
        );
        count(
            &report["evolutionary_comparisons"][id]["direction"],
            &mut counts,
        );
    }
    let debt_diff = &report["summary"]["debt_diff"];
    assert_eq!(debt_diff["worse"].as_u64(), Some(counts.0));
    assert_eq!(debt_diff["better"].as_u64(), Some(counts.1));
    assert_eq!(debt_diff["changed"].as_u64(), Some(counts.2));
    assert_eq!(
        debt_diff["total"].as_u64(),
        Some(counts.0 + counts.1 + counts.2)
    );
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
    if let Some(answered) = report["selected_scope"]
        .as_u64()
        .or_else(|| report["root"].as_u64())
    {
        let coverage = &report["scopes"][answered as usize]["coverage"];
        match report["verdict"].get("qualifier") {
            Some(qualifier) => {
                assert_eq!(qualifier["selected_files"], coverage["selected_files"]);
                assert_eq!(qualifier["analyzed_files"], coverage["analyzed_files"]);
            }
            None => assert_eq!(coverage["selected_files"], coverage["analyzed_files"]),
        }
    }
    for table in [
        "packages",
        "stable_dependency_findings",
        "knowledge_concentration_findings",
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
        assert_evidence_role(&edge["role"], &report["files"][source]["role"]);
        assert_eq!(edge["trust"], report["files"][source]["trust"]);
    }
    for dependency in report["external_dependencies"].as_array().unwrap() {
        let file = dependency["file"].as_u64().unwrap() as usize;
        assert!(file < files);
        assert_evidence_role(&dependency["role"], &report["files"][file]["role"]);
        assert_eq!(dependency["trust"], report["files"][file]["trust"]);
        assert_single_line_target(&dependency["target"]);
    }
    for diagnostic in report["resolution_diagnostics"].as_array().unwrap() {
        let file = diagnostic["file"].as_u64().unwrap() as usize;
        assert!(file < files);
        assert_evidence_role(&diagnostic["role"], &report["files"][file]["role"]);
        assert_eq!(diagnostic["trust"], report["files"][file]["trust"]);
        assert_single_line_target(&diagnostic["target"]);
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
    // A retained file pair names the lower file identity first, crosses a
    // directory boundary, and compares two counts drawn from one population.
    let mut previous = None;
    for pair in report["file_change_coupling"].as_array().unwrap() {
        let left = pair["left"].as_u64().unwrap();
        let right = pair["right"].as_u64().unwrap();
        assert!((left as usize) < files, "pair left index is invalid");
        assert!((right as usize) < files, "pair right index is invalid");
        assert!(left < right, "a pair names the lower file identity first");
        let key = Some((left, right));
        assert!(previous < key, "pairs are ordered by file identity");
        previous = key;
        let shared = pair["shared_commits"].as_u64().unwrap();
        assert!(shared >= 3, "a retained pair clears the support floor");
        assert!(shared <= pair["union_commits"].as_u64().unwrap());
        assert!(shared * 10 >= pair["union_commits"].as_u64().unwrap());
        assert!(
            pair["distance"].as_u64().unwrap() >= 1,
            "a same-directory pair is never stored"
        );
        assert!(pair.get("similarity").is_none(), "no ratio is serialized");
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
        assert_eq!(finding["kind"], "unexplained_coupling");
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
        assert!(
            comparison["shared_commits"].as_u64().unwrap()
                <= comparison["union_commits"].as_u64().unwrap()
        );
    }
    let mut hot_files = HashSet::new();
    for hotspot in report["hotspots"].as_array().unwrap() {
        let file = hotspot["file"].as_u64().unwrap() as usize;
        assert!(file < files);
        assert!(hotspot["touches"].as_u64().unwrap() > 0);
        assert!(hot_files.insert(file), "one hotspot row per file");
    }
    for finding in report["size_findings"].as_array().unwrap() {
        assert!((finding["file"].as_u64().unwrap() as usize) < files);
        match finding["subject"].as_str().unwrap() {
            "file" => assert!(finding["container"].is_null()),
            _ => assert!(finding["container"].as_str().is_some_and(|c| !c.is_empty())),
        }
    }
    let mut orphans = HashSet::new();
    for orphan in report["orphan_files"].as_array().unwrap() {
        let file = orphan.as_u64().unwrap() as usize;
        assert!(file < files);
        assert!(orphans.insert(file), "one orphan row per file");
        assert_eq!(report["files"][file]["role"], "primary");
    }
    for row in report["package_graph"].as_array().unwrap() {
        assert!(
            row["reach_in"].as_u64().unwrap() >= 1,
            "a package always reaches itself"
        );
    }
    // A closure row exists only for a package the file floor admits, so the
    // table is joined by package and is never a complete package index.
    let mut closed = HashSet::new();
    let mut previous = None;
    for closure in report["package_closures"].as_array().unwrap() {
        let package = closure["package"].as_u64().unwrap() as usize;
        assert!(package < packages, "closure package index is invalid");
        assert!(closed.insert(package), "one closure row per package");
        assert!(
            previous.is_none_or(|earlier| earlier < package),
            "closures are ordered by package"
        );
        previous = Some(package);
        let held = closure["files"].as_u64().unwrap();
        let reach = closure["reach"].as_u64().unwrap();
        assert!(held >= 20, "a closure row is above the file floor");
        assert!(
            (1..=held).contains(&reach),
            "a reach counts the changed file and no file outside the package"
        );
    }
    let mut reached = HashSet::new();
    let mut previous = None;
    for row in report["file_reach"].as_array().unwrap() {
        let file = row["file"].as_u64().unwrap() as usize;
        assert!(file < files, "reach file index is invalid");
        assert!(reached.insert(file), "one reach row per file");
        assert!(
            previous.is_none_or(|earlier| earlier < file),
            "candidate reaches are ordered by file"
        );
        previous = Some(file);
        assert!(
            (row["reach"].as_u64().unwrap() as usize) < files,
            "an exact reach excludes the file itself"
        );
    }
    for finding in report["stable_dependency_findings"].as_array().unwrap() {
        assert!((finding["source"].as_u64().unwrap() as usize) < packages);
        assert!((finding["target"].as_u64().unwrap() as usize) < packages);
        assert_ne!(finding["source"], finding["target"]);
        for edge in finding["witness_edges"].as_array().unwrap() {
            assert!((edge.as_u64().unwrap() as usize) < file_edges);
        }
    }
    for finding in report["knowledge_concentration_findings"]
        .as_array()
        .unwrap()
    {
        assert!((finding["package"].as_u64().unwrap() as usize) < packages);
        assert!(finding["numerator"].as_u64().unwrap() <= finding["denominator"].as_u64().unwrap());
    }
    for package in report["packages"].as_array().unwrap() {
        if let Some(name) = package.get("manifest_name") {
            assert!(!name.as_str().unwrap().is_empty());
        }
    }
    assert_problem_integrity(report);
}

/// The tables a problem card may claim from, named the way the report names
/// them, so a claim resolves with one lookup.
const CLAIMABLE_TABLES: [&str; 7] = [
    "findings",
    "size_findings",
    "architecture_findings",
    "stable_dependency_findings",
    "evolutionary_findings",
    "knowledge_concentration_findings",
    "change_leakage_findings",
];

/// The frozen pattern ids, which are the only names a card may carry.
const PROBLEM_PATTERNS: [&str; 10] = [
    "god_file",
    "hub",
    "tangle",
    "hot_mess",
    "shotgun_pair",
    "bus_risk",
    "unstable_dependency",
    "measured",
    "leaky_interface",
    "hidden_coupling",
];

/// Validates the problem table: every anchor and evidence index resolves into
/// the table it names, every pattern and visibility is a frozen id, and the
/// claims cover the claimable tables exactly once each.
///
/// The claim audit has two halves and both are contract failures: a finding
/// claimed by two cards would count one problem twice, and a retained finding
/// claimed by no card would be a problem the report measured and then dropped.
fn assert_problem_integrity(report: &Value) {
    let mut claims: HashSet<(&str, u64)> = HashSet::new();
    for problem in report["problems"].as_array().unwrap() {
        let pattern = problem["pattern"].as_str().unwrap();
        assert!(
            PROBLEM_PATTERNS.contains(&pattern),
            "unknown problem pattern {pattern}"
        );
        let visibility = problem["visibility"].as_str().unwrap();
        assert!(
            visibility == "default" || visibility == "detail",
            "unknown problem visibility {visibility}"
        );
        assert_problem_anchor(report, &problem["anchor"]);
        assert_problem_evidence(report, &problem["evidence"]);
        for claim in problem["claimed"].as_array().unwrap() {
            let table = claimable_table(claim["table"].as_str().unwrap());
            let index = claim["index"].as_u64().unwrap();
            let rows = report[table].as_array().unwrap().len();
            assert!(
                (index as usize) < rows,
                "claim {table} {index} of {rows} rows"
            );
            assert!(
                claims.insert((table, index)),
                "{table} row {index} is claimed twice"
            );
        }
    }
    for table in CLAIMABLE_TABLES {
        for index in 0..report[table].as_array().unwrap().len() as u64 {
            assert!(
                claims.contains(&(table, index)),
                "{table} row {index} reached no problem card"
            );
        }
    }
}

/// The frozen name of the claimable table one claim indexes.
fn claimable_table(name: &str) -> &'static str {
    CLAIMABLE_TABLES
        .into_iter()
        .find(|table| *table == name)
        .unwrap_or_else(|| panic!("unclaimable table {name}"))
}

/// A card's anchor names its kind and carries only the indexes that kind
/// implies, each resolving into the table it names.
fn assert_problem_anchor(report: &Value, anchor: &Value) {
    let files = report["files"].as_array().unwrap().len();
    let packages = report["packages"].as_array().unwrap().len();
    match anchor["kind"].as_str().unwrap() {
        "file" => assert!((anchor["file"].as_u64().unwrap() as usize) < files),
        "files" => {
            let members = anchor["files"].as_array().unwrap();
            assert!(members.len() >= 2, "a file set anchors more than one file");
            for file in members {
                assert!((file.as_u64().unwrap() as usize) < files);
            }
        }
        "package" => assert!((anchor["package"].as_u64().unwrap() as usize) < packages),
        "package_pair" => {
            let pair = anchor["packages"].as_array().unwrap();
            assert_eq!(pair.len(), 2, "a package pair names two packages");
            assert_ne!(pair[0], pair[1]);
            for package in pair {
                assert!((package.as_u64().unwrap() as usize) < packages);
            }
        }
        kind => panic!("unknown anchor kind {kind}"),
    }
}

/// A card's facts are either a link that resolves into the table its kind
/// names or a measured integer, and a card states at least one of them.
fn assert_problem_evidence(report: &Value, evidence: &Value) {
    let evidence = evidence.as_array().unwrap();
    assert!(!evidence.is_empty(), "a card states at least one fact");
    for fact in evidence {
        let kind = fact["kind"].as_str().unwrap();
        if CLAIMABLE_TABLES.contains(&kind) {
            let index = fact["index"].as_u64().unwrap() as usize;
            let rows = report[kind].as_array().unwrap().len();
            assert!(index < rows, "evidence {kind} {index} of {rows} rows");
        } else {
            assert!(
                [
                    "fan_in",
                    "fan_out",
                    "hot",
                    "rated_units",
                    "members",
                    "reach_in",
                    "followers"
                ]
                .contains(&kind),
                "unknown evidence kind {kind}"
            );
            fact["value"].as_u64().expect("a fact carries an integer");
        }
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

/// A dependency-target string must be legible on a single line: the
/// language layer collapses any raw multi-line syntax before a target is
/// retained, so no newline, carriage return, or tab reaches the machine
/// report.
fn assert_single_line_target(target: &Value) {
    let target = target.as_str().unwrap();
    for forbidden in ['\n', '\r', '\t'] {
        assert!(
            !target.contains(forbidden),
            "dependency target contains {forbidden:?}: {target:?}"
        );
    }
}

/// A relation's role is its file's role unless a test scope demoted it.
///
/// A reference declared under a Rust `#[cfg(test)]` scope is test evidence even
/// though the file that declares it ships, so a primary file may publish test
/// relations. No other demotion exists.
fn assert_evidence_role(relation: &Value, file: &Value) {
    if relation == file {
        return;
    }
    assert_eq!(
        (file.as_str().unwrap(), relation.as_str().unwrap()),
        ("primary", "test"),
        "relation role must be the file's role or a test-scoped demotion"
    );
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
        // The banned row was the churn line `<name> · <n> touches · +a -b`, so
        // the ban names the separator that made it a row. The bare word is now
        // the verb of a shipped verdict sentence — `A typical change here
        // touches 4 files.` — and a shipped sentence cannot be banned.
        " touches ·",
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
        "QUALITY",
        "\u{ec3f}",
        // Generated internal identities and ellipsis truncation are gone.
        "<closure ",
        "<lambda ",
        "\u{2026}",
        "No changes",
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

/// Styling reaches decoration only: a glyph or the tier bar, always followed
/// by its word, and always reset immediately.
fn assert_decoration_only_ansi(value: &[u8]) {
    let text = std::str::from_utf8(value).unwrap();
    let mut remainder = text;
    while let Some(start) = remainder.find("\x1b[") {
        remainder = &remainder[start..];
        let style_end = remainder.find('m').expect("ANSI style terminator") + 1;
        let style = &remainder[..style_end];
        remainder = &remainder[style_end..];
        let decoration = remainder.chars().next().expect("styled decoration");
        let expected_style = match decoration {
            '\u{f024}' | '\u{f062}' => "\u{1b}[31m",
            '\u{f0eb}' | '\u{f071}' => "\u{1b}[38;5;208m",
            '\u{f46b}' => "\u{1b}[36m",
            '\u{f063}' => "\u{1b}[32m",
            // The verdict bar carries its tier color.
            '\u{258c}' => style,
            other => panic!("styled non-decoration text {other:?}"),
        };
        assert_eq!(style, expected_style, "wrong style for {decoration:?}");
        remainder = &remainder[decoration.len_utf8()..];
        assert!(
            remainder.starts_with("\x1b["),
            "decoration style was not reset immediately"
        );
        let reset_end = remainder.find('m').expect("ANSI reset terminator") + 1;
        remainder = &remainder[reset_end..];
        // The word the decoration decorates follows it directly.
        assert!(
            remainder.starts_with(' ')
                && remainder[1..]
                    .chars()
                    .next()
                    .is_some_and(|next| next.is_ascii_uppercase() || next.is_ascii_lowercase()),
            "decoration is not adjacent to a word: {remainder:.20}"
        );
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
