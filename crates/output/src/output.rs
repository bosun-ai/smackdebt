//! Stable terminal and JSON views over the shared report.

use std::cmp::Reverse;
use std::fmt;
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use anstyle::{Ansi256Color, AnsiColor, Style};
use smackdebt_analysis::{
    ArchitectureComparisonKind, ArchitectureFindingId, CodebaseTier, Comparison,
    ComparisonDirection, ComparisonKind, CouplingLink, DebtDiffSelection, DebtFamily,
    DependencyEdgeId, Diagnostic, DiagnosticKind, DiffTier, EvolutionaryFindingId, FileId,
    FileRecord, Finding, FindingId, Instability, KnowledgeConcentrationFindingId, Language,
    Measurements, PackageId, ProblemAnchor, ProblemCard, ProblemEvidence, ProblemPattern,
    ProblemVisibility, Rating, Report, ReportMode, ResolutionIssueKind, Scope, ScopeId, ScopeKind,
    Signal, SizeFinding, SizeFindingId, SourceRole, SourceTrust, StableDependencyFindingId,
    UnitKind, Verdict, instability, qualifies_for_finding,
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
    /// A caller-chosen limit on displayed findings or comparisons, replacing
    /// the default of three; every other section keeps its own limit.
    top: Option<NonZeroUsize>,
}

impl TerminalOptions {
    /// Words-only options, which is what a pipe receives.
    pub const fn new(width: usize, all: bool, color: bool) -> Self {
        Self {
            width,
            all,
            color,
            decorations: false,
            top: None,
        }
    }

    /// Adds glyph and tier-bar decoration, resolved by the caller from
    /// terminal detection.
    pub const fn with_decorations(mut self, decorations: bool) -> Self {
        self.decorations = decorations;
        self
    }

    /// Limits displayed findings or comparisons to the given count instead of
    /// the default of three.
    pub const fn with_top(mut self, top: Option<NonZeroUsize>) -> Self {
        self.top = top;
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
            top: None,
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
    let presentation = Presentation::new(report, selected_scope, options.all, options.top);
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
    /// Codebase debt detail, which is one ranked section of named problems.
    problems: Section,
    /// Diff debt detail, which keeps its three sections this round.
    findings: Section,
    architecture: Section,
    history: Section,
    warnings: Section,
    /// Per-file diagnostic context, kept for `--all` and a file scope.
    warning_detail: Vec<String>,
    next: Option<String>,
    /// Whether a clean diff suppresses every section after the verdict.
    verdict_only: bool,
}

impl Presentation {
    fn new(
        report: &Report,
        selected_scope: Option<ScopeId>,
        all: bool,
        top: Option<NonZeroUsize>,
    ) -> Self {
        let selected = selected_scope
            .or_else(|| report.root())
            .and_then(|id| report.scopes().get(id.index()));
        let Some(selected) = selected else {
            return Self {
                mode: report.mode(),
                scope_label: terminal_path(".").to_owned(),
                verdict: Verdict::default(),
                areas: Section::new("AREAS"),
                problems: Section::new("PROBLEMS"),
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
        // A relationship that could not be followed is file detail: it reaches
        // a reader with `--all` or at a file scope, and at every other scope
        // the grouped warning sentence is its whole terminal presence.
        let file_detail = all || selected.kind() == ScopeKind::File;
        // History context still belongs to a path view, which is the view it
        // exists to explain.
        let detail = all || selected.kind() != ScopeKind::Repository;
        let verdict_only = report.mode() == ReportMode::Diff
            && verdict.diff_tier() == Some(DiffTier::NoDebtChange);

        let areas = area_rows(report, displayed);
        let codebase = report.mode() == ReportMode::Codebase;
        // Codebase debt is one ranked section of named problems; a diff keeps
        // its three sections this round.
        let problems = if codebase {
            problem_rows(report, displayed, selected, all, top)
        } else {
            Section::new("PROBLEMS")
        };
        let (findings, architecture, history) = if codebase {
            (
                Section::new("FINDINGS"),
                Section::new("ARCHITECTURE"),
                Section::new("HISTORY"),
            )
        } else {
            (
                diff_finding_rows(report, displayed, all, top, selection),
                diff_architecture_rows(report, verdict.selection()),
                history_rows(report, selected, detail, verdict.selection()),
            )
        };
        let (warnings, warning_detail) = warning_rows(report, selected, file_detail);
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
            problems,
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

/// How many ranked findings or comparisons the default view shows; a
/// caller-chosen limit replaces only this count, never another section's.
fn finding_limit(top: Option<NonZeroUsize>) -> usize {
    top.map_or(3, NonZeroUsize::get)
}

/// The slots one codebase view spends on its problem section.
///
/// A card costs one slot plus one slot per shown evidence line, so the budget
/// bounds what a view states rather than how many lines a width renders it on.
///
/// This is a proposed value under review.
const SCREEN_BUDGET: usize = 24;

/// The card counts the budget is spent through, in the order they are tried.
///
/// A scope with few problems shows each in depth and a scope with many shows
/// more of them with less evidence each.
///
/// These are proposed values under review.
const PROBLEM_LADDER: [usize; 4] = [6, 8, 12, 24];

/// The rung a scope holding more cards than any rung falls back to, which is
/// also the rung that cuts the table at the budget.
const LAST_RUNG: usize = PROBLEM_LADDER[PROBLEM_LADDER.len() - 1];

/// The evidence lines a rung of `cards` allows.
///
/// Each card pays one slot for its head, so what is left of the budget is the
/// depth every card of that rung can afford; every rung therefore costs
/// exactly [`SCREEN_BUDGET`] by construction.
const fn evidence_allowance(cards: usize) -> usize {
    SCREEN_BUDGET / cards - 1
}

/// The first rung whose card count is at least `cards`, with its allowance.
fn ladder_rung(cards: usize) -> (usize, usize) {
    let limit = PROBLEM_LADDER
        .into_iter()
        .find(|limit| *limit >= cards)
        .unwrap_or(LAST_RUNG);
    (limit, evidence_allowance(limit))
}

/// How much of the ranked problem table one view shows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ProblemDetail {
    cards: usize,
    evidence: usize,
}

impl ProblemDetail {
    /// Resolves the detail level from the request and the cards in scope.
    fn resolve(all: bool, file_scope: bool, top: Option<NonZeroUsize>, cards: usize) -> Self {
        // `--top` names how many cards a view shows; `--all` and a selected
        // file show every card in scope, and the budget otherwise cuts the
        // table at its last rung.
        let shown = match top {
            Some(top) => top.get().min(cards),
            None if all || file_scope => cards,
            None => cards.min(ladder_rung(cards).0),
        };
        // `--all` and a selected file both ask for complete evidence, because
        // drilling to a file is itself a request for detail; every other view
        // buys breadth with the depth its own rung allows.
        let evidence = if all || file_scope {
            usize::MAX
        } else {
            ladder_rung(top.map_or(cards, NonZeroUsize::get)).1
        };
        Self {
            cards: shown,
            evidence,
        }
    }
}

/// The ranked problem cards the displayed scope holds, cut to the budget.
///
/// The table is ordered once by analysis, so this filters and truncates it and
/// never sorts.
fn problem_rows(
    report: &Report,
    displayed: &Scope,
    selected: &Scope,
    all: bool,
    top: Option<NonZeroUsize>,
) -> Section {
    let mut section = Section::new("PROBLEMS");
    let cards: Vec<&ProblemCard> = report
        .problems()
        .iter()
        .filter(|card| card_belongs_to_scope(report, card, displayed))
        .filter(|card| shows_card(report, card, selected, all))
        .collect();
    let detail = ProblemDetail::resolve(all, selected.kind() == ScopeKind::File, top, cards.len());
    section.rows = cards
        .into_iter()
        .take(detail.cards)
        .map(|card| problem_row(report, card, detail.evidence))
        .collect();
    section
}

/// Whether the current detail level shows this card.
///
/// A `detail` card is removed from a view rather than moved inside it, so the
/// cards that remain keep the order the problem rank gave them.
fn shows_card(report: &Report, card: &ProblemCard, selected: &Scope, all: bool) -> bool {
    all || card.visibility() == ProblemVisibility::Default
        || anchor_is_scope(report, card.anchor(), selected)
}

/// Whether a card's anchor is the selected scope itself, which is the one
/// place a `detail` card reaches a default view.
fn anchor_is_scope(report: &Report, anchor: &ProblemAnchor, selected: &Scope) -> bool {
    match anchor {
        ProblemAnchor::File(file) => {
            selected.kind() == ScopeKind::File && file_belongs_to_scope(report, *file, selected)
        }
        ProblemAnchor::Package(package) => report
            .packages()
            .get(package.index())
            .is_some_and(|record| record.scope() == selected.id()),
        ProblemAnchor::Files(_) | ProblemAnchor::PackagePair(..) => false,
    }
}

/// Whether a card belongs to a scope, which its anchor decides.
fn card_belongs_to_scope(report: &Report, card: &ProblemCard, scope: &Scope) -> bool {
    match card.anchor() {
        ProblemAnchor::File(file) => file_belongs_to_scope(report, *file, scope),
        ProblemAnchor::Files(files) => files
            .iter()
            .any(|file| file_belongs_to_scope(report, *file, scope)),
        ProblemAnchor::Package(package) => package_meets_scope(report, *package, scope),
        ProblemAnchor::PackagePair(left, right) => {
            package_meets_scope(report, *left, scope) || package_meets_scope(report, *right, scope)
        }
    }
}

/// Whether a package lies within the selected scope or contains it, which is
/// how a package-anchored card reaches the views above and below its package.
fn package_meets_scope(report: &Report, package: PackageId, scope: &Scope) -> bool {
    let Some(record) = report.packages().get(package.index()) else {
        return false;
    };
    scope_within(report, record.scope(), scope.id())
        || scope_within(report, scope.id(), record.scope())
}

/// One card: its rating word, what it is, what it is about, then a prefix of
/// the evidence analysis ordered.
fn problem_row(report: &Report, card: &ProblemCard, evidence: usize) -> Row {
    // A card claiming nothing carries no rating to state, and `watch` would
    // misstate it.
    let word = (card.rating() != Rating::Healthy).then(|| Word::rating(card.rating()));
    // A `measured` card heads on its first fact, so that fact adds only what
    // the head has not already stated: each fact reaches a reader once.
    let headed = card.pattern() == ProblemPattern::Measured;
    // The allowance is taken over evidence items, not over rendered lines: a
    // cycle witness is one fact stated one step per line, and eliding steps
    // would destroy the fact rather than shorten it. Its steps are therefore
    // exempt from the slot accounting, so a card carrying a long witness may
    // render past the budget while the cards a scope shows and the evidence
    // each of them states still respect the rung.
    let stacked = card
        .evidence()
        .iter()
        .take(evidence)
        .enumerate()
        .flat_map(|(index, fact)| evidence_lines(report, *fact, headed && index == 0))
        .collect();
    Row::new(word, problem_head(report, card)).with_stacked(stacked)
}

/// The human name of one pattern, which is presentation and never the frozen
/// machine id.
///
/// A `measured` card has no pattern of its own: its head is the identity of
/// the top finding it claimed, which is the head a finding row states.
const fn pattern_name(pattern: ProblemPattern) -> Option<&'static str> {
    match pattern {
        ProblemPattern::GodFile => Some("does too much"),
        ProblemPattern::Hub => Some("everything depends on this"),
        ProblemPattern::Tangle => Some("circular dependency"),
        ProblemPattern::HotMess => Some("hot and complex"),
        ProblemPattern::ShotgunPair => Some("changes together"),
        ProblemPattern::BusRisk => Some("one author"),
        ProblemPattern::UnstableDependency => Some("depends on less stable code"),
        ProblemPattern::Measured => None,
    }
}

fn problem_head(report: &Report, card: &ProblemCard) -> String {
    let anchor = anchor_label(report, card);
    match pattern_name(card.pattern()) {
        Some(name) => format!("{name} · {anchor}"),
        None => measured_head(report, card, &anchor),
    }
}

/// The head of a `measured` card: the identity of its top claimed finding,
/// then the anchor that identity does not already state.
fn measured_head(report: &Report, card: &ProblemCard, anchor: &str) -> String {
    match card.evidence().first() {
        Some(ProblemEvidence::Finding(id)) => report.findings().get(id.index()).map_or_else(
            || anchor.to_owned(),
            |finding| format!("{} · {anchor}", finding_identity(report, finding)),
        ),
        Some(ProblemEvidence::Size(id)) => size_head(report, *id, anchor),
        _ => anchor.to_owned(),
    }
}

/// One finding's identity, its unit kind, and the role and trust that are
/// worth stating, which is the head a finding row states.
fn finding_identity(report: &Report, finding: &Finding) -> String {
    let path = report.files()[finding.file().index()].path();
    format!(
        "{} · {}{}",
        unit_identity(finding.identity(), path),
        unit_kind_label(finding.identity().kind()),
        evidence_suffix(Some(finding.role()), Some(finding.trust()))
    )
}

/// A size finding's head, in the same identity-then-kind form a finding head
/// takes.
///
/// A container is named beside the file it sits in; a file's own length is
/// measured on the anchor itself, so its identity and its anchor are one.
fn size_head(report: &Report, id: SizeFindingId, anchor: &str) -> String {
    match report.size_findings()[id.index()].container() {
        Some(container) => format!("{container} · container · {anchor}"),
        None => format!("{anchor} · file"),
    }
}

/// What a size finding measures, which is a container or the file itself.
fn size_subject(finding: &SizeFinding) -> String {
    match finding.container() {
        Some(container) => format!("{container} · container"),
        None => "file".to_owned(),
    }
}

/// The finding a card heads on when its head is a claimed finding, which is
/// the one head that carries a span.
fn head_finding<'a>(report: &'a Report, card: &ProblemCard) -> Option<&'a Finding> {
    match (card.pattern(), card.evidence().first()) {
        (ProblemPattern::Measured, Some(ProblemEvidence::Finding(id))) => {
            report.findings().get(id.index())
        }
        _ => None,
    }
}

/// What a card is about, written by the kind of its anchor.
fn anchor_label(report: &Report, card: &ProblemCard) -> String {
    match card.anchor() {
        ProblemAnchor::File(file) => {
            let path = report.files()[file.index()].path();
            match head_finding(report, card) {
                Some(finding) => format!("{path}:{}", finding.span().start_line()),
                None => path.to_owned(),
            }
        }
        ProblemAnchor::Files(files) => cycle_anchor(report, card, files),
        ProblemAnchor::Package(package) => package_name(report, package.index())
            .unwrap_or("?")
            .to_owned(),
        ProblemAnchor::PackagePair(left, right) => {
            let left = package_name(report, left.index()).unwrap_or("?");
            let right = package_name(report, right.index()).unwrap_or("?");
            // One relationship is symmetric and the other is not, so each
            // keeps the wording its family already accepted.
            match card.pattern() {
                ProblemPattern::UnstableDependency => format!("{left} → {right}"),
                _ => format!("{left} ↔ {right}"),
            }
        }
    }
}

/// A cycle's first witness path, which is the identity the worst-offender rule
/// already names for a cycle.
fn cycle_anchor(report: &Report, card: &ProblemCard, files: &[FileId]) -> String {
    card.evidence()
        .iter()
        .find_map(|fact| match fact {
            ProblemEvidence::Architecture(id) => report.architecture_findings().get(id.index()),
            _ => None,
        })
        .and_then(|finding| cycle_first_path(report, finding.witness_edges()))
        .or_else(|| {
            files
                .iter()
                .filter_map(|file| report.files().get(file.index()))
                .map(FileRecord::path)
                .min()
        })
        .unwrap_or("?")
        .to_owned()
}

fn cycle_first_path<'a>(report: &'a Report, witness_edges: &[DependencyEdgeId]) -> Option<&'a str> {
    let edge = report
        .dependency_edges()
        .get(witness_edges.first()?.index())?;
    Some(report.files()[edge.source().index()].path())
}

