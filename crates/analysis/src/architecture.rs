//! Shared dependency graph facts, source evidence, and coverage.
//!
//! Metric policies consume these borrowed graph tables; this module owns their
//! common representation, resolution diagnostics, and report assembly inputs.

use crate::{ArchitectureComparison, ArchitectureFinding, Instability};
use crate::{FileId, PackageId, SourceRole, SourceSpan, SourceTrust, StaticRelationKind};

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

crate::table_index::table_index!(
    /// The position of one dependency edge in its report table.
    DependencyEdgeId
);
crate::table_index::table_index!(
    /// The position of one package edge in its report table.
    PackageEdgeId
);

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
    /// Restates this relation under a role settled after it was resolved.
    ///
    /// The relation itself is untouched; only what it counts as changes, so a
    /// reference written in a file the repository merely carries stops being
    /// evidence about the code that ships.
    #[must_use]
    pub fn in_context_role(mut self, role: SourceRole) -> Self {
        self.role = role;
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
    /// Whether this relation may produce an architecture verdict.
    ///
    /// A verdict describes the structure of the code that ships, so only
    /// trusted parsed `uses` from primary source enters a verdict graph.
    /// Test, example, and benchmark relations stay evidence through
    /// [`Self::affects_verdict`].
    pub const fn enters_verdict_graph(&self) -> bool {
        matches!(self.relation, StaticRelationKind::Uses)
            && matches!(self.role, SourceRole::Primary)
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
    reach_in: u32,
    instability: Option<Instability>,
}

impl PackageGraphMeasurement {
    pub const fn new(package: PackageId, fan_in: u32, fan_out: u32) -> Self {
        Self {
            package,
            fan_in,
            fan_out,
            reach_in: 0,
            instability: Instability::new(fan_out, fan_in + fan_out),
        }
    }

    /// Records how many packages transitively depend on this one, counting
    /// itself, which is the closure the architecture build already computed.
    #[must_use]
    pub const fn with_reach_in(mut self, reach_in: u32) -> Self {
        self.reach_in = reach_in;
        self
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
    /// The packages that transitively depend on this one, counting itself.
    pub const fn reach_in(self) -> u32 {
        self.reach_in
    }
    pub const fn instability(self) -> Option<Instability> {
        self.instability
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

/// Whether dependency-derived human claims have enough source evidence.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GraphEvidence {
    incomplete_packages: Vec<PackageId>,
    parse_failures: u32,
    unresolved_internal: u32,
    ambiguous_internal: u32,
    configuration_failures: Vec<GraphConfigurationFailure>,
    suppressed_reach: u32,
    suppressed_core: u32,
    suppressed_leakage: u32,
}

/// Candidate diff comparisons withheld because one or both graph sides are incomplete.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ComparisonSuppression {
    total: u32,
    current: u32,
    base: u32,
}

impl ComparisonSuppression {
    pub const fn total(self) -> u32 {
        self.total
    }

    pub const fn current(self) -> u32 {
        self.current
    }

    pub const fn base(self) -> u32 {
        self.base
    }

    pub fn record(&mut self, current: bool, base: bool) {
        self.total = self.total.saturating_add(1);
        self.current = self.current.saturating_add(u32::from(current));
        self.base = self.base.saturating_add(u32::from(base));
    }
}

/// The two graph inputs to a diff and the comparisons their evidence withheld.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffGraphEvidence {
    current: GraphEvidence,
    base: GraphEvidence,
    propagation: ComparisonSuppression,
    core: ComparisonSuppression,
    leakage: ComparisonSuppression,
}

impl DiffGraphEvidence {
    pub const fn new(
        current: GraphEvidence,
        base: GraphEvidence,
        propagation: ComparisonSuppression,
        core: ComparisonSuppression,
        leakage: ComparisonSuppression,
    ) -> Self {
        Self {
            current,
            base,
            propagation,
            core,
            leakage,
        }
    }

    pub const fn current(&self) -> &GraphEvidence {
        &self.current
    }

    pub const fn base(&self) -> &GraphEvidence {
        &self.base
    }

    pub const fn propagation(&self) -> ComparisonSuppression {
        self.propagation
    }

    pub const fn core(&self) -> ComparisonSuppression {
        self.core
    }

    pub const fn leakage(&self) -> ComparisonSuppression {
        self.leakage
    }

    pub const fn suppressed_total(&self) -> u32 {
        self.propagation.total + self.core.total + self.leakage.total
    }
}

impl GraphEvidence {
    pub fn new(
        mut incomplete_packages: Vec<PackageId>,
        parse_failures: u32,
        unresolved_internal: u32,
        ambiguous_internal: u32,
        configuration_failures: Vec<GraphConfigurationFailure>,
    ) -> Self {
        incomplete_packages.sort_unstable();
        incomplete_packages.dedup();
        Self {
            incomplete_packages,
            parse_failures,
            unresolved_internal,
            ambiguous_internal,
            configuration_failures,
            suppressed_reach: 0,
            suppressed_core: 0,
            suppressed_leakage: 0,
        }
    }

    pub fn with_suppressed(mut self, reach: u32, core: u32, leakage: u32) -> Self {
        self.suppressed_reach = reach;
        self.suppressed_core = core;
        self.suppressed_leakage = leakage;
        self
    }

    pub fn is_complete(&self) -> bool {
        self.incomplete_packages.is_empty()
    }

    pub fn package_is_complete(&self, package: PackageId) -> bool {
        self.incomplete_packages.binary_search(&package).is_err()
    }

    pub fn incomplete_packages(&self) -> &[PackageId] {
        &self.incomplete_packages
    }
    pub const fn parse_failures(&self) -> u32 {
        self.parse_failures
    }
    pub const fn unresolved_internal(&self) -> u32 {
        self.unresolved_internal
    }
    pub const fn ambiguous_internal(&self) -> u32 {
        self.ambiguous_internal
    }
    pub fn configuration_failures(&self) -> &[GraphConfigurationFailure] {
        &self.configuration_failures
    }
    pub const fn suppressed_reach(&self) -> u32 {
        self.suppressed_reach
    }
    pub const fn suppressed_core(&self) -> u32 {
        self.suppressed_core
    }
    pub const fn suppressed_leakage(&self) -> u32 {
        self.suppressed_leakage
    }
}

/// One package configuration that could not safely provide resolution data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphConfigurationFailure {
    package: PackageId,
    reason: String,
}

