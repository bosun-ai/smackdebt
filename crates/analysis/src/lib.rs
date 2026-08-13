#![forbid(unsafe_code)]

mod architecture;
mod architecture_comparison;
mod change_coupling;
mod churn;
mod comparison;
mod contributor_concentration;
mod cycle_witness;
mod dependency_degree;
mod evolution;
mod evolutionary_comparison;
mod health;
mod instability;
mod report;
mod source;
mod strongly_connected_components;

pub use architecture::{
    ArchitectureComparison, ArchitectureComparisonId, ArchitectureComparisonKind,
    ArchitectureFinding, ArchitectureFindingId, ArchitectureFindingKind, ArchitectureGraph,
    ArchitectureReportFacts, DependencyCoverage, DependencyEdge, DependencyEdgeId,
    ExternalDependency, Instability, PackageEdge, PackageEdgeId, PackageGraphMeasurement,
    ResolutionDiagnostic, ResolutionIssueKind,
};
pub use architecture_comparison::{PackageCycle, compare_architecture};
pub use change_coupling::{change_coupling, unexplained_coupling};
pub use churn::churn;
pub use contributor_concentration::contributor_concentration;
pub use cycle_witness::cycle_witness;
pub use dependency_degree::dependency_degree;
pub use evolution::{
    ChangeCoupling, ContributorConcentration, ContributorId, EvolutionAccumulator,
    EvolutionaryComparison, EvolutionaryComparisonId, EvolutionaryComparisonKind,
    EvolutionaryFinding, EvolutionaryFindingId, EvolutionaryReportFacts, FileHistory,
    HistoryAvailability, HistoryChangeFact, HistoryCommitFact, HistoryCoverage, PackageHistory,
};
pub use evolutionary_comparison::compare_evolution;
pub use instability::instability;
pub use strongly_connected_components::strongly_connected_components;

pub use comparison::{Comparison, ComparisonDirection, ComparisonKind, compare_units};
pub use health::{
    HealthAssessment, HealthCounts, HealthPolicy, Measurements, Rating, Signal, SignalAssessment,
    Thresholds,
};
pub use report::{
    ComparisonId, Coverage, Diagnostic, DiagnosticId, DiagnosticKind, DiffCounts, FileActivity,
    FileId, FileRecord, Finding, FindingId, PackageId, PathId, Report, ReportBuilder, ReportMode,
    Scope, ScopeId, ScopeKind, aggregate_comparisons, aggregate_scopes,
};
pub use source::{
    DependencyIntent, DependencyKind, DependencySyntax, DependencySyntaxState, FileAnalysis,
    Language, LocalUnitId, ParseStatus, SourceSpan, UnitFact, UnitIdentity, UnitKind,
};
