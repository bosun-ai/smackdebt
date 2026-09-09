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
fn csharp_project_reference_edits_resolve_each_diff_side() {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "-q"]);
    git(root.path(), &["config", "user.name", "Language Test"]);
    git(
        root.path(),
        &["config", "user.email", "language@example.invalid"],
    );
    for (path, source) in [
        (
            "app/App.csproj",
            r#"<Project><ItemGroup><ProjectReference Include="../old/Old.csproj" /></ItemGroup></Project>"#,
        ),
        (
            "app/Run.cs",
            "class Runner { Core.Worker Run() => new Core.Worker(); }",
        ),
        ("old/Old.csproj", "<Project />"),
        ("old/Worker.cs", "namespace Core; class Worker {}"),
        ("new/New.csproj", "<Project />"),
        ("new/Worker.cs", "namespace Core; class Worker {}"),
    ] {
        write(root.path(), path, source);
    }
    git(root.path(), &["add", "."]);
    git(root.path(), &["commit", "-qm", "initial"]);
    write(
        root.path(),
        "app/App.csproj",
        r#"<Project><ItemGroup><ProjectReference Include="../new/New.csproj" /></ItemGroup></Project>"#,
    );
    let output = DiffRequest::new(root.path())
        .with_reference("HEAD")
        .analyze()
        .unwrap();
    assert_eq!(pairs(output.report()), [("app/Run.cs", "new/Worker.cs")]);
    assert!(output.report().resolution_diagnostics().is_empty());
    assert_eq!(output.stats().git_processes(), 6);
}
