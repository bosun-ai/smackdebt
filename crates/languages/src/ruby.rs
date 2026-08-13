use smackdebt_analysis::{Language, ParseStatus};

use crate::analyzer::{CodeUnit, Measurements, RawFileAnalysis, Span, UnitKind, line_count};

pub(super) fn analyze_ruby(
    source: &[u8],
    methods: &mut Vec<RubyBlock>,
    lambdas: &mut Vec<RubyLambda>,
) -> RawFileAnalysis {
    let text = String::from_utf8_lossy(source);
    let lines: Vec<&str> = text.lines().collect();
    let mut units: Vec<CodeUnit> = Vec::new();
    let mut containers: Vec<(String, usize)> = Vec::new();
    methods.clear();
    lambdas.clear();
    let mut block_depth = 0usize;
    let mut index = 0usize;
    while index < lines.len() {
        let line = lines[index].trim();
        if let Some(name) = ruby_container_start(line) {
            containers.push((name, block_depth));
        }
        if let Some(name) = ruby_method_start(line) {
            let start_depth = block_depth;
            let end = ruby_block_end(&lines, index, start_depth);
            for lambda_line in index..=end {
                let lambda_count = lines[lambda_line].matches("->").count()
                    + lines[lambda_line].matches("lambda").count();
                for ordinal in 0..lambda_count {
                    lambdas.push(RubyLambda {
                        name: format!("<lambda {}>", ordinal + 1),
                        start: lambda_line,
                        end: lambda_end(&lines, lambda_line),
                        method_start: index,
                    });
                }
            }
            methods.push(RubyBlock {
                name,
                start: index,
                end,
                container: containers.last().map(|container| container.0.clone()),
            });
            index = end;
            block_depth = start_depth;
        }
        if line == "end" || line.starts_with("end ") {
            if let Some((_, depth)) = containers.last()
                && *depth >= block_depth.saturating_sub(1)
            {
                containers.pop();
            }
            block_depth = block_depth.saturating_sub(1);
        } else if ruby_opens_block(line) {
            block_depth += 1;
        }
        index += 1;
    }

    for method in methods.drain(..) {
        let nested_ranges: Vec<(usize, usize)> = lambdas
            .iter()
            .filter(|lambda| lambda.method_start == method.start)
            .map(|lambda| (lambda.start, lambda.end))
            .collect();
        let local_ranges: Vec<(usize, usize)> = nested_ranges
            .iter()
            .map(|(start, end)| {
                (
                    start.saturating_sub(method.start),
                    end.saturating_sub(method.start),
                )
            })
            .collect();
        let measurements = ruby_measurements(&lines[method.start..=method.end], &local_ranges);
        let index = units.len();
        units.push(CodeUnit {
            name: method.name,
            container: method.container,
            kind: UnitKind::Method,
            span: Span {
                start_line: (method.start + 1) as u32,
                end_line: (method.end + 1) as u32,
            },
            measurements,
            parent: None,
        });
        for lambda in lambdas
            .iter()
            .filter(|lambda| lambda.method_start == method.start)
        {
            let measurements = ruby_measurements(&lines[lambda.start..=lambda.end], &[]);
            units.push(CodeUnit {
                name: lambda.name.clone(),
                container: Some(units[index].name.clone()),
                kind: UnitKind::Lambda,
                span: Span {
                    start_line: (lambda.start + 1) as u32,
                    end_line: (lambda.end + 1) as u32,
                },
                measurements,
                parent: Some(index),
            });
        }
    }
    RawFileAnalysis {
        language: Language::Ruby,
        source_lines: line_count(source),
        parse_status: if block_depth == 0 && containers.is_empty() {
            ParseStatus::Parsed
        } else {
            ParseStatus::Recovered
        },
        units,
    }
}

#[derive(Clone)]
pub(super) struct RubyBlock {
    name: String,
    start: usize,
    end: usize,
    container: Option<String>,
}

#[derive(Clone)]
pub(super) struct RubyLambda {
    name: String,
    start: usize,
    end: usize,
    method_start: usize,
}

fn ruby_container_start(line: &str) -> Option<String> {
    line.strip_prefix("class ")
        .or_else(|| line.strip_prefix("module "))
        .map(|name| {
            name.split_whitespace()
                .next()
                .unwrap_or("<anonymous>")
                .to_owned()
        })
}

fn ruby_method_start(line: &str) -> Option<String> {
    let name = line.strip_prefix("def ")?.trim();
    Some(
        name.split(['(', ' ', '\t'])
            .next()
            .unwrap_or("<anonymous>")
            .to_owned(),
    )
}

fn ruby_opens_block(line: &str) -> bool {
    let first = line.split_whitespace().next().unwrap_or_default();
    matches!(
        first,
        "def" | "class" | "module" | "if" | "unless" | "case" | "while" | "until" | "for" | "begin"
    ) || line.contains(" do")
        || line.ends_with("{")
}

fn ruby_block_end(lines: &[&str], start: usize, depth: usize) -> usize {
    let mut level = depth + 1;
    for (index, line) in lines.iter().enumerate().skip(start + 1) {
        let line = line.trim();
        if ruby_opens_block(line) {
            level += 1;
        }
        if line == "end" || line.starts_with("end ") {
            level = level.saturating_sub(1);
            if level == depth {
                return index;
            }
        }
    }
    lines.len().saturating_sub(1)
}

fn lambda_end(lines: &[&str], start: usize) -> usize {
    let mut level = lines[start]
        .matches('{')
        .count()
        .saturating_sub(lines[start].matches('}').count());
    for (index, line) in lines.iter().enumerate().skip(start + 1) {
        level += line.matches('{').count();
        level = level.saturating_sub(line.matches('}').count());
        if level == 0 {
            return index;
        }
    }
    start
}

fn ruby_measurements(lines: &[&str], nested: &[(usize, usize)]) -> Measurements {
    let mut cognitive = 0u32;
    let mut cyclomatic = 1u32;
    let mut logical = 0u32;
    let mut nesting = 0u32;
    for (offset, line) in lines.iter().enumerate() {
        let global = offset;
        if nested
            .iter()
            .any(|(start, end)| global >= *start && global <= *end)
        {
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        logical += 1;
        let first = trimmed.split_whitespace().next().unwrap_or_default();
        let measured_block =
            !matches!(first, "def" | "class" | "module") && ruby_opens_block(trimmed);
        if measured_block {
            cognitive += 1 + nesting;
            cyclomatic += u32::from(matches!(
                first,
                "if" | "unless" | "case" | "while" | "until" | "for"
            ));
            nesting += 1;
        }
        let branches = trimmed.matches("&&").count()
            + trimmed.matches("||").count()
            + trimmed.matches(" ? ").count();
        cognitive += branches as u32;
        cyclomatic += branches as u32;
        if trimmed == "end" || trimmed.starts_with("end ") {
            nesting = nesting.saturating_sub(1);
        }
    }
    Measurements {
        cognitive_complexity: cognitive,
        cyclomatic_complexity: cyclomatic,
        logical_lines: logical,
    }
}
