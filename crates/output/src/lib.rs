#![forbid(unsafe_code)]

//! Stable terminal and JSON views over the shared report.

use std::borrow::Cow;
use std::cmp::Reverse;
use std::io::{self, Write};

use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use smackdebt_analysis::{
    Comparison, ComparisonKind, Diagnostic, DiagnosticKind, FileRecord, Finding, HealthCounts,
    Language, Measurements, Rating, Report, ReportMode, Scope, ScopeKind,
};

/// Terminal display choices. Color is intentionally absent until styling adds
/// useful meaning; output is therefore safe for redirected streams and
/// `NO_COLOR` by default.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalOptions {
    pub width: usize,
}

impl Default for TerminalOptions {
    fn default() -> Self {
        Self { width: 100 }
    }
}

/// Writes a concise, stable terminal report.
pub fn write_terminal(
    writer: &mut impl Write,
    report: &Report,
    options: TerminalOptions,
) -> io::Result<()> {
    let mode = match report.mode() {
        ReportMode::Codebase => "smackdebt",
        ReportMode::Diff => "smackdebt diff",
    };
    let root = report.root().and_then(|id| report.scopes().get(id.index()));
    writeln!(writer, "{mode}  {}", root.map_or(".", Scope::name))?;

    if let Some(root) = root {
        let coverage = root.coverage();
        match report.mode() {
            ReportMode::Codebase => {
                let packages = report
                    .scopes()
                    .iter()
                    .filter(|scope| scope.kind() == ScopeKind::Package)
                    .count();
                let analyzed_percent = if coverage.selected_files() == 0 {
                    100
                } else {
                    coverage.analyzed_files() * 100 / coverage.selected_files()
                };
                writeln!(
                    writer,
                    "{packages} packages · {} files · {} lines · {analyzed_percent}% analyzed",
                    coverage.selected_files(),
                    coverage.source_lines(),
                )?;
                write_health(writer, root.health())?;
            }
            ReportMode::Diff => writeln!(
                writer,
                "{} source files changed · {} analyzed · {} not compared",
                coverage.selected_files(),
                coverage.analyzed_files(),
                coverage.unsupported_files() + coverage.failed_files(),
            )?,
        }
    }

    match report.mode() {
        ReportMode::Codebase => write_findings(writer, report, options.width)?,
        ReportMode::Diff => write_comparisons(writer, report, options.width)?,
    }

    if !report.diagnostics().is_empty() {
        writeln!(writer, "\nCoverage notes")?;
        for diagnostic in report.diagnostics().iter().take(5) {
            writeln!(writer, "  {}", diagnostic.message())?;
        }
    }

    if report.mode() == ReportMode::Codebase
        && let Some(path) = drill_path(report)
    {
        writeln!(writer, "\nExplore")?;
        writeln!(writer, "  smackdebt {path}")?;
    }
    Ok(())
}

fn write_health(writer: &mut impl Write, health: HealthCounts) -> io::Result<()> {
    writeln!(writer, "\nHealth")?;
    writeln!(writer, "  high    {} units", health.high())?;
    writeln!(writer, "  watch   {} units", health.watch())?;
    writeln!(writer, "  healthy {} units", health.healthy())
}

fn write_findings(writer: &mut impl Write, report: &Report, width: usize) -> io::Result<()> {
    if report.findings().is_empty() {
        writeln!(writer, "\nNo watch or high findings.")?;
        return Ok(());
    }
    if let Some(threshold) = activity_threshold(report) {
        write_finding_group(writer, report, width, "Hotspots", Some((true, threshold)))?;
        write_finding_group(
            writer,
            report,
            width,
            "Other debt findings",
            Some((false, threshold)),
        )?;
    } else {
        write_finding_group(writer, report, width, "Debt findings", None)?;
    }
    Ok(())
}

fn write_finding_group(
    writer: &mut impl Write,
    report: &Report,
    width: usize,
    heading: &str,
    hotspot: Option<(bool, u32)>,
) -> io::Result<()> {
    let findings = top_findings(report, hotspot);
    if findings[0].is_none() {
        return Ok(());
    }
    writeln!(writer, "\n{heading}")?;
    for finding in findings.into_iter().flatten() {
        let file = &report.files()[finding.file().index()];
        let path = truncate(file.path(), width.saturating_sub(8));
        writeln!(writer, "  {path}")?;
        let measurements = finding.measurements();
        writeln!(
            writer,
            "    {}  {} · cognitive {} · cyclomatic {} · {} lines · {} touches",
            finding.identity().name(),
            rating_name(finding.assessment().rating()),
            measurements.cognitive_complexity(),
            measurements.cyclomatic_complexity(),
            measurements.logical_lines(),
            file.activity().map_or(0, |activity| activity.touches())
        )?;
    }
    Ok(())
}

