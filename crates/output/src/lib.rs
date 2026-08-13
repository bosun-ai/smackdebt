#![forbid(unsafe_code)]

//! Stable terminal and JSON views over the shared report.

use std::cmp::Reverse;
use std::fmt;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anstyle::{AnsiColor, Effects, Style};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use smackdebt_analysis::{
    Comparison, ComparisonDirection, ComparisonKind, Diagnostic, DiagnosticKind, FileRecord,
    Finding, HealthCounts, Language, Measurements, Rating, Report, ReportMode, Scope, ScopeKind,
    Signal,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Resolved terminal display choices.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalOptions {
    pub width: usize,
    pub all: bool,
    pub color: bool,
}

impl Default for TerminalOptions {
    fn default() -> Self {
        Self {
            width: 100,
            all: false,
            color: false,
        }
    }
}

/// Writes a responsive terminal report from retained report facts.
pub fn write_terminal(
    writer: &mut impl Write,
    report: &Report,
    options: TerminalOptions,
) -> io::Result<()> {
    let presentation = Presentation::new(report, options.all);
    Renderer::new(writer, options).write(&presentation)
}

#[derive(Clone, Copy)]
enum Layout {
    Full,
    Compact,
    Stacked,
}

impl Layout {
    const fn for_width(width: usize) -> Self {
        if width >= 100 {
            Self::Full
        } else if width >= 70 {
            Self::Compact
        } else {
            Self::Stacked
        }
    }

    const fn bar_width(self) -> usize {
        match self {
            Self::Full => 12,
            Self::Compact => 8,
            Self::Stacked => 10,
        }
    }
}

struct Presentation<'a> {
    report: &'a Report,
    selected: Option<&'a Scope>,
    displayed: Option<&'a Scope>,
    breadcrumbs: Vec<&'a str>,
    areas: Vec<&'a Scope>,
    quiet_areas: usize,
    omitted_areas: usize,
    findings: Vec<&'a Finding>,
    omitted_findings: usize,
    comparisons: Vec<&'a Comparison>,
    omitted_comparisons: usize,
    drill: Option<PathBuf>,
}

impl<'a> Presentation<'a> {
    fn new(report: &'a Report, all: bool) -> Self {
        let selected = report
            .selected_scope()
            .or_else(|| report.root())
            .and_then(|id| report.scopes().get(id.index()));
        let mut displayed = selected;
        let mut breadcrumbs = Vec::new();
        while let Some(scope) = displayed
            && scope.kind() != ScopeKind::File
            && scope.children().len() == 1
        {
            let child = &report.scopes()[scope.children()[0].index()];
            if child.name() != "." {
                breadcrumbs.push(child.name());
            }
            displayed = Some(child);
        }

        let mut areas = displayed.map_or_else(Vec::new, |scope| display_children(report, scope));
        let mut quiet_areas = 0;
        match report.mode() {
            ReportMode::Codebase => {
                areas.sort_by(|left, right| codebase_child_order(left, right));
                quiet_areas = areas
                    .iter()
                    .filter(|area| area.health().debt() == 0)
                    .count();
                if !all {
                    areas.retain(|area| area.health().debt() > 0);
                }
            }
            ReportMode::Diff => areas.sort_by(|left, right| diff_child_order(left, right)),
        }
        let omitted_areas = if all {
            0
        } else {
            areas.len().saturating_sub(10)
        };
        if omitted_areas > 0 {
            areas.truncate(10);
        }

        let mut findings = Vec::new();
        let mut comparisons = Vec::new();
        let mut omitted_findings = 0;
        let mut omitted_comparisons = 0;
        if let Some(scope) = displayed {
            findings = scope
                .findings()
                .iter()
                .map(|id| &report.findings()[id.index()])
                .collect();
            findings.sort_by(|left, right| finding_order(report, left, right));
            let finding_limit = if all || scope.kind() == ScopeKind::File {
                findings.len()
            } else {
                findings.len().min(3)
            };
            omitted_findings = findings.len() - finding_limit;
            findings.truncate(finding_limit);

            comparisons = scope
                .comparisons()
                .iter()
                .map(|id| &report.comparisons()[id.index()])
                .collect();
            comparisons.sort_by(|left, right| {
                direction_rank(left.direction())
                    .cmp(&direction_rank(right.direction()))
                    .then_with(|| left.identity().name().cmp(right.identity().name()))
            });
            let comparison_limit = if all || scope.kind() == ScopeKind::File {
                comparisons.len()
            } else {
                comparisons.len().min(3)
            };
            omitted_comparisons = comparisons.len() - comparison_limit;
            comparisons.truncate(comparison_limit);
        }
        let drill = if report.mode() == ReportMode::Codebase {
            selected
                .and_then(|scope| drill_path_from_visible(report, scope, areas.first().copied()))
        } else {
            None
        };
        Self {
            report,
            selected,
            displayed,
            breadcrumbs,
            areas,
            quiet_areas,
            omitted_areas,
            findings,
            omitted_findings,
            comparisons,
            omitted_comparisons,
            drill,
        }
    }
}

