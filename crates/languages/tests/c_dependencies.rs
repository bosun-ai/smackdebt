use smackdebt_analysis::{DependencySyntaxState, ParseStatus};
use smackdebt_languages::Analyzer;
use std::path::Path;

#[test]
fn includes_keep_angle_targets_for_repository_resolution() {
    for path in ["main.c", "main.cpp"] {
        let result = Analyzer::default()
            .analyze(
                Path::new(path),
                b"#include <project/header.h>\n#include HEADER\n".to_vec(),
            )
            .unwrap();
        assert_eq!(result.dependencies().len(), 2);
        assert!(!matches!(
            result.dependencies()[0].state(),
            DependencySyntaxState::External
        ));
        assert!(matches!(
            result.dependencies()[1].state(),
            DependencySyntaxState::Unresolved(_)
        ));
    }
}

#[test]
fn cpp_modules_keep_parser_limitations_visible() {
    let result = Analyzer::default()
        .analyze(
            Path::new("main.cpp"),
            b"export module example;\nimport other;\n".to_vec(),
        )
        .unwrap();
    assert_ne!(result.parse_status(), &ParseStatus::Parsed);
}

#[test]
fn quoted_header_names_preserve_literal_spaces() {
    let result = Analyzer::default()
        .analyze(Path::new("main.c"), b"#include \"a  b.h\"\n".to_vec())
        .unwrap();
    assert_eq!(result.dependencies()[0].target(), "a  b.h");
}

#[test]
fn quoted_includes_retain_typed_style_internal_intent_and_original_spans() {
    for path in ["x.c", "x.cpp"] {
        let result = Analyzer::default()
            .analyze(Path::new(path), b"\n#include \"local.h\"\n".to_vec())
            .unwrap();
        let dependency = &result.dependencies()[0];
        assert_eq!(dependency.target(), "local.h");
        assert_eq!(
            dependency.state(),
            &DependencySyntaxState::Include(smackdebt_analysis::IncludeStyle::Quoted)
        );
        assert_eq!(
            dependency.intent(),
            smackdebt_analysis::DependencyIntent::Internal
        );
        assert_eq!(dependency.span(), smackdebt_analysis::SourceSpan::new(2, 3));
    }
}
