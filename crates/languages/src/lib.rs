#![forbid(unsafe_code)]

//! Compiled source-language dispatch.
//!
//! The public values in this crate are Smackdebt facts.  The parser and metric
//! structures from `rust-code-analysis` are reduced at this boundary and never
//! appear in a public signature.

use std::path::Path;

use smackdebt_analysis::{
    FileAnalysis as CoreFileAnalysis, FileId, Language, LocalUnitId,
    Measurements as CoreMeasurements, ParseStatus, SourceSpan, UnitFact, UnitIdentity,
    UnitKind as CoreUnitKind,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum UnitKind {
    Function,
    Method,
    Lambda,
    Template,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Measurements {
    cognitive_complexity: u32,
    cyclomatic_complexity: u32,
    logical_lines: u32,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Span {
    start_line: u32,
    end_line: u32,
}

#[derive(Clone, Debug, PartialEq)]
struct CodeUnit {
    name: String,
    container: Option<String>,
    kind: UnitKind,
    span: Span,
    measurements: Measurements,
    parent: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
struct RawFileAnalysis {
    language: Language,
    source_lines: u32,
    parse_status: ParseStatus,
    units: Vec<CodeUnit>,
}

fn language_name(language: Language) -> &'static str {
    match language {
        Language::C => "C",
        Language::Cpp => "C++",
        Language::Java => "Java",
        Language::JavaScript | Language::Jsx => "JavaScript",
        Language::Python => "Python",
        Language::Rust => "Rust",
        Language::TypeScript | Language::Tsx => "TypeScript",
        Language::Ruby => "Ruby",
        Language::Vue => "Vue",
        Language::Kotlin => "Kotlin",
        Language::Unknown => "Unknown",
    }
}

fn supported(language: Language) -> bool {
    !matches!(language, Language::Kotlin | Language::Unknown)
}

/// Detects a language from a source path. Detection never reads source bytes.
pub fn detect(path: &Path) -> Language {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if name == "Rakefile" || name == "Gemfile" {
        return Language::Ruby;
    }
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
    {
        "c" => Language::C,
        "h" | "cc" | "hh" | "cpp" | "hpp" | "cxx" | "hxx" => Language::Cpp,
        "java" => Language::Java,
        "js" | "mjs" | "cjs" => Language::JavaScript,
        "jsx" => Language::Jsx,
        "py" => Language::Python,
        "rs" => Language::Rust,
        "ts" | "mts" | "cts" => Language::TypeScript,
        "tsx" => Language::Tsx,
        "rb" | "rake" | "gemspec" => Language::Ruby,
        "vue" => Language::Vue,
        "kt" | "kts" => Language::Kotlin,
        _ => Language::Unknown,
    }
}

/// An error before a file analysis can be produced.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnalysisError {
    Unsupported(Language),
    Parser(String),
}

impl std::fmt::Display for AnalysisError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(language) => {
                write!(formatter, "{} is not supported", language_name(*language))
            }
            Self::Parser(message) => write!(formatter, "parser failed: {message}"),
        }
    }
}

impl std::error::Error for AnalysisError {}

/// An owned source buffer supplied by the project worker.
pub struct SourceFile<'a> {
    pub file: FileId,
    pub path: &'a Path,
    pub source: Vec<u8>,
}

/// Per-worker language state. Parser and scratch details stay private while
/// project orchestration reuses this value across files on the same worker.
#[derive(Default)]
pub struct Analyzer {
    ruby_methods: Vec<RubyBlock>,
    ruby_lambdas: Vec<RubyLambda>,
}

impl Analyzer {
    /// Analyzes one source buffer through the compiled language registry.
    pub fn analyze(&mut self, source: SourceFile<'_>) -> Result<CoreFileAnalysis, AnalysisError> {
        let language = detect(source.path);
        if !supported(language) {
            return Err(AnalysisError::Unsupported(language));
        }
        let raw = match language {
            Language::Ruby => Ok(analyze_ruby(
                &source.source,
                &mut self.ruby_methods,
                &mut self.ruby_lambdas,
            )),
            Language::Vue => analyze_vue(&source.source),
            Language::C
            | Language::Cpp
            | Language::Java
            | Language::JavaScript
            | Language::Jsx
            | Language::Python
            | Language::Rust
            | Language::TypeScript
            | Language::Tsx => analyze_upstream(language, source.source, source.path),
            Language::Kotlin | Language::Unknown => Err(AnalysisError::Unsupported(language)),
        }?;
        Ok(to_core(source.file, raw))
    }
}