fn top_findings(report: &Report, hotspot: Option<(bool, u32)>) -> [Option<&Finding>; 10] {
    let mut selected = [None; 10];
    for finding in report.findings() {
        if let Some((expected, threshold)) = hotspot
            && is_hotspot(report, finding, threshold) != expected
        {
            continue;
        }
        let Some(position) = selected.iter().position(|entry| {
            entry.is_none_or(|current| finding_order(report, finding, current).is_lt())
        }) else {
            continue;
        };
        for index in (position + 1..selected.len()).rev() {
            selected[index] = selected[index - 1];
        }
        selected[position] = Some(finding);
    }
    selected
}

fn activity_threshold(report: &Report) -> Option<u32> {
    let mut touches: Vec<u32> = report
        .files()
        .iter()
        .filter_map(|file| file.activity().map(|value| value.touches()))
        .filter(|touches| *touches >= 2)
        .collect();
    if touches.is_empty() {
        return None;
    }
    touches.sort_unstable();
    Some(touches[(touches.len() - 1) * 3 / 4])
}

fn is_hotspot(report: &Report, finding: &Finding, threshold: u32) -> bool {
    report.files()[finding.file().index()]
        .activity()
        .is_some_and(|activity| activity.touches() >= 2 && activity.touches() >= threshold)
}

fn finding_order(report: &Report, left: &Finding, right: &Finding) -> std::cmp::Ordering {
    let left_file = &report.files()[left.file().index()];
    let right_file = &report.files()[right.file().index()];
    (
        Reverse(rating_rank(left.assessment().rating())),
        Reverse(
            left_file
                .activity()
                .map_or(0, |activity| activity.touches()),
        ),
        Reverse(left.measurements().cognitive_complexity()),
        Reverse(left.measurements().cyclomatic_complexity()),
        Reverse(left.measurements().logical_lines()),
        left_file.path(),
        left.span().start_line(),
    )
        .cmp(&(
            Reverse(rating_rank(right.assessment().rating())),
            Reverse(
                right_file
                    .activity()
                    .map_or(0, |activity| activity.touches()),
            ),
            Reverse(right.measurements().cognitive_complexity()),
            Reverse(right.measurements().cyclomatic_complexity()),
            Reverse(right.measurements().logical_lines()),
            right_file.path(),
            right.span().start_line(),
        ))
}

fn write_comparisons(writer: &mut impl Write, report: &Report, width: usize) -> io::Result<()> {
    let changed = report
        .comparisons()
        .iter()
        .filter(|comparison| comparison.kind() != ComparisonKind::Unchanged)
        .count();
    writeln!(writer, "\nHealth change")?;
    writeln!(writer, "  {changed} changed units")?;
    for comparison in report
        .comparisons()
        .iter()
        .filter(|value| value.kind() != ComparisonKind::Unchanged)
        .take(10)
    {
        let name = truncate(comparison.identity().name(), width.saturating_sub(24));
        writeln!(writer, "  {name}  {}", comparison_name(comparison.kind()))?;
        if let (Some(before), Some(after)) = (comparison.before(), comparison.after()) {
            writeln!(
                writer,
                "    cognitive {} → {} · cyclomatic {} → {} · lines {} → {}",
                before.cognitive_complexity(),
                after.cognitive_complexity(),
                before.cyclomatic_complexity(),
                after.cyclomatic_complexity(),
                before.logical_lines(),
                after.logical_lines()
            )?;
        }
    }
    Ok(())
}

fn drill_path(report: &Report) -> Option<&str> {
    top_findings(report, None)[0]
        .and_then(|finding| report.files().get(finding.file().index()))
        .map(FileRecord::path)
        .or_else(|| report.files().first().map(FileRecord::path))
}

fn truncate(value: &str, width: usize) -> Cow<'_, str> {
    if value.chars().count() <= width {
        return Cow::Borrowed(value);
    }
    if width <= 1 {
        return Cow::Owned("…".to_owned());
    }
    let tail: String = value
        .chars()
        .rev()
        .take(width - 1)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    Cow::Owned(format!("…{tail}"))
}