impl GraphConfigurationFailure {
    pub fn new(package: PackageId, reason: impl Into<String>) -> Self {
        Self {
            package,
            reason: reason.into(),
        }
    }
    pub const fn package(&self) -> PackageId {
        self.package
    }
    pub fn reason(&self) -> &str {
        &self.reason
    }
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
    /// Relations that are real evidence but never enter the verdict graph:
    /// those read from an untrusted source or a role that does not affect the
    /// verdict, plus every asset reference — an import of a file discovery
    /// never inventories, which no lookup could have matched and which is
    /// therefore not a hole. Assets have no counter of their own; a consumer
    /// that wants them alone counts `resolution_diagnostics` rows whose kind
    /// is `asset`.
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

/// What reading one reference against the repository concluded.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ResolutionIssueKind {
    /// The target is dynamic, or no repository file carries the name.
    Unresolved,
    /// Several repository files carry the name.
    Ambiguous,
    /// The target names a file the source languages never analyze, such as a
    /// YAML fixture or an image imported for its bytes. The reference is
    /// disclosed rather than counted as a hole in the dependency graph,
    /// because no analyzable file was ever there to reach.
    Asset,
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
    /// Restates this diagnostic under a role settled after resolution.
    #[must_use]
    pub fn in_context_role(mut self, role: SourceRole) -> Self {
        self.role = role;
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
    /// Restates this external reference under a role settled after resolution.
    #[must_use]
    pub fn in_context_role(mut self, role: SourceRole) -> Self {
        self.role = role;
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
    use super::{
        DependencyCoverage, DependencyEdge, DependencyEdgeId, FileId, SourceRole, SourceTrust,
        StaticRelationKind,
    };

    fn edge(role: SourceRole, trust: SourceTrust, relation: StaticRelationKind) -> DependencyEdge {
        DependencyEdge::new(
            DependencyEdgeId::from_index(0),
            FileId::from_index(0),
            FileId::from_index(1),
            1,
            Vec::new(),
        )
        .with_relation(relation)
        .with_evidence(role, trust)
    }

    #[test]
    fn only_trusted_primary_uses_enter_a_verdict_graph_while_evidence_stays_wider() {
        let primary = edge(
            SourceRole::Primary,
            SourceTrust::Trusted,
            StaticRelationKind::Uses,
        );
        assert!(primary.enters_verdict_graph());
        assert!(primary.affects_verdict());

        for role in [SourceRole::Test, SourceRole::Example, SourceRole::Benchmark] {
            let value = edge(role, SourceTrust::Trusted, StaticRelationKind::Uses);
            assert!(!value.enters_verdict_graph(), "{role:?} entered a verdict");
            assert!(value.affects_verdict(), "{role:?} stopped being evidence");
        }

        assert!(
            !edge(
                SourceRole::Primary,
                SourceTrust::Advisory,
                StaticRelationKind::Uses
            )
            .enters_verdict_graph()
        );
        assert!(
            !edge(
                SourceRole::Primary,
                SourceTrust::Trusted,
                StaticRelationKind::ModuleOwnership
            )
            .enters_verdict_graph()
        );
    }

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
