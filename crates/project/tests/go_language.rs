mod language_support;
use language_support::{pairs, write};
use smackdebt_analysis::{Language, SourceRole};
use smackdebt_project::CodebaseRequest;

#[test]
fn go_imports_resolve_package_members_once_and_keep_test_files_out() {
    let root = tempfile::tempdir().unwrap();
    for (path, source) in [
        ("go.mod", "module example.test/app\ngo 1.23\n"),
        (
            "main.go",
            "package main\nimport (\n alias \"example.test/app/core\"\n \"fmt\"\n)\nfunc main() { fmt.Println(alias.One()) }\n",
        ),
        ("core/one.go", "package core\nfunc One() int { return 1 }\n"),
        ("core/two.go", "package core\nfunc Two() int { return 2 }\n"),
        ("core/one_test.go", "package core\nfunc TestOne() {}\n"),
    ] {
        write(root.path(), path, source);
    }
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    let report = output.report();
    assert_eq!(
        pairs(report),
        [("main.go", "core/one.go"), ("main.go", "core/two.go")]
    );
    assert_eq!(report.dependency_coverage().resolved_internal_uses(), 1);
    assert_eq!(report.dependency_coverage().external_uses(), 1);
    assert_eq!(output.stats().source_reads(), 4);
    assert_eq!(
        report
            .files()
            .iter()
            .find(|file| file.path().ends_with("_test.go"))
            .unwrap()
            .role(),
        SourceRole::Test
    );
    assert!(
        report
            .files()
            .iter()
            .all(|file| file.language() == Some(Language::Go))
    );
}

#[test]
fn go_workspace_and_replacement_paths_are_local_module_facts() {
    let root = tempfile::tempdir().unwrap();
    for (path, source) in [
        ("go.work", "go 1.23\nuse (\n ./app\n ./core\n)\n"),
        (
            "app/go.mod",
            "module example.test/app\nreplace example.test/old => ../replacement\n",
        ),
        ("core/go.mod", "module example.test/core\n"),
        ("replacement/go.mod", "module example.test/replacement\n"),
        (
            "app/main.go",
            "package app\nimport (\n _ \"example.test/core\"\n _ \"example.test/old\"\n)\nfunc Run() {}\n",
        ),
        ("core/core.go", "package core\nfunc Core() {}\n"),
        (
            "replacement/core.go",
            "package replacement\nfunc Core() {}\n",
        ),
    ] {
        write(root.path(), path, source);
    }
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert_eq!(
        pairs(output.report()),
        [
            ("app/main.go", "core/core.go"),
            ("app/main.go", "replacement/core.go")
        ]
    );
    assert!(output.report().resolution_diagnostics().is_empty());
}

#[test]
fn missing_go_module_keeps_graph_evidence_incomplete() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "main.go",
        "package main\nimport _ \"example.test/core\"\nfunc main() {}\n",
    );
    write(
        root.path(),
        "core/core.go",
        "package core\nfunc Core() {}\n",
    );
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert!(pairs(output.report()).is_empty());
    assert!(!output.report().graph_evidence().is_complete());
}
