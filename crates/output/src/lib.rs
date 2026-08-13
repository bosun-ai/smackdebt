#![forbid(unsafe_code)]

//! Stable terminal and JSON views over the shared report.

use std::borrow::Cow;
use std::cmp::Reverse;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use smackdebt_analysis::{
    Comparison, ComparisonDirection, ComparisonKind, Diagnostic, DiagnosticKind, FileRecord,
    Finding, HealthCounts, Language, Measurements, Rating, Report, ReportMode, Scope, ScopeKind,
    Signal,
};

/// Terminal display choices. Color is intentionally absent until styling adds
/// useful meaning; output is therefore safe for redirected streams and
/// `NO_COLOR` by default.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalOptions {
    pub width: usize,
    pub all: bool,
}

impl Default for TerminalOptions {
    fn default() -> Self {
        Self {
            width: 100,
            all: false,
        }
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
    let selected = report
        .selected_scope()
        .or_else(|| report.root())
        .and_then(|id| report.scopes().get(id.index()));
    writeln!(writer, "{mode}  {}", selected.map_or(".", Scope::name))?;

    if let Some(scope) = selected {
        let coverage = scope.coverage();
        match report.mode() {
            ReportMode::Codebase => {
                let packages = selected.map_or(0, |scope| package_count(report, scope));
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
                write_health(
                    writer,
                    scope.health(),
                    coverage.unsupported_files() + coverage.failed_files(),
                )?;
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
        ReportMode::Codebase => write_codebase_view(writer, report, selected, options)?,
        ReportMode::Diff => write_diff_view(writer, report, selected, options)?,
    }

    if !report.diagnostics().is_empty() {
        writeln!(writer, "\nCoverage notes")?;
        for diagnostic in report.diagnostics().iter().take(5) {
            writeln!(writer, "  {}", diagnostic.message())?;
        }
    }

    if report.mode() == ReportMode::Codebase
        && let Some(selected) = selected
        && let Some(path) = drill_path(report, selected)
    {
        writeln!(writer, "\nExplore")?;
        writeln!(writer, "  smackdebt {}", path.display())?;
    }
    Ok(())
}

fn display_scope<'a>(
    report: &'a Report,
    mut scope: &'a Scope,
    writer: &mut impl Write,
) -> io::Result<&'a Scope> {
    while scope.kind() != ScopeKind::File && scope.children().len() == 1 {
        let child = &report.scopes()[scope.children()[0].index()];
        if child.name() != "." {
            writeln!(writer, "  ↓ {}", child.name())?;
        }
        scope = child;
    }
    Ok(scope)
}

fn write_codebase_view(
    writer: &mut impl Write,
    report: &Report,
    selected: Option<&Scope>,
    options: TerminalOptions,
) -> io::Result<()> {
    let Some(selected) = selected else {
        return Ok(());
    };
    let scope = display_scope(report, selected, writer)?;
    write_scope_distribution(writer, report, scope, options.all)?;
    write_scope_findings(writer, report, scope, options.width, options.all)?;
    Ok(())
}

fn write_diff_view(
    writer: &mut impl Write,
    report: &Report,
    selected: Option<&Scope>,
    options: TerminalOptions,
) -> io::Result<()> {
    let Some(selected) = selected else {
        return Ok(());
    };
    let scope = display_scope(report, selected, writer)?;
    write_diff_distribution(writer, report, scope, options.all)?;
    write_scope_comparisons(writer, report, scope, options.width, options.all)?;
    Ok(())
}

fn write_health(
    writer: &mut impl Write,
    health: HealthCounts,
    excluded_files: u32,
) -> io::Result<()> {
    writeln!(writer, "\nQuality")?;
    writeln!(
        writer,
        "  {} of {} rated units need attention ({})",
        health.debt(),
        health.total(),
        DisplayPercent::new(health.debt(), health.total())
    )?;
    writeln!(
        writer,
        "  {} high · {} watch · {} healthy · {excluded_files} files excluded",
        health.high(),
        health.watch(),
        health.healthy()
    )
}

fn write_scope_distribution(
    writer: &mut impl Write,
    report: &Report,
    scope: &Scope,
    all: bool,
) -> io::Result<()> {
    if scope.children().is_empty() {
        return Ok(());
    }
    let denominator = scope.health().debt();
    let mut children = display_children(report, scope);
    children.sort_by(|left, right| codebase_child_order(left, right));
    let healthy_only = children
        .iter()
        .filter(|child| child.health().debt() == 0)
        .count();
    if !all {
        children.retain(|child| child.health().debt() > 0);
    }
    if children.is_empty() {
        writeln!(writer, "\nNo child areas need attention.")?;
        if healthy_only > 0 {
            writeln!(writer, "  {healthy_only} quiet areas hidden (use --all)")?;
        }
        return Ok(());
    }
    let limit = if all {
        children.len()
    } else {
        children.len().min(10)
    };
    writeln!(writer, "\nDebt by area")?;
    writeln!(writer, "  area  high  watch  share  rate")?;
    for child in children.iter().take(limit) {
        writeln!(
            writer,
            "  {}  {}  {}  {}  {}",
            child.name(),
            child.health().high(),
            child.health().watch(),
            DisplayPercent::new(child.health().debt(), denominator),
            DisplayPercent::new(child.health().debt(), child.health().total())
        )?;
    }
    if !all && children.len() > limit && healthy_only > 0 {
        writeln!(
            writer,
            "  … {} more debt-bearing · {healthy_only} quiet areas hidden (use --all)",
            children.len() - limit
        )?;
    } else if !all && children.len() > limit {
        writeln!(
            writer,
            "  … {} more debt-bearing areas hidden (use --all)",
            children.len() - limit
        )?;
    } else if !all && healthy_only > 0 {
        writeln!(writer, "  … {healthy_only} quiet areas hidden (use --all)")?;
    }
    Ok(())
}

fn codebase_child_order(left: &Scope, right: &Scope) -> std::cmp::Ordering {
    right
        .health()
        .high()
        .cmp(&left.health().high())
        .then_with(|| right.health().watch().cmp(&left.health().watch()))
        .then_with(|| left.name().cmp(right.name()))
}

fn write_diff_distribution(
    writer: &mut impl Write,
    report: &Report,
    scope: &Scope,
    all: bool,
) -> io::Result<()> {
    if scope.children().is_empty() {
        return Ok(());
    }
    let denominator = scope.diff().total();
    let mut children = display_children(report, scope);
    children.sort_by(|left, right| {
        right
            .diff()
            .worse()
            .cmp(&left.diff().worse())
            .then_with(|| right.diff().better().cmp(&left.diff().better()))
            .then_with(|| right.diff().changed().cmp(&left.diff().changed()))
            .then_with(|| left.name().cmp(right.name()))
    });
    let limit = if all {
        children.len()
    } else {
        children.len().min(10)
    };
    writeln!(writer, "\nChange by area")?;
    writeln!(writer, "  area  worse  better  changed  share")?;
    for child in children.iter().take(limit) {
        let diff = child.diff();
        writeln!(
            writer,
            "  {}  {}  {}  {}  {}%",
            child.name(),
            diff.worse(),
            diff.better(),
            diff.changed(),
            percent(diff.total(), denominator)
        )?;
    }
    if !all && children.len() > limit {
        writeln!(
            writer,
            "  … {} areas omitted (use --all)",
            children.len() - limit
        )?;
    }
    Ok(())
}

fn percent(value: u32, denominator: u32) -> u32 {
    value
        .saturating_mul(100)
        .saturating_add(denominator / 2)
        .checked_div(denominator)
        .unwrap_or(0)
}

struct DisplayPercent {
    value: u32,
    denominator: u32,
}

impl DisplayPercent {
    const fn new(value: u32, denominator: u32) -> Self {
        Self { value, denominator }
    }
}

impl std::fmt::Display for DisplayPercent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let rounded = percent(self.value, self.denominator);
        if self.value > 0 && rounded == 0 {
            formatter.write_str("<1%")
        } else {
            write!(formatter, "{rounded}%")
        }
    }
}

