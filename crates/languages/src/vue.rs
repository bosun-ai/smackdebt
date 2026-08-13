use smackdebt_analysis::{
    FileAnalysis, Language, LocalUnitId, ParseStatus, SourceSpan, UnitFact, UnitIdentity, UnitKind,
};
use tree_sitter::{Node, Parser};

use crate::engine::{self, MetricState, Scratch};
use crate::javascript_language::{JavaScript, TypeScript};
use crate::language::Language as LanguageContract;
use crate::semantic::Syntax;

struct Vue;

impl LanguageContract for Vue {
    const REPORT_LANGUAGE: Language = Language::Vue;

    fn grammar() -> tree_sitter::Language {
        tree_sitter_vue_updated::language()
    }

    fn unit_query() -> &'static str {
        "(template_element) @unit"
    }

    fn unit_kind(node: Node<'_>) -> Option<UnitKind> {
        (node.kind() == "template_element").then_some(UnitKind::Template)
    }

    fn is_container(_node: Node<'_>) -> bool {
        false
    }

    fn name(_node: Node<'_>, _source: &[u8]) -> String {
        "<template>".to_owned()
    }

    fn syntax(node: Node<'_>, source: &[u8]) -> Syntax {
        let text = node.utf8_text(source).unwrap_or_default();
        match node.kind() {
            "directive_attribute" if text.starts_with("v-else-if") => Syntax {
                logical_statement: true,
                ..Syntax::alternative(true)
            },
            "directive_attribute" if text.starts_with("v-if") || text.starts_with("v-for") => {
                Syntax {
                    logical_statement: true,
                    ..Syntax::structural(true)
                }
            }
            "interpolation" => Syntax::statement(),
            _ => Syntax::default(),
        }
    }
}

pub(super) fn analyze_vue(
    document_parser: &mut Parser,
    javascript_parser: &mut Parser,
    typescript_parser: &mut Parser,
    source: &[u8],
    scratch: &mut Scratch,
) -> Result<FileAnalysis, String> {
    document_parser
        .set_language(&Vue::grammar())
        .map_err(|error| format!("Vue language setup failed: {error}"))?;
    let started = std::time::Instant::now();
    let tree = document_parser
        .parse(source, None)
        .ok_or_else(|| "tree-sitter returned no Vue syntax tree".to_owned())?;
    crate::analyzer::record_parser_time(started.elapsed());
    engine::reserve_unit_capacity::<Vue>(tree.root_node(), source, scratch)?;
    let mut status = if tree.root_node().has_error() {
        ParseStatus::Recovered
    } else {
        ParseStatus::Parsed
    };
    let mut units = Vec::new();
    let mut dependencies = Vec::new();
    let mut template_added = false;
    let typescript_document = std::str::from_utf8(source)
        .is_ok_and(|text| text.contains("lang=\"ts") || text.contains("lang='ts"));
    let mut error = None;
    engine::walk(tree.root_node(), |node, _| {
        if error.is_some() {
            return false;
        }
        match node.kind() {
            "script_element" => {
                if let Some(raw) = named_child(node, "raw_text") {
                    let start = raw.start_byte();
                    let end = raw.end_byte();
                    let is_typescript = node
                        .utf8_text(source)
                        .is_ok_and(|text| text.contains("lang=\"ts") || text.contains("lang='ts"));
                    let result = if is_typescript {
                        engine::analyze_included::<TypeScript>(
                            typescript_parser,
                            &source[start..end],
                            raw.start_position().row as u32,
                            units.len(),
                            scratch,
                        )
                    } else {
                        engine::analyze_included::<JavaScript>(
                            javascript_parser,
                            &source[start..end],
                            raw.start_position().row as u32,
                            units.len(),
                            scratch,
                        )
                    };
                    let result = match result {
                        Ok(result) => result,
                        Err(message) => {
                            error = Some(message);
                            return false;
                        }
                    };
                    if result.0 == ParseStatus::Recovered {
                        status = ParseStatus::Recovered;
                    }
                    units.extend(result.1);
                    dependencies.extend(result.2);
                }
                false
            }
            "template_element" if !template_added => {
                let index = units.len();
                let measurements = match template_measurements(
                    node,
                    source,
                    javascript_parser,
                    typescript_parser,
                    typescript_document,
                    scratch,
                ) {
                    Ok(measurements) => measurements,
                    Err(message) => {
                        error = Some(message);
                        return false;
                    }
                };
                units.push(UnitFact::new(
                    LocalUnitId::from_index(index),
                    UnitIdentity::new("<template>", UnitKind::Template),
                    SourceSpan::new(
                        node.start_position().row as u32 + 1,
                        node.end_position().row as u32 + 1,
                    ),
                    measurements,
                    None,
                ));
                template_added = true;
                false
            }
            _ => true,
        }
    });
    if let Some(message) = error {
        return Err(message);
    }
    Ok(FileAnalysis::with_dependencies(
        Language::Vue,
        line_count(source),
        status,
        units,
        dependencies,
    ))
}

