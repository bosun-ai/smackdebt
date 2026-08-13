//! Repository use cases. This crate is the only place that composes discovery,
//! parsers, Git, health policy, and parallel execution.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use rayon::prelude::*;
use smackdebt_analysis::{
    ArchitectureFinding, ArchitectureFindingId, ArchitectureFindingKind, ArchitectureGraph,
    ArchitectureReportFacts, Comparison, ComparisonId, ContributorId, Coverage, DependencyCoverage,
    DependencyEdge, DependencyEdgeId, DependencySyntax, DependencySyntaxState, Diagnostic,
    DiagnosticId, DiagnosticKind, EvolutionAccumulator, ExternalDependency, FileActivity,
    FileAnalysis, FileId, FileRecord, Finding, FindingId, HealthAssessment, HealthCounts,
    HealthPolicy, HistoryAvailability, HistoryChangeFact, HistoryCommitFact, HistoryCoverage,
    Language, PackageEdge, PackageEdgeId, PackageGraphMeasurement, PackageId, ParseStatus, Rating,
    Report, ReportBuilder as AnalysisReportBuilder, ReportMode, ResolutionDiagnostic,
    ResolutionIssueKind, Scope, ScopeId, ScopeKind, compare_architecture, compare_units,
    cycle_witness, dependency_degree, strongly_connected_components,
};
use smackdebt_discovery::{DiscoveredFile, Inventory};
use smackdebt_git::{Change, ContributorIdentity, GitRepository};
use smackdebt_languages::{AnalysisError as LanguageError, Analyzer};

use crate::requests::WorkStats;
use crate::requests::{CodebaseRequest, DiffRequest, ExecutionWidth, ProjectError, ProjectReport};

const PARALLEL_FILE_CUTOVER: usize = 100;

