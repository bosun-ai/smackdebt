//! Compiled source-language dispatch.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::engine::Scratch;
use crate::registry::Parsers;
use smackdebt_analysis::{FileAnalysis, Language};

static PARSER_TIME_NS: AtomicU64 = AtomicU64::new(0);

pub(crate) fn record_parser_time(elapsed: std::time::Duration) {
    PARSER_TIME_NS.fetch_add(
        u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX),
        Ordering::Relaxed,
    );
}

#[doc(hidden)]
pub fn reset_parser_time() {
    PARSER_TIME_NS.store(0, Ordering::Relaxed);
}

#[doc(hidden)]
pub fn parser_time_ns() -> u64 {
    PARSER_TIME_NS.load(Ordering::Relaxed)
}

fn detect(path: &Path) -> Language {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(Language::from_filename)
        .unwrap_or(Language::Unknown)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnalysisError {
    Unsupported(Language),
    Parser(String),
}

impl std::fmt::Display for AnalysisError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(language) => {
                write!(formatter, "{} is not supported", language.name())
            }
            Self::Parser(message) => write!(formatter, "parser failed: {message}"),
        }
    }
}

impl std::error::Error for AnalysisError {}

/// Per-worker source analysis state. Each parser is reused by consecutive files.
#[derive(Default)]
pub struct Analyzer {
    parsers: Parsers,
    scratch: Scratch,
}

impl Analyzer {
    pub fn language(path: &Path) -> Language {
        detect(path)
    }

    pub fn has_generated_marker(path: &Path, source: &[u8]) -> bool {
        Parsers::generated_marker(detect(path), path, source)
    }

