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
mod history_window;
mod hotspot;
mod instability;
mod orphan;
mod report;
mod size;
mod source;
mod stable_dependencies;
mod strongly_connected_components;
mod verdict;

pub use architecture::{
    ArchitectureComparison, ArchitectureComparisonId, ArchitectureComparisonKind,
    ArchitectureFinding, ArchitectureFindingId, ArchitectureFindingKind, ArchitectureGraph,
    ArchitectureReportFacts, DependencyCoverage, DependencyEdge, DependencyEdgeId,
    ExternalDependency, Instability, PackageEdge, PackageEdgeId, PackageGraphMeasurement,
    ResolutionDiagnostic, ResolutionIssueKind, StableDependencyEvidence, StableDependencyFinding,
    StableDependencyFindingId,
};
pub use architecture_comparison::{PackageCycle, compare_architecture};
pub use change_coupling::{PackageContainment, change_coupling, unexplained_coupling};
pub use churn::churn;
pub use contributor_concentration::{
    MINIMUM_CONCENTRATION_COMMITS, MINIMUM_CONCENTRATION_PERCENT, contributor_concentration,
    knowledge_concentration,
};
pub use cycle_witness::cycle_witness;
pub use dependency_degree::dependency_degree;
pub use evolution::{
    ChangeCoupling, ContributorConcentration, ContributorId, CouplingEvidence,
    EvolutionAccumulator, EvolutionaryComparison, EvolutionaryComparisonId,
    EvolutionaryComparisonKind, EvolutionaryFinding, EvolutionaryFindingId,
    EvolutionaryFindingKind, EvolutionaryReportFacts, FileHistory, HistoryAvailability,
    HistoryChangeFact, HistoryCommitFact, HistoryCoverage, KnowledgeConcentrationFinding,
    KnowledgeConcentrationFindingId, PackageHistory,
};
pub use evolutionary_comparison::compare_evolution;
pub use history_window::HistoryWindow;
pub use hotspot::{DEFAULT_MINIMUM_TOUCHES, FileDebt, Hotspot, HotspotPolicy};
pub use instability::instability;
pub use orphan::{ENTRY_FILENAMES, OrphanCandidate, OrphanFile, is_entry_filename, orphan_files};
pub use size::{SizeFinding, SizePolicy, SizeSubject};
pub use stable_dependencies::{MINIMUM_STABLE_DEPENDENCY_REFERENCES, stable_dependency_findings};
pub use strongly_connected_components::strongly_connected_components;

pub use comparison::{Comparison, ComparisonDirection, ComparisonKind, compare_units};
pub use health::{
    HealthAssessment, HealthCounts, HealthPolicy, Measurements, Rating, Signal, SignalAssessment,
    Thresholds,
};
pub use report::{
    ComparisonId, Coverage, Diagnostic, DiagnosticId, DiagnosticKind, DiffCounts, FileActivity,
    FileId, FileRecord, Finding, FindingId, FindingRank, PackageId, PackagePresence, PackageRecord,
    PathId, Report, ReportBuilder, ReportMode, Scope, ScopeId, ScopeKind, SourceCoverageOutcome,
    aggregate_comparisons, aggregate_scopes,
};
pub use source::{
    CRATE_ROOT_CANDIDATE, DECLARING_FILE_CANDIDATE, DependencyIntent, DependencyKind,
    DependencySyntax, DependencySyntaxState, FileAnalysis, Language, LocalUnitId, ParseStatus,
    SourceRole, SourceSpan, SourceTrust, StaticRelationKind, UnitFact, UnitIdentity, UnitKind,
    is_symbolic_candidate,
};
pub use verdict::{
    CodebaseTier, DebtDiffFacts, DebtDiffSelection, DebtFamily, DiffTier, FIGHTS_BACK_PERMILLE,
    Verdict, VerdictCounts, WORN_PERMILLE, WorstOffender, WorstOffenderReason,
};
