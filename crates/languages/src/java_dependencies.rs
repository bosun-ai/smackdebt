//! Java package declarations and type references, independent of source layout.
use crate::language_common::{named_reference, qualified_name};
use smackdebt_analysis::{DependencySyntax, NameDeclaration, NameImport, SourceNames, SymbolKind};
use tree_sitter::Node;

fn package(node: Node<'_>, source: &[u8]) -> String {
    let root = std::iter::successors(Some(node), |node| node.parent())
        .last()
        .unwrap();
    let mut cursor = root.walk();
    let declaration = root
        .named_children(&mut cursor)
        .find(|node| node.kind() == "package_declaration");
    declaration
        .and_then(|node| {
            let mut cursor = node.walk();
            node.named_children(&mut cursor)
                .find(|child| matches!(child.kind(), "identifier" | "scoped_identifier"))
                .and_then(|name| name.utf8_text(source).ok())
        })
        .unwrap_or("")
        .to_owned()
}
fn type_declaration(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "class_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration"
            | "annotation_type_declaration"
    )
}
fn namespace(node: Node<'_>, source: &[u8]) -> String {
    let mut types = Vec::new();
    for parent in std::iter::successors(node.parent(), |node| node.parent()) {
        if type_declaration(parent)
            && let Some(name) = parent
                .child_by_field_name("name")
                .and_then(|node| node.utf8_text(source).ok())
        {
            types.push(name);
        }
    }
    let mut result = package(node, source);
    for name in types.into_iter().rev() {
        result = qualified_name(&result, name);
    }
    result
}
fn import(node: Node<'_>, source: &[u8]) -> Option<(String, bool, bool)> {
    if node.kind() != "import_declaration" {
        return None;
    }
    let mut cursor = node.walk();
    let target = node
        .named_children(&mut cursor)
        .find(|child| matches!(child.kind(), "identifier" | "scoped_identifier"))?;
    let target = qualified_text(target, source);
    let wildcard = node
        .children(&mut cursor)
        .any(|child| child.kind() == "asterisk");
    let static_import = node
        .children(&mut cursor)
        .any(|child| child.kind() == "static");
    Some((target, wildcard, static_import))
}
pub(crate) fn collect(node: Node<'_>, source: &[u8], names: &mut SourceNames) {
    if type_declaration(node)
        && let Some(name) = node
            .child_by_field_name("name")
            .and_then(|node| node.utf8_text(source).ok())
    {
        names.declare(NameDeclaration::new(
            qualified_name(&namespace(node, source), name),
            SymbolKind::Type,
            false,
        ));
    }
    if let Some((target, wildcard, false)) = import(node, source) {
        let alias = (!wildcard).then(|| target.rsplit('.').next().unwrap().to_owned());
        names.import(NameImport::new(
            target,
            alias,
            0..usize::MAX,
            false,
            SymbolKind::Type,
        ));
    }
}
pub(crate) fn reference(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
    if node.kind() == "import_declaration" {
        return import_reference(node, source);
    }
    let parent = node.parent()?;
    let name = match node.kind() {
        "type_identifier" | "scoped_type_identifier"
            if !matches!(parent.kind(), "scoped_type_identifier" | "type_parameter") =>
        {
            node.utf8_text(source).ok()?
        }
        "method_invocation" => node.child_by_field_name("object")?.utf8_text(source).ok()?,
        "field_access"
            if parent.kind() != "field_access" && parent.kind() != "method_invocation" =>
        {
            node.child_by_field_name("object")?.utf8_text(source).ok()?
        }
        _ => return None,
    };
    if matches!(node.kind(), "method_invocation" | "field_access")
        && receiver_shadowed(node, source, name.split('.').next().unwrap_or(name))
    {
        return None;
    }
    if is_generic_parameter(node, source, name) {
        return None;
    }
    Some(named_reference(
        node,
        name.to_owned(),
        namespace(node, source),
        SymbolKind::Type,
    ))
}

fn qualified_text(node: Node<'_>, source: &[u8]) -> String {
    if matches!(node.kind(), "identifier" | "type_identifier") {
        return node.utf8_text(source).unwrap_or("").to_owned();
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|child| !matches!(child.kind(), "block_comment" | "line_comment"))
        .map(|child| qualified_text(child, source))
        .collect::<Vec<_>>()
        .join(".")
}

fn receiver_shadowed(node: Node<'_>, source: &[u8], name: &str) -> bool {
    fn binds(node: Node<'_>, source: &[u8], name: &str) -> bool {
        if matches!(
            node.kind(),
            "variable_declarator"
                | "formal_parameter"
                | "spread_parameter"
                | "catch_formal_parameter"
                | "enhanced_for_statement"
        ) && node
            .child_by_field_name("name")
            .and_then(|node| node.utf8_text(source).ok())
            == Some(name)
        {
            return true;
        }
        if type_declaration(node)
            || matches!(
                node.kind(),
                "block" | "method_declaration" | "constructor_declaration" | "lambda_expression"
            )
        {
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

fn import_reference(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
    let (target, wildcard, static_import) = import(node, source)?;
    if wildcard && !static_import {
        return None;
    }
    let owner = if static_import && !wildcard {
        target.rsplit_once('.')?.0
    } else {
        &target
    };
    Some(named_reference(
        node,
        format!(".{owner}"),
        String::new(),
        SymbolKind::Type,
    ))
}

fn is_generic_parameter(node: Node<'_>, source: &[u8], name: &str) -> bool {
    // Type parameters belong to this declaration rather than another source file.
    for ancestor in std::iter::successors(node.parent(), |node| node.parent()) {
        if let Some(parameters) = ancestor.child_by_field_name("type_parameters") {
            let mut cursor = parameters.walk();
            if parameters.named_children(&mut cursor).any(|parameter| {
                let mut cursor = parameter.walk();
                parameter
                    .named_children(&mut cursor)
                    .find(|child| child.kind() == "type_identifier")
                    .and_then(|node| node.utf8_text(source).ok())
                    == Some(name)
            }) {
                return true;
            }
        }
    }
    false
}
