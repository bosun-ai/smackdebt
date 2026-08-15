use crate::comparison::{Comparison, ComparisonDirection};
use crate::health::{HealthAssessment, HealthCounts, Measurements, Rating};
use crate::hotspot::Hotspot;
use crate::orphan::OrphanFile;
use crate::size::SizeFinding;
use crate::source::{Language, ParseStatus, SourceRole, SourceSpan, SourceTrust, UnitIdentity};
use crate::verdict::{
    DebtDiffSelection, Verdict, VerdictCounts, WorstOffender, WorstOffenderReason,
};
use crate::{
    ArchitectureComparison, ArchitectureComparisonId, ArchitectureFinding, ArchitectureFindingId,
    ArchitectureFindingKind, ArchitectureReportFacts, DependencyCoverage, DependencyEdge,
    ExternalDependency, PackageEdge, PackageGraphMeasurement, ResolutionDiagnostic,
    StableDependencyFinding,
};
use crate::{
    ChangeCoupling, ContributorConcentration, EvolutionaryComparison, EvolutionaryComparisonId,
    EvolutionaryFinding, EvolutionaryFindingId, EvolutionaryReportFacts, FileHistory,
    HistoryCoverage, KnowledgeConcentrationFinding, PackageHistory,
};
#[cfg(test)]
use crate::{HealthPolicy, LocalUnitId, Signal, Thresholds, compare_units};
use std::cmp::Reverse;

macro_rules! index_type {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u32);

        impl $name {
            /// Creates an index from a table position.
            pub const fn from_index(index: usize) -> Self {
                Self(index as u32)
            }

            /// Returns the table position represented by this index.
            pub const fn index(self) -> usize {
                self.0 as usize
            }

            /// Returns the compact integer representation.
            pub const fn get(self) -> u32 {
                self.0
            }
        }
    };
}

index_type!(PackageId);
index_type!(ScopeId);
index_type!(FileId);
index_type!(FindingId);
index_type!(DiagnosticId);
index_type!(ComparisonId);
index_type!(PathId);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PackagePresence {
    Current,
    BaseOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageRecord {
    id: PackageId,
    scope: ScopeId,
    path: String,
    presence: PackagePresence,
}

impl PackageRecord {
    pub fn current(id: PackageId, scope: ScopeId, path: impl Into<String>) -> Self {
        Self {
            id,
            scope,
            path: path.into(),
            presence: PackagePresence::Current,
        }
    }
    pub fn base_only(id: PackageId, scope: ScopeId, path: impl Into<String>) -> Self {
        Self {
            id,
            scope,
            path: path.into(),
            presence: PackagePresence::BaseOnly,
        }
    }
    pub const fn id(&self) -> PackageId {
        self.id
    }
    pub const fn scope(&self) -> ScopeId {
        self.scope
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub const fn presence(&self) -> PackagePresence {
        self.presence
    }
}

/// The progressive levels at which a report can be explored.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ScopeKind {
    Repository,
    Package,
    Directory,
    File,
}

/// Coverage values for a source selection.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SourceCoverageOutcome {
    Clean,
    Recovered,
    Unsupported,
    Failed,
    Context,
}

/// Coverage values for a source selection.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Coverage {
    selected_files: u32,
    clean_files: u32,
    recovered_files: u32,
    unsupported_files: u32,
    failed_files: u32,
    context_files: u32,
    source_lines: u32,
    excluded_lines: u32,
}

impl Coverage {
    pub const fn new(
        selected_files: u32,
        analyzed_files: u32,
        unsupported_files: u32,
        failed_files: u32,
        source_lines: u32,
        excluded_lines: u32,
    ) -> Self {
        Self {
            selected_files,
            clean_files: analyzed_files,
            recovered_files: 0,
            unsupported_files,
            failed_files,
            context_files: 0,
            source_lines,
            excluded_lines,
        }
    }

    pub const fn classified(
        selected_files: u32,
        outcome: SourceCoverageOutcome,
        source_lines: u32,
        excluded_lines: u32,
    ) -> Self {
        Self {
            selected_files,
            clean_files: if matches!(outcome, SourceCoverageOutcome::Clean) {
                selected_files
            } else {
                0
            },
            recovered_files: if matches!(outcome, SourceCoverageOutcome::Recovered) {
                selected_files
            } else {
                0
            },
            unsupported_files: if matches!(outcome, SourceCoverageOutcome::Unsupported) {
                selected_files
            } else {
                0
            },
            failed_files: if matches!(outcome, SourceCoverageOutcome::Failed) {
                selected_files
            } else {
                0
            },
            context_files: if matches!(outcome, SourceCoverageOutcome::Context) {
                selected_files
            } else {
                0
            },
            source_lines,
            excluded_lines,
        }
    }

    pub const fn selected_files(self) -> u32 {
        self.selected_files
    }
    pub const fn analyzed_files(self) -> u32 {
        self.clean_files + self.recovered_files + self.context_files
    }
    pub const fn clean_files(self) -> u32 {
        self.clean_files
    }
    pub const fn recovered_files(self) -> u32 {
        self.recovered_files
    }
    pub const fn unsupported_files(self) -> u32 {
        self.unsupported_files
    }
    pub const fn failed_files(self) -> u32 {
        self.failed_files
    }
    pub const fn context_files(self) -> u32 {
        self.context_files
    }
    pub const fn source_lines(self) -> u32 {
        self.source_lines
    }
    pub const fn excluded_lines(self) -> u32 {
        self.excluded_lines
    }

    pub fn combine(self, other: Self) -> Self {
        Self {
            selected_files: self.selected_files + other.selected_files,
            clean_files: self.clean_files + other.clean_files,
            recovered_files: self.recovered_files + other.recovered_files,
            unsupported_files: self.unsupported_files + other.unsupported_files,
            failed_files: self.failed_files + other.failed_files,
            context_files: self.context_files + other.context_files,
            source_lines: self.source_lines + other.source_lines,
            excluded_lines: self.excluded_lines + other.excluded_lines,
        }
    }
}

/// Non-merge file activity retained for hotspot context.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct FileActivity {
    touches: u32,
}

impl FileActivity {
    pub const fn new(touches: u32) -> Self {
        Self { touches }
    }
    pub const fn touches(self) -> u32 {
        self.touches
    }
}

/// One inventory file identity and its aggregate source facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileRecord {
    id: FileId,
    scope: ScopeId,
    path: String,
    language: Option<Language>,
    coverage: Coverage,
    health: HealthCounts,
    activity: Option<FileActivity>,
    path_id: Option<PathId>,
    package: Option<PackageId>,
    role: SourceRole,
    parse_status: Option<ParseStatus>,
}

impl FileRecord {
    pub fn new(
        id: FileId,
        scope: ScopeId,
        path: impl Into<String>,
        coverage: Coverage,
        health: HealthCounts,
    ) -> Self {
        Self {
            id,
            scope,
            path: path.into(),
            language: None,
            coverage,
            health,
            activity: None,
            path_id: None,
            package: None,
            role: SourceRole::Primary,
            parse_status: None,
        }
    }

    pub fn with_language(mut self, language: Language) -> Self {
        self.language = Some(language);
        self
    }

    pub fn with_activity(mut self, activity: FileActivity) -> Self {
        self.activity = Some(activity);
        self
    }

    pub const fn id(&self) -> FileId {
        self.id
    }
    pub const fn scope(&self) -> ScopeId {
        self.scope
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub const fn language(&self) -> Option<Language> {
        self.language
    }
    pub const fn coverage(&self) -> Coverage {
        self.coverage
    }
    pub const fn health(&self) -> HealthCounts {
        self.health
    }
    pub const fn activity(&self) -> Option<FileActivity> {
        self.activity
    }
    pub const fn path_id(&self) -> Option<PathId> {
        self.path_id
    }
    pub const fn package(&self) -> Option<PackageId> {
        self.package
    }
    pub fn with_package(mut self, package: PackageId) -> Self {
        self.package = Some(package);
        self
    }
    pub fn with_source_state(mut self, role: SourceRole, status: ParseStatus) -> Self {
        self.role = role;
        self.parse_status = Some(status);
        self
    }
    pub const fn role(&self) -> SourceRole {
        self.role
    }
    pub const fn parse_status(&self) -> Option<&ParseStatus> {
        self.parse_status.as_ref()
    }
    pub fn trust(&self) -> SourceTrust {
        self.parse_status
            .as_ref()
            .map_or(SourceTrust::Failed, ParseStatus::trust)
    }
    fn set_path_id(&mut self, path: PathId) {
        self.path_id = Some(path);
    }
}

/// One flat scope summary and its child links.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scope {
    id: ScopeId,
    kind: ScopeKind,
    name: String,
    parent: Option<ScopeId>,
    children: Vec<ScopeId>,
    files: Vec<FileId>,
    findings: Vec<FindingId>,
    comparisons: Vec<ComparisonId>,
    architecture_findings: Vec<ArchitectureFindingId>,
    architecture_comparisons: Vec<ArchitectureComparisonId>,
    evolutionary_findings: Vec<EvolutionaryFindingId>,
    evolutionary_comparisons: Vec<EvolutionaryComparisonId>,
    coverage: Coverage,
    health: HealthCounts,
    diff: DiffCounts,
    path: Option<PathId>,
}

