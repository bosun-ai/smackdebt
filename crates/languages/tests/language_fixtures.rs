use std::path::Path;

use smackdebt_analysis::{
    DependencyIntent, DependencyKind, DependencyScope, DependencySyntax, DependencySyntaxState,
    Language, ParseStatus, SourceSpan, StaticRelationKind, UnitKind,
};
use smackdebt_languages::Analyzer;

struct ExpectedFile<'a> {
    path: &'a str,
    source: &'a [u8],
    language: Language,
    units: &'a [ExpectedUnit<'a>],
}

struct ExpectedUnit<'a> {
    name: &'a str,
    container: Option<&'a str>,
    kind: UnitKind,
    parent: Option<usize>,
    start: u32,
    end: u32,
    cognitive: u32,
    cyclomatic: u32,
    logical: u32,
    nesting: u32,
    parameters: u32,
}

#[test]
fn every_supported_language_has_a_complete_ordered_truth_fixture() {
    const FIXTURES: &[ExpectedFile<'_>] = &[
        file(
            "c.c",
            include_bytes!("fixtures/c.c"),
            Language::C,
            &[unit(
                "controls",
                None,
                UnitKind::Function,
                None,
                1,
                6,
                9,
                8,
                6,
                2,
                3,
            )],
        ),
        file(
            "cpp.cpp",
            include_bytes!("fixtures/cpp.cpp"),
            Language::Cpp,
            &[
                unit(
                    "run",
                    Some("Worker"),
                    UnitKind::Method,
                    None,
                    4,
                    8,
                    1,
                    2,
                    3,
                    1,
                    1,
                ),
                unit(
                    "<lambda 5>",
                    Some("Worker"),
                    UnitKind::Lambda,
                    Some(0),
                    5,
                    5,
                    2,
                    3,
                    1,
                    1,
                    1,
                ),
            ],
        ),
        file(
            "java.java",
            include_bytes!("fixtures/java.java"),
            Language::Java,
            &[
                unit(
                    "Worker",
                    Some("Worker"),
                    UnitKind::Method,
                    None,
                    2,
                    2,
                    1,
                    2,
                    1,
                    1,
                    1,
                ),
                unit(
                    "run",
                    Some("Worker"),
                    UnitKind::Method,
                    None,
                    3,
                    10,
                    4,
                    4,
                    5,
                    1,
                    1,
                ),
                unit(
                    "<lambda 4>",
                    Some("Worker"),
                    UnitKind::Lambda,
                    Some(1),
                    4,
                    4,
                    1,
                    2,
                    0,
                    1,
                    1,
                ),
            ],
        ),
        file(
            "javascript.js",
            include_bytes!("fixtures/javascript.js"),
            Language::JavaScript,
            &[
                unit(
                    "handler",
                    Some("Worker"),
                    UnitKind::Closure,
                    None,
                    2,
                    2,
                    1,
                    2,
                    0,
                    0,
                    1,
                ),
                unit(
                    "run",
                    Some("Worker"),
                    UnitKind::Method,
                    None,
                    3,
                    7,
                    2,
                    3,
                    3,
                    1,
                    1,
                ),
            ],
        ),
        file(
            "jsx.jsx",
            include_bytes!("fixtures/jsx.jsx"),
            Language::Jsx,
            &[
                unit(
                    "render",
                    Some("View"),
                    UnitKind::Method,
                    None,
                    2,
                    5,
                    1,
                    2,
                    2,
                    0,
                    0,
                ),
                unit(
                    "select",
                    Some("View"),
                    UnitKind::Closure,
                    Some(0),
                    3,
                    3,
                    1,
                    2,
                    0,
                    1,
                    1,
                ),
            ],
        ),
        file(
            "python.py",
            include_bytes!("fixtures/python.py"),
            Language::Python,
            &[
                unit(
                    "outer",
                    None,
                    UnitKind::Function,
                    None,
                    1,
                    12,
                    0,
                    1,
                    2,
                    0,
                    1,
                ),
                unit(
                    "inner",
                    Some("outer"),
                    UnitKind::Function,
                    Some(0),
                    2,
                    10,
                    4,
                    5,
                    4,
                    1,
                    1,
                ),
                unit(
                    "<lambda 11>",
                    Some("outer"),
                    UnitKind::Lambda,
                    Some(0),
                    11,
                    11,
                    1,
                    2,
                    0,
                    1,
                    1,
                ),
            ],
        ),
        file(
            "rust.rs",
            include_bytes!("fixtures/rust.rs"),
            Language::Rust,
            &[
                unit(
                    "run",
                    Some("Work"),
                    UnitKind::Method,
                    None,
                    2,
                    2,
                    0,
                    1,
                    0,
                    0,
                    2,
                ),
                unit(
                    "run",
                    Some("Worker"),
                    UnitKind::Method,
                    None,
                    6,
                    14,
                    7,
                    4,
                    3,
                    2,
                    2,
                ),
                unit(
                    "<closure 7>",
                    Some("Worker"),
                    UnitKind::Closure,
                    Some(1),
                    7,
                    7,
                    2,
                    2,
                    0,
                    1,
                    1,
                ),
            ],
        ),
        file(
            "typescript.ts",
            include_bytes!("fixtures/typescript.ts"),
            Language::TypeScript,
            &[
                unit(
                    "handler",
                    Some("Worker"),
                    UnitKind::Closure,
                    None,
                    2,
                    2,
                    1,
                    2,
                    0,
                    1,
                    1,
                ),
                unit(
                    "run",
                    Some("Worker"),
                    UnitKind::Method,
                    None,
                    3,
                    6,
                    1,
                    2,
                    2,
                    1,
                    1,
                ),
            ],
        ),
        file(
            "tsx.tsx",
            include_bytes!("fixtures/tsx.tsx"),
            Language::Tsx,
            &[
                unit(
                    "render",
                    Some("View"),
                    UnitKind::Method,
                    None,
                    2,
                    5,
                    1,
                    2,
                    2,
                    0,
                    0,
                ),
                unit(
                    "select",
                    Some("View"),
                    UnitKind::Closure,
                    Some(0),
                    3,
                    3,
                    1,
                    2,
                    0,
                    1,
                    1,
                ),
            ],
        ),
        file(
            "ruby.rb",
            include_bytes!("fixtures/ruby.rb"),
            Language::Ruby,
            &[
                unit(
                    "run",
                    Some("Worker"),
                    UnitKind::Method,
                    None,
                    3,
                    9,
                    0,
                    1,
                    2,
                    0,
                    1,
                ),
                unit(
                    "<lambda 4>",
                    Some("Worker"),
                    UnitKind::Lambda,
                    Some(0),
                    4,
                    4,
                    3,
                    4,
                    2,
                    1,
                    1,
                ),
                unit(
                    "<closure 5>",
                    Some("Worker"),
                    UnitKind::Closure,
                    Some(0),
                    5,
                    8,
                    1,
                    2,
                    2,
                    1,
                    1,
                ),
                unit(
                    "self.empty",
                    Some("Worker"),
                    UnitKind::Method,
                    None,
                    11,
                    13,
                    0,
                    1,
                    0,
                    0,
                    0,
                ),
            ],
        ),
    ];
    let mut analyzer = Analyzer::default();
    for fixture in FIXTURES {
        let result = analyzer
            .analyze(Path::new(fixture.path), fixture.source.to_vec())
            .unwrap();
        assert_eq!(result.language(), fixture.language, "{}", fixture.path);
        assert_eq!(
            result.parse_status(),
            &ParseStatus::Parsed,
            "{}",
            fixture.path
        );
        assert_eq!(
            result.units().len(),
            fixture.units.len(),
            "{}",
            fixture.path
        );
        for (actual, expected) in result.units().iter().zip(fixture.units) {
            assert_eq!(actual.identity().name(), expected.name, "{}", fixture.path);
            assert_eq!(
                actual.identity().container(),
                expected.container,
                "{}",
                fixture.path
            );
            assert_eq!(actual.identity().kind(), expected.kind, "{}", fixture.path);
            assert_eq!(
                actual.parent().map(|id| id.index()),
                expected.parent,
                "{}",
                fixture.path
            );
            assert_eq!(
                (actual.span().start_line(), actual.span().end_line()),
                (expected.start, expected.end),
                "{}",
                fixture.path
            );
            assert_eq!(
                (
                    actual.measurements().cognitive_complexity(),
                    actual.measurements().cyclomatic_complexity(),
                    actual.measurements().logical_lines()
                ),
                (expected.cognitive, expected.cyclomatic, expected.logical),
                "{}",
                fixture.path
            );
            assert_eq!(
                (
                    actual.measurements().max_nesting(),
                    actual.measurements().parameter_count()
                ),
                (expected.nesting, expected.parameters),
                "{}",
                fixture.path
            );
        }
    }
}

