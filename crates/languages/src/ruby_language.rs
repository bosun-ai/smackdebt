use smackdebt_analysis::{DependencySyntax, Language as ReportLanguage, UnitKind};
use tree_sitter::{Language as Grammar, Node};

use crate::language::{Language, node_name};
use crate::language_common::{boolean_syntax, statement};
use crate::semantic::Syntax;

pub(crate) struct Ruby;

impl Language for Ruby {
    const REPORT_LANGUAGE: ReportLanguage = ReportLanguage::Ruby;

    fn grammar() -> Grammar {
        tree_sitter_ruby::LANGUAGE.into()
    }

    fn unit_query() -> &'static str {
        "[(method) (singleton_method) (lambda) (block) (do_block)] @unit"
    }

    fn generated_marker(path: &std::path::Path, source: &[u8]) -> bool {
        let rails_schema = path.file_name().is_some_and(|name| name == "schema.rb")
            && path
                .parent()
                .and_then(std::path::Path::file_name)
                .is_some_and(|name| name == "db");
        rails_schema
            || std::str::from_utf8(source).is_ok_and(|text| {
                text.lines().take(5).any(|line| {
                    let line = line.to_ascii_lowercase();
                    line.contains("generated") && line.contains("do not edit")
                })
            })
    }

    fn unit_kind(node: Node<'_>) -> Option<UnitKind> {
        match node.kind() {
            "method" | "singleton_method" => Some(UnitKind::Method),
            "lambda" => Some(UnitKind::Lambda),
            "block" | "do_block"
                if node
                    .parent()
                    .is_some_and(|parent| parent.kind() == "lambda") =>
            {
                None
            }
            "block" | "do_block" => Some(UnitKind::Closure),
            _ => None,
        }
    }

    fn is_container(node: Node<'_>) -> bool {
        matches!(node.kind(), "class" | "module")
    }

    fn name(node: Node<'_>, source: &[u8]) -> String {
        match node.kind() {
            "lambda" => format!("<lambda {}>", node.start_position().row + 1),
            "block" | "do_block" => {
                format!("<closure {}>", node.start_position().row + 1)
            }
            "singleton_method" => {
                let object = node
                    .child_by_field_name("object")
                    .and_then(|value| value.utf8_text(source).ok())
                    .unwrap_or("self");
                let name = node
                    .child_by_field_name("name")
                    .and_then(|value| value.utf8_text(source).ok())
                    .unwrap_or("<anonymous>");
                format!("{object}.{name}")
            }
            _ => node_name(node, source),
        }
    }

    fn has_declared_identity(node: Node<'_>, source: &[u8]) -> bool {
        matches!(node.kind(), "method" | "singleton_method")
            && node
                .child_by_field_name("name")
                .and_then(|name| name.utf8_text(source).ok())
                .is_some_and(|name| !name.trim().is_empty())
    }

    fn match_anchor(node: Node<'_>, source: &[u8]) -> Option<String> {
        ruby_example_anchor(node, source).or_else(|| ruby_binding_anchor(node, source))
    }

    fn syntax(node: Node<'_>, source: &[u8]) -> Syntax {
        match node.kind() {
            "if" | "unless" | "while" | "until" | "for" | "case" | "rescue" | "if_modifier"
            | "unless_modifier" | "while_modifier" | "until_modifier" => Syntax::structural(true),
            "elsif" | "when" | "else" => Syntax::alternative(node.kind() == "when"),
            "binary" => boolean_syntax(node, source).unwrap_or_default(),
            kind => statement(
                kind,
                &[
                    "assignment",
                    "call",
                    "return",
                    "break",
                    "next",
                    "yield",
                    "operator_assignment",
                ],
            ),
        }
    }

    fn dependency(node: Node<'_>, source: &[u8]) -> Option<DependencySyntax> {
        crate::dependency::ruby(node, source)
    }
}

fn ruby_example_anchor(node: Node<'_>, source: &[u8]) -> Option<String> {
    if !matches!(node.kind(), "block" | "do_block") {
        return None;
    }
    let mut descriptions = Vec::new();
    let mut current = Some(node);
    while let Some(value) = current {
        if matches!(value.kind(), "block" | "do_block")
            && let Some(call) = value.parent()
            && let Some(description) = ruby_example_call(call, source)
        {
            descriptions.push(description);
        }
        current = value.parent();
    }
    if descriptions.is_empty() {
        return None;
    }
    descriptions.reverse();
    Some(descriptions.join("/"))
}

fn ruby_example_call(call: Node<'_>, source: &[u8]) -> Option<String> {
    if call.kind() != "call" {
        return None;
    }
    let method = call
        .child_by_field_name("method")
        .and_then(|value| value.utf8_text(source).ok())?
        .trim();
    if !matches!(
        method,
        "describe" | "context" | "it" | "specify" | "example"
    ) {
        return None;
    }
    let argument = call
        .child_by_field_name("arguments")
        .and_then(|value| value.named_child(0))
        .or_else(|| {
            let mut cursor = call.walk();
            call.named_children(&mut cursor)
                .find(|child| matches!(child.kind(), "string" | "simple_symbol"))
        })?;
    let description = argument.utf8_text(source).ok()?.trim();
    (!description.is_empty()).then(|| format!("example:{method}:{description}"))
}

fn ruby_binding_anchor(node: Node<'_>, source: &[u8]) -> Option<String> {
    let parent = node.parent()?;
    if !matches!(parent.kind(), "assignment" | "operator_assignment") {
        return None;
    }
    let left = parent.child_by_field_name("left")?;
    let name = left.utf8_text(source).ok()?.trim();
    (!name.is_empty()).then(|| format!("binding:{name}"))
}
