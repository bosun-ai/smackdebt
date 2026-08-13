use smackdebt_analysis::{Language as ReportLanguage, UnitKind};
use tree_sitter::{Language as Grammar, Node};

use crate::language::{Language, node_name};
use crate::language_common::{boolean_syntax, statement};
use crate::semantic::Syntax;

pub(super) trait JavaScriptFamily {
    const REPORT_LANGUAGE: ReportLanguage;
    fn grammar() -> Grammar;
}

macro_rules! javascript_language {
    ($name:ident, $report:expr, $grammar:expr) => {
        pub(crate) struct $name;

        impl JavaScriptFamily for $name {
            const REPORT_LANGUAGE: ReportLanguage = $report;
            fn grammar() -> Grammar {
                $grammar
            }

        }

        impl Language for $name {
            const REPORT_LANGUAGE: ReportLanguage = <$name as JavaScriptFamily>::REPORT_LANGUAGE;

            fn grammar() -> Grammar {
                <$name as JavaScriptFamily>::grammar()
            }

            fn unit_query() -> &'static str {
                "[(function_declaration) (generator_function_declaration) (function_expression) (generator_function) (method_definition) (arrow_function)] @unit"
            }

            fn unit_kind(node: Node<'_>) -> Option<UnitKind> {
                javascript_unit(node)
            }

            fn is_container(node: Node<'_>) -> bool {
                matches!(node.kind(), "class_declaration" | "class")
            }

            fn name(node: Node<'_>, source: &[u8]) -> String {
                javascript_name::<Self>(node, source)
            }

            fn syntax(node: Node<'_>, source: &[u8]) -> Syntax {
                javascript_syntax(node, source)
            }
        }
    };
}

javascript_language!(
    JavaScript,
    ReportLanguage::JavaScript,
    tree_sitter_javascript::LANGUAGE.into()
);
javascript_language!(
    Jsx,
    ReportLanguage::Jsx,
    tree_sitter_javascript::LANGUAGE.into()
);
javascript_language!(
    TypeScript,
    ReportLanguage::TypeScript,
    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
);
javascript_language!(
    Tsx,
    ReportLanguage::Tsx,
    tree_sitter_typescript::LANGUAGE_TSX.into()
);

fn javascript_unit(node: Node<'_>) -> Option<UnitKind> {
    match node.kind() {
        "function_declaration"
        | "generator_function_declaration"
        | "function_expression"
        | "generator_function" => Some(UnitKind::Function),
        "method_definition" => Some(UnitKind::Method),
        "arrow_function" => Some(UnitKind::Closure),
        _ => None,
    }
}

fn javascript_name<L: Language>(node: Node<'_>, source: &[u8]) -> String {
    if node.kind() == "arrow_function" {
        node.parent()
            .and_then(|parent| {
                parent
                    .child_by_field_name("name")
                    .or_else(|| parent.child_by_field_name("property"))
            })
            .and_then(|name| name.utf8_text(source).ok())
            .map_or_else(
                || format!("<closure {}>", node.start_position().row + 1),
                str::to_owned,
            )
    } else {
        let _ = std::marker::PhantomData::<L>;
        node_name(node, source)
    }
}

fn javascript_syntax(node: Node<'_>, source: &[u8]) -> Syntax {
    match node.kind() {
        "if_statement" if is_else_if(node) => Syntax::alternative(true),
        "if_statement" => Syntax::structural(true),
        "switch_statement" | "for_statement" | "for_in_statement" | "while_statement"
        | "do_statement" | "catch_clause" | "ternary_expression" => Syntax::structural(true),
        "switch_case" => Syntax::alternative(true),
        "else_clause"
            if node
                .named_child(0)
                .is_some_and(|child| child.kind() == "if_statement") =>
        {
            Syntax::default()
        }
        "else_clause" => Syntax::alternative(false),
        "binary_expression" => boolean_syntax(node, source).unwrap_or_default(),
        "jsx_expression" => boolean_syntax(node, source).unwrap_or_default(),
        kind => statement(
            kind,
            &[
                "expression_statement",
                "lexical_declaration",
                "variable_declaration",
                "return_statement",
                "throw_statement",
                "break_statement",
                "continue_statement",
                "import_statement",
                "export_statement",
            ],
        ),
    }
}

fn is_else_if(node: Node<'_>) -> bool {
    node.parent()
        .is_some_and(|parent| parent.kind() == "else_clause")
}