#[test]
fn every_supported_language_translates_dependency_syntax_with_original_spans() {
    let cases = [
        (
            "x.c",
            "#include \"local.h\"",
            DependencyKind::Include,
            "local.h",
            DependencyIntent::Internal,
            StaticRelationKind::Uses,
            vec!["./local.h"],
            1,
        ),
        (
            "x.cpp",
            "#include \"local.hpp\"",
            DependencyKind::Include,
            "local.hpp",
            DependencyIntent::Internal,
            StaticRelationKind::Uses,
            vec!["./local.hpp"],
            1,
        ),
        (
            "x.java",
            "import app.Local;\nclass X { int x() { return 1; } }",
            DependencyKind::Import,
            "app.Local",
            DependencyIntent::Package,
            StaticRelationKind::Uses,
            vec![
                "app/Local.java",
                "src/main/java/app/Local.java",
                "src/test/java/app/Local.java",
            ],
            1,
        ),
        (
            "x.js",
            "import x from './local';\nfunction x() {}",
            DependencyKind::Import,
            "./local",
            DependencyIntent::Internal,
            StaticRelationKind::Uses,
            vec![
                "./local",
                "./local.js",
                "./local/index.js",
                "./local.jsx",
                "./local/index.jsx",
                "./local.ts",
                "./local/index.ts",
                "./local.tsx",
                "./local/index.tsx",
            ],
            1,
        ),
        (
            "x.jsx",
            "import X from './local';\nfunction x() { return <X/>; }",
            DependencyKind::Import,
            "./local",
            DependencyIntent::Internal,
            StaticRelationKind::Uses,
            vec![
                "./local",
                "./local.js",
                "./local/index.js",
                "./local.jsx",
                "./local/index.jsx",
                "./local.ts",
                "./local/index.ts",
                "./local.tsx",
                "./local/index.tsx",
            ],
            1,
        ),
        (
            "x.py",
            "from . import local\ndef x():\n  pass\n",
            DependencyKind::Import,
            ".local",
            DependencyIntent::Internal,
            StaticRelationKind::Uses,
            vec!["./local.py", "./local/__init__.py"],
            1,
        ),
        (
            "x.rs",
            "mod local;\nfn x() {}",
            DependencyKind::Module,
            "local",
            DependencyIntent::Internal,
            StaticRelationKind::ModuleOwnership,
            vec!["./local.rs", "./local/mod.rs"],
            1,
        ),
        (
            "x.ts",
            "import x from './local';\nfunction x() {}",
            DependencyKind::Import,
            "./local",
            DependencyIntent::Internal,
            StaticRelationKind::Uses,
            vec![
                "./local",
                "./local.js",
                "./local/index.js",
                "./local.jsx",
                "./local/index.jsx",
                "./local.ts",
                "./local/index.ts",
                "./local.tsx",
                "./local/index.tsx",
            ],
            1,
        ),
        (
            "x.tsx",
            "import X from './local';\nfunction x() { return <X/>; }",
            DependencyKind::Import,
            "./local",
            DependencyIntent::Internal,
            StaticRelationKind::Uses,
            vec![
                "./local",
                "./local.js",
                "./local/index.js",
                "./local.jsx",
                "./local/index.jsx",
                "./local.ts",
                "./local/index.ts",
                "./local.tsx",
                "./local/index.tsx",
            ],
            1,
        ),
        (
            "x.rb",
            "require_relative 'local'\ndef x; end\n",
            DependencyKind::Require,
            "local",
            DependencyIntent::Internal,
            StaticRelationKind::Uses,
            vec![
                "./local",
                "./local.rb",
                "./local/index.rb",
                "./local/init.rb",
                "./local/index/init.rb",
            ],
            1,
        ),
        (
            "x.vue",
            "<script setup lang=\"ts\">\nimport x from './local'\n</script>\n<template><p /></template>",
            DependencyKind::Import,
            "./local",
            DependencyIntent::Internal,
            StaticRelationKind::Uses,
            vec![
                "./local",
                "./local.js",
                "./local/index.js",
                "./local.jsx",
                "./local/index.jsx",
                "./local.ts",
                "./local/index.ts",
                "./local.tsx",
                "./local/index.tsx",
            ],
            2,
        ),
    ];
    let mut analyzer = Analyzer::default();
    for (path, source, kind, target, intent, relation, candidates, line) in cases {
        let analysis = analyzer
            .analyze(Path::new(path), source.as_bytes().to_vec())
            .unwrap();
        assert_eq!(analysis.dependencies().len(), 1, "{path}");
        let dependency = &analysis.dependencies()[0];
        assert_eq!(dependency.kind(), kind, "{path}");
        assert_eq!(dependency.target(), target, "{path}");
        assert_eq!(dependency.intent(), intent, "{path}");
        assert_eq!(dependency.relation(), relation, "{path}");
        assert_eq!(
            (dependency.span().start_line(), dependency.span().end_line()),
            (line, line),
            "{path}"
        );
        assert_eq!(
            dependency.state(),
            &DependencySyntaxState::Candidates(candidates.into_iter().map(str::to_owned).collect()),
            "{path}"
        );
    }
}

