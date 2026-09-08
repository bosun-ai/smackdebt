use crate::language::{Language, node_name};
use crate::language_common::{boolean_syntax, statement};
use crate::semantic::Syntax;
use smackdebt_analysis::{
    DependencyKind, DependencySyntax, DependencySyntaxState, Language as ReportLanguage,
    SourceSpan, UnitKind,
};
use tree_sitter::{Language as Grammar, Node};

pub(crate) struct Go;
impl Language for Go {
    const REPORT_LANGUAGE: ReportLanguage = ReportLanguage::Go;
    fn grammar() -> Grammar {
        tree_sitter_go::LANGUAGE.into()
    }
    fn unit_query() -> &'static str {
        "[(function_declaration) (method_declaration) (func_literal)] @unit"
    }
    fn unit_kind(node: Node<'_>) -> Option<UnitKind> {
        match node.kind() {
            "function_declaration" if node.child_by_field_name("body").is_some() => {
                Some(UnitKind::Function)
            }
            "method_declaration" if node.child_by_field_name("body").is_some() => {
                Some(UnitKind::Method)
            }
            "func_literal" => Some(UnitKind::Closure),
            _ => None,
        }
    }
    fn is_container(_: Node<'_>) -> bool {
        false
    }
    fn container(node: Node<'_>, source: &[u8]) -> Option<String> {
        let receiver = node
            .child_by_field_name("receiver")?
            .named_child(0)?
            .child_by_field_name("type")?;
        Some(
            receiver
                .utf8_text(source)
                .ok()?
                .trim_start_matches('*')
                .to_owned(),
        )
    }
    fn generated_marker(_: &std::path::Path, source: &[u8]) -> bool {
        std::str::from_utf8(source).is_ok_and(|text| {
            text.lines()
                .take_while(|line| !line.trim_start().starts_with("package "))
                .any(|line| {
                    let line = line.trim();
                    line.starts_with("// Code generated ") && line.ends_with(" DO NOT EDIT.")
                })
        })
    }
    fn name(node: Node<'_>, source: &[u8]) -> String {
        if node.kind() == "func_literal" {
            binding(node, source)
                .unwrap_or_else(|| format!("<closure {}>", node.start_position().row + 1))
        } else {
            node_name(node, source)
        }
    }
    fn match_anchor(node: Node<'_>, source: &[u8]) -> Option<String> {
        binding(node, source).map(|name| format!("binding:{name}"))
    }
    fn parameter_count(node: Node<'_>, _: &[u8]) -> u32 {
        let Some(parameters) = node.child_by_field_name("parameters") else {
            return 0;
        };
        let mut cursor = parameters.walk();
        parameters
            .named_children(&mut cursor)
            .filter(|node| {
                matches!(
                    node.kind(),
                    "parameter_declaration" | "variadic_parameter_declaration"
                )
            })
            .map(|parameter| {
                let mut cursor = parameter.walk();
                parameter
                    .children_by_field_name("name", &mut cursor)
                    .count()
                    .max(1) as u32
            })
            .sum()
    }
    fn syntax(node: Node<'_>, source: &[u8]) -> Syntax {
        match node.kind() {
            "if_statement"
                if node
                    .parent()
                    .is_some_and(|parent| parent.kind() == "if_statement") =>
            {
                alternative(node)
            }
            "if_statement" => {
                let syntax = Syntax::structural(true);
                if plain_else(node) {
                    syntax.with_alternative()
                } else {
                    syntax
                }
            }
            "for_statement"
            | "expression_switch_statement"
            | "type_switch_statement"
            | "select_statement" => Syntax::structural(true),
            "expression_case" | "type_case" | "communication_case" => Syntax::alternative(true),
            "default_case" => Syntax::alternative(false),
            "binary_expression" => boolean_syntax(node, source).unwrap_or_default(),
            kind => statement(
                kind,
                &[
                    "short_var_declaration",
                    "var_spec",
                    "const_spec",
                    "assignment_statement",
                    "expression_statement",
                    "return_statement",
                    "inc_statement",
                    "dec_statement",
                    "send_statement",
                    "receive_statement",
                    "go_statement",
                    "defer_statement",
                    "break_statement",
                    "continue_statement",
                    "fallthrough_statement",
                    "goto_statement",
                ],
            ),
        }
    }
    fn dependency(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
        if node.kind() != "import_spec" {
            return None;
        }
        let path = node.child_by_field_name("path")?.utf8_text(source).ok()?;
        let target = path.trim_matches(['"', '`']);
        Some(DependencySyntax::new(
            DependencyKind::Import,
            target,
            SourceSpan::new(
                node.start_position().row as u32 + 1,
                node.end_position().row as u32 + 1,
            ),
            DependencySyntaxState::PackageMembers,
        ))
    }
}
fn plain_else(node: Node<'_>) -> bool {
    node.child_by_field_name("alternative")
        .is_some_and(|node| node.kind() != "if_statement")
}
fn alternative(node: Node<'_>) -> Syntax {
    let syntax = Syntax::alternative(true);
    if plain_else(node) {
        syntax.with_alternative()
    } else {
        syntax
    }
}
fn binding(node: Node<'_>, source: &[u8]) -> Option<String> {
    if node.kind() != "func_literal" {
        return None;
    }
    let parent = node.parent()?;
    let declaration = if parent.kind() == "expression_list" {
        parent.parent()?
    } else {
        parent
    };
    let name = declaration
        .child_by_field_name("left")
        .or_else(|| declaration.child_by_field_name("name"))?;
    Some(name.utf8_text(source).ok()?.trim().to_owned())
}
