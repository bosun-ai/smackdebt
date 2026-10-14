use smackdebt_analysis::{DependencyKind, DependencySyntax, DependencySyntaxState};
use tree_sitter::Node;

/// Each imported module is independent, even when one statement lists several.
pub(super) fn collect(node: Node<'_>, source: &[u8], output: &mut Vec<DependencySyntax>) {
    if !matches!(node.kind(), "import_statement" | "import_from_statement") {
        return;
    }
    let mut cursor = node.walk();
    let names: Vec<_> = node
        .children_by_field_name("name", &mut cursor)
        .filter_map(|name| {
            let name = if name.kind() == "aliased_import" {
                name.child_by_field_name("name")?
            } else {
                name
            };
            name.utf8_text(source).ok().map(str::to_owned)
        })
        .collect();
    if node.kind() == "import_statement" {
        for name in names {
            output.push(reference(node, &name, Vec::new()));
        }
    } else if let Some(module) = node
        .child_by_field_name("module_name")
        .and_then(|module| module.utf8_text(source).ok())
    {
        if names.is_empty() {
            output.push(reference(node, module, Vec::new()));
        }
        for name in names {
            output.push(reference(node, module, vec![name]));
        }
    }
}

fn reference(node: Node<'_>, module: &str, names: Vec<String>) -> DependencySyntax {
    let module: String = module
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '\\')
        .collect();
    let names = names
        .into_iter()
        .map(|name| {
            name.chars()
                .filter(|character| !character.is_whitespace() && *character != '\\')
                .collect()
        })
        .collect();
    let reference = DependencySyntax::new(
        DependencyKind::Import,
        &module,
        crate::dependency::span(node),
        DependencySyntaxState::PythonImport { names },
    );
    if module.starts_with('.') {
        reference.with_internal_intent()
    } else {
        reference
    }
}
