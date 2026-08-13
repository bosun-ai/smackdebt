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

pub(super) fn statement(kind: &str, kinds: &[&str]) -> Syntax {
    if kinds.contains(&kind) {
        Syntax::statement()
    } else {
        Syntax::default()
    }
}
