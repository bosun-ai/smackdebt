#![doc = "Pure measurements, health policy, comparison, and report values."]

use std::cmp::Ordering;

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
index_type!(LocalUnitId);
index_type!(UnitId);
index_type!(FindingId);
index_type!(DiagnosticId);
index_type!(ComparisonId);
index_type!(PathId);

/// The progressive levels at which a report can be explored.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ScopeKind {
    Repository,
    Package,
    Directory,
    File,
}

/// A source location using one-based inclusive lines.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceSpan {
    start_line: u32,
    end_line: u32,
}

impl SourceSpan {
    pub const fn new(start_line: u32, end_line: u32) -> Self {
        Self {
            start_line,
            end_line,
        }
    }

    pub const fn start_line(self) -> u32 {
        self.start_line
    }

    pub const fn end_line(self) -> u32 {
        self.end_line
    }

    pub const fn line_count(self) -> u32 {
        if self.end_line < self.start_line {
            0
        } else {
            self.end_line - self.start_line + 1
        }
    }
}

/// Language classification retained in report facts.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Language {
    C,
    Cpp,
    Java,
    JavaScript,
    Jsx,
    Python,
    Rust,
    TypeScript,
    Tsx,
    Ruby,
    Vue,
    Kotlin,
    Unknown,
}

/// The kind of a measured code unit.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum UnitKind {
    Function,
    Method,
    Closure,
    Lambda,
    SyntheticTopLevel,
    Template,
}

/// Identity used to match units across two source versions.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UnitIdentity {
    container: Option<String>,
    kind: UnitKind,
    name: String,
}

impl UnitIdentity {
    pub fn new(name: impl Into<String>, kind: UnitKind) -> Self {
        Self {
            container: None,
            kind,
            name: name.into(),
        }
    }

    pub fn in_container(mut self, container: impl Into<String>) -> Self {
        self.container = Some(container.into());
        self
    }

    pub fn container(&self) -> Option<&str> {
        self.container.as_deref()
    }

    pub const fn kind(&self) -> UnitKind {
        self.kind
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// The three values retained by the product policy.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Measurements {
    cognitive_complexity: u32,
    cyclomatic_complexity: u32,
    logical_lines: u32,
}

impl Measurements {
    pub const fn new(
        cognitive_complexity: u32,
        cyclomatic_complexity: u32,
        logical_lines: u32,
    ) -> Self {
        Self {
            cognitive_complexity,
            cyclomatic_complexity,
            logical_lines,
        }
    }

    pub const fn cognitive_complexity(self) -> u32 {
        self.cognitive_complexity
    }

    pub const fn cyclomatic_complexity(self) -> u32 {
        self.cyclomatic_complexity
    }

    pub const fn logical_lines(self) -> u32 {
        self.logical_lines
    }
}

/// Parser result retained at the file boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseStatus {
    Parsed,
    Recovered,
    Failed,
}

/// A measured unit before health policy is applied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitFact {
    local_id: LocalUnitId,
    identity: UnitIdentity,
    span: SourceSpan,
    measurements: Measurements,
    parent: Option<LocalUnitId>,
}

impl UnitFact {
    pub fn new(
        local_id: LocalUnitId,
        identity: UnitIdentity,
        span: SourceSpan,
        measurements: Measurements,
        parent: Option<LocalUnitId>,
    ) -> Self {
        Self {
            local_id,
            identity,
            span,
            measurements,
            parent,
        }
    }

    pub const fn local_id(&self) -> LocalUnitId {
        self.local_id
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

    pub const fn parent(&self) -> Option<LocalUnitId> {
        self.parent
    }
}

/// Facts produced for exactly one selected source file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileAnalysis {
    file: FileId,
    language: Language,
    source_lines: u32,
    parse_status: ParseStatus,
    units: Vec<UnitFact>,
}

impl FileAnalysis {
    pub fn new(
        file: FileId,
        language: Language,
        source_lines: u32,
        parse_status: ParseStatus,
        units: Vec<UnitFact>,
    ) -> Self {
        Self {
            file,
            language,
            source_lines,
            parse_status,
            units,
        }
    }

    pub const fn file(&self) -> FileId {
        self.file
    }

