use smackdebt_analysis::{Language, ParseStatus};
use smackdebt_languages::Analyzer;
use std::path::Path;

fn analyze(path: &str, source: &str) -> smackdebt_analysis::FileAnalysis {
    Analyzer::default()
        .analyze(Path::new(path), source.as_bytes().to_vec())
        .unwrap()
}

#[test]
fn php_measures_nested_work_independently() {
    let result = analyze("sample.php", include_str!("fixtures/php.php"));
    assert_eq!(result.language(), Language::Php);
    assert_eq!(result.parse_status(), &ParseStatus::Parsed);
    let actual: Vec<_> = result
        .units()
        .iter()
        .map(|unit| {
            let measurements = unit.measurements();
            (
                unit.identity().name(),
                measurements.cognitive_complexity(),
                measurements.cyclomatic_complexity(),
                measurements.logical_lines(),
                measurements.max_nesting(),
                measurements.parameter_count(),
                unit.parent().map(|id| id.index()),
            )
        })
        .collect();
    assert_eq!(
        actual,
        [
            ("outer", 1, 2, 3, 1, 3, None),
            ("$inner", 1, 2, 2, 1, 1, Some(0))
        ]
    );
}

#[test]
fn php_namespaces_keep_equal_method_names_distinct() {
    let result = analyze(
        "x.php",
        "<?php namespace One { class Item { function run() {} } } namespace Two { class Item { function run() {} } }",
    );
    assert_eq!(
        result
            .units()
            .iter()
            .map(|unit| unit.identity().container())
            .collect::<Vec<_>>(),
        [Some("One.Item"), Some("Two.Item")]
    );
}

#[test]
fn php_counts_promoted_parameters_without_closure_captures() {
    let result = analyze(
        "x.php",
        "<?php class X { function __construct(public int $a, protected string $b) {} } $f = function($x) use ($captured) {};",
    );
    assert_eq!(
        result
            .units()
            .iter()
            .map(|unit| unit.measurements().parameter_count())
            .collect::<Vec<_>>(),
        [0, 2, 1]
    );
}

#[test]
fn mixed_php_keeps_top_level_work_and_expression_bodies() {
    let result = analyze(
        "x.php",
        "<h1>Heading</h1><?php if ($ready) { echo 'yes'; } $f = fn($x) => $x ? 1 : 0; ?>",
    );
    let values: Vec<_> = result
        .units()
        .iter()
        .map(|unit| {
            let m = unit.measurements();
            (
                m.cognitive_complexity(),
                m.cyclomatic_complexity(),
                m.logical_lines(),
                m.max_nesting(),
                m.parameter_count(),
            )
        })
        .collect();
    assert_eq!(values, [(1, 2, 2, 1, 0), (1, 2, 1, 1, 1)]);
}

#[test]
fn php_failures_templates_and_generated_markers_remain_visible() {
    assert_ne!(
        analyze("x.php", "<?php function broken( { if ($x) {").parse_status(),
        &ParseStatus::Parsed
    );
    assert!(matches!(
        Analyzer::default().analyze(Path::new("view.blade.php"), b"<h1>view</h1>".to_vec()),
        Err(smackdebt_languages::AnalysisError::Unsupported(_))
    ));
    assert!(Analyzer::has_generated_marker(
        Path::new("x.php"),
        b"<?php\n// @generated\n"
    ));
    assert!(!Analyzer::has_generated_marker(
        Path::new("x.php"),
        b"<?php $message = '@generated';"
    ));
}
