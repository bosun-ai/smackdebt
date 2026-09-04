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

/// The architectural meaning of one extracted static relation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StaticRelationKind {
    Uses,
    ModuleOwnership,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DependencyIntent {
    Internal,
    Package,
}

/// The candidate that names the file declaring the reference.
///
/// A symbolic candidate is not a repository path.  It is consulted only when no
/// path candidate of the same reference matches a discovered file.
pub const DECLARING_FILE_CANDIDATE: &str = ".";

/// The candidate that names the module root of the declaring file's package.
pub const CRATE_ROOT_CANDIDATE: &str = "crate";

/// The candidate that names the file declaring the module enclosing the
/// declaring file.
///
/// The enclosing module can live beside the declaring file's directory
/// (`src/a.rs` for `src/a/b.rs`), a spelling no relative path from the
/// declaring file expresses without knowing that directory's name.
pub const PARENT_MODULE_CANDIDATE: &str = "super";

/// Whether a candidate names a symbolic target rather than a repository path.
pub fn is_symbolic_candidate(candidate: &str) -> bool {
    matches!(
        candidate,
        DECLARING_FILE_CANDIDATE | CRATE_ROOT_CANDIDATE | PARENT_MODULE_CANDIDATE
    )
}

/// Why a dependency cannot safely produce path candidates.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DependencySyntaxState {
    Candidates(Vec<String>),
    External,
    Unresolved(String),
}

/// The configuration scope a reference is declared under.
///
/// A test scope means the reference only exists when the language's test
/// configuration is active, so it describes test code even when the file that
/// declares it ships in production.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DependencyScope {
    #[default]
    Default,
    Test,
}

/// A grammar-owned dependency reference before repository resolution.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DependencySyntax {
    kind: DependencyKind,
    target: String,
    span: SourceSpan,
    state: DependencySyntaxState,
    intent: DependencyIntent,
    relation: StaticRelationKind,
    scope: DependencyScope,
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
            relation: StaticRelationKind::Uses,
            scope: DependencyScope::Default,
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
    pub const fn relation(&self) -> StaticRelationKind {
        self.relation
    }
    pub const fn scope(&self) -> DependencyScope {
        self.scope
    }

    pub fn with_internal_intent(mut self) -> Self {
        self.intent = DependencyIntent::Internal;
        self
    }
    pub fn with_relation(mut self, relation: StaticRelationKind) -> Self {
        self.relation = relation;
        self
    }
    pub fn with_test_scope(mut self) -> Self {
        self.scope = DependencyScope::Test;
        self
    }

    /// Returns the same reference relocated to `span`.
    ///
    /// Every other field is carried unchanged, so a new field cannot be
    /// silently dropped by a relocation.
    #[must_use]
    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = span;
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
    Astro,
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

    /// The role this one becomes where a test configuration selects the code.
    ///
    /// Only production code is reclassified: every other role already states
    /// what the file is, so a test configuration tells nothing new about it.
    pub const fn demoted_by_test_scope(self) -> Self {
        match self {
            Self::Primary => Self::Test,
            other => other,
        }
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

/// Private evidence used to pair one unit across two versions of a file.
///
/// Language adapters can create this value, but only analysis can inspect it.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UnitMatchEvidence(UnitMatchKey);

impl std::fmt::Debug for UnitMatchEvidence {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("UnitMatchEvidence")
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum UnitMatchKey {
    Declared,
    Semantic {
        language_container: Option<String>,
        declared_unit: Option<UnitIdentity>,
        kind: UnitKind,
        anchor: String,
    },
    Fingerprint {
        digest: [u8; 32],
        syntax_len: u64,
    },
    None,
}

impl UnitMatchEvidence {
    pub const fn declared() -> Self {
        Self(UnitMatchKey::Declared)
    }

    pub fn semantic(
        language_container: Option<&str>,
        declared_unit: Option<&UnitIdentity>,
        kind: UnitKind,
        anchor: impl Into<String>,
    ) -> Self {
        Self(UnitMatchKey::Semantic {
            language_container: language_container.map(str::to_owned),
            declared_unit: declared_unit.cloned(),
            kind,
            anchor: anchor.into(),
        })
    }

    pub fn exact_syntax(syntax: &[u8]) -> Self {
        Self(UnitMatchKey::Fingerprint {
            digest: *blake3::hash(syntax).as_bytes(),
            syntax_len: syntax.len() as u64,
        })
    }

    pub const fn none() -> Self {
        Self(UnitMatchKey::None)
    }

    pub(crate) const fn key(&self) -> &UnitMatchKey {
        &self.0
    }
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
    match_evidence: UnitMatchEvidence,
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
            match_evidence: UnitMatchEvidence::declared(),
        }
    }

    pub fn with_match_evidence(mut self, match_evidence: UnitMatchEvidence) -> Self {
        self.match_evidence = match_evidence;
        self
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

    pub(crate) const fn match_evidence(&self) -> &UnitMatchEvidence {
        &self.match_evidence
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
