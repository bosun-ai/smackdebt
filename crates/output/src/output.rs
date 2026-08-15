//! Stable terminal and JSON views over the shared report.

use std::cmp::Reverse;
use std::fmt;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anstyle::{Ansi256Color, AnsiColor, Style};
use smackdebt_analysis::{
    ArchitectureComparisonKind, ArchitectureFindingKind, CodebaseTier, Comparison,
    ComparisonDirection, ComparisonKind, DebtDiffSelection, DebtFamily, Diagnostic, DiagnosticKind,
    DiffTier, FileId, FileRecord, Finding, Instability, Language, Rating, Report, ReportMode,
    ResolutionIssueKind, Scope, ScopeId, ScopeKind, Signal, SourceRole, SourceTrust,
    StaticRelationKind, UnitKind, Verdict, instability, qualifies_for_finding,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// The indent every continued or stacked row line uses.
const INDENT: usize = 8;

/// Resolved terminal display choices.
///
/// Words carry every meaning. Decoration is a separate resolved choice so the
/// same report can be written for a terminal and for a pipe from one code path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalOptions {
    width: usize,
    all: bool,
    color: bool,
    decorations: bool,
}

impl TerminalOptions {
    /// Words-only options, which is what a pipe receives.
    pub const fn new(width: usize, all: bool, color: bool) -> Self {
        Self {
            width,
            all,
            color,
            decorations: false,
        }
    }

    /// Adds glyph and tier-bar decoration, resolved by the caller from
    /// terminal detection.
    pub const fn with_decorations(mut self, decorations: bool) -> Self {
        self.decorations = decorations;
        self
    }
}

impl Default for TerminalOptions {
    fn default() -> Self {
        Self {
            width: 100,
            all: false,
            color: false,
            decorations: false,
        }
    }
}

/// Writes a responsive terminal report from retained report facts.
pub fn write_terminal(
    writer: &mut impl Write,
    report: &Report,
    selected_scope: Option<ScopeId>,
    options: TerminalOptions,
) -> io::Result<()> {
    let presentation = Presentation::new(report, selected_scope, options.all);
    let mut width_writer = WidthWriter::new(writer, options.width);
    Renderer::new(&mut width_writer, options).write(&presentation)?;
    width_writer.finish()?;
    #[cfg(feature = "width-test-checks")]
    assert_eq!(
        width_writer.truncations(),
        0,
        "structured terminal layout reached the safety shortening"
    );
    Ok(())
}

struct WidthWriter<'a, W> {
    writer: &'a mut W,
    width: usize,
    line: Vec<u8>,
    truncations: usize,
}

impl<'a, W: Write> WidthWriter<'a, W> {
    fn new(writer: &'a mut W, width: usize) -> Self {
        Self {
            writer,
            width,
            line: Vec::new(),
            truncations: 0,
        }
    }

    fn finish(&mut self) -> io::Result<()> {
        if !self.line.is_empty() {
            self.flush_line(false)?;
        }
        self.writer.flush()
    }

    fn flush_line(&mut self, newline: bool) -> io::Result<()> {
        let line = std::str::from_utf8(&self.line)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if ansi_display_width(line) <= self.width {
            self.writer.write_all(&self.line)?;
        } else {
            self.truncations += 1;
            self.writer
                .write_all(truncate_ansi_end(line, self.width).as_bytes())?;
        }
        if newline {
            self.writer.write_all(b"\n")?;
        }
        self.line.clear();
        Ok(())
    }

    #[cfg(any(test, feature = "width-test-checks"))]
    const fn truncations(&self) -> usize {
        self.truncations
    }
}

impl<W: Write> Write for WidthWriter<'_, W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let mut start = 0;
        for (index, byte) in buffer.iter().enumerate() {
            if *byte == b'\n' {
                self.line.extend_from_slice(&buffer[start..index]);
                self.flush_line(true)?;
                start = index + 1;
            }
        }
        self.line.extend_from_slice(&buffer[start..]);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.finish()
    }
}

/// One word of the human vocabulary.
///
/// The word carries the meaning. A glyph, when decoration is enabled, is
/// written immediately before the word it decorates and never instead of it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Word {
    High,
    Watch,
    Worse,
    Better,
    Changed,
    Warning,
    Next,
}

impl Word {
    const fn text(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Watch => "watch",
            Self::Worse => "worse",
            Self::Better => "better",
            Self::Changed => "changed",
            Self::Warning => "warning",
            Self::Next => "next:",
        }
    }

    const fn glyph(self) -> char {
        match self {
            Self::High => '\u{f024}',
            Self::Watch => '\u{f0eb}',
            Self::Worse => '\u{f062}',
            Self::Better => '\u{f063}',
            Self::Changed => '\u{f111}',
            Self::Warning => '\u{f071}',
            Self::Next => '\u{f46b}',
        }
    }

    fn style(self) -> Option<Style> {
        match self {
            Self::High | Self::Worse => Some(Style::new().fg_color(Some(AnsiColor::Red.into()))),
            Self::Watch | Self::Warning => {
                Some(Style::new().fg_color(Some(Ansi256Color(208).into())))
            }
            Self::Next => Some(Style::new().fg_color(Some(AnsiColor::Cyan.into()))),
            Self::Better => Some(Style::new().fg_color(Some(AnsiColor::Green.into()))),
            Self::Changed => None,
        }
    }

    const fn rating(rating: Rating) -> Self {
        match rating {
            Rating::High => Self::High,
            Rating::Watch | Rating::Healthy => Self::Watch,
        }
    }

    const fn direction(direction: ComparisonDirection) -> Self {
        match direction {
            ComparisonDirection::Worse => Self::Worse,
            ComparisonDirection::Better => Self::Better,
            ComparisonDirection::Changed => Self::Changed,
        }
    }
}

/// The tier-colored verdict bar, which is decoration only.
const TIER_BAR: char = '\u{258c}';

fn codebase_tier_style(tier: CodebaseTier) -> Option<Style> {
    match tier {
        CodebaseTier::Empty => None,
        CodebaseTier::Clean | CodebaseTier::Solid => {
            Some(Style::new().fg_color(Some(AnsiColor::Green.into())))
        }
        CodebaseTier::Worn => Some(Style::new().fg_color(Some(Ansi256Color(208).into()))),
        CodebaseTier::FightsBack | CodebaseTier::Lost => {
            Some(Style::new().fg_color(Some(AnsiColor::Red.into())))
        }
    }
}

fn diff_tier_style(tier: DiffTier) -> Option<Style> {
    match tier {
        DiffTier::NoDebtChange => None,
        DiffTier::Better => Some(Style::new().fg_color(Some(AnsiColor::Green.into()))),
        DiffTier::Worse => Some(Style::new().fg_color(Some(AnsiColor::Red.into()))),
        DiffTier::Mixed => Some(Style::new().fg_color(Some(Ansi256Color(208).into()))),
    }
}

/// One completed row, joined once so the renderer only writes it.
#[derive(Clone, Debug, Default)]
struct Row {
    word: Option<Word>,
    head: String,
    /// A `path:line` written on its own line, which makes a row a card.
    location: Option<String>,
    /// Facts written together when they fit and stacked when they do not.
    facts: Vec<String>,
    /// Facts that always own their line, such as a cycle witness step.
    stacked: Vec<String>,
}

impl Row {
    fn new(word: Option<Word>, head: impl Into<String>) -> Self {
        Self {
            word,
            head: head.into(),
            ..Self::default()
        }
    }

    fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    fn with_fact(mut self, fact: impl Into<String>) -> Self {
        self.facts.push(fact.into());
        self
    }

    fn with_facts(mut self, facts: Vec<String>) -> Self {
        self.facts = facts;
        self
    }

    fn with_stacked(mut self, stacked: Vec<String>) -> Self {
        self.stacked = stacked;
        self
    }
}

/// One section of completed rows.
#[derive(Clone, Debug, Default)]
struct Section {
    heading: &'static str,
    rows: Vec<Row>,
}