/// One evidence item, stated in the words its kind owns.
///
/// Every kind states one line, except a cycle witness, which is one fact
/// stated one step per line so it is never shortened with an ellipsis, and a
/// fact the card's head already states, which adds only what is left.
fn evidence_lines(report: &Report, fact: ProblemEvidence, headed: bool) -> Vec<String> {
    match fact {
        ProblemEvidence::Finding(id) => finding_evidence(report, id, headed),
        ProblemEvidence::Size(id) => size_evidence(report, id, headed),
        ProblemEvidence::Architecture(id) => witness_evidence(report, id),
        ProblemEvidence::StableDependency(id) => vec![stable_dependency_evidence(report, id)],
        ProblemEvidence::Coupling(id) => vec![coupling_evidence(report, id)],
        ProblemEvidence::Knowledge(id) => vec![knowledge_evidence(report, id)],
        // The rest are integers the report already measured, so they need no
        // finding table to be stated.
        measured => counted_evidence(measured).into_iter().collect(),
    }
}

/// One integer fact, stated in the words its kind owns.
///
/// A fact that links a finding is stated from that finding instead and never
/// reaches here.
fn counted_evidence(fact: ProblemEvidence) -> Option<String> {
    Some(match fact {
        ProblemEvidence::FanIn(value) => fan_in_evidence(value),
        ProblemEvidence::FanOut(value) => format!("imports {}", counted_files(value)),
        // Clustering carries a touch count only for a hotspot, so heat is the
        // only activity a card states.
        ProblemEvidence::Hot(value) => format!(
            "hot ({})",
            Counted::new(value as usize, "commit", "commits")
        ),
        ProblemEvidence::RatedUnits(value) => {
            Counted::new(value as usize, "rated unit", "rated units").to_string()
        }
        ProblemEvidence::Members(value) => format!("{} in the cycle", counted_files(value)),
        _ => return None,
    })
}

/// A count of files, which several evidence kinds state.
const fn counted_files(value: u32) -> Counted {
    Counted::new(value as usize, "file", "files")
}

/// The files that depend on the anchor, whose verb agrees with its count.
fn fan_in_evidence(value: u32) -> String {
    let verb = if value == 1 { "imports" } else { "import" };
    format!("{} {verb} this", counted_files(value))
}

/// The cycle a card anchors, stated one step per line.
fn witness_evidence(report: &Report, id: ArchitectureFindingId) -> Vec<String> {
    report
        .architecture_findings()
        .get(id.index())
        .map(|finding| cycle_witness_steps(report, finding.witness_edges()))
        .unwrap_or_default()
}

/// One claimed finding: where it is, what kind of unit it measures, and the
/// measurements that rated it.
///
/// A card whose head already names this finding states its measurements
/// alone, and states nothing when policy rated it on no reportable value.
fn finding_evidence(report: &Report, id: FindingId, headed: bool) -> Vec<String> {
    let finding = &report.findings()[id.index()];
    let measurements = finding_measurements(finding);
    if headed {
        return if measurements.is_empty() {
            Vec::new()
        } else {
            vec![measurements.join(" · ")]
        };
    }
    let file = &report.files()[finding.file().index()];
    let mut facts = vec![
        format!("{}:{}", file.path(), finding.span().start_line()),
        unit_kind_label(finding.identity().kind()).to_owned(),
    ];
    facts.extend(evidence_facts(Some(finding.role()), Some(finding.trust())));
    facts.extend(measurements);
    vec![facts.join(" · ")]
}

/// The measurements one finding states, which are the signals policy rated.
fn finding_measurements(finding: &Finding) -> Vec<String> {
    finding
        .assessment()
        .signals()
        .iter()
        .filter(|signal| signal.rating() != Rating::Healthy)
        // Cyclomatic complexity starts at one, so a value of one states
        // nothing and never reaches a reader.
        .filter(|signal| signal.signal() != Signal::CyclomaticComplexity || signal.value() != 1)
        .map(|signal| format!("{} {}", signal_name(signal.signal()), signal.value()))
        .collect()
}

