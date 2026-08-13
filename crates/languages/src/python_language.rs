use smackdebt_analysis::{Language as ReportLanguage, UnitKind};
use tree_sitter::{Language as Grammar, Node};

use crate::language::{Language, node_name};
use crate::language_common::{boolean_syntax, statement};
use crate::semantic::Syntax;

pub(crate) struct Python;

impl Language for Python {
    const REPORT_LANGUAGE: ReportLanguage = ReportLanguage::Python;

    fn grammar() -> Grammar {
        tree_sitter_python::LANGUAGE.into()
    }

    fn unit_query() -> &'static str {
        "[(function_definition) (lambda)] @unit"
    }

    fn unit_kind(node: Node<'_>) -> Option<UnitKind> {
        match node.kind() {
            "function_definition" => Some(UnitKind::Function),
            "lambda" => Some(UnitKind::Lambda),
            _ => None,
        }
    }

    fn is_container(node: Node<'_>) -> bool {
        node.kind() == "class_definition"
    }

    fn name(node: Node<'_>, source: &[u8]) -> String {
        if node.kind() == "lambda" {
            format!("<lambda {}>", node.start_position().row + 1)
        } else {
            node_name(node, source)
        }
    }

    fn syntax(node: Node<'_>, source: &[u8]) -> Syntax {
        match node.kind() {
            "if_statement"
            | "conditional_expression"
            | "for_statement"
            | "while_statement"
            | "except_clause"
            | "with_statement"
            | "list_comprehension"
            | "set_comprehension"
            | "dictionary_comprehension"
            | "generator_expression" => Syntax::structural(true),
            "elif_clause" => Syntax::alternative(true),
            "else_clause" => Syntax::alternative(false),
            "boolean_operator" => boolean_syntax(node, source).unwrap_or_default(),
            kind => statement(
                kind,
                &[
                    "expression_statement",
                    "return_statement",
                    "raise_statement",
                    "assert_statement",
                    "delete_statement",
                    "pass_statement",
                    "break_statement",
                    "continue_statement",
                    "import_statement",
                    "import_from_statement",
                ],
            ),
        }
    }
}
