//! Public cycle reports must agree across languages, formats, and worker counts.
#[path = "support/hermetic.rs"]
mod hermetic;

use assert_cmd::prelude::*;
use serde_json::Value;
use std::{fs, path::Path, process::Command};

fn write(root: &Path, path: &str, source: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, source).unwrap();
}

fn run(root: &Path, json: bool, serial: bool) -> Vec<u8> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_smackdebt"));
    hermetic::hermetic_env(&mut command);
    command
        .current_dir(root)
        .env("COLUMNS", "100")
        .env("NO_COLOR", "1");
    if json {
        command.arg("--json");
    } else {
        command.args(["--color", "never"]);
    }
    if serial {
        command.args(["--jobs", "1"]);
    }
    command.assert().success().get_output().stdout.clone()
}

fn report(root: &Path, cycle: bool) -> Value {
    let terminal = run(root, false, true);
    assert_eq!(
        terminal,
        run(root, false, false),
        "terminal worker counts differ"
    );
    let terminal = String::from_utf8(terminal).unwrap();
    assert_eq!(
        terminal.contains("circular dependency"),
        cycle,
        "{terminal}"
    );
    let json = run(root, true, true);
    assert_eq!(json, run(root, true, false), "JSON worker counts differ");
    serde_json::from_slice(&json).unwrap()
}

fn assert_closed_witness(report: &Value, kind: &str, expected_paths: &[&str]) {
    let findings = report["architecture_findings"].as_array().unwrap();
    let finding = findings
        .iter()
        .find(|finding| finding["kind"] == kind)
        .unwrap_or_else(|| panic!("missing {kind}: {findings:?}"));
    let files = report["files"].as_array().unwrap();
    let mut actual_paths: Vec<_> = finding["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| {
            let file = files.iter().find(|file| file["id"] == *id).unwrap();
            report["paths"][file["path"].as_u64().unwrap() as usize]
                .as_str()
                .unwrap()
        })
        .collect();
    actual_paths.sort_unstable();
    let mut expected_paths = expected_paths.to_vec();
    expected_paths.sort_unstable();
    assert_eq!(actual_paths, expected_paths);
    let edges = report["dependency_edges"].as_array().unwrap();
    let witness: Vec<_> = finding["witness_edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| edges.iter().find(|edge| edge["id"] == *id).unwrap())
        .collect();
    assert_eq!(witness.len(), expected_paths.len());
    for (edge, next) in witness.iter().zip(witness.iter().cycle().skip(1)) {
        assert_eq!(
            edge["target"], next["source"],
            "witness must close by file identity"
        );
        for endpoint in ["source", "target"] {
            assert!(
                finding["files"]
                    .as_array()
                    .unwrap()
                    .contains(&edge[endpoint])
            );
        }
        assert_eq!(edge["relation"], "uses");
        let locations = edge["locations"].as_array().unwrap();
        assert!(!locations.is_empty());
        for span in locations {
            assert!(span[0].as_u64().unwrap() > 0);
            assert!(span[1].as_u64().unwrap() >= span[0].as_u64().unwrap());
        }
    }
}

fn cycle_case(files: &[(&str, &str)], members: &[&str], cut: (&str, &str), kind: &str) {
    let root = tempfile::tempdir().unwrap();
    for (path, source) in files {
        write(root.path(), path, source);
    }
    assert_closed_witness(&report(root.path(), true), kind, members);
    write(root.path(), cut.0, cut.1);
    let acyclic = report(root.path(), false);
    assert!(
        acyclic["architecture_findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| {
                !matches!(
                    finding["kind"].as_str(),
                    Some("file_cycle" | "package_cycle")
                )
            })
    );
}

macro_rules! language_cycle {
    ($test:ident, $a:literal, $asrc:literal, $b:literal, $bsrc:literal, $cut:literal $(, $extra:literal => $source:literal)*) => {
        #[test]
        fn $test() {
            cycle_case(&[($a, $asrc), ($b, $bsrc), $(($extra, $source)),*], &[$a, $b], ($b, $cut), "file_cycle");
        }
    };
}