#[test]
fn external_and_dynamic_references_remain_explicit() {
    let mut analyzer = Analyzer::default();
    let analysis = analyzer
        .analyze(
            Path::new("x.js"),
            b"import pkg from 'pkg';\nconst x = require(name);".to_vec(),
        )
        .unwrap();
    assert!(matches!(
        analysis.dependencies()[0].state(),
        DependencySyntaxState::Candidates(_)
    ));
    assert!(matches!(
        analysis.dependencies()[1].state(),
        DependencySyntaxState::Unresolved(_)
    ));
}

#[test]
fn root_package_malformed_and_unsupported_forms_do_not_guess() {
    let mut analyzer = Analyzer::default();
    let rust = analyzer
        .analyze(
            Path::new("x.rs"),
            b"use crate::core::work;\nfn x() {}\n".to_vec(),
        )
        .unwrap();
    assert_eq!(
        rust.dependencies(),
        &[DependencySyntax::new(
            DependencyKind::Import,
            "crate::core::work",
            SourceSpan::new(1, 1),
            DependencySyntaxState::Candidates(vec![
                "core/work.rs".to_owned(),
                "core/work/mod.rs".to_owned(),
                "core.rs".to_owned(),
                "core/mod.rs".to_owned(),
                "crate".to_owned(),
            ]),
        )
        .with_internal_intent()]
    );

    let package = analyzer
        .analyze(
            Path::new("x.js"),
            b"import value from 'package-name';\n".to_vec(),
        )
        .unwrap();
    assert_eq!(
        package.dependencies(),
        &[DependencySyntax::new(
            DependencyKind::Import,
            "package-name",
            SourceSpan::new(1, 1),
            DependencySyntaxState::Candidates(vec![
                "package-name".to_owned(),
                "package-name.js".to_owned(),
                "package-name/index.js".to_owned(),
                "package-name.jsx".to_owned(),
                "package-name/index.jsx".to_owned(),
                "package-name.ts".to_owned(),
                "package-name/index.ts".to_owned(),
                "package-name.tsx".to_owned(),
                "package-name/index.tsx".to_owned(),
            ]),
        )]
    );

    let malformed = analyzer
        .analyze(Path::new("x.c"), b"#include value".to_vec())
        .unwrap();
    assert_eq!(
        malformed.dependencies(),
        &[DependencySyntax::new(
            DependencyKind::Include,
            "#include value",
            SourceSpan::new(1, 1),
            DependencySyntaxState::Unresolved("dependency target is dynamic".to_owned()),
        )]
    );

    let unsupported = analyzer
        .analyze(
            Path::new("x.rs"),
            b"include!(concat!(env!(\"OUT_DIR\"), \"/generated.rs\"));\nfn x() {}\n".to_vec(),
        )
        .unwrap();
    assert_eq!(
        unsupported.dependencies(),
        &[DependencySyntax::new(
            DependencyKind::Include,
            "include!(concat!(env!(\"OUT_DIR\"), \"/generated.rs\"))",
            SourceSpan::new(1, 1),
            DependencySyntaxState::Unresolved("dependency target is dynamic".to_owned()),
        )
        .with_internal_intent()]
    );

    let fixed = analyzer
        .analyze(Path::new("x.rs"), b"include!(\"generated.rs\");".to_vec())
        .unwrap();
    assert_eq!(
        fixed.dependencies(),
        &[DependencySyntax::new(
            DependencyKind::Include,
            "generated.rs",
            SourceSpan::new(1, 1),
            DependencySyntaxState::Candidates(vec!["./generated.rs".to_owned()]),
        )
        .with_internal_intent()]
    );
}