struct Renderer<'a, W> {
    writer: &'a mut W,
    options: TerminalOptions,
    layout: Layout,
    theme: Theme,
}

impl<'a, W: Write> Renderer<'a, W> {
    fn new(writer: &'a mut W, options: TerminalOptions) -> Self {
        Self {
            writer,
            layout: Layout::for_width(options.width),
            theme: Theme::new(options.color),
            options,
        }
    }

    fn write(mut self, view: &Presentation<'_>) -> io::Result<()> {
        self.write_header(view)?;
        if let Some(scope) = view.selected {
            match view.report.mode() {
                ReportMode::Codebase => self.write_quality(view.report, scope)?,
                ReportMode::Diff => self.write_change(scope)?,
            }
        }
        for breadcrumb in &view.breadcrumbs {
            writeln!(
                self.writer,
                "{}  ↓ {breadcrumb}{}",
                self.theme.dim.0, self.theme.dim.1
            )?;
        }
        match view.report.mode() {
            ReportMode::Codebase => self.write_codebase_areas(view)?,
            ReportMode::Diff => self.write_diff_areas(view)?,
        }
        self.write_details(view)?;
        self.write_diagnostics(view.report.diagnostics())?;
        if let Some(path) = &view.drill {
            writeln!(self.writer)?;
            self.theme
                .write(self.writer, Role::Navigation, "→ Explore")?;
            writeln!(self.writer, ": smackdebt {}", path.display())?;
        }
        Ok(())
    }

    fn write_header(&mut self, view: &Presentation<'_>) -> io::Result<()> {
        self.theme.write(self.writer, Role::Brand, "smackdebt")?;
        write!(self.writer, " · {}", mode_name(view.report.mode()))?;
        writeln!(self.writer)?;
        writeln!(self.writer, "{}", view.selected.map_or(".", Scope::name))?;
        if let Some(scope) = view.selected {
            let coverage = scope.coverage();
            match view.report.mode() {
                ReportMode::Codebase => writeln!(
                    self.writer,
                    "{} · {} · {}",
                    Counted::new(package_count(view.report, scope), "package", "packages"),
                    Counted::new(coverage.selected_files() as usize, "file", "files"),
                    Counted::new(coverage.source_lines() as usize, "line", "lines")
                ),
                ReportMode::Diff => writeln!(
                    self.writer,
                    "{} · {} analyzed",
                    Counted::new(
                        coverage.selected_files() as usize,
                        "source file changed",
                        "source files changed"
                    ),
                    Grouped(coverage.analyzed_files() as usize)
                ),
            }?;
        }
        Ok(())
    }

    fn write_quality(&mut self, _report: &Report, scope: &Scope) -> io::Result<()> {
        let health = scope.health();
        let coverage = scope.coverage();
        writeln!(self.writer)?;
        self.heading("QUALITY")?;
        writeln!(
            self.writer,
            "  {} need attention",
            DisplayPercent::new(health.debt(), health.total())
        )?;
        self.theme.write(
            self.writer,
            Role::Navigation,
            &Bar::new(health.debt(), health.total(), self.layout.bar_width()),
        )?;
        writeln!(
            self.writer,
            "  {} / {} code units",
            Grouped(health.debt() as usize),
            Grouped(health.total() as usize)
        )?;
        self.theme.write(self.writer, Role::Bad, "▲")?;
        write!(self.writer, " {} high · ", Grouped(health.high() as usize))?;
        self.theme.write(self.writer, Role::Watch, "●")?;
        writeln!(
            self.writer,
            " {} watch · {} healthy",
            Grouped(health.watch() as usize),
            Grouped(health.healthy() as usize)
        )?;
        let excluded = coverage.unsupported_files() + coverage.failed_files();
        if excluded == 0 {
            self.theme.write(self.writer, Role::Good, "✓")?;
            writeln!(
                self.writer,
                " all {} analyzed",
                Counted::new(
                    coverage.selected_files() as usize,
                    "source file",
                    "source files"
                )
            )?;
        } else {
            self.theme.write(self.writer, Role::Warning, "!")?;
            writeln!(
                self.writer,
                " {} of {} excluded from analysis",
                Counted::new(excluded as usize, "source file", "source files"),
                Grouped(coverage.selected_files() as usize)
            )?;
        }
        Ok(())
    }