    pub const fn language(&self) -> Language {
        self.language
    }

    pub const fn source_lines(&self) -> u32 {
        self.source_lines
    }

    pub const fn parse_status(&self) -> &ParseStatus {
        &self.parse_status
    }

    pub fn units(&self) -> &[UnitFact] {
        &self.units
    }
}

/// A threshold pair for one signal.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Thresholds {
    watch: u32,
    high: u32,
}

impl Thresholds {
    pub const fn new(watch: u32, high: u32) -> Self {
        Self { watch, high }
    }

    pub const fn watch(self) -> u32 {
        self.watch
    }

    pub const fn high(self) -> u32 {
        self.high
    }

    const fn level(self, value: u32) -> Rating {
        if value >= self.high {
            Rating::High
        } else if value >= self.watch {
            Rating::Watch
        } else {
            Rating::Healthy
        }
    }
}

/// Independent health signals retained for an assessed unit.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Signal {
    CognitiveComplexity,
    CyclomaticComplexity,
    LogicalLines,
}

/// The severity of one signal or a unit as a whole.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Rating {
    Healthy,
    Watch,
    High,
}

impl Rating {
    const fn rank(self) -> u8 {
        match self {
            Self::Healthy => 0,
            Self::Watch => 1,
            Self::High => 2,
        }
    }
}

/// One threshold result, stored without heap allocation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SignalAssessment {
    signal: Signal,
    value: u32,
    rating: Rating,
}

impl SignalAssessment {
    pub const fn signal(self) -> Signal {
        self.signal
    }

    pub const fn value(self) -> u32 {
        self.value
    }

    pub const fn rating(self) -> Rating {
        self.rating
    }
}

/// The health result for one unit. The array is intentionally fixed at three signals.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct HealthAssessment {
    rating: Rating,
    signals: [SignalAssessment; 3],
}

impl HealthAssessment {
    pub const fn rating(self) -> Rating {
        self.rating
    }

    pub const fn signals(self) -> [SignalAssessment; 3] {
        self.signals
    }

    pub const fn signal(self, signal: Signal) -> SignalAssessment {
        match signal {
            Signal::CognitiveComplexity => self.signals[0],
            Signal::CyclomaticComplexity => self.signals[1],
            Signal::LogicalLines => self.signals[2],
        }
    }
}

/// Configurable policy for the three retained measurements.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct HealthPolicy {
    cognitive: Thresholds,
    cyclomatic: Thresholds,
    logical_lines: Thresholds,
}

impl Default for HealthPolicy {
    fn default() -> Self {
        Self {
            cognitive: Thresholds::new(15, 25),
            cyclomatic: Thresholds::new(11, 21),
            logical_lines: Thresholds::new(50, 100),
        }
    }
}

impl HealthPolicy {
    pub const fn new(
        cognitive: Thresholds,
        cyclomatic: Thresholds,
        logical_lines: Thresholds,
    ) -> Self {
        Self {
            cognitive,
            cyclomatic,
            logical_lines,
        }
    }

    pub const fn cognitive(self) -> Thresholds {
        self.cognitive
    }

    pub const fn cyclomatic(self) -> Thresholds {
        self.cyclomatic
    }

    pub const fn logical_lines(self) -> Thresholds {
        self.logical_lines
    }

    pub const fn assess(self, measurements: Measurements) -> HealthAssessment {
        let signals = [
            SignalAssessment {
                signal: Signal::CognitiveComplexity,
                value: measurements.cognitive_complexity,
                rating: self.cognitive.level(measurements.cognitive_complexity),
            },
            SignalAssessment {
                signal: Signal::CyclomaticComplexity,
                value: measurements.cyclomatic_complexity,
                rating: self.cyclomatic.level(measurements.cyclomatic_complexity),
            },
            SignalAssessment {
                signal: Signal::LogicalLines,
                value: measurements.logical_lines,
                rating: self.logical_lines.level(measurements.logical_lines),
            },
        ];
        let mut rating = Rating::Healthy;
        let mut index = 0;
        while index < signals.len() {
            if signals[index].rating.rank() > rating.rank() {
                rating = signals[index].rating;
            }
            index += 1;
        }
        HealthAssessment { rating, signals }
    }
}

