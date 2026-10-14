//! C# declarations, import scopes, and type references.
use super::CSharp;
use crate::language::Language;
use crate::language_common::{named_reference, qualified_name as qualified};
use smackdebt_analysis::{DependencySyntax, NameDeclaration, NameImport, SourceNames, SymbolKind};
use tree_sitter::Node;

pub(super) fn namespace(node: Node<'_>, source: &[u8], include_types: bool) -> String {
    let mut segments = Vec::new();
    let mut current = node.parent();
    while let Some(parent) = current {
        if (parent.kind() == "namespace_declaration"
            || include_types && CSharp::is_container(parent))
            && let Some(name) = parent
                .child_by_field_name("name")
                .and_then(|name| name.utf8_text(source).ok())
        {
            segments.push(name.to_owned());
        }
        if parent.kind() == "compilation_unit" {
            let mut cursor = parent.walk();
            if let Some(declaration) = parent
                .named_children(&mut cursor)
                .find(|node| node.kind() == "file_scoped_namespace_declaration")
                && let Some(name) = declaration
                    .child_by_field_name("name")
                    .and_then(|name| name.utf8_text(source).ok())
            {
                segments.push(name.to_owned());
            }
        }
        current = parent.parent();
    }
    segments.reverse();
    segments.join(".")
}

fn import_scope(node: Node<'_>) -> std::ops::Range<usize> {
    std::iter::successors(node.parent(), |node| node.parent())
        .find(|node| matches!(node.kind(), "namespace_declaration" | "compilation_unit"))
        .map_or(0..usize::MAX, |node| node.byte_range())
}
fn import_target(node: Node<'_>, source: &[u8]) -> Option<(String, Option<String>, bool, bool)> {
    if node.kind() != "using_directive" {
        return None;
    }
    let alias_node = node.child_by_field_name("name");
    let alias = alias_node
        .and_then(|node| node.utf8_text(source).ok())
        .map(str::to_owned);
    let mut cursor = node.walk();
    let target = node
        .named_children(&mut cursor)
        .find(|child| {
            Some(*child) != alias_node
                && matches!(
                    child.kind(),
                    "identifier" | "qualified_name" | "alias_qualified_name" | "generic_name"
                )
        })?
        .utf8_text(source)
        .ok()?;
    let global = node
        .children(&mut cursor)
        .any(|child| child.kind() == "global");
    let static_import = node
        .children(&mut cursor)
        .any(|child| child.kind() == "static");
    Some((
        target.trim_start_matches("global::").to_owned(),
        alias,
        global,
        static_import,
    ))
}
pub(super) fn collect_names(node: Node<'_>, source: &[u8], names: &mut SourceNames) {
    if (CSharp::is_container(node)
        || matches!(node.kind(), "enum_declaration" | "delegate_declaration"))
        && let Some(name) = node
            .child_by_field_name("name")
            .and_then(|name| name.utf8_text(source).ok())
    {
        let mut cursor = node.walk();
        let partial = node.named_children(&mut cursor).any(|child| {
            child.kind() == "modifier" && child.utf8_text(source).ok() == Some("partial")
        });
        names.declare(NameDeclaration::new(
            qualified(&namespace(node, source, true), name),
            SymbolKind::Type,
            partial,
        ));
    }
    if let Some((target, alias, global, _)) = import_target(node, source) {
        names.import(NameImport::new(
            target,
            alias,
            import_scope(node),
            global,
            SymbolKind::Type,
        ));
    }
}
pub(super) fn named_dependency(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
    if let Some((target, _, _, true)) = import_target(node, source) {
        return Some(name_reference(node, &format!(".{target}"), ""));
    }
    if node.kind() == "member_access_expression" {
        return receiver_dependency(node, source);
    }
    if !matches!(
        node.kind(),
        "identifier" | "qualified_name" | "alias_qualified_name" | "generic_name"
    ) {
        return None;
    }
    let parent = node.parent()?;
    if !is_type_reference(node, parent) {
        return None;
    }
    let raw = node.utf8_text(source).ok()?;
    let name = type_name(raw);
    if is_generic_parameter(node, source, &name) {
        return None;
    }

    Some(name_reference(node, &name, &namespace(node, source, true)))
}
fn name_reference(node: Node<'_>, name: &str, namespace: &str) -> DependencySyntax {
    let name = name
        .strip_prefix("global::")
        .map_or_else(|| name.replace("::", "."), |name| format!(".{name}"));
    named_reference(node, name, namespace.to_owned(), SymbolKind::Type)
}

fn is_generic_parameter(node: Node<'_>, source: &[u8], name: &str) -> bool {
    // Generic parameters belong to the containing declaration, not another file.
    let mut ancestor = node.parent();
    while let Some(parent) = ancestor {
        if let Some(parameters) = parent.child_by_field_name("type_parameters") {
            let mut cursor = parameters.walk();
            if parameters.named_children(&mut cursor).any(|parameter| {
                parameter
                    .child_by_field_name("name")
                    .and_then(|name| name.utf8_text(source).ok())
                    == Some(name)
            }) {
                return true;
            }
        }
        ancestor = parent.parent();
    }
    false
}

fn type_name(raw: &str) -> std::borrow::Cow<'_, str> {
    if !raw.contains('<') {
        return raw.into();
    }
    let mut name = String::with_capacity(raw.len());
    let mut depth = 0usize;
    for character in raw.chars() {
        match character {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            _ if depth == 0 => name.push(character),
            _ => {}
        }
    }
    name.into()
}

// Inspect only enclosing lexical scopes; sibling methods and inner blocks do not bind here.
fn receiver_shadowed(node: Node<'_>, source: &[u8], name: &str) -> bool {
    fn binds(node: Node<'_>, source: &[u8], name: &str) -> bool {
        if matches!(
            node.kind(),
            "variable_declarator"
                | "parameter"
                | "implicit_parameter"
                | "foreach_statement"
                | "catch_declaration"
        ) && node
            .child_by_field_name("name")
            .and_then(|node| node.utf8_text(source).ok())
            == Some(name)
        {
            return true;
        }
        if matches!(
            node.kind(),
            "block"
                | "class_declaration"
                | "method_declaration"
                | "lambda_expression"
                | "local_function_statement"
        ) {
            return false;
        }
        let mut cursor = node.walk();
        node.named_children(&mut cursor)
            .any(|child| binds(child, source, name))
    }
    for scope in std::iter::successors(node.parent(), |node| node.parent()) {
        let mut cursor = scope.walk();
        if scope
            .named_children(&mut cursor)
            .any(|child| binds(child, source, name))
        {
            return true;
        }
    }
    false
}

fn receiver_dependency(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
    let receiver = node.child_by_field_name("expression")?;
    if !matches!(
        receiver.kind(),
        "identifier" | "member_access_expression" | "alias_qualified_name" | "generic_name"
    ) {
        return None;
    }
    let name = type_name(receiver.utf8_text(source).ok()?);
    let first = name.split('.').next()?;
    if receiver_shadowed(node, source, first) || is_generic_parameter(node, source, first) {
        return None;
    }
    Some(name_reference(
        receiver,
        &name,
        &namespace(node, source, true),
    ))
}

fn is_type_reference(node: Node<'_>, parent: Node<'_>) -> bool {
    if parent.kind() == "using_directive" {
        return false;
    }
    parent.child_by_field_name("type") == Some(node)
        || parent.child_by_field_name("returns") == Some(node)
        || matches!(parent.kind(), "base_list" | "type_argument_list")
        || parent.kind() == "attribute" && parent.child_by_field_name("name") == Some(node)
}