    pub fn analyze(&mut self, path: &Path, source: Vec<u8>) -> Result<FileAnalysis, AnalysisError> {
        self.parsers
            .analyze(detect(path), &source, &mut self.scratch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smackdebt_analysis::{ParseStatus, RecoveredFacts, UnitKind};

    fn analyze(path: &str, source: &str) -> FileAnalysis {
        Analyzer::default()
            .analyze(Path::new(path), source.as_bytes().to_vec())
            .unwrap()
    }

    #[test]
    fn dependency_calls_and_exports_are_read_from_exact_syntax() {
        let javascript = analyze(
            "x.ts",
            "export type WorkflowRunGraph = { note: 'from elsewhere' };\n\
             export { run } from './run.js';\n\
             required.value('./not-a-module');\n\
             require('./needed.js');\n",
        );
        let targets: Vec<_> = javascript
            .dependencies()
            .iter()
            .map(smackdebt_analysis::DependencySyntax::target)
            .collect();
        assert_eq!(targets, ["./run.js", "./needed.js"]);

        let ruby = analyze(
            "x.rb",
            "requires :reason\nrequired.value('not-a-module')\nrequire_relative './needed'\n",
        );
        let targets: Vec<_> = ruby
            .dependencies()
            .iter()
            .map(smackdebt_analysis::DependencySyntax::target)
            .collect();
        assert_eq!(targets, ["./needed"]);
    }

    #[test]
    fn listed_languages_use_owned_parsers_and_recognized_documents_stay_explicit() {
        for (path, source) in [
            ("x.c", "int x(void) { return 1; }"),
            ("x.cpp", "int x() { return 1; }"),
            ("x.java", "class X { int x() { return 1; } }"),
            ("x.js", "function x() { return 1; }"),
            ("x.jsx", "function X() { return <div />; }"),
            ("x.py", "def x():\n    return 1\n"),
            ("x.rs", "fn x() -> i32 { 1 }"),
            ("x.ts", "function x(): number { return 1; }"),
            ("x.tsx", "function X() { return <div />; }"),
            ("x.rb", "def x\n  1\nend\n"),
        ] {
            let result = analyze(path, source);
            assert_eq!(result.parse_status(), &ParseStatus::Parsed, "{path}");
            assert!(!result.units().is_empty(), "{path}");
        }
        assert!(matches!(
            Analyzer::default().analyze(Path::new("x.kt"), b"fun x() {}".to_vec()),
            Err(AnalysisError::Unsupported(Language::Kotlin))
        ));
        assert_eq!(Analyzer::language(Path::new("x.astro")), Language::Astro);
        assert!(matches!(
            Analyzer::default().analyze(Path::new("x.astro"), b"<h1>Hello</h1>".to_vec()),
            Err(AnalysisError::Unsupported(Language::Astro))
        ));
    }

    #[test]
    fn nested_units_have_independent_measurements_and_original_spans() {
        let result = analyze(
            "x.rs",
            "fn outer() {\n let inner = || { if ready { work(); } };\n if done { work(); }\n}\n",
        );
        let outer = &result.units()[0];
        let closure = &result.units()[1];
        assert_eq!(outer.measurements().cyclomatic_complexity(), 2);
        assert!(closure.measurements().cyclomatic_complexity() > 1);
        assert_eq!(closure.parent(), Some(outer.local_id()));
        assert_eq!(closure.span().start_line(), 2);
    }

    #[test]
    fn ruby_methods_singletons_and_closures_share_metric_algorithms() {
        let result = analyze(
            "x.rb",
            "class Cart\n  def total(items)\n    items.each do |item|\n      if item\n        puts item\n      end\n    end\n  end\n  def self.empty\n    []\n  end\nend\n",
        );
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
                .any(|unit| unit.identity().kind() == UnitKind::Closure)
        );
    }

    #[test]
    fn recovered_trees_still_return_visible_facts() {
        let result = analyze("x.py", "def broken(:\n    if yes:\n        pass\n");
        assert_eq!(
            result.parse_status(),
            &ParseStatus::Recovered(RecoveredFacts::InDoubt)
        );
    }

    #[test]
    fn each_language_owns_its_generated_header_rules() {
        for (path, source) in [
            ("x.c", "/* generated file */\n"),
            ("x.cpp", "// code generated\n"),
            ("X.java", "// auto generated\n"),
            ("x.js", "// @generated\n"),
            ("x.jsx", "// @generated\n"),
            ("x.py", "# generated - do not edit\n"),
            ("x.rs", "// @generated\n"),
            ("x.ts", "// @generated\n"),
            ("x.tsx", "// @generated\n"),
            ("x.rb", "# generated - do not edit\n"),
            ("x.vue", "<!-- @generated -->\n"),
        ] {
            assert!(
                Analyzer::has_generated_marker(Path::new(path), source.as_bytes()),
                "{path}"
            );
        }
        assert!(!Analyzer::has_generated_marker(
            Path::new("x.py"),
            b"# generated helper\n"
        ));
        for path in [
            "x.c", "x.cpp", "X.java", "x.js", "x.jsx", "x.py", "x.rs", "x.ts", "x.tsx", "x.rb",
            "x.vue",
        ] {
            assert!(
                !Analyzer::has_generated_marker(
                    Path::new(path),
                    b"// do not edit this hand-written section\n"
                ),
                "{path}"
            );
        }
    }

    #[test]
    fn ruby_owns_the_rails_schema_path_rule_without_claiming_similar_user_files() {
        assert!(Analyzer::has_generated_marker(
            Path::new("db/schema.rb"),
            b"def complex_schema; if one; if two; end; end; end\n"
        ));
        assert!(Analyzer::has_generated_marker(
            Path::new("app/db/schema.rb"),
            b"def complex_schema; if one; if two; end; end; end\n"
        ));
        for path in [
            "schema.rb",
            "db/schema_helper.rb",
            "database/schema.rb",
            "app/models/schema.rb",
        ] {
            assert!(
                !Analyzer::has_generated_marker(
                    Path::new(path),
                    b"def user_schema; if one; if two; end; end; end\n"
                ),
                "{path}"
            );
        }
    }
}
