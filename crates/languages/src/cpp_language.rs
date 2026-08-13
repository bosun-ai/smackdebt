use smackdebt_analysis::{DependencySyntax, Language as ReportLanguage, UnitKind};
use tree_sitter::{Language as Grammar, Node};

use crate::c_language::{c_syntax, declarator_name};
use crate::language::{Language, node_name};
use crate::semantic::Syntax;

pub(crate) struct Cpp;

impl Language for Cpp {
    const REPORT_LANGUAGE: ReportLanguage = ReportLanguage::Cpp;

    fn grammar() -> Grammar {
        tree_sitter_cpp::LANGUAGE.into()
    }

    fn unit_query() -> &'static str {
        "[(function_definition) (lambda_expression)] @unit"
    }

    fn unit_kind(node: Node<'_>) -> Option<UnitKind> {
        match node.kind() {
            "function_definition" if has_class_ancestor(node) => Some(UnitKind::Method),
            "function_definition" => Some(UnitKind::Function),
            "lambda_expression" => Some(UnitKind::Lambda),
            _ => None,
        }
    }

    fn is_container(node: Node<'_>) -> bool {
        matches!(
            node.kind(),
            "namespace_definition" | "class_specifier" | "struct_specifier"
        )
    }

    fn name(node: Node<'_>, source: &[u8]) -> String {
        if node.kind() == "lambda_expression" {
            format!("<lambda {}>", node.start_position().row + 1)
        } else if node.kind() == "function_definition" {
            declarator_name(
                node.child_by_field_name("declarator").unwrap_or(node),
                source,
            )
        } else {
            node_name(node, source)
        }
    }

    fn syntax(node: Node<'_>, source: &[u8]) -> Syntax {
        c_syntax(node, source)
    }

    fn dependency(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
        crate::dependency::include(node, source)
    }
}

fn has_class_ancestor(node: Node<'_>) -> bool {
    let mut parent = node.parent();
    while let Some(candidate) = parent {
        if matches!(candidate.kind(), "class_specifier" | "struct_specifier") {
            return true;
        }
        parent = candidate.parent();
    }
    false
}