#[test]
fn rust_relations_partition_ownership_inline_modules_uses_and_macros() {
    let analysis = Analyzer::default()
        .analyze(
            Path::new("src/lib.rs"),
            b"mod child;\nmod inline { pub fn local() {} }\nuse crate::shared::work;\nfn run() { crate::qualified::call(); }\ninclude!(\"generated.rs\");\n"
                .to_vec(),
        )
        .unwrap();
    let dependencies = analysis.dependencies();
    assert_eq!(dependencies.len(), 4);
    assert_eq!(dependencies[0].target(), "child");
    assert_eq!(
        dependencies[0].relation(),
        StaticRelationKind::ModuleOwnership
    );
    assert_eq!(dependencies[0].span(), SourceSpan::new(1, 1));
    assert_eq!(dependencies[1].target(), "crate::shared::work");
    assert_eq!(dependencies[1].relation(), StaticRelationKind::Uses);
    assert_eq!(dependencies[1].span(), SourceSpan::new(3, 3));
    assert_eq!(dependencies[2].target(), "crate::qualified::call");
    assert_eq!(dependencies[2].relation(), StaticRelationKind::Uses);
    assert_eq!(dependencies[2].span(), SourceSpan::new(4, 4));
    assert_eq!(dependencies[3].target(), "generated.rs");
    assert_eq!(dependencies[3].relation(), StaticRelationKind::Uses);
    assert_eq!(dependencies[3].span(), SourceSpan::new(5, 5));
    assert!(
        dependencies
            .iter()
            .all(|dependency| dependency.target() != "inline")
    );
}

