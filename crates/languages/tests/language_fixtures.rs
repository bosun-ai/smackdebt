use std::path::Path;

use smackdebt_analysis::{Language, ParseStatus, UnitKind};
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
            )],
        ),
        file(
            "cpp.cpp",
            include_bytes!("fixtures/cpp.cpp"),
            Language::Cpp,
            &[
                unit("run", Some("Worker"), UnitKind::Method, None, 4, 8, 1, 2, 3),
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
                ),
                unit("run", Some("Worker"), UnitKind::Method, None, 3, 7, 2, 3, 3),
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
                ),
            ],
        ),
        file(
            "python.py",
            include_bytes!("fixtures/python.py"),
            Language::Python,
            &[
                unit("outer", None, UnitKind::Function, None, 1, 12, 0, 1, 2),
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
                ),
            ],
        ),
        file(
            "rust.rs",
            include_bytes!("fixtures/rust.rs"),
            Language::Rust,
            &[
                unit("run", Some("Work"), UnitKind::Method, None, 2, 2, 0, 1, 0),
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
                ),
                unit("run", Some("Worker"), UnitKind::Method, None, 3, 6, 1, 2, 2),
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
                ),
            ],
        ),
        file(
            "ruby.rb",
            include_bytes!("fixtures/ruby.rb"),
            Language::Ruby,
            &[
                unit("run", Some("Worker"), UnitKind::Method, None, 3, 9, 0, 1, 2),
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
        }
    }
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
        unit("<template>", None, UnitKind::Template, None, 1, 6, 5, 6, 3),
        unit("helper", None, UnitKind::Closure, None, 8, 8, 1, 2, 0),
        unit("save", None, UnitKind::Function, None, 9, 12, 1, 2, 2),
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
    }
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
    }
}