impl Section {
    const fn new(heading: &'static str) -> Self {
        Self {
            heading,
            rows: Vec::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// The completed presentation the renderer writes without deciding anything.
struct Presentation {
    mode: ReportMode,
    scope_label: String,
    verdict: Verdict,
    areas: Section,
    findings: Section,
    architecture: Section,
    history: Section,
    warnings: Section,
    /// Per-file diagnostic context, kept for `--all` and path views.
    warning_detail: Vec<String>,
    next: Option<String>,
    /// Whether a clean diff suppresses every section after the verdict.
    verdict_only: bool,
}

impl Presentation {
    fn new(report: &Report, selected_scope: Option<ScopeId>, all: bool) -> Self {
        let selected = selected_scope
            .or_else(|| report.root())
            .and_then(|id| report.scopes().get(id.index()));
        let Some(selected) = selected else {
            return Self {
                mode: report.mode(),
                scope_label: terminal_path(".").to_owned(),
                verdict: Verdict::default(),
                areas: Section::new("AREAS"),
                findings: Section::new("FINDINGS"),
                architecture: Section::new("ARCHITECTURE"),
                history: Section::new("HISTORY"),
                warnings: Section::new("WARNINGS"),
                warning_detail: Vec::new(),
                next: None,
                verdict_only: false,
            };
        };
        let verdict = report.scope_verdict(selected.id());
        let displayed = descend(report, selected);
        let displayed_verdict = if displayed.id() == selected.id() {
            verdict.clone()
        } else {
            report.scope_verdict(displayed.id())
        };
        let selection = displayed_verdict.selection();
        let detail = all || selected.kind() != ScopeKind::Repository;
        let verdict_only = report.mode() == ReportMode::Diff
            && verdict.diff_tier() == Some(DiffTier::NoDebtChange);

        let areas = area_rows(report, displayed);
        let findings = match report.mode() {
            ReportMode::Codebase => codebase_finding_rows(report, displayed, all),
            ReportMode::Diff => diff_finding_rows(report, displayed, all, selection),
        };
        let architecture = architecture_rows(report, selected, all, detail, verdict.selection());
        let history = history_rows(report, selected, detail, verdict.selection());
        let (warnings, warning_detail) = warning_rows(report, selected, detail);
        let next = (report.mode() == ReportMode::Codebase)
            .then(|| drill_path_from_visible(report, selected, areas.first()))
            .flatten()
            .map(|path| format!("smackdebt {}", path.to_string_lossy()));

        let mut area_section = Section::new("AREAS");
        if areas.len() >= 2 {
            area_section.rows = areas
                .iter()
                .take(5)
                .map(|area| area_row(report, area))
                .collect();
        }
        Self {
            mode: report.mode(),
            scope_label: terminal_path(selected.name()).to_owned(),
            verdict,
            areas: area_section,
            findings,
            architecture,
            history,
            warnings,
            warning_detail,
            next,
            verdict_only,
        }
    }
}

/// Walks through collapsed single-child scopes so a repository whose only
/// package is its root still shows that package's children.
fn descend<'a>(report: &'a Report, selected: &'a Scope) -> &'a Scope {
    let mut displayed = selected;
    while displayed.kind() != ScopeKind::File && displayed.children().len() == 1 {
        displayed = &report.scopes()[displayed.children()[0].index()];
    }
    displayed
}

fn area_rows<'a>(report: &'a Report, displayed: &'a Scope) -> Vec<&'a Scope> {
    let mut areas = display_children(report, displayed);
    match report.mode() {
        ReportMode::Codebase => {
            areas.sort_by(|left, right| codebase_child_order(left, right));
            areas.retain(|area| area.health().debt() > 0);
        }
        ReportMode::Diff => {
            areas.sort_by(|left, right| diff_child_order(report, left, right));
            areas.retain(|area| debt_movement(report, area).total() > 0);
        }
    }
    areas
}

/// The debt one child area moved, which excludes the healthy units a refactor
/// adds or deletes.
fn debt_movement(report: &Report, scope: &Scope) -> smackdebt_analysis::DiffCounts {
    report.scope_verdict(scope.id()).facts().total()
}

fn area_row(report: &Report, area: &Scope) -> Row {
    let mut facts = Vec::new();
    match report.mode() {
        ReportMode::Codebase => {
            let health = area.health();
            for (value, word) in [(health.high(), Word::High), (health.watch(), Word::Watch)] {
                if value > 0 {
                    facts.push(format!("{} {}", Grouped(value as usize), word.text()));
                }
            }
        }
        ReportMode::Diff => {
            let diff = debt_movement(report, area);
            for (value, word) in [
                (diff.worse(), Word::Worse),
                (diff.better(), Word::Better),
                (diff.changed(), Word::Changed),
            ] {
                if value > 0 {
                    facts.push(format!("{} {}", word.text(), Grouped(value as usize)));
                }
            }
        }
    }
    Row::new(None, terminal_path(area.name())).with_facts(facts)
}

fn codebase_finding_rows(report: &Report, displayed: &Scope, all: bool) -> Section {
    let mut section = Section::new("FINDINGS");
    let mut findings: Vec<&Finding> = displayed
        .findings()
        .iter()
        .map(|id| &report.findings()[id.index()])
        .collect();
    if !all {
        findings.retain(|finding| finding.affects_verdict());
    }
    findings.sort_by(|left, right| finding_order(report, left, right));
    if !all && displayed.kind() != ScopeKind::File {
        findings.truncate(3);
    }
    for finding in findings {
        let file = &report.files()[finding.file().index()];
        let mut facts: Vec<String> = finding
            .assessment()
            .signals()
            .iter()
            .filter(|signal| signal.rating() != Rating::Healthy)
            // Cyclomatic complexity starts at one, so a value of one states
            // nothing and never reaches a reader.
            .filter(|signal| signal.signal() != Signal::CyclomaticComplexity || signal.value() != 1)
            .map(|signal| format!("{} {}", signal_name(signal.signal()), signal.value()))
            .collect();
        if let Some(touches) = hotspot_touches(report, finding.file()) {
            facts.push(format!(
                "hot ({})",
                Counted::new(touches as usize, "commit", "commits")
            ));
        } else if let Some(activity) = file.activity().filter(|activity| activity.touches() > 0) {
            facts.push(Counted::new(activity.touches() as usize, "commit", "commits").to_string());
        }
        section.rows.push(
            Row::new(
                Some(Word::rating(finding.assessment().rating())),
                format!(
                    "{} · {}{}",
                    unit_identity(finding.identity(), file.path()),
                    unit_kind_label(finding.identity().kind()),
                    evidence_suffix(Some(finding.role()), Some(finding.trust()))
                ),
            )
            .with_location(format!("{}:{}", file.path(), finding.span().start_line()))
            .with_facts(facts),
        );
    }
    if all {
        section.rows.extend(size_rows(report, displayed));
    }
    section
}

/// Rated file and container size findings, which measure code the unit
/// signals cannot see and therefore stay out of the ranked default view.
fn size_rows(report: &Report, displayed: &Scope) -> Vec<Row> {
    report
        .size_findings()
        .iter()
        .filter(|finding| file_belongs_to_scope(report, finding.file(), displayed))
        .map(|finding| {
            let path = report.files()[finding.file().index()].path();
            let head = match finding.container() {
                Some(container) => format!("{container} · container"),
                None => format!("{path} · file"),
            };
            Row::new(Some(Word::rating(finding.rating())), head)
                .with_location(path)
                .with_fact(format!("{} lines", Grouped(finding.value() as usize)))
        })
        .collect()
}

fn diff_finding_rows(
    report: &Report,
    displayed: &Scope,
    all: bool,
    selection: &DebtDiffSelection,
) -> Section {
    let mut section = Section::new("FINDINGS");
    // The selection already holds each debt-moving comparison exactly once,
    // so this visits every identity without a de-duplication set.
    let mut comparisons: Vec<&Comparison> = selection
        .source()
        .iter()
        .map(|id| &report.comparisons()[id.index()])
        .collect();
    comparisons.sort_by(|left, right| {
        direction_rank(left.direction())
            .cmp(&direction_rank(right.direction()))
            .then_with(|| left.identity().name().cmp(right.identity().name()))
    });
    if !all && displayed.kind() != ScopeKind::File {
        comparisons.truncate(3);
    }
    for comparison in comparisons {
        let path = comparison
            .file()
            .and_then(|file| report.files().get(file.index()))
            .map(FileRecord::path);
        let head = format!(
            "{} · {}",
            unit_identity(comparison.identity(), path.unwrap_or_default()),
            unit_kind_label(comparison.identity().kind())
        );
        let mut row = Row::new(Some(Word::direction(comparison.direction())), head);
        if let Some(path) = path {
            row = row.with_location(match comparison.span() {
                Some(span) => format!("{path}:{}", span.start_line()),
                None => path.to_owned(),
            });
        }
        section
            .rows
            .push(row.with_facts(changed_measurements(comparison)));
    }
    section
}

fn changed_measurements(comparison: &Comparison) -> Vec<String> {
    match comparison.kind() {
        ComparisonKind::Added => return vec!["added".to_owned()],
        ComparisonKind::Removed => return vec!["removed".to_owned()],
        ComparisonKind::Ambiguous => {
            return vec!["identity could not be matched safely".to_owned()];
        }
        _ => {}
    }
    let (Some(before), Some(after)) = (comparison.before(), comparison.after()) else {
        return vec![changed_summary(comparison.kind()).to_owned()];
    };
    let values = [
        (
            "cognitive",
            before.cognitive_complexity(),
            after.cognitive_complexity(),
        ),
        (
            "cyclomatic",
            before.cyclomatic_complexity(),
            after.cyclomatic_complexity(),
        ),
        ("statements", before.logical_lines(), after.logical_lines()),
        ("nesting", before.max_nesting(), after.max_nesting()),
        (
            "parameters",
            before.parameter_count(),
            after.parameter_count(),
        ),
    ];
    let facts: Vec<String> = values
        .into_iter()
        .filter(|(_, before, after)| before != after)
        .map(|(name, before, after)| format!("{name} {before} → {after}"))
        .collect();
    if facts.is_empty() {
        return vec![changed_summary(comparison.kind()).to_owned()];
    }
    facts
}

/// The human sentence a comparison falls back to when no measurement moved.
///
/// The machine name of a kind is snake_case, which is a machine word: human
/// output states what happened in the words the rest of the report uses.
const fn changed_summary(kind: ComparisonKind) -> &'static str {
    match kind {
        ComparisonKind::Added => "added",
        ComparisonKind::Removed => "removed",
        ComparisonKind::Improved => "rating improved",
        ComparisonKind::Regressed => "rating regressed",
        ComparisonKind::MetricChanged => "measurements changed",
        ComparisonKind::Ambiguous => "identity could not be matched safely",
        ComparisonKind::Unchanged => "unchanged",
    }
}

fn architecture_rows(
    report: &Report,
    selected: &Scope,
    all: bool,
    detail: bool,
    selection: &DebtDiffSelection,
) -> Section {
    let mut section = Section::new("ARCHITECTURE");
    let relationships = selected.kind() != ScopeKind::Repository;
    match report.mode() {
        ReportMode::Codebase => {
            let mut findings = selected.architecture_findings().to_vec();
            if !all {
                findings.truncate(3);
            }
            for id in findings {
                let finding = &report.architecture_findings()[id.index()];
                section.rows.push(
                    Row::new(
                        Some(Word::rating(finding.rating())),
                        architecture_finding_name(finding.kind()),
                    )
                    .with_stacked(cycle_witness_steps(report, finding.witness_edges())),
                );
            }
            section
                .rows
                .extend(stable_dependency_rows(report, selected));
        }
        ReportMode::Diff => {
            // Only the selected cycle changes move debt; every other edge
            // change is context a path view or `--all` may still show.
            for id in selection.architecture() {
                let comparison = &report.architecture_comparisons()[id.index()];
                let head = if comparison.kind() == ArchitectureComparisonKind::CycleIntroduced {
                    "package dependency cycle introduced"
                } else {
                    "package dependency cycle removed"
                };
                let witness = comparison
                    .witness()
                    .iter()
                    .enumerate()
                    .map(|(index, package)| {
                        let name = package_name(report, package.index()).unwrap_or("?");
                        if index == 0 {
                            name.to_owned()
                        } else {
                            format!("→ {name}")
                        }
                    })
                    .collect();
                section.rows.push(
                    Row::new(Some(Word::direction(comparison.direction())), head)
                        .with_stacked(witness),
                );
            }
            if detail {
                section
                    .rows
                    .extend(edge_change_rows(report, selected, selection));
            }
        }
    }
    if relationships {
        section.rows.extend(relationship_rows(report, selected));
    }
    if detail {
        section.rows.extend(unmatched_import_rows(report, selected));
    }
    section
}

/// The added and removed dependency edges of a diff, which are context rather
/// than debt movement.
fn edge_change_rows(report: &Report, selected: &Scope, selection: &DebtDiffSelection) -> Vec<Row> {
    selected
        .architecture_comparisons()
        .iter()
        .filter(|id| !selection.architecture().contains(id))
        .map(|id| &report.architecture_comparisons()[id.index()])
        .filter(|comparison| {
            matches!(
                comparison.kind(),
                ArchitectureComparisonKind::EdgeAdded | ArchitectureComparisonKind::EdgeRemoved
            )
        })
        .map(|comparison| {
            let change = if comparison.kind() == ArchitectureComparisonKind::EdgeAdded {
                "added"
            } else {
                "removed"
            };
            let source = comparison
                .files()
                .first()
                .and_then(|id| report.files().get(id.index()))
                .map_or("?", FileRecord::path);
            let target = comparison
                .files()
                .get(1)
                .and_then(|id| report.files().get(id.index()))
                .map_or("?", FileRecord::path);
            let head = if comparison.relation() == Some(StaticRelationKind::ModuleOwnership) {
                format!("{source} owns {target}")
            } else {
                format!("{source} → {target}")
            };
            // These rows are context: they never reach a `changed n` count, so
            // they never wear a verdict word either, exactly like the
            // relationship rows they sit beside.
            let mut row = Row::new(None, head).with_fact(change);
            for fact in evidence_facts(comparison.role(), comparison.trust()) {
                row = row.with_fact(fact);
            }
            row
        })
        .collect()
}

/// The closed witness of one cycle, one step per line so it is never
/// shortened with an ellipsis.
fn cycle_witness_steps(
    report: &Report,
    witness_edges: &[smackdebt_analysis::DependencyEdgeId],
) -> Vec<String> {
    let mut steps = Vec::new();
    let mut edges = witness_edges
        .iter()
        .filter_map(|id| report.dependency_edges().get(id.index()));
    if let Some(first) = edges.next() {
        steps.push(report.files()[first.source().index()].path().to_owned());
        steps.push(format!(
            "→ {}",
            report.files()[first.target().index()].path()
        ));
        for edge in edges {
            steps.push(format!(
                "→ {}",
                report.files()[edge.target().index()].path()
            ));
        }
    }
    steps
}

fn stable_dependency_rows(report: &Report, selected: &Scope) -> Vec<Row> {
    report
        .stable_dependency_findings()
        .iter()
        .filter(|finding| {
            finding
                .witness_edges()
                .iter()
                .filter_map(|id| report.dependency_edges().get(id.index()))
                .any(|edge| {
                    file_belongs_to_scope(report, edge.source(), selected)
                        || file_belongs_to_scope(report, edge.target(), selected)
                })
        })
        .map(|finding| {
            let evidence = finding.evidence();
            let source = package_name(report, finding.source().index()).unwrap_or("?");
            let target = package_name(report, finding.target().index()).unwrap_or("?");
            let mut row = Row::new(
                Some(Word::rating(finding.rating())),
                format!("{source} → {target}"),
            )
            .with_fact("depends on less stable code");
            if let (Some(from), Some(to)) = (
                package_instability(evidence.source()),
                package_instability(evidence.target()),
            ) {
                row = row.with_fact(format!(
                    "instability {}/{} → {}/{}",
                    from.numerator(),
                    from.denominator(),
                    to.numerator(),
                    to.denominator()
                ));
            }
            row.with_fact(
                Counted::new(evidence.references() as usize, "import", "imports").to_string(),
            )
        })
        .collect()
}

fn package_instability(
    measurement: smackdebt_analysis::PackageGraphMeasurement,
) -> Option<Instability> {
    instability(measurement.fan_in(), measurement.fan_out())
}

/// The debt-bearing relationships a selected path keeps, including those whose
/// other endpoint lies outside it.
fn relationship_rows(report: &Report, selected: &Scope) -> Vec<Row> {
    report
        .dependency_edges()
        .iter()
        .filter(|edge| {
            file_belongs_to_scope(report, edge.source(), selected)
                || file_belongs_to_scope(report, edge.target(), selected)
        })
        .map(|edge| {
            let source = report.files()[edge.source().index()].path();
            let target = report.files()[edge.target().index()].path();
            let mut row = match edge.relation() {
                StaticRelationKind::Uses => Row::new(None, format!("{source} → {target}"))
                    .with_fact(
                        Counted::new(edge.references() as usize, "import", "imports").to_string(),
                    ),
                StaticRelationKind::ModuleOwnership => {
                    Row::new(None, format!("{source} owns {target}"))
                }
            };
            for fact in evidence_facts(Some(edge.role()), Some(edge.trust())) {
                row = row.with_fact(fact);
            }
            row
        })
        .collect()
}

/// The per-file import problems the grouped warning summarises.
fn unmatched_import_rows(report: &Report, selected: &Scope) -> Vec<Row> {
    report
        .resolution_diagnostics()
        .iter()
        .filter(|value| file_belongs_to_scope(report, value.file(), selected))
        .map(|diagnostic| {
            let source = report.files()[diagnostic.file().index()].path();
            let status = match diagnostic.kind() {
                ResolutionIssueKind::Unresolved => "could not be matched",
                ResolutionIssueKind::Ambiguous => "matched more than one file",
            };
            let mut row = Row::new(
                None,
                format!(
                    "{source}:{} → {}",
                    diagnostic.span().start_line(),
                    diagnostic.target()
                ),
            )
            .with_fact(status);
            for fact in evidence_facts(Some(diagnostic.role()), Some(diagnostic.trust())) {
                row = row.with_fact(fact);
            }
            row
        })
        .collect()
}

fn history_rows(
    report: &Report,
    selected: &Scope,
    detail: bool,
    selection: &DebtDiffSelection,
) -> Section {
    let mut section = Section::new("HISTORY");
    let relevant_packages = report
        .files()
        .iter()
        .filter(|file| file_belongs_to_scope(report, file.id(), selected))
        .filter_map(FileRecord::package)
        .collect::<std::collections::BTreeSet<_>>();

    let mut couplings = selected
        .evolutionary_findings()
        .iter()
        .map(|id| report.evolutionary_findings()[id.index()].coupling())
        .collect::<Vec<_>>();
    couplings.sort_by(|left, right| {
        Reverse(left.shared_commits())
            .cmp(&Reverse(right.shared_commits()))
            .then_with(|| right.similarity().total_cmp(&left.similarity()))
            .then_with(|| {
                package_name(report, left.left().index())
                    .cmp(&package_name(report, right.left().index()))
            })
            .then_with(|| {
                package_name(report, left.right().index())
                    .cmp(&package_name(report, right.right().index()))
            })
            .then_with(|| left.left().cmp(&right.left()))
            .then_with(|| left.right().cmp(&right.right()))
    });
    // A path view or `--all` also states the strong coupling a code
    // dependency already explains; weak coupling stays in JSON.
    let context: Vec<_> = if detail {
        report
            .change_coupling()
            .iter()
            .copied()
            .filter(|pair| qualifies_for_finding(*pair))
            .filter(|pair| {
                relevant_packages.contains(&pair.left())
                    || relevant_packages.contains(&pair.right())
            })
            .filter(|pair| !couplings.iter().any(|finding| same_pair(*finding, *pair)))
            .collect()
    } else {
        Vec::new()
    };
    for (pair, finding) in couplings
        .iter()
        .copied()
        .map(|pair| (pair, true))
        .chain(context.into_iter().map(|pair| (pair, false)))
    {
        let left = package_name(report, pair.left().index()).unwrap_or("?");
        let right = package_name(report, pair.right().index()).unwrap_or("?");
        section.rows.push(
            Row::new(
                finding.then_some(Word::Watch),
                format!(
                    "{left} ↔ {right} changed together in {} of {} commits",
                    pair.shared_commits(),
                    pair.union_commits()
                ),
            )
            .with_fact(format!("{}%", (pair.similarity() * 100.0).round() as u32))
            .with_fact(if coupling_has_code_dependency(report, pair) {
                "code dependency exists"
            } else {
                "no code dependency"
            }),
        );
    }

    // One package states its concentration once, however many source roles
    // contributed to it.
    let mut stated = std::collections::BTreeSet::new();
    for finding in report.knowledge_concentration_findings() {
        let concentration = finding.concentration();
        if !relevant_packages.contains(&concentration.package())
            || !stated.insert(concentration.package())
        {
            continue;
        }
        let package = package_name(report, concentration.package().index()).unwrap_or("?");
        section.rows.push(Row::new(
            Some(Word::rating(finding.rating())),
            format!(
                "one contributor made {} of {} commits to {package}",
                Grouped(concentration.numerator() as usize),
                Grouped(concentration.denominator() as usize)
            ),
        ));
    }
    // The concise view states at most three actionable history rows; a path
    // view and `--all` keep the relevant context they exist to show.
    if !detail {
        section.rows.truncate(3);
    }

    if report.mode() == ReportMode::Diff {
        for id in selection.evolutionary() {
            let comparison = report.evolutionary_comparisons()[id.index()];
            let pair = comparison.coupling();
            let left = package_name(report, pair.left().index()).unwrap_or("?");
            let right = package_name(report, pair.right().index()).unwrap_or("?");
            let outcome = match comparison.kind() {
                smackdebt_analysis::EvolutionaryComparisonKind::FindingIntroduced => {
                    "now change together without a code dependency"
                }
                smackdebt_analysis::EvolutionaryComparisonKind::FindingRemoved => {
                    "no longer change together without a code dependency"
                }
            };
            section.rows.push(Row::new(
                Some(Word::direction(comparison.direction())),
                format!("{left} ↔ {right} {outcome}"),
            ));
        }
    }
    section
}

fn warning_rows(report: &Report, selected: &Scope, detail: bool) -> (Section, Vec<String>) {
    let mut section = Section::new("WARNINGS");
    let mut warnings = Vec::new();
    let history = report.history_coverage();
    match history.availability() {
        smackdebt_analysis::HistoryAvailability::Incomplete => {
            warnings.push("History is incomplete.".to_owned());
        }
        smackdebt_analysis::HistoryAvailability::Unavailable => {
            warnings.push("History is unavailable.".to_owned());
        }
        smackdebt_analysis::HistoryAvailability::Complete => {}
    }
    if history.rename_gaps() > 0 {
        warnings.push("Some renamed files could not be matched.".to_owned());
    }
    let unfollowed = report
        .resolution_diagnostics()
        .iter()
        .filter(|diagnostic| file_belongs_to_scope(report, diagnostic.file(), selected))
        .count();
    if unfollowed > 0 {
        warnings.push(format!(
            "{} could not be followed.",
            Counted::new(unfollowed, "import", "imports")
        ));
    }
    let relevant = |diagnostic: &Diagnostic| {
        diagnostic
            .file()
            .is_none_or(|file| file_belongs_to_scope(report, file, selected))
    };
    for kind in [
        DiagnosticKind::UnsupportedLanguage,
        DiagnosticKind::UnreadableFile,
        DiagnosticKind::OversizedFile,
        DiagnosticKind::ParseFailure,
        DiagnosticKind::AmbiguousIdentity,
        DiagnosticKind::UnsafeReference,
        DiagnosticKind::Other,
    ] {
        if !detail {
            continue;
        }
        let count = report
            .diagnostics()
            .iter()
            .filter(|diagnostic| relevant(diagnostic) && diagnostic.kind() == kind)
            .filter(|diagnostic| !diagnostic.message().starts_with("Git history"))
            .count();
        if count > 0 {
            warnings.push(diagnostic_summary(kind, count));
        }
    }
    section.rows = warnings
        .into_iter()
        .map(|warning| Row::new(Some(Word::Warning), warning))
        .collect();
    let mut warning_detail = Vec::new();
    if detail {
        for diagnostic in report
            .diagnostics()
            .iter()
            .filter(|diagnostic| relevant(diagnostic))
            .filter(|diagnostic| !diagnostic.message().starts_with("Git history"))
        {
            if let Some(file) = diagnostic.file() {
                warning_detail.push(format!(
                    "{}: {}",
                    report.files()[file.index()].path(),
                    diagnostic.message()
                ));
            }
        }
    }
    (section, warning_detail)
}

struct Renderer<'a, W> {
    writer: &'a mut W,
    options: TerminalOptions,
}

impl<'a, W: Write> Renderer<'a, W> {
    const fn new(writer: &'a mut W, options: TerminalOptions) -> Self {
        Self { writer, options }
    }

