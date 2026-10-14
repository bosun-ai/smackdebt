use super::named_child;
use tree_sitter::Node;

pub(super) fn external_script(
    node: Node<'_>,
    source: &[u8],
) -> Option<smackdebt_analysis::DependencySyntax> {
    let tag = named_child(node, "start_tag")?;
    let mut cursor = tag.walk();
    for attribute in tag.named_children(&mut cursor) {
        let text = attribute.utf8_text(source).ok()?;
        let Some((name, value)) = text.split_once('=') else {
            continue;
        };
        if !matches!(name.trim(), "src" | ":src" | "v-bind:src") {
            continue;
        }
        if name.trim() != "src" {
            return Some(
                crate::dependency::unresolved(
                    attribute,
                    smackdebt_analysis::DependencyKind::Import,
                    text,
                    "script target is dynamic",
                )
                .with_internal_intent(),
            );
        }
        let value = value.trim();
        let target = value
            .strip_prefix('"')
            .and_then(|text| text.strip_suffix('"'))
            .or_else(|| {
                value
                    .strip_prefix('\'')
                    .and_then(|text| text.strip_suffix('\''))
            })
            .unwrap_or(value);
        if target.contains('&') {
            return Some(
                crate::dependency::unresolved(
                    attribute,
                    smackdebt_analysis::DependencyKind::Import,
                    text,
                    "encoded script target is not supported",
                )
                .with_internal_intent(),
            );
        }
        let dependency = crate::dependency::candidates(
            attribute,
            smackdebt_analysis::DependencyKind::Import,
            target,
            crate::dependency::path_candidates(target, &[".js", ".ts", ".jsx", ".tsx"]),
        );
        return Some(if target.starts_with('.') {
            dependency.with_internal_intent()
        } else {
            dependency
        });
    }
    None
}
