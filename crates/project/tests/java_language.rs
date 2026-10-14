mod language_support;
use language_support::{pairs, write};
use smackdebt_project::CodebaseRequest;

#[test]
fn java_source_packages_resolve_types_independent_of_directories() {
    let root = tempfile::tempdir().unwrap();
    for (path, source) in [
        ("odd/A.java", "package app; class A { B value; }"),
        ("elsewhere/B.java", "package app; class B { A value; }"),
        ("unused/C.java", "package app; class C {}"),
        (
            "client/Run.java",
            "package client; import app.*; class Run { B value; }",
        ),
    ] {
        write(root.path(), path, source);
    }
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert_eq!(
        pairs(output.report()),
        [
            ("client/Run.java", "elsewhere/B.java"),
            ("elsewhere/B.java", "odd/A.java"),
            ("odd/A.java", "elsewhere/B.java")
        ]
    );
}

#[test]
fn java_explicit_qualified_and_static_uses_resolve_declared_types() {
    let root = tempfile::tempdir().unwrap();
    for (path, source) in [
        (
            "types/B.java",
            "package lib; class B { static void run() {} }",
        ),
        (
            "A.java",
            "import lib.B; class A { void run() { B.run(); } }",
        ),
        (
            "C.java",
            "class C { lib.B value; void run() { lib.B.run(); } }",
        ),
        (
            "D.java",
            "import static lib.B.run; class D { void work() { run(); } }",
        ),
    ] {
        write(root.path(), path, source);
    }
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert_eq!(
        pairs(output.report()),
        [
            ("A.java", "types/B.java"),
            ("C.java", "types/B.java"),
            ("D.java", "types/B.java")
        ]
    );
}

#[test]
fn java_duplicate_declarations_and_competing_wildcards_withhold_edges() {
    let root = tempfile::tempdir().unwrap();
    for (path, source) in [
        ("first/B.java", "package lib; class B {}"),
        ("second/B.java", "package lib; class B {}"),
        ("other/B.java", "package other; class B {}"),
        ("A.java", "import lib.B; class A { B value; }"),
        (
            "C.java",
            "import lib.*; import other.*; class C { B value; }",
        ),
    ] {
        write(root.path(), path, source);
    }
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert!(pairs(output.report()).is_empty());
    assert!(!output.report().resolution_diagnostics().is_empty());
}

#[test]
fn java_subpackages_do_not_inherit_parent_or_default_package_types() {
    let root = tempfile::tempdir().unwrap();
    for (path, source) in [
        ("parent/B.java", "package lib; class B {}"),
        ("C.java", "class C {}"),
        ("A.java", "package lib.child; class A { B one; C two; }"),
    ] {
        write(root.path(), path, source);
    }
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert!(pairs(output.report()).is_empty());
}

#[test]
fn java_value_receivers_do_not_guess_type_dependencies() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "Foo.java", "package p; class Foo {}");
    write(
        root.path(),
        "Use.java",
        "package p; class Use { void run(Object Foo) { Foo.toString(); } void more() { Object Foo = null; Foo.toString(); } }",
    );
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert!(pairs(output.report()).is_empty());
}

#[test]
fn java_enclosing_types_own_nested_receivers_and_field_types() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "A.java",
        "package p; class A { static class B { static void f() {} } B value; void use() { B.f(); } class Inner { B nestedValue; void use() { B.f(); } } }",
    );
    write(
        root.path(),
        "B.java",
        "package p; class B { static void f() {} }",
    );
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert!(pairs(output.report()).is_empty());
}

#[test]
fn java_nested_types_precede_explicit_imports_at_reference_sites() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "A.java",
        "package p; import other.B; class A { static class B { static void f() {} } B value; void use() { B.f(); } }",
    );
    write(
        root.path(),
        "B.java",
        "package other; class B { static void f() {} }",
    );
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    // Only the import crosses files; the two references belong to A.B.
    assert_eq!(pairs(output.report()), [("A.java", "B.java")]);
    assert_eq!(output.report().dependency_edges()[0].references(), 1);
}