/// Analyzes the selected codebase.
pub(super) fn analyze_codebase(request: &CodebaseRequest) -> Result<ProjectReport, ProjectError> {
    let selection = Selection::resolve(&request.path, request.automatic_scope)?;
    let inventory =
        Inventory::discover_sources(&selection.inventory_root, request.excludes.clone()).map_err(
            |source| ProjectError::Inspect {
                path: selection.inventory_root.clone(),
                source,
            },
        )?;
    #[cfg(feature = "evidence-stats")]
    crate::evidence::record_inventory(inventory.visited_entries());
    let aliases = load_resolution_aliases(&selection.inventory_root);
    let candidates: Vec<&DiscoveredFile> = inventory.source_files().collect();
    let work = AnalysisWork::default();
    let analyses = analyze_current_files(
        &inventory,
        &candidates,
        request.width,
        request.policy,
        &work,
    )?;

    let history_files = candidates
        .iter()
        .enumerate()
        .map(|(index, file)| {
            (
                file.path().as_path().to_path_buf(),
                FileId::from_index(index),
                file.package(),
            )
        })
        .collect::<Vec<_>>();
    let history = load_evolution(
        &selection.inventory_root,
        request.history_days,
        &history_files,
    );
    let mut builder = CodebaseReportBuilder::new(
        ReportMode::Codebase,
        selection.label,
        &inventory,
        &candidates,
        &history.activity,
        aliases,
        history.evolution,
    );
    if let Some(message) = history.diagnostic {
        builder.add_general_diagnostic(DiagnosticKind::Other, message);
    }
    for (file_index, analysis) in analyses.into_iter().enumerate() {
        builder.add_analysis(file_index, analysis);
    }
    let report = builder.finish(&work);
    let _inventory_visits = inventory.visited_entries();
    let selected_path = selection
        .exact_file
        .as_deref()
        .or(selection.prefix.as_deref())
        .and_then(Path::to_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(".");
    let selected_scope = if selected_path == "." {
        report.root()
    } else {
        report
            .scopes()
            .iter()
            .find(|scope| scope.name() == selected_path && scope.kind() != ScopeKind::Repository)
            .map(Scope::id)
            .or_else(|| report.root())
    };
    Ok(ProjectReport {
        report,
        selected_scope,
        stats: WorkStats {
            inventory_walks: 1,
            inventory_visits: _inventory_visits,
            source_reads: work.source_reads.load(Ordering::Relaxed),
            git_processes: history.processes,
        },
    })
}

/// Compares changed source units with the selected ref.
pub(super) fn analyze_diff(request: &DiffRequest) -> Result<ProjectReport, ProjectError> {
    let repository = GitRepository::discover(&request.path)?;
    let reference = match &request.reference {
        Some(reference) => reference.clone(),
        None => repository
            .default_ref()?
            .ok_or(ProjectError::MissingReference)?,
    };
    let base = repository.merge_base(&reference, "HEAD")?;
    let mut changed = repository.changes_from(&base)?;
    let inventory =
        Inventory::discover_sources(repository.root(), Vec::new()).map_err(|source| {
            ProjectError::Inspect {
                path: repository.root().to_path_buf(),
                source,
            }
        })?;
    #[cfg(feature = "evidence-stats")]
    crate::evidence::record_inventory(inventory.visited_entries());
    let aliases = load_resolution_aliases(repository.root());

    let path_filter = (!request.automatic_scope)
        .then(|| diff_filter(repository.root(), &request.path))
        .flatten();
    let all_changed = changed.clone();
    changed.retain(|entry| Analyzer::language(entry.current_path()) != Language::Unknown);
    let selected_paths: std::collections::BTreeSet<_> = changed
        .iter()
        .filter(|entry| {
            path_filter
                .as_ref()
                .is_none_or(|path| entry.current_path().starts_with(path))
        })
        .map(|entry| entry.current_path().to_path_buf())
        .collect();
    let selected_count = selected_paths.len();
    let changed_count = changed.len();
    let current_package_roots: Vec<_> = inventory
        .packages()
        .iter()
        .map(|package| package.root().as_path().to_path_buf())
        .collect();
    let work = AnalysisWork::default();
    let width = request.width.threads().min(changed_count.max(1));
    let mut batch = repository.object_reader(width * 2)?;
    let before_aliases = load_base_resolution_aliases(&mut batch, &base);
    let before_package_roots =
        base_package_roots(&current_package_roots, &all_changed, &mut batch, &base);
    let mut package_roots = current_package_roots.clone();
    package_roots.extend(before_package_roots.iter().cloned());
    package_roots.extend(diff_package_roots(repository.root(), &all_changed));
    package_roots.sort();
    package_roots.dedup();
    let mut hierarchy = HierarchyBuilder::new(".".to_owned(), &package_roots);
    let changed_paths_for_hierarchy: std::collections::BTreeSet<_> = changed
        .iter()
        .map(|entry| entry.current_path().to_path_buf())
        .collect();
    for entry in &changed {
        let package_root = nearest_package_root(entry.current_path(), &package_roots);
        let package_index = package_roots
            .iter()
            .position(|root| root == &package_root)
            .unwrap_or(0);
        hierarchy.add_file(entry.current_path(), package_index);
    }
    for file in inventory
        .source_files()
        .filter(|file| Analyzer::language(file.path().as_path()) != Language::Unknown)
    {
        if changed_paths_for_hierarchy.contains(file.path().as_path()) {
            continue;
        }
        let package_root = nearest_package_root(file.path().as_path(), &package_roots);
        let package_index = package_roots
            .iter()
            .position(|root| root == &package_root)
            .unwrap_or(0);
        hierarchy.add_file(file.path().as_path(), package_index);
    }
    let file_scopes = hierarchy.file_scopes.clone();
    let results = analyze_diff_inputs(
        changed,
        repository.root().to_path_buf(),
        base,
        batch,
        request.policy,
        width,
        work.clone(),
    )?;
    let changed_paths: std::collections::BTreeSet<_> = results
        .iter()
        .filter(|result| result.change.current_exists())
        .map(|result| result.change.current_path().to_path_buf())
        .collect();
    let unchanged_candidates: Vec<_> = inventory
        .source_files()
        .filter(|file| {
            !changed_paths.contains(file.path().as_path())
                && Analyzer::language(file.path().as_path()) != Language::Unknown
        })
        .collect();
    let unchanged = analyze_current_files(
        &inventory,
        &unchanged_candidates,
        request.width,
        request.policy,
        &work,
    )?;
    let root = ScopeId::from_index(0);
    let mut builder = AnalysisReportBuilder::with_capacity(
        ReportMode::Diff,
        selected_count * 2 + 2,
        selected_count,
        0,
        selected_count * 2,
        selected_count,
    );
    for scope in hierarchy.scopes {
        builder.add_scope(scope);
    }
    builder.set_root(root);
    let mut comparison_index = 0usize;
    let mut current_dependencies = Vec::new();
    let mut before_dependencies = Vec::new();
    for result in results {
        let file_id = FileId::from_index(result.index);
        if let DiffSide::Analyzed { analysis, .. } = &result.current {
            current_dependencies.push((
                file_id,
                result.change.current_path().to_path_buf(),
                analysis.dependencies().to_vec(),
            ));
        }
        if let DiffSide::Analyzed { analysis, .. } = &result.before {
            before_dependencies.push((
                file_id,
                result.change.base_path().to_path_buf(),
                analysis.dependencies().to_vec(),
            ));
        }
        let is_selected = selected_paths.contains(result.change.current_path());
        let scope_id = file_scopes
            .get(result.change.current_path())
            .copied()
            .expect("diff hierarchy contains every changed file");
        let package_root = nearest_package_root(result.change.current_path(), &package_roots);
        let package = PackageId::from_index(
            package_roots
                .iter()
                .position(|root| root == &package_root)
                .unwrap_or(0),
        );
        add_diff_result(
            &mut builder,
            result,
            &mut comparison_index,
            scope_id,
            package,
            is_selected,
        );
    }
    for (offset, (file, result)) in unchanged_candidates.iter().zip(unchanged).enumerate() {
        let file_id = FileId::from_index(changed_count + offset);
        let package_root = nearest_package_root(file.path().as_path(), &package_roots);
        let package_index = package_roots
            .iter()
            .position(|root| root == &package_root)
            .unwrap_or(0);
        let package = PackageId::from_index(package_index);
        let package_scope = file_scopes[file.path().as_path()];
        let mut record = FileRecord::new(
            file_id,
            package_scope,
            file.path().to_string(),
            Coverage::default(),
            HealthCounts::default(),
        )
        .with_package(package);
        match result {
            FileResult::Analyzed(rated) => {
                record = record.with_language(rated.analysis.language());
                let dependencies = rated.analysis.dependencies().to_vec();
                current_dependencies.push((
                    file_id,
                    file.path().as_path().to_path_buf(),
                    dependencies.clone(),
                ));
                before_dependencies.push((
                    file_id,
                    file.path().as_path().to_path_buf(),
                    dependencies,
                ));
            }
            FileResult::Unsupported(language) => record = record.with_language(language),
            FileResult::Failed(_) => {}
        }
        builder.add_file(record);
    }
    let current_architecture = build_architecture(
        &work,
        builder.files(),
        &current_dependencies,
        &aliases,
        &current_package_roots,
        &package_roots,
    );
    let before_architecture = build_architecture(
        &work,
        builder.files(),
        &before_dependencies,
        &before_aliases,
        &before_package_roots,
        &package_roots,
    );
    let before_edges: Vec<_> = before_architecture
        .package_edges
        .iter()
        .map(|edge| (edge.source(), edge.target()))
        .collect();
    let current_edges: Vec<_> = current_architecture
        .package_edges
        .iter()
        .map(|edge| (edge.source(), edge.target()))
        .collect();
    work.record_algorithm_pass();
    let mut architecture_comparisons = compare_architecture(
        &before_edges,
        &current_edges,
        &before_architecture.cycles,
        &current_architecture.cycles,
    );
    for comparison in &mut architecture_comparisons {
        let source = if comparison.kind()
            == smackdebt_analysis::ArchitectureComparisonKind::CycleRemoved
            || comparison.kind() == smackdebt_analysis::ArchitectureComparisonKind::EdgeRemoved
        {
            &before_architecture
        } else {
            &current_architecture
        };
        let files: std::collections::BTreeSet<_> = match comparison.kind() {
            smackdebt_analysis::ArchitectureComparisonKind::EdgeAdded
            | smackdebt_analysis::ArchitectureComparisonKind::EdgeRemoved => source
                .package_edges
                .iter()
                .filter(|edge| comparison.packages() == [edge.source(), edge.target()])
                .flat_map(|edge| {
                    edge.file_edges().iter().flat_map(|id| {
                        let edge = &source.file_edges[id.index()];
                        [edge.source(), edge.target()]
                    })
                })
                .collect(),
            _ => source
                .findings
                .iter()
                .filter(|finding| {
                    finding.kind() == ArchitectureFindingKind::PackageCycle
                        && finding
                            .packages()
                            .iter()
                            .any(|package| comparison.packages().contains(package))
                })
                .flat_map(|finding| finding.files().iter().copied())
                .collect(),
        };
        *comparison = comparison.clone().with_files(files.into_iter().collect());
    }
    let architecture_comparison_ids: Vec<_> = architecture_comparisons
        .iter()
        .map(|comparison| comparison.id())
        .collect();
    let architecture_comparisons_for_links = architecture_comparisons.clone();
    let architecture_finding_ids: Vec<_> = current_architecture
        .findings
        .iter()
        .map(|finding| finding.id())
        .collect();
    let architecture_findings_for_links = current_architecture.findings.clone();
    let history_files = builder
        .files()
        .iter()
        .filter_map(|file| {
            file.package()
                .map(|package| (PathBuf::from(file.path()), file.id(), package))
        })
        .collect::<Vec<_>>();
    let history = load_evolution(repository.root(), request.history_days, &history_files);
    let history_diagnostic = history.diagnostic.clone();
    let current_package_edges = current_architecture.package_edges.clone();
    let before_package_edges = before_architecture.package_edges.clone();
    work.record_algorithm_pass();
    let evolution = history.evolution.accumulator.finish(
        history.evolution.coverage,
        builder.files().len(),
        package_roots.len(),
        &current_package_edges,
        Some(&before_package_edges),
    );
    let evolutionary_findings = evolution.findings().to_vec();
    let evolutionary_comparisons = evolution.comparisons().to_vec();
    builder.set_architecture(ArchitectureReportFacts::new(
        ArchitectureGraph::new(
            current_architecture.coverage,
            current_architecture.file_edges,
            current_architecture.package_edges,
            current_architecture.external,
            current_architecture.diagnostics,
            current_architecture.measurements,
        ),
        current_architecture.findings,
        architecture_comparisons,
    ));
    builder.set_evolution(evolution);
    if let Some(message) = history_diagnostic {
        let id = DiagnosticId::from_index(builder.diagnostic_count());
        builder.add_diagnostic(Diagnostic::new(id, None, DiagnosticKind::Other, message, 0));
    }
    for finding in evolutionary_findings {
        builder.link_evolutionary_finding(root, finding.id());
        let pair = finding.coupling();
        builder
            .link_evolutionary_finding(ScopeId::from_index(1 + pair.left().index()), finding.id());
        builder
            .link_evolutionary_finding(ScopeId::from_index(1 + pair.right().index()), finding.id());
    }
    for comparison in evolutionary_comparisons {
        builder.link_evolutionary_comparison(root, comparison.id());
        let pair = comparison.coupling();
        builder.link_evolutionary_comparison(
            ScopeId::from_index(1 + pair.left().index()),
            comparison.id(),
        );
        builder.link_evolutionary_comparison(
            ScopeId::from_index(1 + pair.right().index()),
            comparison.id(),
        );
    }
    for id in architecture_comparison_ids {
        builder.link_architecture_comparison(root, id);
        let comparison = &architecture_comparisons_for_links[id.index()];
        for package in comparison.packages() {
            builder.link_architecture_comparison(ScopeId::from_index(1 + package.index()), id);
        }
        for file in comparison.files() {
            let scope = builder.files()[file.index()].scope();
            builder.link_architecture_comparison(scope, id);
        }
    }
    for id in architecture_finding_ids {
        builder.link_architecture_finding(root, id);
        let finding = &architecture_findings_for_links[id.index()];
        for package in finding.packages() {
            builder.link_architecture_finding(ScopeId::from_index(1 + package.index()), id);
        }
        for file in finding.files() {
            let scope = builder.files()[file.index()].scope();
            builder.link_architecture_finding(scope, id);
        }
    }
    let report = builder.finish();
    let selected_scope = path_filter
        .as_ref()
        .and_then(|path| {
            let name = if path.as_os_str().is_empty() {
                ".".to_owned()
            } else {
                path.display().to_string()
            };
            report
                .scopes()
                .iter()
                .find(|scope| scope.name() == name)
                .map(Scope::id)
        })
        .or(Some(root));
    Ok(ProjectReport {
        report,
        selected_scope,
        stats: WorkStats {
            inventory_walks: 1,
            inventory_visits: inventory.visited_entries(),
            source_reads: work.source_reads.load(Ordering::Relaxed),
            git_processes: repository.git_processes(),
        },
    })
}

struct InputSide {
    bytes: Option<Vec<u8>>,
    error: Option<String>,
}

impl InputSide {
    fn missing() -> Self {
        Self {
            bytes: None,
            error: None,
        }
    }

    fn bytes(bytes: Vec<u8>) -> Self {
        Self {
            bytes: Some(bytes),
            error: None,
        }
    }

    fn failed(error: String) -> Self {
        Self {
            bytes: None,
            error: Some(error),
        }
    }
}

struct DiffInput {
    index: usize,
    change: Change,
    current: InputSide,
    before: InputSide,
}

#[derive(Clone, Debug, Default)]
struct AnalysisWork {
    source_reads: Arc<AtomicUsize>,
}

impl AnalysisWork {
    #[cfg(feature = "evidence-stats")]
    fn record_algorithm_pass(&self) {
        crate::evidence::record_algorithm_pass();
    }

    #[cfg(not(feature = "evidence-stats"))]
    fn record_algorithm_pass(&self) {}

    #[cfg(feature = "evidence-stats")]
    fn record_parser_visit(&self) {
        crate::evidence::record_parser_visit();
    }

    #[cfg(not(feature = "evidence-stats"))]
    fn record_parser_visit(&self) {}
}

struct DiffResult {
    index: usize,
    change: Change,
    current: DiffSide,
    before: DiffSide,
    comparisons: Vec<Comparison>,
}

enum DiffSide {
    Missing,
    Analyzed {
        analysis: FileAnalysis,
        health: HealthCounts,
    },
    Unsupported(Language),
    Failed(String),
}

fn analyze_diff_inputs(
    changes: Vec<Change>,
    root_path: PathBuf,
    base: String,
    mut batch: smackdebt_git::ObjectReader,
    policy: HealthPolicy,
    width: usize,
    work: AnalysisWork,
) -> Result<Vec<DiffResult>, ProjectError> {
    if changes.len() <= 1 {
        let mut analyzer = Analyzer::default();
        return Ok(changes
            .into_iter()
            .enumerate()
            .map(|(index, change)| {
                let input = read_diff_input(index, change, &root_path, &base, &mut batch, &work);
                analyze_diff_input(input, policy, &mut analyzer, &work)
            })
            .collect());
    }
    let worker_count = width.max(1).min(changes.len());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(worker_count)
        .build()?;
    let (input_tx, input_rx) = mpsc::sync_channel::<DiffInput>(worker_count * 2);
    let input_rx = Arc::new(Mutex::new(input_rx));
    let (result_tx, result_rx) = mpsc::channel();
    let producer_work = work.clone();
    std::thread::scope(|threads| {
        let producer = threads.spawn(move || {
            for (index, change) in changes.into_iter().enumerate() {
                let input =
                    read_diff_input(index, change, &root_path, &base, &mut batch, &producer_work);
                if input_tx.send(input).is_err() {
                    break;
                }
            }
        });
        pool.scope(|scope| {
            for _ in 0..worker_count {
                let input_rx = Arc::clone(&input_rx);
                let result_tx = result_tx.clone();
                let worker_work = work.clone();
                scope.spawn(move |_| {
                    let mut analyzer = Analyzer::default();
                    loop {
                        let input = {
                            let receiver = input_rx.lock().expect("diff input queue poisoned");
                            receiver.recv()
                        };
                        let Ok(input) = input else { break };
                        if result_tx
                            .send(analyze_diff_input(
                                input,
                                policy,
                                &mut analyzer,
                                &worker_work,
                            ))
                            .is_err()
                        {
                            break;
                        }
                    }
                });
            }
        });
        producer.join().expect("diff input producer panicked");
    });
    drop(result_tx);
    let mut results: Vec<_> = result_rx.into_iter().collect();
    results.sort_by_key(|result| result.index);
    Ok(results)
}

fn read_diff_input(
    index: usize,
    change: Change,
    root_path: &Path,
    base: &str,
    batch: &mut smackdebt_git::ObjectReader,
    work: &AnalysisWork,
) -> DiffInput {
    let current = if !change.current_exists() {
        InputSide::missing()
    } else {
        match safe_worktree_path(root_path, change.current_path())
            .and_then(|path| fs::read(path).map_err(|error| error.to_string()))
        {
            Ok(bytes) => {
                work.source_reads.fetch_add(1, Ordering::Relaxed);
                #[cfg(feature = "evidence-stats")]
                crate::evidence::record_source_read();
                InputSide::bytes(bytes)
            }
            Err(error) => InputSide::failed(format!("could not read current file: {error}")),
        }
    };
    let before = if !change.base_exists() {
        InputSide::missing()
    } else {
        match batch.read_path(base, change.base_path()) {
            Ok(bytes) => InputSide::bytes(bytes),
            Err(error) => InputSide::failed(format!("could not read base file: {error}")),
        }
    };
    DiffInput {
        index,
        change,
        current,
        before,
    }
}

fn analyze_diff_input(
    input: DiffInput,
    policy: HealthPolicy,
    analyzer: &mut Analyzer,
    work: &AnalysisWork,
) -> DiffResult {
    let file_id = FileId::from_index(input.index);
    let current = analyze_diff_side(
        analyzer,
        file_id,
        input.change.current_path(),
        input.current,
        policy,
        work,
    );
    let before_path = input.change.base_path();
    let before = analyze_diff_side(analyzer, file_id, before_path, input.before, policy, work);
    let current_units = match &current {
        DiffSide::Analyzed { analysis, .. } => analysis.units(),
        _ => &[],
    };
    let before_units = match &before {
        DiffSide::Analyzed { analysis, .. } => analysis.units(),
        _ => &[],
    };
    work.record_algorithm_pass();
    let mut comparisons = compare_units(before_units, current_units, policy);
    comparisons
        .retain(|comparison| comparison.kind() != smackdebt_analysis::ComparisonKind::Unchanged);
    DiffResult {
        index: input.index,
        change: input.change,
        current,
        before,
        comparisons,
    }
}

fn analyze_diff_side(
    analyzer: &mut Analyzer,
    file: FileId,
    path: &Path,
    input: InputSide,
    policy: HealthPolicy,
    work: &AnalysisWork,
) -> DiffSide {
    if let Some(error) = input.error {
        return DiffSide::Failed(error);
    }
    let Some(bytes) = input.bytes else {
        return DiffSide::Missing;
    };
    match analyze_bytes(analyzer, file, path, bytes, work) {
        Ok(analysis) => {
            let mut health = HealthCounts::default();
            for unit in analysis.units() {
                health.add_rating(policy.assess(unit.measurements()).rating());
            }
            DiffSide::Analyzed { analysis, health }
        }
        Err(LanguageError::Unsupported(language)) => DiffSide::Unsupported(language),
        Err(error) => DiffSide::Failed(error.to_string()),
    }
}

fn add_diff_result(
    report: &mut AnalysisReportBuilder,
    result: DiffResult,
    comparison_index: &mut usize,
    scope_id: ScopeId,
    package: PackageId,
    included_in_code_diff: bool,
) {
    let file_id = FileId::from_index(result.index);

    for comparison in result.comparisons.iter().filter(|_| included_in_code_diff) {
        let comparison_id = ComparisonId::from_index(*comparison_index);
        report.add_comparison(
            Comparison::new(
                comparison_id,
                comparison.identity().clone(),
                comparison.kind(),
                comparison.before(),
                comparison.after(),
                comparison.before_rating(),
                comparison.after_rating(),
            )
            .with_file(file_id),
        );
        report.link_comparison(scope_id, comparison_id);
        *comparison_index += 1;
    }

    let selected = match &result.current {
        DiffSide::Missing => &result.before,
        current => current,
    };
    let (coverage, health, language) = if included_in_code_diff {
        diff_side_summary(selected)
    } else {
        let (_, _, language) = diff_side_summary(selected);
        (Coverage::default(), HealthCounts::default(), language)
    };
    let mut file = FileRecord::new(
        file_id,
        scope_id,
        result.change.current_path().to_string_lossy(),
        coverage,
        health,
    )
    .with_package(package);
    if let Some(language) = language {
        file = file.with_language(language);
    }
    report.add_file(file);
    report.link_file(scope_id, file_id);
    add_diff_diagnostic(report, file_id, &result.current, "current");
    add_diff_diagnostic(report, file_id, &result.before, "base");
}

fn diff_side_summary(side: &DiffSide) -> (Coverage, HealthCounts, Option<Language>) {
    match side {
        DiffSide::Missing => (
            Coverage::new(1, 0, 0, 0, 0, 0),
            HealthCounts::default(),
            None,
        ),
        DiffSide::Unsupported(language) => (
            Coverage::new(1, 0, 1, 0, 0, 0),
            HealthCounts::default(),
            Some(*language),
        ),
        DiffSide::Failed(_) => (
            Coverage::new(1, 0, 0, 1, 0, 0),
            HealthCounts::default(),
            None,
        ),
        DiffSide::Analyzed { analysis, health } => {
            let failed = matches!(analysis.parse_status(), ParseStatus::Failed);
            (
                Coverage::new(
                    1,
                    u32::from(!failed),
                    0,
                    u32::from(failed),
                    analysis.source_lines(),
                    u32::from(failed) * analysis.source_lines(),
                ),
                *health,
                Some(analysis.language()),
            )
        }
    }
}

fn add_diff_diagnostic(
    report: &mut AnalysisReportBuilder,
    file: FileId,
    side: &DiffSide,
    label: &str,
) {
    let (kind, message) = match side {
        DiffSide::Unsupported(language) => (
            DiagnosticKind::UnsupportedLanguage,
            format!("{label} side uses unsupported language: {language:?}"),
        ),
        DiffSide::Failed(message) => (DiagnosticKind::Other, format!("{label} side: {message}")),
        DiffSide::Analyzed { analysis, .. }
            if matches!(analysis.parse_status(), ParseStatus::Failed) =>
        {
            (
                DiagnosticKind::ParseFailure,
                format!("{label} side parser failed"),
            )
        }
        DiffSide::Missing | DiffSide::Analyzed { .. } => return,
    };
    report.add_diagnostic(Diagnostic::new(
        DiagnosticId::from_index(report.diagnostic_count()),
        Some(file),
        kind,
        message,
        0,
    ));
}

fn safe_worktree_path(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::RootDir
            )
        })
    {
        return Err(format!("unsafe worktree path {}", relative.display()));
    }
    let mut path = root.to_path_buf();
    for component in relative.components() {
        path.push(component);
        if let Ok(metadata) = fs::symlink_metadata(&path)
            && metadata.file_type().is_symlink()
        {
            return Err(format!(
                "symlink source is not analyzed: {}",
                relative.display()
            ));
        }
    }
    Ok(path)
}

