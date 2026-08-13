use std::path::Path;

use smackdebt_analysis::{Language, ParseStatus};

use crate::analyzer::{
    AnalysisError, CodeUnit, Measurements, RawFileAnalysis, Span, UnitKind, line_count,
};
use crate::upstream::analyze_upstream;

pub(super) fn analyze_vue(source: &[u8]) -> Result<RawFileAnalysis, AnalysisError> {
    let text = String::from_utf8_lossy(source);
    let lines: Vec<&str> = text.lines().collect();
    let mut units = Vec::new();
    let mut status = ParseStatus::Parsed;
    let mut cursor = 0usize;
    while cursor < lines.len() {
        let line = lines[cursor].trim();
        if line.starts_with("<script") {
            let is_type_script = line.contains("lang=\"ts") || line.contains("lang='ts");
            let (start, end, content) = if let Some(close) = line.find("</script>") {
                let open = line.find('>').map_or(0, |position| position + 1);
                (cursor, cursor, line[open..close].to_owned())
            } else {
                let start = cursor + 1;
                let end = lines
                    .iter()
                    .enumerate()
                    .skip(cursor + 1)
                    .find(|(_, value)| value.contains("</script>"))
                    .map(|(index, _)| index)
                    .unwrap_or_else(|| {
                        status = ParseStatus::Recovered;
                        lines.len().saturating_sub(1)
                    });
                let content_end = end.saturating_sub(1).min(lines.len().saturating_sub(1));
                let content = if start < end {
                    lines[start..=content_end].join("\n")
                } else {
                    String::new()
                };
                (start, end, content)
            };
            let path = if is_type_script {
                Path::new("component.ts")
            } else {
                Path::new("component.js")
            };
            if !content.is_empty() {
                let script = analyze_upstream(
                    if is_type_script {
                        Language::TypeScript
                    } else {
                        Language::JavaScript
                    },
                    content.as_bytes().to_vec(),
                    path,
                )?;
                let parent_offset = units.len();
                for mut unit in script.units {
                    unit.span.start_line += start as u32;
                    unit.span.end_line += start as u32;
                    unit.parent = unit.parent.map(|parent| parent + parent_offset);
                    units.push(unit);
                }
            }
            cursor = end;
        } else if line.starts_with("<template") {
            let start = cursor;
            let end = if line.contains("</template>") {
                cursor
            } else {
                lines
                    .iter()
                    .enumerate()
                    .skip(cursor + 1)
                    .find(|(_, value)| value.contains("</template>"))
                    .map(|(index, _)| index)
                    .unwrap_or_else(|| {
                        status = ParseStatus::Recovered;
                        lines.len().saturating_sub(1)
                    })
            };
            let body = lines[start..=end].join("\n");
            units.push(CodeUnit {
                name: "<template>".to_owned(),
                container: None,
                kind: UnitKind::Template,
                span: Span {
                    start_line: (start + 1) as u32,
                    end_line: (end + 1) as u32,
                },
                measurements: template_measurements(&body),
                parent: None,
            });
            cursor = end;
        }
        cursor += 1;
    }
    Ok(RawFileAnalysis {
        language: Language::Vue,
        source_lines: line_count(source),
        parse_status: status,
        units,
    })
}

fn template_measurements(body: &str) -> Measurements {
    let logical_lines = body
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with("<!--"))
        .count() as u32;
    let inline_handlers = body.matches(" @").count() + body.matches("v-on:").count();
    let cognitive = (body.matches("v-if").count()
        + body.matches("v-else-if").count()
        + body.matches("v-for").count()
        + body.matches(" ? ").count()
        + body.matches("&&").count()
        + body.matches("||").count()
        + inline_handlers) as u32;
    let branches = body.matches("v-if").count()
        + body.matches("v-else-if").count()
        + body.matches("v-for").count()
        + body.matches(" ? ").count()
        + body.matches("&&").count()
        + body.matches("||").count()
        + inline_handlers;
    Measurements {
        cognitive_complexity: cognitive,
        cyclomatic_complexity: branches as u32 + 1,
        logical_lines,
    }
}
