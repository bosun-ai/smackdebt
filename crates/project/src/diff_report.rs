//! Writing one diff into the report builder: both sides' records, the
//! architecture and history facts, and the scope links that restate them.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use smackdebt_analysis::{
    ArchitectureComparison, ArchitectureGraph, ArchitectureReportFacts, Coverage, Diagnostic,
    DiagnosticId, DiagnosticKind, DirectoryTree, FileId, FileRecord, HealthCounts, HealthPolicy,
    PackageContainment, PackageId, PackageRecord, ParseStatus,
    ReportBuilder as AnalysisReportBuilder, ReportMode, Scope, ScopeId, SourceRole,
};
use smackdebt_git::GitRepository;

use crate::dependencies::{DiffTables, SourceDependencies};
use crate::diff_changes::DiffPackages;
use crate::diff_findings::{DiffIndexes, DiffSpot, add_diff_result};
use crate::diff_graphs::{ArchitectureLinks, DiffArchitectures};
use crate::diff_impact::{DiffImpact, ImpactComparisons};
use crate::diff_source::{DiffResult, DiffSide, DiffUnchanged};
use crate::history_stream::{ChangeCommits, history_directory_paths, load_evolution};
use crate::paths::{package_of, report_package_path};
use crate::rating::{FileResult, file_result_role};
use crate::requests::DiffRequest;

/// Where a diff's files sit: the shared package positions, the scope every
/// path was given, and the paths the request selected.
#[derive(Clone, Copy)]
pub(crate) struct DiffPlacement<'a> {
    pub(crate) packages: &'a DiffPackages,
    pub(crate) file_scopes: &'a BTreeMap<PathBuf, ScopeId>,
    pub(crate) selected_paths: &'a BTreeSet<PathBuf>,
}
impl DiffPlacement<'_> {
    pub(crate) fn scope_of(&self, path: &Path) -> ScopeId {
        self.file_scopes
            .get(path)
            .copied()
            .expect("diff hierarchy contains every analyzable file")
    }
}
/// Adds every changed file's records and comparisons to the report.
pub(crate) fn record_changed_files(
    builder: &mut AnalysisReportBuilder,
    results: Vec<DiffResult>,
    placement: &DiffPlacement<'_>,
    tables: &mut DiffTables,
    policy: HealthPolicy,
) {
    let mut indexes = DiffIndexes::default();
    for result in results {
        let file_id = FileId::from_index(result.index);
        tables
            .current
            .dependencies
            .extend(changed_side_dependencies(
                file_id,
                result.change.current_path(),
                &result.current,
            ));
        tables.before.dependencies.extend(changed_side_dependencies(
            file_id,
            result.change.base_path(),
            &result.before,
        ));
        let is_selected = placement
            .selected_paths
            .contains(result.change.current_path());
        let scope_id = placement.scope_of(result.change.current_path());
        let package = package_of(
            result.change.current_path(),
            &placement.packages.roots,
            &placement.packages.roots,
        );
        let before_package = result.change.base_exists().then(|| {
            package_of(
                result.change.base_path(),
                &placement.packages.before_roots,
                &placement.packages.roots,
            )
        });
        tables.current.files.push(diff_side_file_record(
            file_id,
            scope_id,
            result.change.current_path(),
            result.change.current_exists().then_some(package),
            &result.current,
        ));
        tables.before.files.push(diff_side_file_record(
            file_id,
            scope_id,
            result.change.base_path(),
            before_package,
            &result.before,
        ));
        add_diff_result(
            builder,
            result,
            &mut indexes,
            DiffSpot {
                scope: scope_id,
                package,
                selected: is_selected,
            },
            policy,
        );
    }
}
/// Adds every unchanged file's records for both sides.
pub(crate) fn record_unchanged_files(
    builder: &mut AnalysisReportBuilder,
    unchanged: DiffUnchanged<'_>,
    placement: &DiffPlacement<'_>,
    tables: &mut DiffTables,
) {
    for (offset, (file, result)) in unchanged
        .candidates
        .iter()
        .zip(unchanged.results)
        .enumerate()
    {
        let file_id = FileId::from_index(unchanged.first_file_index + offset);
        let package = package_of(
            file.path().as_path(),
            &placement.packages.roots,
            &placement.packages.roots,
        );
        let package_scope = placement.scope_of(file.path().as_path());
        if let Some(dependencies) = unchanged_dependencies(file_id, file.path().as_path(), &result)
        {
            tables.before.dependencies.push(SourceDependencies {
                role: unchanged.before_roles[offset],
                ..dependencies.clone()
            });
            tables.current.dependencies.push(dependencies);
        }
        let record = unchanged_side_record(
            unchanged_base_record(file_id, package_scope, file.path().to_string(), package),
            &result,
            file_result_role(&result),
        );
        tables.current.files.push(record.clone());
        builder.add_file(record);
        let before_package = package_of(
            file.path().as_path(),
            &placement.packages.before_roots,
            &placement.packages.roots,
        );
        tables.before.files.push(unchanged_side_record(
            unchanged_base_record(
                file_id,
                package_scope,
                file.path().to_string(),
                before_package,
            ),
            &result,
            unchanged.before_roles[offset],
        ));
    }
}
/// What the streamed history window states about the diff.
pub(crate) struct DiffEvolution {
    pub(crate) facts: smackdebt_analysis::EvolutionaryReportFacts,
    pub(crate) diagnostic: Option<String>,
    pub(crate) explanation_pairs: BTreeSet<(PackageId, PackageId)>,
}
/// The history rows every scope restates, taken before the facts are handed
/// on.
pub(crate) struct EvolutionLinks {
    pub(crate) findings: Vec<smackdebt_analysis::EvolutionaryFinding>,
    pub(crate) comparisons: Vec<smackdebt_analysis::EvolutionaryComparison>,
    pub(crate) concentration: Vec<smackdebt_analysis::ConcentrationComparison>,
    pub(crate) suppressions: Vec<smackdebt_analysis::HistoryComparisonSuppression>,
}
/// Streams the history window and settles the evolutionary facts a diff
/// states.
/// The history one diff reads: the repository holding it and the base the
/// change is measured from.
#[derive(Clone, Copy)]
pub(crate) struct DiffHistorySource<'a> {
    pub(crate) repository: &'a GitRepository,
    pub(crate) base: &'a str,
}