impl Scope {
    pub fn new(
        id: ScopeId,
        kind: ScopeKind,
        name: impl Into<String>,
        parent: Option<ScopeId>,
    ) -> Self {
        Self {
            id,
            kind,
            name: name.into(),
            parent,
            children: Vec::new(),
            files: Vec::new(),
            findings: Vec::new(),
            comparisons: Vec::new(),
            architecture_findings: Vec::new(),
            architecture_comparisons: Vec::new(),
            evolutionary_findings: Vec::new(),
            evolutionary_comparisons: Vec::new(),
            coverage: Coverage::default(),
            health: HealthCounts::default(),
            diff: DiffCounts::default(),
            path: None,
        }
    }

    pub const fn id(&self) -> ScopeId {
        self.id
    }
    pub const fn kind(&self) -> ScopeKind {
        self.kind
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub const fn parent(&self) -> Option<ScopeId> {
        self.parent
    }
    pub fn children(&self) -> &[ScopeId] {
        &self.children
    }
    pub fn files(&self) -> &[FileId] {
        &self.files
    }
    pub fn findings(&self) -> &[FindingId] {
        &self.findings
    }
    pub fn comparisons(&self) -> &[ComparisonId] {
        &self.comparisons
    }
    pub fn architecture_findings(&self) -> &[ArchitectureFindingId] {
        &self.architecture_findings
    }
    pub fn architecture_comparisons(&self) -> &[ArchitectureComparisonId] {
        &self.architecture_comparisons
    }
    pub fn evolutionary_findings(&self) -> &[EvolutionaryFindingId] {
        &self.evolutionary_findings
    }
    pub fn evolutionary_comparisons(&self) -> &[EvolutionaryComparisonId] {
        &self.evolutionary_comparisons
    }
    pub const fn diff(&self) -> DiffCounts {
        self.diff
    }
    pub const fn path(&self) -> Option<PathId> {
        self.path
    }
    pub const fn coverage(&self) -> Coverage {
        self.coverage
    }
    pub const fn health(&self) -> HealthCounts {
        self.health
    }

    pub fn add_child(&mut self, child: ScopeId) {
        if !self.children.contains(&child) {
            self.children.push(child);
        }
    }
    pub fn add_file(&mut self, file: FileId) {
        if !self.files.contains(&file) {
            self.files.push(file);
        }
    }
    pub fn add_finding(&mut self, finding: FindingId) {
        if !self.findings.contains(&finding) {
            self.findings.push(finding);
        }
    }
    pub fn add_comparison(&mut self, comparison: ComparisonId) {
        if !self.comparisons.contains(&comparison) {
            self.comparisons.push(comparison);
        }
    }
    pub fn add_architecture_finding(&mut self, finding: ArchitectureFindingId) {
        if !self.architecture_findings.contains(&finding) {
            self.architecture_findings.push(finding);
        }
    }
    pub fn add_architecture_comparison(&mut self, comparison: ArchitectureComparisonId) {
        if !self.architecture_comparisons.contains(&comparison) {
            self.architecture_comparisons.push(comparison);
        }
    }
    pub fn add_evolutionary_finding(&mut self, finding: EvolutionaryFindingId) {
        if !self.evolutionary_findings.contains(&finding) {
            self.evolutionary_findings.push(finding);
        }
    }
    pub fn add_evolutionary_comparison(&mut self, comparison: EvolutionaryComparisonId) {
        if !self.evolutionary_comparisons.contains(&comparison) {
            self.evolutionary_comparisons.push(comparison);
        }
    }
    pub fn set_path(&mut self, path: PathId) {
        self.path = Some(path);
    }
    /// Reserves finding links before aggregation.
    pub fn reserve_findings(&mut self, additional: usize) {
        self.findings.reserve(additional);
    }
}

/// Direction counts retained by diff scopes.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct DiffCounts {
    worse: u32,
    better: u32,
    changed: u32,
}

impl DiffCounts {
    pub const fn new(worse: u32, better: u32, changed: u32) -> Self {
        Self {
            worse,
            better,
            changed,
        }
    }
    pub const fn worse(self) -> u32 {
        self.worse
    }
    pub const fn better(self) -> u32 {
        self.better
    }
    pub const fn changed(self) -> u32 {
        self.changed
    }
    pub const fn total(self) -> u32 {
        self.worse + self.better + self.changed
    }
    pub fn add_direction(&mut self, direction: ComparisonDirection) {
        match direction {
            ComparisonDirection::Worse => self.worse += 1,
            ComparisonDirection::Better => self.better += 1,
            ComparisonDirection::Changed => self.changed += 1,
        }
    }
    pub fn add_counts(&mut self, other: Self) {
        self.worse += other.worse;
        self.better += other.better;
        self.changed += other.changed;
    }
}

/// A retained Watch or High unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    id: FindingId,
    file: FileId,
    identity: UnitIdentity,
    span: SourceSpan,
    measurements: Measurements,
    assessment: HealthAssessment,
    role: SourceRole,
    trust: SourceTrust,
}

impl Finding {
    pub fn new(
        id: FindingId,
        file: FileId,
        identity: UnitIdentity,
        span: SourceSpan,
        measurements: Measurements,
        assessment: HealthAssessment,
    ) -> Self {
        Self {
            id,
            file,
            identity,
            span,
            measurements,
            assessment,
            role: SourceRole::Primary,
            trust: SourceTrust::Trusted,
        }
    }

    pub const fn id(&self) -> FindingId {
        self.id
    }
    pub const fn file(&self) -> FileId {
        self.file
    }
    pub fn identity(&self) -> &UnitIdentity {
        &self.identity
    }
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    pub const fn measurements(&self) -> Measurements {
        self.measurements
    }
    pub const fn assessment(&self) -> HealthAssessment {
        self.assessment
    }
    pub fn with_evidence(mut self, role: SourceRole, trust: SourceTrust) -> Self {
        self.role = role;
        self.trust = trust;
        self
    }
    pub const fn role(&self) -> SourceRole {
        self.role
    }
    pub const fn trust(&self) -> SourceTrust {
        self.trust
    }
    pub const fn affects_verdict(&self) -> bool {
        self.role.affects_verdict() && matches!(self.trust, SourceTrust::Trusted)
    }
}

/// The complete source-finding display order owned by analysis policy.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct FindingRank<'a> {
    rating: Reverse<u8>,
    signals_at_rating: Reverse<u8>,
    triggered_signals: Reverse<u8>,
    hot: Reverse<bool>,
    role_class: u8,
    cognitive_complexity: Reverse<u32>,
    cyclomatic_complexity: Reverse<u32>,
    logical_lines: Reverse<u32>,
    activity: Reverse<u32>,
    path: &'a str,
    start_line: u32,
    end_line: u32,
}

impl<'a> FindingRank<'a> {
    /// Ranks one finding, where `hot` states whether its file is a hotspot.
    pub fn new(finding: &Finding, hot: bool, activity: u32, path: &'a str) -> Self {
        let measurements = finding.measurements();
        Self {
            rating: Reverse(finding.assessment().rating().rank()),
            signals_at_rating: Reverse(finding.assessment().signals_at_rating()),
            triggered_signals: Reverse(finding.assessment().triggered_signals()),
            hot: Reverse(hot),
            role_class: role_class(finding.role()),
            cognitive_complexity: Reverse(measurements.cognitive_complexity()),
            cyclomatic_complexity: Reverse(measurements.cyclomatic_complexity()),
            logical_lines: Reverse(measurements.logical_lines()),
            activity: Reverse(activity),
            path,
            start_line: finding.span().start_line(),
            end_line: finding.span().end_line(),
        }
    }
}

