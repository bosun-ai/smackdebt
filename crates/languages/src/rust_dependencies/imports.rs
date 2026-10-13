use super::{ancestors, paths::resolved_path_reference};
use smackdebt_analysis::{DependencyKind, DependencySyntax};
use tree_sitter::Node;

/// One reference per imported item of a grouped `use` list.
///
/// Grammar members are the direct named children of a `use_list`; path
/// segments live under `scoped_use_list` or `scoped_identifier` parents and
/// never match here, so every member is counted exactly once.
pub(super) fn use_list_member(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
    if node.parent()?.kind() != "use_list" {
        return None;
    }
    let prefix = use_list_prefix(node, source);
    let target = match node.kind() {
        "identifier" | "scoped_identifier" | "use_wildcard" => {
            joined_path(prefix, node.utf8_text(source).ok()?.trim())
        }
        // An `as` clause references the original item, never the alias.
        "use_as_clause" => joined_path(
            prefix,
            node.child_by_field_name("path")?
                .utf8_text(source)
                .ok()?
                .trim(),
        ),
        // A `self` member imports the enclosing list's own path.
        "self" => prefix?,
        _ => return None,
    };
    Some(resolved_path_reference(
        node,
        DependencyKind::Import,
        &target,
        false,
        true,
    ))
}

/// The `::`-joined path the enclosing `use` lists prefix a member with.
fn use_list_prefix(node: Node<'_>, source: &[u8]) -> Option<String> {
    let mut prefix = String::new();
    for ancestor in ancestors(node).take_while(|node| node.kind() != "use_declaration") {
        let Some(text) = list_path(ancestor, source) else {
            continue;
        };
        prefix = if prefix.is_empty() {
            text.trim().to_owned()
        } else {
            format!("{}::{prefix}", text.trim())
        };
    }
    (!prefix.is_empty()).then_some(prefix)
}

/// Joins an optional list prefix and one member path with `::`.
fn joined_path(prefix: Option<String>, member: &str) -> String {
    match prefix {
        Some(prefix) => format!("{prefix}::{member}"),
        None => member.to_owned(),
    }
}

/// Removes a leading Rust visibility modifier so it cannot become a target.
pub(super) fn strip_visibility(text: &str) -> &str {
    let Some(rest) = text.strip_prefix("pub") else {
        return text;
    };
    let Some(arguments) = rest.strip_prefix('(') else {
        return if rest.starts_with(char::is_whitespace) {
            rest.trim_start()
        } else {
            text
        };
    };
    let mut depth = 1usize;
    for (index, character) in arguments.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return arguments[index + 1..].trim_start();
                }
            }
            _ => {}
        }
    }
    text
}

fn list_path<'a>(node: Node<'_>, source: &'a [u8]) -> Option<&'a str> {
    if node.kind() != "scoped_use_list" {
        return None;
    }
    node.child_by_field_name("path")?.utf8_text(source).ok()
}
