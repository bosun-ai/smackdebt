//! The single compiled registration of source analyzers and their worker state.
use crate::analyzer::AnalysisError;
use crate::c_language::C;
use crate::cpp_language::Cpp;
use crate::csharp_language::CSharp;
use crate::engine::{self, Scratch};
use crate::go_language::Go;
use crate::java_language::Java;
use crate::javascript_language::{JavaScript, Jsx, Tsx, TypeScript};
use crate::language::Language as LanguageContract;
use crate::php_language::Php;
use crate::python_language::Python;
use crate::ruby_language::Ruby;
use crate::rust_language::Rust;
use smackdebt_analysis::{FileAnalysis, Language};
use std::path::Path;
use tree_sitter::Parser;

macro_rules! analyzers {
    ($($variant:ident => $field:ident : $implementation:ty),* $(,)?) => {
        #[derive(Default)]
        pub(super) struct Parsers { $($field: Parser,)* vue: Parser }

        #[derive(Clone, Copy)]
        enum Slot { $($variant,)* Vue }
        pub(super) const PARSER_COUNT: usize = Slot::Vue as usize + 1;

        pub(super) fn query_slot(language: Language) -> usize {
            match language {
                $(Language::$variant => Slot::$variant as usize,)*
                Language::Vue => Slot::Vue as usize,
                _ => unreachable!("only registered grammars request a query"),
            }
        }

        impl Parsers {
            pub(super) fn analyze(&mut self, language: Language, source: &[u8], scratch: &mut Scratch) -> Result<FileAnalysis, AnalysisError> {
                match language {
                    $(Language::$variant => engine::analyze::<$implementation>(&mut self.$field, source, scratch),)*
                    Language::Vue => crate::vue::analyze_vue(&mut self.vue, &mut self.javascript, &mut self.typescript, source, scratch),
                    language => return Err(AnalysisError::Unsupported(language)),
                }.map_err(AnalysisError::Parser)
            }

            pub(super) fn generated_marker(language: Language, path: &Path, source: &[u8]) -> bool {
                match language {
                    $(Language::$variant => <$implementation>::generated_marker(path, source),)*
                    Language::Vue => crate::vue::has_generated_marker(source),
                    _ => false,
                }
            }
        }
    };
}

analyzers! {
    C => c: C,
    Cpp => cpp: Cpp,
    Java => java: Java,
    JavaScript => javascript: JavaScript,
    Jsx => jsx: Jsx,
    Python => python: Python,
    Rust => rust: Rust,
    TypeScript => typescript: TypeScript,
    Tsx => tsx: Tsx,
    Ruby => ruby: Ruby,
    Go => go: Go,
    Php => php: Php,
    CSharp => csharp: CSharp,
}
