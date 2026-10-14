mod language_support;
use language_support::{pairs, write};
use smackdebt_analysis::ArchitectureFindingKind;
use smackdebt_project::CodebaseRequest;

#[test]
fn every_python_import_in_a_statement_can_close_a_cycle() {
    for import in ["import os, b as other", "from . import (os, b as other)"] {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "pyproject.toml", "[project]\nname='example'\n");
        write(
            root.path(),
            "a.py",
            &format!("{import}\ndef run(): return other.run()\n"),
        );
        write(root.path(), "b.py", "import a\ndef run(): return a.run()\n");
        let output = CodebaseRequest::new(root.path()).analyze().unwrap();
        assert_eq!(
            pairs(output.report()),
            [("a.py", "b.py"), ("b.py", "a.py")],
            "{import}"
        );
        assert!(
            output
                .report()
                .architecture_findings()
                .iter()
                .any(|f| f.kind() == ArchitectureFindingKind::FileCycle)
        );
    }
}

#[test]
fn a_computed_javascript_target_cannot_invent_a_cycle() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "package.json", "{}");
    write(
        root.path(),
        "a.js",
        "const suffix = '-other'; const b = require('./b' + suffix); export const a = b;",
    );
    write(
        root.path(),
        "b.js",
        "import {a} from './a'; export const b = a;",
    );
    write(root.path(), "b-other.js", "export const b = 1;");
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert_eq!(pairs(output.report()), [("b.js", "a.js")]);
    assert!(output.report().architecture_findings().is_empty());
    assert_eq!(output.report().resolution_diagnostics().len(), 1);
    assert!(!output.report().graph_evidence().is_complete());
}

#[test]
fn a_vue_external_script_can_close_a_cycle_at_its_document_line() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "package.json", "{}");
    write(
        root.path(),
        "A.vue",
        "<template><div /></template>\n<script src=\"./a.js\"></script>\n",
    );
    write(
        root.path(),
        "a.js",
        "import A from './A.vue'; export default A;",
    );
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert_eq!(
        pairs(output.report()),
        [("A.vue", "a.js"), ("a.js", "A.vue")]
    );
    assert_eq!(
        output.report().dependency_edges()[0].locations()[0].start_line(),
        2
    );
    assert_eq!(output.report().architecture_findings().len(), 1);
}

#[test]
fn python_package_imports_link_initializers_submodules_and_exported_values() {
    let root = tempfile::tempdir().unwrap();
    for (path, source) in [
        ("pyproject.toml", "[project]\nname='example'"),
        ("app.py", "from pkg import (child as c, value)\n"),
        ("pkg/__init__.py", "value = 1\n"),
        ("pkg/child.py", "import app\n"),
    ] {
        write(root.path(), path, source);
    }
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert_eq!(
        pairs(output.report()),
        [
            ("app.py", "pkg/__init__.py"),
            ("app.py", "pkg/child.py"),
            ("pkg/child.py", "app.py")
        ]
    );
    assert_eq!(output.stats().source_reads(), 3);
    assert!(output.report().resolution_diagnostics().is_empty());
}

#[test]
fn a_missing_relative_python_module_cannot_be_called_external() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "main.py", "from .missing import value\n");
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert!(pairs(output.report()).is_empty());
    assert_eq!(output.report().resolution_diagnostics().len(), 1);
    assert!(!output.report().graph_evidence().is_complete());
}

#[test]
fn a_python_module_file_cannot_supply_a_sibling_directory_as_a_submodule() {
    let root = tempfile::tempdir().unwrap();
    for (path, source) in [
        ("main.py", "from pkg import child\n"),
        ("pkg.py", "child = 1\n"),
        ("pkg/child.py", "import main\n"),
    ] {
        write(root.path(), path, source);
    }
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert!(pairs(output.report()).contains(&("main.py", "pkg.py")));
    assert!(!pairs(output.report()).contains(&("main.py", "pkg/child.py")));
    assert!(output.report().architecture_findings().is_empty());
}

#[test]
fn a_dotted_python_import_also_depends_on_package_initializers() {
    let root = tempfile::tempdir().unwrap();
    for (path, source) in [
        ("app.py", "import pkg.child\n"),
        ("pkg/__init__.py", "import app\n"),
        ("pkg/child.py", "value = 1\n"),
    ] {
        write(root.path(), path, source);
    }
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert_eq!(
        pairs(output.report()),
        [
            ("app.py", "pkg/__init__.py"),
            ("app.py", "pkg/child.py"),
            ("pkg/__init__.py", "app.py")
        ]
    );
    assert_eq!(output.report().architecture_findings().len(), 1);
}

#[test]
fn an_encoded_vue_script_target_is_not_treated_as_a_literal_filename() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "A.vue",
        "<script src=\"./a&amp;.js\"></script><template><div /></template>",
    );
    write(root.path(), "a&amp;.js", "export default 1;");
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert!(pairs(output.report()).is_empty());
    assert_eq!(output.report().resolution_diagnostics().len(), 1);
    assert!(!output.report().graph_evidence().is_complete());
}

#[test]
fn javascript_paths_keep_quotes_that_are_part_of_the_filename() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "a.js",
        "import value from \"./'worker'\"; export default value;",
    );
    write(root.path(), "'worker'.js", "export default 1;");
    write(
        root.path(),
        "worker.js",
        "import a from './a'; export default a;",
    );
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert_eq!(
        pairs(output.report()),
        [("a.js", "'worker'.js"), ("worker.js", "a.js")]
    );
    assert!(output.report().architecture_findings().is_empty());
}

#[test]
fn python_import_line_continuations_work_with_windows_newlines() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "a.py", "import pkg.\\\r\n    child\r\n");
    write(root.path(), "pkg/child.py", "import a\n");
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert_eq!(
        pairs(output.report()),
        [("a.py", "pkg/child.py"), ("pkg/child.py", "a.py")]
    );
    assert_eq!(output.report().architecture_findings().len(), 1);
}

#[test]
fn a_python_namespace_member_does_not_turn_an_absolute_import_into_a_sibling() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "pyproject.toml", "[project]\nname='example'\n");
    write(root.path(), "pkg/a.py", "import random\n");
    write(root.path(), "pkg/random.py", "value = 1\n");
    let output = CodebaseRequest::new(root.path()).analyze().unwrap();
    assert!(pairs(output.report()).is_empty());
}