    fn write(mut self, view: &Presentation) -> io::Result<()> {
        self.write_verdict(view)?;
        if view.verdict_only {
            return Ok(());
        }
        for section in [
            &view.areas,
            &view.findings,
            &view.architecture,
            &view.history,
        ] {
            self.write_section(section)?;
        }
        if !view.warnings.is_empty() {
            self.write_section(&view.warnings)?;
            for detail in &view.warning_detail {
                self.write_indented(2, detail)?;
            }
        }
        if let Some(next) = &view.next {
            writeln!(self.writer)?;
            self.write_head(Some(Word::Next), next)?;
        }
        Ok(())
    }

    fn write_verdict(&mut self, view: &Presentation) -> io::Result<()> {
        let scope = match view.mode {
            ReportMode::Codebase => format!("smackdebt · {}", view.scope_label),
            ReportMode::Diff => format!("smackdebt diff · {}", view.scope_label),
        };
        self.write_text(&scope, 0)?;
        let style = match view.verdict.diff_tier() {
            Some(tier) => diff_tier_style(tier),
            None => codebase_tier_style(view.verdict.tier()),
        };
        // The two cells the bar occupies are reserved either way, so a
        // decorated and an undecorated report break their lines identically.
        if self.options.decorations {
            self.write_decoration(TIER_BAR, style)?;
            write!(self.writer, " ")?;
        } else {
            write!(self.writer, "  ")?;
        }
        self.write_text(view.verdict.sentence(), 2)?;
        self.write_text(&verdict_counts(&view.verdict, view.mode), 0)?;
        if let Some(worst) = view.verdict.worst_offender()
            && view.verdict.diff_tier() != Some(DiffTier::NoDebtChange)
        {
            self.write_text(
                &format!("worst: {} — {}", worst.path(), worst.reason().text()),
                0,
            )?;
        }
        Ok(())
    }

