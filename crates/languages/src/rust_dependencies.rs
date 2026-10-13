mod attributes;
mod imports;
mod paths;

use crate::dependency::{candidates, external, string_value, unresolved};
use attributes::{declared_in_test_scope, explicit_module_path};
use imports::{strip_visibility, use_list_member};
use paths::resolved_path_reference;
use smackdebt_analysis::{DependencyKind, DependencySyntax, StaticRelationKind};
use tree_sitter::Node;

pub(super) fn rust(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
    let dependency = rust_reference(node, source)?;
    let dependency = if node.kind() != "mod_item" {
        {
            let path = inline_module_path(node, source);
            if path.is_empty() {
                dependency
            } else {
                dependency.with_module_path(path)
            }
        }
    } else {
        dependency
    };
    Some(if declared_in_test_scope(node, source) {
        dependency.with_test_scope()
    } else {
        dependency
    })
}

fn rust_reference(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
    if let Some(member) = use_list_member(node, source) {
        return Some(member);
    }
    let kind = reference_kind(node)?;
    let text = strip_visibility(node.utf8_text(source).ok()?.trim());
    if reference_is_nested(node, text) {
        return None;
    }
    if node.kind() == "macro_invocation" {
        return include_reference(node, text);
    }
    if node.kind() == "extern_crate_declaration" {
        return Some(external(node, kind, text));
    }
    if node.kind() == "mod_item" && !text.ends_with(';') {
        return None;
    }
    let target = text
        .strip_prefix("use ")
        .and_then(|value| value.strip_suffix(';'))
        .or_else(|| {
            text.strip_prefix("mod ")
                .and_then(|value| value.strip_suffix(';'))
        })
        .unwrap_or(text)
        .trim()
        .split(['{', ',', ' '])
        .next()
        .unwrap_or("")
        .trim_end_matches("::");
    if target.is_empty() {
        return Some(unresolved(
            node,
            kind,
            text,
            "dependency target is malformed",
        ));
    }
    if node.kind() == "mod_item"
        && let Some(path) = explicit_module_path(node, source)
    {
        return explicit_reference(node, source, target, path);
    }
    Some(resolved_path_reference(
        node,
        kind,
        target,
        node.kind() == "mod_item",
        matches!(node.kind(), "use_declaration" | "scoped_identifier"),
    ))
}

/// The nodes that contain this node, innermost first.
fn ancestors(node: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    std::iter::successors(node.parent(), |current| current.parent())
}

fn inline_module_path(node: Node<'_>, source: &[u8]) -> String {
    let mut names: Vec<_> = ancestors(node)
        .filter(|node| node.kind() == "mod_item")
        .filter_map(|node| node.child_by_field_name("name"))
        .filter_map(|node| node.utf8_text(source).ok())
        .collect();
    names.reverse();
    names.join("::")
}

fn has_ancestor(node: Node<'_>, kind: &str) -> bool {
    ancestors(node).any(|parent| parent.kind() == kind)
}

fn include_reference(node: Node<'_>, text: &str) -> Option<DependencySyntax> {
    let kind = DependencyKind::Include;
    if !text.starts_with("include!") {
        return None;
    }
    let argument = text
        .strip_prefix("include!(")
        .and_then(|value| value.strip_suffix(')'))
        .map(str::trim);
    let Some(target) = argument
        .filter(|value| {
            let bytes = value.as_bytes();
            bytes.len() >= 2
                && matches!(bytes.first(), Some(b'\'' | b'\"'))
                && bytes.first() == bytes.last()
        })
        .and_then(string_value)
        .filter(|target| argument.is_some_and(|value| value.len() == target.len() + 2))
    else {
        return Some(
            unresolved(node, kind, text, "dependency target is dynamic").with_internal_intent(),
        );
    };
    if target.starts_with('/') {
        return Some(
            unresolved(
                node,
                kind,
                target,
                "absolute include paths are not repository-safe",
            )
            .with_internal_intent(),
        );
    }
    let candidate = if target.starts_with('.') {
        target.to_owned()
    } else {
        format!("./{target}")
    };
    Some(candidates(node, kind, target, vec![candidate]).with_internal_intent())
}

fn explicit_reference(
    node: Node<'_>,
    source: &[u8],
    target: &str,
    path: Result<&str, ()>,
) -> Option<DependencySyntax> {
    let kind = DependencyKind::Module;
    let logical = inline_module_path(node, source);
    let logical = if logical.is_empty() {
        target.to_owned()
    } else {
        format!("{logical}::{target}")
    };
    Some(match path {
        Ok(path) => candidates(node, kind, target, vec![path.to_owned()])
            .with_internal_intent()
            .with_relation(StaticRelationKind::ModuleOwnership)
            .with_module_path(logical),
        Err(()) => unresolved(
            node,
            kind,
            target,
            "Rust module path is not a static relative literal",
        )
        .with_internal_intent()
        .with_relation(StaticRelationKind::ModuleOwnership),
    })
}

fn reference_is_nested(node: Node<'_>, text: &str) -> bool {
    match node.kind() {
        "use_declaration" => node
            .child_by_field_name("argument")
            .is_some_and(|argument| matches!(argument.kind(), "use_list" | "scoped_use_list")),
        "scoped_identifier" => {
            node.parent()
                .is_some_and(|parent| parent.kind() == "scoped_identifier")
                || has_ancestor(node, "use_declaration")
                || !["crate::", "self::", "super::"]
                    .iter()
                    .any(|prefix| text.starts_with(prefix))
        }
        _ => false,
    }
}

fn reference_kind(node: Node<'_>) -> Option<DependencyKind> {
    Some(match node.kind() {
        "use_declaration" | "extern_crate_declaration" | "scoped_identifier" => {
            DependencyKind::Import
        }
        "mod_item" => DependencyKind::Module,
        "macro_invocation" => DependencyKind::Include,
        _ => return None,
    })
}