    fn write_change(&mut self, scope: &Scope) -> io::Result<()> {
        let diff = scope.diff();
        writeln!(self.writer)?;
        self.heading("CHANGE")?;
        writeln!(
            self.writer,
            "  {} retained units",
            Grouped(diff.total() as usize)
        )?;
        self.theme.write(self.writer, Role::Bad, "▲ WORSE")?;
        write!(self.writer, " {} · ", Grouped(diff.worse() as usize))?;
        self.theme.write(self.writer, Role::Good, "▼ BETTER")?;
        write!(self.writer, " {} · ", Grouped(diff.better() as usize))?;
        self.theme.write(self.writer, Role::Watch, "● CHANGED")?;
        writeln!(self.writer, " {}", Grouped(diff.changed() as usize))
    }

    fn write_codebase_areas(&mut self, view: &Presentation<'_>) -> io::Result<()> {
        let Some(scope) = view.displayed else {
            return Ok(());
        };
        if scope.children().is_empty() {
            return Ok(());
        }
        if view.areas.is_empty() {
            writeln!(self.writer, "\nNo child areas need attention.")?;
            return self.write_area_omissions(view, true);
        }
        writeln!(self.writer)?;
        self.heading_line("DEBT BY AREA")?;
        match self.layout {
            Layout::Stacked => self.write_codebase_cards(&view.areas, scope.health().debt())?,
            Layout::Full | Layout::Compact => {
                self.write_codebase_table(&view.areas, scope.health().debt())?
            }
        }
        self.write_area_omissions(view, true)
    }

    fn write_codebase_table(&mut self, areas: &[&Scope], denominator: u32) -> io::Result<()> {
        let bar_width = self.layout.bar_width();
        let fixed = match self.layout {
            Layout::Full => 49,
            Layout::Compact => 41,
            Layout::Stacked => unreachable!(),
        };
        let available = self.options.width.saturating_sub(fixed).max(12);
        let label_width = areas
            .iter()
            .map(|area| UnicodeWidthStr::width(area.name()))
            .max()
            .unwrap_or(12)
            .min(available)
            .max(12);
        writeln!(
            self.writer,
            "  {:label_width$}  {:>5}  {:>5}  {:>6}  {:bar_width$}  {:>5}",
            "area",
            "high",
            "watch",
            "share",
            "rate",
            "",
            label_width = label_width,
            bar_width = bar_width
        )?;
        for area in areas {
            let health = area.health();
            let label = middle_truncate(area.name(), label_width);
            write!(self.writer, "  {}  ", Padded::new(&label, label_width))?;
            self.table_count(health.high(), Role::Bad, 5)?;
            write!(self.writer, "  ")?;
            self.table_count(health.watch(), Role::Watch, 5)?;
            write!(
                self.writer,
                "  {:>6}  ",
                DisplayPercent::new(health.debt(), denominator)
            )?;
            self.theme.write(
                self.writer,
                Role::Navigation,
                &Bar::new(health.debt(), health.total(), bar_width),
            )?;
            writeln!(
                self.writer,
                "  {:>5}",
                DisplayPercent::new(health.debt(), health.total())
            )?;
        }
        Ok(())
    }

    fn write_codebase_cards(&mut self, areas: &[&Scope], denominator: u32) -> io::Result<()> {
        for area in areas {
            let health = area.health();
            let role = if health.high() > 0 {
                Role::Bad
            } else {
                Role::Watch
            };
            let marker = if health.high() > 0 { "▲" } else { "●" };
            self.theme.write(self.writer, role, marker)?;
            writeln!(self.writer, " {}", area.name())?;
            writeln!(
                self.writer,
                "  {} high · {} watch · {} share",
                Grouped(health.high() as usize),
                Grouped(health.watch() as usize),
                DisplayPercent::new(health.debt(), denominator)
            )?;
            write!(self.writer, "  rate ")?;
            self.theme.write(
                self.writer,
                Role::Navigation,
                &Bar::new(health.debt(), health.total(), self.layout.bar_width()),
            )?;
            writeln!(
                self.writer,
                " {}",
                DisplayPercent::new(health.debt(), health.total())
            )?;
        }
        Ok(())
    }

