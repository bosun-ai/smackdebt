mod names;
use crate::language::{Language, node_name};
use crate::language_common::qualified_name as qualified;
use crate::language_common::{boolean_syntax, statement};
use crate::semantic::Syntax;
use smackdebt_analysis::SourceNames;
use smackdebt_analysis::{
    DependencyKind, DependencySyntax, DependencySyntaxState, Language as ReportLanguage,
    SourceSpan, UnitKind,
};
use tree_sitter::{Language as Grammar, Node};

pub(crate) struct Php;
impl Language for Php {
    const REPORT_LANGUAGE: ReportLanguage = ReportLanguage::Php;
    fn grammar() -> Grammar {
        tree_sitter_php::LANGUAGE_PHP.into()
    }
    fn unit_query() -> &'static str {
        "[(program) (function_definition) (method_declaration) (anonymous_function) (arrow_function)] @unit"
    }
    fn unit_kind(node: Node<'_>) -> Option<UnitKind> {
        match node.kind() {
            "program" if has_top_level_work(node) => Some(UnitKind::SyntheticTopLevel),
            "function_definition" => Some(UnitKind::Function),
            "method_declaration" if node.child_by_field_name("body").is_some() => {
                Some(UnitKind::Method)
            }
            "anonymous_function" | "arrow_function" => Some(UnitKind::Closure),
            _ => None,
        }
    }
    fn is_container(node: Node<'_>) -> bool {
        matches!(
            node.kind(),
            "class_declaration"
                | "trait_declaration"
                | "interface_declaration"
                | "enum_declaration"
                | "anonymous_class"
        )
    }
    fn generated_marker(_: &std::path::Path, source: &[u8]) -> bool {
        std::str::from_utf8(source).is_ok_and(|text| {
            text.lines().take(8).any(|line| {
                let line = line.trim().to_ascii_lowercase();
                (line.starts_with("//")
                    || line.starts_with('#')
                    || line.starts_with("/*")
                    || line.starts_with('*'))
                    && (line.contains("@generated")
                        || line.contains("generated") && line.contains("do not edit"))
            })
        })
    }
    fn name(node: Node<'_>, source: &[u8]) -> String {
        match node.kind() {
            "program" => "<top-level>".to_owned(),
            "anonymous_class" => format!("<class {}>", node.start_position().row + 1),
            "anonymous_function" | "arrow_function" => binding(node, source)
                .unwrap_or_else(|| format!("<closure {}>", node.start_position().row + 1)),
            _ => node_name(node, source),
        }
    }
    fn has_declared_identity(node: Node<'_>, _: &[u8]) -> bool {
        node.kind() == "program" || node.child_by_field_name("name").is_some()
    }
    fn match_anchor(node: Node<'_>, source: &[u8]) -> Option<String> {
        binding(node, source).map(|name| format!("binding:{name}"))
    }
    fn syntax(node: Node<'_>, source: &[u8]) -> Syntax {
        match node.kind() {
            "if_statement"
            | "for_statement"
            | "foreach_statement"
            | "while_statement"
            | "do_statement"
            | "switch_statement"
            | "match_expression"
            | "catch_clause"
            | "conditional_expression" => Syntax::structural(true),
            "else_if_clause" | "case_statement" | "match_conditional_expression" => {
                Syntax::alternative(true)
            }
            "else_clause" | "default_statement" | "match_default_expression" => {
                Syntax::alternative(false)
            }
            "binary_expression" => boolean_syntax(node, source).unwrap_or_default(),
            kind => statement(
                kind,
                &[
                    "expression_statement",
                    "return_statement",
                    "throw_expression",
                    "echo_statement",
                    "unset_statement",
                    "break_statement",
                    "continue_statement",
                    "goto_statement",
                    "global_declaration",
                    "function_static_declaration",
                    "arrow_function",
                ],
            ),
        }
    }
    fn collect_names(node: Node<'_>, source: &[u8], names: &mut SourceNames) {
        names::collect(node, source, names);
    }
    fn container(node: Node<'_>, source: &[u8]) -> Option<String> {
        let namespace = names::namespace(node, source);
        let owner = std::iter::successors(node.parent(), |node| node.parent())
            .find(|node| Self::is_container(*node));
        let name = owner.map(|owner| Self::name(owner, source));
        match (namespace.is_empty(), name) {
            (_, Some(name)) => Some(qualified(&namespace, &name)),
            (false, None) => Some(namespace),
            _ => None,
        }
    }
    fn dependency(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
        if let Some(reference) = names::dependency(node, source) {
            return Some(reference);
        }
        let kind = match node.kind() {
            "include_expression" | "include_once_expression" => DependencyKind::Include,
            "require_expression" | "require_once_expression" => DependencyKind::Require,
            _ => return None,
        };
        let expression = node.named_child(0)?;
        let text = expression.utf8_text(source).ok()?;
        let state = literal_path(expression, source).map_or_else(
            || DependencySyntaxState::Unresolved("dependency target is dynamic".to_owned()),
            |target| {
                if target.starts_with('/') {
                    DependencySyntaxState::Unresolved(
                        "absolute include paths are not repository-safe".to_owned(),
                    )
                } else {
                    DependencySyntaxState::Candidates(vec![format!("./{target}")])
                }
            },
        );
        Some(
            DependencySyntax::new(
                kind,
                text.split_whitespace().collect::<Vec<_>>().join(" "),
                SourceSpan::new(
                    node.start_position().row as u32 + 1,
                    node.end_position().row as u32 + 1,
                ),
                state,
            )
            .with_internal_intent(),
        )
    }
}
fn has_top_level_work(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).any(|child| {
        (child.kind() == "namespace_definition"
            && child
                .child_by_field_name("body")
                .is_some_and(has_top_level_work))
            || matches!(
                child.kind(),
                "expression_statement"
                    | "echo_statement"
                    | "if_statement"
                    | "for_statement"
                    | "foreach_statement"
                    | "while_statement"
                    | "do_statement"
                    | "switch_statement"
                    | "return_statement"
            )
    })
}
fn binding(node: Node<'_>, source: &[u8]) -> Option<String> {
    if !matches!(node.kind(), "anonymous_function" | "arrow_function") {
        return None;
    }
    let parent = node.parent()?;
    let name = parent.child_by_field_name("left")?;
    Some(name.utf8_text(source).ok()?.trim().to_owned())
}
fn literal_path(node: Node<'_>, source: &[u8]) -> Option<String> {
    if node.kind() == "parenthesized_expression" {
        return literal_path(node.named_child(0)?, source);
    }
    if matches!(node.kind(), "string" | "encapsed_string") {
        let text = node.utf8_text(source).ok()?;
        if text.contains('$') || text.contains('\\') {
            return None;
        }
        return Some(text[1..text.len().checked_sub(1)?].to_owned());
    }
    if node.kind() == "binary_expression" {
        if node
            .child_by_field_name("operator")
            .and_then(|operator| operator.utf8_text(source).ok())
            != Some(".")
        {
            return None;
        }
        let left = node.child_by_field_name("left")?;
        if left.utf8_text(source).ok()? != "__DIR__" {
            return None;
        }
        let right = node.child_by_field_name("right")?;
        return literal_path(right, source).map(|path| path.trim_start_matches('/').to_owned());
    }
    None
}