fn size_evidence(report: &Report, id: SizeFindingId, headed: bool) -> Vec<String> {
    let finding = &report.size_findings()[id.index()];
    let value = format!("{} lines", Grouped(finding.value() as usize));
    vec![if headed {
        value
    } else {
        format!("{} · {value}", size_subject(finding))
    }]
}

fn stable_dependency_evidence(report: &Report, id: StableDependencyFindingId) -> String {
    let finding = &report.stable_dependency_findings()[id.index()];
    let evidence = finding.evidence();
    let mut facts = Vec::new();
    if let (Some(from), Some(to)) = (
        package_instability(evidence.source()),
        package_instability(evidence.target()),
    ) {
        facts.push(format!(
            "instability {}/{} → {}/{}",
            from.numerator(),
            from.denominator(),
            to.numerator(),
            to.denominator()
        ));
    }
    facts.push(Counted::new(evidence.references() as usize, "import", "imports").to_string());
    facts.join(" · ")
}

fn coupling_evidence(report: &Report, id: EvolutionaryFindingId) -> String {
    let pair = report.evolutionary_findings()[id.index()].coupling();
    let mut facts = vec![
        format!(
            "changed together in {} of {} commits",
            pair.shared_commits(),
            pair.union_commits()
        ),
        format!("{}%", (pair.similarity() * 100.0).round() as u32),
    ];
    // The link is a recorded fact and informs wording only; the finding for an
    // unexplained pair is created regardless of a path.
    match report.coupling_link(pair.left(), pair.right()) {
        CouplingLink::Direct => facts.push("code dependency exists".to_owned()),
        CouplingLink::Indirect(via) => {
            facts.push("no direct dependency".to_owned());
            facts.push(format!(
                "linked via {}",
                package_name(report, via.index()).unwrap_or("?")
            ));
        }
        CouplingLink::None => facts.push("no code dependency".to_owned()),
    }
    facts.join(" · ")
}

fn knowledge_evidence(report: &Report, id: KnowledgeConcentrationFindingId) -> String {
    let concentration = report.knowledge_concentration_findings()[id.index()].concentration();
    format!(
        "one contributor made {} of {} commits",
        Grouped(concentration.numerator() as usize),
        Grouped(concentration.denominator() as usize)
    )
}