/// Primary source ranks before every other role at equal rating, and
/// non-primary source stays visible below it rather than being removed.
const fn role_class(role: SourceRole) -> u8 {
    match role {
        SourceRole::Primary => 0,
        SourceRole::Test
        | SourceRole::Example
        | SourceRole::Benchmark
        | SourceRole::Fixture
        | SourceRole::Generated => 1,
    }
}

/// Why source coverage was excluded from debt analysis.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DiagnosticKind {
    UnsupportedLanguage,
    UnreadableFile,
    OversizedFile,
    ParseFailure,
    AmbiguousIdentity,
    UnsafeReference,
    Other,
}

/// A recoverable file-level issue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    id: DiagnosticId,
    file: Option<FileId>,
    kind: DiagnosticKind,
    message: String,
    excluded_lines: u32,
}

impl Diagnostic {
    pub fn new(
        id: DiagnosticId,
        file: Option<FileId>,
        kind: DiagnosticKind,
        message: impl Into<String>,
        excluded_lines: u32,
    ) -> Self {
        Self {
            id,
            file,
            kind,
            message: message.into(),
            excluded_lines,
        }
    }

    pub const fn id(&self) -> DiagnosticId {
        self.id
    }
    pub const fn file(&self) -> Option<FileId> {
        self.file
    }
    pub const fn kind(&self) -> DiagnosticKind {
        self.kind
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    pub const fn excluded_lines(&self) -> u32 {
        self.excluded_lines
    }
}

/// A full report consumed by output adapters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Report {
    schema_version: u32,
    mode: ReportMode,
    root: Option<ScopeId>,
    scopes: Vec<Scope>,
    files: Vec<FileRecord>,
    findings: Vec<Finding>,
    diagnostics: Vec<Diagnostic>,
    comparisons: Vec<Comparison>,
    paths: Vec<String>,
    packages: Vec<PackageRecord>,
    dependency_coverage: DependencyCoverage,
    dependency_edges: Vec<DependencyEdge>,
    package_edges: Vec<PackageEdge>,
    external_dependencies: Vec<ExternalDependency>,
    resolution_diagnostics: Vec<ResolutionDiagnostic>,
    package_graph: Vec<PackageGraphMeasurement>,
    architecture_findings: Vec<ArchitectureFinding>,
    architecture_comparisons: Vec<ArchitectureComparison>,
    history_coverage: HistoryCoverage,
    file_history: Vec<FileHistory>,
    package_history: Vec<PackageHistory>,
    change_coupling: Vec<ChangeCoupling>,
    contributor_concentration: Vec<ContributorConcentration>,
    evolutionary_findings: Vec<EvolutionaryFinding>,
    evolutionary_comparisons: Vec<EvolutionaryComparison>,
    hotspots: Vec<Hotspot>,
    size_findings: Vec<SizeFinding>,
    orphan_files: Vec<OrphanFile>,
    stable_dependency_findings: Vec<StableDependencyFinding>,
    knowledge_concentration_findings: Vec<KnowledgeConcentrationFinding>,
    verdict: Option<Verdict>,
}

/// The source operation represented by a report.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReportMode {
    Codebase,
    Diff,
}

/// Builds and completes one immutable report.
pub struct ReportBuilder {
    report: Report,
}

impl ReportBuilder {
    pub fn with_capacity(
        mode: ReportMode,
        scopes: usize,
        files: usize,
        findings: usize,
        diagnostics: usize,
        comparisons: usize,
    ) -> Self {
        Self {
            report: Report::with_capacity(mode, scopes, files, findings, diagnostics, comparisons),
        }
    }

    pub fn new(mode: ReportMode) -> Self {
        Self {
            report: Report::new(mode),
        }
    }

    pub fn add_scope(&mut self, scope: Scope) -> ScopeId {
        self.report.add_scope(scope)
    }

    pub fn set_root(&mut self, root: ScopeId) {
        self.report.set_root(root);
    }

    pub fn add_file(&mut self, file: FileRecord) -> FileId {
        self.report.add_file(file)
    }

    pub fn set_packages(&mut self, packages: Vec<PackageRecord>) {
        self.report.packages = packages;
    }

    pub fn add_finding(&mut self, finding: Finding) -> FindingId {
        self.report.add_finding(finding)
    }

    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic) -> DiagnosticId {
        self.report.add_diagnostic(diagnostic)
    }

    pub fn add_comparison(&mut self, comparison: Comparison) -> ComparisonId {
        self.report.add_comparison(comparison)
    }

    pub fn set_architecture(&mut self, facts: ArchitectureReportFacts) {
        self.report.dependency_coverage = facts.graph.coverage;
        self.report.dependency_edges = facts.graph.dependency_edges;
        self.report.package_edges = facts.graph.package_edges;
        self.report.external_dependencies = facts.graph.external_dependencies;
        self.report.resolution_diagnostics = facts.graph.resolution_diagnostics;
        self.report.package_graph = facts.graph.package_graph;
        self.report.architecture_findings = facts.findings;
        self.report.architecture_comparisons = facts.comparisons;
    }

    /// Sets the derived hotspot table, ordered by file table position.
    pub fn set_hotspots(&mut self, hotspots: Vec<Hotspot>) {
        self.report.hotspots = hotspots;
    }

    /// Sets the rated size findings, ordered by file, subject, and container.
    pub fn set_size_findings(&mut self, findings: Vec<SizeFinding>) {
        self.report.size_findings = findings;
    }

    /// Sets the descriptive orphan table, ordered by file table position.
    pub fn set_orphan_files(&mut self, orphans: Vec<OrphanFile>) {
        self.report.orphan_files = orphans;
    }

    /// Sets the stable-dependency findings, ordered by package edge position.
    pub fn set_stable_dependency_findings(&mut self, findings: Vec<StableDependencyFinding>) {
        self.report.stable_dependency_findings = findings;
    }

    pub fn set_evolution(&mut self, facts: EvolutionaryReportFacts) {
        self.report.history_coverage = facts.coverage;
        self.report.file_history = facts.file_history;
        self.report.package_history = facts.package_history;
        self.report.change_coupling = facts.coupling;
        self.report.contributor_concentration = facts.concentration;
        self.report.evolutionary_findings = facts.findings;
        self.report.evolutionary_comparisons = facts.comparisons;
        self.report.knowledge_concentration_findings = facts.concentration_findings;
    }

    pub fn link_architecture_finding(&mut self, scope: ScopeId, finding: ArchitectureFindingId) {
        self.report.scopes[scope.index()].add_architecture_finding(finding);
    }

    pub fn link_architecture_comparison(
        &mut self,
        scope: ScopeId,
        comparison: ArchitectureComparisonId,
    ) {
        self.report.scopes[scope.index()].add_architecture_comparison(comparison);
    }

    pub fn link_evolutionary_finding(&mut self, scope: ScopeId, finding: EvolutionaryFindingId) {
        self.report.scopes[scope.index()].add_evolutionary_finding(finding);
    }

    pub fn link_evolutionary_comparison(
        &mut self,
        scope: ScopeId,
        comparison: EvolutionaryComparisonId,
    ) {
        self.report.scopes[scope.index()].add_evolutionary_comparison(comparison);
    }

    pub fn link_file(&mut self, scope: ScopeId, file: FileId) {
        self.report.scopes[scope.index()].add_file(file);
    }

    pub fn link_finding(&mut self, scope: ScopeId, finding: FindingId) {
        self.report.scopes[scope.index()].add_finding(finding);
    }

    pub fn link_comparison(&mut self, scope: ScopeId, comparison: ComparisonId) {
        self.report.scopes[scope.index()].add_comparison(comparison);
    }

    pub fn diagnostic_count(&self) -> usize {
        self.report.diagnostics.len()
    }

    pub fn files(&self) -> &[FileRecord] {
        &self.report.files
    }

    pub fn finish(mut self) -> Report {
        self.report.aggregate();
        self.report
    }
}

