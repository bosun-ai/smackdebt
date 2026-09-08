mod language_support;
use language_support::{pairs, write};
use smackdebt_project::CodebaseRequest;

#[test]
fn php_aliases_autoload_and_literal_includes_resolve_without_running_php() {
    let root = tempfile::tempdir().unwrap();
    for (path, source) in [
        (
            "composer.json",
            r#"{"name":"example/app","autoload":{"psr-4":{"App\\":"src/"},"files":["bootstrap.php"]}}"#,
        ),
        ("package.json", r#"{"name":"frontend"}"#),
        (
            "src/Worker.php",
            "<?php namespace App; class Worker { public function run() { return 1; } }",
        ),
        (
            "src/Entry.php",
            "<?php namespace App; use App\\Worker as Service; function run() { return new Service(); }",
        ),
        ("bootstrap.php", "<?php require __DIR__ . '/src/Entry.php';"),
    ] {
        write(root.path(), path, source);
    }
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert_eq!(output.report().packages().len(), 1);
    assert_eq!(
        pairs(output.report()),
        [
            ("bootstrap.php", "src/Entry.php"),
            ("src/Entry.php", "src/Worker.php")
        ]
    );
    assert!(output.report().resolution_diagnostics().is_empty());
    assert!(
        output
            .report()
            .orphan_files()
            .iter()
            .all(|orphan| output.report().files()[orphan.file().index()].path() != "bootstrap.php")
    );
}

#[test]
fn php_grouped_aliases_functions_and_constants_resolve_in_their_namespace() {
    let root = tempfile::tempdir().unwrap();
    for (path, source) in [
        (
            "composer.json",
            r#"{"autoload":{"classmap":["src"],"files":["src/helpers.php"]}}"#,
        ),
        ("src/Worker.php", "<?php namespace App; class Worker {}"),
        (
            "src/helpers.php",
            "<?php namespace App; function helper() {} const LIMIT = 1;",
        ),
        (
            "src/Run.php",
            "<?php namespace Other; use app\\{Worker as Service, function helper, const LIMIT}; function run() { helper(); return new Service(); }",
        ),
    ] {
        write(root.path(), path, source);
    }
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert_eq!(
        pairs(output.report()),
        [
            ("src/Run.php", "src/Worker.php"),
            ("src/Run.php", "src/helpers.php")
        ]
    );
    assert!(output.report().resolution_diagnostics().is_empty());
}

#[test]
fn missing_php_names_and_dynamic_includes_stay_visible() {
    let root = tempfile::tempdir().unwrap();
    for (path, source) in [
        (
            "composer.json",
            r#"{"autoload":{"psr-4":{"App\\":"src/"}}}"#,
        ),
        ("src/A.php", "<?php namespace App; class Duplicate {}"),
        ("src/B.php", "<?php namespace App; class Duplicate {}"),
        (
            "src/Run.php",
            "<?php namespace App; require $path; function run() { return new Missing(); }",
        ),
    ] {
        write(root.path(), path, source);
    }
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert_eq!(output.report().resolution_diagnostics().len(), 2);
    assert!(!output.report().graph_evidence().is_complete());
}