    fn write_diff_areas(&mut self, view: &Presentation<'_>) -> io::Result<()> {
        let Some(scope) = view.displayed else {
            return Ok(());
        };
        if scope.children().is_empty() {
            return Ok(());
        }
        writeln!(self.writer)?;
        self.heading_line("CHANGE BY AREA")?;
        match self.layout {
            Layout::Stacked => self.write_diff_cards(&view.areas, scope.diff().total())?,
            Layout::Full | Layout::Compact => {
                self.write_diff_table(&view.areas, scope.diff().total())?
            }
        }
        self.write_area_omissions(view, false)
    }

    fn write_diff_table(&mut self, areas: &[&Scope], denominator: u32) -> io::Result<()> {
        let bar_width = self.layout.bar_width();
        let fixed = match self.layout {
            Layout::Full => 55,
            Layout::Compact => 47,
            Layout::Stacked => unreachable!(),
        };
        let available = self.options.width.saturating_sub(fixed).max(12);
        let label_width = areas
            .iter()
            .map(|area| UnicodeWidthStr::width(area.name()))
            .max()
            .unwrap_or(12)
            .min(available)
            .max(12);
        writeln!(
            self.writer,
            "  {:label_width$}  {:>5}  {:>6}  {:>7}  {:bar_width$}  {:>5}",
            "area",
            "worse",
            "better",
            "changed",
            "share",
            "",
            label_width = label_width,
            bar_width = bar_width
        )?;
        for area in areas {
            let diff = area.diff();
            let label = middle_truncate(area.name(), label_width);
            write!(self.writer, "  {}  ", Padded::new(&label, label_width))?;
            self.table_count(diff.worse(), Role::Bad, 5)?;
            write!(self.writer, "  ")?;
            self.table_count(diff.better(), Role::Good, 6)?;
            write!(self.writer, "  ")?;
            self.table_count(diff.changed(), Role::Watch, 7)?;
            write!(self.writer, "  ")?;
            self.theme.write(
                self.writer,
                Role::Navigation,
                &Bar::new(diff.total(), denominator, bar_width),
            )?;
            writeln!(
                self.writer,
                "  {:>5}",
                DisplayPercent::new(diff.total(), denominator)
            )?;
        }
        Ok(())
    }

    fn write_diff_cards(&mut self, areas: &[&Scope], denominator: u32) -> io::Result<()> {
        for area in areas {
            let diff = area.diff();
            let (marker, role) = if diff.worse() > 0 {
                ("▲", Role::Bad)
            } else if diff.better() > 0 {
                ("▼", Role::Good)
            } else {
                ("●", Role::Watch)
            };
            self.theme.write(self.writer, role, marker)?;
            writeln!(self.writer, " {}", area.name())?;
            writeln!(
                self.writer,
                "  {} worse · {} better · {} changed",
                Grouped(diff.worse() as usize),
                Grouped(diff.better() as usize),
                Grouped(diff.changed() as usize)
            )?;
            write!(self.writer, "  share ")?;
            self.theme.write(
                self.writer,
                Role::Navigation,
                &Bar::new(diff.total(), denominator, self.layout.bar_width()),
            )?;
            writeln!(
                self.writer,
                " {}",
                DisplayPercent::new(diff.total(), denominator)
            )?;
        }
        Ok(())
    }

    fn write_area_omissions(&mut self, view: &Presentation<'_>, codebase: bool) -> io::Result<()> {
        if view.omitted_areas == 0 && (!codebase || view.quiet_areas == 0 || self.options.all) {
            return Ok(());
        }
        self.theme.start(self.writer, Role::Secondary)?;
        if view.omitted_areas > 0 && codebase && view.quiet_areas > 0 {
            writeln!(
                self.writer,
                "  … {} more debt-bearing {} · {} hidden (use --all)",
                Grouped(view.omitted_areas),
                if view.omitted_areas == 1 {
                    "area"
                } else {
                    "areas"
                },
                Counted::new(view.quiet_areas, "quiet area", "quiet areas")
            )?;
        } else if view.omitted_areas > 0 {
            writeln!(
                self.writer,
                "  … {} more {} hidden (use --all)",
                Grouped(view.omitted_areas),
                if view.omitted_areas == 1 {
                    "area"
                } else {
                    "areas"
                }
            )?;
        } else {
            writeln!(
                self.writer,
                "  … {} hidden (use --all)",
                Counted::new(view.quiet_areas, "quiet area", "quiet areas")
            )?;
        }
        self.theme.end(self.writer, Role::Secondary)
    }