    fn write_section(&mut self, section: &Section) -> io::Result<()> {
        if section.is_empty() {
            return Ok(());
        }
        writeln!(self.writer)?;
        writeln!(self.writer, "{}", section.heading)?;
        for row in &section.rows {
            self.write_row(row)?;
        }
        Ok(())
    }

    /// Writes one row aligned when its own content fits and stacked when it
    /// does not, so no fact is ever shortened away.
    fn write_row(&mut self, row: &Row) -> io::Result<()> {
        let card = row.location.is_some() || !row.stacked.is_empty();
        let lead = self.lead_width(row.word);
        let inline = joined(&row.head, &row.facts);
        if !card && lead + UnicodeWidthStr::width(inline.as_str()) <= self.options.width {
            return self.write_head(row.word, &inline);
        }
        self.write_head(row.word, &row.head)?;
        if let Some(location) = &row.location {
            self.write_indented(INDENT, location)?;
        }
        if !row.facts.is_empty() {
            let facts = row.facts.join(" · ");
            if INDENT + UnicodeWidthStr::width(facts.as_str()) <= self.options.width {
                self.write_indented(INDENT, &facts)?;
            } else {
                for fact in &row.facts {
                    self.write_indented(INDENT, fact)?;
                }
            }
        }
        for stacked in &row.stacked {
            self.write_indented(INDENT, stacked)?;
        }
        Ok(())
    }

    /// The visible cells a row's lead occupies before its head text.
    ///
    /// A decorated glyph fills the same two cells an undecorated report leaves
    /// blank, so both reports choose the same shape for every row.
    fn lead_width(&self, word: Option<Word>) -> usize {
        match word {
            None => 2,
            Some(word) => 2 + UnicodeWidthStr::width(word.text()) + 1,
        }
    }

    /// Writes one row head: its optional decorated word, then its text.
    fn write_head(&mut self, word: Option<Word>, text: &str) -> io::Result<()> {
        match word {
            None => write!(self.writer, "  ")?,
            Some(word) => {
                if self.options.decorations {
                    self.write_decoration(word.glyph(), word.style())?;
                    write!(self.writer, " ")?;
                } else {
                    write!(self.writer, "  ")?;
                }
                write!(self.writer, "{} ", word.text())?;
            }
        }
        self.write_text(text, self.lead_width(word))
    }

    fn write_indented(&mut self, indent: usize, text: &str) -> io::Result<()> {
        write!(self.writer, "{}", " ".repeat(indent))?;
        self.write_text(text, indent)
    }

    /// Writes `text` after `lead` cells that are already on the line,
    /// continuing on indented lines instead of clipping.
    fn write_text(&mut self, text: &str, lead: usize) -> io::Result<()> {
        let continuation = INDENT.max(lead);
        let lines = wrap(
            text,
            self.options.width.saturating_sub(lead).max(1),
            self.options.width.saturating_sub(continuation).max(1),
        );
        for (index, line) in lines.iter().enumerate() {
            if index > 0 {
                write!(self.writer, "{}", " ".repeat(continuation))?;
            }
            writeln!(self.writer, "{line}")?;
        }
        Ok(())
    }

