use crate::{
    ComparisonDirection, FileId, PackageId, Rating, SourceRole, SourceSpan, SourceTrust,
    StaticRelationKind,
};

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
architecture_index!(StableDependencyFindingId);
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
    relation: StaticRelationKind,
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
            relation: StaticRelationKind::Uses,
        }
    }
    pub fn with_evidence(mut self, role: SourceRole, trust: SourceTrust) -> Self {
        self.role = role;
        self.trust = trust;
        self
    }
    pub fn with_relation(mut self, relation: StaticRelationKind) -> Self {
        self.relation = relation;
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
    pub const fn relation(&self) -> StaticRelationKind {
        self.relation
    }
    pub const fn affects_verdict(&self) -> bool {
        matches!(self.relation, StaticRelationKind::Uses)
            && self.role.affects_verdict()
            && matches!(self.trust, SourceTrust::Trusted)
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

/// One package that depends on a less stable package.
///
/// Stable-dependency violations own their table and their own index type, so a
/// row can never be mistaken for a position in the cycle finding table.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StableDependencyFinding {
    id: StableDependencyFindingId,
    rating: Rating,
    source: PackageId,
    target: PackageId,
    evidence: StableDependencyEvidence,
    witness_edges: Vec<DependencyEdgeId>,
}

impl StableDependencyFinding {
    pub fn new(
        id: StableDependencyFindingId,
        source: PackageId,
        target: PackageId,
        evidence: StableDependencyEvidence,
        witness_edges: Vec<DependencyEdgeId>,
    ) -> Self {
        Self {
            id,
            rating: Rating::Watch,
            source,
            target,
            evidence,
            witness_edges,
        }
    }
    pub const fn id(&self) -> StableDependencyFindingId {
        self.id
    }
    pub const fn rating(&self) -> Rating {
        self.rating
    }
    /// The depending package.
    pub const fn source(&self) -> PackageId {
        self.source
    }
    /// The depended-on package.
    pub const fn target(&self) -> PackageId {
        self.target
    }
    pub const fn evidence(&self) -> StableDependencyEvidence {
        self.evidence
    }
    pub fn witness_edges(&self) -> &[DependencyEdgeId] {
        &self.witness_edges
    }
}

/// The exact degree operands behind one stable-dependency violation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StableDependencyEvidence {
    source: PackageGraphMeasurement,
    target: PackageGraphMeasurement,
    references: u32,
}