fn display_children<'a>(report: &'a Report, scope: &'a Scope) -> Vec<&'a Scope> {
    let mut children = Vec::new();
    for id in scope.children() {
        let child = &report.scopes()[id.index()];
        if scope.kind() == ScopeKind::Repository
            && child.kind() == ScopeKind::Package
            && child.name() == "."
        {
            children.extend(
                child
                    .children()
                    .iter()
                    .map(|id| &report.scopes()[id.index()]),
            );
        } else {
            children.push(child);
        }
    }
    children
}

fn package_count(report: &Report, scope: &Scope) -> usize {
    let own = usize::from(scope.kind() == ScopeKind::Package);
    let descendants = own
        + scope
            .children()
            .iter()
            .map(|id| package_count(report, &report.scopes()[id.index()]))
            .sum::<usize>();
    if descendants == 0 && scope.kind() != ScopeKind::Repository {
        1
    } else {
        descendants
    }
}

fn write_scope_findings(
    writer: &mut impl Write,
    report: &Report,
    scope: &Scope,
    _width: usize,
    all: bool,
) -> io::Result<()> {
    if scope.findings().is_empty() {
        writeln!(writer, "\nNo watch or high findings.")?;
        return Ok(());
    }
    let mut findings: Vec<&Finding> = scope
        .findings()
        .iter()
        .map(|id| &report.findings()[id.index()])
        .collect();
    findings.sort_by(|left, right| finding_order(report, left, right));
    let limit = if all || scope.kind() == ScopeKind::File {
        findings.len()
    } else {
        findings.len().min(3)
    };
    writeln!(writer, "\nDebt findings")?;
    for finding in findings.into_iter().take(limit) {
        let file = &report.files()[finding.file().index()];
        if let Some(container) = finding.identity().container() {
            writeln!(
                writer,
                "  {}:{}  {container}::{}",
                file.path(),
                finding.span().start_line(),
                finding.identity().name()
            )?;
        } else {
            writeln!(
                writer,
                "  {}:{}  {}",
                file.path(),
                finding.span().start_line(),
                finding.identity().name()
            )?;
        }
        write_attention_reasons(writer, finding, file)?;
    }
    if !all && limit < scope.findings().len() {
        writeln!(
            writer,
            "  … {} findings omitted (use --all)",
            scope.findings().len() - limit
        )?;
    }
    Ok(())
}