impl Report {
    fn with_capacity(
        mode: ReportMode,
        scopes: usize,
        files: usize,
        findings: usize,
        diagnostics: usize,
        comparisons: usize,
    ) -> Self {
        Self {
            schema_version: 3,
            mode,
            root: None,
            scopes: Vec::with_capacity(scopes),
            files: Vec::with_capacity(files),
            findings: Vec::with_capacity(findings),
            diagnostics: Vec::with_capacity(diagnostics),
            comparisons: Vec::with_capacity(comparisons),
            paths: Vec::new(),
            packages: Vec::new(),
            dependency_coverage: DependencyCoverage::default(),
            dependency_edges: Vec::new(),
            package_edges: Vec::new(),
            external_dependencies: Vec::new(),
            resolution_diagnostics: Vec::new(),
            package_graph: Vec::new(),
            architecture_findings: Vec::new(),
            architecture_comparisons: Vec::new(),
            history_coverage: HistoryCoverage::default(),
            file_history: Vec::new(),
            package_history: Vec::new(),
            change_coupling: Vec::new(),
            contributor_concentration: Vec::new(),
            evolutionary_findings: Vec::new(),
            evolutionary_comparisons: Vec::new(),
            hotspots: Vec::new(),
            size_findings: Vec::new(),
            orphan_files: Vec::new(),
            stable_dependency_findings: Vec::new(),
            knowledge_concentration_findings: Vec::new(),
            verdict: None,
        }
    }

    fn new(mode: ReportMode) -> Self {
        Self::with_capacity(mode, 1, 0, 0, 0, 0)
    }
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }
    pub const fn mode(&self) -> ReportMode {
        self.mode
    }
    pub const fn root(&self) -> Option<ScopeId> {
        self.root
    }
    pub fn scopes(&self) -> &[Scope] {
        &self.scopes
    }
    pub fn files(&self) -> &[FileRecord] {
        &self.files
    }
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    pub fn comparisons(&self) -> &[Comparison] {
        &self.comparisons
    }
    pub fn paths(&self) -> &[String] {
        &self.paths
    }
    pub fn packages(&self) -> &[PackageRecord] {
        &self.packages
    }
    pub const fn dependency_coverage(&self) -> DependencyCoverage {
        self.dependency_coverage
    }
    pub fn dependency_edges(&self) -> &[DependencyEdge] {
        &self.dependency_edges
    }
    pub fn package_edges(&self) -> &[PackageEdge] {
        &self.package_edges
    }
    pub fn external_dependencies(&self) -> &[ExternalDependency] {
        &self.external_dependencies
    }
    pub fn resolution_diagnostics(&self) -> &[ResolutionDiagnostic] {
        &self.resolution_diagnostics
    }
    pub fn package_graph(&self) -> &[PackageGraphMeasurement] {
        &self.package_graph
    }
    pub fn architecture_findings(&self) -> &[ArchitectureFinding] {
        &self.architecture_findings
    }
    pub fn architecture_comparisons(&self) -> &[ArchitectureComparison] {
        &self.architecture_comparisons
    }
    pub const fn history_coverage(&self) -> &HistoryCoverage {
        &self.history_coverage
    }
    pub fn file_history(&self) -> &[FileHistory] {
        &self.file_history
    }
    pub fn package_history(&self) -> &[PackageHistory] {
        &self.package_history
    }
    pub fn change_coupling(&self) -> &[ChangeCoupling] {
        &self.change_coupling
    }
    pub fn contributor_concentration(&self) -> &[ContributorConcentration] {
        &self.contributor_concentration
    }
    pub fn evolutionary_findings(&self) -> &[EvolutionaryFinding] {
        &self.evolutionary_findings
    }
    pub fn evolutionary_comparisons(&self) -> &[EvolutionaryComparison] {
        &self.evolutionary_comparisons
    }
    pub fn hotspots(&self) -> &[Hotspot] {
        &self.hotspots
    }
    pub fn size_findings(&self) -> &[SizeFinding] {
        &self.size_findings
    }
    pub fn orphan_files(&self) -> &[OrphanFile] {
        &self.orphan_files
    }
    pub fn stable_dependency_findings(&self) -> &[StableDependencyFinding] {
        &self.stable_dependency_findings
    }
    pub fn knowledge_concentration_findings(&self) -> &[KnowledgeConcentrationFinding] {
        &self.knowledge_concentration_findings
    }
    /// Whether a file crossed rated debt with enough change activity.
    pub fn is_hotspot(&self, file: FileId) -> bool {
        self.hotspots
            .binary_search_by_key(&file, |hotspot| hotspot.file())
            .is_ok()
    }
    /// The completed answer for the report's root scope.
    pub const fn verdict(&self) -> Option<&Verdict> {
        self.verdict.as_ref()
    }

    /// Answers for any retained scope from the completed report.
    ///
    /// This reads owned tables only: no filesystem, Git, parser, or analysis
    /// work runs, so a renderer can drill into a path without reanalyzing it.
    pub fn scope_verdict(&self, scope: ScopeId) -> Verdict {
        let scope = &self.scopes[scope.index()];
        let counts = VerdictCounts::new(scope.health(), self.high_architecture_findings(scope));
        let worst_offender = self.worst_offender(scope);
        match self.mode {
            ReportMode::Codebase => Verdict::codebase(counts, worst_offender),
            ReportMode::Diff => Verdict::diff(counts, self.debt_diff(scope), worst_offender),
        }
    }

    /// Counts the High-rated architecture findings that escalate a tier.
    fn high_architecture_findings(&self, scope: &Scope) -> u32 {
        let count = scope
            .architecture_findings()
            .iter()
            .filter(|id| self.architecture_findings[id.index()].rating() == Rating::High)
            .count();
        u32::try_from(count).unwrap_or(u32::MAX)
    }

    /// Selects the comparisons and findings that move human debt in a scope.
    fn debt_diff(&self, scope: &Scope) -> DebtDiffSelection {
        let mut selection = DebtDiffSelection::default();
        for &id in scope.comparisons() {
            let comparison = &self.comparisons[id.index()];
            let role = comparison
                .file()
                .map_or(SourceRole::Primary, |file| self.files[file.index()].role());
            selection.select_source(comparison, role);
        }
        for &id in scope.architecture_comparisons() {
            selection.select_architecture(&self.architecture_comparisons[id.index()]);
        }
        for &id in scope.evolutionary_comparisons() {
            selection.select_evolutionary(self.evolutionary_comparisons[id.index()]);
        }
        selection
    }

    /// Names the single worst thing in a scope with its resolved path.
    ///
    /// Fixture and generated debt never moves a verdict, so it can never be
    /// the worst offender either. A scope whose only debt is structural falls
    /// back to the first witness of its first package dependency cycle.
    fn worst_offender(&self, scope: &Scope) -> Option<WorstOffender> {
        let ranked = scope
            .findings()
            .iter()
            .map(|id| &self.findings[id.index()])
            .filter(|finding| finding.affects_verdict())
            .map(|finding| (self.rank(finding), finding))
            .min_by(|left, right| left.0.cmp(&right.0));
        if let Some((_, finding)) = ranked {
            let reason = if self.is_hotspot(finding.file()) {
                WorstOffenderReason::HotAndComplex
            } else {
                WorstOffenderReason::MostComplex
            };
            return Some(WorstOffender::new(
                self.files[finding.file().index()].path(),
                reason,
            ));
        }
        let cycle = scope
            .architecture_findings()
            .iter()
            .map(|id| &self.architecture_findings[id.index()])
            .find(|finding| finding.kind() == ArchitectureFindingKind::PackageCycle)?;
        Some(WorstOffender::new(
            self.cycle_witness_path(cycle)?,
            WorstOffenderReason::PackageDependencyCycle,
        ))
    }

    fn rank(&self, finding: &Finding) -> FindingRank<'_> {
        let file = &self.files[finding.file().index()];
        FindingRank::new(
            finding,
            self.is_hotspot(finding.file()),
            file.activity().map_or(0, FileActivity::touches),
            file.path(),
        )
    }

    /// The path of a cycle's first witness, which is the source of its first
    /// witness edge.
    fn cycle_witness_path(&self, finding: &ArchitectureFinding) -> Option<&str> {
        let witness = finding
            .witness_edges()
            .first()
            .and_then(|id| self.dependency_edges.get(id.index()))
            .map(DependencyEdge::source)
            .or_else(|| finding.files().first().copied());
        witness.map(|file| self.files[file.index()].path())
    }
    fn add_scope(&mut self, scope: Scope) -> ScopeId {
        let id = scope.id();
        let path_id = self.intern_path(scope.name());
        let mut scope = scope;
        scope.set_path(path_id);
        self.scopes.push(scope);
        id
    }

    fn intern_path(&mut self, path: &str) -> PathId {
        if let Some(index) = self.paths.iter().position(|value| value == path) {
            PathId::from_index(index)
        } else {
            let id = PathId::from_index(self.paths.len());
            self.paths.push(path.to_owned());
            id
        }
    }

    fn set_root(&mut self, root: ScopeId) {
        self.root = Some(root);
    }

    fn add_file(&mut self, file: FileRecord) -> FileId {
        let id = file.id();
        let mut file = file;
        let path_id = self.intern_path(file.path());
        file.set_path_id(path_id);
        self.files.push(file);
        id
    }

    fn add_finding(&mut self, finding: Finding) -> FindingId {
        let id = finding.id();
        self.findings.push(finding);
        id
    }

    fn add_diagnostic(&mut self, diagnostic: Diagnostic) -> DiagnosticId {
        let id = diagnostic.id();
        self.diagnostics.push(diagnostic);
        id
    }

    fn add_comparison(&mut self, comparison: Comparison) -> ComparisonId {
        let id = comparison.id();
        self.comparisons.push(comparison);
        id
    }

    /// Completes scope summaries from the report's owned tables.
    fn aggregate(&mut self) {
        let Some(root) = self.root else {
            return;
        };
        aggregate_scope(&mut self.scopes, &self.files, root);
        aggregate_comparison_scope(&mut self.scopes, &self.comparisons, root);
        self.verdict = Some(self.scope_verdict(root));
    }
}

