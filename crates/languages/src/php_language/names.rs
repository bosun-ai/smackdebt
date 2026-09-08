//! PHP declarations, namespaces, aliases, and name references.
use super::Php;
use crate::language::Language;
use crate::language_common::{named_reference, qualified_name as qualified};
use smackdebt_analysis::{DependencySyntax, NameDeclaration, NameImport, SourceNames, SymbolKind};
use tree_sitter::Node;
fn normalized(name: &str, kind: SymbolKind) -> String {
    let name = name.replace('\\', ".");
    if kind == SymbolKind::Constant {
        match name.rsplit_once('.') {
            Some((namespace, constant)) => format!("{}.{constant}", namespace.to_ascii_lowercase()),
            None => name,
        }
    } else {
        name.to_ascii_lowercase()
    }
}
pub(super) fn namespace(node: Node<'_>, source: &[u8]) -> String {
    std::iter::successors(Some(node), |node| node.parent())
        .find_map(|item| {
            let declaration = if item.kind() == "namespace_definition" {
                Some(item)
            } else {
                std::iter::successors(item.prev_named_sibling(), |node| node.prev_named_sibling())
                    .find(|node| {
                        node.kind() == "namespace_definition"
                            && node.child_by_field_name("body").is_none()
                    })
            }?;
            Some(
                declaration
                    .child_by_field_name("name")
                    .and_then(|name| name.utf8_text(source).ok())
                    .unwrap_or_default()
                    .replace('\\', "."),
            )
        })
        .unwrap_or_default()
}
fn import_extent(node: Node<'_>) -> std::ops::Range<usize> {
    let owner = std::iter::successors(node.parent(), |node| node.parent())
        .find(|node| matches!(node.kind(), "namespace_definition" | "program"));
    match owner {
        Some(owner) if owner.kind() == "program" => namespace_extent(owner, node.start_byte()),
        Some(owner) => owner.byte_range(),
        None => node.byte_range(),
    }
}
fn use_target(node: Node<'_>, source: &[u8]) -> Option<NameImport> {
    if node.kind() != "namespace_use_clause" {
        return None;
    }
    let mut declaration = node.parent()?;
    if declaration.kind() == "namespace_use_group" {
        declaration = declaration.parent()?;
    }
    let kind = match node
        .child_by_field_name("type")
        .or_else(|| declaration.child_by_field_name("type"))
        .and_then(|node| node.utf8_text(source).ok())
    {
        Some("function") => SymbolKind::Function,
        Some("const") => SymbolKind::Constant,
        _ => SymbolKind::Type,
    };
    let target_node = node.named_child(0)?;
    let mut target = target_node
        .utf8_text(source)
        .ok()?
        .trim_start_matches('\\')
        .to_owned();
    if node
        .parent()
        .is_some_and(|parent| parent.kind() == "namespace_use_group")
    {
        let prefix = declaration
            .named_child(0)?
            .utf8_text(source)
            .ok()?
            .trim_end_matches('\\');
        target = format!("{prefix}\\{target}");
    }
    let alias = node
        .child_by_field_name("alias")
        .and_then(|node| node.utf8_text(source).ok())
        .or_else(|| target.rsplit('\\').next());
    Some(NameImport::new(
        normalized(&target, kind),
        alias.map(|alias| normalized(alias, kind)),
        import_extent(node),
        false,
        kind,
    ))
}
pub(super) fn collect(node: Node<'_>, source: &[u8], names: &mut SourceNames) {
    let kind = match node.kind() {
        "class_declaration"
        | "interface_declaration"
        | "trait_declaration"
        | "enum_declaration" => Some(SymbolKind::Type),
        "function_definition" => Some(SymbolKind::Function),
        "const_element"
            if !std::iter::successors(node.parent(), |node| node.parent())
                .any(Php::is_container) =>
        {
            Some(SymbolKind::Constant)
        }
        _ => None,
    };
    if let Some(kind) = kind
        && let Some(name) = node
            .child_by_field_name("name")
            .or_else(|| {
                (node.kind() == "const_element")
                    .then(|| node.named_child(0))
                    .flatten()
            })
            .and_then(|name| name.utf8_text(source).ok())
    {
        names.declare(NameDeclaration::new(
            normalized(&qualified(&namespace(node, source), name), kind),
            kind,
            false,
        ));
    }
    if let Some(import) = use_target(node, source) {
        names.import(import);
    }
}
pub(super) fn dependency(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
    if let Some(import) = use_target(node, source) {
        return Some(named_reference(
            node,
            format!(".{}", import.target()),
            String::new(),
            import.kind(),
        ));
    }
    if !matches!(node.kind(), "name" | "qualified_name" | "relative_name") {
        return None;
    }
    let parent = node.parent()?;
    let kind = match parent.kind() {
        "object_creation_expression"
        | "base_clause"
        | "class_interface_clause"
        | "named_type"
        | "use_declaration"
        | "attribute" => SymbolKind::Type,
        "scoped_call_expression" | "scoped_property_access_expression"
            if parent.child_by_field_name("scope") == Some(node) =>
        {
            SymbolKind::Type
        }
        "class_constant_access_expression" if parent.named_child(0) == Some(node) => {
            SymbolKind::Type
        }
        "function_call_expression" if parent.child_by_field_name("function") == Some(node) => {
            SymbolKind::Function
        }
        _ => return None,
    };
    let text = node.utf8_text(source).ok()?;
    if matches!(text, "self" | "parent" | "static") {
        return None;
    }
    let namespace = namespace(node, source);
    if let Some(tail) = text.strip_prefix("namespace\\") {
        return Some(name_reference(
            node,
            &format!(".{}", qualified(&namespace, tail)),
            "",
            kind,
        ));
    }
    Some(name_reference(node, text, &namespace, kind))
}
fn name_reference(
    node: Node<'_>,
    name: &str,
    namespace: &str,
    kind: SymbolKind,
) -> DependencySyntax {
    let name = normalized(name, kind);
    let namespace = normalized(namespace, kind);
    named_reference(node, name, namespace, kind)
}

fn namespace_extent(parent: Node<'_>, position: usize) -> std::ops::Range<usize> {
    let mut start = 0;
    let mut end = parent.end_byte();
    let mut cursor = parent.walk();
    for sibling in parent
        .named_children(&mut cursor)
        .filter(|node| node.kind() == "namespace_definition")
    {
        if sibling.start_byte() < position {
            start = sibling.start_byte();
        } else {
            end = sibling.start_byte();
            break;
        }
    }
    start..end
}