fn write_attention_reasons(
    writer: &mut impl Write,
    finding: &Finding,
    file: &FileRecord,
) -> io::Result<()> {
    write!(
        writer,
        "    {} because ",
        rating_name(finding.assessment().rating())
    )?;
    let mut first = true;
    for signal in finding.assessment().signals() {
        if signal.rating() == Rating::Healthy {
            continue;
        }
        if !first {
            write!(writer, " · ")?;
        }
        write!(
            writer,
            "{} {}",
            signal_name(signal.signal()),
            signal.value()
        )?;
        first = false;
    }
    writeln!(
        writer,
        " · {} touches",
        file.activity().map_or(0, |activity| activity.touches())
    )
}

fn signal_name(signal: Signal) -> &'static str {
    match signal {
        Signal::CognitiveComplexity => "cognitive",
        Signal::CyclomaticComplexity => "cyclomatic",
        Signal::LogicalLines => "lines",
    }
}

fn write_scope_comparisons(
    writer: &mut impl Write,
    report: &Report,
    scope: &Scope,
    width: usize,
    all: bool,
) -> io::Result<()> {
    if scope.comparisons().is_empty() {
        writeln!(writer, "\nNo changed units.")?;
        return Ok(());
    }
    let mut comparisons: Vec<&Comparison> = scope
        .comparisons()
        .iter()
        .map(|id| &report.comparisons()[id.index()])
        .collect();
    comparisons.sort_by(|left, right| {
        direction_rank(left.direction())
            .cmp(&direction_rank(right.direction()))
            .then_with(|| left.identity().name().cmp(right.identity().name()))
    });
    let limit = if all || scope.kind() == ScopeKind::File {
        comparisons.len()
    } else {
        comparisons.len().min(3)
    };
    writeln!(writer, "\nHealth change")?;
    for comparison in comparisons.into_iter().take(limit) {
        let name = truncate(comparison.identity().name(), width.saturating_sub(24));
        writeln!(
            writer,
            "  {name}  {}",
            direction_name(comparison.direction())
        )?;
        if let Some(file_id) = comparison.file() {
            writeln!(writer, "    {}", report.files()[file_id.index()].path())?;
        }
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
    if !all && limit < scope.comparisons().len() {
        writeln!(
            writer,
            "  … {} comparisons omitted (use --all)",
            scope.comparisons().len() - limit
        )?;
    }
    Ok(())
}

fn direction_rank(direction: ComparisonDirection) -> u8 {
    match direction {
        ComparisonDirection::Worse => 0,
        ComparisonDirection::Better => 1,
        ComparisonDirection::Changed => 2,
    }
}
fn direction_name(direction: ComparisonDirection) -> &'static str {
    match direction {
        ComparisonDirection::Worse => "worse",
        ComparisonDirection::Better => "better",
        ComparisonDirection::Changed => "changed",
    }
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

fn drill_path(report: &Report, selected: &Scope) -> Option<PathBuf> {
    let scope = skipped_scope(report, selected);
    let mut children = display_children(report, scope);
    children.retain(|child| child.health().debt() > 0);
    children.sort_by(|left, right| codebase_child_order(left, right));
    let child = children.first()?;
    let root = report.root().map(|id| &report.scopes()[id.index()])?;
    let invocation_path = Path::new(root.name());
    if invocation_path == Path::new(".") {
        return Some(PathBuf::from(child.name()));
    }
    let suffix = Path::new(child.name())
        .strip_prefix(selected.name())
        .unwrap_or_else(|_| Path::new(child.name()));
    Some(invocation_path.join(suffix))
}

fn skipped_scope<'a>(report: &'a Report, mut scope: &'a Scope) -> &'a Scope {
    while scope.kind() != ScopeKind::File && scope.children().len() == 1 {
        scope = &report.scopes()[scope.children()[0].index()];
    }
    scope
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
        let mut map = serializer.serialize_map(Some(11))?;
        map.serialize_entry("schema_version", &report.schema_version())?;
        map.serialize_entry("mode", mode_name(report.mode()))?;
        map.serialize_entry(
            "root",
            &report
                .root()
                .map(|root| report.scopes()[root.index()].name()),
        )?;
        map.serialize_entry(
            "selected_scope",
            &report.selected_scope().map(|id| id.get()),
        )?;
        map.serialize_entry("paths", &report.paths())?;
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
        let mut map = serializer.serialize_map(Some(11))?;
        map.serialize_entry("id", &scope.id().get())?;
        map.serialize_entry("kind", scope_kind(scope.kind()))?;
        map.serialize_entry("name", scope.name())?;
        map.serialize_entry("parent", &scope.parent().map(|id| id.get()))?;
        map.serialize_entry("children", &ChildIds(scope.children()))?;
        map.serialize_entry("path", &scope.path().map(|id| id.get()))?;
        map.serialize_entry("findings", &FindingIds(scope.findings()))?;
        map.serialize_entry("comparisons", &ComparisonIds(scope.comparisons()))?;
        map.serialize_entry("coverage", &CoverageView(scope.coverage()))?;
        map.serialize_entry("health", &HealthView(scope.health()))?;
        map.serialize_entry("diff", &DiffView(scope.diff()))?;
        map.end()
    }
}

