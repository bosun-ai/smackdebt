mod language_support;
use language_support::{pairs, write};
use smackdebt_project::CodebaseRequest;

#[test]
fn compiler_search_order_resolves_local_angle_includes_and_header_cycles() {
    let root = tempfile::tempdir().unwrap();
    for (path, source) in [
        (
            "compile_commands.json",
            r#"[{"directory":".","file":"src/main.cpp","arguments":["c++","-Iinclude","-Ifallback"]}]"#,
        ),
        (
            "src/main.cpp",
            "#include <a.h>\n#include <vector>\nint main() { return 0; }\n",
        ),
        ("include/a.h", "#include \"b.h\"\n"),
        ("include/b.h", "#include \"a.h\"\n"),
        ("fallback/a.h", "int unused();\n"),
    ] {
        write(root.path(), path, source);
    }
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert_eq!(
        pairs(output.report()),
        [
            ("include/a.h", "include/b.h"),
            ("include/b.h", "include/a.h"),
            ("src/main.cpp", "include/a.h")
        ]
    );
    assert!(output.report().resolution_diagnostics().is_empty());
}

#[test]
fn unsupported_macros_and_missing_angle_paths_do_not_create_guessed_edges() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "main.c",
        "#include HEADER\n#include <nearby.h>\n",
    );
    write(root.path(), "nearby.h", "int nearby();\n");
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert!(pairs(output.report()).is_empty());
    assert_eq!(output.report().resolution_diagnostics().len(), 2);
}

#[test]
fn forced_includes_in_compile_arguments_keep_import_free_files_incomplete() {
    for argument in [
        "-includeforced.h",
        "-imacrosforced.h",
        "-include=forced.h",
        "--include=forced.h",
        "--imacros=forced.h",
        "-Wp,-include,forced.h",
        "/FIforced.h",
        "-FIforced.h",
    ] {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "compile_commands.json",
            &serde_json::json!([{
                "directory": ".", "file": "main.c", "arguments": ["cc", argument, "-c", "main.c"]
            }])
            .to_string(),
        );
        write(root.path(), "main.c", "int main(void) { return VALUE; }\n");
        write(root.path(), "forced.h", "#define VALUE 1\n");
        let output = CodebaseRequest::new(root.path()).analyze().unwrap();
        assert!(
            !output.report().graph_evidence().is_complete(),
            "{argument}"
        );
        assert_eq!(
            output
                .report()
                .graph_evidence()
                .configuration_failures()
                .len(),
            1,
            "{argument}"
        );
        assert!(
            output.report().graph_evidence().configuration_failures()[0]
                .reason()
                .contains("unsupported include configuration"),
            "{argument}"
        );
        assert!(pairs(output.report()).is_empty(), "{argument}");
    }
}