#[test]
fn rust_qualified_paths_require_explicit_repository_intent() {
    let analysis = Analyzer::default()
        .analyze(
            Path::new("src/lib.rs"),
            b"fn run() { crate::local::work(); self::sibling::work(); super::parent::work(); std::fmt::format(); serde::value(); Type::associated(); }\n"
                .to_vec(),
        )
        .unwrap();

    let targets: Vec<_> = analysis
        .dependencies()
        .iter()
        .map(DependencySyntax::target)
        .collect();
    assert_eq!(
        targets,
        [
            "crate::local::work",
            "self::sibling::work",
            "super::parent::work",
        ]
    );
    assert!(analysis.dependencies().iter().all(|dependency| {
        dependency.intent() == DependencyIntent::Internal
            && dependency.relation() == StaticRelationKind::Uses
            && dependency.span() == SourceSpan::new(1, 1)
    }));
}

#[test]
fn rust_visibility_keywords_never_become_dependency_targets() {
    let analysis = Analyzer::default()
        .analyze(
            Path::new("src/lib.rs"),
            b"pub use a::b;\npub(crate) use c::d;\npub(in crate::e) use f::g;\npub mod child;\n"
                .to_vec(),
        )
        .unwrap();
    let targets: Vec<_> = analysis
        .dependencies()
        .iter()
        .map(DependencySyntax::target)
        .collect();
    assert_eq!(targets, ["a::b", "c::d", "f::g", "child"]);
    assert_eq!(
        analysis.dependencies()[3].relation(),
        StaticRelationKind::ModuleOwnership
    );
}

#[test]
fn rust_crate_rooted_references_offer_the_crate_root_as_a_fallback() {
    let analysis = Analyzer::default()
        .analyze(
            Path::new("src/report.rs"),
            b"use crate::{First, Second};\nuse crate::Item;\n".to_vec(),
        )
        .unwrap();
    assert_eq!(
        analysis.dependencies(),
        &[
            DependencySyntax::new(
                DependencyKind::Import,
                "crate",
                SourceSpan::new(1, 1),
                DependencySyntaxState::Candidates(vec!["crate".to_owned()]),
            )
            .with_internal_intent(),
            DependencySyntax::new(
                DependencyKind::Import,
                "crate::Item",
                SourceSpan::new(2, 2),
                DependencySyntaxState::Candidates(vec![
                    "Item.rs".to_owned(),
                    "Item/mod.rs".to_owned(),
                    "crate".to_owned(),
                ]),
            )
            .with_internal_intent(),
        ]
    );
}