/// Counts retained by every scope.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct HealthCounts {
    healthy: u32,
    watch: u32,
    high: u32,
}

impl HealthCounts {
    pub const fn new(healthy: u32, watch: u32, high: u32) -> Self {
        Self {
            healthy,
            watch,
            high,
        }
    }

    pub const fn healthy(self) -> u32 {
        self.healthy
    }

    pub const fn watch(self) -> u32 {
        self.watch
    }

    pub const fn high(self) -> u32 {
        self.high
    }

    pub const fn total(self) -> u32 {
        self.healthy + self.watch + self.high
    }

    pub const fn debt(self) -> u32 {
        self.watch + self.high
    }

    pub fn add_rating(&mut self, rating: Rating) {
        match rating {
            Rating::Healthy => self.healthy += 1,
            Rating::Watch => self.watch += 1,
            Rating::High => self.high += 1,
        }
    }

    pub fn add_counts(&mut self, other: Self) {
        self.healthy += other.healthy;
        self.watch += other.watch;
        self.high += other.high;
    }
}

/// Coverage values for a source selection.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Coverage {
    selected_files: u32,
    analyzed_files: u32,
    unsupported_files: u32,
    failed_files: u32,
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
            analyzed_files,
            unsupported_files,
            failed_files,
            source_lines,
            excluded_lines,
        }
    }

    pub const fn selected_files(self) -> u32 {
        self.selected_files
    }
    pub const fn analyzed_files(self) -> u32 {
        self.analyzed_files
    }
    pub const fn unsupported_files(self) -> u32 {
        self.unsupported_files
    }
    pub const fn failed_files(self) -> u32 {
        self.failed_files
    }
    pub const fn source_lines(self) -> u32 {
        self.source_lines
    }
    pub const fn excluded_lines(self) -> u32 {
        self.excluded_lines
    }

    pub fn combine(self, other: Self) -> Self {
        Self::new(
            self.selected_files + other.selected_files,
            self.analyzed_files + other.analyzed_files,
            self.unsupported_files + other.unsupported_files,
            self.failed_files + other.failed_files,
            self.source_lines + other.source_lines,
            self.excluded_lines + other.excluded_lines,
        )
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
    unit: UnitId,
    file: FileId,
    identity: UnitIdentity,
    span: SourceSpan,
    measurements: Measurements,
    assessment: HealthAssessment,
}

impl Finding {
    pub fn new(
        id: FindingId,
        unit: UnitId,
        file: FileId,
        identity: UnitIdentity,
        span: SourceSpan,
        measurements: Measurements,
        assessment: HealthAssessment,
    ) -> Self {
        Self {
            id,
            unit,
            file,
            identity,
            span,
            measurements,
            assessment,
        }
    }

    pub const fn id(&self) -> FindingId {
        self.id
    }
    pub const fn unit(&self) -> UnitId {
        self.unit
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

/// Whether a diff unit was added, removed, or changed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ComparisonKind {
    Added,
    Removed,
    Improved,
    Regressed,
    MetricChanged,
    Ambiguous,
    Unchanged,
}

/// The user-facing direction of a retained diff comparison.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ComparisonDirection {
    Worse,
    Better,
    Changed,
}

/// A named unit comparison between two source versions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Comparison {
    id: ComparisonId,
    identity: UnitIdentity,
    kind: ComparisonKind,
    before: Option<Measurements>,
    after: Option<Measurements>,
    before_rating: Option<Rating>,
    after_rating: Option<Rating>,
    file: Option<FileId>,
}

impl Comparison {
    pub fn new(
        id: ComparisonId,
        identity: UnitIdentity,
        kind: ComparisonKind,
        before: Option<Measurements>,
        after: Option<Measurements>,
        before_rating: Option<Rating>,
        after_rating: Option<Rating>,
    ) -> Self {
        Self {
            id,
            identity,
            kind,
            before,
            after,
            before_rating,
            after_rating,
            file: None,
        }
    }