/// Streams JSON schema version 1 without cloning report strings or arrays.
pub fn write_json(writer: &mut impl Write, report: &Report) -> io::Result<()> {
    let mut serializer = serde_json::Serializer::new(writer);
    ReportView(report)
        .serialize(&mut serializer)
        .map_err(io::Error::other)
}

struct ReportView<'a>(&'a Report);

impl Serialize for ReportView<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let report = self.0;
        let mut map = serializer.serialize_map(Some(9))?;
        map.serialize_entry("schema_version", &report.schema_version())?;
        map.serialize_entry("mode", mode_name(report.mode()))?;
        map.serialize_entry(
            "root",
            &report
                .root()
                .map(|root| report.scopes()[root.index()].name()),
        )?;
        map.serialize_entry("scopes", &Scopes(report.scopes()))?;
        map.serialize_entry("files", &Files(report.files()))?;
        map.serialize_entry("findings", &Findings(report.findings()))?;
        map.serialize_entry("diagnostics", &Diagnostics(report.diagnostics()))?;
        map.serialize_entry("comparisons", &Comparisons(report.comparisons()))?;
        map.serialize_entry("summary", &Summary(report))?;
        map.end()
    }
}

struct Scopes<'a>(&'a [Scope]);
impl Serialize for Scopes<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for scope in self.0 {
            sequence.serialize_element(&ScopeView(scope))?;
        }
        sequence.end()
    }
}

struct ScopeView<'a>(&'a Scope);
impl Serialize for ScopeView<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let scope = self.0;
        let mut map = serializer.serialize_map(Some(7))?;
        map.serialize_entry("id", &scope.id().get())?;
        map.serialize_entry("kind", scope_kind(scope.kind()))?;
        map.serialize_entry("name", scope.name())?;
        map.serialize_entry("parent", &scope.parent().map(|id| id.get()))?;
        map.serialize_entry("children", &ChildIds(scope.children()))?;
        map.serialize_entry("coverage", &CoverageView(scope.coverage()))?;
        map.serialize_entry("health", &HealthView(scope.health()))?;
        map.end()
    }
}

struct ChildIds<'a>(&'a [smackdebt_analysis::ScopeId]);

impl Serialize for ChildIds<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for child in self.0 {
            sequence.serialize_element(&child.get())?;
        }
        sequence.end()
    }
}

struct Files<'a>(&'a [FileRecord]);
impl Serialize for Files<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for file in self.0 {
            sequence.serialize_element(&FileView(file))?;
        }
        sequence.end()
    }
}

struct FileView<'a>(&'a FileRecord);
impl Serialize for FileView<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let file = self.0;
        let mut map = serializer.serialize_map(Some(7))?;
        map.serialize_entry("id", &file.id().get())?;
        map.serialize_entry("scope", &file.scope().get())?;
        map.serialize_entry("path", file.path())?;
        map.serialize_entry("language", &file.language().map(language_name))?;
        map.serialize_entry("coverage", &CoverageView(file.coverage()))?;
        map.serialize_entry("health", &HealthView(file.health()))?;
        map.serialize_entry("touches", &file.activity().map(|value| value.touches()))?;
        map.end()
    }
}

struct Findings<'a>(&'a [Finding]);
impl Serialize for Findings<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for finding in self.0 {
            sequence.serialize_element(&FindingView(finding))?;
        }
        sequence.end()
    }
}

struct FindingView<'a>(&'a Finding);
impl Serialize for FindingView<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let finding = self.0;
        let mut map = serializer.serialize_map(Some(8))?;
        map.serialize_entry("id", &finding.id().get())?;
        map.serialize_entry("file", &finding.file().get())?;
        map.serialize_entry("name", finding.identity().name())?;
        map.serialize_entry("container", &finding.identity().container())?;
        map.serialize_entry("start_line", &finding.span().start_line())?;
        map.serialize_entry("end_line", &finding.span().end_line())?;
        map.serialize_entry("rating", rating_name(finding.assessment().rating()))?;
        map.serialize_entry("measurements", &MeasurementsView(finding.measurements()))?;
        map.end()
    }
}

struct Diagnostics<'a>(&'a [Diagnostic]);
impl Serialize for Diagnostics<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for diagnostic in self.0 {
            sequence.serialize_element(&DiagnosticView(diagnostic))?;
        }
        sequence.end()
    }
}