fn analyze_current_files(
    inventory: &Inventory,
    candidates: &[&DiscoveredFile],
    width: ExecutionWidth,
    policy: HealthPolicy,
    work: &AnalysisWork,
) -> Result<Vec<FileResult>, ProjectError> {
    let analyze = |analyzer: &mut Analyzer, (index, file): (usize, &&DiscoveredFile)| {
        let path = file.path().as_path();
        let language = Analyzer::language(path);
        if matches!(language, Language::Kotlin | Language::Unknown) {
            return FileResult::Unsupported(language);
        }
        let Some(absolute) = inventory.absolute_path(file.path()) else {
            return FileResult::Failed("source path escaped the selected root".to_owned());
        };
        match fs::read(&absolute) {
            Ok(source) => {
                work.source_reads.fetch_add(1, Ordering::Relaxed);
                #[cfg(feature = "evidence-stats")]
                crate::evidence::record_source_read();
                match analyze_bytes(analyzer, FileId::from_index(index), path, source, work) {
                    Ok(value) => FileResult::Analyzed(rate_file(value, policy)),
                    Err(LanguageError::Unsupported(language)) => FileResult::Unsupported(language),
                    Err(error) => FileResult::Failed(error.to_string()),
                }
            }
            Err(error) => FileResult::Failed(error.to_string()),
        }
    };

    if candidates.len() < PARALLEL_FILE_CUTOVER || width.threads() == 1 {
        let mut analyzer = Analyzer::default();
        return Ok(candidates
            .iter()
            .enumerate()
            .map(|entry| analyze(&mut analyzer, entry))
            .collect());
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(width.threads())
        .build()?;
    Ok(pool.install(|| {
        candidates
            .par_iter()
            .enumerate()
            .map_init(Analyzer::default, analyze)
            .collect()
    }))
}

fn analyze_bytes(
    analyzer: &mut Analyzer,
    file: FileId,
    path: &Path,
    source: Vec<u8>,
    work: &AnalysisWork,
) -> Result<FileAnalysis, LanguageError> {
    let _ = file;
    work.record_parser_visit();
    work.record_algorithm_pass();
    analyzer.analyze(path, source)
}

struct EvolutionInput {
    accumulator: EvolutionAccumulator,
    coverage: HistoryCoverage,
}

struct LoadedEvolution {
    evolution: EvolutionInput,
    activity: HashMap<PathBuf, u32>,
    processes: usize,
    diagnostic: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HistoryAlias {
    Resolved(FileId, PackageId),
    Unusable,
}

fn load_evolution(
    inventory_root: &Path,
    history_days: u32,
    files: &[(PathBuf, FileId, PackageId)],
) -> LoadedEvolution {
    let Ok(repository) = GitRepository::discover(inventory_root) else {
        return LoadedEvolution {
            evolution: EvolutionInput {
                accumulator: EvolutionAccumulator::default(),
                coverage: HistoryCoverage::unavailable("not a Git repository"),
            },
            activity: HashMap::new(),
            processes: 0,
            diagnostic: Some("Git history unavailable: not a Git repository".to_owned()),
        };
    };
    let relative_root = inventory_root
        .strip_prefix(repository.root())
        .unwrap_or(Path::new(""));
    let mut aliases: HashMap<PathBuf, HistoryAlias> = files
        .iter()
        .map(|(path, file, package)| (path.clone(), HistoryAlias::Resolved(*file, *package)))
        .collect();
    let mut contributors = HashMap::<ContributorIdentity, ContributorId>::new();
    let mut accumulator = EvolutionAccumulator::default();
    let mut activity = HashMap::<PathBuf, u32>::new();
    let mut textual_changes = 0u32;
    let mut uncounted_changes = 0u32;
    let mut excluded_paths = 0u32;
    let mut rename_gaps = 0u32;
    let mut streamed_commits = 0u32;
    let cutoff = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(i64::MIN, |duration| duration.as_secs() as i64)
        .saturating_sub(i64::from(history_days) * 86_400);
    let history = repository.stream_history(|commit| {
        streamed_commits += 1;
        let contributes_activity = commit.timestamp() >= cutoff;
        let next_contributor = ContributorId::from_index(contributors.len());
        let contributor = *contributors
            .entry(commit.contributor().clone())
            .or_insert(next_contributor);
        let mut changes = Vec::new();
        for change in commit.changes() {
            let path = change
                .path()
                .strip_prefix(relative_root)
                .unwrap_or(change.path());
            let identity = aliases.get(path).copied();
            let Some(HistoryAlias::Resolved(file, package)) = identity else {
                excluded_paths += 1;
                continue;
            };
            if let Some(previous) = change.previous_path() {
                let previous = previous
                    .strip_prefix(relative_root)
                    .unwrap_or(previous)
                    .to_path_buf();
                match aliases.get(&previous) {
                    Some(HistoryAlias::Resolved(existing_file, existing_package))
                        if (*existing_file, *existing_package) != (file, package) =>
                    {
                        rename_gaps += 1;
                        aliases.insert(previous, HistoryAlias::Unusable);
                    }
                    Some(HistoryAlias::Unusable) => {}
                    _ => {
                        aliases.insert(previous, HistoryAlias::Resolved(file, package));
                    }
                }
            }
            if change.added_lines().is_some() && change.deleted_lines().is_some() {
                textual_changes += 1;
            } else {
                uncounted_changes += 1;
            }
            if contributes_activity {
                *activity.entry(files[file.index()].0.clone()).or_default() += 1;
            }
            changes.push(HistoryChangeFact::new(
                file,
                package,
                change.added_lines(),
                change.deleted_lines(),
            ));
        }
        if !changes.is_empty() {
            accumulator.accept(HistoryCommitFact::new(contributor, changes));
        }
        Ok(())
    });
    let process_count = repository.git_processes();
    match history {
        Ok(summary) => LoadedEvolution {
            evolution: EvolutionInput {
                accumulator,
                coverage: HistoryCoverage::new(
                    if summary.is_shallow() {
                        HistoryAvailability::Incomplete
                    } else {
                        HistoryAvailability::Complete
                    },
                    summary.revision().map(str::to_owned),
                    summary.commits(),
                    summary.newest_timestamp(),
                    summary.oldest_timestamp(),
                    textual_changes,
                    uncounted_changes,
                    excluded_paths,
                    rename_gaps,
                    summary
                        .is_shallow()
                        .then(|| "repository history is shallow".to_owned()),
                ),
            },
            activity,
            processes: process_count,
            diagnostic: None,
        },
        Err(error) => {
            let reason = error.to_string();
            let availability = failed_history_availability(&error, streamed_commits);
            let incomplete = availability == HistoryAvailability::Incomplete;
            let label = if incomplete {
                "incomplete"
            } else {
                "unavailable"
            };
            LoadedEvolution {
                evolution: EvolutionInput {
                    accumulator,
                    coverage: HistoryCoverage::new(
                        availability,
                        None,
                        streamed_commits,
                        None,
                        None,
                        textual_changes,
                        uncounted_changes,
                        excluded_paths,
                        rename_gaps,
                        Some(reason.clone()),
                    ),
                },
                activity,
                processes: process_count,
                diagnostic: Some(format!("Git history {label}: {reason}")),
            }
        }
    }
}

fn failed_history_availability(
    error: &smackdebt_git::GitError,
    streamed_commits: u32,
) -> HistoryAvailability {
    if matches!(error, smackdebt_git::GitError::InvalidOutput(_)) || streamed_commits > 0 {
        HistoryAvailability::Incomplete
    } else {
        HistoryAvailability::Unavailable
    }
}

struct RatedFile {
    analysis: FileAnalysis,
    health: HealthCounts,
    debt: Vec<(usize, HealthAssessment)>,
}

fn rate_file(analysis: FileAnalysis, policy: HealthPolicy) -> RatedFile {
    let mut health = HealthCounts::default();
    let mut debt = Vec::new();
    for (index, unit) in analysis.units().iter().enumerate() {
        let assessment = policy.assess(unit.measurements());
        health.add_rating(assessment.rating());
        if assessment.rating() != Rating::Healthy {
            debt.push((index, assessment));
        }
    }
    RatedFile {
        analysis,
        health,
        debt,
    }
}

enum FileResult {
    Analyzed(RatedFile),
    Unsupported(Language),
    Failed(String),
}

struct Selection {
    inventory_root: PathBuf,
    exact_file: Option<PathBuf>,
    prefix: Option<PathBuf>,
    label: String,
}

impl Selection {
    fn resolve(path: &Path, automatic_scope: bool) -> Result<Self, ProjectError> {
        let absolute = std::path::absolute(path).map_err(|source| ProjectError::Inspect {
            path: path.to_path_buf(),
            source,
        })?;
        if let Ok(repository) = GitRepository::discover(&absolute) {
            let root = repository
                .root()
                .canonicalize()
                .unwrap_or_else(|_| repository.root().to_path_buf());
            let selected_absolute = absolute.canonicalize().unwrap_or_else(|_| absolute.clone());
            let prefix = if automatic_scope {
                None
            } else {
                Some(
                    selected_absolute
                        .strip_prefix(&root)
                        .unwrap_or(Path::new(""))
                        .to_path_buf(),
                )
            };
            let exact_file = selected_absolute.is_file().then(|| {
                selected_absolute
                    .strip_prefix(&root)
                    .unwrap_or(Path::new(""))
                    .to_path_buf()
            });
            return Ok(Self {
                inventory_root: root,
                exact_file,
                prefix,
                label: if automatic_scope {
                    ".".to_owned()
                } else {
                    path.display().to_string()
                },
            });
        }
        if absolute.is_file() {
            let root = absolute.parent().unwrap_or(Path::new(".")).to_path_buf();
            let exact_file = absolute.file_name().map(PathBuf::from);
            return Ok(Self {
                inventory_root: root,
                exact_file,
                prefix: None,
                label: path.display().to_string(),
            });
        }
        Ok(Self {
            inventory_root: absolute,
            exact_file: None,
            prefix: None,
            label: path.display().to_string(),
        })
    }
}

struct CodebaseReportBuilder<'a> {
    mode: ReportMode,
    scopes: Vec<Scope>,
    files: Vec<FileRecord>,
    findings: Vec<Finding>,
    diagnostics: Vec<Diagnostic>,
    file_scopes: Vec<ScopeId>,
    activity: &'a HashMap<PathBuf, u32>,
    package_ids: Vec<PackageId>,
    dependencies: Vec<(FileId, PathBuf, Vec<DependencySyntax>)>,
    aliases: Vec<ResolutionAlias>,
    package_roots: Vec<PathBuf>,
    evolution: EvolutionInput,
}

impl<'a> CodebaseReportBuilder<'a> {
    fn new(
        mode: ReportMode,
        label: String,
        inventory: &Inventory,
        candidates: &[&DiscoveredFile],
        activity: &'a HashMap<PathBuf, u32>,
        aliases: Vec<ResolutionAlias>,
        evolution: EvolutionInput,
    ) -> Self {
        let package_roots: Vec<PathBuf> = inventory
            .packages()
            .iter()
            .map(|package| package.root().as_path().to_path_buf())
            .collect();
        let included_packages: Vec<bool> = (0..package_roots.len())
            .map(|index| {
                candidates
                    .iter()
                    .any(|file| file.package().index() == index)
            })
            .collect();
        let included_roots: Vec<PathBuf> = package_roots
            .iter()
            .zip(&included_packages)
            .filter_map(|(root, included)| included.then_some(root.clone()))
            .collect();
        let mut hierarchy = HierarchyBuilder::new(label, &included_roots);
        let mut package_ids = Vec::with_capacity(candidates.len());
        for file in candidates {
            let included_package_index = included_packages[..file.package().index()]
                .iter()
                .filter(|included| **included)
                .count();
            hierarchy.add_file(file.path().as_path(), included_package_index);
            package_ids.push(PackageId::from_index(included_package_index));
        }
        let file_scopes = candidates
            .iter()
            .map(|file| hierarchy.file_scopes[file.path().as_path()])
            .collect();

        Self {
            mode,
            scopes: hierarchy.scopes,
            files: Vec::with_capacity(candidates.len()),
            findings: Vec::with_capacity(candidates.len()),
            diagnostics: Vec::with_capacity(candidates.len()),
            file_scopes,
            activity,
            package_ids,
            dependencies: Vec::with_capacity(candidates.len()),
            aliases,
            package_roots: included_roots,
            evolution,
        }
    }

    fn add_analysis(&mut self, index: usize, result: FileResult) {
        let file_id = FileId::from_index(index);
        let scope_id = self.file_scopes[index];
        let path = self.scopes[scope_id.index()].name().to_owned();
        let touches = self.activity.get(Path::new(&path)).copied();
        let mut health = HealthCounts::default();
        let (coverage, language) = match result {
            FileResult::Analyzed(rated) => {
                health = rated.health;
                for (unit_index, assessment) in rated.debt {
                    let unit = &rated.analysis.units()[unit_index];
                    let finding_id = FindingId::from_index(self.findings.len());
                    self.scopes[scope_id.index()].add_finding(finding_id);
                    self.findings.push(Finding::new(
                        finding_id,
                        file_id,
                        unit.identity().clone(),
                        unit.span(),
                        unit.measurements(),
                        assessment,
                    ));
                }
                let analysis = rated.analysis;
                self.dependencies.push((
                    file_id,
                    PathBuf::from(&path),
                    analysis.dependencies().to_vec(),
                ));
                let failed = matches!(analysis.parse_status(), ParseStatus::Failed);
                let recovered = matches!(analysis.parse_status(), ParseStatus::Recovered);
                if failed {
                    self.add_diagnostic(
                        file_id,
                        DiagnosticKind::ParseFailure,
                        "parser failed",
                        analysis.source_lines(),
                    );
                } else if recovered {
                    self.add_diagnostic(
                        file_id,
                        DiagnosticKind::ParseFailure,
                        "parser recovered from syntax errors",
                        0,
                    );
                }
                (
                    Coverage::new(
                        1,
                        u32::from(!failed),
                        0,
                        u32::from(failed),
                        analysis.source_lines(),
                        u32::from(failed) * analysis.source_lines(),
                    ),
                    Some(analysis.language()),
                )
            }
            FileResult::Unsupported(language) => {
                self.add_diagnostic(
                    file_id,
                    DiagnosticKind::UnsupportedLanguage,
                    format!("{path} uses an unsupported language"),
                    0,
                );
                (Coverage::new(1, 0, 1, 0, 0, 0), Some(language))
            }
            FileResult::Failed(message) => {
                self.add_diagnostic(
                    file_id,
                    DiagnosticKind::UnreadableFile,
                    format!("{path}: {message}"),
                    0,
                );
                (Coverage::new(1, 0, 0, 1, 0, 0), None)
            }
        };
        let mut file = FileRecord::new(file_id, scope_id, path, coverage, health);
        file = file.with_package(self.package_ids[index]);
        if let Some(language) = language {
            file = file.with_language(language);
        }
        if let Some(touches) = touches {
            file = file.with_activity(FileActivity::new(touches));
        }
        self.scopes[scope_id.index()].add_file(file_id);
        self.files.push(file);
    }

    fn add_diagnostic(
        &mut self,
        file: FileId,
        kind: DiagnosticKind,
        message: impl Into<String>,
        excluded_lines: u32,
    ) {
        let id = DiagnosticId::from_index(self.diagnostics.len());
        self.diagnostics.push(Diagnostic::new(
            id,
            Some(file),
            kind,
            message,
            excluded_lines,
        ));
    }

    fn add_general_diagnostic(&mut self, kind: DiagnosticKind, message: impl Into<String>) {
        let id = DiagnosticId::from_index(self.diagnostics.len());
        self.diagnostics
            .push(Diagnostic::new(id, None, kind, message, 0));
    }

    fn finish(self, work: &AnalysisWork) -> Report {
        let architecture = build_architecture(
            work,
            &self.files,
            &self.dependencies,
            &self.aliases,
            &self.package_roots,
            &self.package_roots,
        );
        let architecture_findings_for_links = architecture.findings.clone();
        let package_edges = architecture.package_edges.clone();
        work.record_algorithm_pass();
        let evolution = self.evolution.accumulator.finish(
            self.evolution.coverage,
            self.files.len(),
            self.package_roots.len(),
            &package_edges,
            None,
        );
        let evolutionary_findings = evolution.findings().to_vec();
        let root = ScopeId::from_index(0);
        let mut builder = AnalysisReportBuilder::with_capacity(
            self.mode,
            self.scopes.len(),
            self.files.len(),
            self.findings.len(),
            self.diagnostics.len(),
            0,
        );
        for scope in self.scopes {
            builder.add_scope(scope);
        }
        builder.set_root(root);
        for file in self.files {
            builder.add_file(file);
        }
        for finding in self.findings {
            builder.add_finding(finding);
        }
        for diagnostic in self.diagnostics {
            builder.add_diagnostic(diagnostic);
        }
        builder.set_architecture(ArchitectureReportFacts::new(
            ArchitectureGraph::new(
                architecture.coverage,
                architecture.file_edges,
                architecture.package_edges,
                architecture.external,
                architecture.diagnostics,
                architecture.measurements,
            ),
            architecture.findings,
            Vec::new(),
        ));
        builder.set_evolution(evolution);
        for finding in evolutionary_findings {
            builder.link_evolutionary_finding(root, finding.id());
            let pair = finding.coupling();
            builder.link_evolutionary_finding(
                ScopeId::from_index(1 + pair.left().index()),
                finding.id(),
            );
            builder.link_evolutionary_finding(
                ScopeId::from_index(1 + pair.right().index()),
                finding.id(),
            );
        }
        for (index, finding) in architecture.finding_links {
            builder.link_architecture_finding(index, finding);
        }
        for finding in &architecture_findings_for_links {
            for package in finding.packages() {
                builder.link_architecture_finding(
                    ScopeId::from_index(1 + package.index()),
                    finding.id(),
                );
            }
            for file in finding.files() {
                builder
                    .link_architecture_finding(builder.files()[file.index()].scope(), finding.id());
            }
        }
        builder.finish()
    }
}

struct ArchitectureBuild {
    coverage: DependencyCoverage,
    file_edges: Vec<DependencyEdge>,
    package_edges: Vec<PackageEdge>,
    external: Vec<ExternalDependency>,
    diagnostics: Vec<ResolutionDiagnostic>,
    measurements: Vec<PackageGraphMeasurement>,
    findings: Vec<ArchitectureFinding>,
    finding_links: Vec<(ScopeId, ArchitectureFindingId)>,
    cycles: Vec<smackdebt_analysis::PackageCycle>,
}

#[derive(Clone)]
struct ResolutionAlias {
    prefix: String,
    suffix: String,
    replacement: String,
}

impl ResolutionAlias {
    fn expand(&self, candidate: &str) -> Option<String> {
        let middle = candidate
            .strip_prefix(&self.prefix)?
            .strip_suffix(&self.suffix)?;
        Some(self.replacement.replace('*', middle))
    }
}

fn load_resolution_aliases(root: &Path) -> Vec<ResolutionAlias> {
    for name in ["tsconfig.json", "jsconfig.json"] {
        let Ok(source) = fs::read(root.join(name)) else {
            continue;
        };
        let Some(aliases) = parse_resolution_aliases(&source) else {
            continue;
        };
        return aliases;
    }
    Vec::new()
}

fn load_base_resolution_aliases(
    reader: &mut smackdebt_git::ObjectReader,
    base: &str,
) -> Vec<ResolutionAlias> {
    for name in ["tsconfig.json", "jsconfig.json"] {
        if let Ok(source) = reader.read_path(base, Path::new(name))
            && let Some(aliases) = parse_resolution_aliases(&source)
        {
            return aliases;
        }
    }
    Vec::new()
}

fn parse_resolution_aliases(source: &[u8]) -> Option<Vec<ResolutionAlias>> {
    let value = serde_json::from_slice::<serde_json::Value>(source).ok()?;
    let base = value["compilerOptions"]["baseUrl"]
        .as_str()
        .unwrap_or("")
        .trim_matches('/');
    let base = if base == "." { "" } else { base };
    let Some(paths) = value["compilerOptions"]["paths"].as_object() else {
        return Some(Vec::new());
    };
    let mut aliases = Vec::new();
    for (pattern, replacements) in paths {
        let (prefix, suffix) = pattern
            .split_once('*')
            .map_or((pattern.as_str(), ""), |parts| parts);
        for replacement in replacements.as_array().into_iter().flatten() {
            let Some(replacement) = replacement.as_str() else {
                continue;
            };
            let replacement = if base.is_empty() {
                replacement.to_owned()
            } else {
                format!("{base}/{replacement}")
            };
            aliases.push(ResolutionAlias {
                prefix: prefix.to_owned(),
                suffix: suffix.to_owned(),
                replacement,
            });
        }
    }
    aliases.sort_by(|left, right| {
        (&left.prefix, &left.suffix, &left.replacement).cmp(&(
            &right.prefix,
            &right.suffix,
            &right.replacement,
        ))
    });
    Some(aliases)
}

fn build_architecture(
    work: &AnalysisWork,
    files: &[FileRecord],
    dependencies: &[(FileId, PathBuf, Vec<DependencySyntax>)],
    aliases: &[ResolutionAlias],
    side_package_roots: &[PathBuf],
    package_roots: &[PathBuf],
) -> ArchitectureBuild {
    work.record_algorithm_pass();
    let mut index = BTreeMap::new();
    for (file, path, _) in dependencies {
        index.insert(path.clone(), *file);
    }
    let mut internal = 0u32;
    let mut external_count = 0u32;
    let mut unresolved = 0u32;
    let mut ambiguous = 0u32;
    let mut edge_values: BTreeMap<(FileId, FileId), (u32, Vec<smackdebt_analysis::SourceSpan>)> =
        BTreeMap::new();
    let mut external_values: BTreeMap<(FileId, String), u32> = BTreeMap::new();
    let mut diagnostics = Vec::new();

    for (source, source_path, references) in dependencies {
        for reference in references {
            match reference.state() {
                DependencySyntaxState::External => {
                    external_count += 1;
                    *external_values
                        .entry((*source, reference.target().to_owned()))
                        .or_default() += 1;
                }
                DependencySyntaxState::Unresolved(reason) => {
                    unresolved += 1;
                    diagnostics.push(ResolutionDiagnostic::new(
                        *source,
                        reference.span(),
                        reference.target(),
                        ResolutionIssueKind::Unresolved,
                        reason,
                    ));
                }
                DependencySyntaxState::Candidates(candidates) => {
                    let matches = resolve_candidates(source_path, candidates, &index, aliases);
                    match matches.as_slice() {
                        [] if reference.intent()
                            == smackdebt_analysis::DependencyIntent::Internal =>
                        {
                            unresolved += 1;
                            diagnostics.push(ResolutionDiagnostic::new(
                                *source,
                                reference.span(),
                                reference.target(),
                                ResolutionIssueKind::Unresolved,
                                "no repository file matches",
                            ));
                        }
                        [] => {
                            external_count += 1;
                            *external_values
                                .entry((*source, reference.target().to_owned()))
                                .or_default() += 1;
                        }
                        [target] => {
                            internal += 1;
                            if source != target {
                                let entry = edge_values.entry((*source, *target)).or_default();
                                entry.0 += 1;
                                if entry.1.len() < 3 {
                                    entry.1.push(reference.span());
                                }
                            }
                        }
                        _ => {
                            ambiguous += 1;
                            diagnostics.push(ResolutionDiagnostic::new(
                                *source,
                                reference.span(),
                                reference.target(),
                                ResolutionIssueKind::Ambiguous,
                                "several repository files match",
                            ));
                        }
                    }
                }
            }
        }
    }

    let file_edges: Vec<_> = edge_values
        .into_iter()
        .enumerate()
        .map(
            |(edge_index, ((source, target), (references, locations)))| {
                DependencyEdge::new(
                    DependencyEdgeId::from_index(edge_index),
                    source,
                    target,
                    references,
                    locations,
                )
            },
        )
        .collect();
    let external: Vec<_> = external_values
        .into_iter()
        .map(|((file, target), references)| ExternalDependency::new(file, target, references))
        .collect();

    let mut package_values: BTreeMap<(PackageId, PackageId), (u32, u32, Vec<DependencyEdgeId>)> =
        BTreeMap::new();
    for edge in &file_edges {
        let source_path = dependencies
            .iter()
            .find(|(id, _, _)| *id == edge.source())
            .map(|(_, path, _)| path.as_path())
            .unwrap_or_else(|| Path::new(files[edge.source().index()].path()));
        let target_path = dependencies
            .iter()
            .find(|(id, _, _)| *id == edge.target())
            .map(|(_, path, _)| path.as_path())
            .unwrap_or_else(|| Path::new(files[edge.target().index()].path()));
        let source_root = nearest_package_root(source_path, side_package_roots);
        let target_root = nearest_package_root(target_path, side_package_roots);
        let source = PackageId::from_index(
            package_roots
                .iter()
                .position(|root| root == &source_root)
                .unwrap_or(0),
        );
        let target = PackageId::from_index(
            package_roots
                .iter()
                .position(|root| root == &target_root)
                .unwrap_or(0),
        );
        if source != target {
            let value = package_values.entry((source, target)).or_default();
            value.0 += 1;
            value.1 += edge.references();
            value.2.push(edge.id());
        }
    }
    let package_edges: Vec<_> = package_values
        .into_iter()
        .enumerate()
        .map(|(index, ((source, target), (pairs, references, edges)))| {
            PackageEdge::new(
                PackageEdgeId::from_index(index),
                source,
                target,
                pairs,
                references,
                edges,
            )
        })
        .collect();
    let package_count = package_roots.len();
    let package_pairs: Vec<_> = package_edges
        .iter()
        .map(|edge| (edge.source().index(), edge.target().index()))
        .collect();
    let degrees = dependency_degree(package_count, &package_pairs);
    let measurements = degrees
        .into_iter()
        .enumerate()
        .map(|(index, (incoming, outgoing))| {
            PackageGraphMeasurement::new(PackageId::from_index(index), incoming, outgoing)
        })
        .collect();

    let mut findings = Vec::new();
    let mut finding_links = Vec::new();
    let mut cycles = Vec::new();
    for component in strongly_connected_components(package_count, &package_pairs)
        .into_iter()
        .filter(|component| component.len() > 1)
    {
        let witness = cycle_witness(&component, &package_pairs).unwrap_or_default();
        let witness_edges: Vec<_> = witness
            .iter()
            .filter_map(|&(source, target)| {
                package_edges
                    .iter()
                    .find(|edge| edge.source().index() == source && edge.target().index() == target)
            })
            .flat_map(|edge| edge.file_edges().iter().copied().take(1))
            .collect();
        let involved_files: Vec<_> = witness_edges
            .iter()
            .flat_map(|id| {
                let edge = &file_edges[id.index()];
                [edge.source(), edge.target()]
            })
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let packages: Vec<_> = component.into_iter().map(PackageId::from_index).collect();
        let mut package_witness: Vec<_> = witness
            .iter()
            .map(|(source, _)| PackageId::from_index(*source))
            .collect();
        if let Some((_, target)) = witness.last() {
            package_witness.push(PackageId::from_index(*target));
        }
        cycles.push((packages.clone(), package_witness));
        let id = ArchitectureFindingId::from_index(findings.len());
        for package in &packages {
            if let Some(scope) = files
                .iter()
                .find(|file| file.package() == Some(*package))
                .map(FileRecord::scope)
            {
                finding_links.push((scope, id));
            }
        }
        findings.push(ArchitectureFinding::new(
            id,
            ArchitectureFindingKind::PackageCycle,
            packages,
            involved_files,
            witness_edges,
        ));
    }
    let file_pairs: Vec<_> = file_edges
        .iter()
        .map(|edge| (edge.source().index(), edge.target().index()))
        .collect();
    for component in strongly_connected_components(files.len(), &file_pairs)
        .into_iter()
        .filter(|component| component.len() > 1)
    {
        let packages: std::collections::BTreeSet<_> = component
            .iter()
            .filter_map(|file| files[*file].package())
            .collect();
        if packages.len() != 1 {
            continue;
        }
        let witness = cycle_witness(&component, &file_pairs).unwrap_or_default();
        let witness_edges: Vec<_> = witness
            .iter()
            .filter_map(|&(source, target)| {
                file_edges
                    .iter()
                    .find(|edge| edge.source().index() == source && edge.target().index() == target)
                    .map(DependencyEdge::id)
            })
            .collect();
        let id = ArchitectureFindingId::from_index(findings.len());
        for file in &component {
            finding_links.push((files[*file].scope(), id));
        }
        findings.push(ArchitectureFinding::new(
            id,
            ArchitectureFindingKind::FileCycle,
            packages.into_iter().collect(),
            component.into_iter().map(FileId::from_index).collect(),
            witness_edges,
        ));
    }

    ArchitectureBuild {
        coverage: DependencyCoverage::new(internal, external_count, unresolved, ambiguous),
        file_edges,
        package_edges,
        external,
        diagnostics,
        measurements,
        findings,
        finding_links,
        cycles,
    }
}

fn resolve_candidates(
    source: &Path,
    candidates: &[String],
    index: &BTreeMap<PathBuf, FileId>,
    aliases: &[ResolutionAlias],
) -> Vec<FileId> {
    let parent = source.parent().unwrap_or(Path::new(""));
    let mut matches = std::collections::BTreeSet::new();
    for candidate in candidates {
        let mut expanded = vec![candidate.clone()];
        expanded.extend(aliases.iter().filter_map(|alias| alias.expand(candidate)));
        for candidate in expanded {
            let path = Path::new(&candidate);
            let joined = if candidate.starts_with("./") || candidate.starts_with("../") {
                parent.join(path)
            } else {
                path.to_path_buf()
            };
            if let Some(clean) = clean_relative(&joined)
                && let Some(file) = index.get(&clean)
            {
                matches.insert(*file);
            }
        }
    }
    matches.into_iter().collect()
}

fn clean_relative(path: &Path) -> Option<PathBuf> {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::Normal(value) => result.push(value),
            std::path::Component::ParentDir => {
                if !result.pop() {
                    return None;
                }
            }
            std::path::Component::RootDir | std::path::Component::Prefix(_) => return None,
        }
    }
    Some(result)
}

