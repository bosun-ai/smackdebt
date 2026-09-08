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
fn go_manifest_edits_resolve_each_diff_side_independently() {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "-q"]);
    git(root.path(), &["config", "user.name", "Language Test"]);
    git(
        root.path(),
        &["config", "user.email", "language@example.invalid"],
    );
    for (path, source) in [
        (
            "go.mod",
            "module example.test/app\nreplace example.test/core => ./old\n",
        ),
        (
            "main.go",
            "package main\nimport _ \"example.test/core\"\nfunc main() {}\n",
        ),
        ("old/go.mod", "module example.test/core\n"),
        ("new/go.mod", "module example.test/core\n"),
        ("old/core.go", "package core\nfunc Core() {}\n"),
        ("new/core.go", "package core\nfunc Core() {}\n"),
    ] {
        write(root.path(), path, source);
    }
    git(root.path(), &["add", "."]);
    git(root.path(), &["commit", "-qm", "initial"]);
    write(
        root.path(),
        "go.mod",
        "module example.test/app\nreplace example.test/core => ./new\n",
    );
    let output = DiffRequest::new(root.path())
        .with_reference("HEAD")
        .analyze()
        .unwrap();
    assert_eq!(pairs(output.report()), [("main.go", "new/core.go")]);
    assert!(!output.report().architecture_comparisons().is_empty());
    assert_eq!(output.stats().git_processes(), 6);
}
