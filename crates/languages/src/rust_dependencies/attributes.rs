use super::ancestors;
use tree_sitter::Node;

/// Whether the item declaring this reference only exists under `cfg(test)`.
///
/// The rule is syntactic: it reads the outer attributes of the declaring item
/// and of every module that contains it, and never evaluates a configuration
/// predicate or consults enabled features.
pub(super) fn declared_in_test_scope(node: Node<'_>, source: &[u8]) -> bool {
    outer_attributes_select_test(node, source)
        || ancestors(node)
            .filter(|item| matches!(item.kind(), "mod_item" | "use_declaration"))
            .any(|item| outer_attributes_select_test(item, source))
}

/// Reads the run of attributes that immediately precedes an item.
fn outer_attributes_select_test(item: Node<'_>, source: &[u8]) -> bool {
    let mut sibling = item.prev_named_sibling();
    while let Some(current) = sibling {
        match current.kind() {
            "attribute_item" => {
                if attribute_selects_test(current, source) {
                    return true;
                }
            }
            "line_comment" | "block_comment" => {}
            _ => return false,
        }
        sibling = current.prev_named_sibling();
    }
    false
}

/// Whether one attribute is a `cfg` predicate that selects the test configuration.
fn attribute_selects_test(attribute_item: Node<'_>, source: &[u8]) -> bool {
    let Some(attribute) = attribute_item.named_child(0) else {
        return false;
    };
    if attribute.kind() != "attribute" {
        return false;
    }
    let Some(path) = attribute.named_child(0) else {
        return false;
    };
    if path.kind() != "identifier" || path.utf8_text(source).ok() != Some("cfg") {
        return false;
    }
    attribute
        .named_child(1)
        .filter(|arguments| arguments.kind() == "token_tree")
        .is_some_and(|arguments| token_tree_selects_test(arguments, source))
}

/// Whether a `cfg` token tree names `test` outside a negated predicate.
fn token_tree_selects_test(tree: Node<'_>, source: &[u8]) -> bool {
    let mut cursor = tree.walk();
    let mut negated = false;
    for child in tree.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                let text = child.utf8_text(source).ok();
                if text == Some("test") {
                    return true;
                }
                negated = text == Some("not");
            }
            "token_tree" => {
                if !negated && token_tree_selects_test(child, source) {
                    return true;
                }
                negated = false;
            }
            _ => negated = false,
        }
    }
    false
}

pub(super) fn explicit_module_path<'a>(
    node: Node<'_>,
    source: &'a [u8],
) -> Option<Result<&'a str, ()>> {
    let mut sibling = node.prev_named_sibling();
    while let Some(attribute) = sibling {
        if !matches!(
            attribute.kind(),
            "attribute_item" | "line_comment" | "block_comment"
        ) {
            break;
        }
        if attribute.kind() == "attribute_item" {
            let text = attribute.utf8_text(source).ok()?.trim();
            let body = text.strip_prefix("#[")?.strip_suffix(']')?.trim();
            if let Some(value) = body
                .strip_prefix("path")
                .and_then(|v| v.trim_start().strip_prefix('='))
            {
                let value = value.trim();
                return Some(
                    value
                        .strip_prefix('"')
                        .and_then(|v| v.strip_suffix('"'))
                        .filter(|v| !v.contains('"') && !v.contains('\\') && !v.starts_with('/'))
                        .ok_or(()),
                );
            }
        }
        sibling = attribute.prev_named_sibling();
    }
    None
}