    pub const fn id(&self) -> ComparisonId {
        self.id
    }
    pub fn identity(&self) -> &UnitIdentity {
        &self.identity
    }
    pub const fn kind(&self) -> ComparisonKind {
        self.kind
    }
    pub const fn before(&self) -> Option<Measurements> {
        self.before
    }
    pub const fn after(&self) -> Option<Measurements> {
        self.after
    }
    pub const fn before_rating(&self) -> Option<Rating> {
        self.before_rating
    }
    pub const fn after_rating(&self) -> Option<Rating> {
        self.after_rating
    }
    pub const fn file(&self) -> Option<FileId> {
        self.file
    }
    pub fn with_file(mut self, file: FileId) -> Self {
        self.file = Some(file);
        self
    }
    pub const fn direction(&self) -> ComparisonDirection {
        match self.kind {
            ComparisonKind::Regressed => ComparisonDirection::Worse,
            ComparisonKind::Improved => ComparisonDirection::Better,
            ComparisonKind::Added => match self.after_rating {
                Some(Rating::Watch | Rating::High) => ComparisonDirection::Worse,
                _ => ComparisonDirection::Changed,
            },
            ComparisonKind::Removed => match self.before_rating {
                Some(Rating::Watch | Rating::High) => ComparisonDirection::Better,
                _ => ComparisonDirection::Changed,
            },
            ComparisonKind::MetricChanged
            | ComparisonKind::Ambiguous
            | ComparisonKind::Unchanged => ComparisonDirection::Changed,
        }
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
    selected_scope: Option<ScopeId>,
}

/// The source operation represented by a report.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReportMode {
    Codebase,
    Diff,
}

impl Report {
    pub fn with_capacity(
        mode: ReportMode,
        scopes: usize,
        files: usize,
        findings: usize,
        diagnostics: usize,
        comparisons: usize,
    ) -> Self {
        Self {
            schema_version: 1,
            mode,
            root: None,
            scopes: Vec::with_capacity(scopes),
            files: Vec::with_capacity(files),
            findings: Vec::with_capacity(findings),
            diagnostics: Vec::with_capacity(diagnostics),
            comparisons: Vec::with_capacity(comparisons),
            paths: Vec::new(),
            selected_scope: None,
        }
    }

