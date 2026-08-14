use smackdebt_analysis::{DependencySyntax, Language as ReportLanguage, UnitKind};
use tree_sitter::{Language as Grammar, Node};

use crate::language::{Language, node_name};
use crate::language_common::{boolean_syntax, statement};
use crate::semantic::Syntax;

pub(crate) struct Ruby;

impl Language for Ruby {
    const REPORT_LANGUAGE: ReportLanguage = ReportLanguage::Ruby;

    fn grammar() -> Grammar {
        tree_sitter_ruby::LANGUAGE.into()
    }

    fn unit_query() -> &'static str {
        "[(method) (singleton_method) (lambda) (block) (do_block)] @unit"
    }

    fn generated_marker(path: &std::path::Path, source: &[u8]) -> bool {
        let rails_schema = path.file_name().is_some_and(|name| name == "schema.rb")
            && path
                .parent()
                .and_then(std::path::Path::file_name)
                .is_some_and(|name| name == "db");
        rails_schema
            || std::str::from_utf8(source).is_ok_and(|text| {
                text.lines().take(5).any(|line| {
                    let line = line.to_ascii_lowercase();
                    line.contains("generated") && line.contains("do not edit")
                })
            })
    }

    fn unit_kind(node: Node<'_>) -> Option<UnitKind> {
        match node.kind() {
            "method" | "singleton_method" => Some(UnitKind::Method),
            "lambda" => Some(UnitKind::Lambda),
            "block" | "do_block"
                if node
                    .parent()
                    .is_some_and(|parent| parent.kind() == "lambda") =>
            {
                None
            }
            "block" | "do_block" => Some(UnitKind::Closure),
            _ => None,
        }
    }

    fn is_container(node: Node<'_>) -> bool {
        matches!(node.kind(), "class" | "module")
    }

    fn name(node: Node<'_>, source: &[u8]) -> String {
        match node.kind() {
            "lambda" => format!("<lambda {}>", node.start_position().row + 1),
            "block" | "do_block" => {
                format!("<closure {}>", node.start_position().row + 1)
            }
            "singleton_method" => {
                let object = node
                    .child_by_field_name("object")
                    .and_then(|value| value.utf8_text(source).ok())
                    .unwrap_or("self");
                let name = node
                    .child_by_field_name("name")
                    .and_then(|value| value.utf8_text(source).ok())
                    .unwrap_or("<anonymous>");
                format!("{object}.{name}")
            }
            _ => node_name(node, source),
        }
    }

    fn syntax(node: Node<'_>, source: &[u8]) -> Syntax {
        match node.kind() {
            "if" | "unless" | "while" | "until" | "for" | "case" | "rescue" | "if_modifier"
            | "unless_modifier" | "while_modifier" | "until_modifier" => Syntax::structural(true),
            "elsif" | "when" | "else" => Syntax::alternative(node.kind() == "when"),
            "binary" => boolean_syntax(node, source).unwrap_or_default(),
            kind => statement(
                kind,
                &[
                    "assignment",
                    "call",
                    "return",
                    "break",
                    "next",
                    "yield",
                    "operator_assignment",
                ],
            ),
        }
    }

    fn dependency(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
        crate::dependency::ruby(node, source)
    }
}
