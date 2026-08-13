use smackdebt_analysis::{DependencySyntax, Language as ReportLanguage, UnitKind};
use tree_sitter::{Language as Grammar, Node};

use crate::semantic::Syntax;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct Classification {
    pub(super) unit: Option<UnitKind>,
    pub(super) syntax: Syntax,
    pub(super) dependency: Option<DependencySyntax>,
}

pub(super) trait Language {
    const REPORT_LANGUAGE: ReportLanguage;

    fn grammar() -> Grammar;
    fn unit_query() -> &'static str;
    fn unit_kind(node: Node<'_>) -> Option<UnitKind>;
    fn is_container(node: Node<'_>) -> bool;
    fn syntax(node: Node<'_>, source: &[u8]) -> Syntax;

    fn dependency(_node: Node<'_>, _source: &[u8]) -> Option<DependencySyntax> {
        None
    }

    fn classify(node: Node<'_>, source: &[u8], include_dependency: bool) -> Classification {
        if !node.is_named() {
            return Classification::default();
        }
        Classification {
            unit: Self::unit_kind(node),
            syntax: Self::syntax(node, source),
            dependency: include_dependency
                .then(|| Self::dependency(node, source))
                .flatten(),
        }
    }

    fn name(node: Node<'_>, source: &[u8]) -> String {
        node_name(node, source)
    }
}

pub(super) fn node_name(node: Node<'_>, source: &[u8]) -> String {
    node.child_by_field_name("name")
        .and_then(|name| name.utf8_text(source).ok())
        .filter(|name| !name.trim().is_empty())
        .map_or_else(|| "<anonymous>".to_owned(), |name| name.trim().to_owned())
}