impl StableDependencyEvidence {
    pub const fn new(
        source: PackageGraphMeasurement,
        target: PackageGraphMeasurement,
        references: u32,
    ) -> Self {
        Self {
            source,
            target,
            references,
        }
    }
    /// The depending package's degree facts.
    pub const fn source(self) -> PackageGraphMeasurement {
        self.source
    }
    /// The depended-on package's degree facts.
    pub const fn target(self) -> PackageGraphMeasurement {
        self.target
    }
    /// References the depending package has into the depended-on package.
    pub const fn references(self) -> u32 {
        self.references
    }
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
    relation: Option<StaticRelationKind>,
    role: Option<SourceRole>,
    trust: Option<SourceTrust>,
    before_references: Option<u32>,
    after_references: Option<u32>,
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
            relation: None,
            role: None,
            trust: None,
            before_references: None,
            after_references: None,
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
    pub fn with_relation_evidence(
        mut self,
        relation: StaticRelationKind,
        role: SourceRole,
        trust: SourceTrust,
    ) -> Self {
        self.relation = Some(relation);
        self.role = Some(role);
        self.trust = Some(trust);
        self
    }
    pub fn with_reference_counts(mut self, before: u32, after: u32) -> Self {
        self.before_references = Some(before);
        self.after_references = Some(after);
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
    pub const fn relation(&self) -> Option<StaticRelationKind> {
        self.relation
    }
    pub const fn role(&self) -> Option<SourceRole> {
        self.role
    }
    pub const fn trust(&self) -> Option<SourceTrust> {
        self.trust
    }
    pub const fn before_references(&self) -> Option<u32> {
        self.before_references
    }
    pub const fn after_references(&self) -> Option<u32> {
        self.after_references
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DependencyCoverage {
    resolved_internal_uses: u32,
    unresolved_internal_uses: u32,
    ambiguous_internal_uses: u32,
    external_uses: u32,
    unresolved_package_uses: u32,
    module_ownership_relations: u32,
    context_relations: u32,
}

impl DependencyCoverage {
    pub const fn new(
        resolved_internal_uses: u32,
        unresolved_internal_uses: u32,
        ambiguous_internal_uses: u32,
        external_uses: u32,
        unresolved_package_uses: u32,
        module_ownership_relations: u32,
        context_relations: u32,
    ) -> Self {
        Self {
            resolved_internal_uses,
            unresolved_internal_uses,
            ambiguous_internal_uses,
            external_uses,
            unresolved_package_uses,
            module_ownership_relations,
            context_relations,
        }
    }
    pub const fn resolved_internal_uses(self) -> u32 {
        self.resolved_internal_uses
    }
    pub const fn unresolved_internal_uses(self) -> u32 {
        self.unresolved_internal_uses
    }
    pub const fn ambiguous_internal_uses(self) -> u32 {
        self.ambiguous_internal_uses
    }
    pub const fn external_uses(self) -> u32 {
        self.external_uses
    }
    pub const fn unresolved_package_uses(self) -> u32 {
        self.unresolved_package_uses
    }
    pub const fn module_ownership_relations(self) -> u32 {
        self.module_ownership_relations
    }
    pub const fn context_relations(self) -> u32 {
        self.context_relations
    }
    pub const fn internal(self) -> u32 {
        self.resolved_internal_uses + self.module_ownership_relations
    }
    pub const fn external(self) -> u32 {
        self.external_uses
    }
    pub const fn unresolved(self) -> u32 {
        self.unresolved_internal_uses + self.unresolved_package_uses
    }
    pub const fn ambiguous(self) -> u32 {
        self.ambiguous_internal_uses
    }
    pub const fn total(self) -> u32 {
        self.resolved_internal_uses
            + self.unresolved_internal_uses
            + self.ambiguous_internal_uses
            + self.external_uses
            + self.unresolved_package_uses
            + self.module_ownership_relations
            + self.context_relations
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
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
    relation: StaticRelationKind,
    role: SourceRole,
    trust: SourceTrust,
    references: u32,
    locations: Vec<SourceSpan>,
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
            relation: StaticRelationKind::Uses,
            role: SourceRole::Primary,
            trust: SourceTrust::Trusted,
            references: 1,
            locations: vec![span],
        }
    }
    pub fn with_evidence(
        mut self,
        relation: StaticRelationKind,
        role: SourceRole,
        trust: SourceTrust,
    ) -> Self {
        self.relation = relation;
        self.role = role;
        self.trust = trust;
        self
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
    pub const fn relation(&self) -> StaticRelationKind {
        self.relation
    }
    pub const fn role(&self) -> SourceRole {
        self.role
    }
    pub const fn trust(&self) -> SourceTrust {
        self.trust
    }
    pub const fn references(&self) -> u32 {
        self.references
    }
    pub fn locations(&self) -> &[SourceSpan] {
        &self.locations
    }
    pub fn with_occurrences(mut self, references: u32, locations: Vec<SourceSpan>) -> Self {
        self.references = references;
        self.locations = locations;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalDependency {
    file: FileId,
    target: String,
    references: u32,
    locations: Vec<SourceSpan>,
    relation: StaticRelationKind,
    role: SourceRole,
    trust: SourceTrust,
}

impl ExternalDependency {
    pub fn new(file: FileId, target: impl Into<String>, references: u32) -> Self {
        Self {
            file,
            target: target.into(),
            references,
            locations: Vec::new(),
            relation: StaticRelationKind::Uses,
            role: SourceRole::Primary,
            trust: SourceTrust::Trusted,
        }
    }
    pub fn with_evidence(
        mut self,
        locations: Vec<SourceSpan>,
        relation: StaticRelationKind,
        role: SourceRole,
        trust: SourceTrust,
    ) -> Self {
        self.locations = locations;
        self.relation = relation;
        self.role = role;
        self.trust = trust;
        self
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
    pub fn locations(&self) -> &[SourceSpan] {
        &self.locations
    }
    pub const fn relation(&self) -> StaticRelationKind {
        self.relation
    }
    pub const fn role(&self) -> SourceRole {
        self.role
    }
    pub const fn trust(&self) -> SourceTrust {
        self.trust
    }
}

#[cfg(test)]
mod tests {
    use super::DependencyCoverage;

    #[test]
    fn dependency_evidence_partitions_have_one_exact_total() {
        let coverage = DependencyCoverage::new(1, 2, 3, 4, 5, 6, 7);

        assert_eq!(coverage.resolved_internal_uses(), 1);
        assert_eq!(coverage.unresolved_internal_uses(), 2);
        assert_eq!(coverage.ambiguous_internal_uses(), 3);
        assert_eq!(coverage.external_uses(), 4);
        assert_eq!(coverage.unresolved_package_uses(), 5);
        assert_eq!(coverage.module_ownership_relations(), 6);
        assert_eq!(coverage.context_relations(), 7);
        assert_eq!(coverage.total(), 28);
    }
}