    pub fn new(mode: ReportMode) -> Self {
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
    /// Mutably borrows scope summaries for one post-order aggregation pass.
    pub fn scopes_mut(&mut self) -> &mut [Scope] {
        &mut self.scopes
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
    pub const fn selected_scope(&self) -> Option<ScopeId> {
        self.selected_scope
    }
    pub fn set_selected_scope(&mut self, scope: ScopeId) {
        self.selected_scope = Some(scope);
    }

    pub fn add_scope(&mut self, scope: Scope) -> ScopeId {
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

    pub fn set_root(&mut self, root: ScopeId) {
        self.root = Some(root);
    }

    pub fn add_file(&mut self, file: FileRecord) -> FileId {
        let id = file.id();
        let mut file = file;
        let path_id = self.intern_path(file.path());
        file.set_path_id(path_id);
        self.files.push(file);
        id
    }

    pub fn add_finding(&mut self, finding: Finding) -> FindingId {
        let id = finding.id();
        self.findings.push(finding);
        id
    }

    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic) -> DiagnosticId {
        let id = diagnostic.id();
        self.diagnostics.push(diagnostic);
        id
    }

    pub fn add_comparison(&mut self, comparison: Comparison) -> ComparisonId {
        let id = comparison.id();
        self.comparisons.push(comparison);
        id
    }
}

/// Compares units after both sides have been reduced to sorted identities.
pub fn compare_units(
    before: &[UnitFact],
    after: &[UnitFact],
    policy: HealthPolicy,
) -> Vec<Comparison> {
    let mut left: Vec<&UnitFact> = before.iter().collect();
    let mut right: Vec<&UnitFact> = after.iter().collect();
    left.sort_unstable_by(|a, b| a.identity().cmp(b.identity()));
    right.sort_unstable_by(|a, b| a.identity().cmp(b.identity()));

    let mut comparisons = Vec::with_capacity(left.len() + right.len());
    let mut left_index = 0;
    let mut right_index = 0;
    let mut comparison_id = 0;
    while left_index < left.len() || right_index < right.len() {
        let left_identity = left.get(left_index).map(|unit| unit.identity());
        let right_identity = right.get(right_index).map(|unit| unit.identity());
        let identity_order = match (left_identity, right_identity) {
            (Some(left_identity), Some(right_identity)) => left_identity.cmp(right_identity),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        };

        if identity_order == Ordering::Equal {
            let left_start = left_index;
            while left_index < left.len()
                && left[left_index].identity() == left[left_start].identity()
            {
                left_index += 1;
            }
            let right_start = right_index;
            while right_index < right.len()
                && right[right_index].identity() == right[right_start].identity()
            {
                right_index += 1;
            }
            let identity = left[left_start].identity().clone();
            if left_index - left_start != 1 || right_index - right_start != 1 {
                comparisons.push(Comparison::new(
                    ComparisonId::from_index(comparison_id),
                    identity,
                    ComparisonKind::Ambiguous,
                    None,
                    None,
                    None,
                    None,
                ));
            } else {
                let left_unit = left[left_start];
                let right_unit = right[right_start];
                let left_assessment = policy.assess(left_unit.measurements());
                let right_assessment = policy.assess(right_unit.measurements());
                let kind = match left_assessment.rating().cmp(&right_assessment.rating()) {
                    Ordering::Less => ComparisonKind::Regressed,
                    Ordering::Greater => ComparisonKind::Improved,
                    Ordering::Equal if left_unit.measurements() != right_unit.measurements() => {
                        ComparisonKind::MetricChanged
                    }
                    Ordering::Equal => ComparisonKind::Unchanged,
                };
                comparisons.push(Comparison::new(
                    ComparisonId::from_index(comparison_id),
                    identity,
                    kind,
                    Some(left_unit.measurements()),
                    Some(right_unit.measurements()),
                    Some(left_assessment.rating()),
                    Some(right_assessment.rating()),
                ));
            }
        } else if identity_order == Ordering::Less {
            let start = left_index;
            while left_index < left.len() && left[left_index].identity() == left[start].identity() {
                left_index += 1;
            }
            let identity = left[start].identity().clone();
            if left_index - start != 1 {
                comparisons.push(Comparison::new(
                    ComparisonId::from_index(comparison_id),
                    identity,
                    ComparisonKind::Ambiguous,
                    None,
                    None,
                    None,
                    None,
                ));
            } else {
                let unit = left[start];
                let assessment = policy.assess(unit.measurements());
                comparisons.push(Comparison::new(
                    ComparisonId::from_index(comparison_id),
                    identity,
                    ComparisonKind::Removed,
                    Some(unit.measurements()),
                    None,
                    Some(assessment.rating()),
                    None,
                ));
            }
        } else {
            let start = right_index;
            while right_index < right.len()
                && right[right_index].identity() == right[start].identity()
            {
                right_index += 1;
            }
            let identity = right[start].identity().clone();
            if right_index - start != 1 {
                comparisons.push(Comparison::new(
                    ComparisonId::from_index(comparison_id),
                    identity,
                    ComparisonKind::Ambiguous,
                    None,
                    None,
                    None,
                    None,
                ));
            } else {
                let unit = right[start];
                let assessment = policy.assess(unit.measurements());
                comparisons.push(Comparison::new(
                    ComparisonId::from_index(comparison_id),
                    identity,
                    ComparisonKind::Added,
                    None,
                    Some(unit.measurements()),
                    None,
                    Some(assessment.rating()),
                ));
            }
        }
        comparison_id += 1;
    }
    comparisons
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

/// Classifies a comparison into the compact direction used by distribution views.
pub const fn comparison_direction(comparison: &Comparison) -> ComparisonDirection {
    comparison.direction()
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
    scopes[index].coverage = coverage;
    scopes[index].health = health;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(name: &str, measurements: Measurements) -> UnitFact {
        UnitFact::new(
            LocalUnitId::from_index(0),
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
    fn default_policy_has_documented_limits() {
        let policy = HealthPolicy::default();
        assert_eq!(policy.cognitive(), Thresholds::new(15, 25));
        assert_eq!(policy.cyclomatic(), Thresholds::new(11, 21));
        assert_eq!(policy.logical_lines(), Thresholds::new(50, 100));
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
        let mut second = unit("same", Measurements::new(2, 1, 1));
        second.local_id = LocalUnitId::from_index(1);
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
}