/// Analyze one selected source buffer using the compiled registry.
pub fn analyze(source: SourceFile<'_>) -> Result<CoreFileAnalysis, AnalysisError> {
    Analyzer::default().analyze(source)
}

fn to_core(file: FileId, raw: RawFileAnalysis) -> CoreFileAnalysis {
    let units = raw
        .units
        .into_iter()
        .enumerate()
        .map(|(index, unit)| {
            let kind = match unit.kind {
                UnitKind::Function => CoreUnitKind::Function,
                UnitKind::Method => CoreUnitKind::Method,
                UnitKind::Lambda => CoreUnitKind::Lambda,
                UnitKind::Template => CoreUnitKind::Template,
            };
            let identity = match unit.container {
                Some(container) => UnitIdentity::new(unit.name, kind).in_container(container),
                None => UnitIdentity::new(unit.name, kind),
            };
            UnitFact::new(
                LocalUnitId::from_index(index),
                identity,
                SourceSpan::new(unit.span.start_line, unit.span.end_line),
                CoreMeasurements::new(
                    unit.measurements.cognitive_complexity,
                    unit.measurements.cyclomatic_complexity,
                    unit.measurements.logical_lines,
                ),
                unit.parent.map(LocalUnitId::from_index),
            )
        })
        .collect();
    CoreFileAnalysis::new(
        file,
        raw.language,
        raw.source_lines,
        raw.parse_status,
        units,
    )
}

