use crate::{ComparisonDirection, FileId, PackageId, Rating, SourceRole, SourceSpan, SourceTrust};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ArchitectureGraph {
    pub(crate) coverage: DependencyCoverage,
    pub(crate) dependency_edges: Vec<DependencyEdge>,
    pub(crate) package_edges: Vec<PackageEdge>,
    pub(crate) external_dependencies: Vec<ExternalDependency>,
    pub(crate) resolution_diagnostics: Vec<ResolutionDiagnostic>,
    pub(crate) package_graph: Vec<PackageGraphMeasurement>,
}

impl ArchitectureGraph {
    pub fn new(
        coverage: DependencyCoverage,
        dependency_edges: Vec<DependencyEdge>,
        package_edges: Vec<PackageEdge>,
        external_dependencies: Vec<ExternalDependency>,
        resolution_diagnostics: Vec<ResolutionDiagnostic>,
        package_graph: Vec<PackageGraphMeasurement>,
    ) -> Self {
        Self {
            coverage,
            dependency_edges,
            package_edges,
            external_dependencies,
            resolution_diagnostics,
            package_graph,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ArchitectureReportFacts {
    pub(crate) graph: ArchitectureGraph,
    pub(crate) findings: Vec<ArchitectureFinding>,
    pub(crate) comparisons: Vec<ArchitectureComparison>,
}

impl ArchitectureReportFacts {
    pub fn new(
        graph: ArchitectureGraph,
        findings: Vec<ArchitectureFinding>,
        comparisons: Vec<ArchitectureComparison>,
    ) -> Self {
        Self {
            graph,
            findings,
            comparisons,
        }
    }
}

macro_rules! architecture_index {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u32);

        impl $name {
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
    };
}

architecture_index!(DependencyEdgeId);
architecture_index!(PackageEdgeId);
architecture_index!(ArchitectureFindingId);
architecture_index!(ArchitectureComparisonId);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyEdge {
    id: DependencyEdgeId,
    source: FileId,
    target: FileId,
    references: u32,
    locations: Vec<SourceSpan>,
    role: SourceRole,
    trust: SourceTrust,
}

impl DependencyEdge {
    pub fn new(
        id: DependencyEdgeId,
        source: FileId,
        target: FileId,
        references: u32,
        locations: Vec<SourceSpan>,
    ) -> Self {
        Self {
            id,
            source,
            target,
            references,
            locations,
            role: SourceRole::Primary,
            trust: SourceTrust::Trusted,
        }
    }
    pub fn with_evidence(mut self, role: SourceRole, trust: SourceTrust) -> Self {
        self.role = role;
        self.trust = trust;
        self
    }
    pub const fn id(&self) -> DependencyEdgeId {
        self.id
    }
    pub const fn source(&self) -> FileId {
        self.source
    }
    pub const fn target(&self) -> FileId {
        self.target
    }
    pub const fn references(&self) -> u32 {
        self.references
    }
    pub fn locations(&self) -> &[SourceSpan] {
        &self.locations
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageEdge {
    id: PackageEdgeId,
    source: PackageId,
    target: PackageId,
    file_pairs: u32,
    references: u32,
    file_edges: Vec<DependencyEdgeId>,
}

impl PackageEdge {
    pub fn new(
        id: PackageEdgeId,
        source: PackageId,
        target: PackageId,
        file_pairs: u32,
        references: u32,
        file_edges: Vec<DependencyEdgeId>,
    ) -> Self {
        Self {
            id,
            source,
            target,
            file_pairs,
            references,
            file_edges,
        }
    }
    pub const fn id(&self) -> PackageEdgeId {
        self.id
    }
    pub const fn source(&self) -> PackageId {
        self.source
    }
    pub const fn target(&self) -> PackageId {
        self.target
    }
    pub const fn file_pairs(&self) -> u32 {
        self.file_pairs
    }
    pub const fn references(&self) -> u32 {
        self.references
    }
    pub fn file_edges(&self) -> &[DependencyEdgeId] {
        &self.file_edges
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageGraphMeasurement {
    package: PackageId,
    fan_in: u32,
    fan_out: u32,
    instability: Option<Instability>,
}

impl PackageGraphMeasurement {
    pub const fn new(package: PackageId, fan_in: u32, fan_out: u32) -> Self {
        Self {
            package,
            fan_in,
            fan_out,
            instability: Instability::new(fan_out, fan_in + fan_out),
        }
    }
    pub const fn package(self) -> PackageId {
        self.package
    }
    pub const fn fan_in(self) -> u32 {
        self.fan_in
    }
    pub const fn fan_out(self) -> u32 {
        self.fan_out
    }
    pub const fn instability(self) -> Option<Instability> {
        self.instability
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Instability {
    numerator: u32,
    denominator: u32,
}

impl Instability {
    pub const fn new(numerator: u32, denominator: u32) -> Option<Self> {
        if denominator == 0 {
            None
        } else {
            Some(Self {
                numerator,
                denominator,
            })
        }
    }
    pub const fn numerator(self) -> u32 {
        self.numerator
    }
    pub const fn denominator(self) -> u32 {
        self.denominator
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchitectureFindingKind {
    PackageCycle,
    FileCycle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchitectureFinding {
    id: ArchitectureFindingId,
    kind: ArchitectureFindingKind,
    rating: Rating,
    packages: Vec<PackageId>,
    files: Vec<FileId>,
    witness_edges: Vec<DependencyEdgeId>,
}

impl ArchitectureFinding {
    pub fn new(
        id: ArchitectureFindingId,
        kind: ArchitectureFindingKind,
        packages: Vec<PackageId>,
        files: Vec<FileId>,
        witness_edges: Vec<DependencyEdgeId>,
    ) -> Self {
        let rating = match kind {
            ArchitectureFindingKind::PackageCycle => Rating::High,
            ArchitectureFindingKind::FileCycle => Rating::Watch,
        };
        Self {
            id,
            kind,
            rating,
            packages,
            files,
            witness_edges,
        }
    }
    pub const fn id(&self) -> ArchitectureFindingId {
        self.id
    }
    pub const fn kind(&self) -> ArchitectureFindingKind {
        self.kind
    }
    pub const fn rating(&self) -> Rating {
        self.rating
    }
    pub fn packages(&self) -> &[PackageId] {
        &self.packages
    }
    pub fn files(&self) -> &[FileId] {
        &self.files
    }
    pub fn witness_edges(&self) -> &[DependencyEdgeId] {
        &self.witness_edges
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchitectureComparisonKind {
    EdgeAdded,
    EdgeRemoved,
    CycleIntroduced,
    CycleRemoved,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchitectureComparison {
    id: ArchitectureComparisonId,
    kind: ArchitectureComparisonKind,
    direction: ComparisonDirection,
    packages: Vec<PackageId>,
    witness: Vec<PackageId>,
    files: Vec<FileId>,
}

impl ArchitectureComparison {
    pub fn new(
        id: ArchitectureComparisonId,
        kind: ArchitectureComparisonKind,
        packages: Vec<PackageId>,
    ) -> Self {
        let direction = match kind {
            ArchitectureComparisonKind::CycleIntroduced => ComparisonDirection::Worse,
            ArchitectureComparisonKind::CycleRemoved => ComparisonDirection::Better,
            ArchitectureComparisonKind::EdgeAdded | ArchitectureComparisonKind::EdgeRemoved => {
                ComparisonDirection::Changed
            }
        };
        Self {
            id,
            kind,
            direction,
            packages,
            witness: Vec::new(),
            files: Vec::new(),
        }
    }
    pub fn with_witness(mut self, witness: Vec<PackageId>) -> Self {
        self.witness = witness;
        self
    }
    pub fn with_files(mut self, files: Vec<FileId>) -> Self {
        self.files = files;
        self
    }
    pub const fn id(&self) -> ArchitectureComparisonId {
        self.id
    }
    pub const fn kind(&self) -> ArchitectureComparisonKind {
        self.kind
    }
    pub const fn direction(&self) -> ComparisonDirection {
        self.direction
    }
    pub fn packages(&self) -> &[PackageId] {
        &self.packages
    }
    pub fn witness(&self) -> &[PackageId] {
        &self.witness
    }
    pub fn files(&self) -> &[FileId] {
        &self.files
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DependencyCoverage {
    internal: u32,
    external: u32,
    unresolved: u32,
    ambiguous: u32,
}

impl DependencyCoverage {
    pub const fn new(internal: u32, external: u32, unresolved: u32, ambiguous: u32) -> Self {
        Self {
            internal,
            external,
            unresolved,
            ambiguous,
        }
    }
    pub const fn internal(self) -> u32 {
        self.internal
    }
    pub const fn external(self) -> u32 {
        self.external
    }
    pub const fn unresolved(self) -> u32 {
        self.unresolved
    }
    pub const fn ambiguous(self) -> u32 {
        self.ambiguous
    }
    pub const fn total(self) -> u32 {
        self.internal + self.external + self.unresolved + self.ambiguous
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolutionIssueKind {
    Unresolved,
    Ambiguous,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolutionDiagnostic {
    file: FileId,
    span: SourceSpan,
    target: String,
    kind: ResolutionIssueKind,
    reason: String,
}

impl ResolutionDiagnostic {
    pub fn new(
        file: FileId,
        span: SourceSpan,
        target: impl Into<String>,
        kind: ResolutionIssueKind,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            file,
            span,
            target: target.into(),
            kind,
            reason: reason.into(),
        }
    }
    pub const fn file(&self) -> FileId {
        self.file
    }
    pub const fn span(&self) -> SourceSpan {
        self.span
    }
    pub fn target(&self) -> &str {
        &self.target
    }
    pub const fn kind(&self) -> ResolutionIssueKind {
        self.kind
    }
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalDependency {
    file: FileId,
    target: String,
    references: u32,
}

impl ExternalDependency {
    pub fn new(file: FileId, target: impl Into<String>, references: u32) -> Self {
        Self {
            file,
            target: target.into(),
            references,
        }
    }
    pub const fn file(&self) -> FileId {
        self.file
    }
    pub fn target(&self) -> &str {
        &self.target
    }
    pub const fn references(&self) -> u32 {
        self.references
    }
}