struct FindingIds<'a>(&'a [smackdebt_analysis::FindingId]);
impl Serialize for FindingIds<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for id in self.0 {
            sequence.serialize_element(&id.get())?;
        }
        sequence.end()
    }
}

struct ComparisonIds<'a>(&'a [smackdebt_analysis::ComparisonId]);
impl Serialize for ComparisonIds<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for id in self.0 {
            sequence.serialize_element(&id.get())?;
        }
        sequence.end()
    }
}

struct DiffView(smackdebt_analysis::DiffCounts);
impl Serialize for DiffView {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("worse", &self.0.worse())?;
        map.serialize_entry("better", &self.0.better())?;
        map.serialize_entry("changed", &self.0.changed())?;
        map.serialize_entry("total", &self.0.total())?;
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
        let mut map = serializer.serialize_map(Some(8))?;
        map.serialize_entry("id", &file.id().get())?;
        map.serialize_entry("scope", &file.scope().get())?;
        map.serialize_entry("path", file.path())?;
        map.serialize_entry("language", &file.language().map(language_name))?;
        map.serialize_entry("coverage", &CoverageView(file.coverage()))?;
        map.serialize_entry("health", &HealthView(file.health()))?;
        map.serialize_entry("touches", &file.activity().map(|value| value.touches()))?;
        map.serialize_entry("path_id", &file.path_id().map(|id| id.get()))?;
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
        let mut map = serializer.serialize_map(Some(9))?;
        map.serialize_entry("id", &comparison.id().get())?;
        map.serialize_entry("file", &comparison.file().map(|id| id.get()))?;
        map.serialize_entry("name", comparison.identity().name())?;
        map.serialize_entry("container", &comparison.identity().container())?;
        map.serialize_entry("kind", comparison_name(comparison.kind()))?;
        map.serialize_entry("direction", direction_name(comparison.direction()))?;
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
        Scope, ScopeId, SourceSpan, UnitId, UnitIdentity, aggregate_scopes,
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
            report.scopes_mut()[0].add_finding(FindingId::from_index(index));
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
        assert!(terminal.contains("Debt findings"));
        assert!(terminal.contains("file-10.rs"));
        assert!(!terminal.contains("file-1.rs"));
    }

    #[test]
    fn absent_activity_does_not_call_findings_hotspots() {
        let report = report_with_findings(false);
        let mut terminal = Vec::new();
        write_terminal(&mut terminal, &report, TerminalOptions::default()).unwrap();
        let terminal = String::from_utf8(terminal).unwrap();
        assert!(terminal.contains("Debt findings"));
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

    #[test]
    fn small_nonzero_percentages_are_not_shown_as_zero() {
        assert_eq!(DisplayPercent::new(1, 201).to_string(), "<1%");
        assert_eq!(DisplayPercent::new(0, 200).to_string(), "0%");
        assert_eq!(DisplayPercent::new(3, 100).to_string(), "3%");
    }

    #[test]
    fn drill_path_preserves_an_external_invocation_path() {
        let mut report = Report::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        let package = ScopeId::from_index(1);
        let child = ScopeId::from_index(2);
        let quiet = ScopeId::from_index(3);
        let mut root_scope = Scope::new(root, ScopeKind::Repository, "/work/project/bow", None);
        root_scope.add_child(package);
        report.add_scope(root_scope);
        let mut package_scope = Scope::new(package, ScopeKind::Package, "bow", Some(root));
        package_scope.add_child(child);
        package_scope.add_child(quiet);
        report.add_scope(package_scope);
        report.add_scope(Scope::new(
            child,
            ScopeKind::Directory,
            "bow/src",
            Some(package),
        ));
        report.add_scope(Scope::new(
            quiet,
            ScopeKind::Directory,
            "bow/tests",
            Some(package),
        ));
        report.set_root(root);
        let file_id = FileId::from_index(0);
        report.add_file(FileRecord::new(
            file_id,
            child,
            "bow/src/lib.rs",
            Coverage::new(1, 1, 0, 0, 10, 0),
            HealthCounts::new(0, 1, 0),
        ));
        report.scopes_mut()[child.index()].add_file(file_id);
        let files = report.files().to_vec();
        aggregate_scopes(report.scopes_mut(), &files, root);

        let path = drill_path(&report, &report.scopes()[package.index()]);

        assert_eq!(path, Some(PathBuf::from("/work/project/bow/src")));
    }
}