#[test]
fn rust_super_inside_an_inline_module_targets_the_declaring_file() {
    let analysis = Analyzer::default()
        .analyze(
            Path::new("src/report.rs"),
            b"mod tests {\n    use super::*;\n    use super::Item;\n}\nuse super::sibling::work;\n"
                .to_vec(),
        )
        .unwrap();
    let states: Vec<_> = analysis
        .dependencies()
        .iter()
        .filter(|dependency| dependency.relation() == StaticRelationKind::Uses)
        .map(|dependency| (dependency.target(), dependency.state().clone()))
        .collect();
    assert_eq!(
        states,
        [
            (
                "super::*",
                DependencySyntaxState::Candidates(vec![".".to_owned()])
            ),
            (
                "super::Item",
                DependencySyntaxState::Candidates(vec![
                    "../Item.rs".to_owned(),
                    "../Item/mod.rs".to_owned(),
                    ".".to_owned(),
                ])
            ),
            (
                "super::sibling::work",
                DependencySyntaxState::Candidates(vec![
                    "../sibling/work.rs".to_owned(),
                    "../sibling/work/mod.rs".to_owned(),
                    "../sibling.rs".to_owned(),
                    "../sibling/mod.rs".to_owned(),
                ])
            ),
        ]
    );
}

fn rust_scopes(source: &str) -> Vec<(String, DependencyScope)> {
    Analyzer::default()
        .analyze(Path::new("src/lib.rs"), source.as_bytes().to_vec())
        .unwrap()
        .dependencies()
        .iter()
        .map(|dependency| (dependency.target().to_owned(), dependency.scope()))
        .collect()
}

#[test]
fn only_a_cfg_attribute_that_selects_test_scopes_a_rust_reference() {
    for attribute in [
        "#[cfg(test)]",
        "#[cfg(all(test, not(loom)))]",
        // The identifier order inside the predicate does not matter: `test` is
        // still selected when it follows a negated identifier.
        "#[cfg(all(not(loom), test))]",
        "#[cfg(any(test, fuzzing))]",
    ] {
        assert_eq!(
            rust_scopes(&format!("{attribute}\nuse crate::helper;\n")),
            [("crate::helper".to_owned(), DependencyScope::Test)],
            "{attribute}"
        );
    }
    for attribute in [
        "#[cfg(not(test))]",
        "#[cfg(feature = \"test\")]",
        "#[cfg_attr(test, derive(Debug))]",
    ] {
        assert_eq!(
            rust_scopes(&format!("{attribute}\nuse crate::helper;\n")),
            [("crate::helper".to_owned(), DependencyScope::Default)],
            "{attribute}"
        );
    }
    assert_eq!(
        rust_scopes("use crate::helper;\n"),
        [("crate::helper".to_owned(), DependencyScope::Default)]
    );
}

#[test]
fn an_ancestor_test_module_scopes_every_reference_declared_inside_it() {
    assert_eq!(
        rust_scopes(concat!(
            "use crate::shipped;\n",
            "#[cfg(test)]\n",
            "mod tests {\n",
            "    // a comment between the attribute and the item\n",
            "    mod inner {\n",
            "        mod child;\n",
            "        use crate::helper;\n",
            "        include!(\"generated.rs\");\n",
            "        fn run() { crate::qualified::call(); }\n",
            "    }\n",
            "}\n",
        )),
        [
            ("crate::shipped".to_owned(), DependencyScope::Default),
            ("child".to_owned(), DependencyScope::Test),
            ("crate::helper".to_owned(), DependencyScope::Test),
            ("generated.rs".to_owned(), DependencyScope::Test),
            ("crate::qualified::call".to_owned(), DependencyScope::Test),
        ]
    );
}

#[test]
fn a_comment_between_a_test_attribute_and_its_item_keeps_the_scope() {
    assert_eq!(
        rust_scopes(concat!(
            "#[cfg(test)]\n",
            "// why the module is test only\n",
            "#[allow(unused)]\n",
            "mod tests {\n",
            "    use crate::helper;\n",
            "}\n",
        )),
        [("crate::helper".to_owned(), DependencyScope::Test)]
    );
}

