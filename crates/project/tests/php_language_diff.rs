mod language_support;
use language_support::{pairs, write};
use smackdebt_project::DiffRequest;
use std::path::Path;
use std::process::Command;

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn composer_edits_change_local_visibility_on_each_diff_side() {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "-q"]);
    git(root.path(), &["config", "user.name", "Language Test"]);
    git(
        root.path(),
        &["config", "user.email", "language@example.invalid"],
    );
    for (path, source) in [
        (
            "composer.json",
            r#"{"autoload":{"psr-4":{"App\\":"old/"}}}"#,
        ),
        (
            "Entry.php",
            "<?php function run() { return new \\App\\Worker(); }",
        ),
        ("old/Worker.php", "<?php namespace App; class Worker {}"),
        ("new/Worker.php", "<?php namespace App; class Worker {}"),
    ] {
        write(root.path(), path, source);
    }
    git(root.path(), &["add", "."]);
    git(root.path(), &["commit", "-qm", "initial"]);
    write(
        root.path(),
        "composer.json",
        r#"{"autoload":{"psr-4":{"App\\":"new/"}}}"#,
    );
    let output = DiffRequest::new(root.path())
        .with_reference("HEAD")
        .analyze()
        .unwrap();
    assert_eq!(pairs(output.report()), [("Entry.php", "new/Worker.php")]);
    assert!(output.report().resolution_diagnostics().is_empty());
    assert_eq!(output.stats().git_processes(), 6);
}