fn named_child<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| child.kind() == kind)
}

fn template_measurements(
    root: Node<'_>,
    source: &[u8],
    javascript_parser: &mut Parser,
    typescript_parser: &mut Parser,
    typescript_document: bool,
    scratch: &mut Scratch,
) -> Result<smackdebt_analysis::Measurements, String> {
    scratch.observations.clear();
    scratch.nesting_by_depth.clear();
    let mut expression_error = None;
    engine::walk(root, |node, depth| {
        if expression_error.is_some() {
            return false;
        }
        scratch.nesting_by_depth.truncate(depth);
        let nesting = scratch.nesting_by_depth.last().copied().unwrap_or(0);
        let syntax = Vue::syntax(node, source);
        scratch
            .observations
            .push(engine::Observation { syntax, nesting });
        scratch
            .nesting_by_depth
            .push(nesting + u32::from(syntax.nests));
        if let Some(expression) = template_expression(node, source) {
            let result = if typescript_document {
                engine::append_expression::<TypeScript>(
                    typescript_parser,
                    expression,
                    nesting + u32::from(syntax.nests),
                    scratch,
                )
            } else {
                engine::append_expression::<JavaScript>(
                    javascript_parser,
                    expression,
                    nesting + u32::from(syntax.nests),
                    scratch,
                )
            };
            if let Err(message) = result {
                expression_error = Some(message);
                return false;
            }
        }
        true
    });
    let mut metrics = MetricState::default();
    for observation in scratch.observations.iter().copied() {
        metrics.observe(observation.syntax, observation.nesting);
    }
    if let Some(message) = expression_error {
        Err(message)
    } else {
        Ok(metrics.finish())
    }
}

fn template_expression<'a>(node: Node<'_>, source: &'a [u8]) -> Option<&'a [u8]> {
    match node.kind() {
        "directive_attribute" => {
            let text = node.utf8_text(source).ok()?;
            if !(text.starts_with("v-if")
                || text.starts_with("v-else-if")
                || text.starts_with("v-for"))
            {
                return None;
            }
            let value = descendant(node, "attribute_value")?;
            Some(&source[value.start_byte()..value.end_byte()])
        }
        "interpolation" => {
            let bytes = &source[node.start_byte()..node.end_byte()];
            let start = bytes.iter().position(|byte| *byte == b'{')? + 2;
            let end = bytes
                .iter()
                .rposition(|byte| *byte == b'}')?
                .saturating_sub(1);
            (start <= end).then_some(&bytes[start..end])
        }
        _ => None,
    }
}

fn descendant<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut found = None;
    engine::walk(node, |candidate, _| {
        if candidate.kind() == kind {
            found = Some(candidate);
            false
        } else {
            found.is_none()
        }
    });
    found
}

fn line_count(source: &[u8]) -> u32 {
    if source.is_empty() {
        0
    } else {
        source.iter().filter(|&&byte| byte == b'\n').count() as u32
            + u32::from(source.last() != Some(&b'\n'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_template_and_style_keep_document_coverage_and_positions() {
        let source = b"<template>\n<div v-if=\"ready && valid\" @click=\"save\">{{ ready ? 'yes' : 'no' }}</div>\n</template>\n<script setup lang=\"ts\">\nfunction save(value: boolean) { if (value) return 1; return 0 }\n</script>\n<style>.x { color: red }</style>\n";
        let result = analyze_vue(
            &mut Parser::new(),
            &mut Parser::new(),
            &mut Parser::new(),
            source,
            &mut Scratch::default(),
        )
        .unwrap();
        assert_eq!(result.source_lines(), 7);
        assert!(result.units().iter().any(|unit| {
            unit.identity().kind() == UnitKind::Template
                && unit.measurements().cyclomatic_complexity() > 1
        }));
        assert!(
            result
                .units()
                .iter()
                .any(|unit| { unit.identity().name() == "save" && unit.span().start_line() == 5 })
        );
    }

    #[test]
    fn plain_event_reference_does_not_create_a_branch() {
        let source = b"<template><button @click=\"save\">go</button></template>";
        let result = analyze_vue(
            &mut Parser::new(),
            &mut Parser::new(),
            &mut Parser::new(),
            source,
            &mut Scratch::default(),
        )
        .unwrap();
        assert_eq!(result.units()[0].measurements().cyclomatic_complexity(), 1);
    }
}
