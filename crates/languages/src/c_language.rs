use smackdebt_analysis::{Language as ReportLanguage, UnitKind};
use tree_sitter::{Language as Grammar, Node};

use crate::language::Language;
use crate::language_common::{boolean_syntax, statement};
use crate::semantic::Syntax;

pub(crate) struct C;

impl Language for C {
    const REPORT_LANGUAGE: ReportLanguage = ReportLanguage::C;

    fn grammar() -> Grammar {
        tree_sitter_c::LANGUAGE.into()
    }

    fn unit_query() -> &'static str {
        "(function_definition) @unit"
    }

    fn unit_kind(node: Node<'_>) -> Option<UnitKind> {
        (node.kind() == "function_definition").then_some(UnitKind::Function)
    }

    fn is_container(_node: Node<'_>) -> bool {
        false
    }

    fn name(node: Node<'_>, source: &[u8]) -> String {
        declarator_name(
            node.child_by_field_name("declarator").unwrap_or(node),
            source,
        )
    }

    fn syntax(node: Node<'_>, source: &[u8]) -> Syntax {
        c_syntax(node, source)
    }
}

pub(super) fn declarator_name(node: Node<'_>, source: &[u8]) -> String {
    let mut name = None;
    crate::engine::walk(node, |candidate, _| {
        if name.is_none() && matches!(candidate.kind(), "identifier" | "field_identifier") {
            name = candidate.utf8_text(source).ok().map(str::to_owned);
        }
        name.is_none()
    });
    name.unwrap_or_else(|| "<anonymous>".to_owned())
}

pub(super) fn c_syntax(node: Node<'_>, source: &[u8]) -> Syntax {
    match node.kind() {
        "if_statement"
        | "for_statement"
        | "while_statement"
        | "do_statement"
        | "conditional_expression" => Syntax::structural(true),
        "switch_statement" => Syntax::structural(false),
        "case_statement" => Syntax::alternative(true),
        "binary_expression" => boolean_syntax(node, source).unwrap_or_default(),
        kind => statement(
            kind,
            &[
                "expression_statement",
                "declaration",
                "return_statement",
                "break_statement",
                "continue_statement",
                "goto_statement",
            ],
        ),
    }
}
