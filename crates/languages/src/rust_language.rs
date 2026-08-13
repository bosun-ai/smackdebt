use smackdebt_analysis::{Language as ReportLanguage, UnitKind};
use tree_sitter::{Language as Grammar, Node};

use crate::language::{Language, node_name};
use crate::language_common::{boolean_syntax, statement, text_contains};
use crate::semantic::{CognitiveEvent, Syntax};

pub(crate) struct Rust;

impl Language for Rust {
    const REPORT_LANGUAGE: ReportLanguage = ReportLanguage::Rust;

    fn grammar() -> Grammar {
        tree_sitter_rust::LANGUAGE.into()
    }

    fn unit_query() -> &'static str {
        "[(function_item) (function_signature_item) (closure_expression)] @unit"
    }

    fn unit_kind(node: Node<'_>) -> Option<UnitKind> {
        match node.kind() {
            "function_item" if has_method_ancestor(node) => Some(UnitKind::Method),
            "function_item" => Some(UnitKind::Function),
            "function_signature_item" => Some(UnitKind::Method),
            "closure_expression" => Some(UnitKind::Closure),
            _ => None,
        }
    }

    fn is_container(node: Node<'_>) -> bool {
        matches!(
            node.kind(),
            "struct_item" | "enum_item" | "trait_item" | "impl_item" | "mod_item"
        )
    }

    fn name(node: Node<'_>, source: &[u8]) -> String {
        if node.kind() == "closure_expression" {
            format!("<closure {}>", node.start_position().row + 1)
        } else if node.kind() == "impl_item" {
            node.child_by_field_name("type")
                .and_then(|name| name.utf8_text(source).ok())
                .unwrap_or("impl")
                .to_owned()
        } else {
            node_name(node, source)
        }
    }

    fn syntax(node: Node<'_>, source: &[u8]) -> Syntax {
        match node.kind() {
            "if_expression" | "while_expression" | "for_expression" | "loop_expression" => {
                Syntax::structural(true)
            }
            "match_expression" => Syntax::structural(false),
            "match_arm" => Syntax::alternative(true),
            "else_clause" => Syntax::alternative(false),
            "break_expression" | "continue_expression" if text_contains(node, source, "'") => {
                Syntax {
                    cognitive: Some(CognitiveEvent::Jump),
                    ..Syntax::statement()
                }
            }
            "binary_expression" => boolean_syntax(node, source).unwrap_or_default(),
            "expression_statement"
                if node.named_child(0).is_some_and(|child| {
                    matches!(
                        child.kind(),
                        "if_expression"
                            | "while_expression"
                            | "for_expression"
                            | "loop_expression"
                            | "match_expression"
                    )
                }) =>
            {
                Syntax::default()
            }
            kind => statement(
                kind,
                &[
                    "let_declaration",
                    "expression_statement",
                    "return_expression",
                    "break_expression",
                    "continue_expression",
                ],
            ),
        }
    }
}

fn has_method_ancestor(node: Node<'_>) -> bool {
    let mut parent = node.parent();
    while let Some(candidate) = parent {
        if matches!(candidate.kind(), "impl_item" | "trait_item") {
            return true;
        }
        parent = candidate.parent();
    }
    false
}