#[test]
fn vue_fixture_keeps_script_and_template_facts_in_original_document_positions() {
    let result = Analyzer::default()
        .analyze(
            Path::new("vue.vue"),
            include_bytes!("fixtures/vue.vue").to_vec(),
        )
        .unwrap();
    assert_eq!(result.language(), Language::Vue);
    assert_eq!(result.parse_status(), &ParseStatus::Parsed);
    let expected = [
        unit(
            "<template>",
            None,
            UnitKind::Template,
            None,
            1,
            6,
            5,
            6,
            3,
            1,
            0,
        ),
        unit("helper", None, UnitKind::Closure, None, 8, 8, 1, 2, 0, 1, 1),
        unit("save", None, UnitKind::Function, None, 9, 12, 1, 2, 2, 1, 1),
    ];
    assert_eq!(result.units().len(), expected.len());
    for (actual, expected) in result.units().iter().zip(expected) {
        assert_eq!(actual.identity().name(), expected.name);
        assert_eq!(actual.identity().container(), expected.container);
        assert_eq!(actual.identity().kind(), expected.kind);
        assert_eq!(actual.parent().map(|id| id.index()), expected.parent);
        assert_eq!(
            (actual.span().start_line(), actual.span().end_line()),
            (expected.start, expected.end)
        );
        assert_eq!(
            (
                actual.measurements().cognitive_complexity(),
                actual.measurements().cyclomatic_complexity(),
                actual.measurements().logical_lines()
            ),
            (expected.cognitive, expected.cyclomatic, expected.logical)
        );
        assert_eq!(
            (
                actual.measurements().max_nesting(),
                actual.measurements().parameter_count()
            ),
            (expected.nesting, expected.parameters)
        );
    }
}

#[test]
fn a_plain_vue_script_region_reports_its_own_nesting_and_parameters() {
    let result = Analyzer::default()
        .analyze(
            Path::new("vue-script.vue"),
            include_bytes!("fixtures/vue-script.vue").to_vec(),
        )
        .unwrap();
    assert_eq!(result.language(), Language::Vue);
    assert_eq!(result.parse_status(), &ParseStatus::Parsed);
    let shapes: Vec<_> = result
        .units()
        .iter()
        .map(|unit| {
            (
                unit.identity().name(),
                unit.measurements().max_nesting(),
                unit.measurements().parameter_count(),
            )
        })
        .collect();
    assert_eq!(shapes, [("<template>", 1, 0), ("save", 2, 2)]);
}

#[test]
fn nested_units_are_separate_where_the_language_supports_them() {
    let cases = [
        (
            "x.cpp",
            "int outer() { auto inner = []() { return 1; }; return inner(); }",
        ),
        (
            "x.java",
            "class X { int outer() { var inner = () -> 1; return inner.get(); } }",
        ),
        (
            "x.js",
            "function outer() { const inner = () => 1; return inner(); }",
        ),
        (
            "x.jsx",
            "function Outer() { const Inner = () => <b />; return <Inner />; }",
        ),
        (
            "x.py",
            "def outer():\n    def inner():\n        return 1\n    return inner()\n",
        ),
        ("x.rs", "fn outer() -> i32 { let inner = || 1; inner() }"),
        (
            "x.ts",
            "function outer(): number { const inner = () => 1; return inner(); }",
        ),
        (
            "x.tsx",
            "function Outer() { const Inner = () => <b />; return <Inner />; }",
        ),
        ("x.rb", "def outer\n  inner = -> { 1 }\n  inner.call\nend\n"),
        (
            "x.vue",
            "<script setup>function outer() { const inner = () => 1; return inner() }</script>",
        ),
    ];
    let mut analyzer = Analyzer::default();
    for (path, source) in cases {
        let result = analyzer
            .analyze(Path::new(path), source.as_bytes().to_vec())
            .unwrap();
        assert!(result.units().len() >= 2, "{path}");
        assert!(
            result
                .units()
                .iter()
                .skip(1)
                .any(|unit| unit.parent().is_some()),
            "{path}"
        );
    }
}

#[test]
fn parser_recovery_is_visible_for_each_grammar_family() {
    let cases = [
        ("x.c", "int broken( { if (x) return 1;"),
        ("x.cpp", "namespace X { int broken( {"),
        ("x.java", "class X { int broken( {"),
        ("x.js", "function broken( { if (x)"),
        ("x.jsx", "function Broken( { return <div>"),
        ("x.py", "def broken(:\n    if x:\n"),
        ("x.rs", "fn broken( { if x {"),
        ("x.ts", "function broken(: number {"),
        ("x.tsx", "function Broken( { return <div>"),
        ("x.rb", "def broken(\n  if value\n"),
        ("x.vue", "<template><div v-if=\"ready></template>"),
    ];
    let mut analyzer = Analyzer::default();
    for (path, source) in cases {
        let result = analyzer
            .analyze(Path::new(path), source.as_bytes().to_vec())
            .unwrap();
        assert_eq!(result.parse_status(), &ParseStatus::Recovered, "{path}");
    }
}

