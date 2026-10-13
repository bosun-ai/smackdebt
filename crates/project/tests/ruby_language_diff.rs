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
fn gemspec_edits_change_require_resolution_on_each_diff_side() {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "-q"]);
    git(root.path(), &["config", "user.name", "Language Test"]);
    git(
        root.path(),
        &["config", "user.email", "language@example.invalid"],
    );
    for (path, source) in [
        (
            "sample.gemspec",
            "Gem::Specification.new do |s|\n s.require_paths = ['old']\nend\n",
        ),
        ("main.rb", "require 'worker'\n"),
        ("old/worker.rb", "def old_worker; end\n"),
        ("new/worker.rb", "def new_worker; end\n"),
    ] {
        write(root.path(), path, source);
    }
    git(root.path(), &["add", "."]);
    git(root.path(), &["commit", "-qm", "initial"]);
    write(
        root.path(),
        "sample.gemspec",
        "Gem::Specification.new do |s|\n s.require_paths = ['new']\nend\n",
    );
    let output = DiffRequest::new(root.path())
        .with_reference("HEAD")
        .analyze()
        .unwrap();
    assert_eq!(pairs(output.report()), [("main.rb", "new/worker.rb")]);
    assert!(output.report().resolution_diagnostics().is_empty());
    assert_eq!(output.stats().git_processes(), 6);
}