    fn write_decoration(&mut self, value: char, style: Option<Style>) -> io::Result<()> {
        match style.filter(|_| self.options.color) {
            Some(style) => write!(
                self.writer,
                "{}{value}{}",
                style.render(),
                style.render_reset()
            ),
            None => write!(self.writer, "{value}"),
        }
    }
}

fn joined(head: &str, facts: &[String]) -> String {
    if facts.is_empty() {
        return head.to_owned();
    }
    format!("{head} · {}", facts.join(" · "))
}

/// The verdict counts, every one labeled with its word and printed even when
/// it is zero.
fn verdict_counts(verdict: &Verdict, mode: ReportMode) -> String {
    match mode {
        ReportMode::Codebase => {
            let counts = verdict.counts();
            format!(
                "{} high · {} watch · {} checked",
                Grouped(counts.high() as usize),
                Grouped(counts.watch() as usize),
                Grouped(counts.checked() as usize)
            )
        }
        ReportMode::Diff => {
            let facts = verdict.facts();
            let total = facts.total();
            [
                (Word::Worse, total.worse()),
                (Word::Better, total.better()),
                (Word::Changed, total.changed()),
            ]
            .into_iter()
            .map(|(word, value)| {
                let families: Vec<&str> = DebtFamily::ALL
                    .into_iter()
                    .filter(|family| match word {
                        Word::Worse => facts.counts(*family).worse() > 0,
                        Word::Better => facts.counts(*family).better() > 0,
                        _ => facts.counts(*family).changed() > 0,
                    })
                    .map(DebtFamily::name)
                    .collect();
                if families.is_empty() {
                    format!("{} {}", word.text(), Grouped(value as usize))
                } else {
                    format!(
                        "{} {} ({})",
                        word.text(),
                        Grouped(value as usize),
                        families.join(", ")
                    )
                }
            })
            .collect::<Vec<_>>()
            .join(" · ")
        }
    }
}

/// Breaks `text` so no line exceeds its available width and no fact is lost.
fn wrap(text: &str, first: usize, continuation: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut remainder = text;
    let mut available = first;
    while !remainder.is_empty() {
        if UnicodeWidthStr::width(remainder) <= available {
            lines.push(remainder.to_owned());
            break;
        }
        let split = break_point(remainder, available);
        let (line, rest) = remainder.split_at(split);
        lines.push(line.trim_end().to_owned());
        remainder = rest.trim_start();
        available = continuation;
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// The byte offset that fills at most `available` cells.
///
/// A word boundary is preferred, then a path separator, so a long path
/// continues on the next line instead of splitting a line number or a count.
fn break_point(value: &str, available: usize) -> usize {
    let mut used = 0;
    let mut limit = value.len();
    let mut space = None;
    let mut slash = None;
    for (offset, character) in value.char_indices() {
        let width = character.width().unwrap_or(0);
        if used + width > available {
            limit = offset;
            break;
        }
        used += width;
        match character {
            ' ' => space = Some(offset + character.len_utf8()),
            '/' => slash = Some(offset + character.len_utf8()),
            _ => {}
        }
    }
    let usable = |offset: &usize| *offset > 0 && *offset < limit;
    // The later boundary packs more onto the line and keeps an arrow with the
    // path it points at.
    match space.filter(usable).max(slash.filter(usable)) {
        Some(offset) => offset,
        None => limit.max(first_char_len(value)),
    }
}

fn first_char_len(value: &str) -> usize {
    value.chars().next().map_or(1, char::len_utf8)
}

fn diagnostic_summary(kind: DiagnosticKind, count: usize) -> String {
    let subject = Counted::new(count, "source file", "source files");
    match kind {
        DiagnosticKind::UnsupportedLanguage => format!("{subject} use unsupported languages."),
        DiagnosticKind::UnreadableFile => format!("{subject} could not be read."),
        DiagnosticKind::OversizedFile => format!("{subject} are too large to inspect."),
        DiagnosticKind::ParseFailure => format!("{subject} could not be fully parsed."),
        DiagnosticKind::AmbiguousIdentity => {
            format!("{subject} contain code that could not be matched.")
        }
        DiagnosticKind::UnsafeReference => format!("{subject} contain unsafe references."),
        DiagnosticKind::Other => format!("{subject} could not be analyzed."),
    }
}

fn architecture_finding_name(kind: ArchitectureFindingKind) -> &'static str {
    match kind {
        ArchitectureFindingKind::PackageCycle => "package dependency cycle",
        ArchitectureFindingKind::FileCycle => "file dependency cycle",
    }
}

fn file_belongs_to_scope(report: &Report, file: FileId, selected: &Scope) -> bool {
    let Some(record) = report.files().get(file.index()) else {
        return false;
    };
    let mut scope = Some(record.scope());
    while let Some(id) = scope {
        if id == selected.id() {
            return true;
        }
        scope = report.scopes().get(id.index()).and_then(Scope::parent);
    }
    false
}

fn hotspot_touches(report: &Report, file: FileId) -> Option<u32> {
    report
        .hotspots()
        .binary_search_by_key(&file, |hotspot| hotspot.file())
        .ok()
        .map(|index| report.hotspots()[index].touches())
}

fn package_name(report: &Report, package_index: usize) -> Option<&str> {
    report
        .packages()
        .get(package_index)
        .map(|package| terminal_path(package.path()))
}

fn terminal_path(path: &str) -> &str {
    if path == "." { "repository root" } else { path }
}

/// The human identity of one unit.
///
/// A generated internal identity such as `<closure 1177>` never reaches a
/// reader: the file name and the unit kind name it instead.
fn unit_identity(identity: &smackdebt_analysis::UnitIdentity, path: &str) -> String {
    let name = identity.name();
    if name.starts_with('<') && name.ends_with('>') {
        return file_name(path).to_owned();
    }
    match identity.container() {
        Some(container) => format!("{container}::{name}"),
        None => name.to_owned(),
    }
}

fn file_name(path: &str) -> &str {
    match path.rsplit('/').next() {
        Some(name) if !name.is_empty() => name,
        _ => "file",
    }
}

fn unit_kind_label(kind: UnitKind) -> &'static str {
    match kind {
        UnitKind::Function => "function",
        UnitKind::Method => "method",
        UnitKind::Closure => "closure",
        UnitKind::Lambda => "lambda",
        UnitKind::SyntheticTopLevel => "top level",
        UnitKind::Template => "template",
    }
}

/// Whether two rows describe the same unordered package pair.
fn same_pair(
    left: smackdebt_analysis::ChangeCoupling,
    right: smackdebt_analysis::ChangeCoupling,
) -> bool {
    (left.left(), left.right()) == (right.left(), right.right())
        || (left.left(), left.right()) == (right.right(), right.left())
}

fn coupling_has_code_dependency(
    report: &Report,
    coupling: smackdebt_analysis::ChangeCoupling,
) -> bool {
    report.package_edges().iter().any(|edge| {
        (edge.source() == coupling.left() && edge.target() == coupling.right())
            || (edge.source() == coupling.right() && edge.target() == coupling.left())
    })
}

fn history_role_name(role: SourceRole) -> &'static str {
    match role {
        SourceRole::Primary => "primary",
        SourceRole::Test => "test",
        SourceRole::Example => "example",
        SourceRole::Benchmark => "benchmark",
        SourceRole::Fixture => "fixture",
        SourceRole::Generated => "generated",
    }
}

fn evidence_facts(role: Option<SourceRole>, trust: Option<SourceTrust>) -> Vec<String> {
    let mut facts = Vec::new();
    if let Some(role) = role.filter(|role| *role != SourceRole::Primary) {
        facts.push(history_role_name(role).to_owned());
    }
    if trust == Some(SourceTrust::Advisory) {
        facts.push("advisory".to_owned());
    }
    facts
}

fn evidence_suffix(role: Option<SourceRole>, trust: Option<SourceTrust>) -> String {
    let facts = evidence_facts(role, trust);
    if facts.is_empty() {
        return String::new();
    }
    format!(" · {}", facts.join(" · "))
}

struct Grouped(usize);

impl fmt::Display for Grouped {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let digits = self.0.to_string();
        let first = digits.len() % 3;
        if first > 0 {
            formatter.write_str(&digits[..first])?;
        }
        for (index, chunk) in digits.as_bytes()[first..].chunks(3).enumerate() {
            if first > 0 || index > 0 {
                formatter.write_str(",")?;
            }
            formatter.write_str(std::str::from_utf8(chunk).map_err(|_| fmt::Error)?)?;
        }
        Ok(())
    }
}

struct Counted {
    count: usize,
    singular: &'static str,
    plural: &'static str,
}

impl Counted {
    const fn new(count: usize, singular: &'static str, plural: &'static str) -> Self {
        Self {
            count,
            singular,
            plural,
        }
    }
}

impl fmt::Display for Counted {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {}",
            Grouped(self.count),
            if self.count == 1 {
                self.singular
            } else {
                self.plural
            }
        )
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

fn signal_name(signal: Signal) -> &'static str {
    match signal {
        Signal::CognitiveComplexity => "cognitive",
        Signal::CyclomaticComplexity => "cyclomatic",
        Signal::LogicalLines => "statements",
        Signal::MaxNesting => "nesting",
        Signal::ParameterCount => "parameters",
    }
}

fn direction_rank(direction: ComparisonDirection) -> u8 {
    match direction {
        ComparisonDirection::Worse => 0,
        ComparisonDirection::Better => 1,
        ComparisonDirection::Changed => 2,
    }
}

