use std::path::Path;

use smackdebt_analysis::{Language, ParseStatus};

use crate::analyzer::{
    AnalysisError, CodeUnit, Measurements, RawFileAnalysis, Span, UnitKind, line_count,
};

pub(super) fn analyze_upstream(
    language: Language,
    source: Vec<u8>,
    path: &Path,
) -> Result<RawFileAnalysis, AnalysisError> {
    let upstream = match language {
        Language::Java => rust_code_analysis::LANG::Java,
        Language::C | Language::Cpp => rust_code_analysis::LANG::Cpp,
        Language::JavaScript => rust_code_analysis::LANG::Javascript,
        Language::Jsx => rust_code_analysis::LANG::Mozjs,
        Language::Python => rust_code_analysis::LANG::Python,
        Language::Rust => rust_code_analysis::LANG::Rust,
        Language::TypeScript => rust_code_analysis::LANG::Typescript,
        Language::Tsx => rust_code_analysis::LANG::Tsx,
        _ => return Err(AnalysisError::Unsupported(language)),
    };
    match upstream {
        rust_code_analysis::LANG::Cpp => {
            analyze_parser::<rust_code_analysis::CppParser>(language, source, path)
        }
        rust_code_analysis::LANG::Java => {
            analyze_parser::<rust_code_analysis::JavaParser>(language, source, path)
        }
        rust_code_analysis::LANG::Javascript => {
            analyze_parser::<rust_code_analysis::JavascriptParser>(language, source, path)
        }
        rust_code_analysis::LANG::Mozjs => {
            analyze_parser::<rust_code_analysis::MozjsParser>(language, source, path)
        }
        rust_code_analysis::LANG::Python => {
            analyze_parser::<rust_code_analysis::PythonParser>(language, source, path)
        }
        rust_code_analysis::LANG::Rust => {
            analyze_parser::<rust_code_analysis::RustParser>(language, source, path)
        }
        rust_code_analysis::LANG::Typescript => {
            analyze_parser::<rust_code_analysis::TypescriptParser>(language, source, path)
        }
        rust_code_analysis::LANG::Tsx => {
            analyze_parser::<rust_code_analysis::TsxParser>(language, source, path)
        }
        _ => Err(AnalysisError::Unsupported(language)),
    }
}

fn analyze_parser<T: rust_code_analysis::ParserTrait>(
    language: Language,
    source: Vec<u8>,
    path: &Path,
) -> Result<RawFileAnalysis, AnalysisError> {
    let parser = T::new(source, path, None);
    let source_lines = line_count(parser.get_code());
    let parse_status = if parser.get_root().has_error() {
        ParseStatus::Recovered
    } else {
        ParseStatus::Parsed
    };
    let Some(root) = rust_code_analysis::metrics(&parser, path) else {
        return Err(AnalysisError::Parser("no metric space returned".to_owned()));
    };
    let mut units: Vec<CodeUnit> = Vec::new();
    flatten_upstream(&root, None, None, &mut units);
    Ok(RawFileAnalysis {
        language,
        source_lines,
        parse_status,
        units,
    })
}

fn flatten_upstream(
    space: &rust_code_analysis::FuncSpace,
    parent: Option<usize>,
    container: Option<&str>,
    output: &mut Vec<CodeUnit>,
) {
    let is_container = matches!(
        space.kind,
        rust_code_analysis::SpaceKind::Class
            | rust_code_analysis::SpaceKind::Struct
            | rust_code_analysis::SpaceKind::Trait
            | rust_code_analysis::SpaceKind::Impl
            | rust_code_analysis::SpaceKind::Namespace
            | rust_code_analysis::SpaceKind::Interface
    );
    let (current_container, function_index) = if is_container {
        (
            Some(clean_name(space.name.as_deref().unwrap_or("<anonymous>"))),
            None,
        )
    } else if matches!(space.kind, rust_code_analysis::SpaceKind::Function) {
        let child_lines: usize = space
            .spaces
            .iter()
            .map(|child| child.metrics.loc.lloc() as usize)
            .sum();
        let lines = (space.metrics.loc.lloc() as usize).saturating_sub(child_lines);
        let index = output.len();
        output.push(CodeUnit {
            name: clean_name(space.name.as_deref().unwrap_or("<anonymous>")),
            container: container
                .map(str::to_owned)
                .or_else(|| parent.map(|index| output[index].name.clone())),
            kind: UnitKind::Function,
            span: Span {
                start_line: space.start_line.max(1) as u32,
                end_line: space.end_line.max(space.start_line).max(1) as u32,
            },
            measurements: Measurements {
                cognitive_complexity: nonnegative_metric(space.metrics.cognitive.cognitive()),
                cyclomatic_complexity: nonnegative_metric(space.metrics.cyclomatic.cyclomatic()),
                logical_lines: lines as u32,
            },
            parent,
        });
        (container.map(str::to_owned), Some(index))
    } else {
        (container.map(str::to_owned), None)
    };

    for child in &space.spaces {
        flatten_upstream(
            child,
            function_index.or(parent),
            current_container.as_deref(),
            output,
        );
    }
}

fn nonnegative_metric(value: f64) -> u32 {
    if value.is_finite() && value >= 0.0 {
        value.round() as u32
    } else {
        0
    }
}

fn clean_name(name: &str) -> String {
    name.rsplit('/').next().unwrap_or(name).trim().to_owned()
}
