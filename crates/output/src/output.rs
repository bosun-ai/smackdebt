//! Stable terminal and JSON views over the shared report.

use std::cmp::Reverse;
use std::fmt;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anstyle::{Ansi256Color, AnsiColor, Style};
use smackdebt_analysis::{
    ArchitectureFindingKind, Comparison, ComparisonDirection, ComparisonKind, Diagnostic,
    DiagnosticKind, FileId, FileRecord, Finding, Language, Rating, Report, ReportMode, Scope,
    ScopeKind, Signal, SourceRole, SourceTrust,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Resolved terminal display choices.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TerminalOptions {
    width: usize,
    all: bool,
    color: bool,
}

impl TerminalOptions {
    pub const fn new(width: usize, all: bool, color: bool) -> Self {
        Self { width, all, color }
    }
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
    selected_scope: Option<smackdebt_analysis::ScopeId>,
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

#[derive(Clone, Copy, Eq, PartialEq)]
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
}

struct Presentation<'a> {
    report: &'a Report,
    selected: Option<&'a Scope>,
    breadcrumbs: Vec<&'a str>,
    areas: Vec<&'a Scope>,
    findings: Vec<&'a Finding>,
    comparisons: Vec<&'a Comparison>,
    drill: Option<PathBuf>,
}

impl<'a> Presentation<'a> {
    fn new(
        report: &'a Report,
        selected_scope: Option<smackdebt_analysis::ScopeId>,
        all: bool,
    ) -> Self {
        let selected = selected_scope
            .or_else(|| report.root())
            .and_then(|id| report.scopes().get(id.index()));
        let mut displayed = selected;
        let mut breadcrumbs = Vec::new();
        while let Some(scope) = displayed
            && scope.kind() != ScopeKind::File
            && scope.children().len() == 1
        {
            let child = &report.scopes()[scope.children()[0].index()];
            if child.name() != "." || scope.name() != "." {
                breadcrumbs.push(terminal_path(child.name()));
            }
            displayed = Some(child);
        }

        let mut areas = displayed.map_or_else(Vec::new, |scope| display_children(report, scope));
        match report.mode() {
            ReportMode::Codebase => {
                areas.sort_by(|left, right| codebase_child_order(left, right));
                areas.retain(|area| area.health().debt() > 0);
            }
            ReportMode::Diff => {
                areas.sort_by(|left, right| diff_child_order(left, right));
                areas.retain(|area| area.diff().total() > 0);
            }
        }
        if !all && areas.len() > 5 {
            areas.truncate(5);
        }

        let mut findings = Vec::new();
        let mut comparisons = Vec::new();
        if let Some(scope) = displayed {
            findings = scope
                .findings()
                .iter()
                .map(|id| &report.findings()[id.index()])
                .collect();
            if !all {
                findings.retain(|finding| finding.affects_verdict());
            }
            findings.sort_by(|left, right| finding_order(report, left, right));
            let finding_limit = if all || scope.kind() == ScopeKind::File {
                findings.len()
            } else {
                findings.len().min(3)
            };
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
            breadcrumbs,
            areas,
            findings,
            comparisons,
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
                "  {}",
                middle_truncate(breadcrumb, self.options.width.saturating_sub(2))
            )?;
        }
        match view.report.mode() {
            ReportMode::Codebase => self.write_codebase_areas(view)?,
            ReportMode::Diff => self.write_diff_areas(view)?,
        }
        self.write_details(view)?;
        self.write_architecture(view)?;
        self.write_evolution(view)?;
        self.write_diagnostics(view)?;
        if let Some(path) = &view.drill {
            writeln!(self.writer)?;
            self.theme.write_glyph(self.writer, Glyph::Discover)?;
            let path = path.to_string_lossy();
            writeln!(
                self.writer,
                " smackdebt {}",
                middle_truncate(&path, self.options.width.saturating_sub(12))
            )?;
        }
        Ok(())
    }

    fn write_architecture(&mut self, view: &Presentation<'_>) -> io::Result<()> {
        let Some(scope) = view.selected else {
            return Ok(());
        };
        let report = view.report;
        let finding_ids = scope.architecture_findings();
        let relevant_edges: Vec<_> = report
            .dependency_edges()
            .iter()
            .filter(|edge| {
                file_belongs_to_scope(report, edge.source(), scope)
                    || file_belongs_to_scope(report, edge.target(), scope)
            })
            .collect();
        let show_relationships = self.options.all || scope.kind() != ScopeKind::Repository;
        let comparisons = scope
            .architecture_comparisons()
            .iter()
            .map(|id| &report.architecture_comparisons()[id.index()])
            .filter(|comparison| {
                show_relationships
                    || matches!(
                        comparison.kind(),
                        smackdebt_analysis::ArchitectureComparisonKind::CycleIntroduced
                            | smackdebt_analysis::ArchitectureComparisonKind::CycleRemoved
                    )
            })
            .collect::<Vec<_>>();
        let has_relationships = show_relationships
            && (!relevant_edges.is_empty()
                || report
                    .external_dependencies()
                    .iter()
                    .any(|value| file_belongs_to_scope(report, value.file(), scope))
                || report
                    .resolution_diagnostics()
                    .iter()
                    .any(|value| file_belongs_to_scope(report, value.file(), scope)));
        let has_findings = match report.mode() {
            ReportMode::Codebase => !finding_ids.is_empty(),
            ReportMode::Diff => !comparisons.is_empty(),
        };
        if !has_findings && !has_relationships {
            return Ok(());
        }
        writeln!(self.writer)?;
        self.heading_line("ARCHITECTURE")?;
        match report.mode() {
            ReportMode::Codebase => {
                let limit = if self.options.all {
                    finding_ids.len()
                } else {
                    finding_ids.len().min(3)
                };
                for id in finding_ids.iter().take(limit) {
                    let finding = &report.architecture_findings()[id.index()];
                    let glyph = if finding.rating() == Rating::High {
                        Glyph::High
                    } else {
                        Glyph::Watch
                    };
                    self.theme.write_glyph(self.writer, glyph)?;
                    writeln!(
                        self.writer,
                        " {}",
                        architecture_finding_name(finding.kind())
                    )?;
                    if !finding.witness_edges().is_empty() {
                        let mut edges = finding
                            .witness_edges()
                            .iter()
                            .filter_map(|id| report.dependency_edges().get(id.index()));
                        if let Some(first) = edges.next() {
                            if self.layout == Layout::Stacked {
                                self.write_witness_path(
                                    report.files()[first.source().index()].path(),
                                    false,
                                )?;
                                self.write_witness_path(
                                    report.files()[first.target().index()].path(),
                                    true,
                                )?;
                                for edge in edges {
                                    self.write_witness_path(
                                        report.files()[edge.target().index()].path(),
                                        true,
                                    )?;
                                }
                            } else {
                                write!(
                                    self.writer,
                                    "        {} → {}",
                                    report.files()[first.source().index()].path(),
                                    report.files()[first.target().index()].path()
                                )?;
                                for edge in edges {
                                    write!(
                                        self.writer,
                                        " → {}",
                                        report.files()[edge.target().index()].path()
                                    )?;
                                }
                                writeln!(self.writer)?;
                            }
                        }
                    }
                }
                if show_relationships {
                    self.write_resolved_edges(report, &relevant_edges)?;
                    self.write_other_relations(report, scope)?;
                }
            }
            ReportMode::Diff => {
                for comparison in comparisons {
                    let glyph = match comparison.direction() {
                        ComparisonDirection::Worse => Glyph::Worse,
                        ComparisonDirection::Better => Glyph::Better,
                        ComparisonDirection::Changed => Glyph::Changed,
                    };
                    self.theme.write_glyph(self.writer, glyph)?;
                    match comparison.kind() {
                        smackdebt_analysis::ArchitectureComparisonKind::CycleIntroduced
                        | smackdebt_analysis::ArchitectureComparisonKind::CycleRemoved => {
                            writeln!(
                                self.writer,
                                " package cycle {}",
                                if matches!(
                                    comparison.kind(),
                                    smackdebt_analysis::ArchitectureComparisonKind::CycleIntroduced
                                ) {
                                    "introduced"
                                } else {
                                    "removed"
                                }
                            )?;
                            if !comparison.witness().is_empty() {
                                for (index, package) in comparison.witness().iter().enumerate() {
                                    let name = package_name(report, package.index()).unwrap_or("?");
                                    if self.layout == Layout::Stacked {
                                        self.write_witness_path(name, index > 0)?;
                                    } else {
                                        if index == 0 {
                                            write!(self.writer, "        ")?;
                                        } else {
                                            write!(self.writer, " → ")?;
                                        }
                                        write!(self.writer, "{name}")?;
                                    }
                                }
                                if self.layout != Layout::Stacked {
                                    writeln!(self.writer)?;
                                }
                            }
                        }
                        smackdebt_analysis::ArchitectureComparisonKind::EdgeAdded
                        | smackdebt_analysis::ArchitectureComparisonKind::EdgeRemoved => {
                            let change = if matches!(
                                comparison.kind(),
                                smackdebt_analysis::ArchitectureComparisonKind::EdgeAdded
                            ) {
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
                            if self.layout == Layout::Stacked {
                                self.write_relation_identity(
                                    source,
                                    target,
                                    comparison.relation(),
                                )?;
                                writeln!(self.writer, "        {change}")?;
                                self.write_stacked_evidence(comparison.role(), comparison.trust())?;
                                continue;
                            }
                            match comparison.relation() {
                                Some(smackdebt_analysis::StaticRelationKind::ModuleOwnership) => {
                                    write!(self.writer, " {source} owns {target} · {change}")?;
                                }
                                _ => {
                                    write!(self.writer, " {source} → {target} · {change}")?;
                                }
                            }
                            self.write_evidence_suffix(comparison.role(), comparison.trust())?;
                            writeln!(self.writer)?;
                        }
                    }
                }
                if show_relationships {
                    self.write_resolved_edges(report, &relevant_edges)?;
                    self.write_other_relations(report, scope)?;
                }
            }
        }
        Ok(())
    }

    fn write_witness_path(&mut self, path: &str, arrow: bool) -> io::Result<()> {
        let prefix = if arrow { "        → " } else { "        " };
        writeln!(
            self.writer,
            "{prefix}{}",
            middle_truncate(path, self.options.width.saturating_sub(10))
        )
    }

    fn write_relation_identity(
        &mut self,
        source: &str,
        target: &str,
        relation: Option<smackdebt_analysis::StaticRelationKind>,
    ) -> io::Result<()> {
        writeln!(
            self.writer,
            "  {}",
            middle_truncate(source, self.options.width.saturating_sub(2))
        )?;
        let verb = if relation == Some(smackdebt_analysis::StaticRelationKind::ModuleOwnership) {
            "owns"
        } else {
            "→"
        };
        writeln!(
            self.writer,
            "        {verb} {}",
            middle_truncate(target, self.options.width.saturating_sub(9 + verb.len()))
        )
    }

    fn write_resolved_edges(
        &mut self,
        report: &Report,
        edges: &[&smackdebt_analysis::DependencyEdge],
    ) -> io::Result<()> {
        for edge in edges {
            let source = report
                .files()
                .get(edge.source().index())
                .map_or("?", FileRecord::path);
            let target = report
                .files()
                .get(edge.target().index())
                .map_or("?", FileRecord::path);
            if self.layout == Layout::Stacked {
                self.write_relation_identity(source, target, Some(edge.relation()))?;
                if edge.relation() == smackdebt_analysis::StaticRelationKind::Uses {
                    writeln!(
                        self.writer,
                        "        {}",
                        Counted::new(edge.references() as usize, "import", "imports")
                    )?;
                }
                self.write_stacked_evidence(Some(edge.role()), Some(edge.trust()))?;
                continue;
            }
            match edge.relation() {
                smackdebt_analysis::StaticRelationKind::Uses => write!(
                    self.writer,
                    "  {source} → {target} · {}",
                    Counted::new(edge.references() as usize, "import", "imports")
                )?,
                smackdebt_analysis::StaticRelationKind::ModuleOwnership => {
                    write!(self.writer, "  {source} owns {target}")?;
                }
            }
            self.write_evidence_suffix(Some(edge.role()), Some(edge.trust()))?;
            writeln!(self.writer)?;
        }
        Ok(())
    }

    fn write_other_relations(&mut self, report: &Report, scope: &Scope) -> io::Result<()> {
        for dependency in report
            .external_dependencies()
            .iter()
            .filter(|value| file_belongs_to_scope(report, value.file(), scope))
        {
            let source = report.files()[dependency.file().index()].path();
            if self.layout == Layout::Stacked {
                self.write_relation_identity(source, dependency.target(), None)?;
                writeln!(self.writer, "        external")?;
                self.write_stacked_evidence(Some(dependency.role()), Some(dependency.trust()))?;
                continue;
            }
            write!(
                self.writer,
                "  {} → {} · external",
                source,
                dependency.target(),
            )?;
            self.write_evidence_suffix(Some(dependency.role()), Some(dependency.trust()))?;
            writeln!(self.writer)?;
        }
        for diagnostic in report
            .resolution_diagnostics()
            .iter()
            .filter(|value| file_belongs_to_scope(report, value.file(), scope))
        {
            let source = report.files()[diagnostic.file().index()].path();
            let status = match diagnostic.kind() {
                smackdebt_analysis::ResolutionIssueKind::Unresolved => "could not be matched",
                smackdebt_analysis::ResolutionIssueKind::Ambiguous => "matched more than one file",
            };
            if self.layout == Layout::Stacked {
                let location = format!("{source}:{}", diagnostic.span().start_line());
                writeln!(
                    self.writer,
                    "  {}",
                    middle_truncate(&location, self.options.width.saturating_sub(2))
                )?;
                writeln!(
                    self.writer,
                    "        → {}",
                    middle_truncate(diagnostic.target(), self.options.width.saturating_sub(10))
                )?;
                writeln!(self.writer, "        {status}")?;
                self.write_stacked_evidence(Some(diagnostic.role()), Some(diagnostic.trust()))?;
                continue;
            }
            write!(
                self.writer,
                "  {}:{} → {} · {}",
                source,
                diagnostic.span().start_line(),
                diagnostic.target(),
                status,
            )?;
            self.write_evidence_suffix(Some(diagnostic.role()), Some(diagnostic.trust()))?;
            writeln!(self.writer)?;
        }
        Ok(())
    }

    fn write_stacked_evidence(
        &mut self,
        role: Option<SourceRole>,
        trust: Option<SourceTrust>,
    ) -> io::Result<()> {
        let suffix = evidence_suffix(role, trust);
        if !suffix.is_empty() {
            writeln!(self.writer, "        {}", suffix.trim_start_matches(" · "))?;
        }
        Ok(())
    }

    fn write_evidence_suffix(
        &mut self,
        role: Option<SourceRole>,
        trust: Option<SourceTrust>,
    ) -> io::Result<()> {
        write!(self.writer, "{}", evidence_suffix(role, trust))
    }

    fn write_evolution(&mut self, view: &Presentation<'_>) -> io::Result<()> {
        let Some(scope) = view.selected else {
            return Ok(());
        };
        let report = view.report;
        let relevant_packages = report
            .files()
            .iter()
            .filter(|file| file_belongs_to_scope(report, file.id(), scope))
            .filter_map(FileRecord::package)
            .collect::<std::collections::BTreeSet<_>>();
        let relevant =
            |package: smackdebt_analysis::PackageId| relevant_packages.contains(&package);
        let detail = self.options.all || scope.kind() != ScopeKind::Repository;
        let mut histories = report
            .package_history()
            .iter()
            .filter(|value| relevant(value.package()))
            .filter(|value| detail && value.touches() > 0)
            .collect::<Vec<_>>();
        histories.sort_by_key(|value| (Reverse(value.touches()), value.package()));
        if !detail {
            let mut displayed_packages = std::collections::BTreeSet::new();
            histories.retain(|value| displayed_packages.insert(value.package()));
        }
        let mut files = report
            .file_history()
            .iter()
            .filter(|value| {
                detail && value.touches() > 0 && file_belongs_to_scope(report, value.file(), scope)
            })
            .collect::<Vec<_>>();
        files.sort_by(|left, right| {
            Reverse(left.touches())
                .cmp(&Reverse(right.touches()))
                .then_with(|| {
                    report.files()[left.file().index()]
                        .path()
                        .cmp(report.files()[right.file().index()].path())
                })
        });
        let mut finding_couplings = scope
            .evolutionary_findings()
            .iter()
            .map(|id| report.evolutionary_findings()[id.index()].coupling())
            .collect::<Vec<_>>();
        if !detail {
            finding_couplings.sort_by(|left, right| {
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
            finding_couplings.truncate(3);
        }
        let mut couplings = if detail {
            report
                .change_coupling()
                .iter()
                .copied()
                .filter(|pair| relevant(pair.left()) || relevant(pair.right()))
                .filter(|pair| {
                    !finding_couplings
                        .iter()
                        .any(|candidate| same_coupling_pair(*candidate, *pair))
                })
                .map(|pair| (pair, false))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        couplings.extend(finding_couplings.iter().copied().map(|pair| (pair, true)));
        let has_comparisons =
            report.mode() == ReportMode::Diff && !scope.evolutionary_comparisons().is_empty();
        if couplings.is_empty() && histories.is_empty() && files.is_empty() && !has_comparisons {
            return Ok(());
        }
        writeln!(self.writer)?;
        self.heading_line("HISTORY")?;
        for (pair, finding) in &couplings {
            let left = package_name(report, pair.left().index()).unwrap_or("?");
            let right = package_name(report, pair.right().index()).unwrap_or("?");
            if *finding {
                self.theme.write_glyph(self.writer, Glyph::Watch)?;
                write!(self.writer, " ")?;
            } else {
                write!(self.writer, "  ")?;
            }
            if self.layout == Layout::Stacked {
                let identity_width = self.options.width.saturating_sub(4);
                let identity = format!("{left} ↔ {right}");
                writeln!(
                    self.writer,
                    "{}",
                    middle_truncate(&identity, identity_width)
                )?;
                writeln!(
                    self.writer,
                    "        {} of {} commits · {}%",
                    pair.shared_commits(),
                    pair.union_commits(),
                    (pair.similarity() * 100.0).round() as u32,
                )?;
                writeln!(
                    self.writer,
                    "        {}",
                    if coupling_has_code_dependency(report, *pair) {
                        "code dependency exists"
                    } else {
                        "no code dependency"
                    }
                )?;
                continue;
            }
            write!(
                self.writer,
                "{} ↔ {} changed together in {} of {} commits · {}% · ",
                left,
                right,
                pair.shared_commits(),
                pair.union_commits(),
                (pair.similarity() * 100.0).round() as u32,
            )?;
            writeln!(
                self.writer,
                "{}",
                if coupling_has_code_dependency(report, *pair) {
                    "code dependency exists"
                } else {
                    "no code dependency"
                }
            )?;
        }
        for value in &histories {
            let name = package_name(report, value.package().index()).unwrap_or("?");
            if self.layout == Layout::Stacked {
                writeln!(
                    self.writer,
                    "  {}",
                    middle_truncate(name, self.options.width.saturating_sub(2))
                )?;
                writeln!(
                    self.writer,
                    "        {} · +{} -{}",
                    Counted::new(value.touches() as usize, "commit", "commits"),
                    Grouped(value.added_lines() as usize),
                    Grouped(value.deleted_lines() as usize)
                )?;
                continue;
            }
            writeln!(
                self.writer,
                "  {} · {} · +{} -{}",
                name,
                Counted::new(value.touches() as usize, "commit", "commits"),
                Grouped(value.added_lines() as usize),
                Grouped(value.deleted_lines() as usize)
            )?;
        }
        for value in files {
            let path = report.files()[value.file().index()].path();
            if self.layout == Layout::Stacked {
                writeln!(
                    self.writer,
                    "  {}",
                    middle_truncate(path, self.options.width.saturating_sub(2))
                )?;
                writeln!(
                    self.writer,
                    "        {} · +{} -{}",
                    Counted::new(value.touches() as usize, "commit", "commits"),
                    Grouped(value.added_lines() as usize),
                    Grouped(value.deleted_lines() as usize)
                )?;
                continue;
            }
            writeln!(
                self.writer,
                "  {} · {} · +{} -{}",
                path,
                Counted::new(value.touches() as usize, "commit", "commits"),
                Grouped(value.added_lines() as usize),
                Grouped(value.deleted_lines() as usize)
            )?;
        }
        if report.mode() == ReportMode::Diff {
            for id in scope.evolutionary_comparisons() {
                let comparison = report.evolutionary_comparisons()[id.index()];
                let pair = comparison.coupling();
                let glyph = match comparison.direction() {
                    ComparisonDirection::Better => Glyph::Better,
                    ComparisonDirection::Worse => Glyph::Worse,
                    ComparisonDirection::Changed => Glyph::Changed,
                };
                self.theme.write_glyph(self.writer, glyph)?;
                let left = package_name(report, pair.left().index()).unwrap_or("?");
                let right = package_name(report, pair.right().index()).unwrap_or("?");
                if self.layout == Layout::Stacked {
                    let identity_width = self.options.width.saturating_sub(4);
                    let identity = format!("{left} ↔ {right}");
                    writeln!(
                        self.writer,
                        " {}",
                        middle_truncate(&identity, identity_width)
                    )?;
                    writeln!(
                        self.writer,
                        "        {}",
                        match comparison.kind() {
                            smackdebt_analysis::EvolutionaryComparisonKind::FindingIntroduced => {
                                "now change together"
                            }
                            smackdebt_analysis::EvolutionaryComparisonKind::FindingRemoved => {
                                "no longer change together"
                            }
                        }
                    )?;
                    writeln!(self.writer, "        without a code dependency")?;
                    continue;
                }
                writeln!(
                    self.writer,
                    " {} ↔ {} {}",
                    left,
                    right,
                    match comparison.kind() {
                        smackdebt_analysis::EvolutionaryComparisonKind::FindingIntroduced =>
                            "now change together without a code dependency",
                        smackdebt_analysis::EvolutionaryComparisonKind::FindingRemoved => {
                            "no longer change together without a code dependency"
                        }
                    }
                )?;
            }
        }
        Ok(())
    }

    fn write_header(&mut self, view: &Presentation<'_>) -> io::Result<()> {
        writeln!(self.writer, "smackdebt {}", mode_name(view.report.mode()))?;
        let selected = terminal_path(view.selected.map_or(".", Scope::name));
        writeln!(
            self.writer,
            "{}",
            middle_truncate(selected, self.options.width)
        )
    }

    fn write_quality(&mut self, _report: &Report, scope: &Scope) -> io::Result<()> {
        let health = scope.health();
        let coverage = scope.coverage();
        writeln!(self.writer)?;
        self.heading_line("QUALITY")?;
        writeln!(
            self.writer,
            "{} rated {} · {} need attention",
            Grouped(health.total() as usize),
            if health.total() == 1 { "unit" } else { "units" },
            Grouped(health.debt() as usize),
        )?;
        if health.high() > 0 || health.watch() > 0 {
            if health.high() > 0 {
                self.theme.write_glyph(self.writer, Glyph::High)?;
                write!(self.writer, " {}", Grouped(health.high() as usize))?;
            }
            if health.watch() > 0 {
                if health.high() > 0 {
                    write!(self.writer, " · ")?;
                }
                self.theme.write_glyph(self.writer, Glyph::Watch)?;
                write!(self.writer, " {}", Grouped(health.watch() as usize))?;
            }
            writeln!(self.writer)?;
        }
        let gaps = coverage.recovered_files()
            + coverage.unsupported_files()
            + coverage.failed_files()
            + coverage.context_files();
        if gaps > 0 {
            self.theme.write_glyph(self.writer, Glyph::Warning)?;
            writeln!(
                self.writer,
                " {} were not included in quality.",
                Counted::new(gaps as usize, "source file", "source files")
            )?;
        }
        Ok(())
    }

    fn write_change(&mut self, scope: &Scope) -> io::Result<()> {
        let diff = scope.diff();
        writeln!(self.writer)?;
        self.heading_line("QUALITY")?;
        let mut separator = false;
        for (value, glyph) in [
            (diff.worse(), Glyph::Worse),
            (diff.better(), Glyph::Better),
            (diff.changed(), Glyph::Changed),
        ] {
            if value == 0 {
                continue;
            }
            if separator {
                write!(self.writer, " · ")?;
            }
            self.theme.write_glyph(self.writer, glyph)?;
            write!(self.writer, " {}", Grouped(value as usize))?;
            separator = true;
        }
        if !separator {
            write!(self.writer, "No changes")?;
        }
        writeln!(self.writer)
    }

    fn write_codebase_areas(&mut self, view: &Presentation<'_>) -> io::Result<()> {
        if view.areas.len() < 2 {
            return Ok(());
        }
        writeln!(self.writer)?;
        self.heading_line("AREAS")?;
        match self.layout {
            Layout::Stacked => self.write_codebase_cards(&view.areas)?,
            Layout::Full | Layout::Compact => self.write_codebase_table(&view.areas)?,
        }
        Ok(())
    }

    fn write_codebase_table(&mut self, areas: &[&Scope]) -> io::Result<()> {
        let available = self.options.width.saturating_sub(16).max(12);
        let label_width = areas
            .iter()
            .map(|area| UnicodeWidthStr::width(terminal_path(area.name())))
            .max()
            .unwrap_or(12)
            .min(available)
            .max(12);
        for area in areas {
            let health = area.health();
            let label = middle_truncate(terminal_path(area.name()), label_width);
            write!(self.writer, "{}  ", Padded::new(&label, label_width))?;
            self.write_status_count(Glyph::High, health.high())?;
            write!(self.writer, "  ")?;
            self.write_status_count(Glyph::Watch, health.watch())?;
            writeln!(self.writer)?;
        }
        Ok(())
    }

    fn write_codebase_cards(&mut self, areas: &[&Scope]) -> io::Result<()> {
        for area in areas {
            let health = area.health();
            writeln!(
                self.writer,
                "{}",
                middle_truncate(terminal_path(area.name()), self.options.width)
            )?;
            write!(self.writer, "  ")?;
            self.write_status_count(Glyph::High, health.high())?;
            write!(self.writer, "  ")?;
            self.write_status_count(Glyph::Watch, health.watch())?;
            writeln!(self.writer)?;
        }
        Ok(())
    }

    fn write_diff_areas(&mut self, view: &Presentation<'_>) -> io::Result<()> {
        if view.areas.len() < 2 {
            return Ok(());
        }
        writeln!(self.writer)?;
        self.heading_line("AREAS")?;
        match self.layout {
            Layout::Stacked => self.write_diff_cards(&view.areas)?,
            Layout::Full | Layout::Compact => self.write_diff_table(&view.areas)?,
        }
        Ok(())
    }

    fn write_diff_table(&mut self, areas: &[&Scope]) -> io::Result<()> {
        let available = self.options.width.saturating_sub(24).max(12);
        let label_width = areas
            .iter()
            .map(|area| UnicodeWidthStr::width(terminal_path(area.name())))
            .max()
            .unwrap_or(12)
            .min(available)
            .max(12);
        for area in areas {
            let diff = area.diff();
            let label = middle_truncate(terminal_path(area.name()), label_width);
            write!(self.writer, "{}  ", Padded::new(&label, label_width))?;
            self.write_status_count(Glyph::Worse, diff.worse())?;
            write!(self.writer, "  ")?;
            self.write_status_count(Glyph::Better, diff.better())?;
            write!(self.writer, "  ")?;
            self.write_status_count(Glyph::Changed, diff.changed())?;
            writeln!(self.writer)?;
        }
        Ok(())
    }

    fn write_diff_cards(&mut self, areas: &[&Scope]) -> io::Result<()> {
        for area in areas {
            let diff = area.diff();
            writeln!(
                self.writer,
                "{}",
                middle_truncate(terminal_path(area.name()), self.options.width)
            )?;
            write!(self.writer, "  ")?;
            self.write_status_count(Glyph::Worse, diff.worse())?;
            write!(self.writer, "  ")?;
            self.write_status_count(Glyph::Better, diff.better())?;
            write!(self.writer, "  ")?;
            self.write_status_count(Glyph::Changed, diff.changed())?;
            writeln!(self.writer)?;
        }
        Ok(())
    }

    fn write_details(&mut self, view: &Presentation<'_>) -> io::Result<()> {
        match view.report.mode() {
            ReportMode::Codebase => self.write_findings(view),
            ReportMode::Diff => self.write_comparisons(view),
        }
    }

    fn write_findings(&mut self, view: &Presentation<'_>) -> io::Result<()> {
        if view.findings.is_empty() {
            return Ok(());
        }
        writeln!(self.writer)?;
        self.heading_line("FINDINGS")?;
        for finding in &view.findings {
            let glyph = if finding.assessment().rating() == Rating::High {
                Glyph::High
            } else {
                Glyph::Watch
            };
            self.theme.write_glyph(self.writer, glyph)?;
            write!(self.writer, "  ")?;
            let identity = if let Some(container) = finding.identity().container() {
                format!("{container}::{}", finding.identity().name())
            } else {
                finding.identity().name().to_owned()
            };
            if self.layout == Layout::Stacked {
                writeln!(
                    self.writer,
                    "{}",
                    middle_truncate(&identity, self.options.width.saturating_sub(3))
                )?;
                write!(
                    self.writer,
                    "        {}",
                    unit_kind_label(finding.identity().kind())
                )?;
                self.write_evidence_suffix(Some(finding.role()), Some(finding.trust()))?;
                writeln!(self.writer)?;
            } else {
                write!(
                    self.writer,
                    "{identity} · {}",
                    unit_kind_label(finding.identity().kind())
                )?;
                self.write_evidence_suffix(Some(finding.role()), Some(finding.trust()))?;
                writeln!(self.writer)?;
            }
            let file = &view.report.files()[finding.file().index()];
            let line = finding.span().start_line().to_string();
            writeln!(
                self.writer,
                "        {}:{}",
                middle_truncate(
                    file.path(),
                    self.options.width.saturating_sub(9 + line.len())
                ),
                line
            )?;
            if self.layout == Layout::Stacked {
                for signal in finding
                    .assessment()
                    .signals()
                    .iter()
                    .filter(|signal| signal.rating() != Rating::Healthy)
                {
                    writeln!(
                        self.writer,
                        "        {} {}",
                        signal_name(signal.signal()),
                        signal.value()
                    )?;
                }
                if let Some(activity) = file.activity().filter(|activity| activity.touches() > 0) {
                    writeln!(
                        self.writer,
                        "        {}",
                        Counted::new(activity.touches() as usize, "commit", "commits")
                    )?;
                }
            } else {
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
                        Counted::new(activity.touches() as usize, "commit", "commits")
                    )?;
                }
                writeln!(self.writer)?;
            }
        }
        Ok(())
    }

    fn write_comparisons(&mut self, view: &Presentation<'_>) -> io::Result<()> {
        if view.comparisons.is_empty() {
            return Ok(());
        }
        writeln!(self.writer)?;
        self.heading_line("FINDINGS")?;
        for comparison in &view.comparisons {
            let glyph = match comparison.direction() {
                ComparisonDirection::Worse => Glyph::Worse,
                ComparisonDirection::Better => Glyph::Better,
                ComparisonDirection::Changed => Glyph::Changed,
            };
            self.theme.write_glyph(self.writer, glyph)?;
            write!(self.writer, "  ")?;
            if let Some(container) = comparison.identity().container() {
                write!(self.writer, "{container}::")?;
            }
            writeln!(self.writer, "{}", comparison.identity().name())?;
            if let Some(file) = comparison.file() {
                writeln!(
                    self.writer,
                    "        {}",
                    middle_truncate(
                        view.report.files()[file.index()].path(),
                        self.options.width.saturating_sub(8)
                    )
                )?;
            }
            match comparison.kind() {
                ComparisonKind::Added => writeln!(self.writer, "        added")?,
                ComparisonKind::Removed => writeln!(self.writer, "        removed")?,
                ComparisonKind::Ambiguous => {
                    writeln!(self.writer, "        identity could not be matched safely")?
                }
                _ if self.layout == Layout::Stacked => {
                    self.write_changed_measurement_lines(comparison)?;
                }
                _ => {
                    write!(self.writer, "        ")?;
                    self.write_changed_measurements(comparison)?;
                    writeln!(self.writer)?;
                }
            }
        }
        Ok(())
    }

    fn write_changed_measurement_lines(&mut self, comparison: &Comparison) -> io::Result<()> {
        let (Some(before), Some(after)) = (comparison.before(), comparison.after()) else {
            return writeln!(
                self.writer,
                "        {}",
                comparison_name(comparison.kind())
            );
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
        ];
        let mut changed = false;
        for (name, before, after) in values
            .into_iter()
            .filter(|(_, before, after)| before != after)
        {
            writeln!(self.writer, "        {name} {before} → {after}")?;
            changed = true;
        }
        if !changed {
            writeln!(
                self.writer,
                "        {}",
                comparison_name(comparison.kind())
            )?;
        }
        Ok(())
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
            ("statements", before.logical_lines(), after.logical_lines()),
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

    fn write_diagnostics(&mut self, view: &Presentation<'_>) -> io::Result<()> {
        let report = view.report;
        let selected = view.selected;
        let detail =
            self.options.all || selected.is_some_and(|scope| scope.kind() != ScopeKind::Repository);
        let relevant = |diagnostic: &Diagnostic| {
            diagnostic.file().is_none_or(|file| {
                selected.is_none_or(|scope| file_belongs_to_scope(report, file, scope))
            })
        };
        let mut warnings = Vec::new();
        let history = report.history_coverage();
        if matches!(
            history.availability(),
            smackdebt_analysis::HistoryAvailability::Incomplete
        ) {
            warnings.push("History is incomplete.".to_owned());
        } else if matches!(
            history.availability(),
            smackdebt_analysis::HistoryAvailability::Unavailable
        ) {
            warnings.push("History is unavailable.".to_owned());
        }
        if history.rename_gaps() > 0 {
            warnings.push("Some renamed files could not be matched.".to_owned());
        }
        let unresolved = report
            .resolution_diagnostics()
            .iter()
            .filter(|diagnostic| {
                selected.is_none_or(|scope| file_belongs_to_scope(report, diagnostic.file(), scope))
                    && matches!(
                        diagnostic.kind(),
                        smackdebt_analysis::ResolutionIssueKind::Unresolved
                    )
            })
            .count();
        if unresolved > 0 {
            warnings.push(format!(
                "{} could not be matched.",
                Counted::new(unresolved, "import", "imports")
            ));
        }
        let ambiguous = report
            .resolution_diagnostics()
            .iter()
            .filter(|diagnostic| {
                selected.is_none_or(|scope| file_belongs_to_scope(report, diagnostic.file(), scope))
                    && matches!(
                        diagnostic.kind(),
                        smackdebt_analysis::ResolutionIssueKind::Ambiguous
                    )
            })
            .count();
        if ambiguous > 0 {
            warnings.push(format!(
                "{} matched more than one file.",
                Counted::new(ambiguous, "import", "imports")
            ));
        }
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
        if warnings.is_empty() {
            return Ok(());
        }
        writeln!(self.writer)?;
        self.heading_line("WARNINGS")?;
        for warning in warnings {
            self.theme.write_glyph(self.writer, Glyph::Warning)?;
            writeln!(self.writer, " {warning}")?;
        }
        if detail {
            for diagnostic in report
                .diagnostics()
                .iter()
                .filter(|diagnostic| relevant(diagnostic))
                .filter(|diagnostic| !diagnostic.message().starts_with("Git history"))
            {
                if let Some(file) = diagnostic.file() {
                    writeln!(
                        self.writer,
                        "  {}: {}",
                        report.files()[file.index()].path(),
                        diagnostic.message()
                    )?;
                }
            }
        }
        Ok(())
    }

    fn write_status_count(&mut self, glyph: Glyph, value: u32) -> io::Result<()> {
        self.theme.write_glyph(self.writer, glyph)?;
        if value == 0 {
            write!(self.writer, " –")
        } else {
            write!(self.writer, " {}", Grouped(value as usize))
        }
    }

    fn heading_line(&mut self, heading: &str) -> io::Result<()> {
        writeln!(self.writer, "{heading}")
    }
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

fn package_name(report: &Report, package_index: usize) -> Option<&str> {
    report
        .packages()
        .get(package_index)
        .map(|package| terminal_path(package.path()))
}

fn terminal_path(path: &str) -> &str {
    if path == "." { "repository root" } else { path }
}

fn unit_kind_label(kind: smackdebt_analysis::UnitKind) -> &'static str {
    match kind {
        smackdebt_analysis::UnitKind::Function => "function",
        smackdebt_analysis::UnitKind::Method => "method",
        smackdebt_analysis::UnitKind::Closure => "closure",
        smackdebt_analysis::UnitKind::Lambda => "lambda",
        smackdebt_analysis::UnitKind::SyntheticTopLevel => "top level",
        smackdebt_analysis::UnitKind::Template => "template",
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Glyph {
    High,
    Watch,
    Discover,
    Worse,
    Better,
    Changed,
    Warning,
}

impl Glyph {
    const fn value(self) -> char {
        match self {
            Self::High => '\u{f024}',
            Self::Watch => '\u{f0eb}',
            Self::Discover => '\u{f46b}',
            Self::Worse => '\u{f062}',
            Self::Better => '\u{f063}',
            Self::Changed => '\u{f111}',
            Self::Warning => '\u{f071}',
        }
    }

    fn style(self) -> Option<Style> {
        match self {
            Self::High | Self::Worse => Some(Style::new().fg_color(Some(AnsiColor::Red.into()))),
            Self::Watch | Self::Warning => {
                Some(Style::new().fg_color(Some(Ansi256Color(208).into())))
            }
            Self::Discover => Some(Style::new().fg_color(Some(AnsiColor::Cyan.into()))),
            Self::Better => Some(Style::new().fg_color(Some(AnsiColor::Green.into()))),
            Self::Changed => None,
        }
    }
}

struct Theme {
    enabled: bool,
}

impl Theme {
    fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    fn write_glyph(&self, writer: &mut impl Write, glyph: Glyph) -> io::Result<()> {
        if self.enabled
            && let Some(style) = glyph.style()
        {
            write!(
                writer,
                "{}{}{}",
                style.render(),
                glyph.value(),
                style.render_reset()
            )
        } else {
            write!(writer, "{}", glyph.value())
        }
    }
}

/// Whether two coupling rows describe the same unordered package pair.
///
/// One pair is rendered once, with the finding's operands when it has a
/// finding, so no view can print the same pair with different numbers.
fn same_coupling_pair(
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

fn evidence_suffix(role: Option<SourceRole>, trust: Option<SourceTrust>) -> String {
    let mut suffix = String::new();
    if let Some(role) = role.filter(|role| *role != SourceRole::Primary) {
        suffix.push_str(" · ");
        suffix.push_str(history_role_name(role));
    }
    if trust == Some(SourceTrust::Advisory) {
        suffix.push_str(" · advisory");
    }
    suffix
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
        left_file
            .activity()
            .map_or(0, |activity| activity.touches()),
        left_file.path(),
    )
    .cmp(&smackdebt_analysis::FindingRank::new(
        right,
        right_file
            .activity()
            .map_or(0, |activity| activity.touches()),
        right_file.path(),
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
        ArchitectureFinding, ArchitectureFindingId, ArchitectureFindingKind, ArchitectureGraph,
        ArchitectureReportFacts, ChangeCoupling, Coverage, DependencyCoverage, DependencyEdge,
        DependencyEdgeId, EvolutionaryFinding, EvolutionaryFindingId, EvolutionaryReportFacts,
        FileActivity, FileId, FileRecord, FindingId, HealthCounts, HealthPolicy, HistoryCoverage,
        Measurements, PackageId, PackageRecord, ParseStatus, Report, ReportBuilder, ReportMode,
        Scope, ScopeId, SourceRole, SourceSpan, SourceTrust, UnitIdentity,
    };

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
                UnitIdentity::new(
                    format!("unit-{index}"),
                    smackdebt_analysis::UnitKind::Function,
                ),
                SourceSpan::new(1, 2),
                measurements,
                policy.assess(measurements),
            ));
            builder.link_finding(root, FindingId::from_index(index));
        }
        builder.finish()
    }

    #[test]
    fn package_labels_come_from_the_package_table_not_scope_position() {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        let directory = ScopeId::from_index(1);
        let package_scope = ScopeId::from_index(2);
        builder.add_scope(Scope::new(root, ScopeKind::Repository, ".", None));
        builder.add_scope(Scope::new(
            directory,
            ScopeKind::Directory,
            "unrelated",
            Some(root),
        ));
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
    fn machine_root_dot_is_presented_as_repository_root() {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        builder.add_scope(Scope::new(root, ScopeKind::Repository, ".", None));
        builder.set_root(root);
        let report = builder.finish();
        let mut output = Vec::new();
        write_terminal(
            &mut output,
            &report,
            report.root(),
            TerminalOptions::default(),
        )
        .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.starts_with("smackdebt codebase\nrepository root\n"));
        assert_eq!(report.scopes()[0].name(), ".");
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
                UnitIdentity::new("broken", smackdebt_analysis::UnitKind::Function),
                SourceSpan::new(1, 4),
                Measurements::new(25, 3, 4),
                HealthPolicy::default().assess(Measurements::new(25, 3, 4)),
            )
            .with_evidence(SourceRole::Benchmark, SourceTrust::Advisory),
        );
        builder.link_finding(root, finding);
        let report = builder.finish();
        assert!(
            Presentation::new(&report, report.root(), false)
                .findings
                .is_empty()
        );
        assert_eq!(
            Presentation::new(&report, report.root(), true)
                .findings
                .len(),
            1
        );
        let mut output = Vec::new();
        write_terminal(
            &mut output,
            &report,
            report.root(),
            TerminalOptions {
                all: true,
                ..TerminalOptions::default()
            },
        )
        .unwrap();
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("broken · function · benchmark · advisory")
        );
    }

    #[test]
    fn empty_report_is_valid_json_and_stable_terminal_text() {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        builder.add_scope(Scope::new(root, ScopeKind::Repository, ".", None));
        builder.set_root(root);
        let report = builder.finish();

        let mut terminal = Vec::new();
        write_terminal(
            &mut terminal,
            &report,
            report.root(),
            TerminalOptions::default(),
        )
        .unwrap();
        let terminal = String::from_utf8(terminal).unwrap();
        assert!(terminal.contains("QUALITY"));
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
    fn architecture_cycle_uses_the_ordered_closed_edge_witness() {
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
                let edge = DependencyEdge::new(
                    DependencyEdgeId::from_index(index),
                    FileId::from_index(source),
                    FileId::from_index(target),
                    1,
                    vec![SourceSpan::new(1, 1)],
                );
                if index == 0 {
                    edge.with_evidence(SourceRole::Test, SourceTrust::Advisory)
                } else {
                    edge
                }
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
        let mut terminal = Vec::new();
        write_terminal(
            &mut terminal,
            &report,
            report.root(),
            TerminalOptions::default(),
        )
        .unwrap();
        let terminal = String::from_utf8(terminal).unwrap();
        assert!(terminal.contains("        b.js → c.js → a.js → b.js\n"));
        assert!(!terminal.contains("        c.js → a.js → b.js\n"));

        let mut detail = Vec::new();
        write_terminal(
            &mut detail,
            &report,
            report.root(),
            TerminalOptions {
                all: true,
                ..TerminalOptions::default()
            },
        )
        .unwrap();
        let detail = String::from_utf8(detail).unwrap();
        assert!(detail.contains("b.js → c.js · 1 import · test · advisory"));
        assert!(detail.contains("c.js → a.js · 1 import\n"));
        assert!(!detail.contains("primary"));
        assert!(!detail.contains("trusted"));
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

        let render = |all| {
            let mut terminal = Vec::new();
            write_terminal(
                &mut terminal,
                &report,
                report.root(),
                TerminalOptions {
                    all,
                    ..TerminalOptions::default()
                },
            )
            .unwrap();
            String::from_utf8(terminal).unwrap()
        };
        let default = render(false);
        assert_eq!(default.matches('').count(), 3, "{default}");
        assert!(
            default
                .contains("b ↔ c changed together in 8 of 10 commits · 80% · no code dependency")
        );
        assert!(
            default.contains("c ↔ d changed together in 3 of 4 commits · 75% · no code dependency")
        );
        assert!(
            default.contains("a ↔ b changed together in 3 of 5 commits · 60% · no code dependency")
        );
        assert!(!default.contains("d ↔ e"));
        assert!(
            default.find("b ↔ c").unwrap() < default.find("c ↔ d").unwrap(),
            "{default}"
        );
        assert!(
            default.find("c ↔ d").unwrap() < default.find("a ↔ b").unwrap(),
            "{default}"
        );
        let detailed = render(true);
        assert_eq!(detailed.matches('').count(), 4, "{detailed}");
        assert!(
            detailed
                .contains("d ↔ e changed together in 3 of 5 commits · 60% · no code dependency")
        );
    }

    #[test]
    fn activity_changes_rank_and_keeps_three_findings() {
        let report = report_with_findings(true);
        let mut terminal = Vec::new();
        write_terminal(
            &mut terminal,
            &report,
            report.root(),
            TerminalOptions::default(),
        )
        .unwrap();
        let terminal = String::from_utf8(terminal).unwrap();
        assert!(terminal.contains("FINDINGS"));
        assert!(terminal.contains("file-10.rs"));
        assert!(!terminal.contains("file-1.rs"));
    }

    #[test]
    fn absent_activity_keeps_the_findings_heading() {
        let report = report_with_findings(false);
        let mut terminal = Vec::new();
        write_terminal(
            &mut terminal,
            &report,
            report.root(),
            TerminalOptions::default(),
        )
        .unwrap();
        let terminal = String::from_utf8(terminal).unwrap();
        assert!(terminal.contains("FINDINGS"));
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
    fn grouped_counts_and_nouns_are_readable() {
        assert_eq!(Grouped(19_761).to_string(), "19,761");
        assert_eq!(Counted::new(1, "file", "files").to_string(), "1 file");
        assert_eq!(Counted::new(2, "file", "files").to_string(), "2 files");
    }

    #[test]
    fn width_writer_limits_unicode_lines_without_corrupting_glyph_styling() {
        let mut output = Vec::new();
        {
            let mut writer = WidthWriter::new(&mut output, 12);
            writeln!(writer, "\u{1b}[31m\u{1b}[0m  very-long-visible-line").unwrap();
            writer.finish().unwrap();
            assert_eq!(writer.truncations(), 1);
        }
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("\u{1b}[31m\u{1b}[0m"));
        assert!(output.ends_with("…\n"));
        assert!(output.lines().all(|line| ansi_display_width(line) <= 12));
    }

    #[test]
    fn full_compact_and_stacked_layouts_keep_the_same_area_facts() {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        let package = ScopeId::from_index(1);
        let debt = ScopeId::from_index(2);
        let quiet = ScopeId::from_index(3);
        let mut root_scope = Scope::new(root, ScopeKind::Repository, ".", None);
        root_scope.add_child(package);
        builder.add_scope(root_scope);
        let mut package_scope = Scope::new(package, ScopeKind::Package, ".", Some(root));
        package_scope.add_child(debt);
        package_scope.add_child(quiet);
        builder.add_scope(package_scope);
        builder.add_scope(Scope::new(
            debt,
            ScopeKind::Directory,
            "src/very-long-feature-name",
            Some(package),
        ));
        builder.add_scope(Scope::new(
            quiet,
            ScopeKind::Directory,
            "tests",
            Some(package),
        ));
        builder.set_root(root);
        for (index, scope, health) in [
            (0, debt, HealthCounts::new(8, 2, 1)),
            (1, quiet, HealthCounts::new(4, 1, 0)),
        ] {
            let file = FileRecord::new(
                FileId::from_index(index),
                scope,
                format!("file-{index}.rs"),
                Coverage::new(1, 1, 0, 0, 1_234, 0),
                health,
            );
            builder.link_file(scope, file.id());
            builder.add_file(file);
        }
        let report = builder.finish();

        let render = |width| {
            let mut bytes = Vec::new();
            write_terminal(
                &mut bytes,
                &report,
                report.root(),
                TerminalOptions::new(width, false, false),
            )
            .unwrap();
            String::from_utf8(bytes).unwrap()
        };
        let full = render(120);
        let compact = render(80);
        let stacked = render(50);
        for output in [&full, &compact, &stacked] {
            assert!(output.contains("src/very-long-feature-name"));
            assert!(output.contains(Glyph::High.value()));
            assert!(output.contains(Glyph::Watch.value()));
            assert!(!output.contains('%'));
            assert!(!output.contains("░"));
            assert!(!output.contains("healthy"));
        }
        assert!(full.contains("AREAS"));
        assert!(compact.contains("AREAS"));
        assert!(stacked.contains("src/very-long-feature-name\n"));
    }

    #[test]
    fn glyph_vocabulary_has_exact_one_cell_values() {
        let expected = [
            (Glyph::High, '\u{f024}'),
            (Glyph::Watch, '\u{f0eb}'),
            (Glyph::Discover, '\u{f46b}'),
            (Glyph::Worse, '\u{f062}'),
            (Glyph::Better, '\u{f063}'),
            (Glyph::Changed, '\u{f111}'),
            (Glyph::Warning, '\u{f071}'),
        ];
        for (glyph, value) in expected {
            assert_eq!(glyph.value(), value);
            assert_eq!(glyph.value().width(), Some(1));
            assert_ne!(glyph.value(), '\u{ec3f}');
        }
    }

    #[test]
    fn glyph_styles_have_exact_colors_and_changed_is_unstyled() {
        for (glyph, expected) in [
            (Glyph::High, "\u{1b}[31m"),
            (Glyph::Watch, "\u{1b}[38;5;208m"),
            (Glyph::Discover, "\u{1b}[36m"),
            (Glyph::Worse, "\u{1b}[31m"),
            (Glyph::Better, "\u{1b}[32m"),
            (Glyph::Warning, "\u{1b}[38;5;208m"),
        ] {
            assert_eq!(glyph.style().unwrap().render().to_string(), expected);
        }
        assert!(Glyph::Changed.style().is_none());
        let mut changed = Vec::new();
        Theme::new(true)
            .write_glyph(&mut changed, Glyph::Changed)
            .unwrap();
        assert_eq!(changed, Glyph::Changed.value().to_string().as_bytes());
    }

    #[test]
    fn removing_ansi_from_styled_output_reproduces_plain_output() {
        let report = report_with_findings(true);
        let render = |color| {
            let mut bytes = Vec::new();
            write_terminal(
                &mut bytes,
                &report,
                report.root(),
                TerminalOptions::new(80, false, color),
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

        let path = drill_path_from_visible(
            &report,
            &report.scopes()[package.index()],
            Some(&report.scopes()[child.index()]),
        );

        assert_eq!(path, Some(PathBuf::from("/work/project/bow/src")));
    }
}