struct DiagnosticView<'a>(&'a Diagnostic);
impl Serialize for DiagnosticView<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let diagnostic = self.0;
        let mut map = serializer.serialize_map(Some(5))?;
        map.serialize_entry("id", &diagnostic.id().get())?;
        map.serialize_entry("file", &diagnostic.file().map(|id| id.get()))?;
        map.serialize_entry("kind", diagnostic_name(diagnostic.kind()))?;
        map.serialize_entry("message", diagnostic.message())?;
        map.serialize_entry("excluded_lines", &diagnostic.excluded_lines())?;
        map.end()
    }
}

struct Comparisons<'a>(&'a [Comparison]);
impl Serialize for Comparisons<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for comparison in self.0 {
            sequence.serialize_element(&ComparisonView(comparison))?;
        }
        sequence.end()
    }
}

struct ComparisonView<'a>(&'a Comparison);
impl Serialize for ComparisonView<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let comparison = self.0;
        let mut map = serializer.serialize_map(Some(7))?;
        map.serialize_entry("id", &comparison.id().get())?;
        map.serialize_entry("name", comparison.identity().name())?;
        map.serialize_entry("container", &comparison.identity().container())?;
        map.serialize_entry("kind", comparison_name(comparison.kind()))?;
        map.serialize_entry("before", &comparison.before().map(MeasurementsView))?;
        map.serialize_entry("after", &comparison.after().map(MeasurementsView))?;
        map.serialize_entry(
            "ratings",
            &(
                comparison.before_rating().map(rating_name),
                comparison.after_rating().map(rating_name),
            ),
        )?;
        map.end()
    }
}

struct Summary<'a>(&'a Report);
impl Serialize for Summary<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let health = self
            .0
            .root()
            .and_then(|id| self.0.scopes().get(id.index()))
            .map_or_else(HealthCounts::default, Scope::health);
        HealthView(health).serialize(serializer)
    }
}

struct HealthView(HealthCounts);
impl Serialize for HealthView {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("healthy", &self.0.healthy())?;
        map.serialize_entry("watch", &self.0.watch())?;
        map.serialize_entry("high", &self.0.high())?;
        map.end()
    }
}

struct CoverageView(smackdebt_analysis::Coverage);
impl Serialize for CoverageView {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(6))?;
        map.serialize_entry("selected_files", &self.0.selected_files())?;
        map.serialize_entry("analyzed_files", &self.0.analyzed_files())?;
        map.serialize_entry("unsupported_files", &self.0.unsupported_files())?;
        map.serialize_entry("failed_files", &self.0.failed_files())?;
        map.serialize_entry("source_lines", &self.0.source_lines())?;
        map.serialize_entry("excluded_lines", &self.0.excluded_lines())?;
        map.end()
    }
}

struct MeasurementsView(Measurements);
impl Serialize for MeasurementsView {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("cognitive_complexity", &self.0.cognitive_complexity())?;
        map.serialize_entry("cyclomatic_complexity", &self.0.cyclomatic_complexity())?;
        map.serialize_entry("logical_lines", &self.0.logical_lines())?;
        map.end()
    }
}

fn rating_rank(rating: Rating) -> u8 {
    match rating {
        Rating::Healthy => 0,
        Rating::Watch => 1,
        Rating::High => 2,
    }
}

fn rating_name(rating: Rating) -> &'static str {
    match rating {
        Rating::Healthy => "healthy",
        Rating::Watch => "watch",
        Rating::High => "high",
    }
}

fn mode_name(mode: ReportMode) -> &'static str {
    match mode {
        ReportMode::Codebase => "codebase",
        ReportMode::Diff => "diff",
    }
}

fn scope_kind(kind: ScopeKind) -> &'static str {
    match kind {
        ScopeKind::Repository => "repository",
        ScopeKind::Package => "package",
        ScopeKind::Directory => "directory",
        ScopeKind::File => "file",
    }
}

fn language_name(language: Language) -> &'static str {
    match language {
        Language::C => "c",
        Language::Cpp => "cpp",
        Language::Java => "java",
        Language::JavaScript => "javascript",
        Language::Jsx => "jsx",
        Language::Python => "python",
        Language::Rust => "rust",
        Language::TypeScript => "typescript",
        Language::Tsx => "tsx",
        Language::Ruby => "ruby",
        Language::Vue => "vue",
        Language::Kotlin => "kotlin",
        Language::Unknown => "unknown",
    }
}

fn diagnostic_name(kind: DiagnosticKind) -> &'static str {
    match kind {
        DiagnosticKind::UnsupportedLanguage => "unsupported_language",
        DiagnosticKind::UnreadableFile => "unreadable_file",
        DiagnosticKind::OversizedFile => "oversized_file",
        DiagnosticKind::ParseFailure => "parse_failure",
        DiagnosticKind::AmbiguousIdentity => "ambiguous_identity",
        DiagnosticKind::UnsafeReference => "unsafe_reference",
        DiagnosticKind::Other => "other",
    }
}