#[test]
fn boolean_operator_runs_and_statement_layout_follow_shared_rules() {
    let result = Analyzer::default()
        .analyze(
            Path::new("rules.js"),
            b"function rules(a, b, c, d) {\n  const ready = a && b && c || d;\n  first(); second();\n  return call(\n    ready\n  );\n}\n"
                .to_vec(),
        )
        .unwrap();
    let unit = &result.units()[0];
    assert_eq!(unit.measurements().cognitive_complexity(), 2);
    assert_eq!(unit.measurements().cyclomatic_complexity(), 4);
    assert_eq!(unit.measurements().logical_lines(), 4);
}

#[test]
fn javascript_family_class_fields_keep_owned_closure_units() {
    let cases = [
        (
            "field.js",
            "class View { handler = (ready) => { if (ready) return 1; return 0; } }",
        ),
        (
            "field.ts",
            "class View { handler = (ready: boolean): number => { if (ready) return 1; return 0; } }",
        ),
    ];
    let mut analyzer = Analyzer::default();
    for (path, source) in cases {
        let result = analyzer
            .analyze(Path::new(path), source.as_bytes().to_vec())
            .unwrap();
        let closure = result
            .units()
            .iter()
            .find(|unit| unit.identity().name() == "handler")
            .unwrap();
        assert_eq!(closure.identity().container(), Some("View"), "{path}");
        assert_eq!(closure.measurements().cyclomatic_complexity(), 2, "{path}");
    }
}

#[test]
fn java_final_else_adds_an_alternative_after_else_if() {
    let mut analyzer = Analyzer::default();
    let without_final_else = analyzer
        .analyze(
            Path::new("without.java"),
            b"class X { int run(int value) { if (value > 1) return 2; else if (value == 1) return 1; return 0; } }".to_vec(),
        )
        .unwrap();
    let with_final_else = analyzer
        .analyze(
            Path::new("with.java"),
            b"class X { int run(int value) { if (value > 1) return 2; else if (value == 1) return 1; else return 0; } }".to_vec(),
        )
        .unwrap();

    let without = without_final_else.units()[0]
        .measurements()
        .cognitive_complexity();
    let with = with_final_else.units()[0]
        .measurements()
        .cognitive_complexity();
    assert_eq!(without, 2);
    assert_eq!(with, 3);
}

#[test]
fn c_family_preprocessing_keeps_function_facts_visible() {
    for path in ["preprocessed.c", "preprocessed.cpp"] {
        let result = Analyzer::default()
            .analyze(
                Path::new(path),
                b"#if FEATURE\nint enabled(int value) { if (value) return 1; return 0; }\n#endif\n"
                    .to_vec(),
            )
            .unwrap();
        assert!(
            result
                .units()
                .iter()
                .any(|unit| unit.identity().name() == "enabled"),
            "{path}"
        );
    }
}

#[test]
fn ruby_modifiers_use_the_same_structural_events_as_block_conditions() {
    let result = Analyzer::default()
        .analyze(
            Path::new("modifier.rb"),
            b"def choose(ready)\n  return 1 if ready\n  return 0\nend\n".to_vec(),
        )
        .unwrap();
    let unit = &result.units()[0];
    assert_eq!(unit.measurements().cognitive_complexity(), 1);
    assert_eq!(unit.measurements().cyclomatic_complexity(), 2);
    assert_eq!(unit.measurements().logical_lines(), 2);
}

const fn file<'a>(
    path: &'a str,
    source: &'a [u8],
    language: Language,
    units: &'a [ExpectedUnit<'a>],
) -> ExpectedFile<'a> {
    ExpectedFile {
        path,
        source,
        language,
        units,
    }
}

#[allow(clippy::too_many_arguments)]
const fn unit<'a>(
    name: &'a str,
    container: Option<&'a str>,
    kind: UnitKind,
    parent: Option<usize>,
    start: u32,
    end: u32,
    cognitive: u32,
    cyclomatic: u32,
    logical: u32,
    nesting: u32,
    parameters: u32,
) -> ExpectedUnit<'a> {
    ExpectedUnit {
        name,
        container,
        kind,
        parent,
        start,
        end,
        cognitive,
        cyclomatic,
        logical,
        nesting,
        parameters,
    }
}