    fn write_details(&mut self, view: &Presentation<'_>) -> io::Result<()> {
        match view.report.mode() {
            ReportMode::Codebase => self.write_findings(view),
            ReportMode::Diff => self.write_comparisons(view),
        }
    }

    fn write_findings(&mut self, view: &Presentation<'_>) -> io::Result<()> {
        if view.findings.is_empty() {
            writeln!(self.writer, "\nNo watch or high findings.")?;
            return Ok(());
        }
        writeln!(self.writer)?;
        self.heading_line("TOP FINDINGS")?;
        for finding in &view.findings {
            let role = if finding.assessment().rating() == Rating::High {
                Role::Bad
            } else {
                Role::Watch
            };
            let label = if role == Role::Bad {
                "▲ HIGH"
            } else {
                "● WATCH"
            };
            self.theme.write(self.writer, role, label)?;
            write!(self.writer, "  ")?;
            if let Some(container) = finding.identity().container() {
                write!(self.writer, "{container}::")?;
            }
            writeln!(self.writer, "{}", finding.identity().name())?;
            let file = &view.report.files()[finding.file().index()];
            writeln!(
                self.writer,
                "        {}:{}",
                file.path(),
                finding.span().start_line()
            )?;
            write!(self.writer, "        ")?;
            let mut first = true;
            for signal in finding
                .assessment()
                .signals()
                .iter()
                .filter(|signal| signal.rating() != Rating::Healthy)
            {
                if !first {
                    write!(self.writer, " · ")?;
                }
                write!(
                    self.writer,
                    "{} {}",
                    signal_name(signal.signal()),
                    signal.value()
                )?;
                first = false;
            }
            if let Some(activity) = file.activity().filter(|activity| activity.touches() > 0) {
                if !first {
                    write!(self.writer, " · ")?;
                }
                write!(
                    self.writer,
                    "{}",
                    Counted::new(activity.touches() as usize, "touch", "touches")
                )?;
            }
            writeln!(self.writer)?;
        }
        self.write_omitted(view.omitted_findings, "findings")
    }

    fn write_comparisons(&mut self, view: &Presentation<'_>) -> io::Result<()> {
        if view.comparisons.is_empty() {
            writeln!(self.writer, "\nNo changed units.")?;
            return Ok(());
        }
        writeln!(self.writer)?;
        self.heading_line("TOP CHANGES")?;
        for comparison in &view.comparisons {
            let (label, role) = match comparison.direction() {
                ComparisonDirection::Worse => ("▲ WORSE", Role::Bad),
                ComparisonDirection::Better => ("▼ BETTER", Role::Good),
                ComparisonDirection::Changed => ("● CHANGED", Role::Watch),
            };
            self.theme.write(self.writer, role, label)?;
            write!(self.writer, "  ")?;
            if let Some(container) = comparison.identity().container() {
                write!(self.writer, "{container}::")?;
            }
            writeln!(self.writer, "{}", comparison.identity().name())?;
            if let Some(file) = comparison.file() {
                writeln!(
                    self.writer,
                    "        {}",
                    view.report.files()[file.index()].path()
                )?;
            }
            write!(self.writer, "        ")?;
            match comparison.kind() {
                ComparisonKind::Added => write!(
                    self.writer,
                    "added as {}",
                    comparison.after_rating().map_or("unrated", rating_name)
                )?,
                ComparisonKind::Removed => write!(
                    self.writer,
                    "removed from {}",
                    comparison.before_rating().map_or("unrated", rating_name)
                )?,
                ComparisonKind::Ambiguous => {
                    write!(self.writer, "identity could not be matched safely")?
                }
                _ => self.write_changed_measurements(comparison)?,
            }
            writeln!(self.writer)?;
        }
        self.write_omitted(view.omitted_comparisons, "comparisons")
    }