fn comparison_name(kind: ComparisonKind) -> &'static str {
    match kind {
        ComparisonKind::Added => "added",
        ComparisonKind::Removed => "removed",
        ComparisonKind::Improved => "improved",
        ComparisonKind::Regressed => "regressed",
        ComparisonKind::MetricChanged => "metric_changed",
        ComparisonKind::Ambiguous => "ambiguous",
        ComparisonKind::Unchanged => "unchanged",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smackdebt_analysis::{
        Coverage, FileActivity, FileId, FileRecord, FindingId, HealthPolicy, Report, ReportMode,
        Scope, ScopeId, SourceSpan, UnitId, UnitIdentity,
    };

    fn report_with_findings(activity: bool) -> Report {
        let mut report = Report::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        report.add_scope(Scope::new(root, ScopeKind::Repository, ".", None));
        report.set_root(root);
        let policy = HealthPolicy::default();
        for index in 0..11 {
            let file_id = FileId::from_index(index);
            let mut file = FileRecord::new(
                file_id,
                root,
                format!("file-{index}.rs"),
                Coverage::new(1, 1, 0, 0, 10, 0),
                HealthCounts::new(0, 1, 0),
            );
            if activity {
                file = file.with_activity(FileActivity::new(index as u32));
            }
            report.add_file(file);
            let measurements = Measurements::new(15, 1, 1);
            report.add_finding(Finding::new(
                FindingId::from_index(index),
                UnitId::from_index(index),
                file_id,
                UnitIdentity::new(
                    format!("unit-{index}"),
                    smackdebt_analysis::UnitKind::Function,
                ),
                SourceSpan::new(1, 2),
                measurements,
                policy.assess(measurements),
            ));
        }
        report
    }

    #[test]
    fn empty_report_is_valid_json_and_stable_terminal_text() {
        let mut report = Report::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        report.add_scope(Scope::new(root, ScopeKind::Repository, ".", None));
        report.set_root(root);

        let mut terminal = Vec::new();
        write_terminal(&mut terminal, &report, TerminalOptions::default()).unwrap();
        assert!(
            String::from_utf8(terminal)
                .unwrap()
                .contains("No watch or high findings")
        );

        let mut json = Vec::new();
        write_json(&mut json, &report).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["mode"], "codebase");
    }

    #[test]
    fn activity_changes_heading_and_selects_the_ten_most_active_findings() {
        let report = report_with_findings(true);
        let mut terminal = Vec::new();
        write_terminal(&mut terminal, &report, TerminalOptions::default()).unwrap();
        let terminal = String::from_utf8(terminal).unwrap();
        assert!(terminal.contains("Hotspots"));
        let hotspot_section = terminal.split("Other debt findings").next().unwrap();
        assert!(!hotspot_section.contains("file-0.rs"));
        assert!(terminal.find("file-10.rs").unwrap() < terminal.find("file-1.rs").unwrap());
    }

    #[test]
    fn absent_activity_does_not_call_findings_hotspots() {
        let report = report_with_findings(false);
        let mut terminal = Vec::new();
        write_terminal(&mut terminal, &report, TerminalOptions::default()).unwrap();
        let terminal = String::from_utf8(terminal).unwrap();
        assert!(terminal.contains("Debt findings"));
        assert!(!terminal.contains("Hotspots"));
    }

    #[test]
    fn truncation_borrows_values_that_fit() {
        assert!(matches!(truncate("short", 20), Cow::Borrowed("short")));
        assert_eq!(truncate("directory/file.rs", 8), "…file.rs");
    }

    #[test]
    fn child_ids_are_written_as_a_json_array() {
        let mut report = Report::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        let child = ScopeId::from_index(1);
        let mut root_scope = Scope::new(root, ScopeKind::Repository, ".", None);
        root_scope.add_child(child);
        root_scope.add_child(ScopeId::from_index(2));
        report.add_scope(root_scope);
        report.add_scope(Scope::new(child, ScopeKind::Package, "pkg", Some(root)));
        report.add_scope(Scope::new(
            ScopeId::from_index(2),
            ScopeKind::Package,
            "other",
            Some(root),
        ));
        report.set_root(root);

        let mut json = Vec::new();
        write_json(&mut json, &report).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(value["scopes"][0]["children"], serde_json::json!([1, 2]));
    }
}