fn diff_finding_rows(
    report: &Report,
    displayed: &Scope,
    all: bool,
    top: Option<NonZeroUsize>,
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
        comparisons.truncate(finding_limit(top));
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

/// The signals a card states, in the order policy rates them.
///
/// Every name comes from `signal_name`, so a measurement is named once for the
/// whole writer.
const RATED_SIGNALS: [Signal; 5] = [
    Signal::CognitiveComplexity,
    Signal::CyclomaticComplexity,
    Signal::LogicalLines,
    Signal::MaxNesting,
    Signal::ParameterCount,
];

fn changed_measurements(comparison: &Comparison) -> Vec<String> {
    match comparison.kind() {
        ComparisonKind::Added => return one_sided_measurements("added", comparison.after()),
        ComparisonKind::Removed => return one_sided_measurements("removed", comparison.before()),
        ComparisonKind::Ambiguous => {
            return vec!["identity could not be matched safely".to_owned()];
        }
        _ => {}
    }
    let (Some(before), Some(after)) = (comparison.before(), comparison.after()) else {
        return vec![changed_summary(comparison.kind()).to_owned()];
    };
    let (before, after) = (rated_values(before), rated_values(after));
    let facts: Vec<String> = RATED_SIGNALS
        .into_iter()
        .map(signal_name)
        .zip(before.into_iter().zip(after))
        .filter(|(_, (before, after))| before != after)
        .map(|(name, (before, after))| format!("{name} {before} → {after}"))
        .collect();
    if facts.is_empty() {
        return vec![changed_summary(comparison.kind()).to_owned()];
    }
    facts
}

/// The direction word of a one-sided comparison, then the side that exists.
///
/// An added or removed unit has no other side to compare against, so its card
/// states the absolute values the report already carries. A zero measures
/// nothing worth reading, and a side that is zero everywhere leaves the word
/// alone.
fn one_sided_measurements(word: &str, present: Option<Measurements>) -> Vec<String> {
    let mut facts = vec![word.to_owned()];
    let Some(present) = present else {
        return facts;
    };
    facts.extend(
        RATED_SIGNALS
            .into_iter()
            .map(signal_name)
            .zip(rated_values(present))
            .filter(|(_, value)| *value != 0)
            .map(|(name, value)| format!("{name} {value}")),
    );
    facts
}

/// The rated measurements in the order `RATED_SIGNALS` states them.
const fn rated_values(measurements: Measurements) -> [u32; 5] {
    let (cognitive, cyclomatic, statements, nesting, parameters) = measurements.rated();
    [cognitive, cyclomatic, statements, nesting, parameters]
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

/// The cycle changes a diff moved, which is the only architecture movement
/// that counts as debt.
///
/// Every other edge change is a graph fact the machine report keeps.
fn diff_architecture_rows(report: &Report, selection: &DebtDiffSelection) -> Section {
    let mut section = Section::new("ARCHITECTURE");
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
            Row::new(Some(Word::direction(comparison.direction())), head).with_stacked(witness),
        );
    }
    section
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

fn package_instability(
    measurement: smackdebt_analysis::PackageGraphMeasurement,
) -> Option<Instability> {
    instability(measurement.fan_in(), measurement.fan_out())
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
        let row = Row::new(
            finding.then_some(Word::Watch),
            format!(
                "{left} ↔ {right} changed together in {} of {} commits",
                pair.shared_commits(),
                pair.union_commits()
            ),
        )
        .with_fact(format!("{}%", (pair.similarity() * 100.0).round() as u32));
        // The link is a recorded fact and informs wording only; the Watch
        // finding for an unexplained pair is created regardless of a path.
        let row = match report.coupling_link(pair.left(), pair.right()) {
            CouplingLink::Direct => row.with_fact("code dependency exists"),
            CouplingLink::Indirect(via) => {
                row.with_fact("no direct dependency").with_fact(format!(
                    "linked via {}",
                    package_name(report, via.index()).unwrap_or("?")
                ))
            }
            CouplingLink::None => row.with_fact("no code dependency"),
        };
        section.rows.push(row);
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

/// The grouped diagnostics, and the per-file detail behind them.
///
/// A grouped sentence states its kind at every scope, because a summary is
/// what a reader above a file needs. Per-file rows are the detail `--all` and
/// a selected file ask for.
fn warning_rows(report: &Report, selected: &Scope, file_detail: bool) -> (Section, Vec<String>) {
    let mut section = Section::new("WARNINGS");
    let mut warnings: Vec<Row> = Vec::new();
    let warning = |text: String| Row::new(Some(Word::Warning), text);
    let history = report.history_coverage();
    match history.availability() {
        smackdebt_analysis::HistoryAvailability::Incomplete => {
            warnings.push(warning("History is incomplete.".to_owned()));
        }
        smackdebt_analysis::HistoryAvailability::Unavailable => {
            warnings.push(warning("History is unavailable.".to_owned()));
        }
        smackdebt_analysis::HistoryAvailability::Complete => {
            // A selected window that contains no commits is disclosed rather
            // than left silent, so an evolution-free report is never mistaken
            // for a quiet one.
            if history.eligible_commits() == 0
                && let Some(days) = history.window_days()
            {
                warnings.push(warning(format!(
                    "No commits in the last {}.",
                    Counted::new(days as usize, "day", "days")
                )));
            }
        }
    }
    if history.rename_gaps() > 0 {
        warnings.push(warning(
            "Some renamed files could not be matched.".to_owned(),
        ));
    }
    let mut unresolved = 0usize;
    let mut ambiguous = 0usize;
    for diagnostic in report
        .resolution_diagnostics()
        .iter()
        .filter(|diagnostic| file_belongs_to_scope(report, diagnostic.file(), selected))
    {
        match diagnostic.kind() {
            smackdebt_analysis::ResolutionIssueKind::Unresolved => unresolved += 1,
            smackdebt_analysis::ResolutionIssueKind::Ambiguous => ambiguous += 1,
        }
    }
    if unresolved + ambiguous > 0 {
        // The total alone hides whether names were missing or duplicated, so
        // each cause states its own count — and only when it happened.
        let mut row = Row::new(
            Some(Word::Warning),
            format!(
                "{} could not be followed",
                Counted::new(unresolved + ambiguous, "import", "imports")
            ),
        );
        if unresolved > 0 {
            row = row.with_fact(format!("{unresolved} named nothing in the repository"));
        }
        if ambiguous > 0 {
            row = row.with_fact(format!("{ambiguous} matched more than one file"));
        }
        warnings.push(row);
    }
    let relevant = |diagnostic: &Diagnostic| {
        diagnostic
            .file()
            .is_none_or(|file| file_belongs_to_scope(report, file, selected))
    };
    for kind in [
        DiagnosticKind::NestedRepository,
        DiagnosticKind::UnsupportedLanguage,
        DiagnosticKind::UnreadableFile,
        DiagnosticKind::OversizedFile,
        DiagnosticKind::ParseFailure,
        DiagnosticKind::AmbiguousIdentity,
        DiagnosticKind::UnsafeReference,
        DiagnosticKind::Other,
    ] {
        let count = report
            .diagnostics()
            .iter()
            .filter(|diagnostic| relevant(diagnostic) && diagnostic.kind() == kind)
            .filter(|diagnostic| !diagnostic.message().starts_with("Git history"))
            .count();
        if count > 0 {
            warnings.push(warning(diagnostic_summary(kind, count)));
        }
    }
    section.rows = warnings;
    if file_detail {
        // An import that could not be followed is file detail; at every other
        // scope the grouped sentence above is its whole terminal presence.
        section.rows.extend(unmatched_import_rows(report, selected));
    }
    let mut warning_detail = Vec::new();
    if file_detail {
        for diagnostic in report
            .diagnostics()
            .iter()
            .filter(|diagnostic| relevant(diagnostic))
            .filter(|diagnostic| !diagnostic.message().starts_with("Git history"))
        {
            if let Some(file) = diagnostic.file() {
                let path = report.files()[file.index()].path();
                warning_detail.push(format!("{path}: {}", diagnostic_detail(path, diagnostic)));
            }
        }
    }
    (section, warning_detail)
}

/// What one file diagnostic says, with the file's own path removed when the
/// message repeats it.
///
/// Some diagnostics name their file and some do not, and the row states the
/// path itself, so a message that opens with its path would otherwise print it
/// twice.
fn diagnostic_detail<'a>(path: &str, diagnostic: &'a Diagnostic) -> &'a str {
    let message = diagnostic.message();
    let remainder = message
        .strip_prefix(path)
        .map(|rest| rest.trim_start_matches([':', ' ']))
        .unwrap_or(message);
    if remainder.is_empty() {
        message
    } else {
        remainder
    }
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
        // Codebase debt is one ranked section of named problems; a diff keeps
        // its three sections this round.
        let sections: &[&Section] = match view.mode {
            ReportMode::Codebase => &[&view.areas, &view.problems],
            ReportMode::Diff => &[
                &view.areas,
                &view.findings,
                &view.architecture,
                &view.history,
            ],
        };
        for section in sections {
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
        if let Some(qualifier) = view.verdict.qualifier() {
            write!(self.writer, "  ")?;
            self.write_text(&format!("{} {}", qualifier.sentence(), qualifier.fact()), 2)?;
        }
        // A sub-scope answers about itself; the analysis-owned share states
        // what fraction of the whole that is, verbatim.
        if let Some(share) = view.verdict.share() {
            write!(self.writer, "  ")?;
            self.write_text(&share.sentence(), 2)?;
        }
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

/// What one diagnostic kind says about its subject, in singular and then
/// plural form.
///
/// A count moves the verb and the object together — one file uses *an
/// unsupported language* and three files use *unsupported languages* — so the
/// whole predicate is chosen at once rather than assembled from parts.
const fn diagnostic_predicate(kind: DiagnosticKind) -> (&'static str, &'static str) {
    match kind {
        DiagnosticKind::NestedRepository => ("was not analyzed.", "were not analyzed."),
        DiagnosticKind::UnsupportedLanguage => (
            "uses an unsupported language.",
            "use unsupported languages.",
        ),
        DiagnosticKind::UnreadableFile => ("could not be read.", "could not be read."),
        DiagnosticKind::OversizedFile => ("is too large to inspect.", "are too large to inspect."),
        DiagnosticKind::ParseFailure => {
            ("could not be fully parsed.", "could not be fully parsed.")
        }
        DiagnosticKind::AmbiguousIdentity => (
            "contains code that could not be matched.",
            "contain code that could not be matched.",
        ),
        DiagnosticKind::UnsafeReference => (
            "contains an unsafe reference.",
            "contain unsafe references.",
        ),
        DiagnosticKind::Other => ("could not be analyzed.", "could not be analyzed."),
    }
}

/// One sentence per diagnostic kind, each with its own subject.
///
/// Every sentence agrees with its own count, because a grouped sentence states
/// its kind at every scope and one file is the common case.
fn diagnostic_summary(kind: DiagnosticKind, count: usize) -> String {
    // A pruned checkout is not a source file, so it names its own subject.
    let subject = if kind == DiagnosticKind::NestedRepository {
        Counted::new(count, "nested repository", "nested repositories")
    } else {
        Counted::new(count, "source file", "source files")
    };
    let (singular, plural) = diagnostic_predicate(kind);
    format!("{subject} {}", if count == 1 { singular } else { plural })
}

fn file_belongs_to_scope(report: &Report, file: FileId, selected: &Scope) -> bool {
    let Some(record) = report.files().get(file.index()) else {
        return false;
    };
    scope_within(report, record.scope(), selected.id())
}

/// Whether `scope` is `ancestor` or lies inside it.
fn scope_within(report: &Report, scope: ScopeId, ancestor: ScopeId) -> bool {
    let mut current = Some(scope);
    while let Some(id) = current {
        if id == ancestor {
            return true;
        }
        current = report.scopes().get(id.index()).and_then(Scope::parent);
    }
    false
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

pub(super) struct Counted {
    count: usize,
    singular: &'static str,
    plural: &'static str,
}

impl Counted {
    pub(super) const fn new(count: usize, singular: &'static str, plural: &'static str) -> Self {
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
        DiagnosticKind::NestedRepository => "nested_repository",
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
        ArchitectureFindingId, ArchitectureFindingKind, ArchitectureGraph, ArchitectureReportFacts,
        ChangeCoupling, ComparisonId, ContributorConcentration, Coverage, DependencyCoverage,
        DependencyEdge, DependencyEdgeId, EvolutionaryFinding, EvolutionaryFindingId,
        EvolutionaryReportFacts, FileActivity, FileId, FileRecord, FindingId, HealthCounts,
        HealthPolicy, HistoryCoverage, Hotspot, KnowledgeConcentrationFinding,
        KnowledgeConcentrationFindingId, Measurements, PackageEdge, PackageEdgeId,
        PackageGraphMeasurement, PackageId, PackageRecord, ParseStatus, Report, ReportBuilder,
        ReportMode, Scope, ScopeId, SizePolicy, SourceCoverageOutcome, SourceRole, SourceSpan,
        SourceTrust, StableDependencyEvidence, StableDependencyFinding, StableDependencyFindingId,
        UnitIdentity, UnitKind,
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

    /// A report holding one card of every frozen pattern.
    ///
    /// Three packages keep the file patterns package-relative: `app` carries
    /// the concentrated and the hot file, `core` carries the widely imported
    /// one beside the nine files that import it, and `lib` carries a file
    /// measured by one ordinary finding.
    #[expect(
        clippy::too_many_lines,
        reason = "one fixture that exercises every pattern reads better whole"
    )]
    fn every_pattern_report() -> Report {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        let (app, core, lib) = (
            ScopeId::from_index(1),
            ScopeId::from_index(2),
            ScopeId::from_index(3),
        );
        let mut root_scope = Scope::new(root, ScopeKind::Repository, ".", None);
        for child in [app, core, lib] {
            root_scope.add_child(child);
        }
        builder.add_scope(root_scope);
        for (id, name) in [(app, "app"), (core, "core"), (lib, "lib")] {
            builder.add_scope(Scope::new(id, ScopeKind::Package, name, Some(root)));
        }
        builder.set_root(root);
        builder.set_packages(vec![
            PackageRecord::current(PackageId::from_index(0), app, "app"),
            PackageRecord::current(PackageId::from_index(1), core, "core"),
            PackageRecord::current(PackageId::from_index(2), lib, "lib"),
        ]);
        let policy = HealthPolicy::default();
        let high = Measurements::new(30, 4, 6);
        let watch = Measurements::new(15, 1, 1);
        let mut file = |index: usize, scope: ScopeId, package: usize, path: &str, counts| {
            let record = FileRecord::new(
                FileId::from_index(index),
                scope,
                path,
                Coverage::new(1, 1, 0, 0, 10, 0),
                counts,
            )
            .with_package(PackageId::from_index(package));
            builder.add_file(record);
            builder.link_file(scope, FileId::from_index(index));
            FileId::from_index(index)
        };
        // 0 concentrates three High findings and is long, 1 is hot with one,
        // 2 is imported by nine files, 3 holds one ordinary finding, and 4 and
        // 5 close a cycle.
        let god = file(0, app, 0, "app/god.js", HealthCounts::new(0, 0, 3));
        let hot = file(1, app, 0, "app/hot.js", HealthCounts::new(0, 0, 1));
        let hub = file(2, core, 1, "core/hub.js", HealthCounts::default());
        let plain = file(3, lib, 2, "lib/plain.js", HealthCounts::new(0, 1, 0));
        let left = file(4, app, 0, "app/left.js", HealthCounts::default());
        let right = file(5, app, 0, "app/right.js", HealthCounts::default());
        for index in 0..9 {
            file(
                6 + index,
                core,
                1,
                &format!("core/user-{index}.js"),
                HealthCounts::default(),
            );
        }
        // A file measured only by its length, which heads its card on its
        // size finding because it claims no source finding at all.
        let long = file(15, lib, 2, "lib/long.js", HealthCounts::default());
        let mut finding = |index: usize, target: FileId, name: &str, line: u32, measurements| {
            let id = FindingId::from_index(index);
            builder.add_finding(Finding::new(
                id,
                target,
                UnitIdentity::new(name, UnitKind::Function),
                SourceSpan::new(line, line + 2),
                measurements,
                policy.assess(measurements),
            ));
            builder.link_finding(root, id);
        };
        for (index, line) in [(0usize, 4u32), (1, 20), (2, 40)] {
            finding(index, god, &format!("build-{index}"), line, high);
        }
        finding(3, hot, "render", 7, high);
        finding(4, plain, "value", 2, watch);
        // A hot file states its heat, and only the hotspot table decides it.
        builder.set_hotspots(vec![Hotspot::new(hot, Rating::High, 14)]);
        builder.set_size_findings(vec![
            SizePolicy::default()
                .rate_file(god, 520)
                .expect("a long file is rated"),
            SizePolicy::default()
                .rate_file(long, 640)
                .expect("a long file is rated"),
        ]);
        let mut edges = vec![
            DependencyEdge::new(
                DependencyEdgeId::from_index(0),
                left,
                right,
                1,
                vec![SourceSpan::new(1, 1)],
            ),
            DependencyEdge::new(
                DependencyEdgeId::from_index(1),
                right,
                left,
                1,
                vec![SourceSpan::new(1, 1)],
            ),
        ];
        edges.extend((0..9).map(|index| {
            DependencyEdge::new(
                DependencyEdgeId::from_index(2 + index),
                FileId::from_index(6 + index),
                hub,
                1,
                vec![SourceSpan::new(1, 1)],
            )
        }));
        builder.set_architecture(ArchitectureReportFacts::new(
            ArchitectureGraph::new(
                DependencyCoverage::new(15, 0, 0, 0, 0, 0, 0),
                edges,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
            vec![ArchitectureFinding::new(
                ArchitectureFindingId::from_index(0),
                ArchitectureFindingKind::FileCycle,
                Vec::new(),
                vec![left, right],
                vec![
                    DependencyEdgeId::from_index(0),
                    DependencyEdgeId::from_index(1),
                ],
            )],
            Vec::new(),
        ));
        builder.link_architecture_finding(root, ArchitectureFindingId::from_index(0));
        builder.set_stable_dependency_findings(vec![StableDependencyFinding::new(
            StableDependencyFindingId::from_index(0),
            PackageId::from_index(0),
            PackageId::from_index(1),
            StableDependencyEvidence::new(
                PackageGraphMeasurement::new(PackageId::from_index(0), 1, 1),
                PackageGraphMeasurement::new(PackageId::from_index(1), 1, 2),
                2,
            ),
            vec![DependencyEdgeId::from_index(0)],
        )]);
        let coupling =
            ChangeCoupling::new(PackageId::from_index(0), PackageId::from_index(1), 4, 5);
        builder.set_evolution(
            EvolutionaryReportFacts::new(
                HistoryCoverage::unavailable("test"),
                Vec::new(),
                Vec::new(),
                vec![coupling],
                vec![ContributorConcentration::new(
                    PackageId::from_index(2),
                    1,
                    9,
                    10,
                )],
                vec![EvolutionaryFinding::new(
                    EvolutionaryFindingId::from_index(0),
                    coupling,
                )],
                Vec::new(),
            )
            .with_concentration_findings(vec![KnowledgeConcentrationFinding::new(
                KnowledgeConcentrationFindingId::from_index(0),
                ContributorConcentration::new(PackageId::from_index(2), 1, 9, 10),
            )]),
        );
        builder.link_evolutionary_finding(root, EvolutionaryFindingId::from_index(0));
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

    /// A report whose files each own a file scope under one directory, so a
    /// selected file is a scope and a card's anchor can be that scope.
    fn file_scope_report() -> Report {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        let directory = ScopeId::from_index(1);
        let mut root_scope = Scope::new(root, ScopeKind::Repository, ".", None);
        root_scope.add_child(directory);
        builder.add_scope(root_scope);
        let mut directory_scope = Scope::new(directory, ScopeKind::Directory, "src", Some(root));
        let paths = ["src/advisory.js", "src/plain.js"];
        for index in 0..paths.len() {
            directory_scope.add_child(ScopeId::from_index(2 + index));
        }
        builder.add_scope(directory_scope);
        for (index, path) in paths.into_iter().enumerate() {
            builder.add_scope(Scope::new(
                ScopeId::from_index(2 + index),
                ScopeKind::File,
                path,
                Some(directory),
            ));
        }
        // The first file's whole debt is advisory, so its card is `detail`.
        // The second holds more findings than the widest rung shows, so
        // selecting it proves the ladder is not applied to a file.
        add_scoped_file(&mut builder, 0, paths[0], true, 0..1);
        add_scoped_file(&mut builder, 1, paths[1], false, 1..6);
        builder.set_root(root);
        builder.finish()
    }

    /// Adds one file of [`file_scope_report`] with the findings `ids` names.
    fn add_scoped_file(
        builder: &mut ReportBuilder,
        index: usize,
        path: &str,
        advisory: bool,
        ids: std::ops::Range<usize>,
    ) {
        let scope = ScopeId::from_index(2 + index);
        let file = FileId::from_index(index);
        // Advisory debt cannot move a verdict, so it counts no rated unit.
        let counts = if advisory {
            HealthCounts::default()
        } else {
            HealthCounts::new(0, 0, ids.len() as u32)
        };
        let mut record =
            FileRecord::new(file, scope, path, Coverage::new(1, 1, 0, 0, 10, 0), counts);
        if advisory {
            record = record.with_source_state(SourceRole::Benchmark, ParseStatus::Recovered);
        }
        builder.add_file(record);
        builder.link_file(scope, file);
        let measurements = Measurements::new(30, 4, 6);
        for (unit, id) in ids.enumerate() {
            let line = 3 + unit as u32 * 10;
            let mut finding = Finding::new(
                FindingId::from_index(id),
                file,
                UnitIdentity::new(format!("work-{unit}"), UnitKind::Function),
                SourceSpan::new(line, line + 2),
                measurements,
                HealthPolicy::default().assess(measurements),
            );
            if advisory {
                finding = finding.with_evidence(SourceRole::Benchmark, SourceTrust::Advisory);
            }
            builder.add_finding(finding);
            builder.link_finding(ScopeId::from_index(0), FindingId::from_index(id));
        }
    }

    /// A report whose every file carries one ordinary finding, so it holds
    /// exactly `files` `measured` cards.
    fn report_with_measured_cards(files: usize) -> Report {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        builder.add_scope(Scope::new(root, ScopeKind::Repository, ".", None));
        builder.set_root(root);
        let policy = HealthPolicy::default();
        let measurements = Measurements::new(15, 1, 1);
        for index in 0..files {
            let file = FileId::from_index(index);
            builder.add_file(FileRecord::new(
                file,
                root,
                format!("file-{index:03}.rs"),
                Coverage::new(1, 1, 0, 0, 10, 0),
                HealthCounts::new(0, 1, 0),
            ));
            builder.add_finding(Finding::new(
                FindingId::from_index(index),
                file,
                UnitIdentity::new(format!("unit-{index:03}"), UnitKind::Function),
                SourceSpan::new(1, 2),
                measurements,
                policy.assess(measurements),
            ));
            builder.link_finding(root, FindingId::from_index(index));
        }
        builder.finish()
    }

    /// The rows of one section, read back from rendered text.
    fn section_rows(text: &str, heading: &str) -> Vec<String> {
        text.lines()
            .skip_while(|line| *line != heading)
            .skip(1)
            .take_while(|line| !line.is_empty())
            .map(|line| line.to_owned())
            .collect()
    }

    /// The card heads of a rendered problem section, which are the rows that
    /// are not indented continuations.
    fn problem_heads(text: &str) -> Vec<String> {
        section_rows(text, "PROBLEMS")
            .into_iter()
            .filter(|line| !line.starts_with("        "))
            .collect()
    }

    #[test]
    fn every_ladder_rung_spends_the_whole_budget() {
        for cards in PROBLEM_LADDER {
            assert_eq!(cards * (1 + evidence_allowance(cards)), SCREEN_BUDGET);
        }
        // The rungs trade depth for breadth in one direction only.
        for pair in PROBLEM_LADDER.windows(2) {
            assert!(pair[0] < pair[1], "{pair:?}");
            assert!(
                evidence_allowance(pair[0]) > evidence_allowance(pair[1]),
                "{pair:?}"
            );
        }
        assert_eq!(PROBLEM_LADDER, [6, 8, 12, 24]);
        assert_eq!(LAST_RUNG, 24);
    }

    #[test]
    fn the_ladder_rung_is_the_first_whose_card_count_fits() {
        for (cards, expected) in [
            (0, (6, 3)),
            (5, (6, 3)),
            (6, (6, 3)),
            (7, (8, 2)),
            (8, (8, 2)),
            (9, (12, 1)),
            (12, (12, 1)),
            (13, (24, 0)),
            (20, (24, 0)),
            (24, (24, 0)),
            (40, (24, 0)),
        ] {
            assert_eq!(ladder_rung(cards), expected, "{cards}");
        }
    }

    #[test]
    fn a_default_view_shows_the_rung_its_card_count_selects() {
        // Five cards buy depth; the twenty-fifth card and beyond are cut.
        let five = render(&report_with_measured_cards(5), TerminalOptions::default());
        assert_eq!(problem_heads(&five).len(), 5, "{five}");
        assert_eq!(section_rows(&five, "PROBLEMS").len(), 10, "{five}");

        let twenty = render(&report_with_measured_cards(20), TerminalOptions::default());
        assert_eq!(problem_heads(&twenty).len(), 20, "{twenty}");
        // The last rung allows no evidence, so every row is a card head.
        assert_eq!(section_rows(&twenty, "PROBLEMS").len(), 20, "{twenty}");

        let forty = render(&report_with_measured_cards(40), TerminalOptions::default());
        let heads = problem_heads(&forty);
        assert_eq!(heads.len(), SCREEN_BUDGET, "{forty}");
        assert_eq!(
            section_rows(&forty, "PROBLEMS").len(),
            SCREEN_BUDGET,
            "{forty}"
        );
        // The cut takes the lowest ranked cards and writes no bookkeeping row.
        assert!(heads[0].contains("unit-000"), "{forty}");
        assert!(!forty.contains("unit-024"), "{forty}");
        assert!(!forty.contains("omitted"), "{forty}");
    }

    #[test]
    fn a_top_limit_counts_cards_and_buys_breadth_with_evidence() {
        let report = report_with_measured_cards(12);
        let top = |count| {
            render(
                &report,
                TerminalOptions::default().with_top(NonZeroUsize::new(count)),
            )
        };
        // Ten cards select the rung ten selects, which allows one line each.
        let ten = top(10);
        assert_eq!(problem_heads(&ten).len(), 10, "{ten}");
        assert_eq!(section_rows(&ten, "PROBLEMS").len(), 20, "{ten}");
        // A limit below the first rung buys depth back.
        let four = top(4);
        assert_eq!(problem_heads(&four).len(), 4, "{four}");
        assert_eq!(section_rows(&four, "PROBLEMS").len(), 8, "{four}");
        // A limit above the cards shows them all and adds no filler row.
        let roomy = top(30);
        assert_eq!(problem_heads(&roomy).len(), 12, "{roomy}");
        assert_eq!(section_rows(&roomy, "PROBLEMS").len(), 12, "{roomy}");
    }

    #[test]
    fn each_pattern_states_its_rating_its_name_and_its_anchor() {
        let report = every_pattern_report();
        let detailed = render(&report, TerminalOptions::new(120, true, false));
        for head in [
            "  high does too much · app/god.js",
            "  high hot and complex · app/hot.js",
            "  watch circular dependency · app/left.js",
            "  watch changes together · app ↔ core",
            "  watch one author · lib",
            "  watch depends on less stable code · app → core",
            // A `measured` card heads on its top claimed finding and anchors
            // on the span that finding carries.
            "  watch value · function · lib/plain.js:2",
            // A card that claims nothing carries no rating word to state.
            "  everything depends on this · core/hub.js",
        ] {
            assert!(
                detailed.contains(&format!("{head}\n")),
                "{head}: {detailed}"
            );
        }
        // No renderer invents, renames, or composes a machine pattern id.
        for id in ["god_file", "hot_mess", "shotgun_pair", "bus_risk", "tangle"] {
            assert!(!detailed.contains(id), "{id}: {detailed}");
        }
    }

    #[test]
    fn each_evidence_kind_states_its_own_words() {
        let report = every_pattern_report();
        let detailed = render(&report, TerminalOptions::new(120, true, false));
        for fact in [
            "        3 rated units",
            "        1 rated unit",
            "        9 files import this",
            "        file · 520 lines",
            "        hot (14 commits)",
            "        2 files in the cycle",
            "        app/god.js:4 · function · cognitive 30",
            "        changed together in 4 of 5 commits · 80% · no code dependency",
            "        one contributor made 9 of 10 commits",
            "        instability 1/2 → 2/3 · 2 imports",
            // A cycle witness is stacked one step per line and never shortened.
            "        app/left.js\n        → app/right.js\n        → app/left.js\n",
        ] {
            assert!(detailed.contains(fact), "{fact}: {detailed}");
        }
        assert!(!detailed.contains('…'), "{detailed}");
    }

    #[test]
    fn singular_and_plural_evidence_wording_agree_with_their_counts() {
        let report = every_pattern_report();
        for (fact, expected) in [
            (ProblemEvidence::FanIn(1), "1 file imports this"),
            (ProblemEvidence::FanIn(2), "2 files import this"),
            (ProblemEvidence::FanOut(1), "imports 1 file"),
            (ProblemEvidence::FanOut(3), "imports 3 files"),
            (ProblemEvidence::RatedUnits(1), "1 rated unit"),
            (ProblemEvidence::RatedUnits(4), "4 rated units"),
            (ProblemEvidence::Members(1), "1 file in the cycle"),
            (ProblemEvidence::Members(5), "5 files in the cycle"),
            // Clustering carries a touch count only for a hotspot, so heat is
            // the only activity wording a card states.
            (ProblemEvidence::Hot(1), "hot (1 commit)"),
            (ProblemEvidence::Hot(14), "hot (14 commits)"),
        ] {
            assert_eq!(
                evidence_lines(&report, fact, false),
                vec![expected.to_owned()],
                "{fact:?}"
            );
        }
    }

    #[test]
    fn a_card_headed_on_a_size_finding_names_the_file_and_states_its_value() {
        let report = every_pattern_report();
        let detailed = render(&report, TerminalOptions::new(120, true, false));
        // The head takes the identity-then-kind form a finding head takes,
        // and a file's own length is measured on the anchor itself, so the
        // path is written once.
        assert!(
            detailed.contains("  watch lib/long.js · file\n        640 lines\n"),
            "{detailed}"
        );
        // The same finding on a card that does not head on it keeps its
        // subject, because no head states it there.
        assert!(
            detailed.contains("        file · 520 lines\n"),
            "{detailed}"
        );
        // A container names itself beside the file it sits in.
        let container = SizePolicy::default()
            .rate_container(FileId::from_index(0), "Editor", 900)
            .expect("a long container is rated");
        assert_eq!(size_subject(&container), "Editor · container");
    }

    #[test]
    fn every_diagnostic_sentence_agrees_with_its_own_count() {
        for (kind, singular, plural) in [
            (
                DiagnosticKind::NestedRepository,
                "1 nested repository was not analyzed.",
                "3 nested repositories were not analyzed.",
            ),
            (
                DiagnosticKind::UnsupportedLanguage,
                "1 source file uses an unsupported language.",
                "3 source files use unsupported languages.",
            ),
            (
                DiagnosticKind::UnreadableFile,
                "1 source file could not be read.",
                "3 source files could not be read.",
            ),
            (
                DiagnosticKind::OversizedFile,
                "1 source file is too large to inspect.",
                "3 source files are too large to inspect.",
            ),
            (
                DiagnosticKind::ParseFailure,
                "1 source file could not be fully parsed.",
                "3 source files could not be fully parsed.",
            ),
            (
                DiagnosticKind::AmbiguousIdentity,
                "1 source file contains code that could not be matched.",
                "3 source files contain code that could not be matched.",
            ),
            (
                DiagnosticKind::UnsafeReference,
                "1 source file contains an unsafe reference.",
                "3 source files contain unsafe references.",
            ),
            (
                DiagnosticKind::Other,
                "1 source file could not be analyzed.",
                "3 source files could not be analyzed.",
            ),
        ] {
            assert_eq!(diagnostic_summary(kind, 1), singular, "{kind:?}");
            assert_eq!(diagnostic_summary(kind, 3), plural, "{kind:?}");
        }
    }

    /// A report holding one long cycle beside five ordinary cards, so the
    /// widest rung applies and one card's witness is longer than that rung.
    fn report_with_a_long_cycle(members: usize) -> Report {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        builder.add_scope(Scope::new(root, ScopeKind::Repository, ".", None));
        builder.set_root(root);
        let policy = HealthPolicy::default();
        let measurements = Measurements::new(15, 1, 1);
        // Five files carrying one ordinary finding each, plus the cycle.
        for index in 0..5 {
            let file = FileId::from_index(index);
            builder.add_file(FileRecord::new(
                file,
                root,
                format!("measured-{index}.rs"),
                Coverage::new(1, 1, 0, 0, 10, 0),
                HealthCounts::new(0, 1, 0),
            ));
            builder.add_finding(Finding::new(
                FindingId::from_index(index),
                file,
                UnitIdentity::new(format!("unit-{index}"), UnitKind::Function),
                SourceSpan::new(1, 2),
                measurements,
                policy.assess(measurements),
            ));
            builder.link_finding(root, FindingId::from_index(index));
        }
        let mut edges = Vec::new();
        for index in 0..members {
            builder.add_file(FileRecord::new(
                FileId::from_index(5 + index),
                root,
                format!("cycle-{index:02}.rs"),
                Coverage::new(1, 1, 0, 0, 1, 0),
                HealthCounts::default(),
            ));
            edges.push(DependencyEdge::new(
                DependencyEdgeId::from_index(index),
                FileId::from_index(5 + index),
                FileId::from_index(5 + (index + 1) % members),
                1,
                vec![SourceSpan::new(1, 1)],
            ));
        }
        builder.set_architecture(ArchitectureReportFacts::new(
            ArchitectureGraph::new(
                DependencyCoverage::new(members as u32, 0, 0, 0, 0, 0, 0),
                edges,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
            vec![ArchitectureFinding::new(
                ArchitectureFindingId::from_index(0),
                ArchitectureFindingKind::FileCycle,
                Vec::new(),
                (0..members)
                    .map(|index| FileId::from_index(5 + index))
                    .collect(),
                (0..members).map(DependencyEdgeId::from_index).collect(),
            )],
            Vec::new(),
        ));
        builder.link_architecture_finding(root, ArchitectureFindingId::from_index(0));
        builder.finish()
    }

    /// A cycle witness is one fact stated one step per line, so eliding it
    /// would destroy the fact rather than shorten it: the accounting counts
    /// the evidence item and never its steps.
    #[test]
    fn a_witness_costs_one_slot_however_many_steps_it_stacks() {
        let members = 12;
        let report = report_with_a_long_cycle(members);
        let terminal = render(&report, TerminalOptions::new(120, false, false));
        let rows = section_rows(&terminal, "PROBLEMS");
        // Six cards select the widest-depth rung, which allows three lines.
        let heads = problem_heads(&terminal);
        assert_eq!(heads.len(), 6, "{terminal}");
        assert_eq!(ladder_rung(heads.len()), (6, 3));

        // The witness renders every step and closes the cycle.
        let witness: Vec<&String> = rows
            .iter()
            .filter(|line| {
                line.trim_start()
                    .trim_start_matches("→ ")
                    .starts_with("cycle-")
            })
            .collect();
        assert_eq!(witness.len(), members + 1, "{terminal}");
        assert_eq!(
            witness[0].trim(),
            witness[members].trim().trim_start_matches("→ "),
            "{terminal}"
        );
        assert!(!terminal.contains('…'), "{terminal}");

        // Every card still respects the rung once the witness steps are set
        // aside, which is exactly what the budget accounts for: six heads,
        // the cycle's two facts, and one fact for each ordinary card.
        let card = report
            .problems()
            .iter()
            .find(|card| card.pattern() == ProblemPattern::Tangle)
            .expect("the cycle has a card");
        assert_eq!(card.evidence().len(), 2, "{card:?}");
        let extra = witness.len() - 1;
        let slots = rows.len() - extra;
        assert_eq!(slots, 6 + 2 + 5, "{terminal}");
        assert!(slots <= SCREEN_BUDGET, "{terminal}");
        // The rendered view may run past the budget; the accounting does not.
        assert!(rows.len() > SCREEN_BUDGET, "{terminal}");
    }

    #[test]
    fn one_file_carrying_three_high_findings_is_named_once() {
        let report = every_pattern_report();
        let detailed = render(&report, TerminalOptions::new(120, true, false));
        assert_eq!(
            problem_heads(&detailed)
                .iter()
                .filter(|head| head.contains("app/god.js"))
                .count(),
            1,
            "{detailed}"
        );
        // The card still claims all three, which `--all` reaches.
        for line in ["app/god.js:4", "app/god.js:20", "app/god.js:40"] {
            assert!(detailed.contains(line), "{line}: {detailed}");
        }
    }

    #[test]
    fn a_detail_card_reaches_all_and_its_own_scope_only() {
        let report = file_scope_report();
        let scopes = report.scopes();
        let render_scope = |scope: ScopeId, options| {
            let mut bytes = Vec::new();
            write_terminal(&mut bytes, &report, Some(scope), options).unwrap();
            String::from_utf8(bytes).unwrap()
        };
        let problems = |text: &str| section_rows(text, "PROBLEMS").join("\n");
        let default = problems(&render_scope(
            ScopeId::from_index(0),
            TerminalOptions::default(),
        ));
        assert!(default.contains("src/plain.js"), "{default}");
        assert!(!default.contains("src/advisory.js"), "{default}");
        // `--all` shows it, and so does selecting its own file.
        let all = problems(&render_scope(
            ScopeId::from_index(0),
            TerminalOptions::new(100, true, false),
        ));
        assert!(all.contains("src/advisory.js"), "{all}");
        let own = problems(&render_scope(
            ScopeId::from_index(2),
            TerminalOptions::default(),
        ));
        assert!(own.contains("src/advisory.js"), "{own}");
        // Its neighbour's card belongs to the other file scope alone.
        assert!(!own.contains("src/plain.js"), "{own}");
        assert_eq!(scopes[2].kind(), ScopeKind::File);
    }

    #[test]
    fn a_selected_file_shows_complete_evidence_without_the_ladder() {
        let report = file_scope_report();
        let render_scope = |scope: usize| {
            let mut bytes = Vec::new();
            write_terminal(
                &mut bytes,
                &report,
                Some(ScopeId::from_index(scope)),
                TerminalOptions::default(),
            )
            .unwrap();
            String::from_utf8(bytes).unwrap()
        };
        // Above the file the ladder allows three of the card's five facts.
        let repository = render_scope(0);
        assert_eq!(
            section_rows(&repository, "PROBLEMS").len(),
            4,
            "{repository}"
        );
        assert!(!repository.contains("src/plain.js:33"), "{repository}");
        // The file itself is a request for detail, so it states all five.
        let file = render_scope(3);
        assert_eq!(section_rows(&file, "PROBLEMS").len(), 6, "{file}");
        // A caller-chosen limit still names how many cards a file shows.
        let mut bytes = Vec::new();
        write_terminal(
            &mut bytes,
            &report,
            Some(ScopeId::from_index(3)),
            TerminalOptions::default().with_top(NonZeroUsize::new(1)),
        )
        .unwrap();
        let capped = String::from_utf8(bytes).unwrap();
        assert_eq!(problem_heads(&capped).len(), 1, "{capped}");
        assert_eq!(section_rows(&capped, "PROBLEMS").len(), 6, "{capped}");
        for line in [
            // The head already names the first finding, so its line adds the
            // measurements alone.
            "  high work-0 · function · src/plain.js:3\n        cognitive 30\n",
            "        src/plain.js:43 · function · cognitive 30\n",
        ] {
            assert!(file.contains(line), "{line}: {file}");
        }
    }

    #[test]
    fn one_invocation_states_the_same_cards_at_every_width() {
        let report = every_pattern_report();
        let facts = |width| {
            let text = render(&report, TerminalOptions::new(width, false, false));
            let body: String = section_rows(&text, "PROBLEMS").join("\n");
            // Stacking is the only difference a width may make, so the facts
            // are compared with their line breaks and indentation removed.
            body.split_whitespace().collect::<Vec<_>>().join(" ")
        };
        let wide = facts(120);
        assert_eq!(facts(80), wide);
        assert_eq!(facts(50), wide);
        // A narrow view may still render more lines than the wide one.
        assert!(
            section_rows(
                &render(&report, TerminalOptions::new(50, false, false)),
                "PROBLEMS"
            )
            .len()
                >= section_rows(
                    &render(&report, TerminalOptions::new(120, false, false)),
                    "PROBLEMS"
                )
                .len()
        );
    }

    #[test]
    fn a_codebase_report_writes_one_problem_section_and_a_diff_keeps_its_three() {
        let codebase = render(&every_pattern_report(), TerminalOptions::default());
        assert!(codebase.contains("\nPROBLEMS\n"), "{codebase}");
        for heading in ["\nFINDINGS\n", "\nARCHITECTURE\n", "\nHISTORY\n"] {
            assert!(!codebase.contains(heading), "{heading}: {codebase}");
        }
        // A diff keeps its own sections and never states a problem card.
        let diff = render(
            &diff_report(Some(ArchitectureComparisonKind::CycleIntroduced)),
            TerminalOptions::new(100, true, false),
        );
        assert!(diff.contains("\nARCHITECTURE\n"), "{diff}");
        assert!(!diff.contains("\nPROBLEMS\n"), "{diff}");
        assert!(!diff.contains("circular dependency"), "{diff}");
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
        assert!(!terminal.contains("PROBLEMS"));
        assert!(!terminal.contains("FINDINGS"));
        assert!(!terminal.contains("ARCHITECTURE"));
        assert!(!terminal.contains("HISTORY"));

        let mut json = Vec::new();
        write_json(&mut json, &report, report.root()).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
        assert_eq!(value["schema_version"], 4);
        assert_eq!(value["mode"], "codebase");
    }

    /// Both qualifications a verdict can carry are analysis-owned bytes the
    /// renderer only places, and the accepted placement puts the share
    /// directly under the qualifier row when one exists. No generated fixture
    /// is both mostly unsupported and drilled into, so the stacked order is
    /// proven here over a report built with both.
    #[test]
    fn a_qualified_sub_scope_stacks_the_share_under_the_qualifier_row() {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let (root, package, outside) = (
            ScopeId::from_index(0),
            ScopeId::from_index(1),
            ScopeId::from_index(2),
        );
        let mut root_scope = Scope::new(root, ScopeKind::Repository, ".", None);
        root_scope.add_child(package);
        root_scope.add_child(outside);
        builder.add_scope(root_scope);
        builder.add_scope(Scope::new(package, ScopeKind::Package, "app", Some(root)));
        builder.add_scope(Scope::new(
            outside,
            ScopeKind::File,
            "core/other.rs",
            Some(root),
        ));
        builder.set_root(root);
        builder.set_packages(vec![PackageRecord::current(
            PackageId::from_index(0),
            package,
            "app",
        )]);
        // The package holds one of the repository's two High units, and two
        // thirds of its bytes are in a language no grammar reads.
        let mut file = |index: usize, scope: ScopeId, path: &str, coverage, counts| {
            let id = FileId::from_index(index);
            builder.add_file(
                FileRecord::new(id, scope, path, coverage, counts)
                    .with_package(PackageId::from_index(0)),
            );
            builder.link_file(scope, id);
        };
        file(
            0,
            package,
            "app/work.rs",
            Coverage::new(1, 1, 0, 0, 10, 0).with_bytes(100, 0),
            HealthCounts::new(0, 0, 1),
        );
        file(
            1,
            package,
            "app/tool.go",
            Coverage::classified(1, SourceCoverageOutcome::Unsupported, 0, 0).with_bytes(200, 200),
            HealthCounts::default(),
        );
        file(
            2,
            outside,
            "core/other.rs",
            Coverage::new(1, 1, 0, 0, 10, 0).with_bytes(100, 0),
            HealthCounts::new(0, 0, 1),
        );
        let report = builder.finish();
        let mut bytes = Vec::new();
        write_terminal(
            &mut bytes,
            &report,
            Some(package),
            TerminalOptions::default(),
        )
        .unwrap();
        let terminal = String::from_utf8(bytes).unwrap();
        // Scope, tier sentence, qualifier, share, counts — in that order and
        // with no line between them.
        assert!(
            terminal.starts_with(concat!(
                "smackdebt · app\n",
                "  Worn in the usual places.\n",
                "  Not all source was checked. 66% of source bytes are Go.\n",
                "  1 of the repository's 2 high live here.\n",
                "1 high · 0 watch · 1 checked\n",
            )),
            "{terminal}"
        );
        // The renderer placed two analysis-owned facts and composed neither.
        let verdict = report.scope_verdict(package);
        let qualifier = verdict.qualifier().expect("a qualified sub-scope");
        let share = verdict.share().expect("a framed sub-scope");
        assert!(terminal.contains(&share.sentence()), "{terminal}");
        assert!(terminal.contains(qualifier.sentence()), "{terminal}");
        // The root carries the qualifier its own bytes earn and no share.
        let mut root_bytes = Vec::new();
        write_terminal(
            &mut root_bytes,
            &report,
            Some(root),
            TerminalOptions::default(),
        )
        .unwrap();
        let root_terminal = String::from_utf8(root_bytes).unwrap();
        assert!(!root_terminal.contains("live here."), "{root_terminal}");
        assert!(
            root_terminal.contains("Not all source was checked."),
            "{root_terminal}"
        );
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
    fn a_diff_states_no_edge_change_row_even_with_all_detail() {
        let report = diff_report(Some(ArchitectureComparisonKind::CycleIntroduced));
        let terminal = render(&report, TerminalOptions::new(100, true, false));
        // An added edge is a graph fact the machine report keeps; the diff
        // states the cycle it caused and nothing else.
        assert!(!terminal.contains("added"), "{terminal}");
        assert!(
            terminal.contains("  worse package dependency cycle introduced"),
            "{terminal}"
        );
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
    fn a_one_sided_comparison_states_its_direction_then_its_present_side() {
        let one_sided = |kind, before, after| {
            changed_measurements(&Comparison::new(
                ComparisonId::from_index(0),
                UnitIdentity::new("work", UnitKind::Function),
                kind,
                before,
                after,
                before.map(|_| Rating::Watch),
                after.map(|_| Rating::Watch),
            ))
        };
        // An added unit has an after side only, and it is stated absolutely.
        assert_eq!(
            one_sided(
                ComparisonKind::Added,
                None,
                Some(Measurements::new(6, 4, 2).with_shape(3, 1)),
            ),
            vec![
                "added".to_owned(),
                "cognitive 6".to_owned(),
                "cyclomatic 4".to_owned(),
                "statements 2".to_owned(),
                "nesting 3".to_owned(),
                "parameters 1".to_owned(),
            ]
        );
        // A removed unit has a before side only, and zeros are not facts.
        assert_eq!(
            one_sided(
                ComparisonKind::Removed,
                Some(Measurements::new(5, 3, 4).with_shape(0, 0)),
                None,
            ),
            vec![
                "removed".to_owned(),
                "cognitive 5".to_owned(),
                "cyclomatic 3".to_owned(),
                "statements 4".to_owned(),
            ]
        );
        // A side that measures zero everywhere leaves the word alone.
        assert_eq!(
            one_sided(ComparisonKind::Added, None, Some(Measurements::default())),
            vec!["added".to_owned()]
        );
        // No side can be trusted, so the sentence stands by itself.
        assert_eq!(
            one_sided(ComparisonKind::Ambiguous, None, None),
            vec!["identity could not be matched safely".to_owned()]
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
                .contains("  high broken · function · benchmark · advisory · broken.py:1")
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
    fn cycle_cards_are_ordered_by_rating_and_state_every_witness() {
        // Four cycle findings arrive in the order analysis collected them and
        // the only High one arrives last, so an order of arrival would bury
        // the worst finding in the scope. Each witness starts at its own file,
        // which is how a rendered card is identified below.
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        builder.add_scope(Scope::new(root, ScopeKind::Repository, ".", None));
        builder.set_root(root);
        let paths = ["a.js", "b.js", "c.js", "d.js"];
        let mut edges = Vec::new();
        let mut findings = Vec::new();
        for (index, path) in paths.into_iter().enumerate() {
            let file = FileId::from_index(index);
            let id = ArchitectureFindingId::from_index(index);
            let kind = if index + 1 == paths.len() {
                ArchitectureFindingKind::PackageCycle
            } else {
                ArchitectureFindingKind::FileCycle
            };
            builder.add_file(FileRecord::new(
                file,
                root,
                path,
                Coverage::new(1, 1, 0, 0, 1, 0),
                HealthCounts::default(),
            ));
            edges.push(DependencyEdge::new(
                DependencyEdgeId::from_index(index),
                file,
                FileId::from_index((index + 1) % paths.len()),
                1,
                vec![SourceSpan::new(1, 1)],
            ));
            findings.push(ArchitectureFinding::new(
                id,
                kind,
                Vec::new(),
                vec![file, FileId::from_index((index + 1) % paths.len())],
                vec![DependencyEdgeId::from_index(index)],
            ));
            builder.link_architecture_finding(root, id);
        }
        builder.set_architecture(ArchitectureReportFacts::new(
            ArchitectureGraph::new(
                DependencyCoverage::new(4, 0, 0, 0, 0, 0, 0),
                edges,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
            findings,
            Vec::new(),
        ));
        let terminal = render(&builder.finish(), TerminalOptions::new(120, false, false));
        let at = |needle: &str| terminal.find(needle).expect(&terminal);
        // The High card heads the section and the Watch cards keep the order
        // analysis gave them. Four cards buy three evidence lines each, so
        // every witness is stated and none is cut.
        assert!(at("high circular dependency · d.js") < at("watch circular dependency · a.js"));
        assert!(at("        d.js") < at("        a.js"), "{terminal}");
        assert!(at("        a.js") < at("        b.js"), "{terminal}");
        assert!(terminal.contains("        c.js"), "{terminal}");
        assert_eq!(problem_heads(&terminal).len(), 4, "{terminal}");
    }

    #[test]
    fn every_coupling_finding_reaches_a_card_ranked_among_the_other_problems() {
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
        // Every actionable pair is a card now: the section-local limit that
        // hid the fourth pair competed with nothing else on the screen.
        assert_eq!(problem_heads(&default).len(), 4, "{default}");
        assert!(
            default.contains(
                "  watch changes together · b ↔ c\n        changed together in 8 of 10 commits · 80% · no code dependency\n"
            ),
            "{default}"
        );
        assert!(
            default.contains(
                "  watch changes together · d ↔ e\n        changed together in 3 of 5 commits · 60% · no code dependency\n"
            ),
            "{default}"
        );
        // The cards tie on every rank key before the anchor, so the anchor
        // package path orders them.
        assert!(default.find("a ↔ b").unwrap() < default.find("b ↔ c").unwrap());
        assert!(default.find("b ↔ c").unwrap() < default.find("c ↔ d").unwrap());
        assert_eq!(
            problem_heads(&render(&report, TerminalOptions::new(100, true, false))).len(),
            4
        );
    }

    /// A pair a manifest test-role reference explains produces no package edge
    /// and no file edge, so the claim can only come from the explanation set
    /// analysis recorded.
    #[test]
    fn a_coupling_row_states_the_explanation_analysis_recorded() {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        builder.add_scope(Scope::new(root, ScopeKind::Repository, ".", None));
        builder.set_root(root);
        let packages = ["a", "b", "c"]
            .into_iter()
            .enumerate()
            .map(|(index, name)| {
                let scope = ScopeId::from_index(index + 1);
                builder.add_scope(Scope::new(scope, ScopeKind::Package, name, Some(root)));
                PackageRecord::current(PackageId::from_index(index), scope, name)
            })
            .collect::<Vec<_>>();
        builder.set_packages(packages);
        let couplings = [(0, 1), (1, 2)]
            .into_iter()
            .map(|(left, right)| {
                ChangeCoupling::new(
                    PackageId::from_index(left),
                    PackageId::from_index(right),
                    3,
                    4,
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
        // Only the second pair is explained, and in the opposite direction.
        builder.set_explanation_pairs(std::collections::BTreeSet::from([(
            PackageId::from_index(2),
            PackageId::from_index(1),
        )]));
        for index in 0..2 {
            builder.link_evolutionary_finding(root, EvolutionaryFindingId::from_index(index));
        }
        let report = builder.finish();
        assert!(report.package_edges().is_empty());
        assert!(report.dependency_edges().is_empty());

        let terminal = render(&report, TerminalOptions::default());
        assert!(
            terminal.contains(
                "  watch changes together · a ↔ b\n        changed together in 3 of 4 commits · 75% · no code dependency\n"
            ),
            "{terminal}"
        );
        assert!(
            terminal.contains(
                "  watch changes together · b ↔ c\n        changed together in 3 of 4 commits · 75% · code dependency exists\n"
            ),
            "{terminal}"
        );
    }

    /// A pair only a dependency path connects is stated as indirect — naming
    /// the first intermediate — while its Watch finding is created exactly as
    /// when no path exists.
    #[test]
    fn an_indirectly_linked_pair_names_its_intermediate_and_keeps_its_finding() {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let root = ScopeId::from_index(0);
        builder.add_scope(Scope::new(root, ScopeKind::Repository, ".", None));
        builder.set_root(root);
        let packages = ["a", "b", "c"]
            .into_iter()
            .enumerate()
            .map(|(index, name)| {
                let scope = ScopeId::from_index(index + 1);
                builder.add_scope(Scope::new(scope, ScopeKind::Package, name, Some(root)));
                PackageRecord::current(PackageId::from_index(index), scope, name)
            })
            .collect::<Vec<_>>();
        builder.set_packages(packages);
        let edge = |index: usize, source: usize, target: usize| {
            PackageEdge::new(
                PackageEdgeId::from_index(index),
                PackageId::from_index(source),
                PackageId::from_index(target),
                1,
                1,
                Vec::new(),
            )
        };
        builder.set_architecture(ArchitectureReportFacts::new(
            ArchitectureGraph::new(
                DependencyCoverage::default(),
                Vec::new(),
                vec![edge(0, 0, 1), edge(1, 1, 2)],
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
            Vec::new(),
            Vec::new(),
        ));
        builder.set_explanation_pairs(std::collections::BTreeSet::from([
            (PackageId::from_index(0), PackageId::from_index(1)),
            (PackageId::from_index(1), PackageId::from_index(2)),
        ]));
        let couplings = [(0, 1), (0, 2)]
            .into_iter()
            .map(|(left, right)| {
                ChangeCoupling::new(
                    PackageId::from_index(left),
                    PackageId::from_index(right),
                    3,
                    4,
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
        for index in 0..2 {
            builder.link_evolutionary_finding(root, EvolutionaryFindingId::from_index(index));
        }
        let report = builder.finish();

        let terminal = render(&report, TerminalOptions::default());
        assert!(
            terminal.contains(
                "  watch changes together · a ↔ b\n        changed together in 3 of 4 commits · 75% · code dependency exists\n"
            ),
            "{terminal}"
        );
        assert!(
            terminal.contains(
                "  watch changes together · a ↔ c\n        changed together in 3 of 4 commits · 75% · no direct dependency · linked via b\n"
            ),
            "{terminal}"
        );
        assert!(!terminal.contains("no code dependency"), "{terminal}");
    }

    #[test]
    fn activity_changes_the_rank_the_cards_are_read_in() {
        let terminal = render(&report_with_findings(true), TerminalOptions::default());
        assert!(terminal.contains("PROBLEMS"), "{terminal}");
        // Eleven cards buy one evidence line each, and the most recently
        // touched file heads the section.
        let heads = problem_heads(&terminal);
        assert_eq!(heads.len(), 11, "{terminal}");
        assert!(heads[0].contains("file-10.rs"), "{terminal}");
    }

    #[test]
    fn absent_activity_keeps_the_problem_heading() {
        assert!(
            render(&report_with_findings(false), TerminalOptions::default())
                .contains("\nPROBLEMS\n")
        );
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