    fn write_changed_measurements(&mut self, comparison: &Comparison) -> io::Result<()> {
        let (Some(before), Some(after)) = (comparison.before(), comparison.after()) else {
            return write!(self.writer, "{}", comparison_name(comparison.kind()));
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
            ("lines", before.logical_lines(), after.logical_lines()),
        ];
        let mut first = true;
        for (name, before, after) in values
            .into_iter()
            .filter(|(_, before, after)| before != after)
        {
            if !first {
                write!(self.writer, " · ")?;
            }
            write!(self.writer, "{name} {before} → {after}")?;
            first = false;
        }
        if first {
            write!(self.writer, "{}", comparison_name(comparison.kind()))?;
        }
        Ok(())
    }

    fn write_diagnostics(&mut self, diagnostics: &[Diagnostic]) -> io::Result<()> {
        if diagnostics.is_empty() {
            return Ok(());
        }
        writeln!(self.writer)?;
        self.heading_line("COVERAGE NOTES")?;
        for diagnostic in diagnostics.iter().take(5) {
            self.theme.write(self.writer, Role::Warning, "!")?;
            writeln!(self.writer, " {}", diagnostic.message())?;
        }
        if diagnostics.len() > 5 {
            self.write_omitted(diagnostics.len() - 5, "coverage notes")?;
        }
        Ok(())
    }

    fn write_omitted(&mut self, count: usize, noun: &str) -> io::Result<()> {
        if count == 0 {
            return Ok(());
        }
        self.theme.start(self.writer, Role::Secondary)?;
        writeln!(
            self.writer,
            "  … {} {noun} omitted (use --all)",
            Grouped(count)
        )?;
        self.theme.end(self.writer, Role::Secondary)
    }

    fn table_count(&mut self, value: u32, role: Role, width: usize) -> io::Result<()> {
        let shown = if value == 0 {
            "–".to_owned()
        } else {
            Grouped(value as usize).to_string()
        };
        let padding = width.saturating_sub(UnicodeWidthStr::width(shown.as_str()));
        write!(self.writer, "{}", " ".repeat(padding))?;
        self.theme.write(self.writer, role, &shown)
    }

    fn heading(&mut self, heading: &str) -> io::Result<()> {
        self.theme.write(self.writer, Role::Heading, heading)
    }

    fn heading_line(&mut self, heading: &str) -> io::Result<()> {
        self.heading(heading)?;
        writeln!(self.writer)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Role {
    Brand,
    Heading,
    Bad,
    Watch,
    Good,
    Warning,
    Navigation,
    Secondary,
}

struct Theme {
    enabled: bool,
    dim: (&'static str, &'static str),
}

impl Theme {
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
            dim: if enabled {
                ("\x1b[2m", "\x1b[0m")
            } else {
                ("", "")
            },
        }
    }

    fn style(role: Role) -> Style {
        match role {
            Role::Brand => Style::new().bold().fg_color(Some(AnsiColor::Cyan.into())),
            Role::Heading => Style::new().effects(Effects::BOLD),
            Role::Bad => Style::new().fg_color(Some(AnsiColor::Red.into())),
            Role::Watch | Role::Warning => Style::new().fg_color(Some(AnsiColor::Yellow.into())),
            Role::Good => Style::new().fg_color(Some(AnsiColor::Green.into())),
            Role::Navigation => Style::new().fg_color(Some(AnsiColor::Cyan.into())),
            Role::Secondary => Style::new().effects(Effects::DIMMED),
        }
    }

    fn write(
        &self,
        writer: &mut impl Write,
        role: Role,
        value: &(impl fmt::Display + ?Sized),
    ) -> io::Result<()> {
        self.start(writer, role)?;
        write!(writer, "{value}")?;
        self.end(writer, role)
    }

    fn start(&self, writer: &mut impl Write, role: Role) -> io::Result<()> {
        if self.enabled {
            write!(writer, "{}", Self::style(role).render())
        } else {
            Ok(())
        }
    }

    fn end(&self, writer: &mut impl Write, role: Role) -> io::Result<()> {
        if self.enabled {
            write!(writer, "{}", Self::style(role).render_reset())
        } else {
            Ok(())
        }
    }
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

struct Bar {
    value: u32,
    denominator: u32,
    width: usize,
}

impl Bar {
    const fn new(value: u32, denominator: u32, width: usize) -> Self {
        Self {
            value,
            denominator,
            width,
        }
    }
}

impl fmt::Display for Bar {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        const FRACTIONS: [char; 8] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];
        let eighths = if self.denominator == 0 {
            0
        } else {
            (self.value as usize * self.width * 8 + self.denominator as usize / 2)
                / self.denominator as usize
        };
        let eighths = if self.value > 0 { eighths.max(1) } else { 0 }.min(self.width * 8);
        let full = eighths / 8;
        let fraction = eighths % 8;
        for _ in 0..full {
            formatter.write_str("█")?;
        }
        if fraction > 0 {
            write!(formatter, "{}", FRACTIONS[fraction])?;
        }
        for _ in (full + usize::from(fraction > 0))..self.width {
            formatter.write_str("░")?;
        }
        Ok(())
    }
}

