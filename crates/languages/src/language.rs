use smackdebt_analysis::{DependencySyntax, Language as ReportLanguage, UnitKind};
use std::path::Path;

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
    fn generated_marker(path: &Path, source: &[u8]) -> bool;

    fn collect_names(
        _node: Node<'_>,
        _source: &[u8],
        _names: &mut smackdebt_analysis::SourceNames,
    ) {
    }

    fn container(_node: Node<'_>, _source: &[u8]) -> Option<String> {
        None
    }

    fn dependency(_node: Node<'_>, _source: &[u8]) -> Option<DependencySyntax> {
        None
    }

    /// The number of parameters the rated unit at `node` declares.
    ///
    /// The shared default reads the declared parameter list, so a language
    /// implements this only where its grammar names parameters differently. A
    /// unit that cannot declare parameters reports zero.
    fn parameter_count(node: Node<'_>, _source: &[u8]) -> u32 {
        crate::language_common::declared_parameter_count(node)
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

    fn has_declared_identity(node: Node<'_>, source: &[u8]) -> bool {
        node.child_by_field_name("name")
            .and_then(|name| name.utf8_text(source).ok())
            .is_some_and(|name| !name.trim().is_empty())
    }

    fn match_anchor(_node: Node<'_>, _source: &[u8]) -> Option<String> {
        None
    }
}

pub(super) fn node_name(node: Node<'_>, source: &[u8]) -> String {
    node.child_by_field_name("name")
        .and_then(|name| name.utf8_text(source).ok())
        .filter(|name| !name.trim().is_empty())
        .map_or_else(|| "<anonymous>".to_owned(), |name| name.trim().to_owned())
}
