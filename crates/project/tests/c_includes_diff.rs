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
fn compilation_database_edits_change_include_resolution_on_each_diff_side() {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "-q"]);
    git(root.path(), &["config", "user.name", "Language Test"]);
    git(
        root.path(),
        &["config", "user.email", "language@example.invalid"],
    );
    for (path, source) in [
        (
            "compile_commands.json",
            r#"[{"directory":".","file":"main.c","arguments":["cc","-Iold"]}]"#,
        ),
        ("main.c", "#include <worker.h>\n"),
        ("old/worker.h", "int old_worker();\n"),
        ("new/worker.h", "int new_worker();\n"),
    ] {
        write(root.path(), path, source);
    }
    git(root.path(), &["add", "."]);
    git(root.path(), &["commit", "-qm", "initial"]);
    write(
        root.path(),
        "compile_commands.json",
        r#"[{"directory":".","file":"main.c","arguments":["cc","-Inew"]}]"#,
    );
    let output = DiffRequest::new(root.path())
        .with_reference("HEAD")
        .analyze()
        .unwrap();
    assert_eq!(pairs(output.report()), [("main.c", "new/worker.h")]);
    assert!(output.report().resolution_diagnostics().is_empty());
    assert_eq!(output.stats().git_processes(), 6);
}