language_cycle!(
    c_cycle,
    "a.c",
    "#include \"b.c\"\nint a() { return 1; }",
    "b.c",
    "#include \"a.c\"\nint b() { return 2; }",
    "int b() { return 2; }"
);
language_cycle!(
    cpp_cycle,
    "a.cpp",
    "#include \"b.cpp\"\nint a() { return 1; }",
    "b.cpp",
    "#include \"a.cpp\"\nint b() { return 2; }",
    "int b() { return 2; }"
);
language_cycle!(
    csharp_cycle,
    "A.cs",
    "namespace App; class A { public static void Run() { B.Run(); } }",
    "B.cs",
    "namespace App; class B { public static void Run() { A.Run(); } }",
    "namespace App; class B { public static void Run() {} }"
);
language_cycle!(go_cycle, "a/a.go", "package a\nimport \"sample/b\"\nfunc A() { b.B() }", "b/b.go", "package b\nimport \"sample/a\"\nfunc B() { a.A() }", "package b\nfunc B() {}", "go.mod" => "module sample\ngo 1.22\n");
language_cycle!(
    java_same_package_cycle,
    "odd/A.java",
    "package app; class A { B value; }",
    "elsewhere/B.java",
    "package app; class B { A value; }",
    "package app; class B {}"
);
language_cycle!(
    javascript_cycle,
    "a.js",
    "import './b.js'; export const a = 1;",
    "b.js",
    "import './a.js'; export const b = 2;",
    "export const b = 2;"
);
language_cycle!(
    jsx_cycle,
    "a.jsx",
    "import './b.jsx'; export const A = () => <div/>;",
    "b.jsx",
    "import './a.jsx'; export const B = () => <div/>;",
    "export const B = () => <div/>;"
);
language_cycle!(
    php_cycle,
    "A.php",
    "<?php namespace App; class A { public B $value; }",
    "B.php",
    "<?php namespace App; class B { public A $value; }",
    "<?php namespace App; class B {}"
);
language_cycle!(
    python_cycle,
    "a.py",
    "import b\ndef a(): return 1\n",
    "b.py",
    "import a\ndef b(): return 2\n",
    "def b(): return 2\n"
);
language_cycle!(ruby_gem_load_root_cycle, "code/a.rb", "require 'b'\ndef a; 1; end\n", "code/b.rb", "require 'a'\ndef b; 2; end\n", "def b; 2; end\n", "sample.gemspec" => "Gem::Specification.new do |s|\n s.require_paths = ['code']\nend\n");
language_cycle!(rust_cycle, "src/a.rs", "use crate::b::b; pub fn a() { b(); }", "src/b.rs", "use crate::a::a; pub fn b() { a(); }", "pub fn b() {}", "Cargo.toml" => "[package]\nname = 'cycle'\nversion = '0.1.0'\nedition = '2024'\n", "src/lib.rs" => "mod a; mod b;");
language_cycle!(
    typescript_cycle,
    "a.ts",
    "import './b.ts'; export const a: number = 1;",
    "b.ts",
    "import './a.ts'; export const b: number = 2;",
    "export const b: number = 2;"
);
language_cycle!(
    tsx_cycle,
    "a.tsx",
    "import './b.tsx'; export const A = () => <div/>;",
    "b.tsx",
    "import './a.tsx'; export const B = () => <div/>;",
    "export const B = () => <div/>;"
);
language_cycle!(
    vue_cycle,
    "A.vue",
    "<script setup>import B from './B.vue'</script><template><B/></template>",
    "B.vue",
    "<script setup>import A from './A.vue'</script><template><A/></template>",
    "<script setup>const b = 1</script><template><div>{{ b }}</div></template>"
);

#[test]
fn javascript_typescript_and_vue_share_one_cycle() {
    cycle_case(
        &[
            ("a.js", "import './b.ts'; export const a = 1;"),
            ("b.ts", "import C from './C.vue'; export const b = C;"),
            (
                "C.vue",
                "<script setup>import { a } from './a.js'</script><template><div>{{ a }}</div></template>",
            ),
        ],
        &["a.js", "b.ts", "C.vue"],
        ("a.js", "export const a = 1;"),
        "file_cycle",
    );
}

#[test]
fn crossing_package_roots_reports_a_package_cycle() {
    cycle_case(
        &[
            ("first/package.json", "{\"name\":\"first\"}"),
            ("second/package.json", "{\"name\":\"second\"}"),
            ("first/a.js", "import '../second/b.js'; export const a = 1;"),
            ("second/b.js", "import '../first/a.js'; export const b = 2;"),
        ],
        &["first/a.js", "second/b.js"],
        ("second/b.js", "export const b = 2;"),
        "package_cycle",
    );
}
