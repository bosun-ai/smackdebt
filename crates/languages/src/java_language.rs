use smackdebt_analysis::{DependencySyntax, Language as ReportLanguage, UnitKind};
use tree_sitter::{Language as Grammar, Node};

use crate::language::{Language, node_name};
use crate::language_common::{boolean_syntax, statement};
use crate::semantic::Syntax;

pub(crate) struct Java;

impl Language for Java {
    const REPORT_LANGUAGE: ReportLanguage = ReportLanguage::Java;

    fn grammar() -> Grammar {
        tree_sitter_java::LANGUAGE.into()
    }

    fn unit_query() -> &'static str {
        "[(method_declaration) (constructor_declaration) (lambda_expression)] @unit"
    }

    fn generated_marker(_path: &std::path::Path, source: &[u8]) -> bool {
        marker(source)
    }

    fn unit_kind(node: Node<'_>) -> Option<UnitKind> {
        match node.kind() {
            "method_declaration" | "constructor_declaration" => Some(UnitKind::Method),
            "lambda_expression" => Some(UnitKind::Lambda),
            _ => None,
        }
    }

    fn is_container(node: Node<'_>) -> bool {
        matches!(
            node.kind(),
            "class_declaration"
                | "interface_declaration"
                | "enum_declaration"
                | "record_declaration"
        )
    }

    fn name(node: Node<'_>, source: &[u8]) -> String {
        if node.kind() == "lambda_expression" {
            format!("<lambda {}>", node.start_position().row + 1)
        } else {
            node_name(node, source)
        }
    }

    fn syntax(node: Node<'_>, source: &[u8]) -> Syntax {
        match node.kind() {
            "if_statement" if is_else_if(node) => {
                let syntax = Syntax::alternative(true);
                if has_plain_else(node) {
                    syntax.with_alternative()
                } else {
                    syntax
                }
            }
            "if_statement" => {
                let syntax = Syntax::structural(true);
                if has_plain_else(node) {
                    syntax.with_alternative()
                } else {
                    syntax
                }
            }
            "switch_expression"
            | "for_statement"
            | "enhanced_for_statement"
            | "while_statement"
            | "do_statement"
            | "catch_clause"
            | "ternary_expression" => Syntax::structural(true),
            "switch_label" => Syntax::alternative(true),
            "binary_expression" => boolean_syntax(node, source).unwrap_or_default(),
            kind => statement(
                kind,
                &[
                    "expression_statement",
                    "local_variable_declaration",
                    "return_statement",
                    "throw_statement",
                    "break_statement",
                    "continue_statement",
                    "assert_statement",
                ],
            ),
        }
    }

    fn dependency(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
        crate::dependency::java(node, source)
    }
}

fn marker(source: &[u8]) -> bool {
    std::str::from_utf8(source).is_ok_and(|text| {
        text.lines().take(5).any(|line| {
            let line = line.to_ascii_lowercase();
            line.contains("generated") && (line.contains("do not edit") || line.contains("auto"))
        })
    })
}

fn is_else_if(node: Node<'_>) -> bool {
    node.parent().is_some_and(|parent| {
        parent.kind() == "if_statement"
            && parent
                .child_by_field_name("alternative")
                .is_some_and(|child| child.id() == node.id())
    })
}

fn has_plain_else(node: Node<'_>) -> bool {
    node.child_by_field_name("alternative")
        .is_some_and(|child| child.kind() != "if_statement")
}
