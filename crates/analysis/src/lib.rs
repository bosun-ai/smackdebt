#![forbid(unsafe_code)]

mod architecture;
mod architecture_comparison;
mod change_coupling;
mod churn;
mod comparison;
mod contributor_concentration;
mod cycle_witness;
mod dependency_degree;
mod directory_tree;
mod evolution;
mod evolutionary_comparison;
mod file_change_coupling;
mod file_reach;
mod gate;
mod health;
mod history_window;
mod hotspot;
mod instability;
mod median;
mod orphan;
mod path_probe;
mod problem;
mod propagation;
mod reachability;
mod report;
mod size;
mod source;
mod stable_dependencies;
mod strongly_connected_components;
mod test_scope;
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
pub use change_coupling::{
    PackageContainment, change_coupling, qualifies_for_finding, unexplained_coupling,
};
pub use churn::churn;
pub use contributor_concentration::{
    MINIMUM_CONCENTRATION_COMMITS, MINIMUM_CONCENTRATION_PERCENT, contributor_concentration,
    knowledge_concentration,
};
pub use cycle_witness::cycle_witness;
pub use dependency_degree::dependency_degree;
pub use directory_tree::{DirectoryId, DirectoryTree};
pub use evolution::{
    ChangeCoupling, ContributorConcentration, ContributorId, CouplingEvidence, CouplingLink,
    EvolutionAccumulator, EvolutionaryComparison, EvolutionaryComparisonId,
    EvolutionaryComparisonKind, EvolutionaryFinding, EvolutionaryFindingId,
    EvolutionaryFindingKind, EvolutionaryReportFacts, FileChangeCoupling, FileHistory,
    HistoryAvailability, HistoryChangeFact, HistoryCommitFact, HistoryCoverage,
    KnowledgeConcentrationFinding, KnowledgeConcentrationFindingId, PackageHistory,
};
pub use evolutionary_comparison::compare_evolution;
pub use file_change_coupling::{
    BULK_COMMIT_FILES, RETAINED_FILE_PAIR_LIMIT, RETAINED_FILE_PAIR_SHARED_COMMITS,
    RETAINED_FILE_PAIR_SIMILARITY_PERMILLE,
};
pub use file_reach::{FileReach, REACH_CANDIDATE_LIMIT, file_reaches};
pub use gate::{GateComparison, GateDelta, GateRow, GateSignal, GateSnapshot};
pub use history_window::HistoryWindow;
pub use hotspot::{DEFAULT_MINIMUM_TOUCHES, FileDebt, Hotspot, HotspotPolicy};
pub use instability::instability;
pub use orphan::{ENTRY_FILENAMES, OrphanCandidate, OrphanFile, is_entry_filename, orphan_files};
pub use path_probe::{PathProbe, ReachAnswer};
pub use problem::{
    BROAD_DEBT_UNITS, CONCENTRATED_HIGH_FINDINGS, ClaimedFinding, GOD_FILE_FAN_OUT, HUB_DEGREE,
    HUB_MEDIAN_MULTIPLE, ProblemAnchor, ProblemCard, ProblemEvidence, ProblemId, ProblemInput,
    ProblemPattern, ProblemPolicy, ProblemRank, ProblemVisibility, cluster_problems,
    duplicate_claim,
};
pub use propagation::{
    CLOSURE_NODE_LIMIT, CORE_SIZE_FILES, CORE_SIZE_PERCENT, PACKAGE_REACH_FILES, PackageClosure,
    PackageClosures, ROOT_REACH_PACKAGES, ROOT_REACH_REACHED, close_over_packages,
    enters_file_graph, graph_file_count,
};
pub use reachability::{largest_component_size, reach_in_counts};
pub use size::{SizeFinding, SizeFindingId, SizePolicy, SizeSubject};
pub use stable_dependencies::{MINIMUM_STABLE_DEPENDENCY_REFERENCES, stable_dependency_findings};
pub use strongly_connected_components::strongly_connected_components;
pub use test_scope::{ModuleDeclaration, test_declared_files};

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
    DependencyScope, DependencySyntax, DependencySyntaxState, FileAnalysis, Language, LocalUnitId,
    ParseStatus, SourceRole, SourceSpan, SourceTrust, StaticRelationKind, UnitFact, UnitIdentity,
    UnitKind, is_symbolic_candidate,
};
pub use verdict::{
    CodebaseTier, CoreSize, CoverageQualifier, DENSITY_EVIDENCE_UNITS, DebtDiffFacts,
    DebtDiffSelection, DebtFamily, DiffTier, FIGHTS_BACK_PERMILLE, PropagationReach,
    SMALL_SCOPE_HIGH_UNITS, UNSUPPORTED_QUALIFIER_PERMILLE, VOLUME_FIGHTS_BACK_HIGH,
    VOLUME_LOST_HIGH, Verdict, VerdictCounts, VerdictShare, WORN_PERMILLE, WORST_OFFENDER_LIMIT,
    WorstOffender, WorstOffenderReason,
};