/// Aggregates flat scope summaries in one post-order pass.
pub fn aggregate_scopes(scopes: &mut [Scope], files: &[FileRecord], root: ScopeId) {
    aggregate_scope(scopes, files, root);
}

/// Aggregates retained comparison links and direction counts after comparisons
/// have been added to a report.
pub fn aggregate_comparisons(scopes: &mut [Scope], comparisons: &[Comparison], root: ScopeId) {
    aggregate_comparison_scope(scopes, comparisons, root);
}

fn aggregate_comparison_scope(scopes: &mut [Scope], comparisons: &[Comparison], scope_id: ScopeId) {
    let index = scope_id.index();
    let children = scopes[index].children.clone();
    for child in children {
        aggregate_comparison_scope(scopes, comparisons, child);
    }
    let mut counts = DiffCounts::default();
    let child_ids = scopes[index].children.clone();
    for child in child_ids {
        let child_comparisons = scopes[child.index()].comparisons.clone();
        let child_diff = scopes[child.index()].diff();
        for comparison_id in child_comparisons {
            scopes[index].add_comparison(comparison_id);
        }
        counts.add_counts(child_diff);
    }
    let own_ids = scopes[index].comparisons.clone();
    if scopes[index].kind() == ScopeKind::File {
        for comparison_id in own_ids {
            counts.add_direction(comparisons[comparison_id.index()].direction());
        }
    }
    scopes[index].diff = counts;
}