fn analyze_upstream(
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

fn line_count(source: &[u8]) -> u32 {
    if source.is_empty() {
        0
    } else {
        source.iter().filter(|&&byte| byte == b'\n').count() as u32
            + u32::from(source.last() != Some(&b'\n'))
    }
}

fn analyze_ruby(
    source: &[u8],
    methods: &mut Vec<RubyBlock>,
    lambdas: &mut Vec<RubyLambda>,
) -> RawFileAnalysis {
    let text = String::from_utf8_lossy(source);
    let lines: Vec<&str> = text.lines().collect();
    let mut units: Vec<CodeUnit> = Vec::new();
    let mut containers: Vec<(String, usize)> = Vec::new();
    methods.clear();
    lambdas.clear();
    let mut block_depth = 0usize;
    let mut index = 0usize;
    while index < lines.len() {
        let line = lines[index].trim();
        if let Some(name) = ruby_container_start(line) {
            containers.push((name, block_depth));
        }
        if let Some(name) = ruby_method_start(line) {
            let start_depth = block_depth;
            let end = ruby_block_end(&lines, index, start_depth);
            for lambda_line in index..=end {
                let lambda_count = lines[lambda_line].matches("->").count()
                    + lines[lambda_line].matches("lambda").count();
                for ordinal in 0..lambda_count {
                    lambdas.push(RubyLambda {
                        name: format!("<lambda {}>", ordinal + 1),
                        start: lambda_line,
                        end: lambda_end(&lines, lambda_line),
                        method_start: index,
                    });
                }
            }
            methods.push(RubyBlock {
                name,
                start: index,
                end,
                container: containers.last().map(|container| container.0.clone()),
            });
            index = end;
            block_depth = start_depth;
        }
        if line == "end" || line.starts_with("end ") {
            if let Some((_, depth)) = containers.last()
                && *depth >= block_depth.saturating_sub(1)
            {
                containers.pop();
            }
            block_depth = block_depth.saturating_sub(1);
        } else if ruby_opens_block(line) {
            block_depth += 1;
        }
        index += 1;
    }

    for method in methods.drain(..) {
        let nested_ranges: Vec<(usize, usize)> = lambdas
            .iter()
            .filter(|lambda| lambda.method_start == method.start)
            .map(|lambda| (lambda.start, lambda.end))
            .collect();
        let local_ranges: Vec<(usize, usize)> = nested_ranges
            .iter()
            .map(|(start, end)| {
                (
                    start.saturating_sub(method.start),
                    end.saturating_sub(method.start),
                )
            })
            .collect();
        let measurements = ruby_measurements(&lines[method.start..=method.end], &local_ranges);
        let index = units.len();
        units.push(CodeUnit {
            name: method.name,
            container: method.container,
            kind: UnitKind::Method,
            span: Span {
                start_line: (method.start + 1) as u32,
                end_line: (method.end + 1) as u32,
            },
            measurements,
            parent: None,
        });
        for lambda in lambdas
            .iter()
            .filter(|lambda| lambda.method_start == method.start)
        {
            let measurements = ruby_measurements(&lines[lambda.start..=lambda.end], &[]);
            units.push(CodeUnit {
                name: lambda.name.clone(),
                container: Some(units[index].name.clone()),
                kind: UnitKind::Lambda,
                span: Span {
                    start_line: (lambda.start + 1) as u32,
                    end_line: (lambda.end + 1) as u32,
                },
                measurements,
                parent: Some(index),
            });
        }
    }
    RawFileAnalysis {
        language: Language::Ruby,
        source_lines: line_count(source),
        parse_status: if block_depth == 0 && containers.is_empty() {
            ParseStatus::Parsed
        } else {
            ParseStatus::Recovered
        },
        units,
    }
}

#[derive(Clone)]
struct RubyBlock {
    name: String,
    start: usize,
    end: usize,
    container: Option<String>,
}

#[derive(Clone)]
struct RubyLambda {
    name: String,
    start: usize,
    end: usize,
    method_start: usize,
}

fn ruby_container_start(line: &str) -> Option<String> {
    line.strip_prefix("class ")
        .or_else(|| line.strip_prefix("module "))
        .map(|name| {
            name.split_whitespace()
                .next()
                .unwrap_or("<anonymous>")
                .to_owned()
        })
}

fn ruby_method_start(line: &str) -> Option<String> {
    let name = line.strip_prefix("def ")?.trim();
    Some(
        name.split(['(', ' ', '\t'])
            .next()
            .unwrap_or("<anonymous>")
            .to_owned(),
    )
}

fn ruby_opens_block(line: &str) -> bool {
    let first = line.split_whitespace().next().unwrap_or_default();
    matches!(
        first,
        "def" | "class" | "module" | "if" | "unless" | "case" | "while" | "until" | "for" | "begin"
    ) || line.contains(" do")
        || line.ends_with("{")
}

fn ruby_block_end(lines: &[&str], start: usize, depth: usize) -> usize {
    let mut level = depth + 1;
    for (index, line) in lines.iter().enumerate().skip(start + 1) {
        let line = line.trim();
        if ruby_opens_block(line) {
            level += 1;
        }
        if line == "end" || line.starts_with("end ") {
            level = level.saturating_sub(1);
            if level == depth {
                return index;
            }
        }
    }
    lines.len().saturating_sub(1)
}

fn lambda_end(lines: &[&str], start: usize) -> usize {
    let mut level = lines[start]
        .matches('{')
        .count()
        .saturating_sub(lines[start].matches('}').count());
    for (index, line) in lines.iter().enumerate().skip(start + 1) {
        level += line.matches('{').count();
        level = level.saturating_sub(line.matches('}').count());
        if level == 0 {
            return index;
        }
    }
    start
}

fn ruby_measurements(lines: &[&str], nested: &[(usize, usize)]) -> Measurements {
    let mut cognitive = 0u32;
    let mut cyclomatic = 1u32;
    let mut logical = 0u32;
    let mut nesting = 0u32;
    for (offset, line) in lines.iter().enumerate() {
        let global = offset;
        if nested
            .iter()
            .any(|(start, end)| global >= *start && global <= *end)
        {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        logical += 1;
        let first = trimmed.split_whitespace().next().unwrap_or_default();
        let measured_block =
            !matches!(first, "def" | "class" | "module") && ruby_opens_block(trimmed);
        if measured_block {
            cognitive += 1 + nesting;
            cyclomatic += u32::from(matches!(
                first,
                "if" | "unless" | "case" | "while" | "until" | "for"
            ));
            nesting += 1;
        }
        let branches = trimmed.matches("&&").count()
            + trimmed.matches("||").count()
            + trimmed.matches(" ? ").count();
        cognitive += branches as u32;
        cyclomatic += branches as u32;
        if trimmed == "end" || trimmed.starts_with("end ") {
            nesting = nesting.saturating_sub(1);
        }
    }
    Measurements {
        cognitive_complexity: cognitive,
        cyclomatic_complexity: cyclomatic,
        logical_lines: logical,
    }
}

fn analyze_vue(source: &[u8]) -> Result<RawFileAnalysis, AnalysisError> {
    let text = String::from_utf8_lossy(source);
    let lines: Vec<&str> = text.lines().collect();
    let mut units = Vec::new();
    let mut status = ParseStatus::Parsed;
    let mut cursor = 0usize;
    while cursor < lines.len() {
        let line = lines[cursor].trim();
        if line.starts_with("<script") {
            let is_type_script = line.contains("lang=\"ts") || line.contains("lang='ts");
            let (start, end, content) = if let Some(close) = line.find("</script>") {
                let open = line.find('>').map_or(0, |position| position + 1);
                (cursor, cursor, line[open..close].to_owned())
            } else {
                let start = cursor + 1;
                let end = lines
                    .iter()
                    .enumerate()
                    .skip(cursor + 1)
                    .find(|(_, value)| value.contains("</script>"))
                    .map(|(index, _)| index)
                    .unwrap_or_else(|| {
                        status = ParseStatus::Recovered;
                        lines.len().saturating_sub(1)
                    });
                let content_end = end.saturating_sub(1).min(lines.len().saturating_sub(1));
                let content = if start < end {
                    lines[start..=content_end].join("\n")
                } else {
                    String::new()
                };
                (start, end, content)
            };
            let path = if is_type_script {
                Path::new("component.ts")
            } else {
                Path::new("component.js")
            };
            if !content.is_empty() {
                let script = analyze_upstream(
                    if is_type_script {
                        Language::TypeScript
                    } else {
                        Language::JavaScript
                    },
                    content.as_bytes().to_vec(),
                    path,
                )?;
                let parent_offset = units.len();
                for mut unit in script.units {
                    unit.span.start_line += start as u32;
                    unit.span.end_line += start as u32;
                    unit.parent = unit.parent.map(|parent| parent + parent_offset);
                    units.push(unit);
                }
            }
            cursor = end;
        } else if line.starts_with("<template") {
            let start = cursor;
            let end = if line.contains("</template>") {
                cursor
            } else {
                lines
                    .iter()
                    .enumerate()
                    .skip(cursor + 1)
                    .find(|(_, value)| value.contains("</template>"))
                    .map(|(index, _)| index)
                    .unwrap_or_else(|| {
                        status = ParseStatus::Recovered;
                        lines.len().saturating_sub(1)
                    })
            };
            let body = lines[start..=end].join("\n");
            units.push(CodeUnit {
                name: "<template>".to_owned(),
                container: None,
                kind: UnitKind::Template,
                span: Span {
                    start_line: (start + 1) as u32,
                    end_line: (end + 1) as u32,
                },
                measurements: template_measurements(&body),
                parent: None,
            });
            cursor = end;
        }
        cursor += 1;
    }
    Ok(RawFileAnalysis {
        language: Language::Vue,
        source_lines: line_count(source),
        parse_status: status,
        units,
    })
}

fn template_measurements(body: &str) -> Measurements {
    let logical_lines = body
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with("<!--"))
        .count() as u32;
    let inline_handlers = body.matches(" @").count() + body.matches("v-on:").count();
    let cognitive = (body.matches("v-if").count()
        + body.matches("v-else-if").count()
        + body.matches("v-for").count()
        + body.matches(" ? ").count()
        + body.matches("&&").count()
        + body.matches("||").count()
        + inline_handlers) as u32;
    let branches = body.matches("v-if").count()
        + body.matches("v-else-if").count()
        + body.matches("v-for").count()
        + body.matches(" ? ").count()
        + body.matches("&&").count()
        + body.matches("||").count()
        + inline_handlers;
    Measurements {
        cognitive_complexity: cognitive,
        cyclomatic_complexity: branches as u32 + 1,
        logical_lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze(path: &str, source: &str) -> CoreFileAnalysis {
        super::analyze(SourceFile {
            file: FileId::from_index(0),
            path: Path::new(path),
            source: source.as_bytes().to_vec(),
        })
        .unwrap()
    }

    #[test]
    fn only_verified_languages_are_supported() {
        assert_eq!(detect(Path::new("x.kt")), Language::Kotlin);
        assert!(
            super::analyze(SourceFile {
                file: FileId::from_index(0),
                path: Path::new("x.kt"),
                source: b"fun x() {}".to_vec()
            })
            .is_err()
        );
        for path in ["x.c", "x.cpp", "x.java", "x.js", "x.py", "x.rs", "x.ts"] {
            assert!(supported(detect(Path::new(path))), "{path}");
        }
    }

    #[test]
    fn upstream_nested_units_have_direct_measurements() {
        let samples = [
            (
                "x.c",
                "int inner(int x) { if (x) return 1; return 0; }\nint outer(int x) { return inner(x); }",
            ),
            (
                "x.cpp",
                "int inner(int x) { if (x) return 1; return 0; }\nint outer(int x) { return inner(x); }",
            ),
            (
                "x.java",
                "class A { int inner(int x) { if (x > 0) return 1; return 0; } }",
            ),
            (
                "x.js",
                "function inner(x) { if (x) return 1; return 0; }\nfunction outer(x) { return inner(x); }",
            ),
            (
                "x.py",
                "def inner(x):\n    if x:\n        return 1\n    return 0\n",
            ),
            ("x.rs", "fn inner(x: bool) -> i32 { if x { return 1; } 0 }"),
            (
                "x.ts",
                "function inner(x: boolean): number { if (x) return 1; return 0; }",
            ),
        ];
        for (path, source) in samples {
            let result = analyze(path, source);
            assert!(!result.units().is_empty(), "{path}");
            assert!(
                result
                    .units()
                    .iter()
                    .any(|unit| unit.measurements().cyclomatic_complexity() > 0),
                "{path}"
            );
            assert!(
                result
                    .units()
                    .iter()
                    .any(|unit| unit.measurements().logical_lines() > 0),
                "{path}"
            );
            assert!(
                result
                    .units()
                    .iter()
                    .any(|unit| unit.measurements().cognitive_complexity() > 0),
                "{path}"
            );
        }
    }

    #[test]
    fn jsx_and_tsx_use_their_syntax_aware_parsers() {
        for (path, source) in [
            (
                "x.jsx",
                "function View() { return <section>{ready && <b>yes</b>}</section>; }",
            ),
            (
                "x.tsx",
                "function View(props: { ready: boolean }) { return <section>{props.ready && <b>yes</b>}</section>; }",
            ),
        ] {
            let result = analyze(path, source);
            assert_eq!(result.parse_status(), &ParseStatus::Parsed, "{path}");
            assert!(!result.units().is_empty(), "{path}");
        }
    }

    #[test]
    fn ruby_methods_singletons_and_lambdas_are_owned_units() {
        let result = analyze(
            "x.rb",
            "class Cart\n  def total(items)\n    items.each do |item|\n      if item\n        puts item\n      end\n    end\n    mapper = ->(item) { item * 2 }\n  end\n  def self.empty\n    []\n  end\nend\n",
        );
        assert_eq!(result.language(), Language::Ruby);
        assert!(
            result
                .units()
                .iter()
                .any(|unit| unit.identity().name() == "total")
        );
        assert!(
            result
                .units()
                .iter()
                .any(|unit| unit.identity().name() == "self.empty")
        );
        assert!(
            result
                .units()
                .iter()
                .any(|unit| unit.identity().kind() == CoreUnitKind::Lambda)
        );
        assert!(
            result
                .units()
                .iter()
                .any(|unit| unit.measurements().cognitive_complexity() > 0)
        );
    }

    #[test]
    fn empty_ruby_method_has_no_control_flow_complexity() {
        let result = analyze("x.rb", "def empty\nend\n");
        let method = result.units().first().unwrap();
        assert_eq!(method.measurements().cognitive_complexity(), 0);
        assert_eq!(method.measurements().cyclomatic_complexity(), 1);
    }

    #[test]
    fn vue_delegates_scripts_and_adds_a_template_unit() {
        let result = analyze(
            "App.vue",
            "<template>\n  <div v-if=\"ok\" @click=\"save\">{{ ok ? 'yes' : 'no' }}</div>\n</template>\n<script setup lang=\"ts\">\nfunction save(value: boolean) { if (value) return 1; return 0 }\n</script>\n<style scoped>\n.x { color: red }\n</style>\n",
        );
        assert_eq!(result.language(), Language::Vue);
        assert!(
            result
                .units()
                .iter()
                .any(|unit| unit.identity().kind() == CoreUnitKind::Template
                    && unit.measurements().cognitive_complexity() > 0)
        );
        assert!(
            result
                .units()
                .iter()
                .any(|unit| unit.identity().name() == "save" && unit.span().start_line() > 3)
        );
        assert_eq!(result.source_lines(), 9);
    }

    #[test]
    fn vue_handles_compact_scripts_and_general_event_handlers() {
        let result = analyze(
            "App.vue",
            "<template><input v-on:focus=\"load\" @keydown.enter=\"save\"></template>\n<script>function save() { return 1 }</script>\n",
        );
        assert!(
            result
                .units()
                .iter()
                .any(|unit| unit.identity().name() == "save")
        );
        let template = result
            .units()
            .iter()
            .find(|unit| unit.identity().kind() == CoreUnitKind::Template)
            .unwrap();
        assert!(template.measurements().cognitive_complexity() >= 2);
    }
}
