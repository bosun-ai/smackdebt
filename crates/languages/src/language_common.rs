//! Helpers shared by concrete language syntax translators.

use tree_sitter::Node;

use crate::semantic::{CognitiveEvent, Syntax};

pub(super) fn text_contains(node: Node<'_>, source: &[u8], needle: &str) -> bool {
    node.utf8_text(source)
        .is_ok_and(|text| text.contains(needle))
}

pub(super) fn boolean_syntax(node: Node<'_>, source: &[u8]) -> Option<Syntax> {
    let field_operator = node
        .child_by_field_name("operator")
        .and_then(|operator| operator.utf8_text(source).ok());
    let mut cursor = node.walk();
    let child_operator = node
        .children(&mut cursor)
        .filter(|child| !child.is_named())
        .find_map(|child| match child.kind() {
            "&&" | "and" => Some(CognitiveEvent::BooleanAnd),
            "||" | "or" => Some(CognitiveEvent::BooleanOr),
            _ => None,
        });
    match field_operator {
        Some("&&" | "and") => Some(Syntax::boolean(CognitiveEvent::BooleanAnd)),
        Some("||" | "or") => Some(Syntax::boolean(CognitiveEvent::BooleanOr)),
        _ => child_operator.map(Syntax::boolean),
    }
}

/// Counts the parameters a unit declares through the common grammar fields.
///
/// A grammar names the list `parameters`; a shorthand form such as a
/// single-identifier arrow function names one `parameter` instead. Anything
/// else declares no parameters.
pub(super) fn declared_parameter_count(node: Node<'_>) -> u32 {
    node.child_by_field_name("parameters").map_or_else(
        || u32::from(node.child_by_field_name("parameter").is_some()),
        parameter_list_count,
    )
}

/// Counts the declared parameters reached through a declarator chain.
///
/// C-family grammars name the parameter list on the declarator rather than on
/// the unit, so the chain is followed until a parameter list appears.
pub(super) fn declarator_parameter_count(node: Node<'_>) -> u32 {
    let mut current = node;
    loop {
        if let Some(list) = current.child_by_field_name("parameters") {
            return parameter_list_count(list);
        }
        match current.child_by_field_name("declarator") {
            Some(next) => current = next,
            None => return 0,
        }
    }
}

/// Counts the declared parameters of one parameter list node.
///
/// A grammar that names a single shorthand parameter, such as an inferred
/// lambda parameter, exposes the parameter itself rather than a list.
fn parameter_list_count(list: Node<'_>) -> u32 {
    if !list.kind().contains("parameter") {
        return 1;
    }
    let mut cursor = list.walk();
    list.named_children(&mut cursor)
        .filter(|child| !matches!(child.kind(), "comment" | "line_comment" | "block_comment"))
        .count() as u32
}

pub(super) fn statement(kind: &str, kinds: &[&str]) -> Syntax {
    if kinds.contains(&kind) {
        Syntax::statement()
    } else {
        Syntax::default()
    }
}