fn aggregate_scope(scopes: &mut [Scope], files: &[FileRecord], scope_id: ScopeId) {
    let index = scope_id.index();
    // Child links are already stable table indexes. Recursing over them keeps
    // this post-order pass allocation-free after the report tables are built.
    let child_count = scopes[index].children.len();
    for child_index in 0..child_count {
        let child_id = scopes[index].children[child_index];
        aggregate_scope(scopes, files, child_id);
    }

    let mut coverage = Coverage::default();
    let mut health = HealthCounts::default();
    for &file_id in &scopes[index].files {
        let file = &files[file_id.index()];
        coverage = coverage.combine(file.coverage());
        health.add_counts(file.health());
    }
    for &child_id in &scopes[index].children {
        let child = &scopes[child_id.index()];
        coverage = coverage.combine(child.coverage());
        health.add_counts(child.health());
    }
    // Each retained finding is linked from its file scope through every
    // broader scope so progressive renderers can drill in without rescanning.
    // Indexes are copied one at a time, so this remains allocation-free.
    for child_index in 0..scopes[index].children.len() {
        let child_id = scopes[index].children[child_index];
        let finding_count = scopes[child_id.index()].findings.len();
        for finding_index in 0..finding_count {
            let finding_id = scopes[child_id.index()].findings[finding_index];
            scopes[index].add_finding(finding_id);
        }
    }
    for child_index in 0..scopes[index].children.len() {
        let child_id = scopes[index].children[child_index];
        let finding_ids = scopes[child_id.index()].architecture_findings.clone();
        for finding_id in finding_ids {
            scopes[index].add_architecture_finding(finding_id);
        }
        let comparison_ids = scopes[child_id.index()].architecture_comparisons.clone();
        for comparison_id in comparison_ids {
            scopes[index].add_architecture_comparison(comparison_id);
        }
        let finding_ids = scopes[child_id.index()].evolutionary_findings.clone();
        for finding_id in finding_ids {
            scopes[index].add_evolutionary_finding(finding_id);
        }
        let comparison_ids = scopes[child_id.index()].evolutionary_comparisons.clone();
        for comparison_id in comparison_ids {
            scopes[index].add_evolutionary_comparison(comparison_id);
        }
    }
    scopes[index].coverage = coverage;
    scopes[index].health = health;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::architecture::{
        ArchitectureComparisonKind, ArchitectureFinding, ArchitectureFindingKind,
        ArchitectureGraph, DependencyEdge, DependencyEdgeId,
    };
    use crate::hotspot::Hotspot;
    use crate::verdict::{CodebaseTier, DebtFamily, DiffTier, WorstOffenderReason};
    use crate::{ComparisonKind, UnitFact, UnitKind};

    /// Builds a one-package repository whose single file carries the given
    /// units, so verdict composition can be proven without any project work.
    struct ReportFixture {
        builder: ReportBuilder,
        root: ScopeId,
        packages: Vec<PackageRecord>,
        files: usize,
    }

    impl ReportFixture {
        fn new(mode: ReportMode) -> Self {
            let mut builder = ReportBuilder::new(mode);
            let root = ScopeId::from_index(0);
            builder.add_scope(Scope::new(root, ScopeKind::Repository, ".", None));
            builder.set_root(root);
            Self {
                builder,
                root,
                packages: Vec::new(),
                files: 0,
            }
        }

        /// Adds one file scope with its rated counts and returns both ids.
        fn add_file(&mut self, path: &str, health: HealthCounts) -> (ScopeId, FileId) {
            let scope = ScopeId::from_index(self.builder.report.scopes.len());
            self.builder
                .add_scope(Scope::new(scope, ScopeKind::File, path, Some(self.root)));
            self.builder.report.scopes[self.root.index()].add_child(scope);
            let file = FileId::from_index(self.files);
            self.files += 1;
            let package = PackageId::from_index(self.packages.len());
            self.packages
                .push(PackageRecord::current(package, scope, path));
            self.builder.add_file(
                FileRecord::new(file, scope, path, Coverage::new(1, 1, 0, 0, 10, 0), health)
                    .with_package(package)
                    .with_source_state(SourceRole::Primary, ParseStatus::Parsed),
            );
            self.builder.link_file(scope, file);
            (scope, file)
        }

        fn add_finding(&mut self, scope: ScopeId, file: FileId, name: &str, cognitive: u32) {
            let measurements = Measurements::new(cognitive, 1, 1);
            let id = FindingId::from_index(self.builder.report.findings.len());
            self.builder.add_finding(Finding::new(
                id,
                file,
                UnitIdentity::new(name, UnitKind::Function),
                SourceSpan::new(1, 4),
                measurements,
                HealthPolicy::default().assess(measurements),
            ));
            self.builder.link_finding(scope, id);
        }

        fn finish(mut self) -> Report {
            let packages = std::mem::take(&mut self.packages);
            self.builder.set_packages(packages);
            self.builder.finish()
        }
    }

    fn comparison(
        index: usize,
        file: FileId,
        kind: ComparisonKind,
        before: Option<Rating>,
        after: Option<Rating>,
    ) -> Comparison {
        let measurements = Measurements::new(1, 1, 1);
        Comparison::new(
            ComparisonId::from_index(index),
            UnitIdentity::new("work", UnitKind::Function),
            kind,
            before.map(|_| measurements),
            after.map(|_| measurements),
            before,
            after,
        )
        .with_file(file)
    }

    #[test]
    fn the_root_verdict_is_completed_while_the_report_is_built() {
        let mut fixture = ReportFixture::new(ReportMode::Codebase);
        let (scope, file) = fixture.add_file("src/work.rs", HealthCounts::new(97, 2, 1));
        fixture.add_finding(scope, file, "work", 25);
        let report = fixture.finish();
        let verdict = report.verdict().expect("a built report answers");
        assert_eq!(verdict.tier(), CodebaseTier::Worn);
        assert_eq!(verdict.sentence(), "Worn in the usual places.");
        assert_eq!(verdict.counts().checked(), 100);
        assert_eq!(verdict.counts().high(), 1);
        assert_eq!(verdict.diff_tier(), None);
        let offender = verdict
            .worst_offender()
            .expect("rated debt has an offender");
        assert_eq!(offender.path(), "src/work.rs");
        assert_eq!(offender.reason(), WorstOffenderReason::MostComplex);
        assert_eq!(&report.scope_verdict(report.root().unwrap()), verdict);
    }

    #[test]
    fn a_worst_offender_in_a_hot_file_is_hot_and_complex() {
        let mut fixture = ReportFixture::new(ReportMode::Codebase);
        let (scope, file) = fixture.add_file("src/work.rs", HealthCounts::new(97, 2, 1));
        fixture.add_finding(scope, file, "work", 25);
        fixture
            .builder
            .set_hotspots(vec![Hotspot::new(file, Rating::High, 9)]);
        let report = fixture.finish();
        let offender = report.verdict().unwrap().worst_offender().unwrap();
        assert_eq!(offender.reason(), WorstOffenderReason::HotAndComplex);
    }

    #[test]
    fn a_scope_without_debt_or_a_cycle_has_no_worst_offender() {
        let mut fixture = ReportFixture::new(ReportMode::Codebase);
        fixture.add_file("src/work.rs", HealthCounts::new(10, 0, 0));
        let report = fixture.finish();
        let verdict = report.verdict().unwrap();
        assert_eq!(verdict.tier(), CodebaseTier::Clean);
        assert!(verdict.worst_offender().is_none());
    }

    #[test]
    fn a_package_cycle_is_the_worst_offender_when_no_source_finding_exists() {
        let mut fixture = ReportFixture::new(ReportMode::Codebase);
        let (left_scope, left) = fixture.add_file("src/left.rs", HealthCounts::new(10, 0, 0));
        let (_, right) = fixture.add_file("src/right.rs", HealthCounts::new(10, 0, 0));
        let edge = DependencyEdgeId::from_index(0);
        let finding = ArchitectureFinding::new(
            ArchitectureFindingId::from_index(0),
            ArchitectureFindingKind::PackageCycle,
            vec![PackageId::from_index(0), PackageId::from_index(1)],
            vec![left, right],
            vec![edge],
        );
        fixture
            .builder
            .set_architecture(ArchitectureReportFacts::new(
                ArchitectureGraph::new(
                    DependencyCoverage::default(),
                    vec![DependencyEdge::new(edge, left, right, 1, Vec::new())],
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                ),
                vec![finding],
                Vec::new(),
            ));
        fixture
            .builder
            .link_architecture_finding(left_scope, ArchitectureFindingId::from_index(0));
        let report = fixture.finish();
        let verdict = report.verdict().unwrap();
        assert_eq!(verdict.counts().high_architecture_findings(), 1);
        assert_eq!(verdict.tier(), CodebaseTier::Worn);
        let offender = verdict.worst_offender().unwrap();
        assert_eq!(offender.path(), "src/left.rs");
        assert_eq!(
            offender.reason(),
            WorstOffenderReason::PackageDependencyCycle
        );
    }

    #[test]
    fn a_scope_verdict_answers_about_that_scope_rather_than_the_repository() {
        let mut fixture = ReportFixture::new(ReportMode::Codebase);
        let (heavy, heavy_file) = fixture.add_file("src/heavy.rs", HealthCounts::new(0, 0, 2));
        fixture.add_finding(heavy, heavy_file, "heavy", 25);
        let (light, _) = fixture.add_file("src/light.rs", HealthCounts::new(998, 0, 0));
        let report = fixture.finish();
        assert_eq!(report.verdict().unwrap().tier(), CodebaseTier::Worn);
        assert_eq!(report.scope_verdict(heavy).tier(), CodebaseTier::Lost);
        assert_eq!(report.scope_verdict(light).tier(), CodebaseTier::Clean);
        assert!(report.scope_verdict(light).worst_offender().is_none());
        assert_eq!(
            report.scope_verdict(heavy).worst_offender().unwrap().path(),
            "src/heavy.rs"
        );
    }

    #[test]
    fn a_diff_selection_holds_only_moved_debt_and_holds_it_once() {
        let mut fixture = ReportFixture::new(ReportMode::Diff);
        let (scope, file) = fixture.add_file("src/work.rs", HealthCounts::new(10, 1, 0));
        let (fixture_scope, fixture_file) =
            fixture.add_file("tests/fixtures/big.js", HealthCounts::default());
        fixture.builder.report.files[fixture_file.index()] = FileRecord::new(
            fixture_file,
            fixture_scope,
            "tests/fixtures/big.js",
            Coverage::new(1, 0, 0, 0, 10, 0),
            HealthCounts::default(),
        )
        .with_source_state(SourceRole::Fixture, ParseStatus::Parsed);
        let members = [
            comparison(
                0,
                file,
                ComparisonKind::Regressed,
                Some(Rating::Healthy),
                Some(Rating::Watch),
            ),
            comparison(1, file, ComparisonKind::Added, None, Some(Rating::Healthy)),
            comparison(
                2,
                fixture_file,
                ComparisonKind::Regressed,
                Some(Rating::Healthy),
                Some(Rating::High),
            ),
        ];
        for member in members {
            let id = member.id();
            let linked = if member.file() == Some(file) {
                scope
            } else {
                fixture_scope
            };
            fixture.builder.add_comparison(member);
            fixture.builder.link_comparison(linked, id);
        }
        let report = fixture.finish();
        let verdict = report.verdict().unwrap();
        assert_eq!(verdict.selection().source(), &[ComparisonId::from_index(0)]);
        assert!(!verdict.selection().has_duplicate_identity());
        assert_eq!(verdict.diff_tier(), Some(DiffTier::Worse));
        assert_eq!(verdict.sentence(), "You made it worse.");
        assert_eq!(verdict.facts().source(), DiffCounts::new(1, 0, 0));
        // The healthy addition and the fixture regression stay in the machine
        // report while neither moves the verdict.
        assert_eq!(report.comparisons().len(), 3);
        assert_eq!(
            report.scope_verdict(scope).diff_tier(),
            Some(DiffTier::Worse)
        );
        assert_eq!(
            report.scope_verdict(fixture_scope).diff_tier(),
            Some(DiffTier::NoDebtChange)
        );
    }

    #[test]
    fn an_introduced_cycle_makes_a_composed_diff_worse_without_any_source_movement() {
        let mut fixture = ReportFixture::new(ReportMode::Diff);
        let (scope, file) = fixture.add_file("src/work.rs", HealthCounts::new(10, 0, 0));
        let unchanged = comparison(
            0,
            file,
            ComparisonKind::Unchanged,
            Some(Rating::High),
            Some(Rating::High),
        );
        fixture.builder.add_comparison(unchanged);
        fixture
            .builder
            .link_comparison(scope, ComparisonId::from_index(0));
        let introduced = ArchitectureComparison::new(
            ArchitectureComparisonId::from_index(0),
            ArchitectureComparisonKind::CycleIntroduced,
            vec![PackageId::from_index(0), PackageId::from_index(1)],
        );
        fixture
            .builder
            .set_architecture(ArchitectureReportFacts::new(
                ArchitectureGraph::default(),
                Vec::new(),
                vec![introduced],
            ));
        fixture
            .builder
            .link_architecture_comparison(scope, ArchitectureComparisonId::from_index(0));
        let report = fixture.finish();
        let verdict = report.verdict().unwrap();
        assert_eq!(verdict.diff_tier(), Some(DiffTier::Worse));
        assert_eq!(verdict.sentence(), "You made it worse.");
        assert!(verdict.facts().moved(DebtFamily::Architecture));
        assert!(!verdict.facts().moved(DebtFamily::Source));
        assert_eq!(verdict.facts().source(), DiffCounts::default());
        assert_eq!(verdict.facts().architecture(), DiffCounts::new(1, 0, 0));
    }

    fn unit(name: &str, measurements: Measurements) -> UnitFact {
        unit_with_id(0, name, measurements)
    }

    fn unit_with_id(index: usize, name: &str, measurements: Measurements) -> UnitFact {
        UnitFact::new(
            LocalUnitId::from_index(index),
            UnitIdentity::new(name, UnitKind::Function),
            SourceSpan::new(1, 4),
            measurements,
            None,
        )
    }

    #[test]
    fn highest_signal_sets_health_rating_and_preserves_all_signal_values() {
        let policy = HealthPolicy::default();
        let assessment = policy.assess(Measurements::new(15, 2, 120));
        assert_eq!(assessment.rating(), Rating::High);
        assert_eq!(
            assessment.signal(Signal::CognitiveComplexity).rating(),
            Rating::Watch
        );
        assert_eq!(
            assessment.signal(Signal::CyclomaticComplexity).rating(),
            Rating::Healthy
        );
        assert_eq!(
            assessment.signal(Signal::LogicalLines).rating(),
            Rating::High
        );
    }

    #[test]
    fn shape_measurements_are_rated_and_explain_a_rating_on_their_own() {
        let policy = HealthPolicy::default();
        let healthy = Measurements::new(1, 1, 1);
        assert_eq!(healthy.max_nesting(), 0);
        assert_eq!(healthy.parameter_count(), 0);
        assert_eq!(policy.assess(healthy).rating(), Rating::Healthy);
        // A unit whose complexity, cyclomatic, and statement values are healthy
        // is still High when it nests seven levels deep.
        let nested = healthy.with_shape(7, 0);
        assert_eq!(policy.assess(nested).rating(), Rating::High);
        assert_eq!(policy.assess(nested).signal(Signal::MaxNesting).value(), 7);
        let many_parameters = healthy.with_shape(0, 9);
        assert_eq!(policy.assess(many_parameters).rating(), Rating::High);
        assert_eq!(
            policy
                .assess(many_parameters)
                .signal(Signal::ParameterCount)
                .value(),
            9
        );
    }

    #[test]
    fn nesting_and_parameter_thresholds_trigger_on_their_exact_values() {
        let policy = HealthPolicy::default();
        let base = Measurements::new(1, 1, 1);
        let nesting = |value| policy.assess(base.with_shape(value, 0)).rating();
        assert_eq!(nesting(3), Rating::Healthy);
        assert_eq!(nesting(4), Rating::Watch);
        assert_eq!(nesting(6), Rating::Watch);
        assert_eq!(nesting(7), Rating::High);
        let parameters = |value| policy.assess(base.with_shape(0, value)).rating();
        assert_eq!(parameters(5), Rating::Healthy);
        assert_eq!(parameters(6), Rating::Watch);
        assert_eq!(parameters(8), Rating::Watch);
        assert_eq!(parameters(9), Rating::High);
    }

    #[test]
    fn both_promoted_thresholds_are_configurable() {
        let policy = HealthPolicy::new(
            Thresholds::new(15, 25),
            Thresholds::new(11, 21),
            Thresholds::new(50, 100),
            Thresholds::new(2, 3),
            Thresholds::new(2, 3),
        );
        assert_eq!(policy.nesting(), Thresholds::new(2, 3));
        assert_eq!(policy.parameters(), Thresholds::new(2, 3));
        let base = Measurements::new(1, 1, 1);
        assert_eq!(policy.assess(base.with_shape(2, 0)).rating(), Rating::Watch);
        assert_eq!(policy.assess(base.with_shape(0, 3)).rating(), Rating::High);
    }

    #[test]
    fn every_rated_measurement_classifies_a_unit_comparison() {
        let before = [unit("same", Measurements::new(2, 1, 1).with_shape(1, 1))];
        let after = [unit("same", Measurements::new(2, 1, 1).with_shape(2, 1))];
        let comparisons = compare_units(&before, &after, HealthPolicy::default());
        assert_eq!(comparisons[0].kind(), ComparisonKind::MetricChanged);
        let unchanged = compare_units(&before, &before, HealthPolicy::default());
        assert_eq!(unchanged[0].kind(), ComparisonKind::Unchanged);
    }

    #[test]
    fn default_policy_has_documented_limits() {
        let policy = HealthPolicy::default();
        assert_eq!(policy.cognitive(), Thresholds::new(15, 25));
        assert_eq!(policy.cyclomatic(), Thresholds::new(11, 21));
        assert_eq!(policy.logical_lines(), Thresholds::new(50, 100));
        assert_eq!(policy.nesting(), Thresholds::new(4, 7));
        assert_eq!(policy.parameters(), Thresholds::new(6, 9));
    }

    #[test]
    fn comparison_matches_sorted_identities_and_classifies_metric_changes() {
        let before = [
            unit("removed", Measurements::new(1, 1, 1)),
            unit("same", Measurements::new(1, 1, 1)),
        ];
        let after = [
            unit("added", Measurements::new(1, 1, 1)),
            unit("same", Measurements::new(2, 1, 1)),
        ];
        let comparisons = compare_units(&before, &after, HealthPolicy::default());
        assert_eq!(comparisons.len(), 3);
        assert_eq!(comparisons[0].kind(), ComparisonKind::Added);
        assert_eq!(comparisons[1].kind(), ComparisonKind::Removed);
        assert_eq!(comparisons[2].kind(), ComparisonKind::MetricChanged);
    }

    #[test]
    fn duplicate_identity_is_ambiguous_instead_of_guessing() {
        let first = unit("same", Measurements::new(1, 1, 1));
        let second = unit_with_id(1, "same", Measurements::new(2, 1, 1));
        let comparisons = compare_units(&[first, second], &[], HealthPolicy::default());
        assert_eq!(comparisons.len(), 1);
        assert_eq!(comparisons[0].kind(), ComparisonKind::Ambiguous);
    }

    #[test]
    fn comparison_distinguishes_improvement_regression_and_unchanged_units() {
        let before = [
            unit("improved", Measurements::new(25, 1, 1)),
            unit("regressed", Measurements::new(1, 1, 1)),
            unit("changed", Measurements::new(2, 1, 1)),
            unit("unchanged", Measurements::new(1, 1, 1)),
        ];
        let after = [
            unit("improved", Measurements::new(1, 1, 1)),
            unit("regressed", Measurements::new(25, 1, 1)),
            unit("changed", Measurements::new(3, 1, 1)),
            unit("unchanged", Measurements::new(1, 1, 1)),
        ];
        let comparisons = compare_units(&before, &after, HealthPolicy::default());
        assert_eq!(
            comparisons.iter().map(Comparison::kind).collect::<Vec<_>>(),
            vec![
                ComparisonKind::MetricChanged,
                ComparisonKind::Improved,
                ComparisonKind::Regressed,
                ComparisonKind::Unchanged,
            ]
        );
    }

    #[test]
    fn comparison_direction_groups_added_removed_and_metric_changes() {
        let identity = UnitIdentity::new("work", UnitKind::Function);
        let measurements = Measurements::new(1, 1, 1);
        let cases = [
            (
                ComparisonKind::Regressed,
                Some(Rating::Watch),
                ComparisonDirection::Worse,
            ),
            (
                ComparisonKind::Improved,
                Some(Rating::Healthy),
                ComparisonDirection::Better,
            ),
            (
                ComparisonKind::Added,
                Some(Rating::High),
                ComparisonDirection::Worse,
            ),
            (
                ComparisonKind::Added,
                Some(Rating::Healthy),
                ComparisonDirection::Changed,
            ),
            (
                ComparisonKind::Removed,
                Some(Rating::High),
                ComparisonDirection::Better,
            ),
            (
                ComparisonKind::Removed,
                Some(Rating::Healthy),
                ComparisonDirection::Changed,
            ),
            (
                ComparisonKind::MetricChanged,
                Some(Rating::Watch),
                ComparisonDirection::Changed,
            ),
            (
                ComparisonKind::Ambiguous,
                None,
                ComparisonDirection::Changed,
            ),
        ];
        for (kind, after_rating, expected) in cases {
            let before_rating = if kind == ComparisonKind::Removed {
                after_rating
            } else {
                None
            };
            let after_rating = if kind == ComparisonKind::Removed {
                None
            } else {
                after_rating
            };
            let comparison = Comparison::new(
                ComparisonId::from_index(0),
                identity.clone(),
                kind,
                (kind != ComparisonKind::Added).then_some(measurements),
                (kind != ComparisonKind::Removed).then_some(measurements),
                before_rating,
                after_rating,
            );
            assert_eq!(comparison.direction(), expected, "{kind:?}");
        }
    }

    #[test]
    fn aggregation_counts_each_file_and_child_scope_once() {
        let root = ScopeId::from_index(0);
        let child = ScopeId::from_index(1);
        let file = FileId::from_index(0);
        let mut scopes = vec![
            Scope::new(root, ScopeKind::Repository, ".", None),
            Scope::new(child, ScopeKind::File, "a.rs", Some(root)),
        ];
        scopes[root.index()].add_child(child);
        scopes[root.index()].add_child(child);
        scopes[child.index()].add_file(file);
        scopes[child.index()].add_file(file);
        let finding = FindingId::from_index(0);
        scopes[child.index()].add_finding(finding);
        scopes[child.index()].add_finding(finding);
        let files = vec![FileRecord::new(
            file,
            child,
            "a.rs",
            Coverage::new(1, 1, 0, 0, 4, 0),
            HealthCounts::new(1, 0, 0),
        )];
        aggregate_scopes(&mut scopes, &files, root);
        assert_eq!(
            scopes[root.index()].coverage(),
            Coverage::new(1, 1, 0, 0, 4, 0)
        );
        assert_eq!(scopes[root.index()].health(), HealthCounts::new(1, 0, 0));
        assert_eq!(scopes[child.index()].findings(), &[finding]);
        assert_eq!(scopes[root.index()].findings(), &[finding]);
    }

    #[test]
    fn finding_rank_uses_every_accepted_key_in_order() {
        fn finding(name: &str, measurements: Measurements, span: SourceSpan) -> Finding {
            Finding::new(
                FindingId::from_index(0),
                FileId::from_index(0),
                UnitIdentity::new(name, UnitKind::Function),
                span,
                measurements,
                HealthPolicy::default().assess(measurements),
            )
        }
        fn rank<'a>(finding: &Finding, activity: u32, path: &'a str) -> FindingRank<'a> {
            FindingRank::new(finding, false, activity, path)
        }

        let high = finding("high", Measurements::new(25, 1, 1), SourceSpan::new(1, 1));
        let watch = finding("watch", Measurements::new(15, 1, 1), SourceSpan::new(1, 1));
        assert!(rank(&high, 0, "z") < rank(&watch, 99, "a"));

        let two_high = finding("two", Measurements::new(25, 21, 1), SourceSpan::new(1, 1));
        assert!(rank(&two_high, 0, "z") < rank(&high, 99, "a"));

        let high_with_watch = finding(
            "triggered",
            Measurements::new(25, 11, 1),
            SourceSpan::new(1, 1),
        );
        assert!(rank(&high_with_watch, 0, "z") < rank(&high, 99, "a"));

        // Hot debt precedes equally rated cold debt, and primary source
        // precedes non-primary source once hot state ties.
        assert!(FindingRank::new(&high, true, 0, "z") < FindingRank::new(&high, false, 99, "a"));
        // Rating still dominates hot state: cold High precedes hot Watch.
        assert!(FindingRank::new(&high, false, 0, "z") < FindingRank::new(&watch, true, 99, "a"));
        let test_watch = watch
            .clone()
            .with_evidence(SourceRole::Test, SourceTrust::Trusted);
        assert!(
            FindingRank::new(&high, false, 0, "z") < FindingRank::new(&test_watch, true, 99, "a")
        );
        // Hot state still dominates every measurement key: a hot Watch finding
        // precedes a cold Watch finding with strictly more cognitive
        // complexity, more cyclomatic complexity, and more statements.
        let heavier_watch = finding(
            "heavier",
            Measurements::new(20, 11, 50),
            SourceSpan::new(1, 1),
        );
        let lighter_watch = finding(
            "lighter",
            Measurements::new(15, 11, 50),
            SourceSpan::new(1, 1),
        );
        assert_eq!(
            heavier_watch.assessment().rating(),
            lighter_watch.assessment().rating()
        );
        assert_eq!(
            (
                heavier_watch.assessment().signals_at_rating(),
                heavier_watch.assessment().triggered_signals()
            ),
            (
                lighter_watch.assessment().signals_at_rating(),
                lighter_watch.assessment().triggered_signals()
            )
        );
        assert!(rank(&heavier_watch, 0, "a") < rank(&lighter_watch, 0, "a"));
        assert!(
            FindingRank::new(&lighter_watch, true, 0, "z")
                < FindingRank::new(&heavier_watch, false, 99, "a")
        );
        let test_role = high
            .clone()
            .with_evidence(SourceRole::Test, SourceTrust::Trusted);
        assert!(rank(&high, 0, "z") < rank(&test_role, 99, "a"));
        assert!(
            FindingRank::new(&test_role, true, 0, "z") < FindingRank::new(&high, false, 99, "a")
        );

        let more_cognitive = finding(
            "cognitive",
            Measurements::new(26, 1, 1),
            SourceSpan::new(1, 1),
        );
        assert!(rank(&more_cognitive, 0, "z") < rank(&high, 99, "a"));

        let more_cyclomatic = finding(
            "cyclomatic",
            Measurements::new(25, 2, 1),
            SourceSpan::new(1, 1),
        );
        assert!(rank(&more_cyclomatic, 0, "z") < rank(&high, 99, "a"));

        let more_lines = finding("lines", Measurements::new(25, 1, 2), SourceSpan::new(1, 1));
        assert!(rank(&more_lines, 0, "z") < rank(&high, 99, "a"));
        assert!(rank(&high, 2, "z") < rank(&high, 1, "a"));
        assert!(rank(&high, 1, "a") < rank(&high, 1, "b"));

        let earlier = finding(
            "earlier",
            Measurements::new(25, 1, 1),
            SourceSpan::new(1, 2),
        );
        let later = finding("later", Measurements::new(25, 1, 1), SourceSpan::new(2, 3));
        assert!(rank(&earlier, 1, "a") < rank(&later, 1, "a"));
        let shorter = finding(
            "shorter",
            Measurements::new(25, 1, 1),
            SourceSpan::new(1, 1),
        );
        assert!(rank(&shorter, 1, "a") < rank(&earlier, 1, "a"));
    }

    #[test]
    fn the_rank_order_is_total_data_stable_and_keeps_non_primary_debt_visible() {
        let measurements = Measurements::new(25, 1, 1);
        let assessment = HealthPolicy::default().assess(measurements);
        let build = |index: usize, role: SourceRole| {
            Finding::new(
                FindingId::from_index(index),
                FileId::from_index(index),
                UnitIdentity::new("work", UnitKind::Function),
                SourceSpan::new(1, 1),
                measurements,
                assessment,
            )
            .with_evidence(role, SourceTrust::Trusted)
        };
        let hot_test = build(0, SourceRole::Test);
        let cold_primary = build(1, SourceRole::Primary);
        let cold_test = build(2, SourceRole::Test);
        let hot_primary = build(3, SourceRole::Primary);
        let mut ranked = vec![
            (FindingRank::new(&hot_test, true, 1, "a"), "hot test"),
            (FindingRank::new(&cold_primary, false, 1, "a"), "primary"),
            (FindingRank::new(&cold_test, false, 1, "a"), "test"),
            (FindingRank::new(&hot_primary, true, 1, "a"), "hot primary"),
        ];
        ranked.sort_by(|left, right| left.0.cmp(&right.0));
        assert_eq!(
            ranked.iter().map(|entry| entry.1).collect::<Vec<_>>(),
            ["hot primary", "hot test", "primary", "test"]
        );
        let mut reversed = ranked.clone();
        reversed.reverse();
        reversed.sort_by(|left, right| left.0.cmp(&right.0));
        assert_eq!(
            reversed.iter().map(|entry| entry.1).collect::<Vec<_>>(),
            ranked.iter().map(|entry| entry.1).collect::<Vec<_>>()
        );
    }
}
