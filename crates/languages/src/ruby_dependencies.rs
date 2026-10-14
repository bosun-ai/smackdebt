use crate::dependency::{candidates, unresolved};
use smackdebt_analysis::{DependencyKind, DependencySyntax};
use tree_sitter::Node;

pub(super) fn ruby(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
    if matches!(
        node.kind(),
        "assignment" | "operator_assignment" | "call" | "binary"
    ) {
        let text = node.utf8_text(source).ok()?.trim();
        if text.starts_with("$LOAD_PATH") || text.starts_with("$:") {
            return Some(
                unresolved(
                    node,
                    DependencyKind::Require,
                    text,
                    "Ruby load-path changes cannot be resolved statically",
                )
                .with_internal_intent(),
            );
        }
    }
    if node.kind() != "call" || node.child_by_field_name("receiver").is_some() {
        return None;
    }
    let method = node.child_by_field_name("method")?.utf8_text(source).ok()?;
    if !matches!(method, "require" | "require_relative") {
        return None;
    }
    let text = node.utf8_text(source).ok()?;
    let argument = node.child_by_field_name("arguments")?;
    let value = argument.named_child(0)?;
    let literal = value.utf8_text(source).ok()?;
    let Some(target) = literal
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| {
            literal
                .strip_prefix('\'')
                .and_then(|v| v.strip_suffix('\''))
        })
        .filter(|_| value.kind() == "string" && argument.named_child_count() == 1)
        .filter(|v| !v.contains("#{") && !v.contains('\\') && !v.contains('\n'))
    else {
        return Some(
            unresolved(
                node,
                DependencyKind::Require,
                text,
                "dependency target is dynamic",
            )
            .with_internal_intent(),
        );
    };
    let relative = method == "require_relative";
    let path = if relative {
        format!("./{target}")
    } else {
        target.to_owned()
    };
    let paths = if path.ends_with(".rb") {
        vec![path]
    } else {
        vec![format!("{path}.rb")]
    };
    let reference = candidates(node, DependencyKind::Require, target, paths);
    Some(if relative {
        reference.with_internal_intent()
    } else {
        reference
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use smackdebt_analysis::DependencySyntaxState;
    fn extract(source: &str) -> Vec<DependencySyntax> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_ruby::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();
        let mut cursor = root.walk();
        root.named_children(&mut cursor)
            .filter_map(|node| ruby(node, source.as_bytes()))
            .collect()
    }
    #[test]
    fn ordinary_requires_are_candidates_for_gem_load_paths() {
        assert!(
            matches!(extract("require 'widget'")[0].state(), DependencySyntaxState::Candidates(values) if values == &["widget.rb"])
        );
    }
    #[test]
    fn computed_requires_and_load_path_changes_are_diagnostics() {
        for source in [
            "require name",
            r##"require "#{name}""##,
            "$LOAD_PATH.unshift('local')",
            "$: << 'local'",
        ] {
            assert!(
                matches!(
                    extract(source)[0].state(),
                    DependencySyntaxState::Unresolved(_)
                ),
                "{source}"
            );
        }
    }
}