pub(crate) fn stream_diff_evolution(
    request: &DiffRequest,
    source: DiffHistorySource<'_>,
    builder: &AnalysisReportBuilder,
    packages: &DiffPackages,
    architectures: &DiffArchitectures,
) -> (DiffEvolution, EvolutionLinks) {
    let repository = source.repository;
    let history_files = builder
        .files()
        .iter()
        .filter_map(|file| {
            file.package().map(|package| {
                (
                    PathBuf::from(file.path()),
                    file.id(),
                    package,
                    file.role(),
                    file.trust(),
                )
            })
        })
        .collect::<Vec<_>>();
    // The diff flow assembles its history files from a filtered list, so its
    // tree is placed by file identity rather than built from a candidate walk.
    // A diff states no amplification, so nothing outside pair distances reads
    // it and it stays local to this call.
    let directories = DirectoryTree::from_file_paths(history_directory_paths(&history_files));
    // The change under review is the commits this worktree holds and the base
    // does not, so the same stream states both what the packages look like now
    // and what they looked like before the change.
    let made_by_change = repository.commits_ahead(source.base).unwrap_or_default();
    let history = load_evolution(
        repository.root(),
        request.history_days,
        &history_files,
        &directories,
        ChangeCommits::of(&made_by_change),
    );
    let diagnostic = history.diagnostic.clone();
    let current_explanation_pairs = architectures.current.explanation_pairs.clone();
    let before_explanation_pairs = architectures.before.explanation_pairs.clone();
    let containment = PackageContainment::from_paths(
        &packages
            .roots
            .iter()
            .map(|root| report_package_path(root))
            .collect::<Vec<_>>(),
    );
    // A diff answers about a change rather than about a tree, so the
    // amplification the same stream accumulated is dropped here rather than
    // joined onto a scope.
    let (facts, _) = history.evolution.accumulator.finish(
        history.evolution.coverage,
        builder.files().len(),
        packages.roots.len(),
        &containment,
        &current_explanation_pairs,
        Some(&before_explanation_pairs),
    );
    let links = EvolutionLinks {
        findings: facts.findings().to_vec(),
        comparisons: facts.comparisons().to_vec(),
        concentration: facts.concentration_comparisons().to_vec(),
        suppressions: facts.comparison_suppressions().to_vec(),
    };
    (
        DiffEvolution {
            facts,
            diagnostic,
            explanation_pairs: current_explanation_pairs,
        },
        links,
    )
}
/// Hands the current tree's architecture facts and both sides' evidence to
/// the report, returning the impact comparisons the scope links restate.
pub(crate) fn set_diff_architecture_facts(
    builder: &mut AnalysisReportBuilder,
    architectures: DiffArchitectures,
    comparisons: Vec<ArchitectureComparison>,
    impact: DiffImpact,
) -> ImpactComparisons {
    let DiffArchitectures { current, before } = architectures;
    let current_graph_evidence =
        current
            .graph_evidence
            .clone()
            .with_suppressed(0, 0, impact.suppressed_leakage);
    let current_closures = current.package_closures.clone();
    let current_file_reach = current.file_reach.clone();
    let current_core_size = current.core_size;
    let current_core_members = if current.core_size.is_some() {
        current.core_members.clone()
    } else {
        Vec::new()
    };
    builder.set_architecture(ArchitectureReportFacts::new(
        ArchitectureGraph::new(
            current.coverage,
            current.file_edges,
            current.package_edges,
            current.external,
            current.diagnostics,
            current.measurements,
        ),
        current.findings,
        comparisons,
    ));
    builder.set_graph_evidence(current_graph_evidence);
    builder.set_diff_graph_evidence(smackdebt_analysis::DiffGraphEvidence::new(
        current.graph_evidence.clone(),
        before.graph_evidence.clone(),
        impact.propagation_suppression,
        impact.core_suppression,
        impact.leakage_suppression,
    ));
    builder.set_propagation(
        current_closures,
        current_file_reach,
        current_core_size,
        current_core_members,
    );
    builder.set_change_leakage_findings(impact.leakage_findings);
    builder.set_impact_comparisons(
        impact.comparisons.propagation.clone(),
        impact.comparisons.core.clone(),
        impact.comparisons.leakage.clone(),
    );
    impact.comparisons
}
/// Hands the comparison reference and the streamed history facts to the
/// report.
pub(crate) fn set_diff_history_facts(
    builder: &mut AnalysisReportBuilder,
    evolution: DiffEvolution,
    reference: String,
) {
    builder.set_comparison_ref(reference);
    builder.set_evolution(evolution.facts);
    builder.set_explanation_pairs(evolution.explanation_pairs);
    if let Some(message) = evolution.diagnostic {
        let id = DiagnosticId::from_index(builder.diagnostic_count());
        builder.add_diagnostic(Diagnostic::new(id, None, DiagnosticKind::Other, message, 0));
    }
}
/// Links every history row to the root and the package pair it names.
pub(crate) fn link_evolution_scopes(
    builder: &mut AnalysisReportBuilder,
    root: ScopeId,
    packages: &[PackageRecord],
    links: &EvolutionLinks,
) {
    for finding in &links.findings {
        let pair = finding.coupling();
        builder.link_evolutionary_finding(root, finding.id());
        builder.link_evolutionary_finding(packages[pair.left().index()].scope(), finding.id());
        builder.link_evolutionary_finding(packages[pair.right().index()].scope(), finding.id());
    }
    for comparison in &links.comparisons {
        builder.link_evolutionary_comparison(root, comparison.id());
        let pair = comparison.coupling();
        builder
            .link_evolutionary_comparison(packages[pair.left().index()].scope(), comparison.id());
        builder
            .link_evolutionary_comparison(packages[pair.right().index()].scope(), comparison.id());
    }
    for comparison in &links.concentration {
        let package = comparison.concentration().package();
        builder.link_concentration_comparison(root, comparison.id());
        builder.link_concentration_comparison(packages[package.index()].scope(), comparison.id());
    }
    for suppression in &links.suppressions {
        builder.link_history_comparison_suppression(root, suppression.id());
        builder.link_history_comparison_suppression(
            packages[suppression.left().index()].scope(),
            suppression.id(),
        );
        builder.link_history_comparison_suppression(
            packages[suppression.right().index()].scope(),
            suppression.id(),
        );
    }
}
/// Links each propagation comparison to the scopes of its subject.
pub(crate) fn link_propagation_scopes(
    builder: &mut AnalysisReportBuilder,
    root: ScopeId,
    packages: &[PackageRecord],
    comparisons: &[smackdebt_analysis::PropagationComparison],
) {
    for comparison in comparisons {
        builder.link_propagation_comparison(root, comparison.id());
        match comparison.subject() {
            smackdebt_analysis::PropagationSubject::Package { source } => {
                builder
                    .link_propagation_comparison(packages[source.index()].scope(), comparison.id());
            }
            smackdebt_analysis::PropagationSubject::File { package, source } => {
                builder.link_propagation_comparison(
                    packages[package.index()].scope(),
                    comparison.id(),
                );
                if let Some(scope) = builder.files().get(source.index()).map(FileRecord::scope) {
                    builder.link_propagation_comparison(scope, comparison.id());
                }
            }
        }
    }
}
/// Links core and change-leakage comparisons to their file anchors.
pub(crate) fn link_file_comparison_scopes(
    builder: &mut AnalysisReportBuilder,
    root: ScopeId,
    core: &[smackdebt_analysis::CoreComparison],
    leakage: &[smackdebt_analysis::ChangeLeakageComparison],
) {
    for comparison in core {
        builder.link_core_comparison(root, comparison.id());
        if let Some(scope) = builder
            .files()
            .get(comparison.anchor().index())
            .map(FileRecord::scope)
        {
            builder.link_core_comparison(scope, comparison.id());
        }
    }
    for comparison in leakage {
        builder.link_change_leakage_comparison(root, comparison.id());
        for file in [comparison.left(), comparison.right()] {
            if let Some(scope) = builder.files().get(file.index()).map(FileRecord::scope) {
                builder.link_change_leakage_comparison(scope, comparison.id());
            }
        }
    }
}
/// Links architecture rows to the root, their packages, and their files.
pub(crate) fn link_architecture_scopes(
    builder: &mut AnalysisReportBuilder,
    root: ScopeId,
    packages: &[PackageRecord],
    links: &ArchitectureLinks,
) {
    for id in &links.comparison_ids {
        builder.link_architecture_comparison(root, *id);
        let comparison = &links.comparisons[id.index()];
        for package in comparison.packages() {
            builder.link_architecture_comparison(packages[package.index()].scope(), *id);
        }
        for file in comparison.files() {
            let scope = builder.files()[file.index()].scope();
            builder.link_architecture_comparison(scope, *id);
        }
    }
    for id in &links.finding_ids {
        builder.link_architecture_finding(root, *id);
        let finding = &links.findings[id.index()];
        for package in finding.packages() {
            builder.link_architecture_finding(packages[package.index()].scope(), *id);
        }
        for file in finding.files() {
            let scope = builder.files()[file.index()].scope();
            builder.link_architecture_finding(scope, *id);
        }
    }
}
/// The report builder for a diff, with its root and scope hierarchy set.
pub(crate) fn new_diff_builder(scopes: Vec<Scope>, selected_count: usize) -> AnalysisReportBuilder {
    let mut builder = AnalysisReportBuilder::with_capacity(
        ReportMode::Diff,
        selected_count * 2 + 2,
        selected_count,
        0,
        selected_count * 2,
        selected_count,
    );
    for scope in scopes {
        builder.add_scope(scope);
    }
    builder.set_root(ScopeId::from_index(0));
    builder
}
pub(crate) fn diff_side_file_record(
    file: FileId,
    scope: ScopeId,
    path: &Path,
    package: Option<PackageId>,
    side: &DiffSide,
) -> FileRecord {
    let mut record = FileRecord::new(
        file,
        scope,
        path.to_string_lossy(),
        Coverage::default(),
        HealthCounts::default(),
    );
    if let Some(package) = package {
        record = record.with_package(package);
    }
    match side {
        DiffSide::Analyzed { analysis, role, .. } => record
            .with_language(analysis.language())
            .with_source_state(*role, analysis.parse_status().clone()),
        DiffSide::Unsupported { language, role, .. } => record
            .with_language(*language)
            .with_source_state(*role, ParseStatus::Failed),
        DiffSide::Failed { role, .. } => record.with_source_state(*role, ParseStatus::Failed),
        DiffSide::Missing | DiffSide::RoleConflict { .. } => record,
    }
}
/// The dependency row one analyzed side of a changed file states.
pub(crate) fn changed_side_dependencies(
    file: FileId,
    path: &Path,
    side: &DiffSide,
) -> Option<SourceDependencies> {
    let DiffSide::Analyzed { analysis, role, .. } = side else {
        return None;
    };
    Some(SourceDependencies {
        file,
        path: path.to_path_buf(),
        references: analysis.dependencies().to_vec(),
        role: *role,
        trust: analysis.parse_status().trust(),
        language: analysis.language(),
        // A diff never classifies dormant source, so the fact that rule reads
        // is not carried across the object boundary.
        module_syntax: false,
    })
}
/// The plain record either side of an unchanged file starts from.
pub(crate) fn unchanged_base_record(
    file: FileId,
    scope: ScopeId,
    path: String,
    package: PackageId,
) -> FileRecord {
    FileRecord::new(
        file,
        scope,
        path,
        Coverage::default(),
        HealthCounts::default(),
    )
    .with_package(package)
}
/// The record one side of an unchanged file settles on: the shared analysis
/// with the role that side gave the file.
pub(crate) fn unchanged_side_record(
    record: FileRecord,
    result: &FileResult,
    role: SourceRole,
) -> FileRecord {
    match result {
        FileResult::Analyzed(rated) => record
            .with_language(rated.analysis.language())
            .with_source_state(role, rated.analysis.parse_status().clone()),
        FileResult::Unsupported { language, .. } | FileResult::Failed { language, .. } => record
            .with_language(*language)
            .with_source_state(role, ParseStatus::Failed),
        FileResult::RoleConflict { .. } => unreachable!("role conflicts stop composition"),
    }
}
/// The dependency row an unchanged file states when its analysis succeeded.
pub(crate) fn unchanged_dependencies(
    file: FileId,
    path: &Path,
    result: &FileResult,
) -> Option<SourceDependencies> {
    let FileResult::Analyzed(rated) = result else {
        return None;
    };
    Some(SourceDependencies {
        file,
        path: path.to_path_buf(),
        references: rated.analysis.dependencies().to_vec(),
        role: rated.role,
        trust: rated.analysis.parse_status().trust(),
        language: rated.analysis.language(),
        module_syntax: rated.module_syntax,
    })
}