fn diff_filter(root: &Path, selected: &Path) -> Option<PathBuf> {
    let absolute = std::path::absolute(selected).ok()?;
    let absolute = absolute.canonicalize().unwrap_or(absolute);
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    absolute.strip_prefix(root).ok().map(Path::to_path_buf)
}

const DIFF_MANIFEST_NAMES: &[&str] = &[
    "Cargo.toml",
    "package.json",
    "pyproject.toml",
    "setup.py",
    "setup.cfg",
    "pom.xml",
    "settings.gradle",
    "settings.gradle.kts",
    "build.gradle",
    "build.gradle.kts",
    "CMakeLists.txt",
    "Gemfile",
    "gems.rb",
];

fn base_package_roots(
    current: &[PathBuf],
    changed: &[Change],
    reader: &mut smackdebt_git::ObjectReader,
    base: &str,
) -> Vec<PathBuf> {
    let mut roots: std::collections::BTreeSet<_> = current.iter().cloned().collect();
    for change in changed {
        let is_manifest = change
            .base_path()
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| DIFF_MANIFEST_NAMES.contains(&name));
        if !is_manifest {
            continue;
        }
        let root = change
            .base_path()
            .parent()
            .unwrap_or(Path::new(""))
            .to_path_buf();
        let base_has_manifest = DIFF_MANIFEST_NAMES
            .iter()
            .any(|name| reader.read_path(base, &root.join(name)).is_ok());
        if base_has_manifest {
            roots.insert(root);
        } else {
            roots.remove(&root);
        }
    }
    if roots.is_empty() {
        roots.insert(PathBuf::new());
    }
    roots.into_iter().collect()
}