pub(super) fn direction_name(direction: ComparisonDirection) -> &'static str {
    match direction {
        ComparisonDirection::Worse => "worse",
        ComparisonDirection::Better => "better",
        ComparisonDirection::Changed => "changed",
    }
}

fn finding_order(report: &Report, left: &Finding, right: &Finding) -> std::cmp::Ordering {
    let left_file = &report.files()[left.file().index()];
    let right_file = &report.files()[right.file().index()];
    smackdebt_analysis::FindingRank::new(
        left,
        report.is_hotspot(left.file()),
        left_file
            .activity()
            .map_or(0, |activity| activity.touches()),
        left_file.path(),
    )
    .cmp(&smackdebt_analysis::FindingRank::new(
        right,
        report.is_hotspot(right.file()),
        right_file
            .activity()
            .map_or(0, |activity| activity.touches()),
        right_file.path(),
    ))
}

fn drill_path_from_visible(
    report: &Report,
    selected: &Scope,
    child: Option<&&Scope>,
) -> Option<PathBuf> {
    let child = child?;
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

fn codebase_child_order(left: &Scope, right: &Scope) -> std::cmp::Ordering {
    right
        .health()
        .high()
        .cmp(&left.health().high())
        .then_with(|| right.health().watch().cmp(&left.health().watch()))
        .then_with(|| left.name().cmp(right.name()))
}

fn diff_child_order(report: &Report, left: &Scope, right: &Scope) -> std::cmp::Ordering {
    let left_counts = debt_movement(report, left);
    let right_counts = debt_movement(report, right);
    right_counts
        .worse()
        .cmp(&left_counts.worse())
        .then_with(|| right_counts.better().cmp(&left_counts.better()))
        .then_with(|| right_counts.changed().cmp(&left_counts.changed()))
        .then_with(|| left.name().cmp(right.name()))
}

fn ansi_display_width(value: &str) -> usize {
    let mut width = 0;
    let mut index = 0;
    while index < value.len() {
        if let Some(end) = ansi_sequence_end(value, index) {
            index = end;
            continue;
        }
        let character = value[index..].chars().next().expect("valid character");
        width += character.width().unwrap_or(0);
        index += character.len_utf8();
    }
    width
}

fn truncate_ansi_end(value: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let target = width - 1;
    let mut output = String::new();
    let mut visible = 0;
    let mut index = 0;
    let mut styled = false;
    while index < value.len() {
        if let Some(end) = ansi_sequence_end(value, index) {
            let sequence = &value[index..end];
            output.push_str(sequence);
            styled = sequence != "\u{1b}[0m";
            index = end;
            continue;
        }
        let character = value[index..].chars().next().expect("valid character");
        let character_width = character.width().unwrap_or(0);
        if visible + character_width > target {
            break;
        }
        output.push(character);
        visible += character_width;
        index += character.len_utf8();
    }
    if styled {
        output.push_str("\u{1b}[0m");
    }
    output.push('…');
    output
}

fn ansi_sequence_end(value: &str, index: usize) -> Option<usize> {
    let bytes = value.as_bytes();
    if bytes.get(index) != Some(&0x1b) || bytes.get(index + 1) != Some(&b'[') {
        return None;
    }
    bytes[index + 2..]
        .iter()
        .position(|byte| byte.is_ascii_alphabetic())
        .map(|offset| index + 3 + offset)
}

pub(super) fn rating_name(rating: Rating) -> &'static str {
    match rating {
        Rating::Healthy => "healthy",
        Rating::Watch => "watch",
        Rating::High => "high",
    }
}

pub(super) fn mode_name(mode: ReportMode) -> &'static str {
    match mode {
        ReportMode::Codebase => "codebase",
        ReportMode::Diff => "diff",
    }
}

pub(super) fn scope_kind(kind: ScopeKind) -> &'static str {
    match kind {
        ScopeKind::Repository => "repository",
        ScopeKind::Package => "package",
        ScopeKind::Directory => "directory",
        ScopeKind::File => "file",
    }
}

pub(super) fn language_name(language: Language) -> &'static str {
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

pub(super) fn diagnostic_name(kind: DiagnosticKind) -> &'static str {
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

