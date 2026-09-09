mod language_support;
use language_support::{pairs, write};
use smackdebt_analysis::SourceRole;
use smackdebt_project::CodebaseRequest;

#[test]
fn csharp_resolves_types_and_partial_members_without_linking_every_namespace_file() {
    let root = tempfile::tempdir().unwrap();
    for (path, source) in [
        (
            "app/App.csproj",
            r#"<Project Sdk="Microsoft.NET.Sdk"><ItemGroup><ProjectReference Include="../core/Core.csproj" /></ItemGroup></Project>"#,
        ),
        ("core/Core.csproj", r#"<Project Sdk="Microsoft.NET.Sdk" />"#),
        ("app/Usings.cs", "global using Core;"),
        (
            "app/Runner.cs",
            "namespace App; class Runner { Worker Run() => new Worker(); }",
        ),
        (
            "core/Worker.cs",
            "namespace Core; public partial class Worker { public int One() => 1; }",
        ),
        (
            "core/Worker.More.cs",
            "namespace Core; public partial class Worker { public int Two() => 2; }",
        ),
        (
            "core/Unrelated.cs",
            "namespace Core; public class Unrelated { }",
        ),
    ] {
        write(root.path(), path, source);
    }
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert_eq!(
        pairs(output.report()),
        [
            ("app/Runner.cs", "core/Worker.More.cs"),
            ("app/Runner.cs", "core/Worker.cs")
        ]
    );
    assert_eq!(
        output
            .report()
            .dependency_coverage()
            .resolved_internal_uses(),
        2
    );
    assert!(output.report().resolution_diagnostics().is_empty());
}

#[test]
fn competing_csharp_types_are_ambiguous_and_test_projects_keep_their_role() {
    let root = tempfile::tempdir().unwrap();
    for (path, source) in [
        (
            "App.csproj",
            "<Project><PropertyGroup><IsTestProject>true</IsTestProject></PropertyGroup></Project>",
        ),
        ("A.cs", "namespace A; class Worker {}"),
        ("B.cs", "namespace B; class Worker {}"),
        (
            "Run.cs",
            "using A; using B; class Runner { Worker Run() => new Worker(); }",
        ),
    ] {
        write(root.path(), path, source);
    }
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert!(pairs(output.report()).is_empty());
    assert_eq!(output.report().resolution_diagnostics().len(), 1);
    assert!(
        output
            .report()
            .files()
            .iter()
            .all(|file| file.role() == SourceRole::Test)
    );
}

#[test]
fn invalid_csharp_project_configuration_withholds_guessed_edges() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "App.csproj", "<Project>");
    write(
        root.path(),
        "Run.cs",
        "class Runner { Core.Worker Run() => new Core.Worker(); }",
    );
    write(root.path(), "Worker.cs", "namespace Core; class Worker {}");
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert!(pairs(output.report()).is_empty());
    assert!(!output.report().graph_evidence().is_complete());
    assert!(!output.report().resolution_diagnostics().is_empty());
}
