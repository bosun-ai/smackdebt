//! Pure measurements, debt policy, comparisons, and completed report values.
//!
//! Metric files own their values, calculations, and rules. Graph algorithms,
//! source facts, history composition, and report assembly provide shared support.
//! The crate root exposes the existing interfaces used by workspace consumers.

#![forbid(unsafe_code)]

mod architecture;
mod architecture_comparison;
mod change_amplification;
mod change_impact;
mod change_leakage;
mod code_churn;
mod code_ownership;
mod comparison;
mod connection_graph;
mod cycle_witness;
mod dependency_cycles;
mod dependency_degree;
mod directory_tree;
mod evolution;
mod file_change_coupling;
mod gate;
mod health;
mod history_window;
mod hotspot;
mod instability;
mod measurements;
mod median;
mod orphan_files;
mod package_change_coupling;
mod path_probe;
mod problem;
mod reachability;
mod report;
mod size;
mod source;
mod stable_dependencies;
mod strongly_connected_components;
mod table_index;
mod test_scope;
mod unit_matching;
mod unit_set_matching;
mod verdict;

pub use architecture::{
    ArchitectureGraph, ArchitectureReportFacts, ComparisonSuppression, DependencyCoverage,
    DependencyEdge, DependencyEdgeId, DiffGraphEvidence, ExternalDependency,
    GraphConfigurationFailure, GraphEvidence, PackageEdge, PackageEdgeId, PackageGraphMeasurement,
    ResolutionDiagnostic, ResolutionIssueKind,
};
pub use architecture_comparison::{
    ArchitectureComparison, ArchitectureComparisonId, ArchitectureComparisonKind, PackageCycle,
    compare_architecture,
};
pub use change_amplification::{
    AMPLIFICATION_MAX_FILES, AMPLIFICATION_MIN_COMMITS, AMPLIFICATION_MIN_MEDIAN,
    ChangeAmplification, DirectoryAmplification, scope_amplification,
};
pub use change_impact::{
    CLOSURE_NODE_LIMIT, FileReach, PACKAGE_REACH_FILES, PackageClosure, PackageClosures,
    PackageFileReach, PropagationComparison, PropagationComparisonId, PropagationReach,
    PropagationSubject, REACH_CANDIDATE_LIMIT, ROOT_REACH_PACKAGES, ROOT_REACH_REACHED,
    close_over_packages, enters_file_graph, file_reaches, graph_file_count,
};
pub use change_leakage::{
    ChangeGraph, ChangeLeakageComparison, ChangeLeakageComparisonId, ChangeLeakageFinding,
    ChangeLeakageFindingId, ChangeLeakageKind, LEAKAGE_MIN_DISTANCE, LEAKAGE_SHARED_COMMITS,
    LEAKAGE_SIMILARITY_FLOOR_PERMILLE, LEAKAGE_SIMILARITY_PERMILLE,
    LEAKAGE_SIMILARITY_STEP_PERMILLE, PATH_PROBE_NODES, WIRING_FILENAMES, change_leakage,
    is_wiring_filename, required_permille,
};
pub use code_churn::{FileActivity, FileHistory, PackageHistory, churn};
pub use code_ownership::{
    ConcentrationComparison, ConcentrationComparisonId, ConcentrationComparisonKind,
    ContributorConcentration, KnowledgeConcentrationFinding, KnowledgeConcentrationFindingId,
    MINIMUM_CONCENTRATION_COMMITS, MINIMUM_CONCENTRATION_PERCENT, compare_concentration,
    contributor_concentration, knowledge_concentration,
};
pub use comparison::{Comparison, ComparisonDirection, ComparisonKind, ComparisonParticipation};
pub use connection_graph::{ConnectionGraph, enters_connection_graph};
pub use cycle_witness::cycle_witness;
pub use dependency_cycles::{
    ArchitectureFinding, ArchitectureFindingId, ArchitectureFindingKind, CORE_SIZE_FILES,
    CORE_SIZE_PERCENT, CoreComparison, CoreComparisonId, CoreSize,
};
pub use dependency_degree::dependency_degree;
pub use directory_tree::{DirectoryId, DirectoryTree};
pub use evolution::{
    ContributorId, EvolutionAccumulator, EvolutionaryFindingKind, EvolutionaryReportFacts,
    HistoryAvailability, HistoryChangeCounts, HistoryChangeFact, HistoryCommitFact,
    HistoryComparisonSuppression, HistoryComparisonSuppressionId, HistoryCoverage,
};
pub use file_change_coupling::{
    BULK_COMMIT_FILES, FileChangeCoupling, FileChangeCouplingId, RETAINED_FILE_PAIR_LIMIT,
    RETAINED_FILE_PAIR_SHARED_COMMITS, RETAINED_FILE_PAIR_SIMILARITY_PERMILLE,
};
pub use gate::{GateComparison, GateDelta, GateRow, GateSignal, GateSnapshot};
pub use health::{
    HealthAssessment, HealthCounts, HealthPolicy, Rating, Signal, SignalAssessment, Thresholds,
};
pub use history_window::HistoryWindow;
pub use hotspot::{DEFAULT_MINIMUM_TOUCHES, FileDebt, Hotspot, HotspotPolicy};
pub use instability::{Instability, instability};
pub use measurements::Measurements;
pub use orphan_files::{
    ENTRY_FILENAMES, OrphanCandidate, OrphanFile, is_entry_filename, orphan_files,
};
pub use package_change_coupling::{
    ChangeCoupling, CouplingEvidence, CouplingLink, EvolutionaryComparison,
    EvolutionaryComparisonId, EvolutionaryComparisonKind, EvolutionaryFinding,
    EvolutionaryFindingId, PackageContainment, change_coupling, compare_evolution,
    qualifies_for_finding, unexplained_coupling,
};
pub use path_probe::{PathProbe, ReachAnswer};
pub use problem::{
    BROAD_DEBT_UNITS, CONCENTRATED_HIGH_FINDINGS, ClaimedFinding, GOD_FILE_FAN_OUT, HUB_DEGREE,
    HUB_MEDIAN_MULTIPLE, ProblemAnchor, ProblemCard, ProblemEvidence, ProblemId, ProblemInput,
    ProblemPattern, ProblemPolicy, ProblemRank, ProblemVisibility, cluster_problems,
    duplicate_claim,
};
pub use reachability::{largest_component_size, reach_in_counts};
pub use report::{
    ComparisonId, Coverage, Diagnostic, DiagnosticId, DiagnosticKind, DiffCounts, FileId,
    FileRecord, Finding, FindingId, FindingRank, PackageId, PackagePresence, PackageRecord, PathId,
    Report, ReportBuilder, ReportMode, Scope, ScopeId, ScopeKind, SourceCoverageOutcome,
    SourcePresence, aggregate_comparisons, aggregate_scopes,
};
pub use size::{SizeFinding, SizeFindingId, SizePolicy, SizeSubject};
pub use source::{
    CRATE_ROOT_CANDIDATE, DECLARING_FILE_CANDIDATE, DependencyIntent, DependencyKind,
    DependencyScope, DependencySyntax, DependencySyntaxState, FileAnalysis, Language, LocalUnitId,
    PARENT_MODULE_CANDIDATE, ParseStatus, RecoveredFacts, SourceRole, SourceSpan, SourceTrust,
    StaticRelationKind, UnitFact, UnitIdentity, UnitKind, UnitMatchEvidence, is_symbolic_candidate,
};
pub use stable_dependencies::{
    MINIMUM_STABLE_DEPENDENCY_REFERENCES, StableDependencyEvidence, StableDependencyFinding,
    StableDependencyFindingId, stable_dependency_findings,
};
pub use strongly_connected_components::strongly_connected_components;
pub use test_scope::{ModuleDeclaration, test_declared_files};
pub use unit_matching::compare_units;
pub use unit_set_matching::{UnitDiffSides, compare_unit_sets};
pub use verdict::{
    CodebaseTier, CoverageQualifier, DENSITY_EVIDENCE_UNITS, DebtDiffFacts, DebtDiffSelection,
    DebtFamily, DiffTier, FIGHTS_BACK_PERMILLE, SMALL_SCOPE_HIGH_UNITS, VOLUME_FIGHTS_BACK_HIGH,
    VOLUME_LOST_HIGH, Verdict, VerdictCounts, VerdictShare, WORN_PERMILLE, WORST_OFFENDER_LIMIT,
    WorstOffender, WorstOffenderReason,
};