pub(super) fn comparison_name(kind: ComparisonKind) -> &'static str {
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
    use crate::json::write_json;
    use smackdebt_analysis::{
        ArchitectureComparison, ArchitectureComparisonId, ArchitectureFinding,
        ArchitectureFindingId, ArchitectureGraph, ArchitectureReportFacts, ChangeCoupling,
        ComparisonId, Coverage, DependencyCoverage, DependencyEdge, DependencyEdgeId,
        EvolutionaryFinding, EvolutionaryFindingId, EvolutionaryReportFacts, FileActivity, FileId,
        FileRecord, FindingId, HealthCounts, HealthPolicy, HistoryCoverage, Measurements,
        PackageId, PackageRecord, ParseStatus, Report, ReportBuilder, ReportMode, Scope, ScopeId,
        SourceRole, SourceSpan, SourceTrust, UnitIdentity, UnitKind,
    };

    /// Every private-use codepoint, which may never reach a machine consumer.
    fn private_use(value: &str) -> Vec<char> {
        value
            .chars()
            .filter(|character| ('\u{e000}'..='\u{f8ff}').contains(character))
            .collect()
    }

    fn render(report: &Report, options: TerminalOptions) -> String {
        let mut bytes = Vec::new();
        write_terminal(&mut bytes, report, report.root(), options).unwrap();
        String::from_utf8(bytes).unwrap()
    }

    fn report_with_findings(activity: bool) -> Report {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        builder.add_scope(Scope::new(root, ScopeKind::Repository, ".", None));
        builder.set_root(root);
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
            builder.add_file(file);
            let measurements = Measurements::new(15, 1, 1);
            builder.add_finding(Finding::new(
                FindingId::from_index(index),
                file_id,
                UnitIdentity::new(format!("unit-{index}"), UnitKind::Function),
                SourceSpan::new(1, 2),
                measurements,
                policy.assess(measurements),
            ));
            builder.link_finding(root, FindingId::from_index(index));
        }
        builder.finish()
    }

    /// A diff report whose only movement is the given architecture change.
    fn diff_report(cycle: Option<ArchitectureComparisonKind>) -> Report {
        let mut builder = ReportBuilder::new(ReportMode::Diff);
        let root = ScopeId::from_index(0);
        let file_scope = ScopeId::from_index(1);
        let mut root_scope = Scope::new(root, ScopeKind::Repository, ".", None);
        root_scope.add_child(file_scope);
        builder.add_scope(root_scope);
        builder.add_scope(Scope::new(
            file_scope,
            ScopeKind::File,
            "api/main.rs",
            Some(root),
        ));
        builder.set_root(root);
        builder.set_packages(vec![
            PackageRecord::current(PackageId::from_index(0), file_scope, "api"),
            PackageRecord::current(PackageId::from_index(1), file_scope, "core"),
        ]);
        let file = FileId::from_index(0);
        builder.add_file(FileRecord::new(
            file,
            file_scope,
            "api/main.rs",
            Coverage::new(1, 1, 0, 0, 10, 0),
            HealthCounts::new(1, 0, 0),
        ));
        builder.link_file(file_scope, file);
        // An unchanged rated unit moves no debt on its own.
        let measurements = Measurements::new(1, 1, 1);
        builder.add_comparison(
            Comparison::new(
                ComparisonId::from_index(0),
                UnitIdentity::new("work", UnitKind::Function),
                ComparisonKind::Unchanged,
                Some(measurements),
                Some(measurements),
                Some(Rating::High),
                Some(Rating::High),
            )
            .with_file(file)
            .with_span(SourceSpan::new(7, 9)),
        );
        builder.link_comparison(file_scope, ComparisonId::from_index(0));
        if let Some(kind) = cycle {
            builder.set_architecture(ArchitectureReportFacts::new(
                ArchitectureGraph::new(
                    DependencyCoverage::new(1, 0, 0, 0, 0, 0, 0),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                ),
                Vec::new(),
                vec![
                    ArchitectureComparison::new(
                        ArchitectureComparisonId::from_index(0),
                        kind,
                        vec![PackageId::from_index(0), PackageId::from_index(1)],
                    )
                    .with_witness(vec![
                        PackageId::from_index(0),
                        PackageId::from_index(1),
                        PackageId::from_index(0),
                    ]),
                    // An edge change is context: it never joins the selection.
                    ArchitectureComparison::new(
                        ArchitectureComparisonId::from_index(1),
                        ArchitectureComparisonKind::EdgeAdded,
                        vec![PackageId::from_index(0), PackageId::from_index(1)],
                    ),
                ],
            ));
            builder
                .link_architecture_comparison(file_scope, ArchitectureComparisonId::from_index(0));
            builder
                .link_architecture_comparison(file_scope, ArchitectureComparisonId::from_index(1));
        }
        builder.finish()
    }

    #[test]
    fn every_word_keeps_its_exact_text_and_one_cell_glyph() {
        let vocabulary = [
            (Word::High, "high", '\u{f024}'),
            (Word::Watch, "watch", '\u{f0eb}'),
            (Word::Worse, "worse", '\u{f062}'),
            (Word::Better, "better", '\u{f063}'),
            (Word::Changed, "changed", '\u{f111}'),
            (Word::Warning, "warning", '\u{f071}'),
            (Word::Next, "next:", '\u{f46b}'),
        ];
        for (word, text, glyph) in vocabulary {
            assert_eq!(word.text(), text);
            assert_eq!(word.glyph(), glyph);
            assert_eq!(word.glyph().width(), Some(1));
            assert_ne!(word.glyph(), '\u{ec3f}');
        }
        assert_eq!(TIER_BAR.width(), Some(1));
    }

    #[test]
    fn glyph_styles_have_exact_colors_and_changed_is_unstyled() {
        for (word, expected) in [
            (Word::High, "\u{1b}[31m"),
            (Word::Watch, "\u{1b}[38;5;208m"),
            (Word::Next, "\u{1b}[36m"),
            (Word::Worse, "\u{1b}[31m"),
            (Word::Better, "\u{1b}[32m"),
            (Word::Warning, "\u{1b}[38;5;208m"),
        ] {
            assert_eq!(word.style().unwrap().render().to_string(), expected);
        }
        assert!(Word::Changed.style().is_none());
    }

    #[test]
    fn undecorated_output_states_every_meaning_without_a_private_use_codepoint() {
        let report = report_with_findings(true);
        let plain = render(&report, TerminalOptions::new(100, true, false));
        assert_eq!(private_use(&plain), Vec::<char>::new(), "{plain}");
        assert!(!plain.contains(TIER_BAR));
        assert!(plain.contains("  watch unit-"), "{plain}");
        assert!(plain.contains("warning "), "{plain}");
    }

    #[test]
    fn a_decorated_glyph_sits_beside_the_word_it_decorates() {
        let report = report_with_findings(true);
        let decorated = render(
            &report,
            TerminalOptions::new(100, true, false).with_decorations(true),
        );
        assert!(decorated.contains("\u{f0eb} watch unit-"), "{decorated}");
        assert!(decorated.contains("\u{f071} warning "), "{decorated}");
        assert!(decorated.contains(TIER_BAR));
        // Removing the decoration reproduces the words-only report exactly.
        assert_eq!(
            strip_decorations(&decorated),
            render(&report, TerminalOptions::new(100, true, false))
        );
    }

    #[test]
    fn removing_ansi_from_styled_output_reproduces_plain_output() {
        let report = report_with_findings(true);
        let render_with = |color| {
            let mut bytes = Vec::new();
            write_terminal(
                &mut bytes,
                &report,
                report.root(),
                TerminalOptions::new(80, false, color).with_decorations(true),
            )
            .unwrap();
            bytes
        };
        let colored = render_with(true);
        let plain = render_with(false);
        assert!(colored.windows(2).any(|bytes| bytes == b"\x1b["));
        assert_eq!(strip_ansi(&colored), plain);
    }

    /// Removes ANSI, every private-use glyph, the tier bar, and the single
    /// space each decoration is separated from its word by.
    fn strip_decorations(value: &str) -> String {
        value
            .chars()
            .map(|character| {
                // A decoration fills the two cells an undecorated report
                // leaves blank, so removing it restores those two spaces.
                if ('\u{e000}'..='\u{f8ff}').contains(&character) || character == TIER_BAR {
                    ' '
                } else {
                    character
                }
            })
            .collect()
    }

    fn strip_ansi(value: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(value.len());
        let mut index = 0;
        while index < value.len() {
            if value[index..].starts_with(b"\x1b[") {
                index += 2;
                while index < value.len() && !(b'@'..=b'~').contains(&value[index]) {
                    index += 1;
                }
                index += usize::from(index < value.len());
            } else {
                result.push(value[index]);
                index += 1;
            }
        }
        result
    }

    #[test]
    fn machine_root_dot_is_presented_as_repository_root() {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        builder.add_scope(Scope::new(root, ScopeKind::Repository, ".", None));
        builder.set_root(root);
        let report = builder.finish();
        let output = render(&report, TerminalOptions::default());
        assert!(
            output.starts_with("smackdebt · repository root\n  Nothing was checked.\n"),
            "{output}"
        );
        assert_eq!(report.scopes()[0].name(), ".");
    }

    #[test]
    fn an_empty_scope_states_its_zero_counts_with_their_words() {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        builder.add_scope(Scope::new(root, ScopeKind::Repository, ".", None));
        builder.set_root(root);
        let report = builder.finish();
        let terminal = render(&report, TerminalOptions::default());
        assert!(
            terminal.contains("0 high · 0 watch · 0 checked"),
            "{terminal}"
        );
        assert!(!terminal.contains("FINDINGS"));
        assert!(!terminal.contains("ARCHITECTURE"));
        assert!(!terminal.contains("HISTORY"));

        let mut json = Vec::new();
        write_json(&mut json, &report, report.root()).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(value["schema_version"], 3);
        assert_eq!(value["mode"], "codebase");
    }

    #[test]
    fn a_clean_diff_writes_the_verdict_block_and_nothing_else() {
        let report = diff_report(None);
        let terminal = render(&report, TerminalOptions::new(100, true, false));
        assert_eq!(
            terminal,
            "smackdebt diff · repository root\n  No debt changed.\nworse 0 · better 0 · changed 0\n"
        );
    }

    #[test]
    fn an_introduced_cycle_makes_a_diff_worse_when_no_source_comparison_moved() {
        let report = diff_report(Some(ArchitectureComparisonKind::CycleIntroduced));
        let terminal = render(&report, TerminalOptions::new(100, false, false));
        assert!(terminal.contains("You made it worse."), "{terminal}");
        assert!(
            terminal.contains("worse 1 (architecture) · better 0 · changed 0"),
            "{terminal}"
        );
        assert!(
            terminal.contains("  worse package dependency cycle introduced"),
            "{terminal}"
        );
        assert!(terminal.contains("        api\n        → core\n        → api\n"));
    }

    #[test]
    fn an_edge_change_row_is_context_and_wears_no_verdict_word() {
        let report = diff_report(Some(ArchitectureComparisonKind::CycleIntroduced));
        let terminal = render(&report, TerminalOptions::new(100, true, false));
        let row = terminal
            .lines()
            .find(|line| line.contains("added"))
            .unwrap_or_else(|| panic!("{terminal}"));
        // The row does not count toward `changed n`, so it states no direction.
        assert!(!row.contains("changed "), "{terminal}");
        assert!(!row.contains("worse "), "{terminal}");
        assert!(!row.contains("better "), "{terminal}");
        assert!(row.starts_with("  ?"), "{terminal}");
    }

    #[test]
    fn a_comparison_without_a_moved_measurement_states_a_human_word() {
        // The machine kind is `metric_changed`; a reader is told what changed.
        assert_eq!(
            changed_summary(ComparisonKind::MetricChanged),
            "measurements changed"
        );
        assert_eq!(changed_summary(ComparisonKind::Improved), "rating improved");
        assert_eq!(
            changed_summary(ComparisonKind::Regressed),
            "rating regressed"
        );
        for kind in [
            ComparisonKind::Added,
            ComparisonKind::Removed,
            ComparisonKind::Improved,
            ComparisonKind::Regressed,
            ComparisonKind::MetricChanged,
            ComparisonKind::Ambiguous,
            ComparisonKind::Unchanged,
        ] {
            assert!(!changed_summary(kind).contains('_'), "{kind:?}");
        }
        let measurements = Measurements::new(1, 1, 1);
        let comparison = Comparison::new(
            ComparisonId::from_index(0),
            UnitIdentity::new("work", UnitKind::Function),
            ComparisonKind::MetricChanged,
            Some(measurements),
            Some(measurements),
            Some(Rating::Watch),
            Some(Rating::Watch),
        );
        assert_eq!(
            changed_measurements(&comparison),
            vec!["measurements changed".to_owned()]
        );
    }

    #[test]
    fn advisory_findings_are_visible_only_with_all() {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        let file = FileId::from_index(0);
        let finding = FindingId::from_index(0);
        builder.add_scope(Scope::new(root, ScopeKind::Repository, ".", None));
        builder.set_root(root);
        builder.add_file(
            FileRecord::new(
                file,
                root,
                "broken.py",
                Coverage::new(1, 1, 0, 0, 4, 0),
                HealthCounts::default(),
            )
            .with_source_state(SourceRole::Benchmark, ParseStatus::Recovered),
        );
        builder.add_finding(
            Finding::new(
                finding,
                file,
                UnitIdentity::new("broken", UnitKind::Function),
                SourceSpan::new(1, 4),
                Measurements::new(25, 3, 4),
                HealthPolicy::default().assess(Measurements::new(25, 3, 4)),
            )
            .with_evidence(SourceRole::Benchmark, SourceTrust::Advisory),
        );
        builder.link_finding(root, finding);
        let report = builder.finish();
        assert!(!render(&report, TerminalOptions::default()).contains("broken"));
        assert!(
            render(&report, TerminalOptions::new(100, true, false))
                .contains("  high broken · function · benchmark · advisory")
        );
    }

    #[test]
    fn architecture_cycle_stacks_its_closed_witness_without_an_ellipsis() {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        builder.add_scope(Scope::new(root, ScopeKind::Repository, ".", None));
        builder.set_root(root);
        for (index, path) in ["a.js", "b.js", "c.js"].into_iter().enumerate() {
            builder.add_file(FileRecord::new(
                FileId::from_index(index),
                root,
                path,
                Coverage::new(1, 1, 0, 0, 1, 0),
                HealthCounts::default(),
            ));
        }
        let edges = [(1, 2), (2, 0), (0, 1)]
            .into_iter()
            .enumerate()
            .map(|(index, (source, target))| {
                DependencyEdge::new(
                    DependencyEdgeId::from_index(index),
                    FileId::from_index(source),
                    FileId::from_index(target),
                    1,
                    vec![SourceSpan::new(1, 1)],
                )
            })
            .collect();
        let finding = ArchitectureFinding::new(
            ArchitectureFindingId::from_index(0),
            ArchitectureFindingKind::FileCycle,
            Vec::new(),
            vec![
                FileId::from_index(2),
                FileId::from_index(0),
                FileId::from_index(1),
            ],
            vec![
                DependencyEdgeId::from_index(0),
                DependencyEdgeId::from_index(1),
                DependencyEdgeId::from_index(2),
            ],
        );
        builder.set_architecture(ArchitectureReportFacts::new(
            ArchitectureGraph::new(
                DependencyCoverage::new(3, 0, 0, 0, 0, 0, 0),
                edges,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
            vec![finding],
            Vec::new(),
        ));
        builder.link_architecture_finding(root, ArchitectureFindingId::from_index(0));
        let report = builder.finish();
        for width in [120, 100, 80, 50] {
            let terminal = render(&report, TerminalOptions::new(width, false, false));
            assert!(
                terminal.contains("        b.js\n        → c.js\n        → a.js\n        → b.js\n"),
                "{width}: {terminal}"
            );
            assert!(!terminal.contains('…'), "{width}: {terminal}");
        }
        // Raw resolved edges never reach the repository view again.
        let detail = render(&report, TerminalOptions::new(120, true, false));
        assert!(!detail.contains("1 import"), "{detail}");
    }

    #[test]
    fn default_history_orders_actionable_findings_by_strength_before_limiting() {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        builder.add_scope(Scope::new(root, ScopeKind::Repository, ".", None));
        builder.set_root(root);
        let packages = ["a", "b", "c", "d", "e"]
            .into_iter()
            .enumerate()
            .map(|(index, name)| {
                let scope = ScopeId::from_index(index + 1);
                builder.add_scope(Scope::new(scope, ScopeKind::Package, name, Some(root)));
                PackageRecord::current(PackageId::from_index(index), scope, name)
            })
            .collect::<Vec<_>>();
        builder.set_packages(packages);
        let couplings = [(0, 1, 3, 5), (1, 2, 8, 10), (2, 3, 3, 4), (3, 4, 3, 5)]
            .into_iter()
            .map(|(left, right, shared, union)| {
                ChangeCoupling::new(
                    PackageId::from_index(left),
                    PackageId::from_index(right),
                    shared,
                    union,
                )
            })
            .collect::<Vec<_>>();
        let findings = couplings
            .iter()
            .copied()
            .enumerate()
            .map(|(index, coupling)| {
                EvolutionaryFinding::new(EvolutionaryFindingId::from_index(index), coupling)
            })
            .collect::<Vec<_>>();
        builder.set_evolution(EvolutionaryReportFacts::new(
            HistoryCoverage::unavailable("test"),
            Vec::new(),
            Vec::new(),
            couplings,
            Vec::new(),
            findings,
            Vec::new(),
        ));
        for index in 0..4 {
            builder.link_evolutionary_finding(root, EvolutionaryFindingId::from_index(index));
        }
        let report = builder.finish();

        let default = render(&report, TerminalOptions::default());
        let watch_rows = |text: &str| {
            text.lines()
                .filter(|line| line.starts_with("  watch "))
                .count()
        };
        assert_eq!(watch_rows(&default), 3, "{default}");
        assert!(
            default
                .contains("b ↔ c changed together in 8 of 10 commits · 80% · no code dependency")
        );
        assert!(
            default.contains("c ↔ d changed together in 3 of 4 commits · 75% · no code dependency")
        );
        assert!(!default.contains("d ↔ e"));
        assert!(default.find("b ↔ c").unwrap() < default.find("c ↔ d").unwrap());
        let detailed = render(&report, TerminalOptions::new(100, true, false));
        assert_eq!(watch_rows(&detailed), 4, "{detailed}");
        assert!(
            detailed
                .contains("d ↔ e changed together in 3 of 5 commits · 60% · no code dependency")
        );
    }

    #[test]
    fn activity_changes_rank_and_keeps_three_findings() {
        let report = report_with_findings(true);
        let terminal = render(&report, TerminalOptions::default());
        assert!(terminal.contains("FINDINGS"));
        assert!(terminal.contains("file-10.rs"));
        assert!(!terminal.contains("file-1.rs:"));
    }

    #[test]
    fn absent_activity_keeps_the_findings_heading() {
        let report = report_with_findings(false);
        assert!(render(&report, TerminalOptions::default()).contains("FINDINGS"));
    }

    #[test]
    fn a_generated_unit_identity_is_replaced_by_its_file_and_kind() {
        let identity = UnitIdentity::new("<closure 1177>", UnitKind::Closure);
        assert_eq!(
            unit_identity(&identity, "src/ui/GraphEditor.vue"),
            "GraphEditor.vue"
        );
        let named = UnitIdentity::new("render", UnitKind::Method).in_container("Editor");
        assert_eq!(unit_identity(&named, "src/ui/editor.ts"), "Editor::render");
    }

    #[test]
    fn every_row_fits_its_width_and_keeps_every_fact() {
        let report = report_with_findings(true);
        for width in [120, 100, 80, 50] {
            let terminal = render(&report, TerminalOptions::new(width, true, false));
            for line in terminal.lines() {
                assert!(UnicodeWidthStr::width(line) <= width, "{width}: {line}");
            }
            assert!(!terminal.contains('…'), "{width}: {terminal}");
            assert!(terminal.contains("cognitive 15"), "{width}");
        }
    }

    #[test]
    fn wrapping_keeps_a_long_path_whole_across_lines() {
        let path = "crates/some-really-long-package/src/inner/module/handler.rs:1302";
        let lines = wrap(path, 30, 30);
        assert!(lines.len() > 1);
        assert_eq!(lines.concat(), path);
        assert!(
            lines
                .iter()
                .all(|line| UnicodeWidthStr::width(line.as_str()) <= 30)
        );
    }

    #[test]
    fn grouped_counts_and_nouns_are_readable() {
        assert_eq!(Grouped(19_761).to_string(), "19,761");
        assert_eq!(Counted::new(1, "file", "files").to_string(), "1 file");
        assert_eq!(Counted::new(2, "file", "files").to_string(), "2 files");
    }

    #[test]
    fn child_ids_are_written_as_a_json_array() {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        let child = ScopeId::from_index(1);
        let mut root_scope = Scope::new(root, ScopeKind::Repository, ".", None);
        root_scope.add_child(child);
        root_scope.add_child(ScopeId::from_index(2));
        builder.add_scope(root_scope);
        builder.add_scope(Scope::new(child, ScopeKind::Package, "pkg", Some(root)));
        builder.add_scope(Scope::new(
            ScopeId::from_index(2),
            ScopeKind::Package,
            "other",
            Some(root),
        ));
        builder.set_root(root);
        let report = builder.finish();

        let mut json = Vec::new();
        write_json(&mut json, &report, report.root()).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(value["scopes"][0]["children"], serde_json::json!([1, 2]));
    }

    #[test]
    fn package_labels_come_from_the_package_table_not_scope_position() {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        let package_scope = ScopeId::from_index(1);
        builder.add_scope(Scope::new(root, ScopeKind::Repository, ".", None));
        builder.add_scope(Scope::new(
            package_scope,
            ScopeKind::Package,
            "scope-label",
            Some(root),
        ));
        builder.set_root(root);
        builder.set_packages(vec![PackageRecord::current(
            PackageId::from_index(0),
            package_scope,
            "table-label",
        )]);
        let report = builder.finish();

        assert_eq!(package_name(&report, 0), Some("table-label"));
    }

    #[test]
    fn width_writer_limits_unicode_lines_without_corrupting_glyph_styling() {
        let mut output = Vec::new();
        {
            let mut writer = WidthWriter::new(&mut output, 12);
            writeln!(
                writer,
                "\u{1b}[31m\u{f024}\u{1b}[0m  very-long-visible-line"
            )
            .unwrap();
            writer.finish().unwrap();
            assert_eq!(writer.truncations(), 1);
        }
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("\u{1b}[31m\u{f024}\u{1b}[0m"));
        assert!(output.ends_with("…\n"));
        assert!(output.lines().all(|line| ansi_display_width(line) <= 12));
    }

    #[test]
    fn drill_path_preserves_an_external_invocation_path() {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        let package = ScopeId::from_index(1);
        let child = ScopeId::from_index(2);
        let quiet = ScopeId::from_index(3);
        let mut root_scope = Scope::new(root, ScopeKind::Repository, "/work/project/bow", None);
        root_scope.add_child(package);
        builder.add_scope(root_scope);
        let mut package_scope = Scope::new(package, ScopeKind::Package, "bow", Some(root));
        package_scope.add_child(child);
        package_scope.add_child(quiet);
        builder.add_scope(package_scope);
        builder.add_scope(Scope::new(
            child,
            ScopeKind::Directory,
            "bow/src",
            Some(package),
        ));
        builder.add_scope(Scope::new(
            quiet,
            ScopeKind::Directory,
            "bow/tests",
            Some(package),
        ));
        builder.set_root(root);
        let file_id = FileId::from_index(0);
        builder.add_file(FileRecord::new(
            file_id,
            child,
            "bow/src/lib.rs",
            Coverage::new(1, 1, 0, 0, 10, 0),
            HealthCounts::new(0, 1, 0),
        ));
        builder.link_file(child, file_id);
        let report = builder.finish();

        let scopes = report.scopes();
        let path = drill_path_from_visible(
            &report,
            &scopes[package.index()],
            Some(&&scopes[child.index()]),
        );

        assert_eq!(path, Some(PathBuf::from("/work/project/bow/src")));
    }
}