fn diff_package_roots(repository_root: &Path, changed: &[Change]) -> Vec<PathBuf> {
    let mut roots = std::collections::BTreeSet::new();
    for change in changed {
        for path in [change.current_path(), change.base_path()] {
            let mut directory = path.parent().unwrap_or(Path::new(""));
            loop {
                let absolute = repository_root.join(directory);
                let changed_manifest = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| DIFF_MANIFEST_NAMES.contains(&name));
                let working_tree_manifest = DIFF_MANIFEST_NAMES
                    .iter()
                    .any(|name| absolute.join(name).is_file());
                if changed_manifest || working_tree_manifest {
                    roots.insert(directory.to_path_buf());
                    break;
                }
                if directory.as_os_str().is_empty() {
                    break;
                }
                directory = directory.parent().unwrap_or(Path::new(""));
            }
        }
    }
    if roots.is_empty() {
        roots.insert(PathBuf::new());
    }
    roots.into_iter().collect()
}

fn nearest_package_root(path: &Path, package_roots: &[PathBuf]) -> PathBuf {
    package_roots
        .iter()
        .filter(|root| path.starts_with(root))
        .max_by_key(|root| root.components().count())
        .cloned()
        .unwrap_or_default()
}

struct HierarchyBuilder {
    scopes: Vec<Scope>,
    file_scopes: BTreeMap<PathBuf, ScopeId>,
    directories: BTreeMap<(PathBuf, PathBuf), ScopeId>,
    package_roots: Vec<PathBuf>,
}

