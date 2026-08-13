use crate::health::Measurements;

/// A compact index into a file's unit table.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocalUnitId(u32);

impl LocalUnitId {
    pub const fn from_index(index: usize) -> Self {
        Self(index as u32)
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A source location using one-based inclusive lines.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceSpan {
    start_line: u32,
    end_line: u32,
}

/// The syntax-level kind of a dependency reference.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DependencyKind {
    Import,
    Include,
    Module,
    Require,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DependencyIntent {
    Internal,
    Package,
}

/// Why a dependency cannot safely produce path candidates.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DependencySyntaxState {
    Candidates(Vec<String>),
    External,
    Unresolved(String),
}

/// A grammar-owned dependency reference before repository resolution.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DependencySyntax {
    kind: DependencyKind,
    target: String,
    span: SourceSpan,
    state: DependencySyntaxState,
    intent: DependencyIntent,
}

impl DependencySyntax {
    pub fn new(
        kind: DependencyKind,
        target: impl Into<String>,
        span: SourceSpan,
        state: DependencySyntaxState,
    ) -> Self {
        Self {
            kind,
            target: target.into(),
            span,
            state,
            intent: DependencyIntent::Package,
        }
    }

    pub const fn kind(&self) -> DependencyKind {
        self.kind
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    pub const fn state(&self) -> &DependencySyntaxState {
        &self.state
    }

    pub const fn intent(&self) -> DependencyIntent {
        self.intent
    }

    pub fn with_internal_intent(mut self) -> Self {
        self.intent = DependencyIntent::Internal;
        self
    }
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

/// The repository role of a selected source file.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SourceRole {
    Primary,
    Test,
    Example,
    Benchmark,
    Fixture,
    Generated,
}

impl SourceRole {
    pub const fn affects_verdict(self) -> bool {
        matches!(
            self,
            Self::Primary | Self::Test | Self::Example | Self::Benchmark
        )
    }
}

/// Whether parsed facts may affect a verdict.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SourceTrust {
    Trusted,
    Advisory,
    Failed,
}

impl ParseStatus {
    pub const fn trust(&self) -> SourceTrust {
        match self {
            Self::Parsed => SourceTrust::Trusted,
            Self::Recovered => SourceTrust::Advisory,
            Self::Failed => SourceTrust::Failed,
        }
    }
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
    language: Language,
    source_lines: u32,
    parse_status: ParseStatus,
    units: Vec<UnitFact>,
    dependencies: Vec<DependencySyntax>,
}

impl FileAnalysis {
    pub fn new(
        language: Language,
        source_lines: u32,
        parse_status: ParseStatus,
        units: Vec<UnitFact>,
    ) -> Self {
        Self::with_dependencies(language, source_lines, parse_status, units, Vec::new())
    }

    pub fn with_dependencies(
        language: Language,
        source_lines: u32,
        parse_status: ParseStatus,
        units: Vec<UnitFact>,
        dependencies: Vec<DependencySyntax>,
    ) -> Self {
        Self {
            language,
            source_lines,
            parse_status,
            units,
            dependencies,
        }
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

    pub fn dependencies(&self) -> &[DependencySyntax] {
        &self.dependencies
    }
}
