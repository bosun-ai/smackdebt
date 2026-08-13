//! Compiled source-language dispatch.
//!
//! The public values in this crate are Smackdebt facts.  The parser and metric
//! structures from `rust-code-analysis` are reduced at this boundary and never
//! appear in a public signature.

use std::path::Path;

use crate::ruby::{RubyBlock, RubyLambda, analyze_ruby};
use crate::upstream::analyze_upstream;
use crate::vue::analyze_vue;
use smackdebt_analysis::{
    FileAnalysis as CoreFileAnalysis, Language, LocalUnitId, Measurements as CoreMeasurements,
    ParseStatus, SourceSpan, UnitFact, UnitIdentity, UnitKind as CoreUnitKind,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum UnitKind {
    Function,
    Method,
    Lambda,
    Template,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Measurements {
    pub(super) cognitive_complexity: u32,
    pub(super) cyclomatic_complexity: u32,
    pub(super) logical_lines: u32,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Span {
    pub(super) start_line: u32,
    pub(super) end_line: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct CodeUnit {
    pub(super) name: String,
    pub(super) container: Option<String>,
    pub(super) kind: UnitKind,
    pub(super) span: Span,
    pub(super) measurements: Measurements,
    pub(super) parent: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct RawFileAnalysis {
    pub(super) language: Language,
    pub(super) source_lines: u32,
    pub(super) parse_status: ParseStatus,
    pub(super) units: Vec<CodeUnit>,
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
fn detect(path: &Path) -> Language {
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
/// Per-worker language state. Parser and scratch details stay private while
/// project orchestration reuses this value across files on the same worker.
#[derive(Default)]
pub struct Analyzer {
    ruby_methods: Vec<RubyBlock>,
    ruby_lambdas: Vec<RubyLambda>,
}

impl Analyzer {
    /// Detects a language from a source path without reading it.
    pub fn language(path: &Path) -> Language {
        detect(path)
    }

    /// Analyzes one source buffer through the compiled language registry.
    pub fn analyze(
        &mut self,
        path: &Path,
        source: Vec<u8>,
    ) -> Result<CoreFileAnalysis, AnalysisError> {
        let language = detect(path);
        if !supported(language) {
            return Err(AnalysisError::Unsupported(language));
        }
        let raw = match language {
            Language::Ruby => Ok(analyze_ruby(
                &source,
                &mut self.ruby_methods,
                &mut self.ruby_lambdas,
            )),
            Language::Vue => analyze_vue(&source),
            Language::C
            | Language::Cpp
            | Language::Java
            | Language::JavaScript
            | Language::Jsx
            | Language::Python
            | Language::Rust
            | Language::TypeScript
            | Language::Tsx => analyze_upstream(language, source, path),
            Language::Kotlin | Language::Unknown => Err(AnalysisError::Unsupported(language)),
        }?;
        Ok(to_core(raw))
    }
}

fn to_core(raw: RawFileAnalysis) -> CoreFileAnalysis {
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
    CoreFileAnalysis::new(raw.language, raw.source_lines, raw.parse_status, units)
}

pub(super) fn line_count(source: &[u8]) -> u32 {
    if source.is_empty() {
        0
    } else {
        source.iter().filter(|&&byte| byte == b'\n').count() as u32
            + u32::from(source.last() != Some(&b'\n'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze(path: &str, source: &str) -> CoreFileAnalysis {
        Analyzer::default()
            .analyze(Path::new(path), source.as_bytes().to_vec())
            .unwrap()
    }

    #[test]
    fn only_verified_languages_are_supported() {
        assert_eq!(detect(Path::new("x.kt")), Language::Kotlin);
        assert!(
            Analyzer::default()
                .analyze(Path::new("x.kt"), b"fun x() {}".to_vec())
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