impl HierarchyBuilder {
    fn new(label: String, package_roots: &[PathBuf]) -> Self {
        let root = ScopeId::from_index(0);
        let mut scopes = vec![Scope::new(root, ScopeKind::Repository, label, None)];
        for package_root in package_roots {
            let id = ScopeId::from_index(scopes.len());
            scopes[root.index()].add_child(id);
            scopes.push(Scope::new(
                id,
                ScopeKind::Package,
                if package_root.as_os_str().is_empty() {
                    ".".to_owned()
                } else {
                    package_root.display().to_string()
                },
                Some(root),
            ));
        }
        Self {
            scopes,
            file_scopes: BTreeMap::new(),
            directories: BTreeMap::new(),
            package_roots: package_roots.to_vec(),
        }
    }

    fn add_file(&mut self, path: &Path, package_index: usize) {
        let package_root = self.package_roots[package_index].clone();
        let package_scope = self.scopes[0].children()[package_index];
        let relative_directory = path
            .parent()
            .unwrap_or(Path::new(""))
            .strip_prefix(&package_root)
            .unwrap_or(Path::new(""));
        let mut parent = package_scope;
        let mut accumulated = package_root.clone();
        for component in relative_directory.components() {
            accumulated.push(component);
            let key = (package_root.clone(), accumulated.clone());
            parent = if let Some(id) = self.directories.get(&key) {
                *id
            } else {
                let id = ScopeId::from_index(self.scopes.len());
                self.scopes[parent.index()].add_child(id);
                self.scopes.push(Scope::new(
                    id,
                    ScopeKind::Directory,
                    accumulated.display().to_string(),
                    Some(parent),
                ));
                self.directories.insert(key, id);
                id
            };
        }
        let file_scope = ScopeId::from_index(self.scopes.len());
        self.scopes[parent.index()].add_child(file_scope);
        self.scopes.push(Scope::new(
            file_scope,
            ScopeKind::File,
            path.display().to_string(),
            Some(parent),
        ));
        self.file_scopes.insert(path.to_path_buf(), file_scope);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn git<const N: usize>(root: &Path, args: [&str; N]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn repository() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        let actual = root.path().join("repo");
        fs::create_dir_all(&actual).unwrap();
        git(&actual, ["init", "-q"]);
        git(&actual, ["config", "user.email", "test@example.invalid"]);
        git(&actual, ["config", "user.name", "Smackdebt Test"]);
        fs::write(
            actual.join("sample.rs"),
            "fn work(value: i32) -> i32 { value + 1 }\n",
        )
        .unwrap();
        git(&actual, ["add", "."]);
        git(&actual, ["commit", "-qm", "initial"]);
        root
    }

    #[test]
    fn execution_width_rejects_zero() {
        assert_eq!(ExecutionWidth::fixed(0), None);
        assert!(matches!(
            ExecutionWidth::fixed(1),
            Some(ExecutionWidth::Fixed(_))
        ));
    }

    #[test]
    fn malformed_or_interrupted_history_is_incomplete_but_empty_history_is_unavailable() {
        assert_eq!(
            failed_history_availability(
                &smackdebt_git::GitError::InvalidOutput("malformed record".to_owned()),
                0,
            ),
            HistoryAvailability::Incomplete
        );
        assert_eq!(
            failed_history_availability(&smackdebt_git::GitError::EmptyHistory, 0),
            HistoryAvailability::Unavailable
        );
        assert_eq!(
            failed_history_availability(&smackdebt_git::GitError::EmptyHistory, 1),
            HistoryAvailability::Incomplete
        );
    }

    #[test]
    fn reused_rename_path_excludes_older_history_from_both_current_files() {
        let root = tempfile::tempdir().unwrap();
        git(root.path(), ["init", "-q"]);
        git(
            root.path(),
            ["config", "user.email", "test@example.invalid"],
        );
        git(root.path(), ["config", "user.name", "Smackdebt Test"]);
        fs::write(root.path().join("old.js"), "export const value = 1;\n").unwrap();
        git(root.path(), ["add", "-A"]);
        git(root.path(), ["commit", "-qm", "initial old path"]);
        fs::rename(root.path().join("old.js"), root.path().join("new.js")).unwrap();
        git(root.path(), ["add", "-A"]);
        git(root.path(), ["commit", "-qm", "rename old to new"]);
        fs::write(root.path().join("old.js"), "export const reused = 2;\n").unwrap();
        git(root.path(), ["add", "-A"]);
        git(root.path(), ["commit", "-qm", "reuse old path"]);

        let result =
            analyze_codebase(&CodebaseRequest::new(root.path()).with_history_days(36_500)).unwrap();
        let report = result.report();
        let touches = report
            .file_history()
            .iter()
            .map(|history| {
                (
                    report.files()[history.file().index()].path(),
                    history.touches(),
                )
            })
            .collect::<HashMap<_, _>>();
        assert_eq!(touches["new.js"], 1);
        assert_eq!(touches["old.js"], 1);
        assert_eq!(report.history_coverage().rename_gaps(), 1);
        assert_eq!(report.history_coverage().excluded_paths(), 1);
    }

    #[test]
    fn one_file_is_read_once_and_produces_a_report() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("sample.rb"),
            "def work\n  if ready\n    go\n  end\nend\n",
        )
        .unwrap();
        let result = analyze_codebase(
            &CodebaseRequest::new(root.path()).with_width(ExecutionWidth::fixed(1).unwrap()),
        )
        .unwrap();
        assert_eq!(result.stats().inventory_walks, 1);
        assert_eq!(result.stats().source_reads, 1);
        assert_eq!(result.report().files().len(), 1);
    }

    #[test]
    fn files_use_compact_report_package_ids_when_a_manifest_has_no_source() {
        let root = tempfile::tempdir().unwrap();
        for package in ["a", "b", "c"] {
            fs::create_dir_all(root.path().join(package)).unwrap();
            fs::write(
                root.path().join(package).join("package.json"),
                format!("{{\"name\":\"{package}\",\"private\":true}}\n"),
            )
            .unwrap();
        }
        fs::write(
            root.path().join("a/main.js"),
            "export function a() { return 1; }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("c/main.js"),
            "export function c() { return 1; }\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        assert_eq!(result.report().package_graph().len(), 2);
        let package_ids: Vec<_> = result
            .report()
            .files()
            .iter()
            .map(|file| file.package().unwrap().index())
            .collect();
        assert_eq!(package_ids, [0, 1]);
    }

    #[test]
    fn codebase_builds_package_cycles_and_exact_dependency_coverage() {
        let root = tempfile::tempdir().unwrap();
        for package in ["app", "core"] {
            fs::create_dir_all(root.path().join(package)).unwrap();
            fs::write(root.path().join(package).join("package.json"), "{}").unwrap();
        }
        fs::write(
            root.path().join("app/a.js"),
            "import core from '../core/b';\nimport ext from 'external';\nconst late = require(name);\nfunction app() {}\n",
        ).unwrap();
        fs::write(
            root.path().join("core/b.js"),
            "import app from '../app/a';\nfunction core() {}\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        assert_eq!(report.dependency_edges().len(), 2);
        assert_eq!(report.package_edges().len(), 2);
        assert_eq!(
            report.dependency_coverage(),
            DependencyCoverage::new(2, 1, 1, 0)
        );
        assert_eq!(report.architecture_findings().len(), 1);
        assert_eq!(
            report.architecture_findings()[0].kind(),
            ArchitectureFindingKind::PackageCycle
        );
        assert_eq!(report.architecture_findings()[0].rating(), Rating::High);
    }

    #[test]
    fn repeated_references_share_file_and_package_edges_with_exact_counts() {
        let root = tempfile::tempdir().unwrap();
        for package in ["app", "core"] {
            fs::create_dir_all(root.path().join(package)).unwrap();
            fs::write(root.path().join(package).join("package.json"), "{}").unwrap();
        }
        fs::write(
            root.path().join("app/a.js"),
            "import first from '../core/b';\nimport second from '../core/b';\nfunction app() {}\n",
        )
        .unwrap();
        fs::write(
            root.path().join("app/other.js"),
            "import core from '../core/b';\nfunction other() {}\n",
        )
        .unwrap();
        fs::write(
            root.path().join("core/b.js"),
            "export default function core() {}\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        assert_eq!(report.dependency_edges().len(), 2);
        assert_eq!(report.package_edges().len(), 1);
        assert_eq!(report.package_edges()[0].file_pairs(), 2);
        assert_eq!(report.package_edges()[0].references(), 3);
    }

    #[test]
    fn project_configuration_aliases_resolve_as_data_without_execution() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("src/core")).unwrap();
        fs::write(root.path().join("package.json"), "{}").unwrap();
        fs::write(
            root.path().join("tsconfig.json"),
            r#"{"compilerOptions":{"baseUrl":".","paths":{"@/*":["src/*"]}}}"#,
        )
        .unwrap();
        fs::write(
            root.path().join("src/main.ts"),
            "import core from '@/core/index';\nfunction main() { return core(); }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("src/core/index.ts"),
            "export default function core() {}\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        assert_eq!(result.report().dependency_edges().len(), 1);
        assert_eq!(result.report().dependency_coverage().internal(), 1);
    }

    #[test]
    fn java_source_root_import_resolves_to_a_repository_file() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("src/main/java/app")).unwrap();
        fs::create_dir_all(root.path().join("src/main/java/usecase")).unwrap();
        fs::write(root.path().join("pom.xml"), "<project />").unwrap();
        fs::write(
            root.path().join("src/main/java/app/Local.java"),
            "package app; public class Local {}\n",
        )
        .unwrap();
        fs::write(
            root.path().join("src/main/java/usecase/Main.java"),
            "package usecase; import app.Local; public class Main {}\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        assert_eq!(result.report().dependency_edges().len(), 1);
        assert_eq!(result.report().dependency_coverage().internal(), 1);
    }

    #[test]
    fn rust_root_use_resolves_leaf_or_parent_module_and_keeps_ambiguity_explicit() {
        for (name, leaf, parent, expected_internal, expected_ambiguous) in [
            ("leaf", true, false, 1, 0),
            ("parent", false, true, 1, 0),
            ("both", true, true, 0, 1),
        ] {
            let root = tempfile::tempdir().unwrap();
            fs::write(
                root.path().join("main.rs"),
                "use crate::core::work;\nfn main() { work(); }\n",
            )
            .unwrap();
            if leaf {
                fs::create_dir_all(root.path().join("core")).unwrap();
                fs::write(root.path().join("core/work.rs"), "pub fn work() {}\n").unwrap();
            }
            if parent {
                fs::write(root.path().join("core.rs"), "pub fn work() {}\n").unwrap();
            }

            let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
            assert_eq!(
                result.report().dependency_coverage().internal(),
                expected_internal,
                "{name}"
            );
            assert_eq!(
                result.report().dependency_coverage().ambiguous(),
                expected_ambiguous,
                "{name}"
            );
            assert_eq!(
                result.report().dependency_edges().len(),
                expected_internal as usize,
                "{name}"
            );
            assert_eq!(
                result.report().resolution_diagnostics().len(),
                expected_ambiguous as usize,
                "{name}"
            );
        }
    }

    #[test]
    fn package_selection_keeps_explanatory_incoming_edges_from_the_root_graph() {
        let root = tempfile::tempdir().unwrap();
        git(root.path(), ["init", "-q"]);
        for package in ["app", "core"] {
            fs::create_dir_all(root.path().join(package)).unwrap();
            fs::write(root.path().join(package).join("package.json"), "{}").unwrap();
        }
        fs::write(
            root.path().join("app/a.js"),
            "import core from '../core/b';\nfunction app() {}\n",
        )
        .unwrap();
        fs::write(root.path().join("core/b.js"), "function core() {}\n").unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path().join("core"))).unwrap();
        assert_eq!(result.report().files().len(), 2);
        assert_eq!(result.report().dependency_edges().len(), 1);
        let selected = result.selected_scope().unwrap();
        assert_eq!(result.report().scopes()[selected.index()].name(), "core");
    }

    #[test]
    fn automatic_scope_uses_the_git_root_from_a_nested_directory() {
        let root = repository();
        let repository_path = root.path().join("repo");
        let nested = repository_path.join("nested/deeper");
        fs::create_dir_all(&nested).unwrap();
        let result = analyze_codebase(&CodebaseRequest::automatic(&nested)).unwrap();
        assert!(
            result
                .report()
                .files()
                .iter()
                .any(|file| file.path() == "sample.rs")
        );
        assert_eq!(result.report().scopes()[0].name(), ".");
    }

    #[test]
    fn codebase_path_selection_keeps_repository_relative_scope_identity() {
        let root = repository();
        let repository_path = root.path().join("repo");
        fs::create_dir_all(repository_path.join("src")).unwrap();
        fs::write(
            repository_path.join("src/lib.rs"),
            "fn selected() { if true {} }\n",
        )
        .unwrap();
        let result = analyze_codebase(&CodebaseRequest::new(repository_path.join("src"))).unwrap();
        assert_ne!(result.selected_scope(), result.report().root());
        assert!(
            result
                .report()
                .files()
                .iter()
                .any(|file| file.path() == "src/lib.rs")
        );
        assert!(
            result
                .report()
                .scopes()
                .iter()
                .any(|scope| scope.name() == "src")
        );
    }

    #[test]
    fn source_outside_manifest_roots_uses_discoverys_fallback_package() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("app/src")).unwrap();
        fs::write(
            root.path().join("app/Cargo.toml"),
            "[package]\nname='app'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(root.path().join("app/src/lib.rs"), "fn app() {}\n").unwrap();
        fs::write(root.path().join("outside.rs"), "fn outside() {}\n").unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();

        assert_eq!(result.report().files().len(), 2);
        assert!(
            result
                .report()
                .files()
                .iter()
                .any(|file| file.path() == "outside.rs")
        );
    }

    #[test]
    fn diff_retains_changed_file_scopes_and_reports_side_failures() {
        let root = repository();
        let repository_path = root.path().join("repo");
        fs::write(
            repository_path.join("sample.rs"),
            "fn work(value: i32) -> i32 { value + value + 1 }\n",
        )
        .unwrap();
        fs::write(repository_path.join("new.rs"), "fn added() {}\n").unwrap();
        let result = analyze_diff(
            &DiffRequest::new(&repository_path)
                .with_reference("HEAD")
                .with_width(ExecutionWidth::fixed(2).unwrap()),
        )
        .unwrap();
        assert_eq!(result.report().files().len(), 2);
        assert_eq!(result.report().scopes().len(), 4);
        assert_eq!(result.report().root(), Some(ScopeId::from_index(0)));
        assert!(!result.report().comparisons().is_empty());
        assert_eq!(result.stats().source_reads, 2);
    }

    #[test]
    fn diff_path_selection_matches_repository_relative_paths() {
        let root = repository();
        let repository_path = root.path().join("repo");
        fs::create_dir_all(repository_path.join("src")).unwrap();
        fs::rename(
            repository_path.join("sample.rs"),
            repository_path.join("src/sample.rs"),
        )
        .unwrap();
        git(&repository_path, ["add", "."]);
        git(&repository_path, ["commit", "-qm", "move"]);
        fs::write(
            repository_path.join("src/sample.rs"),
            "fn work(value: i32) -> i32 { value + value }\n",
        )
        .unwrap();
        let result =
            analyze_diff(&DiffRequest::new(repository_path.join("src")).with_reference("HEAD"))
                .unwrap();
        assert_eq!(result.report().files().len(), 1);
        assert_eq!(result.report().files()[0].path(), "src/sample.rs");
    }

    #[test]
    fn diff_uses_unchanged_edges_to_find_an_introduced_package_cycle() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        for package in ["app", "core"] {
            fs::create_dir_all(repository_path.join(package)).unwrap();
            fs::write(repository_path.join(package).join("package.json"), "{}").unwrap();
        }
        fs::create_dir_all(repository_path.join("app/src")).unwrap();
        fs::write(
            repository_path.join("app/src/a.js"),
            "import core from '../../core/b';\nfunction app() {}\n",
        )
        .unwrap();
        fs::write(repository_path.join("core/b.js"), "function core() {}\n").unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "initial"]);
        fs::write(
            repository_path.join("core/b.js"),
            "import app from '../app/src/a';\nfunction core() {}\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        assert!(
            result
                .report()
                .architecture_comparisons()
                .iter()
                .any(|comparison| {
                    comparison.kind()
                        == smackdebt_analysis::ArchitectureComparisonKind::CycleIntroduced
                        && comparison.direction() == smackdebt_analysis::ComparisonDirection::Worse
                })
        );
        assert_eq!(result.stats().inventory_walks, 1);
        assert_eq!(result.stats().source_reads, 2);
    }

    #[test]
    fn selected_package_diff_uses_an_outside_change_for_incoming_cycle_evidence() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        for package in ["app", "core"] {
            fs::create_dir_all(repository_path.join(package)).unwrap();
            fs::write(repository_path.join(package).join("package.json"), "{}").unwrap();
        }
        fs::create_dir_all(repository_path.join("app/src")).unwrap();
        fs::write(
            repository_path.join("app/src/a.js"),
            "import core from '../../core/b';\nfunction app() {}\n",
        )
        .unwrap();
        fs::write(repository_path.join("core/b.js"), "function core() {}\n").unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "initial"]);
        fs::write(
            repository_path.join("core/b.js"),
            "import app from '../app/src/a';\nfunction core() {}\n",
        )
        .unwrap();

        for selected in ["app", "app/src", "app/src/a.js"] {
            let result = analyze_diff(
                &DiffRequest::new(repository_path.join(selected)).with_reference("HEAD"),
            )
            .unwrap();
            let scope = result.selected_scope().unwrap();
            assert_eq!(result.report().scopes()[scope.index()].name(), selected);
            let comparisons: Vec<_> = result.report().scopes()[scope.index()]
                .architecture_comparisons()
                .iter()
                .map(|id| &result.report().architecture_comparisons()[id.index()])
                .collect();
            assert!(comparisons.iter().any(|value| value.kind()
                == smackdebt_analysis::ArchitectureComparisonKind::CycleIntroduced));
            assert!(
                comparisons.iter().any(|value| value.kind()
                    == smackdebt_analysis::ArchitectureComparisonKind::EdgeAdded)
            );
        }
    }

    #[test]
    fn diff_uses_each_sides_alias_configuration_without_extra_git_processes() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        for package in ["app", "core"] {
            fs::create_dir_all(repository_path.join(package)).unwrap();
            fs::write(repository_path.join(package).join("package.json"), "{}").unwrap();
        }
        fs::write(
            repository_path.join("app/a.ts"),
            "import core from '@core/value';\nfunction app() {}\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("core/value.ts"),
            "export default function core() {}\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("tsconfig.json"),
            r#"{"compilerOptions":{"paths":{"@core/*":["core/*"]}}}"#,
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "initial"]);
        fs::write(
            repository_path.join("tsconfig.json"),
            r#"{"compilerOptions":{"paths":{"@core/*":["app/*"]}}}"#,
        )
        .unwrap();
        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        assert!(result.report().architecture_comparisons().iter().any(
            |value| value.kind() == smackdebt_analysis::ArchitectureComparisonKind::EdgeRemoved
        ));
        assert_eq!(result.stats().git_processes, 5);
    }

    #[test]
    fn added_package_manifests_change_package_cycle_identity_between_sides() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}").unwrap();
        for package in ["app", "core"] {
            fs::create_dir_all(repository_path.join(package)).unwrap();
        }
        fs::write(
            repository_path.join("app/a.js"),
            "import core from '../core/b';\nfunction app() {}\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("core/b.js"),
            "import app from '../app/a';\nfunction core() {}\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "initial"]);
        for package in ["app", "core"] {
            fs::write(repository_path.join(package).join("package.json"), "{}").unwrap();
        }
        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let cycle = result
            .report()
            .architecture_comparisons()
            .iter()
            .find(|value| {
                value.kind() == smackdebt_analysis::ArchitectureComparisonKind::CycleIntroduced
            })
            .unwrap();
        assert_eq!(cycle.witness().first(), cycle.witness().last());
    }

    #[test]
    fn diff_applies_rename_identity_before_architecture_comparison() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}").unwrap();
        fs::write(
            repository_path.join("main.js"),
            "import value from './old';\nfunction main() { return value; }\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("old.js"),
            "export default function value() {}\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "initial"]);
        fs::rename(
            repository_path.join("old.js"),
            repository_path.join("new.js"),
        )
        .unwrap();
        fs::write(
            repository_path.join("main.js"),
            "import value from './new';\nfunction main() { return value; }\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        assert!(result.report().architecture_comparisons().is_empty());
        assert_eq!(result.report().dependency_edges().len(), 1);
    }

    #[test]
    fn diff_covers_committed_staged_unstaged_renamed_deleted_and_untracked_files() {
        let root = repository();
        let repository_path = root.path().join("repo");
        for name in [
            "committed.rs",
            "staged.rs",
            "unstaged.rs",
            "old.rs",
            "deleted.rs",
        ] {
            fs::write(
                repository_path.join(name),
                format!("fn {}() {{}}\n", name.replace('.', "_")),
            )
            .unwrap();
        }
        git(&repository_path, ["add", "."]);
        git(&repository_path, ["commit", "-qm", "add fixture files"]);
        fs::write(
            repository_path.join("committed.rs"),
            "fn committed() { if true {} }\n",
        )
        .unwrap();
        git(&repository_path, ["add", "committed.rs"]);
        git(&repository_path, ["commit", "-qm", "change committed file"]);
        fs::write(
            repository_path.join("staged.rs"),
            "fn staged() { if true {} }\n",
        )
        .unwrap();
        git(&repository_path, ["add", "staged.rs"]);
        fs::write(
            repository_path.join("unstaged.rs"),
            "fn unstaged() { if true {} }\n",
        )
        .unwrap();
        fs::rename(
            repository_path.join("old.rs"),
            repository_path.join("renamed.rs"),
        )
        .unwrap();
        fs::remove_file(repository_path.join("deleted.rs")).unwrap();
        fs::write(repository_path.join("untracked.rs"), "fn untracked() {}\n").unwrap();

        let result = analyze_diff(
            &DiffRequest::new(&repository_path)
                .with_reference("HEAD~1")
                .with_width(ExecutionWidth::fixed(3).unwrap()),
        )
        .unwrap();
        let paths: Vec<_> = result
            .report()
            .files()
            .iter()
            .map(FileRecord::path)
            .collect();
        for expected in [
            "committed.rs",
            "staged.rs",
            "unstaged.rs",
            "renamed.rs",
            "deleted.rs",
            "untracked.rs",
        ] {
            assert!(paths.contains(&expected), "missing {expected}: {paths:?}");
        }
        assert!(result.stats().git_processes <= 6);
    }
}
#[cfg(feature = "evidence-stats")]
#[test]
fn live_evidence_snapshot_observes_analysis_started_after_the_snapshot() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("package.json"),
        "{\"name\":\"live-evidence\",\"private\":true}\n",
    )
    .unwrap();
    fs::write(
        root.path().join("main.js"),
        "export function measured(value) { return value; }\n",
    )
    .unwrap();
    crate::evidence::reset();
    let before = crate::evidence::snapshot();

    analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();

    let after = crate::evidence::snapshot();
    let delta = after.since(before);
    assert!(delta.inventory_walks() >= 1);
    assert!(delta.inventory_visits() > 0);
    assert!(delta.source_reads() >= 1);
    assert!(delta.parser_visits() >= 1);
    assert!(delta.algorithm_passes() >= 3);
    assert!(delta.git_processes() > 0);
}