struct Padded<'a> {
    value: &'a str,
    width: usize,
}
impl<'a> Padded<'a> {
    const fn new(value: &'a str, width: usize) -> Self {
        Self { value, width }
    }
}
impl fmt::Display for Padded<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.value)?;
        for _ in UnicodeWidthStr::width(self.value)..self.width {
            formatter.write_str(" ")?;
        }
        Ok(())
    }
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
    if scope.kind() == ScopeKind::Repository {
        return count_package_descendants(report, scope);
    }
    let mut current = Some(scope);
    while let Some(candidate) = current {
        if candidate.kind() == ScopeKind::Package {
            return 1;
        }
        current = candidate.parent().map(|id| &report.scopes()[id.index()]);
    }
    0
}

fn count_package_descendants(report: &Report, scope: &Scope) -> usize {
    usize::from(scope.kind() == ScopeKind::Package)
        + scope
            .children()
            .iter()
            .map(|id| count_package_descendants(report, &report.scopes()[id.index()]))
            .sum::<usize>()
}

fn signal_name(signal: Signal) -> &'static str {
    match signal {
        Signal::CognitiveComplexity => "cognitive",
        Signal::CyclomaticComplexity => "cyclomatic",
        Signal::LogicalLines => "lines",
    }
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

fn drill_path_from_visible(
    report: &Report,
    selected: &Scope,
    child: Option<&Scope>,
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

fn diff_child_order(left: &Scope, right: &Scope) -> std::cmp::Ordering {
    right
        .diff()
        .worse()
        .cmp(&left.diff().worse())
        .then_with(|| right.diff().better().cmp(&left.diff().better()))
        .then_with(|| right.diff().changed().cmp(&left.diff().changed()))
        .then_with(|| left.name().cmp(right.name()))
}

fn middle_truncate(value: &str, width: usize) -> String {
    if UnicodeWidthStr::width(value) <= width {
        return value.to_owned();
    }
    if width <= 1 {
        return "…".to_owned();
    }
    let left_width = (width - 1) / 2;
    let right_width = width - 1 - left_width;
    let mut left = String::new();
    let mut used = 0;
    for character in value.chars() {
        let character_width = character.width().unwrap_or(0);
        if used + character_width > left_width {
            break;
        }
        left.push(character);
        used += character_width;
    }
    let mut right = String::new();
    used = 0;
    for character in value.chars().rev() {
        let character_width = character.width().unwrap_or(0);
        if used + character_width > right_width {
            break;
        }
        right.insert(0, character);
        used += character_width;
    }
    format!("{left}…{right}")
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
        assert!(terminal.contains("TOP FINDINGS"));
        assert!(terminal.contains("file-10.rs"));
        assert!(!terminal.contains("file-1.rs"));
    }

    #[test]
    fn absent_activity_does_not_call_findings_hotspots() {
        let report = report_with_findings(false);
        let mut terminal = Vec::new();
        write_terminal(&mut terminal, &report, TerminalOptions::default()).unwrap();
        let terminal = String::from_utf8(terminal).unwrap();
        assert!(terminal.contains("TOP FINDINGS"));
    }

    #[test]
    fn area_labels_are_shortened_in_the_middle_by_display_width() {
        assert_eq!(middle_truncate("short", 20), "short");
        assert_eq!(middle_truncate("directory/file.rs", 8), "dir…e.rs");
        assert_eq!(
            UnicodeWidthStr::width(middle_truncate("目录/directory/file.rs", 12).as_str()),
            12
        );
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
    fn fractional_bars_keep_small_nonzero_values_visible() {
        assert_eq!(Bar::new(0, 1_000, 8).to_string(), "░░░░░░░░");
        assert_eq!(Bar::new(1, 1_000, 8).to_string(), "▏░░░░░░░");
        assert_eq!(
            UnicodeWidthStr::width(Bar::new(1, 1_000, 8).to_string().as_str()),
            8
        );
    }

    #[test]
    fn grouped_counts_and_nouns_are_readable() {
        assert_eq!(Grouped(19_761).to_string(), "19,761");
        assert_eq!(Counted::new(1, "file", "files").to_string(), "1 file");
        assert_eq!(Counted::new(2, "file", "files").to_string(), "2 files");
    }

    #[test]
    fn package_count_uses_package_scopes_not_files_or_directories() {
        let mut report = Report::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        let package = ScopeId::from_index(1);
        let directory = ScopeId::from_index(2);
        let file = ScopeId::from_index(3);
        let mut root_scope = Scope::new(root, ScopeKind::Repository, ".", None);
        root_scope.add_child(package);
        report.add_scope(root_scope);
        let mut package_scope = Scope::new(package, ScopeKind::Package, ".", Some(root));
        package_scope.add_child(directory);
        report.add_scope(package_scope);
        let mut directory_scope = Scope::new(directory, ScopeKind::Directory, "src", Some(package));
        directory_scope.add_child(file);
        report.add_scope(directory_scope);
        report.add_scope(Scope::new(
            file,
            ScopeKind::File,
            "src/lib.rs",
            Some(directory),
        ));

        assert_eq!(package_count(&report, &report.scopes()[root.index()]), 1);
        assert_eq!(
            package_count(&report, &report.scopes()[directory.index()]),
            1
        );
        assert_eq!(package_count(&report, &report.scopes()[file.index()]), 1);
    }

    #[test]
    fn full_compact_and_stacked_layouts_keep_the_same_area_facts() {
        let mut report = Report::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        let package = ScopeId::from_index(1);
        let debt = ScopeId::from_index(2);
        let quiet = ScopeId::from_index(3);
        let mut root_scope = Scope::new(root, ScopeKind::Repository, ".", None);
        root_scope.add_child(package);
        report.add_scope(root_scope);
        let mut package_scope = Scope::new(package, ScopeKind::Package, ".", Some(root));
        package_scope.add_child(debt);
        package_scope.add_child(quiet);
        report.add_scope(package_scope);
        report.add_scope(Scope::new(
            debt,
            ScopeKind::Directory,
            "src/very-long-feature-name",
            Some(package),
        ));
        report.add_scope(Scope::new(
            quiet,
            ScopeKind::Directory,
            "tests",
            Some(package),
        ));
        report.set_root(root);
        for (index, scope, health) in [
            (0, debt, HealthCounts::new(8, 2, 1)),
            (1, quiet, HealthCounts::new(4, 0, 0)),
        ] {
            let file = FileRecord::new(
                FileId::from_index(index),
                scope,
                format!("file-{index}.rs"),
                Coverage::new(1, 1, 0, 0, 1_234, 0),
                health,
            );
            report.scopes_mut()[scope.index()].add_file(file.id());
            report.add_file(file);
        }
        let files = report.files().to_vec();
        aggregate_scopes(report.scopes_mut(), &files, root);

        let render = |width| {
            let mut bytes = Vec::new();
            write_terminal(
                &mut bytes,
                &report,
                TerminalOptions {
                    width,
                    all: false,
                    color: false,
                },
            )
            .unwrap();
            String::from_utf8(bytes).unwrap()
        };
        let full = render(120);
        let compact = render(80);
        let stacked = render(50);
        for output in [&full, &compact, &stacked] {
            assert!(output.contains("src/very-long-feature-name"));
            assert!(output.contains("1 high"));
            assert!(output.contains("2 watch"));
            assert!(output.contains("1 quiet area hidden"));
        }
        assert!(full.contains("area"));
        assert!(compact.contains("rate"));
        assert!(stacked.contains("▲ src/very-long-feature-name\n"));
    }

    #[test]
    fn removing_ansi_from_styled_output_reproduces_plain_output() {
        let report = report_with_findings(true);
        let render = |color| {
            let mut bytes = Vec::new();
            write_terminal(
                &mut bytes,
                &report,
                TerminalOptions {
                    width: 80,
                    all: false,
                    color,
                },
            )
            .unwrap();
            bytes
        };
        let colored = render(true);
        let plain = render(false);
        assert!(colored.windows(2).any(|bytes| bytes == b"\x1b["));
        assert_eq!(strip_ansi(&colored), plain);
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

        let path = drill_path_from_visible(
            &report,
            &report.scopes()[package.index()],
            Some(&report.scopes()[child.index()]),
        );

        assert_eq!(path, Some(PathBuf::from("/work/project/bow/src")));
    }
}
