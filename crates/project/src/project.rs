//! Repository use cases. This crate is the only place that composes discovery,
//! parsers, Git, health policy, and parallel execution.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use rayon::prelude::*;
use smackdebt_analysis::{
    ArchitectureComparison, ArchitectureComparisonId, ArchitectureFinding, ArchitectureFindingId,
    ArchitectureFindingKind, ArchitectureGraph, ArchitectureReportFacts, ChangeGraph,
    ChangeLeakageFinding, Comparison, ComparisonId, ConnectionGraph, ContributorId, CoreSize,
    Coverage, DependencyCoverage, DependencyEdge, DependencyEdgeId, DependencySyntax,
    DependencySyntaxState, Diagnostic, DiagnosticId, DiagnosticKind, DirectoryTree,
    EvolutionAccumulator, ExternalDependency, FileActivity, FileAnalysis, FileDebt, FileId,
    FileReach, FileRecord, Finding, FindingId, GraphConfigurationFailure, GraphEvidence,
    HealthAssessment, HealthCounts, HealthPolicy, HistoryAvailability, HistoryChangeFact,
    HistoryCommitFact, HistoryCoverage, HistoryWindow, HotspotPolicy, Language, ModuleDeclaration,
    OrphanCandidate, OrphanFile, PackageClosure, PackageContainment, PackageEdge, PackageEdgeId,
    PackageFileReach, PackageGraphMeasurement, PackageId, PackageRecord, ParseStatus, Rating,
    Report, ReportBuilder as AnalysisReportBuilder, ReportMode, ResolutionDiagnostic,
    ResolutionIssueKind, Scope, ScopeId, ScopeKind, SizeFinding, SizePolicy, SourceCoverageOutcome,
    SourceRole, SourceTrust, StableDependencyFinding, change_leakage, close_over_packages,
    compare_architecture, compare_units, cycle_witness, dependency_degree, enters_connection_graph,
    enters_file_graph, file_reaches, graph_file_count, orphan_files, reach_in_counts,
    stable_dependency_findings, strongly_connected_components, test_declared_files,
};
use smackdebt_discovery::{
    DiscoveredFile, Inventory, discover_snapshot, generic_source_roles, glob_matches,
    has_generated_javascript_name, has_vendored_javascript_name, is_runtime_javascript_path,
    is_source_path, is_tool_configuration_name,
};
use smackdebt_git::{Change, ContributorIdentity, GitRepository};
use smackdebt_languages::{AnalysisError as LanguageError, Analyzer};

use crate::requests::{
    CodebaseRequest, DiffRequest, ExecutionWidth, ProjectError, ProjectReport, SourceRoleRule,
    WorkStats,
};

const PARALLEL_FILE_CUTOVER: usize = 100;
const RETAINED_RELATION_LOCATIONS: usize = 3;

/// Analyzes the selected codebase.
pub(super) fn analyze_codebase(request: &CodebaseRequest) -> Result<ProjectReport, ProjectError> {
    let selection = Selection::resolve(&request.path, request.automatic_scope)?;
    // A selection with a repository behind it is a drill-down into that
    // repository's one report: the walk covers the repository so the
    // resolution index, the dependency graph, and the history are the ones a
    // root run measures, and the selection decides only which scope is
    // answered. A path with no repository behind it has nothing to drill into,
    // so its walk stays the tree it named.
    let inventory = if selection.repository {
        Inventory::discover_sources(&selection.inventory_root, request.excludes.clone())
    } else {
        Inventory::discover_selected_sources(
            &selection.inventory_root,
            &selection.discovery_root,
            request.excludes.clone(),
        )
    }
    .map_err(|source| ProjectError::Inspect {
        path: selection.walk_root().to_path_buf(),
        source,
    })?;
    #[cfg(feature = "evidence-stats")]
    crate::evidence::record_inventory(inventory.visited_entries());
    let aliases = load_resolution_aliases(&selection.inventory_root, &inventory);
    let candidates: Vec<&DiscoveredFile> = inventory.source_files().collect();
    if !request.automatic_scope
        && !candidates
            .iter()
            .any(|file| selection.includes(file.path().as_path()))
    {
        return Err(ProjectError::NoSourceFiles(request.path.clone()));
    }
    let work = AnalysisWork::default();
    let mut analyses = analyze_current_files(
        &inventory,
        &candidates,
        request.width,
        request.policy,
        &request.role_rules,
        &work,
    )?;
    let candidate_paths: Vec<_> = candidates
        .iter()
        .map(|file| file.path().as_path())
        .collect();
    demote_test_declared_roles(
        &candidate_paths,
        &mut analyses,
        &aliases,
        &request.role_rules,
    );

    // The roles history evidence is filed under. They are read here, before the
    // dependency graph exists, so the dormancy rule below cannot have run yet -
    // and it never needs to have. Dormancy requires that no commit inside the
    // window touched the file, so a dormant file contributes no change, no
    // churn row, and no coupling pair for a later role to correct. The two
    // tables agree by construction rather than by being kept in step; the
    // pinning test is `a_dormant_file_has_no_history_row_left_under_the_old_role`.
    let history_files = candidates
        .iter()
        .enumerate()
        .zip(&analyses)
        .map(|((index, file), result)| {
            let (role, trust) = match result {
                FileResult::Analyzed(rated) => (rated.role, rated.analysis.parse_status().trust()),
                FileResult::Unsupported { role, .. } | FileResult::Failed { role, .. } => {
                    (*role, SourceTrust::Failed)
                }
                FileResult::RoleConflict { .. } => {
                    unreachable!("role conflicts stop composition")
                }
            };
            (
                file.path().as_path().to_path_buf(),
                FileId::from_index(index),
                file.package(),
                role,
                trust,
            )
        })
        .collect::<Vec<_>>();
    // The one directory tree of this report, built over every candidate in
    // file order and unfiltered, which is the identity it reads. It outlives
    // history streaming on purpose: pair accumulation borrows it here, and the
    // scope join reads the same tree when the report is composed below, so a
    // second tree is never built and the two can never disagree.
    let directories =
        DirectoryTree::from_file_paths(candidate_paths.iter().map(|path| path.to_string_lossy()));
    let history = load_evolution(
        &selection.inventory_root,
        request.history_days,
        &history_files,
        &directories,
    );
    let mut builder = CodebaseReportBuilder::new(
        selection.label,
        &inventory,
        &candidates,
        &history.activity,
        aliases,
        history.evolution,
        SignalPolicies {
            hotspots: request.hotspots,
            size: request.size,
        },
    );
    for path in inventory.nested_checkouts() {
        builder.add_general_diagnostic(
            DiagnosticKind::NestedRepository,
            format!("{path} is a nested repository"),
        );
    }
    if let Some(message) = history.diagnostic {
        builder.add_general_diagnostic(DiagnosticKind::Other, message);
    }
    for (file_index, analysis) in analyses.into_iter().enumerate() {
        builder.add_analysis(file_index, analysis);
    }
    let report = builder.finish(&work, &directories);
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
    };
    let selected_scope =
        Some(selected_scope.ok_or_else(|| ProjectError::NoSourceFiles(request.path.clone()))?);
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
    let base = repository
        .merge_base(&reference, "HEAD")
        .map_err(|error| match error {
            // Git reports an unknown ref through a failed command, so the
            // failure is restated as the fixable value the user supplied.
            smackdebt_git::GitError::Command { .. } | smackdebt_git::GitError::MissingObject(_) => {
                ProjectError::UnknownReference(reference.clone())
            }
            other => ProjectError::Git(other),
        })?;
    let changed = repository.changes_from(&base)?;
    let inventory =
        Inventory::discover_sources(repository.root(), Vec::new()).map_err(|source| {
            ProjectError::Inspect {
                path: repository.root().to_path_buf(),
                source,
            }
        })?;
    #[cfg(feature = "evidence-stats")]
    crate::evidence::record_inventory(inventory.visited_entries());
    let aliases = load_resolution_aliases(repository.root(), &inventory);

    let path_filter = (!request.automatic_scope)
        .then(|| diff_filter(repository.root(), &request.path))
        .flatten();
    if !request.automatic_scope && request.path.is_file() && !is_source_path(&request.path) {
        return Err(ProjectError::NotSourceFile(request.path.clone()));
    }
    let work = AnalysisWork::default();
    let width = request.width.threads().min(changed.len().max(1));
    let mut batch = repository.object_reader(width * 2)?;
    let base_tree_files = batch.tree_files(&base)?;
    let mut base_metadata: BTreeMap<PathBuf, Result<Vec<u8>, String>> = BTreeMap::new();
    let base_inventory = discover_snapshot(repository.root(), &base_tree_files, |path| {
        if let Some(source) = base_metadata.get(path) {
            return source.clone().map_err(std::io::Error::other);
        }
        let source = batch
            .read_path(&base, path)
            .map_err(|error| error.to_string());
        base_metadata.insert(path.to_path_buf(), source.clone());
        source.map_err(std::io::Error::other)
    })
    .map_err(|source| ProjectError::Inspect {
        path: repository.root().to_path_buf(),
        source,
    })?;
    if !request.automatic_scope {
        let includes = |path: &Path| {
            path_filter
                .as_ref()
                .is_some_and(|selected| path.starts_with(selected))
        };
        let has_selected_source = inventory
            .source_files()
            .any(|file| includes(file.path().as_path()))
            || base_inventory
                .source_paths()
                .iter()
                .any(|path| includes(path));
        if !has_selected_source {
            return Err(ProjectError::NoSourceFiles(request.path.clone()));
        }
    }
    let current_sources = inventory
        .source_files()
        .map(|file| file.path().as_path().to_path_buf())
        .collect::<BTreeSet<_>>();
    let base_sources = base_inventory
        .source_paths()
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let changed_paths = changed
        .iter()
        .flat_map(|change| {
            [
                change.current_path().to_path_buf(),
                change.base_path().to_path_buf(),
            ]
        })
        .collect::<BTreeSet<_>>();
    let mut changed = changed
        .into_iter()
        .filter_map(|change| SelectedChange::from_git(change, &current_sources, &base_sources))
        .collect::<Vec<_>>();
    for path in current_sources.symmetric_difference(&base_sources) {
        if !changed_paths.contains(path.as_path()) {
            changed.push(SelectedChange::from_snapshot(
                path.clone(),
                current_sources.contains(path),
                base_sources.contains(path),
            ));
        }
    }
    changed.sort_by(|left, right| left.current_path().cmp(right.current_path()));
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
    let width = request.width.threads().min(changed_count.max(1));
    let before_package_roots = base_inventory
        .packages()
        .iter()
        .map(|(root, _)| root.clone())
        .collect::<Vec<_>>();
    let mut resolution_config_candidates: Vec<_> = inventory
        .packages()
        .iter()
        .filter_map(|package| package.resolution_config())
        .map(|path| path.as_path().to_path_buf())
        .collect();
    resolution_config_candidates.extend(base_inventory.resolution_configs().iter().cloned());
    for change in &all_changed {
        for path in [change.current_path(), change.base_path()] {
            if matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some("tsconfig.json" | "jsconfig.json")
            ) {
                resolution_config_candidates.push(path.to_path_buf());
            }
        }
    }
    resolution_config_candidates.sort();
    resolution_config_candidates.dedup();
    let before_aliases = load_base_resolution_aliases(
        &mut batch,
        &base,
        &before_package_roots,
        &resolution_config_candidates,
    );
    let mut base_only_roots = before_package_roots.clone();
    base_only_roots.retain(|root| !current_package_roots.contains(root));
    base_only_roots.sort();
    base_only_roots.dedup();
    let mut package_roots = current_package_roots.clone();
    package_roots.extend(base_only_roots.iter().cloned());
    let before_manifest_names = package_roots
        .iter()
        .map(|root| {
            base_inventory
                .packages()
                .iter()
                .find(|(candidate, _)| candidate == root)
                .and_then(|(_, name)| name.clone())
        })
        .collect::<Vec<_>>();
    let mut hierarchy = HierarchyBuilder::new(".".to_owned(), &package_roots);
    let package_records: Vec<_> = package_roots
        .iter()
        .enumerate()
        .map(|(index, root)| {
            let id = PackageId::from_index(index);
            let scope = hierarchy.package_scopes[index];
            let path = report_package_path(root);
            let record = if index < current_package_roots.len() {
                PackageRecord::current(id, scope, path)
            } else {
                PackageRecord::base_only(id, scope, path)
            };
            record.with_manifest_name(declared_manifest_name(&inventory, root))
        })
        .collect();
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
    let mut results = analyze_diff_inputs(
        changed,
        repository.root().to_path_buf(),
        base,
        batch,
        DiffAnalysisPolicy {
            health: request.policy,
            roles: request.role_rules.clone(),
        },
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
    let mut unchanged = analyze_current_files(
        &inventory,
        &unchanged_candidates,
        request.width,
        request.policy,
        &request.role_rules,
        &work,
    )?;
    let mut before_unchanged_roles = unchanged.iter().map(file_result_role).collect::<Vec<_>>();
    for side in [DiffSideSelector::Current, DiffSideSelector::Before] {
        let side_aliases = match side {
            DiffSideSelector::Current => &aliases,
            DiffSideSelector::Before => &before_aliases,
        };
        demote_test_declared_diff_roles(
            side,
            changed_count,
            &mut results,
            &unchanged_candidates,
            &mut unchanged,
            &mut before_unchanged_roles,
            DiffRolePolicy {
                aliases: side_aliases,
                rules: &request.role_rules,
            },
        );
    }
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
    let mut indexes = DiffIndexes::default();
    let mut current_dependencies = Vec::new();
    let mut before_dependencies = Vec::new();
    let mut current_files = Vec::with_capacity(selected_count);
    let mut before_files = Vec::with_capacity(selected_count);
    for result in results {
        let file_id = FileId::from_index(result.index);
        if let DiffSide::Analyzed { analysis, role, .. } = &result.current {
            current_dependencies.push(SourceDependencies {
                file: file_id,
                path: result.change.current_path().to_path_buf(),
                references: analysis.dependencies().to_vec(),
                role: *role,
                trust: analysis.parse_status().trust(),
                language: analysis.language(),
                // A diff never classifies vendored source, so the fact that
                // rule reads is not carried across the object boundary.
                module_syntax: false,
            });
        }
        if let DiffSide::Analyzed { analysis, role, .. } = &result.before {
            before_dependencies.push(SourceDependencies {
                file: file_id,
                path: result.change.base_path().to_path_buf(),
                references: analysis.dependencies().to_vec(),
                role: *role,
                trust: analysis.parse_status().trust(),
                language: analysis.language(),
                module_syntax: false,
            });
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
        let before_package = result.change.base_exists().then(|| {
            let root = nearest_package_root(result.change.base_path(), &before_package_roots);
            PackageId::from_index(
                package_roots
                    .iter()
                    .position(|candidate| candidate == &root)
                    .unwrap_or(0),
            )
        });
        current_files.push(diff_side_file_record(
            file_id,
            scope_id,
            result.change.current_path(),
            result.change.current_exists().then_some(package),
            &result.current,
        ));
        before_files.push(diff_side_file_record(
            file_id,
            scope_id,
            result.change.base_path(),
            before_package,
            &result.before,
        ));
        add_diff_result(
            &mut builder,
            result,
            &mut indexes,
            scope_id,
            package,
            is_selected,
            request.policy,
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
        match &result {
            FileResult::Analyzed(rated) => {
                record = record.with_language(rated.analysis.language());
                record =
                    record.with_source_state(rated.role, rated.analysis.parse_status().clone());
                let dependencies = SourceDependencies {
                    file: file_id,
                    path: file.path().as_path().to_path_buf(),
                    references: rated.analysis.dependencies().to_vec(),
                    role: rated.role,
                    trust: rated.analysis.parse_status().trust(),
                    language: rated.analysis.language(),
                    module_syntax: rated.module_syntax,
                };
                current_dependencies.push(dependencies.clone());
                before_dependencies.push(SourceDependencies {
                    role: before_unchanged_roles[offset],
                    ..dependencies
                });
            }
            FileResult::Unsupported { language, role } => {
                record = record
                    .with_language(*language)
                    .with_source_state(*role, ParseStatus::Failed);
            }
            FileResult::Failed { role, language, .. } => {
                record = record
                    .with_language(*language)
                    .with_source_state(*role, ParseStatus::Failed);
            }
            FileResult::RoleConflict { .. } => unreachable!("role conflicts stop composition"),
        }
        current_files.push(record.clone());
        builder.add_file(record);
        let before_package_root =
            nearest_package_root(file.path().as_path(), &before_package_roots);
        let before_package = PackageId::from_index(
            package_roots
                .iter()
                .position(|root| root == &before_package_root)
                .unwrap_or(0),
        );
        let mut before_record = FileRecord::new(
            file_id,
            package_scope,
            file.path().to_string(),
            Coverage::default(),
            HealthCounts::default(),
        )
        .with_package(before_package);
        match &result {
            FileResult::Analyzed(rated) => {
                before_record = before_record
                    .with_language(rated.analysis.language())
                    .with_source_state(
                        before_unchanged_roles[offset],
                        rated.analysis.parse_status().clone(),
                    );
            }
            FileResult::Unsupported { language, role }
            | FileResult::Failed { language, role, .. } => {
                before_record = before_record.with_language(*language).with_source_state(
                    before_unchanged_roles.get(offset).copied().unwrap_or(*role),
                    ParseStatus::Failed,
                );
            }
            FileResult::RoleConflict { .. } => unreachable!("role conflicts stop composition"),
        }
        before_files.push(before_record);
    }
    let current_manifest_names = manifest_names_for(&inventory, &package_roots);
    let current_manifest_paths = manifest_paths_for(&inventory, &package_roots);
    let current_architecture = build_architecture(
        &work,
        &current_files,
        &current_dependencies,
        &aliases,
        PackageTables {
            side_roots: &current_package_roots,
            roots: &package_roots,
            manifests: ManifestFacts {
                names: &current_manifest_names,
                paths: &current_manifest_paths,
            },
        },
        // A diff answers what two trees say about the changed units. Neither
        // tree carries the per-file window activity the vendored rule reads, so
        // the rule stands down and both sides keep the roles their names and
        // markers state.
        WindowedHistory::Absent,
    );
    let before_architecture = build_architecture(
        &work,
        &before_files,
        &before_dependencies,
        &before_aliases,
        PackageTables {
            side_roots: &before_package_roots,
            roots: &package_roots,
            manifests: ManifestFacts {
                names: &before_manifest_names,
                // The base tree is read from Git objects, which the walk never
                // opens, so no manifest of that tree was read.
                paths: &[],
            },
        },
        WindowedHistory::Absent,
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
    architecture_comparisons.retain(|comparison| {
        matches!(
            comparison.kind(),
            smackdebt_analysis::ArchitectureComparisonKind::CycleIntroduced
                | smackdebt_analysis::ArchitectureComparisonKind::CycleRemoved
        )
    });
    append_relation_comparisons(
        &mut architecture_comparisons,
        &before_architecture.file_edges,
        &current_architecture.file_edges,
        &before_files,
        &current_files,
    );
    for comparison in &mut architecture_comparisons {
        if comparison.relation().is_some() {
            continue;
        }
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
    let history = load_evolution(
        repository.root(),
        request.history_days,
        &history_files,
        &directories,
    );
    let history_diagnostic = history.diagnostic.clone();
    let current_explanation_pairs = current_architecture.explanation_pairs.clone();
    let before_explanation_pairs = before_architecture.explanation_pairs.clone();
    work.record_algorithm_pass();
    let containment = PackageContainment::from_paths(
        &package_roots
            .iter()
            .map(|root| report_package_path(root))
            .collect::<Vec<_>>(),
    );
    // A diff answers about a change rather than about a tree, so the
    // amplification the same stream accumulated is dropped here rather than
    // joined onto a scope.
    let (evolution, _) = history.evolution.accumulator.finish(
        history.evolution.coverage,
        builder.files().len(),
        package_roots.len(),
        &containment,
        &current_explanation_pairs,
        Some(&before_explanation_pairs),
    );
    let evolutionary_findings = evolution.findings().to_vec();
    let evolutionary_comparisons = evolution.comparisons().to_vec();
    let history_comparison_suppressions = evolution.comparison_suppressions().to_vec();
    let (current_leakage_candidates, current_leakage, suppressed_leakage) =
        leakage_findings(&current_architecture, &evolution, &current_files);
    let (before_leakage_candidates, _, _) =
        leakage_findings(&before_architecture, &evolution, &before_files);
    let (propagation_comparisons, propagation_suppression) = propagation_comparisons(
        &current_architecture,
        &before_architecture,
        &current_package_roots,
        &before_package_roots,
        &package_roots,
    );
    let (core_comparisons, core_suppression) =
        core_comparisons(&current_architecture, &before_architecture);
    let leakage_comparison_evidence = LeakageComparisonEvidence {
        pairs: evolution.file_coupling(),
        current: &current_architecture,
        base: &before_architecture,
        current_files: &current_files,
        base_files: &before_files,
    };
    let (change_leakage_comparisons, leakage_suppression) = leakage_comparisons(
        &current_leakage_candidates,
        &before_leakage_candidates,
        &leakage_comparison_evidence,
    );
    let current_graph_evidence =
        current_architecture
            .graph_evidence
            .clone()
            .with_suppressed(0, 0, suppressed_leakage);
    let current_closures = current_architecture.package_closures.clone();
    let current_file_reach = current_architecture.file_reach.clone();
    let current_core_size = current_architecture.core_size;
    let current_core_members = if current_architecture.core_size.is_some() {
        current_architecture.core_members.clone()
    } else {
        Vec::new()
    };
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
    builder.set_graph_evidence(current_graph_evidence);
    builder.set_diff_graph_evidence(smackdebt_analysis::DiffGraphEvidence::new(
        current_architecture.graph_evidence.clone(),
        before_architecture.graph_evidence.clone(),
        propagation_suppression,
        core_suppression,
        leakage_suppression,
    ));
    builder.set_propagation(
        current_closures,
        current_file_reach,
        current_core_size,
        current_core_members,
    );
    builder.set_change_leakage_findings(current_leakage);
    builder.set_impact_comparisons(
        propagation_comparisons.clone(),
        core_comparisons.clone(),
        change_leakage_comparisons.clone(),
    );
    builder.set_comparison_ref(reference.clone());
    builder.set_evolution(evolution);
    builder.set_explanation_pairs(current_explanation_pairs);
    if let Some(message) = history_diagnostic {
        let id = DiagnosticId::from_index(builder.diagnostic_count());
        builder.add_diagnostic(Diagnostic::new(id, None, DiagnosticKind::Other, message, 0));
    }
    for finding in evolutionary_findings {
        let pair = finding.coupling();
        builder.link_evolutionary_finding(root, finding.id());
        builder
            .link_evolutionary_finding(package_records[pair.left().index()].scope(), finding.id());
        builder
            .link_evolutionary_finding(package_records[pair.right().index()].scope(), finding.id());
    }
    for comparison in evolutionary_comparisons {
        builder.link_evolutionary_comparison(root, comparison.id());
        let pair = comparison.coupling();
        builder.link_evolutionary_comparison(
            package_records[pair.left().index()].scope(),
            comparison.id(),
        );
        builder.link_evolutionary_comparison(
            package_records[pair.right().index()].scope(),
            comparison.id(),
        );
    }
    for suppression in history_comparison_suppressions {
        builder.link_history_comparison_suppression(root, suppression.id());
        builder.link_history_comparison_suppression(
            package_records[suppression.left().index()].scope(),
            suppression.id(),
        );
        builder.link_history_comparison_suppression(
            package_records[suppression.right().index()].scope(),
            suppression.id(),
        );
    }
    for comparison in &propagation_comparisons {
        builder.link_propagation_comparison(root, comparison.id());
        match comparison.subject() {
            smackdebt_analysis::PropagationSubject::Package { source } => {
                builder.link_propagation_comparison(
                    package_records[source.index()].scope(),
                    comparison.id(),
                );
            }
            smackdebt_analysis::PropagationSubject::File { package, source } => {
                builder.link_propagation_comparison(
                    package_records[package.index()].scope(),
                    comparison.id(),
                );
                if let Some(scope) = builder.files().get(source.index()).map(FileRecord::scope) {
                    builder.link_propagation_comparison(scope, comparison.id());
                }
            }
        }
    }
    for comparison in &core_comparisons {
        builder.link_core_comparison(root, comparison.id());
        if let Some(scope) = builder
            .files()
            .get(comparison.anchor().index())
            .map(FileRecord::scope)
        {
            builder.link_core_comparison(scope, comparison.id());
        }
    }
    for comparison in &change_leakage_comparisons {
        builder.link_change_leakage_comparison(root, comparison.id());
        for file in [comparison.left(), comparison.right()] {
            if let Some(scope) = builder.files().get(file.index()).map(FileRecord::scope) {
                builder.link_change_leakage_comparison(scope, comparison.id());
            }
        }
    }
    for id in architecture_comparison_ids {
        builder.link_architecture_comparison(root, id);
        let comparison = &architecture_comparisons_for_links[id.index()];
        for package in comparison.packages() {
            builder.link_architecture_comparison(package_records[package.index()].scope(), id);
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
            builder.link_architecture_finding(package_records[package.index()].scope(), id);
        }
        for file in finding.files() {
            let scope = builder.files()[file.index()].scope();
            builder.link_architecture_finding(scope, id);
        }
    }
    builder.set_packages(package_records);
    let report = builder.finish();
    let selected_scope = path_filter.as_ref().and_then(|path| {
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
    });
    let selected_scope = if request.automatic_scope {
        Some(root)
    } else {
        Some(selected_scope.ok_or_else(|| ProjectError::NoSourceFiles(request.path.clone()))?)
    };
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

fn append_relation_comparisons(
    comparisons: &mut Vec<ArchitectureComparison>,
    before_edges: &[DependencyEdge],
    current_edges: &[DependencyEdge],
    before_files: &[FileRecord],
    current_files: &[FileRecord],
) {
    type RelationKey = (
        PackageId,
        PackageId,
        smackdebt_analysis::StaticRelationKind,
        SourceRole,
        SourceTrust,
    );
    fn relations_by_evidence(
        edges: &[DependencyEdge],
        files: &[FileRecord],
    ) -> BTreeMap<RelationKey, (u32, (FileId, FileId))> {
        let mut values: BTreeMap<RelationKey, (u32, (FileId, FileId))> = BTreeMap::new();
        for edge in edges {
            let Some(source_package) = files[edge.source().index()].package() else {
                continue;
            };
            let Some(target_package) = files[edge.target().index()].package() else {
                continue;
            };
            values
                .entry((
                    source_package,
                    target_package,
                    edge.relation(),
                    edge.role(),
                    edge.trust(),
                ))
                .and_modify(|value| {
                    value.0 += edge.references();
                    value.1 = (edge.source(), edge.target());
                })
                .or_insert((edge.references(), (edge.source(), edge.target())));
        }
        values
    }
    let before = relations_by_evidence(before_edges, before_files);
    let current = relations_by_evidence(current_edges, current_files);
    let keys: std::collections::BTreeSet<_> =
        before.keys().chain(current.keys()).copied().collect();
    for (source_package, target_package, relation, role, trust) in keys {
        let key = (source_package, target_package, relation, role, trust);
        let before_references = before.get(&key).map_or(0, |value| value.0);
        let after_references = current.get(&key).map_or(0, |value| value.0);
        if before_references == after_references {
            continue;
        }
        let kind = if after_references > before_references {
            smackdebt_analysis::ArchitectureComparisonKind::EdgeAdded
        } else {
            smackdebt_analysis::ArchitectureComparisonKind::EdgeRemoved
        };
        let files = current
            .get(&key)
            .or_else(|| before.get(&key))
            .expect("changed relation has file evidence")
            .1;
        let id = ArchitectureComparisonId::from_index(comparisons.len());
        let packages = if source_package == target_package {
            vec![source_package]
        } else {
            vec![source_package, target_package]
        };
        comparisons.push(
            ArchitectureComparison::new(id, kind, packages)
                .with_files(vec![files.0, files.1])
                .with_relation_evidence(relation, role, trust)
                .with_reference_counts(before_references, after_references),
        );
    }
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

#[derive(Clone, Debug)]
struct SelectedChange {
    current_path: PathBuf,
    base_path: PathBuf,
    current_exists: bool,
    base_exists: bool,
}

impl SelectedChange {
    fn from_git(
        change: Change,
        current_sources: &BTreeSet<PathBuf>,
        base_sources: &BTreeSet<PathBuf>,
    ) -> Option<Self> {
        let current_exists =
            change.current_exists() && current_sources.contains(change.current_path());
        let base_exists = change.base_exists() && base_sources.contains(change.base_path());
        (current_exists || base_exists).then(|| Self {
            current_path: change.current_path().to_path_buf(),
            base_path: change.base_path().to_path_buf(),
            current_exists,
            base_exists,
        })
    }

    fn from_snapshot(path: PathBuf, current_exists: bool, base_exists: bool) -> Self {
        debug_assert!(current_exists || base_exists);
        Self {
            current_path: path.clone(),
            base_path: path,
            current_exists,
            base_exists,
        }
    }

    fn current_path(&self) -> &Path {
        &self.current_path
    }

    fn base_path(&self) -> &Path {
        &self.base_path
    }

    const fn current_exists(&self) -> bool {
        self.current_exists
    }

    const fn base_exists(&self) -> bool {
        self.base_exists
    }
}

struct DiffInput {
    index: usize,
    change: SelectedChange,
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
    change: SelectedChange,
    current: DiffSide,
    before: DiffSide,
    comparisons: Vec<Comparison>,
}

enum DiffSide {
    Missing,
    Analyzed {
        analysis: FileAnalysis,
        health: HealthCounts,
        role: SourceRole,
        size_bytes: u64,
    },
    Unsupported {
        language: Language,
        role: SourceRole,
        size_bytes: u64,
    },
    Failed {
        message: String,
        role: SourceRole,
        size_bytes: u64,
    },
    RoleConflict {
        path: PathBuf,
        roles: String,
    },
}

#[derive(Clone)]
struct DiffAnalysisPolicy {
    health: HealthPolicy,
    roles: Vec<SourceRoleRule>,
}

fn analyze_diff_inputs(
    changes: Vec<SelectedChange>,
    root_path: PathBuf,
    base: String,
    mut batch: smackdebt_git::ObjectReader,
    policy: DiffAnalysisPolicy,
    width: usize,
    work: AnalysisWork,
) -> Result<Vec<DiffResult>, ProjectError> {
    if changes.len() <= 1 {
        let mut analyzer = Analyzer::default();
        let results = changes
            .into_iter()
            .enumerate()
            .map(|(index, change)| {
                let input = read_diff_input(index, change, &root_path, &base, &mut batch, &work);
                analyze_diff_input(input, policy.health, &policy.roles, &mut analyzer, &work)
            })
            .collect::<Vec<_>>();
        if let Some((path, roles)) = results.iter().find_map(diff_role_conflict) {
            return Err(ProjectError::SourceRoleConflict { path, roles });
        }
        return Ok(results);
    }
    let worker_count = width.max(1).min(changes.len());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(worker_count)
        .build()?;
    let (input_tx, input_rx) = mpsc::sync_channel::<DiffInput>(worker_count * 2);
    let input_rx = Arc::new(Mutex::new(input_rx));
    let (result_tx, result_rx) = mpsc::channel();
    let producer_work = work.clone();
    let policy = Arc::new(policy);
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
                let policy = Arc::clone(&policy);
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
                                policy.health,
                                policy.roles.as_slice(),
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
    if let Some((path, roles)) = results.iter().find_map(diff_role_conflict) {
        return Err(ProjectError::SourceRoleConflict { path, roles });
    }
    Ok(results)
}

fn diff_role_conflict(result: &DiffResult) -> Option<(PathBuf, String)> {
    [&result.current, &result.before]
        .into_iter()
        .find_map(|side| match side {
            DiffSide::RoleConflict { path, roles } => Some((path.clone(), roles.clone())),
            _ => None,
        })
}

fn read_diff_input(
    index: usize,
    change: SelectedChange,
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
    role_rules: &[SourceRoleRule],
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
        role_rules,
        work,
    );
    let before_path = input.change.base_path();
    let before = analyze_diff_side(
        analyzer,
        file_id,
        before_path,
        input.before,
        policy,
        role_rules,
        work,
    );
    work.record_algorithm_pass();
    let mut comparisons = match (diff_units(&before), diff_units(&current)) {
        (Some(before), Some(current)) => compare_units(before, current, policy),
        _ => Vec::new(),
    };
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

fn diff_units(side: &DiffSide) -> Option<&[smackdebt_analysis::UnitFact]> {
    match side {
        DiffSide::Missing => Some(&[]),
        DiffSide::Analyzed { analysis, .. }
            if matches!(analysis.parse_status(), ParseStatus::Parsed) =>
        {
            Some(analysis.units())
        }
        _ => None,
    }
}

fn analyze_diff_side(
    analyzer: &mut Analyzer,
    file: FileId,
    path: &Path,
    input: InputSide,
    policy: HealthPolicy,
    role_rules: &[SourceRoleRule],
    work: &AnalysisWork,
) -> DiffSide {
    if let Some(error) = input.error {
        return role_for_unavailable_source(path, role_rules).map_or_else(
            |roles| DiffSide::RoleConflict {
                path: path.to_path_buf(),
                roles,
            },
            |role| DiffSide::Failed {
                message: error,
                role,
                size_bytes: 0,
            },
        );
    }
    let Some(bytes) = input.bytes else {
        return DiffSide::Missing;
    };
    let size_bytes = bytes.len() as u64;
    let role = match classify_source_role(path, &bytes, role_rules) {
        Ok(role) => role,
        Err(roles) => {
            return DiffSide::RoleConflict {
                path: path.to_path_buf(),
                roles,
            };
        }
    };
    match analyze_bytes(analyzer, file, path, bytes, work) {
        Ok(analysis) => {
            let health = rated_health(&analysis, role, policy);
            DiffSide::Analyzed {
                analysis,
                health,
                role,
                size_bytes,
            }
        }
        Err(LanguageError::Unsupported(language)) => DiffSide::Unsupported {
            language,
            role,
            size_bytes,
        },
        Err(error) => DiffSide::Failed {
            message: error.to_string(),
            role,
            size_bytes,
        },
    }
}

fn add_diff_result(
    report: &mut AnalysisReportBuilder,
    result: DiffResult,
    indexes: &mut DiffIndexes,
    scope_id: ScopeId,
    package: PackageId,
    included_in_code_diff: bool,
    policy: HealthPolicy,
) {
    let file_id = FileId::from_index(result.index);
    let before_role = result.before.role();
    let current_role = result.current.role();
    let mut has_ambiguous_identity = false;
    let mut ambiguity_affects_verdict = false;

    for comparison in result.comparisons.iter().filter(|_| included_in_code_diff) {
        let comparison_id = ComparisonId::from_index(indexes.comparison);
        let mut retained = Comparison::new(
            comparison_id,
            comparison.identity().clone(),
            comparison.kind(),
            comparison.before(),
            comparison.after(),
            comparison.before_rating(),
            comparison.after_rating(),
        )
        .with_file(file_id)
        .with_source_roles(before_role, current_role);
        if let Some(span) = comparison.span() {
            retained = retained.with_span(span);
        }
        if comparison.is_anonymous_ambiguity() {
            has_ambiguous_identity = true;
            retained = retained.with_anonymous_ambiguity();
            ambiguity_affects_verdict |= retained.affects_verdict();
        }
        report.add_comparison(retained);
        report.link_comparison(scope_id, comparison_id);
        indexes.comparison += 1;
    }
    if has_ambiguous_identity {
        let mut diagnostic = Diagnostic::new(
            DiagnosticId::from_index(report.diagnostic_count()),
            Some(file_id),
            DiagnosticKind::AmbiguousIdentity,
            "anonymous units could not be matched safely",
            0,
        );
        if !ambiguity_affects_verdict {
            diagnostic = diagnostic.as_context();
        }
        report.add_diagnostic(diagnostic);
    }

    let selected = match &result.current {
        DiffSide::Missing => &result.before,
        current => current,
    };
    if let DiffSide::Analyzed { analysis, role, .. } = selected
        && !verdict_eligible(analysis, *role)
    {
        for unit in analysis.units() {
            let assessment = policy.assess(unit.measurements());
            if assessment.rating() == Rating::Healthy {
                continue;
            }
            let id = FindingId::from_index(indexes.finding);
            report.add_finding(
                Finding::new(
                    id,
                    file_id,
                    unit.identity().clone(),
                    unit.span(),
                    unit.measurements(),
                    assessment,
                )
                .with_evidence(*role, analysis.parse_status().trust()),
            );
            report.link_finding(scope_id, id);
            indexes.finding += 1;
        }
    }
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
    if let DiffSide::Analyzed { analysis, role, .. } = selected {
        file = file.with_source_state(*role, analysis.parse_status().clone());
    } else if let DiffSide::Unsupported { role, .. } | DiffSide::Failed { role, .. } = selected {
        file = file.with_source_state(*role, ParseStatus::Failed);
    }
    report.add_file(file);
    report.link_file(scope_id, file_id);
    add_diff_diagnostic(report, file_id, &result.current, "current");
    add_diff_diagnostic(report, file_id, &result.before, "base");
}

fn diff_side_file_record(
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

#[derive(Default)]
struct DiffIndexes {
    comparison: usize,
    finding: usize,
}

fn diff_side_summary(side: &DiffSide) -> (Coverage, HealthCounts, Option<Language>) {
    match side {
        DiffSide::Missing => (
            Coverage::new(1, 0, 0, 0, 0, 0),
            HealthCounts::default(),
            None,
        ),
        DiffSide::Unsupported {
            language,
            size_bytes,
            ..
        } => (
            Coverage::classified(1, SourceCoverageOutcome::Unsupported, 0, 0)
                .with_bytes(*size_bytes, *size_bytes),
            HealthCounts::default(),
            Some(*language),
        ),
        DiffSide::Failed { size_bytes, .. } => (
            Coverage::classified(1, SourceCoverageOutcome::Failed, 0, 0).with_bytes(*size_bytes, 0),
            HealthCounts::default(),
            None,
        ),
        DiffSide::RoleConflict { .. } => (
            Coverage::classified(1, SourceCoverageOutcome::Failed, 0, 0),
            HealthCounts::default(),
            None,
        ),
        DiffSide::Analyzed {
            analysis,
            health,
            role,
            size_bytes,
        } => (
            source_coverage(analysis, *role).with_bytes(*size_bytes, 0),
            *health,
            Some(analysis.language()),
        ),
    }
}

fn source_coverage(analysis: &FileAnalysis, role: SourceRole) -> Coverage {
    let outcome = match analysis.parse_status() {
        ParseStatus::Parsed if role.affects_verdict() => SourceCoverageOutcome::Clean,
        ParseStatus::Parsed => SourceCoverageOutcome::Context,
        ParseStatus::Recovered(_) => SourceCoverageOutcome::Recovered,
        ParseStatus::Failed => SourceCoverageOutcome::Failed,
    };
    Coverage::classified(
        1,
        outcome,
        analysis.source_lines(),
        u32::from(matches!(outcome, SourceCoverageOutcome::Failed)) * analysis.source_lines(),
    )
}

fn add_diff_diagnostic(
    report: &mut AnalysisReportBuilder,
    file: FileId,
    side: &DiffSide,
    label: &str,
) {
    let (kind, message) = match side {
        DiffSide::Unsupported { language, .. } => (
            DiagnosticKind::UnsupportedLanguage,
            format!("{label} side uses unsupported language: {language:?}"),
        ),
        DiffSide::Failed { message, .. } => {
            (DiagnosticKind::Other, format!("{label} side: {message}"))
        }
        DiffSide::Analyzed { analysis, .. }
            if matches!(analysis.parse_status(), ParseStatus::Failed) =>
        {
            (
                DiagnosticKind::ParseFailure,
                format!("{label} side parser failed"),
            )
        }
        DiffSide::Analyzed { analysis, .. }
            if matches!(analysis.parse_status(), ParseStatus::Recovered(_)) =>
        {
            (
                DiagnosticKind::ParseFailure,
                format!("{label} side parser recovered from syntax errors"),
            )
        }
        DiffSide::RoleConflict { .. } => return,
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
    role_rules: &[SourceRoleRule],
    work: &AnalysisWork,
) -> Result<Vec<FileResult>, ProjectError> {
    let analyze = |analyzer: &mut Analyzer, (index, file): (usize, &&DiscoveredFile)| {
        let path = file.path().as_path();
        let language = Analyzer::language(path);
        if matches!(
            language,
            Language::Astro | Language::Kotlin | Language::Unknown
        ) {
            return match role_for_unavailable_source(path, role_rules) {
                Ok(role) => FileResult::Unsupported { language, role },
                Err(roles) => FileResult::RoleConflict {
                    path: path.to_path_buf(),
                    roles,
                },
            };
        }
        let Some(absolute) = inventory.absolute_path(file.path()) else {
            return match role_for_unavailable_source(path, role_rules) {
                Ok(role) => FileResult::Failed {
                    message: "source path escaped the selected root".to_owned(),
                    role,
                    language,
                },
                Err(roles) => FileResult::RoleConflict {
                    path: path.to_path_buf(),
                    roles,
                },
            };
        };
        match fs::read(&absolute) {
            Ok(source) => {
                work.source_reads.fetch_add(1, Ordering::Relaxed);
                #[cfg(feature = "evidence-stats")]
                crate::evidence::record_source_read();
                let role = match classify_source_role(path, &source, role_rules) {
                    Ok(role) => role,
                    Err(roles) => {
                        return FileResult::RoleConflict {
                            path: path.to_path_buf(),
                            roles,
                        };
                    }
                };
                let module_syntax = declares_module_syntax(path, &source);
                match analyze_bytes(analyzer, FileId::from_index(index), path, source, work) {
                    Ok(value) => {
                        FileResult::Analyzed(rate_file(value, role, policy, module_syntax))
                    }
                    Err(LanguageError::Unsupported(language)) => {
                        FileResult::Unsupported { language, role }
                    }
                    Err(error) => FileResult::Failed {
                        message: error.to_string(),
                        role,
                        language,
                    },
                }
            }
            Err(error) => match role_for_unavailable_source(path, role_rules) {
                Ok(role) => FileResult::Failed {
                    message: error.to_string(),
                    role,
                    language,
                },
                Err(roles) => FileResult::RoleConflict {
                    path: path.to_path_buf(),
                    roles,
                },
            },
        }
    };

    if candidates.len() < PARALLEL_FILE_CUTOVER || width.threads() == 1 {
        let mut analyzer = Analyzer::default();
        let results = candidates
            .iter()
            .enumerate()
            .map(|entry| analyze(&mut analyzer, entry))
            .collect::<Vec<_>>();
        return role_results(results);
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(width.threads())
        .build()?;
    let results = pool.install(|| {
        candidates
            .par_iter()
            .enumerate()
            .map_init(Analyzer::default, analyze)
            .collect()
    });
    role_results(results)
}

fn role_results(results: Vec<FileResult>) -> Result<Vec<FileResult>, ProjectError> {
    if let Some((path, roles)) = results.iter().find_map(|result| match result {
        FileResult::RoleConflict { path, roles } => Some((path.clone(), roles.clone())),
        _ => None,
    }) {
        Err(ProjectError::SourceRoleConflict { path, roles })
    } else {
        Ok(results)
    }
}

/// The module declarations one side of an analysis states.
///
/// A declaration is a `module_ownership` relation, so this reads the same
/// references the architecture pass resolves later, before any role is read.
fn module_declarations<'a>(
    sources: impl Iterator<Item = (usize, &'a Path, &'a [DependencySyntax])>,
    index: &BTreeMap<PathBuf, FileId>,
    aliases: &ResolutionRules,
) -> BTreeSet<ModuleDeclaration> {
    let mut declarations = BTreeSet::new();
    for (declarer, path, references) in sources {
        for reference in references {
            if reference.relation() != smackdebt_analysis::StaticRelationKind::ModuleOwnership {
                continue;
            }
            let DependencySyntaxState::Candidates(candidates) = reference.state() else {
                continue;
            };
            let test_scoped = reference.scope() == smackdebt_analysis::DependencyScope::Test;
            for target in resolve_candidates(path, candidates, index, aliases) {
                if target.index() != declarer {
                    declarations.insert((declarer, target.index(), test_scoped));
                }
            }
        }
    }
    declarations
}

/// The files a pass reclassifies because the build only compiles them when
/// `test` is set.
///
/// This resolves the module declarations of one file table, runs the
/// test-declared fixpoint over them, and drops every file explicit
/// configuration already claims — configuration keeps precedence over every
/// later rule.
fn test_declared_demotions<P: AsRef<Path>>(
    paths: &[P],
    references: &[&[DependencySyntax]],
    aliases: &ResolutionRules,
    rules: &[SourceRoleRule],
) -> BTreeSet<usize> {
    debug_assert_eq!(paths.len(), references.len());
    let index: BTreeMap<PathBuf, FileId> = paths
        .iter()
        .enumerate()
        .map(|(index, path)| (path.as_ref().to_path_buf(), FileId::from_index(index)))
        .collect();
    let declarations = module_declarations(
        paths
            .iter()
            .zip(references)
            .enumerate()
            .map(|(index, (path, references))| (index, path.as_ref(), *references)),
        &index,
        aliases,
    );
    test_declared_files(&declarations)
        .into_iter()
        .filter(|file| {
            matching_role_rules(paths[*file].as_ref(), rules)
                .next()
                .is_none()
        })
        .collect()
}

/// The roles explicit configuration states for one path, in rule order.
fn matching_role_rules<'a>(
    path: &Path,
    rules: &'a [SourceRoleRule],
) -> impl Iterator<Item = SourceRole> + 'a {
    let normalized = path.to_string_lossy().replace('\\', "/");
    rules
        .iter()
        .filter(move |rule| glob_matches(rule.pattern(), &normalized))
        .map(SourceRoleRule::role)
}

fn classify_source_role(
    path: &Path,
    source: &[u8],
    rules: &[SourceRoleRule],
) -> Result<SourceRole, String> {
    let mut explicit: Vec<_> = matching_role_rules(path, rules).collect();
    explicit.sort();
    explicit.dedup();
    if !explicit.is_empty() {
        return one_role(explicit);
    }
    if Analyzer::has_generated_marker(path, source) {
        return Ok(SourceRole::Generated);
    }
    if has_generated_javascript_name(path) || has_generated_javascript_content(path, source) {
        return Ok(SourceRole::Generated);
    }
    if has_vendored_javascript_name(path) {
        return Ok(SourceRole::Vendored);
    }
    let generic = generic_source_roles(path);
    if generic.is_empty() {
        Ok(SourceRole::Primary)
    } else {
        one_role(generic)
    }
}

/// Whether the source is written as a module rather than as a plain script.
///
/// A module states its own imports and exports, so the dependency graph can see
/// whether anything uses it: nothing importing a module is an orphan, a fact
/// the report already states. A script states nothing - a page or a build tool
/// loads it by name - so no import could ever have named it and an empty fan-in
/// is what such a file is supposed to look like. Only a script can therefore be
/// read as dormant.
///
/// The test reads the first word of each line, not every occurrence, so the
/// `module.exports` a UMD wrapper indents inside a function leaves the file the
/// script it is. That prefix reading is the whole of the rule: it is a cheap
/// approximation of a statement position, not a parse, and the `export` opening
/// a line inside a template literal counts too.
///
/// The two errors are not symmetric. Missing module syntax lets a module be
/// called dormant, which is wrong about a file the team may be working on;
/// seeing it where there is none only spares a file from a rule of absences.
/// So the accepted follow set is generous - whitespace, a brace, a star, a
/// parenthesis, either quote, a carriage return, or the end of the line, which
/// is how a multi-line `import` opens. Only the JavaScript a runtime loads as
/// written is read at all, because that is the only source the dormancy rule
/// can classify.
fn declares_module_syntax(path: &Path, source: &[u8]) -> bool {
    if !is_runtime_javascript_path(path) {
        return false;
    }
    source.split(|byte| *byte == b'\n').any(|line| {
        let line = line.trim_ascii_start();
        ["export", "import"].iter().any(|keyword| {
            line.strip_prefix(keyword.as_bytes()).is_some_and(|rest| {
                matches!(
                    rest.first(),
                    None | Some(b' ' | b'\t' | b'\r' | b'{' | b'*' | b'(' | b'"' | b'\'')
                )
            })
        })
    })
}

fn has_generated_javascript_content(path: &Path, source: &[u8]) -> bool {
    const MINIMUM_BYTES: usize = 65_536;
    const MINIMUM_BYTES_PER_NONEMPTY_LINE: usize = 512;

    let supported_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension, "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx"));
    if !supported_extension || source.len() < MINIMUM_BYTES {
        return false;
    }

    let nonempty_lines = source
        .split(|byte| *byte == b'\n')
        .filter(|line| line.iter().any(|byte| !byte.is_ascii_whitespace()))
        .count();
    nonempty_lines != 0
        && nonempty_lines
            .checked_mul(MINIMUM_BYTES_PER_NONEMPTY_LINE)
            .is_some_and(|minimum| source.len() >= minimum)
}

fn role_for_unavailable_source(
    path: &Path,
    rules: &[SourceRoleRule],
) -> Result<SourceRole, String> {
    classify_source_role(path, &[], rules)
}

fn one_role(roles: Vec<SourceRole>) -> Result<SourceRole, String> {
    if roles.len() == 1 {
        Ok(roles[0])
    } else {
        Err(roles
            .iter()
            .map(|role| format!("{role:?}").to_ascii_lowercase())
            .collect::<Vec<_>>()
            .join(", "))
    }
}

fn analyze_bytes(
    analyzer: &mut Analyzer,
    file: FileId,
    path: &Path,
    source: Vec<u8>,
    work: &AnalysisWork,
) -> Result<FileAnalysis, LanguageError> {
    let _ = file;
    let result = analyzer.analyze(path, source);
    if !matches!(result, Err(LanguageError::Unsupported(_))) {
        work.record_parser_visit();
        work.record_algorithm_pass();
    }
    result
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
    Resolved(FileId, PackageId, SourceRole, SourceTrust),
    Unusable,
}

/// The repository-relative path of every file history can name, each placed at
/// its own [`FileId`].
///
/// [`DirectoryTree`] reads a file's identity from the position it holds in the
/// sequence the tree is built from, so the paths are placed by index rather
/// than pushed in iteration order. The diff flow builds its tree from this list
/// and assembles it by filtering the report's files, so pushing them in order
/// would shift every directory lookup and every distance that follows from it,
/// with no wrong-looking value to notice. A position no history file claims
/// holds the empty path, which the tree files under the repository root and no
/// signal ever asks about.
///
/// The codebase flow does not use this: it builds its tree over the unfiltered
/// candidate walk, so every file has a real directory there and the scope join
/// can read the same tree.
fn history_directory_paths(
    files: &[(PathBuf, FileId, PackageId, SourceRole, SourceTrust)],
) -> Vec<Cow<'_, str>> {
    let count = files
        .iter()
        .map(|(_, file, _, _, _)| file.index() + 1)
        .max()
        .unwrap_or_default();
    let mut paths = vec![Cow::Borrowed(""); count];
    for (path, file, _, _, _) in files {
        paths[file.index()] = path.to_string_lossy();
    }
    paths
}

/// Streams history once, fanning every commit out to the evolution signals.
///
/// The directory tree is borrowed rather than built here: the caller owns the
/// one tree of its report, so the same tree that answers a pair's distance also
/// answers a scope's directory when the report is composed.
fn load_evolution(
    inventory_root: &Path,
    history_days: u32,
    files: &[(PathBuf, FileId, PackageId, SourceRole, SourceTrust)],
    directories: &DirectoryTree,
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
        .map(|(path, file, package, role, trust)| {
            (
                path.clone(),
                HistoryAlias::Resolved(*file, *package, *role, *trust),
            )
        })
        .collect();
    let file_paths: HashMap<FileId, PathBuf> = files
        .iter()
        .map(|(path, file, _, _, _)| (*file, path.clone()))
        .collect();
    let mut contributors = HashMap::<ContributorIdentity, ContributorId>::new();
    let mut accumulator = EvolutionAccumulator::default();
    for (_, file, package, role, trust) in files {
        accumulator.register_source(*file, *package, *role, *trust);
    }
    let mut activity = HashMap::<PathBuf, u32>::new();
    let mut textual_changes = 0u32;
    let mut uncounted_changes = 0u32;
    let mut eligible_commits = 0u32;
    let mut mapped_eligible_changes = 0u32;
    let mut context_changes = 0u32;
    let mut excluded_changes = 0u32;
    let mut rename_gaps = 0u32;
    let mut streamed_commits = 0u32;
    let mut window_excluded_commits = 0u32;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(i64::MIN, |duration| duration.as_secs() as i64);
    let window = HistoryWindow::of_days(history_days, now);
    let history = repository.stream_history(Some(window.cutoff()), |commit| {
        streamed_commits += 1;
        // The window filter runs inside the streamed history process on landed
        // dates, so out-of-window history is never streamed. This defensive
        // boundary check compares the same landed instant and keeps any
        // straggler from becoming a fact; it counts boundary rejects only.
        if !window.includes(commit.timestamp()) {
            window_excluded_commits += 1;
            return Ok(());
        }
        let next_contributor = ContributorId::from_index(contributors.len());
        let contributor = *contributors
            .entry(commit.contributor().clone())
            .or_insert(next_contributor);
        let mut changes = Vec::new();
        let mut contains_eligible_source = false;
        for change in commit.changes() {
            let path = change
                .path()
                .strip_prefix(relative_root)
                .unwrap_or(change.path());
            let identity = aliases.get(path).copied();
            let Some(HistoryAlias::Resolved(file, package, role, trust)) = identity else {
                excluded_changes += 1;
                continue;
            };
            if let Some(previous) = change.previous_path() {
                let previous = previous
                    .strip_prefix(relative_root)
                    .unwrap_or(previous)
                    .to_path_buf();
                match aliases.get(&previous) {
                    Some(HistoryAlias::Resolved(existing_file, existing_package, _, _))
                        if (*existing_file, *existing_package) != (file, package) =>
                    {
                        rename_gaps += 1;
                        aliases.insert(previous, HistoryAlias::Unusable);
                    }
                    Some(HistoryAlias::Unusable) => {}
                    _ => {
                        aliases
                            .insert(previous, HistoryAlias::Resolved(file, package, role, trust));
                    }
                }
            }
            if change.added_lines().is_some() && change.deleted_lines().is_some() {
                textual_changes += 1;
            } else {
                uncounted_changes += 1;
            }
            if let Some(path) = file_paths.get(&file) {
                *activity.entry(path.clone()).or_default() += 1;
            }
            if role.affects_verdict() && trust == SourceTrust::Trusted {
                mapped_eligible_changes += 1;
                contains_eligible_source = true;
            } else {
                context_changes += 1;
            }
            changes.push(
                HistoryChangeFact::new(file, package, change.added_lines(), change.deleted_lines())
                    .with_source_evidence(role, trust),
            );
        }
        if !changes.is_empty() {
            accumulator.accept(HistoryCommitFact::new(contributor, changes), directories);
        }
        eligible_commits += u32::from(contains_eligible_source);
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
                    eligible_commits,
                    mapped_eligible_changes,
                    context_changes,
                    summary.newest_timestamp(),
                    summary.oldest_timestamp(),
                    textual_changes,
                    uncounted_changes,
                    excluded_changes,
                    rename_gaps,
                    summary
                        .is_shallow()
                        .then(|| "repository history is shallow".to_owned()),
                )
                .with_window(window.days(), window_excluded_commits),
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
                        eligible_commits,
                        mapped_eligible_changes,
                        context_changes,
                        None,
                        None,
                        textual_changes,
                        uncounted_changes,
                        excluded_changes,
                        rename_gaps,
                        Some(reason.clone()),
                    )
                    .with_window(window.days(), window_excluded_commits),
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
    role: SourceRole,
    /// Whether the source states its own imports and exports.
    module_syntax: bool,
    health: HealthCounts,
    debt: Vec<(usize, HealthAssessment)>,
    /// Whether this file's facts may produce default signals.
    ///
    /// Only trusted source in a verdict role feeds the hotspot and size
    /// tables: recovered facts stay advisory, and fixture or generated source
    /// stays context. Retained findings are unaffected, so advisory and context
    /// debt remains visible.
    signals_verdict: bool,
    rated_units: u32,
    max_rating: Rating,
    container_statements: Vec<(String, u32)>,
}

fn rate_file(
    analysis: FileAnalysis,
    role: SourceRole,
    policy: HealthPolicy,
    module_syntax: bool,
) -> RatedFile {
    let mut health = HealthCounts::default();
    let mut debt = Vec::new();
    let mut max_rating = Rating::Healthy;
    // Container totals accumulate while units are rated, so no healthy unit is
    // retained to compute container size later.
    let mut container_statements: Vec<(String, u32)> = Vec::new();
    let signals_verdict = verdict_eligible(&analysis, role);
    for (index, unit) in analysis.units().iter().enumerate() {
        let assessment = policy.assess(unit.measurements());
        if assessment.rating() != Rating::Healthy {
            debt.push((index, assessment));
        }
        if !signals_verdict {
            continue;
        }
        health.add_rating(assessment.rating());
        if assessment.rating() > max_rating {
            max_rating = assessment.rating();
        }
        if let Some(container) = unit.identity().container() {
            let statements = unit.measurements().logical_lines();
            match container_statements
                .iter_mut()
                .find(|(name, _)| name == container)
            {
                Some(total) => total.1 += statements,
                None => container_statements.push((container.to_owned(), statements)),
            }
        }
    }
    let rated_units = if signals_verdict {
        u32::try_from(analysis.units().len()).unwrap_or(u32::MAX)
    } else {
        0
    };
    RatedFile {
        analysis,
        role,
        module_syntax,
        health,
        debt,
        signals_verdict,
        rated_units,
        max_rating,
        container_statements,
    }
}

enum FileResult {
    Analyzed(RatedFile),
    Unsupported {
        language: Language,
        role: SourceRole,
    },
    Failed {
        message: String,
        role: SourceRole,
        language: Language,
    },
    RoleConflict {
        path: PathBuf,
        roles: String,
    },
}

impl FileResult {
    fn references(&self) -> &[DependencySyntax] {
        match self {
            Self::Analyzed(rated) => rated.analysis.dependencies(),
            _ => &[],
        }
    }

    /// Reclassifies a file the build compiles only under `test`.
    ///
    /// Rating never changes: a test file is still verdict eligible, so the
    /// health and debt already computed for it stay correct.
    fn demote_to_test(&mut self) {
        let role = match self {
            Self::Analyzed(rated) => &mut rated.role,
            Self::Unsupported { role, .. } | Self::Failed { role, .. } => role,
            Self::RoleConflict { .. } => return,
        };
        *role = role.demoted_by_test_scope();
    }
}

fn file_result_role(result: &FileResult) -> SourceRole {
    match result {
        FileResult::Analyzed(rated) => rated.role,
        FileResult::Unsupported { role, .. } | FileResult::Failed { role, .. } => *role,
        FileResult::RoleConflict { .. } => unreachable!("role conflicts stop composition"),
    }
}

/// Reclassifies every file the build compiles only when `test` is set.
///
/// This runs before any role reaches the report builder, so findings, ratings,
/// coverage, history evidence, and the architecture graphs all read one role
/// per file.
fn demote_test_declared_roles(
    paths: &[&Path],
    results: &mut [FileResult],
    aliases: &ResolutionRules,
    rules: &[SourceRoleRule],
) {
    let demotions = {
        let references: Vec<&[DependencySyntax]> =
            results.iter().map(FileResult::references).collect();
        test_declared_demotions(paths, &references, aliases, rules)
    };
    for file in demotions {
        results[file].demote_to_test();
    }
}

/// Which side of a diff a classification pass reads.
#[derive(Clone, Copy, Eq, PartialEq)]
enum DiffSideSelector {
    Current,
    Before,
}

impl DiffSide {
    fn role(&self) -> Option<SourceRole> {
        match self {
            Self::Analyzed { role, .. }
            | Self::Unsupported { role, .. }
            | Self::Failed { role, .. } => Some(*role),
            Self::Missing | Self::RoleConflict { .. } => None,
        }
    }

    fn references(&self) -> &[DependencySyntax] {
        match self {
            Self::Analyzed { analysis, .. } => analysis.dependencies(),
            _ => &[],
        }
    }

    fn demote_to_test(&mut self) {
        let role = match self {
            Self::Analyzed { role, .. }
            | Self::Unsupported { role, .. }
            | Self::Failed { role, .. } => role,
            Self::Missing | Self::RoleConflict { .. } => return,
        };
        *role = role.demoted_by_test_scope();
    }
}

/// Reclassifies test-declared files on one side of a diff.
///
/// Changed and unchanged files retain a role for each tree independently.
struct DiffRolePolicy<'a> {
    aliases: &'a ResolutionRules,
    rules: &'a [SourceRoleRule],
}

fn demote_test_declared_diff_roles(
    side: DiffSideSelector,
    changed_count: usize,
    results: &mut [DiffResult],
    unchanged_candidates: &[&DiscoveredFile],
    unchanged: &mut [FileResult],
    before_unchanged_roles: &mut [SourceRole],
    policy: DiffRolePolicy<'_>,
) {
    debug_assert_eq!(results.len(), changed_count);
    debug_assert!(
        results
            .iter()
            .enumerate()
            .all(|(offset, result)| result.index == offset),
        "diff results are dense and ordered by file index"
    );
    let demotions =
        {
            let (paths, references): (Vec<PathBuf>, Vec<&[DependencySyntax]>) =
                results
                    .iter()
                    .map(|result| match side {
                        DiffSideSelector::Current => (
                            result.change.current_path().to_path_buf(),
                            result.current.references(),
                        ),
                        DiffSideSelector::Before => (
                            result.change.base_path().to_path_buf(),
                            result.before.references(),
                        ),
                    })
                    .chain(unchanged_candidates.iter().zip(unchanged.iter()).map(
                        |(file, result)| (file.path().as_path().to_path_buf(), result.references()),
                    ))
                    .unzip();
            test_declared_demotions(&paths, &references, policy.aliases, policy.rules)
        };
    for file in demotions {
        if file < changed_count {
            match side {
                DiffSideSelector::Current => results[file].current.demote_to_test(),
                DiffSideSelector::Before => results[file].before.demote_to_test(),
            }
        } else {
            let unchanged_index = file - changed_count;
            match side {
                DiffSideSelector::Current => unchanged[unchanged_index].demote_to_test(),
                DiffSideSelector::Before => {
                    before_unchanged_roles[unchanged_index] =
                        before_unchanged_roles[unchanged_index].demoted_by_test_scope();
                }
            }
        }
    }
}

fn verdict_eligible(analysis: &FileAnalysis, role: SourceRole) -> bool {
    role.affects_verdict() && matches!(analysis.parse_status(), ParseStatus::Parsed)
}

fn rated_health(analysis: &FileAnalysis, role: SourceRole, policy: HealthPolicy) -> HealthCounts {
    let mut health = HealthCounts::default();
    if verdict_eligible(analysis, role) {
        for unit in analysis.units() {
            health.add_rating(policy.assess(unit.measurements()).rating());
        }
    }
    health
}

struct Selection {
    /// The root every recorded path is relative to.
    inventory_root: PathBuf,
    /// The tree walked when no repository stands behind the selection.
    discovery_root: PathBuf,
    exact_file: Option<PathBuf>,
    prefix: Option<PathBuf>,
    label: String,
    /// Whether a repository stands behind the selection, which is what makes
    /// the selection a scope of one repository report rather than a report of
    /// its own.
    repository: bool,
}

impl Selection {
    fn resolve(path: &Path, automatic_scope: bool) -> Result<Self, ProjectError> {
        let absolute = std::path::absolute(path).map_err(|source| ProjectError::Inspect {
            path: path.to_path_buf(),
            source,
        })?;
        if !automatic_scope && absolute.is_file() && !is_source_path(&absolute) {
            return Err(ProjectError::NotSourceFile(path.to_path_buf()));
        }
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
            // The report is the repository however little of it is answered,
            // so its root scope carries the repository's own name.
            return Ok(Self {
                inventory_root: root,
                discovery_root: selected_absolute,
                exact_file,
                prefix,
                label: ".".to_owned(),
                repository: true,
            });
        }
        if absolute.is_file() {
            let root = absolute.parent().unwrap_or(Path::new(".")).to_path_buf();
            let exact_file = absolute.file_name().map(PathBuf::from);
            return Ok(Self {
                inventory_root: root,
                discovery_root: absolute,
                exact_file,
                prefix: None,
                label: path.display().to_string(),
                repository: false,
            });
        }
        Ok(Self {
            inventory_root: absolute.clone(),
            discovery_root: absolute,
            exact_file: None,
            prefix: None,
            label: path.display().to_string(),
            repository: false,
        })
    }

    /// The tree the inventory walk covers, which the reader is shown if the
    /// walk fails.
    fn walk_root(&self) -> &Path {
        if self.repository {
            &self.inventory_root
        } else {
            &self.discovery_root
        }
    }

    fn includes(&self, path: &Path) -> bool {
        self.exact_file.as_deref().map_or_else(
            || {
                self.prefix
                    .as_deref()
                    .is_none_or(|prefix| path.starts_with(prefix))
            },
            |file| path == file,
        )
    }
}

struct CodebaseReportBuilder<'a> {
    scopes: Vec<Scope>,
    files: Vec<FileRecord>,
    findings: Vec<Finding>,
    diagnostics: Vec<Diagnostic>,
    file_scopes: Vec<ScopeId>,
    file_sizes: Vec<u64>,
    activity: &'a HashMap<PathBuf, u32>,
    package_ids: Vec<PackageId>,
    dependencies: Vec<SourceDependencies>,
    aliases: ResolutionRules,
    package_roots: Vec<PathBuf>,
    manifest_names: Vec<Option<String>>,
    manifest_paths: Vec<Vec<String>>,
    packages: Vec<PackageRecord>,
    evolution: EvolutionInput,
    file_debt: Vec<FileDebt>,
    size_findings: Vec<SizeFinding>,
    policies: SignalPolicies,
}

/// The policies that own the derived signal tables.
#[derive(Clone, Copy, Debug, Default)]
struct SignalPolicies {
    hotspots: HotspotPolicy,
    size: SizePolicy,
}

impl<'a> CodebaseReportBuilder<'a> {
    fn new(
        label: String,
        inventory: &Inventory,
        candidates: &[&DiscoveredFile],
        activity: &'a HashMap<PathBuf, u32>,
        aliases: ResolutionRules,
        evolution: EvolutionInput,
        policies: SignalPolicies,
    ) -> Self {
        let package_roots: Vec<PathBuf> = inventory
            .packages()
            .iter()
            .map(|package| package.root().as_path().to_path_buf())
            .collect();
        let manifest_names: Vec<_> = inventory
            .packages()
            .iter()
            .map(|package| package.manifest_name().map(str::to_owned))
            .collect();
        let manifest_paths: Vec<_> = inventory
            .packages()
            .iter()
            .map(|package| package.declared_paths().to_vec())
            .collect();
        let mut hierarchy = HierarchyBuilder::new(label, &package_roots);
        let packages = package_roots
            .iter()
            .enumerate()
            .map(|(index, root)| {
                PackageRecord::current(
                    PackageId::from_index(index),
                    hierarchy.package_scopes[index],
                    report_package_path(root),
                )
                .with_manifest_name(manifest_names[index].clone())
            })
            .collect();
        let mut package_ids = Vec::with_capacity(candidates.len());
        for file in candidates {
            hierarchy.add_file(file.path().as_path(), file.package().index());
            package_ids.push(file.package());
        }
        let file_scopes = candidates
            .iter()
            .map(|file| hierarchy.file_scopes[file.path().as_path()])
            .collect();
        let file_sizes = candidates.iter().map(|file| file.size_bytes()).collect();

        Self {
            scopes: hierarchy.scopes,
            files: Vec::with_capacity(candidates.len()),
            findings: Vec::with_capacity(candidates.len()),
            diagnostics: Vec::with_capacity(candidates.len()),
            file_scopes,
            file_sizes,
            activity,
            package_ids,
            dependencies: Vec::with_capacity(candidates.len()),
            aliases,
            package_roots,
            manifest_names,
            manifest_paths,
            packages,
            evolution,
            file_debt: Vec::with_capacity(candidates.len()),
            size_findings: Vec::with_capacity(candidates.len()),
            policies,
        }
    }

    fn add_analysis(&mut self, index: usize, result: FileResult) {
        let file_id = FileId::from_index(index);
        let scope_id = self.file_scopes[index];
        let size_bytes = self.file_sizes[index];
        let path = self.scopes[scope_id.index()].name().to_owned();
        let touches = self.activity.get(Path::new(&path)).copied();
        let mut health = HealthCounts::default();
        let mut rated_units = 0;
        let mut max_rating = Rating::Healthy;
        let (coverage, language) = match result {
            FileResult::Analyzed(mut rated) => {
                health = rated.health;
                rated_units = rated.rated_units;
                max_rating = rated.max_rating;
                if rated.signals_verdict {
                    self.size_findings.extend(
                        self.policies
                            .size
                            .rate_file(file_id, rated.analysis.source_lines()),
                    );
                }
                let mut containers = std::mem::take(&mut rated.container_statements);
                containers.sort_by(|left, right| left.0.cmp(&right.0));
                for (container, statements) in containers {
                    self.size_findings.extend(
                        self.policies
                            .size
                            .rate_container(file_id, &container, statements),
                    );
                }
                for (unit_index, assessment) in rated.debt {
                    let unit = &rated.analysis.units()[unit_index];
                    let finding_id = FindingId::from_index(self.findings.len());
                    self.scopes[scope_id.index()].add_finding(finding_id);
                    self.findings.push(
                        Finding::new(
                            finding_id,
                            file_id,
                            unit.identity().clone(),
                            unit.span(),
                            unit.measurements(),
                            assessment,
                        )
                        .with_evidence(rated.role, rated.analysis.parse_status().trust()),
                    );
                }
                let analysis = rated.analysis;
                self.dependencies.push(SourceDependencies {
                    file: file_id,
                    path: PathBuf::from(&path),
                    references: analysis.dependencies().to_vec(),
                    role: rated.role,
                    trust: analysis.parse_status().trust(),
                    language: analysis.language(),
                    module_syntax: rated.module_syntax,
                });
                let recovered = matches!(analysis.parse_status(), ParseStatus::Recovered(_));
                let failed = matches!(analysis.parse_status(), ParseStatus::Failed);
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
                    source_coverage(&analysis, rated.role).with_bytes(size_bytes, 0),
                    Some((
                        analysis.language(),
                        rated.role,
                        analysis.parse_status().clone(),
                    )),
                )
            }
            FileResult::Unsupported { language, role } => {
                self.add_diagnostic(
                    file_id,
                    DiagnosticKind::UnsupportedLanguage,
                    format!("{path} uses an unsupported language"),
                    0,
                );
                (
                    Coverage::classified(1, SourceCoverageOutcome::Unsupported, 0, 0)
                        .with_bytes(size_bytes, size_bytes),
                    Some((language, role, ParseStatus::Failed)),
                )
            }
            FileResult::Failed {
                message,
                role,
                language,
            } => {
                self.add_diagnostic(
                    file_id,
                    DiagnosticKind::UnreadableFile,
                    format!("{path}: {message}"),
                    0,
                );
                (
                    Coverage::classified(1, SourceCoverageOutcome::Failed, 0, 0)
                        .with_bytes(size_bytes, 0),
                    Some((language, role, ParseStatus::Failed)),
                )
            }
            FileResult::RoleConflict { .. } => unreachable!("role conflicts stop composition"),
        };
        let mut file = FileRecord::new(file_id, scope_id, path, coverage, health);
        file = file.with_package(self.package_ids[index]);
        if let Some((language, role, status)) = language {
            file = file.with_language(language).with_source_state(role, status);
        }
        if let Some(touches) = touches {
            file = file.with_activity(FileActivity::new(touches));
        }
        self.file_debt.push(FileDebt::new(
            file_id,
            rated_units,
            max_rating,
            touches.unwrap_or(0),
        ));
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

    /// Discloses every package whose file closure the node limit skipped.
    ///
    /// The skip is a fact the machine report owes its reader: the package has a
    /// file reach and this report does not state it.
    fn disclose_skipped_closures(&mut self, skipped: &[PackageId]) {
        for package in skipped {
            let path = report_package_path(&self.package_roots[package.index()]);
            self.add_general_diagnostic(
                DiagnosticKind::PropagationSkipped,
                format!("{path} holds more files than one closure may reach over"),
            );
        }
    }

    /// Restates every file the architecture pass proved dormant.
    ///
    /// The role is settled after these tables were filled, so each one is
    /// brought to the answer the file would have carried had its role been
    /// known when it was rated: the file drops the health it contributed, its
    /// findings are re-evidenced so no verdict, card, or offender counts them,
    /// its size findings go with the verdict they were raised for, and the
    /// hotspot table stops seeing rated units. The findings themselves stay,
    /// because a dormant file is still inspectable on its own.
    ///
    /// The churn and coupling tables are deliberately not restated. They were
    /// filled while history streamed, before this role existed - and they hold
    /// nothing to restate, because a file with any row in them was touched
    /// inside the window and so can never be dormant. The pinning test is
    /// `a_dormant_file_has_no_history_row_left_under_the_old_role`.
    fn restate_dormant_files(&mut self, dormant: &BTreeSet<FileId>) {
        if dormant.is_empty() {
            return;
        }
        for file in dormant {
            let record = &mut self.files[file.index()];
            *record = record.clone().in_context_role(SourceRole::Dormant);
            let debt = &mut self.file_debt[file.index()];
            *debt = FileDebt::new(*file, 0, Rating::Healthy, debt.touches());
        }
        for finding in &mut self.findings {
            if dormant.contains(&finding.file()) {
                *finding = finding
                    .clone()
                    .with_evidence(SourceRole::Dormant, finding.trust());
            }
        }
        self.size_findings
            .retain(|finding| !dormant.contains(&finding.file()));
    }

    fn add_general_diagnostic(&mut self, kind: DiagnosticKind, message: impl Into<String>) {
        let id = DiagnosticId::from_index(self.diagnostics.len());
        self.diagnostics
            .push(Diagnostic::new(id, None, kind, message, 0));
    }

    /// Composes the report, joining the scope facts that need the one
    /// directory tree this report streamed its history against: the tree is
    /// borrowed rather than rebuilt, so the scope a fact is stated at and the
    /// directory it was accumulated under can never disagree.
    fn finish(mut self, work: &AnalysisWork, directories: &DirectoryTree) -> Report {
        let architecture = build_architecture(
            work,
            &self.files,
            &self.dependencies,
            &self.aliases,
            PackageTables {
                side_roots: &self.package_roots,
                roots: &self.package_roots,
                manifests: ManifestFacts {
                    names: &self.manifest_names,
                    paths: &self.manifest_paths,
                },
            },
            if self.evolution.coverage.availability() == HistoryAvailability::Unavailable
                || self.evolution.coverage.commits() == 0
            {
                WindowedHistory::Absent
            } else {
                WindowedHistory::Streamed
            },
        );
        self.restate_dormant_files(&architecture.dormant);
        let architecture_findings_for_links = architecture.findings.clone();
        let explanation_pairs = architecture.explanation_pairs.clone();
        self.disclose_skipped_closures(&architecture.skipped_closures);
        work.record_algorithm_pass();
        let containment = PackageContainment::from_paths(
            &self
                .package_roots
                .iter()
                .map(|root| report_package_path(root))
                .collect::<Vec<_>>(),
        );
        let (evolution, amplification) = self.evolution.accumulator.finish(
            self.evolution.coverage,
            self.files.len(),
            self.package_roots.len(),
            &containment,
            &explanation_pairs,
            None,
        );
        // The dependency join runs once, here, where the retained pairs and the
        // graphs both exist: history streamed before the graph was built, so
        // pair accumulation was graph-blind and this is the first point at
        // which a pair can be asked what depends on what.
        let (_, change_leakage_findings, suppressed_leakage) =
            leakage_findings(&architecture, &evolution, &self.files);
        let suppressed_reach = architecture
            .package_closures
            .iter()
            .filter(|closure| {
                !architecture
                    .graph_evidence
                    .package_is_complete(closure.package())
            })
            .count() as u32
            + u32::from(
                !architecture.graph_evidence.is_complete()
                    && architecture
                        .measurements
                        .iter()
                        .map(|value| value.reach_in())
                        .max()
                        .and_then(|reach| {
                            smackdebt_analysis::PropagationReach::packages(
                                reach,
                                architecture.measurements.len() as u32,
                            )
                        })
                        .is_some(),
            );
        let suppressed_core = u32::from(
            !architecture.graph_evidence.is_complete() && architecture.core_size.is_some(),
        );
        let graph_evidence = architecture.graph_evidence.clone().with_suppressed(
            suppressed_reach,
            suppressed_core,
            suppressed_leakage,
        );
        // The scope join runs once, here, where the one directory tree the
        // histograms were filed under is still in scope: a rendered scope then
        // reads a table position rather than a tree.
        let scope_amplification =
            smackdebt_analysis::scope_amplification(&self.scopes, directories, &amplification);
        let evolutionary_findings = evolution.findings().to_vec();
        let root = ScopeId::from_index(0);
        let mut builder = AnalysisReportBuilder::with_capacity(
            ReportMode::Codebase,
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
        builder.set_graph_evidence(graph_evidence);
        builder.set_propagation(
            architecture.package_closures,
            architecture.file_reach,
            architecture.core_size,
            if architecture.core_size.is_some() {
                architecture.core_members
            } else {
                Vec::new()
            },
        );
        builder.set_scope_amplification(scope_amplification);
        builder.set_hotspots(self.policies.hotspots.hotspots(&self.file_debt));
        builder.set_orphan_files(architecture.orphans);
        builder.set_stable_dependency_findings(architecture.stable_dependencies);
        builder.set_size_findings(self.size_findings);
        builder.set_evolution(evolution);
        builder.set_change_leakage_findings(change_leakage_findings);
        builder.set_explanation_pairs(explanation_pairs);
        for finding in evolutionary_findings {
            let pair = finding.coupling();
            builder.link_evolutionary_finding(root, finding.id());
            builder.link_evolutionary_finding(
                self.packages[pair.left().index()].scope(),
                finding.id(),
            );
            builder.link_evolutionary_finding(
                self.packages[pair.right().index()].scope(),
                finding.id(),
            );
        }
        for (index, finding) in architecture.finding_links {
            builder.link_architecture_finding(index, finding);
        }
        for finding in &architecture_findings_for_links {
            for package in finding.packages() {
                builder.link_architecture_finding(
                    self.packages[package.index()].scope(),
                    finding.id(),
                );
            }
            for file in finding.files() {
                builder
                    .link_architecture_finding(builder.files()[file.index()].scope(), finding.id());
            }
        }
        builder.set_packages(self.packages);
        builder.finish()
    }
}

/// Joins the retained file pairs with the graphs the architecture build
/// carried out, which is the whole of what the change-leakage capability adds
/// to composition: no table is walked twice and no graph is rebuilt.
fn leakage_findings(
    architecture: &ArchitectureBuild,
    evolution: &smackdebt_analysis::EvolutionaryReportFacts,
    files: &[FileRecord],
) -> (Vec<ChangeLeakageFinding>, Vec<ChangeLeakageFinding>, u32) {
    let candidates = change_leakage(
        evolution.file_coupling(),
        &ChangeGraph::new(
            &architecture.cycle_pairs,
            &architecture.connections,
            &architecture.graph_packages,
            files,
        ),
    );
    let before = candidates.len();
    let findings = candidates
        .iter()
        .copied()
        .filter(|finding| {
            leakage_evidence_is_complete(
                &architecture.graph_evidence,
                evolution.file_coupling()[finding.coupling().index()],
                files,
            )
        })
        .collect::<Vec<_>>();
    let suppressed = before.saturating_sub(findings.len()) as u32;
    (candidates, findings, suppressed)
}

/// Whether the code a leakage finding names was read completely enough to
/// state it.
///
/// A finding is about two files, so the gate asks about their two packages,
/// whichever rule decided it. A package every import of which resolved cannot
/// be hiding the dependency that would explain a co-change inside it, and a
/// package that could not be read might be.
///
/// The hidden-coupling rule proves its absence over the whole connection
/// graph, so a hole in a third package could in principle carry a path the
/// proof never saw. Withholding every finding because some unrelated corner of
/// the repository could not be read states nothing at all, and the pair's own
/// two packages are what a reader checks the claim against, so that is what
/// this asks. The withheld findings are counted, so what the gate dropped
/// stays visible.
fn leakage_evidence_is_complete(
    evidence: &GraphEvidence,
    pair: smackdebt_analysis::FileChangeCoupling,
    files: &[FileRecord],
) -> bool {
    [pair.left(), pair.right()].into_iter().all(|file| {
        files[file.index()]
            .package()
            .is_some_and(|package| evidence.package_is_complete(package))
    })
}

fn fraction_direction(
    before: (u32, u32),
    after: (u32, u32),
) -> Option<smackdebt_analysis::ComparisonDirection> {
    if before == after {
        return None;
    }
    let before_cross = u64::from(before.0) * u64::from(after.1);
    let after_cross = u64::from(after.0) * u64::from(before.1);
    Some(match after_cross.cmp(&before_cross) {
        std::cmp::Ordering::Greater => smackdebt_analysis::ComparisonDirection::Worse,
        std::cmp::Ordering::Less => smackdebt_analysis::ComparisonDirection::Better,
        std::cmp::Ordering::Equal => smackdebt_analysis::ComparisonDirection::Changed,
    })
}

fn propagation_comparisons(
    current: &ArchitectureBuild,
    before: &ArchitectureBuild,
    current_roots: &[PathBuf],
    before_roots: &[PathBuf],
    package_roots: &[PathBuf],
) -> (
    Vec<smackdebt_analysis::PropagationComparison>,
    smackdebt_analysis::ComparisonSuppression,
) {
    let mut comparisons = Vec::new();
    let mut suppression = smackdebt_analysis::ComparisonSuppression::default();
    let mut package_subjects = [
        package_reach_subject(current, current_roots.len() as u32),
        package_reach_subject(before, before_roots.len() as u32),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    package_subjects.sort_unstable();
    package_subjects.dedup();
    for package in package_subjects {
        let root = &package_roots[package.index()];
        if !current_roots.contains(root) || !before_roots.contains(root) {
            continue;
        }
        let current_reach = current
            .measurements
            .iter()
            .find(|measurement| measurement.package() == package)
            .map(|measurement| measurement.reach_in());
        let before_reach = before
            .measurements
            .iter()
            .find(|measurement| measurement.package() == package)
            .map(|measurement| measurement.reach_in());
        let (Some(current_reach), Some(before_reach)) = (current_reach, before_reach) else {
            continue;
        };
        let before_counts = (before_reach, before_roots.len() as u32);
        let after_counts = (current_reach, current_roots.len() as u32);
        let material =
            smackdebt_analysis::PropagationReach::packages(before_counts.0, before_counts.1)
                .is_some()
                || smackdebt_analysis::PropagationReach::packages(after_counts.0, after_counts.1)
                    .is_some();
        let Some(direction) = material
            .then(|| fraction_direction(before_counts, after_counts))
            .flatten()
        else {
            continue;
        };
        let current_incomplete = !current.graph_evidence.is_complete();
        let base_incomplete = !before.graph_evidence.is_complete();
        if current_incomplete || base_incomplete {
            suppression.record(current_incomplete, base_incomplete);
        } else {
            comparisons.push(smackdebt_analysis::PropagationComparison::new(
                smackdebt_analysis::PropagationComparisonId::from_index(comparisons.len()),
                smackdebt_analysis::PropagationSubject::Package { source: package },
                direction,
                before_counts,
                after_counts,
            ));
        }
    }

    let mut file_subjects = current
        .package_closures
        .iter()
        .chain(&before.package_closures)
        .map(|closure| (closure.package(), closure.source()))
        .collect::<Vec<_>>();
    file_subjects.sort_unstable();
    file_subjects.dedup();
    for (package, source) in file_subjects {
        append_file_reach_comparison(
            FileReachComparisonInput {
                current,
                before,
                package,
                source,
            },
            &mut comparisons,
            &mut suppression,
        );
    }
    (comparisons, suppression)
}

fn package_reach_subject(architecture: &ArchitectureBuild, packages: u32) -> Option<PackageId> {
    let winner = architecture.measurements.iter().max_by(|left, right| {
        left.reach_in()
            .cmp(&right.reach_in())
            .then_with(|| right.package().cmp(&left.package()))
    })?;
    smackdebt_analysis::PropagationReach::packages(winner.reach_in(), packages)
        .map(|_| winner.package())
}

struct FileReachComparisonInput<'a> {
    current: &'a ArchitectureBuild,
    before: &'a ArchitectureBuild,
    package: PackageId,
    source: FileId,
}

fn append_file_reach_comparison(
    input: FileReachComparisonInput<'_>,
    comparisons: &mut Vec<smackdebt_analysis::PropagationComparison>,
    suppression: &mut smackdebt_analysis::ComparisonSuppression,
) {
    let value = |architecture: &ArchitectureBuild| {
        architecture
            .package_file_reach
            .iter()
            .find(|value| value.package() == input.package && value.source() == input.source)
            .copied()
    };
    let (Some(current_value), Some(before_value)) = (value(input.current), value(input.before))
    else {
        return;
    };
    let before_counts = (before_value.reach(), before_value.files());
    let after_counts = (current_value.reach(), current_value.files());
    let Some(direction) = fraction_direction(before_counts, after_counts) else {
        return;
    };
    let current_incomplete = !input
        .current
        .graph_evidence
        .package_is_complete(input.package);
    let base_incomplete = !input
        .before
        .graph_evidence
        .package_is_complete(input.package);
    if current_incomplete || base_incomplete {
        suppression.record(current_incomplete, base_incomplete);
        return;
    }
    comparisons.push(smackdebt_analysis::PropagationComparison::new(
        smackdebt_analysis::PropagationComparisonId::from_index(comparisons.len()),
        smackdebt_analysis::PropagationSubject::File {
            package: input.package,
            source: input.source,
        },
        direction,
        before_counts,
        after_counts,
    ));
}

fn core_comparisons(
    current: &ArchitectureBuild,
    before: &ArchitectureBuild,
) -> (
    Vec<smackdebt_analysis::CoreComparison>,
    smackdebt_analysis::ComparisonSuppression,
) {
    if current.core_size.is_none() && before.core_size.is_none() {
        return (
            Vec::new(),
            smackdebt_analysis::ComparisonSuppression::default(),
        );
    }
    let anchors = core_comparison_anchors(current, before);
    let mut comparisons = Vec::new();
    let mut suppression = smackdebt_analysis::ComparisonSuppression::default();
    for anchor in anchors {
        append_core_comparison(current, before, anchor, &mut comparisons, &mut suppression);
    }
    (comparisons, suppression)
}

fn core_comparison_anchors(current: &ArchitectureBuild, before: &ArchitectureBuild) -> Vec<FileId> {
    let shared = current
        .core_members
        .iter()
        .find(|file| before.core_members.contains(file))
        .copied();
    if let Some(anchor) = shared {
        return vec![anchor];
    }
    let mut anchors = Vec::new();
    if before.core_size.is_some()
        && let Some(anchor) = before
            .core_members
            .iter()
            .find(|file| graph_contains(current, **file))
    {
        anchors.push(*anchor);
    }
    if current.core_size.is_some()
        && let Some(anchor) = current
            .core_members
            .iter()
            .find(|file| graph_contains(before, **file))
    {
        anchors.push(*anchor);
    }
    anchors.sort_unstable();
    anchors.dedup();
    anchors
}

fn graph_contains(architecture: &ArchitectureBuild, file: FileId) -> bool {
    architecture
        .graph_packages
        .get(file.index())
        .is_some_and(Option::is_some)
}

fn component_containing(architecture: &ArchitectureBuild, anchor: FileId) -> Option<&[FileId]> {
    architecture
        .file_components
        .iter()
        .find(|component| component.contains(&anchor))
        .map(Vec::as_slice)
}

fn append_core_comparison(
    current: &ArchitectureBuild,
    before: &ArchitectureBuild,
    anchor: FileId,
    comparisons: &mut Vec<smackdebt_analysis::CoreComparison>,
    suppression: &mut smackdebt_analysis::ComparisonSuppression,
) {
    let (Some(current_members), Some(before_members)) = (
        component_containing(current, anchor),
        component_containing(before, anchor),
    ) else {
        return;
    };
    let before_counts = (before_members.len() as u32, before.file_graph_count);
    let after_counts = (current_members.len() as u32, current.file_graph_count);
    let material = CoreSize::from_counts(before_counts.0, before_counts.1).is_some()
        || CoreSize::from_counts(after_counts.0, after_counts.1).is_some();
    if !material || (before_counts == after_counts && before_members == current_members) {
        return;
    }
    let current_incomplete = !current.graph_evidence.is_complete();
    let base_incomplete = !before.graph_evidence.is_complete();
    if current_incomplete || base_incomplete {
        suppression.record(current_incomplete, base_incomplete);
        return;
    }
    let direction = fraction_direction(before_counts, after_counts)
        .unwrap_or(smackdebt_analysis::ComparisonDirection::Changed);
    comparisons.push(smackdebt_analysis::CoreComparison::new(
        smackdebt_analysis::CoreComparisonId::from_index(comparisons.len()),
        anchor,
        direction,
        (before_counts.0, before_counts.1, before_members.to_vec()),
        (after_counts.0, after_counts.1, current_members.to_vec()),
    ));
}

fn leakage_comparisons(
    current: &[ChangeLeakageFinding],
    before: &[ChangeLeakageFinding],
    evidence: &LeakageComparisonEvidence<'_>,
) -> (
    Vec<smackdebt_analysis::ChangeLeakageComparison>,
    smackdebt_analysis::ComparisonSuppression,
) {
    type Key = (
        smackdebt_analysis::ChangeLeakageKind,
        smackdebt_analysis::FileChangeCouplingId,
        Option<FileId>,
    );
    let current: BTreeSet<Key> = current
        .iter()
        .map(|finding| (finding.kind(), finding.coupling(), finding.interface()))
        .collect();
    let before: BTreeSet<Key> = before
        .iter()
        .map(|finding| (finding.kind(), finding.coupling(), finding.interface()))
        .collect();
    let mut comparisons = Vec::new();
    let mut suppression = smackdebt_analysis::ComparisonSuppression::default();
    for key in current.symmetric_difference(&before) {
        let &(kind, coupling, _) = key;
        let pair = evidence.pairs[coupling.index()];
        let (current_incomplete, base_incomplete) = evidence.incomplete_sides(pair);
        if current_incomplete || base_incomplete {
            suppression.record(current_incomplete, base_incomplete);
            continue;
        }
        let direction = if current.contains(key) {
            smackdebt_analysis::ComparisonDirection::Worse
        } else {
            smackdebt_analysis::ComparisonDirection::Better
        };
        comparisons.push(smackdebt_analysis::ChangeLeakageComparison::new(
            smackdebt_analysis::ChangeLeakageComparisonId::from_index(comparisons.len()),
            kind,
            pair.left(),
            pair.right(),
            direction,
        ));
    }
    (comparisons, suppression)
}

struct LeakageComparisonEvidence<'a> {
    pairs: &'a [smackdebt_analysis::FileChangeCoupling],
    current: &'a ArchitectureBuild,
    base: &'a ArchitectureBuild,
    current_files: &'a [FileRecord],
    base_files: &'a [FileRecord],
}

impl LeakageComparisonEvidence<'_> {
    fn incomplete_sides(&self, pair: smackdebt_analysis::FileChangeCoupling) -> (bool, bool) {
        (
            !leakage_evidence_is_complete(&self.current.graph_evidence, pair, self.current_files),
            !leakage_evidence_is_complete(&self.base.graph_evidence, pair, self.base_files),
        )
    }
}

struct ArchitectureBuild {
    coverage: DependencyCoverage,
    graph_evidence: GraphEvidence,
    orphans: Vec<OrphanFile>,
    stable_dependencies: Vec<StableDependencyFinding>,
    file_edges: Vec<DependencyEdge>,
    package_edges: Vec<PackageEdge>,
    external: Vec<ExternalDependency>,
    diagnostics: Vec<ResolutionDiagnostic>,
    measurements: Vec<PackageGraphMeasurement>,
    findings: Vec<ArchitectureFinding>,
    finding_links: Vec<(ScopeId, ArchitectureFindingId)>,
    cycles: Vec<smackdebt_analysis::PackageCycle>,
    /// Cross-package pairs that explain change coupling, whether or not they
    /// enter a verdict graph.
    explanation_pairs: BTreeSet<(PackageId, PackageId)>,
    /// How far a change reaches inside each package whose value is material.
    package_closures: Vec<PackageClosure>,
    /// Each computed file value behind the selected package closure subjects.
    package_file_reach: Vec<PackageFileReach>,
    /// The packages whose closure the node limit skipped, which the machine
    /// report discloses rather than leaving silently absent.
    skipped_closures: Vec<PackageId>,
    /// The exact repository-wide reach of the bounded candidate set.
    file_reach: Vec<FileReach>,
    /// The largest file dependency cycle, when it is material.
    core_size: Option<CoreSize>,
    core_members: Vec<FileId>,
    file_components: Vec<Vec<FileId>>,
    file_graph_count: u32,
    /// The imports that enter the file dependency cycle graph, which the
    /// leaky-interface rule reads.
    cycle_pairs: Vec<(usize, usize)>,
    /// The wider graph the hidden-coupling rule proves absence against, with
    /// its package closure derived once.
    connections: ConnectionGraph,
    /// The package of every file that enters the file dependency graph.
    graph_packages: Vec<Option<PackageId>>,
    /// The files the resolved relations and the streamed window together
    /// proved nobody is working on, which the caller restates in the tables it
    /// owns.
    dormant: BTreeSet<FileId>,
}

#[derive(Clone)]
struct SourceDependencies {
    file: FileId,
    path: PathBuf,
    references: Vec<DependencySyntax>,
    role: SourceRole,
    trust: SourceTrust,
    language: Language,
    /// Whether the file is written as a module rather than a plain script.
    module_syntax: bool,
}

type DependencyEdgeKey = (
    FileId,
    FileId,
    smackdebt_analysis::StaticRelationKind,
    SourceRole,
    SourceTrust,
);
type DependencyEdgeValue = (u32, Vec<smackdebt_analysis::SourceSpan>);
type ExternalDependencyKey = (
    FileId,
    String,
    smackdebt_analysis::StaticRelationKind,
    SourceRole,
    SourceTrust,
);
type ResolutionDiagnosticKey = (
    FileId,
    String,
    ResolutionIssueKind,
    String,
    smackdebt_analysis::StaticRelationKind,
    SourceRole,
    SourceTrust,
);

#[derive(Clone, Copy)]
enum RelationResolution {
    ResolvedInternal,
    UnresolvedInternal,
    AmbiguousInternal,
    External,
    UnresolvedPackage,
    /// An internal reference to a file the source languages never analyze.
    AssetReference,
}

#[derive(Default)]
struct DependencyPartitionCounts {
    resolved_internal_uses: u32,
    unresolved_internal_uses: u32,
    ambiguous_internal_uses: u32,
    external_uses: u32,
    unresolved_package_uses: u32,
    module_ownership_relations: u32,
    context_relations: u32,
}

impl DependencyPartitionCounts {
    fn record(
        &mut self,
        relation: smackdebt_analysis::StaticRelationKind,
        role: SourceRole,
        trust: SourceTrust,
        resolution: RelationResolution,
    ) {
        *self.partition(relation, role, trust, resolution) += 1;
    }

    /// The single partition one relation belongs to.
    ///
    /// Every relation lands in exactly one counter, so the partitions add up
    /// to the references the parse found and none is counted twice.
    fn partition(
        &mut self,
        relation: smackdebt_analysis::StaticRelationKind,
        role: SourceRole,
        trust: SourceTrust,
        resolution: RelationResolution,
    ) -> &mut u32 {
        if trust != SourceTrust::Trusted || !role.affects_verdict() {
            return &mut self.context_relations;
        }
        if relation == smackdebt_analysis::StaticRelationKind::ModuleOwnership {
            return &mut self.module_ownership_relations;
        }
        match resolution {
            RelationResolution::ResolvedInternal => &mut self.resolved_internal_uses,
            RelationResolution::UnresolvedInternal => &mut self.unresolved_internal_uses,
            RelationResolution::AmbiguousInternal => &mut self.ambiguous_internal_uses,
            RelationResolution::External => &mut self.external_uses,
            RelationResolution::UnresolvedPackage => &mut self.unresolved_package_uses,
            // An asset reference explains the file it was written in without
            // ever joining the verdict graph, so it stays in the partition
            // that holds relations kept as context.
            RelationResolution::AssetReference => &mut self.context_relations,
        }
    }

    fn finish(self) -> DependencyCoverage {
        DependencyCoverage::new(
            self.resolved_internal_uses,
            self.unresolved_internal_uses,
            self.ambiguous_internal_uses,
            self.external_uses,
            self.unresolved_package_uses,
            self.module_ownership_relations,
            self.context_relations,
        )
    }
}

/// What one unmatched reference means after the manifest-name index is read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ManifestReference {
    /// No unique internal package declares the referenced name.
    Absent,
    /// Several internal packages declare the referenced name.
    Ambiguous,
    /// The matched package presents this file as its entry point.
    Entry(FileId),
    /// The matched package has no resolvable entry file.
    Package(PackageId),
}

/// The tables one architecture pass fills while it partitions references.
#[derive(Default)]
struct ReferenceTables {
    coverage: DependencyPartitionCounts,
    edge_values: BTreeMap<DependencyEdgeKey, DependencyEdgeValue>,
    external_values: BTreeMap<ExternalDependencyKey, DependencyEdgeValue>,
    diagnostic_values: BTreeMap<ResolutionDiagnosticKey, DependencyEdgeValue>,
    internal_issue_files: BTreeSet<FileId>,
    manifest_package_values:
        BTreeMap<(PackageId, PackageId), (std::collections::BTreeSet<FileId>, u32)>,
    /// Manifest-name package pairs that explain change coupling without
    /// entering a verdict graph, such as a dev-dependency test import.
    manifest_explanation_pairs: BTreeSet<(PackageId, PackageId)>,
}

/// The role one reference carries as evidence.
///
/// A reference declared under a test configuration is at least test code, so a
/// primary file's `#[cfg(test)]` imports become test evidence while a file that
/// already carries a later role keeps it.
fn evidence_role(reference: &DependencySyntax, dependencies: &SourceDependencies) -> SourceRole {
    if reference.scope() == smackdebt_analysis::DependencyScope::Test {
        dependencies.role.demoted_by_test_scope()
    } else {
        dependencies.role
    }
}

impl ReferenceTables {
    fn record_internal(
        &mut self,
        source: FileId,
        target: FileId,
        reference: &DependencySyntax,
        dependencies: &SourceDependencies,
    ) {
        let role = evidence_role(reference, dependencies);
        self.coverage.record(
            reference.relation(),
            role,
            dependencies.trust,
            RelationResolution::ResolvedInternal,
        );
        if source == target {
            return;
        }
        let value = self
            .edge_values
            .entry((
                source,
                target,
                reference.relation(),
                role,
                dependencies.trust,
            ))
            .or_default();
        value.0 += 1;
        if value.1.len() < RETAINED_RELATION_LOCATIONS {
            value.1.push(reference.span());
        }
    }

    fn record_external(
        &mut self,
        source: FileId,
        reference: &DependencySyntax,
        dependencies: &SourceDependencies,
    ) {
        let role = evidence_role(reference, dependencies);
        self.coverage.record(
            reference.relation(),
            role,
            dependencies.trust,
            RelationResolution::External,
        );
        let value = self
            .external_values
            .entry((
                source,
                reference.target().to_owned(),
                reference.relation(),
                role,
                dependencies.trust,
            ))
            .or_default();
        value.0 += 1;
        if value.1.len() < RETAINED_RELATION_LOCATIONS {
            value.1.push(reference.span());
        }
    }

    /// Records an internal reference that only names a package, not a file.
    fn record_package(
        &mut self,
        source: PackageId,
        target: PackageId,
        reference: &DependencySyntax,
        dependencies: &SourceDependencies,
    ) {
        let source_file = dependencies.file;
        let role = evidence_role(reference, dependencies);
        self.coverage.record(
            reference.relation(),
            role,
            dependencies.trust,
            RelationResolution::ResolvedInternal,
        );
        if source == target
            || !(reference.relation() == smackdebt_analysis::StaticRelationKind::Uses
                && role.affects_verdict()
                && dependencies.trust == SourceTrust::Trusted)
        {
            return;
        }
        self.manifest_explanation_pairs.insert((source, target));
        if role != SourceRole::Primary {
            return;
        }
        let value = self
            .manifest_package_values
            .entry((source, target))
            .or_default();
        value.0.insert(source_file);
        value.1 += 1;
    }

    fn record_diagnostic(
        &mut self,
        source: FileId,
        reference: &DependencySyntax,
        dependencies: &SourceDependencies,
        resolution: RelationResolution,
        kind: ResolutionIssueKind,
        reason: &str,
    ) {
        let role = evidence_role(reference, dependencies);
        self.coverage
            .record(reference.relation(), role, dependencies.trust, resolution);
        if dependencies.trust == SourceTrust::Trusted
            && role == SourceRole::Primary
            && matches!(
                resolution,
                RelationResolution::UnresolvedInternal | RelationResolution::AmbiguousInternal
            )
        {
            self.internal_issue_files.insert(source);
        }
        record_resolution_diagnostic(
            &mut self.diagnostic_values,
            source,
            reference,
            role,
            dependencies.trust,
            kind,
            reason,
        );
    }

    /// Records an internal reference that no repository file matched.
    ///
    /// A reference looked for under a name whose extension the source
    /// languages never analyze is an asset reference: the repository may well
    /// hold the file, but discovery only inventories source, so no lookup
    /// could ever have matched it. Calling that unresolved would claim a hole
    /// in the dependency graph the code does not have, so it is disclosed
    /// under its own reason instead.
    fn record_unmatched_internal(
        &mut self,
        source: FileId,
        reference: &DependencySyntax,
        dependencies: &SourceDependencies,
        candidates: &[String],
    ) {
        let (resolution, kind, reason) = if candidates_name_an_asset(candidates) {
            (
                RelationResolution::AssetReference,
                ResolutionIssueKind::Asset,
                "target is an asset",
            )
        } else {
            (
                RelationResolution::UnresolvedInternal,
                ResolutionIssueKind::Unresolved,
                "no repository file matches",
            )
        };
        self.record_diagnostic(source, reference, dependencies, resolution, kind, reason);
    }

    /// Records a reference no path candidate matched.
    fn record_unmatched(
        &mut self,
        source: FileId,
        source_package: PackageId,
        reference: &DependencySyntax,
        dependencies: &SourceDependencies,
        resolution: ManifestReference,
    ) {
        match resolution {
            ManifestReference::Entry(target) => {
                self.record_internal(source, target, reference, dependencies);
            }
            ManifestReference::Package(target) => {
                self.record_package(source_package, target, reference, dependencies);
            }
            ManifestReference::Ambiguous => self.record_diagnostic(
                source,
                reference,
                dependencies,
                RelationResolution::AmbiguousInternal,
                ResolutionIssueKind::Ambiguous,
                "several packages declare this name",
            ),
            ManifestReference::Absent => self.record_external(source, reference, dependencies),
        }
    }
}

/// Resolves one unmatched reference against the declared package names.
fn resolve_manifest_name(
    reference: &DependencySyntax,
    dependencies: &SourceDependencies,
    manifest_index: &ManifestNameIndex,
    manifest_names: &[Option<String>],
    package_roots: &[PathBuf],
    index: &BTreeMap<PathBuf, FileId>,
) -> ManifestReference {
    if reference.relation() != smackdebt_analysis::StaticRelationKind::Uses {
        return ManifestReference::Absent;
    }
    match manifest_index.resolve(reference.target(), dependencies.language) {
        ManifestNameMatch::Absent => ManifestReference::Absent,
        ManifestNameMatch::Ambiguous => ManifestReference::Ambiguous,
        ManifestNameMatch::Package(position) => {
            let root = &package_roots[position];
            let name = manifest_names[position].as_deref().unwrap_or_default();
            package_entry_file(root, name, index).map_or(
                ManifestReference::Package(PackageId::from_index(position)),
                ManifestReference::Entry,
            )
        }
    }
}

/// The name the current inventory declares for one package root.
///
/// A base-only package root the working tree no longer has declares nothing,
/// which is exactly what the report states for it.
fn declared_manifest_name(inventory: &Inventory, root: &Path) -> Option<String> {
    inventory
        .packages()
        .iter()
        .find(|package| package.root().as_path() == root)
        .and_then(|package| package.manifest_name().map(str::to_owned))
}

/// Aligns discovered manifest names with the report's package positions.
///
/// A package root the working tree no longer has keeps no declared name: the
/// diff sides read the names the current inventory declares.
fn manifest_names_for(inventory: &Inventory, package_roots: &[PathBuf]) -> Vec<Option<String>> {
    let declared: BTreeMap<_, _> = inventory
        .packages()
        .iter()
        .map(|package| {
            (
                package.root().as_path().to_path_buf(),
                package.manifest_name().map(str::to_owned),
            )
        })
        .collect();
    package_roots
        .iter()
        .map(|root| declared.get(root).cloned().flatten())
        .collect()
}

/// The paths each package's manifest names, aligned with `package_roots`.
fn manifest_paths_for(inventory: &Inventory, package_roots: &[PathBuf]) -> Vec<Vec<String>> {
    let declared: BTreeMap<_, _> = inventory
        .packages()
        .iter()
        .map(|package| {
            (
                package.root().as_path().to_path_buf(),
                package.declared_paths().to_vec(),
            )
        })
        .collect();
    package_roots
        .iter()
        .map(|root| declared.get(root).cloned().unwrap_or_default())
        .collect()
}

/// Returns the report package that owns one repository path.
fn package_of(path: &Path, side_package_roots: &[PathBuf], package_roots: &[PathBuf]) -> PackageId {
    let root = nearest_package_root(path, side_package_roots);
    PackageId::from_index(
        package_roots
            .iter()
            .position(|candidate| candidate == &root)
            .unwrap_or(0),
    )
}

fn record_resolution_diagnostic(
    values: &mut BTreeMap<ResolutionDiagnosticKey, DependencyEdgeValue>,
    source: FileId,
    reference: &DependencySyntax,
    role: SourceRole,
    trust: SourceTrust,
    kind: ResolutionIssueKind,
    reason: &str,
) {
    let value = values
        .entry((
            source,
            reference.target().to_owned(),
            kind,
            reason.to_owned(),
            reference.relation(),
            role,
            trust,
        ))
        .or_default();
    value.0 += 1;
    if value.1.len() < RETAINED_RELATION_LOCATIONS {
        value.1.push(reference.span());
    }
}

#[derive(Clone)]
struct ResolutionAlias {
    prefix: String,
    suffix: String,
    replacement: String,
}

#[derive(Clone, Default)]
struct ResolutionRules {
    packages: Vec<PackageResolution>,
}

#[derive(Clone)]
struct PackageResolution {
    root: PathBuf,
    aliases: Vec<ResolutionAlias>,
    issue: Option<String>,
}

impl ResolutionRules {
    fn aliases_for(&self, source: &Path) -> &[ResolutionAlias] {
        self.packages
            .iter()
            .filter(|package| source.starts_with(&package.root))
            .max_by_key(|package| package.root.components().count())
            .map_or(&[], |package| package.aliases.as_slice())
    }
}

impl ResolutionAlias {
    fn expand(&self, candidate: &str) -> Option<String> {
        let middle = candidate
            .strip_prefix(&self.prefix)?
            .strip_suffix(&self.suffix)?;
        Some(self.replacement.replace('*', middle))
    }
}

fn load_resolution_aliases(root: &Path, inventory: &Inventory) -> ResolutionRules {
    let packages = inventory
        .packages()
        .iter()
        .map(|package| {
            let package_root = package.root().as_path().to_path_buf();
            let Some(config) = package.resolution_config() else {
                return PackageResolution {
                    root: package_root,
                    aliases: Vec::new(),
                    issue: None,
                };
            };
            let config = config.as_path().to_path_buf();
            let mut read =
                |path: &Path| fs::read(root.join(path)).map_err(|error| error.to_string());
            let (aliases, issue) = match load_resolution_chain(&config, &mut read) {
                Ok(aliases) => (aliases, None),
                Err(issue) => (Vec::new(), Some(issue)),
            };
            PackageResolution {
                root: package_root,
                aliases,
                issue,
            }
        })
        .collect();
    ResolutionRules { packages }
}

fn load_base_resolution_aliases(
    reader: &mut smackdebt_git::ObjectReader,
    base: &str,
    package_roots: &[PathBuf],
    config_candidates: &[PathBuf],
) -> ResolutionRules {
    let mut sources = BTreeMap::new();
    for path in config_candidates {
        sources.insert(
            path.clone(),
            reader
                .read_path(base, path)
                .map_err(|error| error.to_string()),
        );
    }
    let mut packages = Vec::with_capacity(package_roots.len());
    for root in package_roots {
        let config = config_candidates
            .iter()
            .filter(|path| {
                path.parent()
                    .is_some_and(|directory| root.starts_with(directory))
            })
            .filter(|path| sources.get(*path).is_some_and(Result::is_ok))
            .min_by_key(|path| {
                root.components()
                    .count()
                    .saturating_sub(path.parent().map_or(0, |value| value.components().count()))
            })
            .cloned();
        let Some(config) = config else {
            packages.push(PackageResolution {
                root: root.clone(),
                aliases: Vec::new(),
                issue: None,
            });
            continue;
        };
        let mut read = |path: &Path| match sources.get(path) {
            Some(source) => source.clone(),
            None => {
                let source = reader
                    .read_path(base, path)
                    .map_err(|error| error.to_string());
                sources.insert(path.to_path_buf(), source.clone());
                source
            }
        };
        let (aliases, issue) = match load_resolution_chain(&config, &mut read) {
            Ok(aliases) => (aliases, None),
            Err(issue) => (Vec::new(), Some(issue)),
        };
        packages.push(PackageResolution {
            root: root.clone(),
            aliases,
            issue,
        });
    }
    ResolutionRules { packages }
}

fn load_resolution_chain(
    config: &Path,
    read: &mut impl FnMut(&Path) -> Result<Vec<u8>, String>,
) -> Result<Vec<ResolutionAlias>, String> {
    fn visit(
        config: &Path,
        read: &mut impl FnMut(&Path) -> Result<Vec<u8>, String>,
        stack: &mut BTreeSet<PathBuf>,
        depth: usize,
    ) -> Result<Vec<ResolutionAlias>, String> {
        if depth >= 16 {
            return Err(format!(
                "configuration inheritance is too deep at {}",
                config.display()
            ));
        }
        let config = clean_relative(config).ok_or_else(|| {
            format!(
                "configuration path leaves the repository: {}",
                config.display()
            )
        })?;
        if !stack.insert(config.clone()) {
            return Err(format!(
                "configuration inheritance cycles at {}",
                config.display()
            ));
        }
        let source =
            read(&config).map_err(|error| format!("cannot read {}: {error}", config.display()))?;
        let text = std::str::from_utf8(&source)
            .map_err(|_| format!("{} is not UTF-8", config.display()))?;
        let value = json5::from_str::<serde_json::Value>(text)
            .map_err(|error| format!("cannot parse {}: {error}", config.display()))?;
        let parent = config.parent().unwrap_or(Path::new(""));
        let mut aliases = Vec::new();
        for inherited in inherited_configs(&value, parent)? {
            aliases.extend(visit(&inherited, read, stack, depth + 1)?);
        }
        let local = parse_resolution_aliases(&value, parent)?;
        for alias in &local {
            aliases.retain(|inherited| {
                inherited.prefix != alias.prefix || inherited.suffix != alias.suffix
            });
        }
        aliases.extend(local);
        aliases.sort_by(|left, right| {
            (&left.prefix, &left.suffix, &left.replacement).cmp(&(
                &right.prefix,
                &right.suffix,
                &right.replacement,
            ))
        });
        stack.remove(&config);
        Ok(aliases)
    }

    visit(config, read, &mut BTreeSet::new(), 0)
}

fn inherited_configs(value: &serde_json::Value, parent: &Path) -> Result<Vec<PathBuf>, String> {
    let inherited: Vec<&str> = match &value["extends"] {
        serde_json::Value::Null => Vec::new(),
        serde_json::Value::String(value) => vec![value],
        serde_json::Value::Array(values) => {
            values.iter().filter_map(|value| value.as_str()).collect()
        }
        _ => return Err("configuration extends must be a path or path list".to_owned()),
    };
    inherited
        .into_iter()
        .map(|value| {
            if !value.starts_with('.') {
                return Err(format!(
                    "configuration inheritance is not repository-relative: {value}"
                ));
            }
            let mut path = parent.join(value);
            if path.extension().is_none() {
                path.set_extension("json");
            }
            clean_relative(&path)
                .ok_or_else(|| format!("configuration inheritance leaves the repository: {value}"))
        })
        .collect()
}

fn parse_resolution_aliases(
    value: &serde_json::Value,
    config_parent: &Path,
) -> Result<Vec<ResolutionAlias>, String> {
    let base = value["compilerOptions"]["baseUrl"]
        .as_str()
        .unwrap_or("")
        .trim_matches('/');
    let base = if base == "." { "" } else { base };
    let Some(paths) = value["compilerOptions"]["paths"].as_object() else {
        return Ok(Vec::new());
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
                config_parent.join(replacement)
            } else {
                config_parent.join(base).join(replacement)
            };
            let replacement = clean_relative(&replacement).ok_or_else(|| {
                format!(
                    "alias target leaves the repository: {}",
                    replacement.display()
                )
            })?;
            aliases.push(ResolutionAlias {
                prefix: prefix.to_owned(),
                suffix: suffix.to_owned(),
                replacement: replacement.to_string_lossy().replace('\\', "/"),
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
    Ok(aliases)
}

/// What each package's own manifest states about itself, in package order.
///
/// The two tables are read together wherever a package is asked what it owns,
/// so a build can never hold one without the other.
#[derive(Clone, Copy)]
struct ManifestFacts<'a> {
    names: &'a [Option<String>],
    /// The paths each manifest names as something it publishes, installs, or
    /// runs, spelled relative to the package directory.
    paths: &'a [Vec<String>],
}

/// The package facts one graph side is built against.
///
/// `side_roots` are the package roots of the tree being read; `roots` are the
/// report's own package positions, which the two sides of a diff share.
#[derive(Clone, Copy)]
struct PackageTables<'a> {
    side_roots: &'a [PathBuf],
    roots: &'a [PathBuf],
    manifests: ManifestFacts<'a>,
}

/// Whether the streamed history window holds commits.
///
/// A file's absence from the window is evidence only when the window has
/// something to be absent from. An empty or unavailable window says nothing
/// about any file, so the rules that read coldness stand down rather than
/// treating ignorance as proof.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WindowedHistory {
    Streamed,
    Absent,
}

fn build_architecture(
    work: &AnalysisWork,
    files: &[FileRecord],
    dependencies: &[SourceDependencies],
    aliases: &ResolutionRules,
    packages: PackageTables<'_>,
    history: WindowedHistory,
) -> ArchitectureBuild {
    let PackageTables {
        side_roots: side_package_roots,
        roots: package_roots,
        manifests,
    } = packages;
    work.record_algorithm_pass();
    let mut index = BTreeMap::new();
    for source in dependencies {
        index.insert(source.path.clone(), source.file);
    }
    let manifest_index = ManifestNameIndex::new(manifests.names);
    let mut tables = ReferenceTables::default();
    for dependencies in dependencies {
        let source = dependencies.file;
        let source_package = package_of(&dependencies.path, side_package_roots, package_roots);
        for reference in &dependencies.references {
            match reference.state() {
                DependencySyntaxState::External => {
                    let resolution = resolve_manifest_name(
                        reference,
                        dependencies,
                        &manifest_index,
                        manifests.names,
                        package_roots,
                        &index,
                    );
                    tables.record_unmatched(
                        source,
                        source_package,
                        reference,
                        dependencies,
                        resolution,
                    );
                }
                DependencySyntaxState::Unresolved(reason) => {
                    let resolution =
                        if reference.intent() == smackdebt_analysis::DependencyIntent::Internal {
                            RelationResolution::UnresolvedInternal
                        } else {
                            RelationResolution::UnresolvedPackage
                        };
                    tables.record_diagnostic(
                        source,
                        reference,
                        dependencies,
                        resolution,
                        ResolutionIssueKind::Unresolved,
                        reason,
                    );
                }
                DependencySyntaxState::Candidates(candidates) => {
                    let matches =
                        resolve_candidates(&dependencies.path, candidates, &index, aliases);
                    match matches.as_slice() {
                        [] if reference.intent()
                            == smackdebt_analysis::DependencyIntent::Internal =>
                        {
                            tables.record_unmatched_internal(
                                source,
                                reference,
                                dependencies,
                                candidates,
                            );
                        }
                        [] => {
                            let resolution = resolve_manifest_name(
                                reference,
                                dependencies,
                                &manifest_index,
                                manifests.names,
                                package_roots,
                                &index,
                            );
                            tables.record_unmatched(
                                source,
                                source_package,
                                reference,
                                dependencies,
                                resolution,
                            );
                        }
                        [target] => {
                            tables.record_internal(source, *target, reference, dependencies);
                        }
                        _ => {
                            tables.record_diagnostic(
                                source,
                                reference,
                                dependencies,
                                RelationResolution::AmbiguousInternal,
                                ResolutionIssueKind::Ambiguous,
                                "several repository files match",
                            );
                        }
                    }
                }
            }
        }
    }
    let ReferenceTables {
        coverage,
        edge_values,
        external_values,
        diagnostic_values,
        internal_issue_files,
        manifest_package_values,
        manifest_explanation_pairs,
    } = tables;

    let coverage = coverage.finish();

    let mut diagnostics: Vec<ResolutionDiagnostic> = diagnostic_values
        .into_iter()
        .map(
            |((source, target, kind, reason, relation, role, trust), (references, locations))| {
                ResolutionDiagnostic::new(source, locations[0], target, kind, reason)
                    .with_evidence(relation, role, trust)
                    .with_occurrences(references, locations)
            },
        )
        .collect();

    let mut file_edges: Vec<_> = edge_values
        .into_iter()
        .enumerate()
        .map(
            |(edge_index, ((source, target, relation, role, trust), (references, locations)))| {
                DependencyEdge::new(
                    DependencyEdgeId::from_index(edge_index),
                    source,
                    target,
                    references,
                    locations,
                )
                .with_relation(relation)
                .with_evidence(role, trust)
            },
        )
        .collect();
    let mut external: Vec<_> = external_values
        .into_iter()
        .map(
            |((file, target, relation, role, trust), (references, locations))| {
                ExternalDependency::new(file, target, references)
                    .with_evidence(locations, relation, role, trust)
            },
        )
        .collect();

    // Computed once, here, and read by both the dormancy rule below and the
    // orphan table further down, so the two can never disagree about what a
    // package owns.
    let declared_entries = declared_entry_files(PackageEntries {
        package_roots,
        manifest_names: manifests.names,
        manifest_paths: manifests.paths,
        index: &index,
    });
    // The last role this build settles, and the first point at which it can be:
    // dormancy is recognized by what nothing does with a file, so the resolved
    // relations above are its evidence. Every graph fact below is derived from
    // the restated table, so the role a file carries in the report is the role
    // its package closure, its core, and its orphan state were computed under.
    let dormant = dormant_javascript(files, dependencies, &file_edges, history, &declared_entries);
    let restated;
    let files = if dormant.is_empty() {
        files
    } else {
        restated = restate_dormant(files, &dormant);
        &restated
    };
    restate_dormant_relations(&dormant, &mut file_edges, &mut external, &mut diagnostics);

    let parse_failure_files: Vec<_> = files
        .iter()
        .filter(|file| {
            file.package().is_some()
                && file.role() == SourceRole::Primary
                && file.trust() != SourceTrust::Trusted
        })
        .map(FileRecord::id)
        .collect();
    // Only a file the graph reads can leave a hole in it. `enters_file_graph`
    // is the predicate the closures, the core, and the connection graph are
    // built with, so asking it here keeps what withholds a fact and what
    // produces it the same rule: a fixture, a test, or a generated file may
    // publish every diagnostic its unread imports earned without costing its
    // package the completeness those facts are stated from.
    let mut incomplete_packages: Vec<_> = parse_failure_files
        .iter()
        .chain(
            internal_issue_files
                .iter()
                .filter(|file| enters_file_graph(&files[file.index()])),
        )
        .filter_map(|file| files[file.index()].package())
        .collect();
    let configuration_failures: Vec<_> = aliases
        .packages
        .iter()
        .filter_map(|package| {
            let issue = package.issue.as_ref()?;
            let owner = package_of(
                &package.root.join("resolution-config"),
                side_package_roots,
                package_roots,
            );
            incomplete_packages.push(owner);
            Some(GraphConfigurationFailure::new(owner, issue.clone()))
        })
        .collect();
    let graph_evidence = GraphEvidence::new(
        incomplete_packages,
        parse_failure_files.len() as u32,
        coverage.unresolved_internal_uses(),
        coverage.ambiguous_internal_uses(),
        configuration_failures,
    );

    let mut package_values: BTreeMap<(PackageId, PackageId), (u32, u32, Vec<DependencyEdgeId>)> =
        BTreeMap::new();
    let mut explanation_pairs: BTreeSet<(PackageId, PackageId)> = BTreeSet::new();
    for edge in &file_edges {
        if !edge.affects_verdict() {
            continue;
        }
        let source_path = dependencies
            .iter()
            .find(|source| source.file == edge.source())
            .map(|source| source.path.as_path())
            .unwrap_or_else(|| Path::new(files[edge.source().index()].path()));
        let target_path = dependencies
            .iter()
            .find(|source| source.file == edge.target())
            .map(|source| source.path.as_path())
            .unwrap_or_else(|| Path::new(files[edge.target().index()].path()));
        let source = package_of(source_path, side_package_roots, package_roots);
        let target = package_of(target_path, side_package_roots, package_roots);
        if source == target {
            continue;
        }
        explanation_pairs.insert((source, target));
        if !edge.enters_verdict_graph() {
            continue;
        }
        let value = package_values.entry((source, target)).or_default();
        value.0 += 1;
        value.1 += edge.references();
        value.2.push(edge.id());
    }
    explanation_pairs.extend(manifest_explanation_pairs);
    for ((source, target), (files, references)) in manifest_package_values {
        let value = package_values.entry((source, target)).or_default();
        value.0 += u32::try_from(files.len()).unwrap_or(u32::MAX);
        value.1 += references;
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
    // The package graph is small enough to close over whole: its node count is
    // the package count, so no limit gates it.
    let package_reach = reach_in_counts(package_count, &package_pairs);
    let measurements: Vec<_> = degrees
        .into_iter()
        .enumerate()
        .map(|(index, (incoming, outgoing))| {
            PackageGraphMeasurement::new(PackageId::from_index(index), incoming, outgoing)
                .with_reach_in(package_reach[index])
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
    let stable_dependencies = stable_dependency_findings(&package_edges, &measurements);

    // Orphan fan-in asks whether anything uses a file at all, so it keeps the
    // wider evidence predicate: a file its own tests import is used. The cycle
    // graph is a verdict, so it keeps primary relations only.
    let orphan_pairs: Vec<_> = file_edges
        .iter()
        .filter(|edge| edge.affects_verdict())
        .map(|edge| (edge.source().index(), edge.target().index()))
        .collect();
    // A Rust `mod` declaration and the imports that accompany it are one wiring
    // relationship rather than a cycle, so the cycle graph drops the uses
    // between an owning pair. The exclusion is pairwise: every other relation of
    // the same component stays, and only this graph sees it.
    let ownership_pairs: BTreeSet<(usize, usize)> = file_edges
        .iter()
        .filter(|edge| edge.relation() == smackdebt_analysis::StaticRelationKind::ModuleOwnership)
        .map(|edge| unordered_pair(edge.source().index(), edge.target().index()))
        .collect();
    let enters_cycle_graph = |edge: &DependencyEdge| {
        edge.enters_verdict_graph()
            && !ownership_pairs.contains(&unordered_pair(
                edge.source().index(),
                edge.target().index(),
            ))
    };
    let file_pairs: Vec<_> = file_edges
        .iter()
        .filter(|edge| enters_cycle_graph(edge))
        .map(|edge| (edge.source().index(), edge.target().index()))
        .collect();
    let orphans = derive_orphans(files, dependencies, &orphan_pairs, &declared_entries);
    // Absence is proved against a wider graph than the cycle graph: every
    // `uses` and every `module_ownership` relation between two graph files, in
    // both directions of travel. It is built here, beside the cycle graph it
    // must never be confused with, and carried to the change-leakage join.
    let connection_relations: Vec<_> = file_edges
        .iter()
        .filter(|edge| enters_connection_graph(edge, files))
        .map(|edge| (edge.source().index(), edge.target().index()))
        .collect();
    let graph_packages = graph_packages(files);
    let connections = ConnectionGraph::new(
        files.len(),
        package_count,
        &connection_relations,
        &graph_packages,
    );
    // The components the cycle findings are made of are also the core and the
    // reach candidates, so they are retained rather than recomputed.
    let file_components = strongly_connected_components(files.len(), &file_pairs);
    let largest_component = file_components
        .iter()
        .max_by(|left, right| left.len().cmp(&right.len()).then_with(|| right.cmp(left)));
    let file_graph_count = graph_file_count(files);
    let core_size = CoreSize::from_counts(
        largest_component.map_or(0, Vec::len) as u32,
        file_graph_count,
    );
    let core_members = largest_component
        .into_iter()
        .flatten()
        .copied()
        .map(FileId::from_index)
        .collect();
    let retained_file_components = file_components
        .iter()
        .map(|component| component.iter().copied().map(FileId::from_index).collect())
        .collect();
    let closures = close_over_packages(package_count, &graph_packages, &file_pairs);
    let file_reach = file_reaches(files, &file_components, &file_pairs);
    for component in file_components
        .iter()
        .filter(|component| component.len() > 1)
    {
        let packages: std::collections::BTreeSet<_> = component
            .iter()
            .filter_map(|file| files[*file].package())
            .collect();
        if packages.len() != 1 {
            continue;
        }
        let witness = cycle_witness(component, &file_pairs).unwrap_or_default();
        let witness_edges: Vec<_> = witness
            .iter()
            .filter_map(|&(source, target)| {
                file_edges
                    .iter()
                    .find(|edge| {
                        enters_cycle_graph(edge)
                            && edge.source().index() == source
                            && edge.target().index() == target
                    })
                    .map(DependencyEdge::id)
            })
            .collect();
        let id = ArchitectureFindingId::from_index(findings.len());
        for file in component {
            finding_links.push((files[*file].scope(), id));
        }
        findings.push(ArchitectureFinding::new(
            id,
            ArchitectureFindingKind::FileCycle,
            packages.into_iter().collect(),
            component.iter().copied().map(FileId::from_index).collect(),
            witness_edges,
        ));
    }

    ArchitectureBuild {
        coverage,
        graph_evidence,
        orphans,
        stable_dependencies,
        file_edges,
        package_edges,
        external,
        diagnostics,
        measurements,
        findings,
        finding_links,
        cycles,
        explanation_pairs,
        package_closures: closures.closures().to_vec(),
        package_file_reach: closures.file_reaches().to_vec(),
        skipped_closures: closures.skipped().to_vec(),
        file_reach,
        core_size,
        core_members,
        file_components: retained_file_components,
        file_graph_count,
        cycle_pairs: file_pairs,
        connections,
        graph_packages,
        dormant,
    }
}

/// The package of every file that enters the file dependency graph, by file
/// table position.
///
/// A file outside the graph belongs to no package closure, so the fraction a
/// package states is a fraction of one population.
fn graph_packages(files: &[FileRecord]) -> Vec<Option<PackageId>> {
    files
        .iter()
        .map(|file| file.package().filter(|_| enters_file_graph(file)))
        .collect()
}

/// Orders one file pair so a relation and its reverse read as the same pair.
const fn unordered_pair(source: usize, target: usize) -> (usize, usize) {
    if source <= target {
        (source, target)
    } else {
        (target, source)
    }
}

/// Derives orphan facts from the degree of the existing verdict file graph.
///
/// No new traversal is introduced: the file pairs and the file table are the
/// tables the architecture pass already produced.
fn derive_orphans(
    files: &[FileRecord],
    dependencies: &[SourceDependencies],
    file_pairs: &[(usize, usize)],
    declared_entries: &BTreeSet<FileId>,
) -> Vec<OrphanFile> {
    let degrees = dependency_degree(files.len(), file_pairs);
    let mut analyzed = BTreeSet::new();
    for source in dependencies {
        analyzed.insert(source.file);
    }
    let candidates: Vec<_> = files
        .iter()
        .enumerate()
        .map(|(position, file)| {
            OrphanCandidate::new(
                file.id(),
                file.path(),
                file.role(),
                analyzed.contains(&file.id()),
                degrees.get(position).map_or(0, |(incoming, _)| *incoming),
                declared_entries.contains(&file.id()),
            )
        })
        .collect();
    orphan_files(&candidates)
}

/// The tables that say which file each package presents as its entry point.
#[derive(Clone, Copy)]
struct PackageEntries<'a> {
    package_roots: &'a [PathBuf],
    manifest_names: &'a [Option<String>],
    /// The paths each package's manifest names, in package order.
    manifest_paths: &'a [Vec<String>],
    index: &'a BTreeMap<PathBuf, FileId>,
}

/// Every file a package presents as its own entry point.
///
/// A declared entry is reached from outside the repository, so nothing inside
/// it needs to import the file for it to be used. Two answers are joined: the
/// conventional entry path of the package, and every path the package's own
/// manifest names as something it publishes, installs, or runs.
///
/// Both the orphan table and the dormancy rule read this one answer, computed
/// once per build, so a package's entry point can never be an orphan in one and
/// unowned in the other.
fn declared_entry_files(entries: PackageEntries<'_>) -> BTreeSet<FileId> {
    let mut declared = BTreeSet::new();
    for (position, root) in entries.package_roots.iter().enumerate() {
        let fallback = root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let name = entries
            .manifest_names
            .get(position)
            .and_then(Option::as_deref)
            .unwrap_or(fallback);
        if let Some(entry) = package_entry_file(root, name, entries.index) {
            declared.insert(entry);
        }
        let manifest_paths = entries
            .manifest_paths
            .get(position)
            .map_or(&[][..], Vec::as_slice);
        for path in manifest_paths {
            let Some(cleaned) = clean_relative(&root.join(path)) else {
                continue;
            };
            if let Some(file) = entries.index.get(&cleaned) {
                declared.insert(*file);
            }
        }
    }
    declared
}

/// The JavaScript files this repository is demonstrably not working on.
///
/// Every signal is an absence, and they must all hold at once: the file is a
/// plain script rather than a module, nothing in the repository imports it, no
/// package names it as something it publishes or runs, its own name is not a
/// conventional entry name, and no commit in the streamed window touched it.
///
/// What that adds up to is inattention, not provenance. A jQuery-era library
/// dropped into a public directory answers to it, and so does a page script the
/// repository wrote years ago and has not opened since - and nothing here can
/// tell those two apart, because no signal here looks at who wrote anything.
/// The role states only what was measured: no one is working on this.
///
/// Only the JavaScript a browser or a runtime loads as written can qualify.
/// TypeScript and JSX compile from source the repository authored, so their
/// spellings are never considered however cold or unimported they are.
fn dormant_javascript(
    files: &[FileRecord],
    dependencies: &[SourceDependencies],
    edges: &[DependencyEdge],
    history: WindowedHistory,
    declared: &BTreeSet<FileId>,
) -> BTreeSet<FileId> {
    if history == WindowedHistory::Absent {
        return BTreeSet::new();
    }
    let scripts: BTreeSet<FileId> = dependencies
        .iter()
        .filter(|source| !source.module_syntax)
        .map(|source| source.file)
        .collect();
    let candidates: Vec<_> = files
        .iter()
        .filter(|file| is_dormancy_candidate(file) && scripts.contains(&file.id()))
        .collect();
    if candidates.is_empty() {
        return BTreeSet::new();
    }
    let imported: BTreeSet<FileId> = edges
        .iter()
        .filter(|edge| edge.affects_verdict())
        .map(DependencyEdge::target)
        .collect();
    candidates
        .into_iter()
        .map(FileRecord::id)
        .filter(|file| !imported.contains(file) && !declared.contains(file))
        .collect()
}

/// Whether the evidence rule may ever look at this file.
///
/// A file that already carries a role states what it is, and a file the window
/// recorded a commit against is worked on, so neither is dormant. Entry points
/// and tool configuration are excluded for the same reason as each other: both
/// are reached by name rather than by import, so having no importer is what
/// they are supposed to look like.
fn is_dormancy_candidate(file: &FileRecord) -> bool {
    let path = Path::new(file.path());
    file.role() == SourceRole::Primary
        && file.activity().is_none()
        && !smackdebt_analysis::is_entry_filename(file.path())
        && !is_tool_configuration_name(path)
        && is_runtime_javascript_path(path)
}

/// Restates every relation written in a dormant file.
///
/// The relations themselves are untouched; only what they count as changes. A
/// reference written in a file nobody is working on stops being evidence about
/// the code that ships, so it shapes no package edge, no cycle, and no orphan
/// pair - which is what the graph facts below are derived from.
fn restate_dormant_relations(
    dormant: &BTreeSet<FileId>,
    file_edges: &mut [DependencyEdge],
    external: &mut [ExternalDependency],
    diagnostics: &mut [ResolutionDiagnostic],
) {
    if dormant.is_empty() {
        return;
    }
    for edge in file_edges
        .iter_mut()
        .filter(|edge| dormant.contains(&edge.source()))
    {
        *edge = edge.clone().in_context_role(SourceRole::Dormant);
    }
    for row in external
        .iter_mut()
        .filter(|row| dormant.contains(&row.file()))
    {
        *row = row.clone().in_context_role(SourceRole::Dormant);
    }
    for row in diagnostics
        .iter_mut()
        .filter(|row| dormant.contains(&row.file()))
    {
        *row = row.clone().in_context_role(SourceRole::Dormant);
    }
}

/// Returns the file table with every dormant file restated under its role.
fn restate_dormant(files: &[FileRecord], dormant: &BTreeSet<FileId>) -> Vec<FileRecord> {
    files
        .iter()
        .map(|file| {
            if dormant.contains(&file.id()) {
                file.clone().in_context_role(SourceRole::Dormant)
            } else {
                file.clone()
            }
        })
        .collect()
}

/// What a declared manifest name means inside this repository.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ManifestNameMatch {
    /// No internal package declares the name.
    Absent,
    /// Several internal packages declare the name.
    Ambiguous,
    /// Exactly one internal package declares the name.
    Package(usize),
}

/// A read-only index from declared package names to internal packages.
///
/// Positions are package-root positions, so a match names the same package the
/// rest of the build already knows.  The index is consulted only for references
/// that path candidates would otherwise classify external.
#[derive(Default)]
struct ManifestNameIndex {
    exact: BTreeMap<String, ManifestNameMatch>,
    rust: BTreeMap<String, ManifestNameMatch>,
}

impl ManifestNameIndex {
    fn new(names: &[Option<String>]) -> Self {
        let mut index = Self::default();
        for (position, name) in names.iter().enumerate() {
            let Some(name) = name else {
                continue;
            };
            index.insert_key(name.clone(), position, false);
            index.insert_key(rust_manifest_key(name), position, true);
        }
        index
    }

    fn insert_key(&mut self, key: String, position: usize, rust: bool) {
        let keys = if rust {
            &mut self.rust
        } else {
            &mut self.exact
        };
        keys.entry(key)
            .and_modify(|value| {
                if *value != ManifestNameMatch::Package(position) {
                    *value = ManifestNameMatch::Ambiguous;
                }
            })
            .or_insert(ManifestNameMatch::Package(position));
    }

    fn resolve(&self, target: &str, language: Language) -> ManifestNameMatch {
        let Some(root) = reference_root(target, language) else {
            return ManifestNameMatch::Absent;
        };
        if language == Language::Rust {
            return self
                .rust
                .get(&rust_manifest_key(root))
                .copied()
                .unwrap_or(ManifestNameMatch::Absent);
        }
        self.exact
            .get(root)
            .copied()
            .unwrap_or(ManifestNameMatch::Absent)
    }
}

/// Normalizes the Rust equivalence of hyphens and underscores in a name.
fn rust_manifest_key(name: &str) -> String {
    name.replace('-', "_")
}

/// Returns the package-naming first segment of an unresolved reference.
fn reference_root(target: &str, language: Language) -> Option<&str> {
    let root = match language {
        Language::Rust => target.split("::").next(),
        Language::Python | Language::Java => target.split(['.', '/']).next(),
        _ if target.starts_with('@') => {
            let mut parts = target.splitn(3, '/');
            match (parts.next(), parts.next()) {
                (Some(scope), Some(name)) => Some(&target[..scope.len() + name.len() + 1]),
                _ => Some(target),
            }
        }
        _ => target.split('/').next(),
    }?;
    (!root.is_empty()).then_some(root)
}

/// Returns the file a package presents as its entry point, when it has one.
fn package_entry_file(
    root: &Path,
    name: &str,
    index: &BTreeMap<PathBuf, FileId>,
) -> Option<FileId> {
    let module = name.rsplit('/').next().unwrap_or(name).replace('-', "_");
    [
        "src/lib.rs".to_owned(),
        "src/main.rs".to_owned(),
        "index.js".to_owned(),
        "index.mjs".to_owned(),
        "index.ts".to_owned(),
        "src/index.js".to_owned(),
        "src/index.mjs".to_owned(),
        "src/index.ts".to_owned(),
        "lib/index.js".to_owned(),
        "__init__.py".to_owned(),
        format!("{module}/__init__.py"),
        format!("src/{module}/__init__.py"),
        format!("lib/{module}.rb"),
    ]
    .into_iter()
    .find_map(|candidate| index.get(&root.join(candidate)).copied())
}

fn resolve_candidates(
    source: &Path,
    candidates: &[String],
    index: &BTreeMap<PathBuf, FileId>,
    aliases: &ResolutionRules,
) -> Vec<FileId> {
    let parent = source.parent().unwrap_or(Path::new(""));
    let rust = source
        .extension()
        .is_some_and(|extension| extension == "rs");
    // A Rust file that is not `mod.rs`, `lib.rs`, or `main.rs` owns a directory
    // of its own name, so `mod child;` in `a.rs` names `a/child.rs` rather than
    // a sibling. That reading wins where it resolves; the sibling reading stays
    // for every other layout.
    let module_directory = rust.then(|| rust_module_directory(source)).flatten();
    let source_root = rust.then(|| rust_source_root(source)).flatten();
    let aliases = aliases.aliases_for(source);
    let resolve = |expanded: &[String], matches: &mut BTreeSet<FileId>| {
        for candidate in expanded {
            let path = Path::new(&candidate);
            let relative = candidate.starts_with("./") || candidate.starts_with("../");
            let module = module_directory
                .as_ref()
                .filter(|_| relative)
                .and_then(|directory| clean_relative(&directory.join(path)))
                .and_then(|clean| index.get(&clean));
            if let Some(file) = module {
                matches.insert(*file);
                continue;
            }
            let mut joined = if relative {
                vec![parent.join(path)]
            } else {
                vec![path.to_path_buf()]
            };
            if !relative && let Some(source_root) = &source_root {
                joined.push(source_root.join(path));
            }
            for joined in joined {
                if let Some(clean) = clean_relative(&joined)
                    && let Some(file) = index.get(&clean)
                {
                    matches.insert(*file);
                }
            }
        }
    };
    if !rust {
        let mut expanded = Vec::new();
        for candidate in candidates
            .iter()
            .filter(|candidate| !smackdebt_analysis::is_symbolic_candidate(candidate))
        {
            let candidate = strip_path_suffix(candidate);
            expanded.push(candidate.to_owned());
            expanded.extend(aliases.iter().filter_map(|alias| alias.expand(candidate)));
        }
        let mut matches = BTreeSet::new();
        resolve(&expanded, &mut matches);
        if matches.is_empty() {
            let source_spellings: Vec<_> = expanded
                .iter()
                .flat_map(|candidate| runtime_source_spellings(candidate))
                .collect();
            resolve(&source_spellings, &mut matches);
        }
        if matches.is_empty() {
            matches.extend(resolve_symbolic_candidates(source, candidates, index));
        }
        return matches.into_iter().collect();
    }
    for candidate in candidates
        .iter()
        .filter(|candidate| !smackdebt_analysis::is_symbolic_candidate(candidate))
    {
        let candidate = strip_path_suffix(candidate);
        let mut expanded = vec![candidate.to_owned()];
        expanded.extend(aliases.iter().filter_map(|alias| alias.expand(candidate)));
        let mut matches = BTreeSet::new();
        resolve(&expanded, &mut matches);
        if matches.is_empty() {
            let source_spellings: Vec<_> = expanded
                .iter()
                .flat_map(|candidate| runtime_source_spellings(candidate))
                .collect();
            resolve(&source_spellings, &mut matches);
        }
        if !matches.is_empty() {
            return matches.into_iter().collect();
        }
    }
    resolve_symbolic_candidates(source, candidates, index)
        .into_iter()
        .collect()
}

fn strip_path_suffix(candidate: &str) -> &str {
    candidate
        .find(['?', '#'])
        .map_or(candidate, |index| &candidate[..index])
}

/// Whether the paths a reference was looked for under spell a file the
/// source languages never analyze.
///
/// The question is asked of the candidate spellings and never of the written
/// target, because an extension only means what it looks like once a language
/// has read its target as a path — and a language says so by handing that path
/// back. Dotted module notation is read as a name instead, and arrives here
/// already turned into paths: Python offers `../core.py` for `..core`, Java
/// offers `app/Local.java` for `app.Local`. Judged on the written target those
/// two would carry the extensions `core` and `Local`, and every broken module
/// import in the repository would vanish under an asset row — the inverse of
/// the false hole this classification exists to remove.
///
/// Nothing is read from the filesystem. Discovery walks once and inventories
/// source only, so an extension it does not claim could never have been
/// indexed whether the file is on disk or not. A spelling with no extension
/// claims nothing, because a bare `./config` is an unwritten source path far
/// more often than it is an asset.
fn candidates_name_an_asset(candidates: &[String]) -> bool {
    candidates
        .iter()
        .filter(|candidate| !smackdebt_analysis::is_symbolic_candidate(candidate))
        .any(|candidate| {
            let path = Path::new(strip_path_suffix(candidate));
            path.extension().is_some() && !is_source_path(path)
        })
}

fn runtime_source_spellings(candidate: &str) -> Vec<String> {
    let path = Path::new(candidate);
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return Vec::new();
    };
    let replacements: &[&str] = match extension {
        "js" => &["ts", "tsx"],
        "jsx" => &["tsx"],
        "mjs" => &["mts", "ts"],
        "cjs" => &["cts", "ts"],
        _ => return Vec::new(),
    };
    replacements
        .iter()
        .map(|extension| {
            path.with_extension(extension)
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

/// Resolves candidates that name a file by role instead of by path.
///
/// Symbolic candidates are the last resort of one reference: they are consulted
/// only when no path candidate of the same reference matched a discovered file.
fn resolve_symbolic_candidates(
    source: &Path,
    candidates: &[String],
    index: &BTreeMap<PathBuf, FileId>,
) -> Vec<FileId> {
    let mut matches = Vec::new();
    for candidate in candidates {
        let file = match candidate.as_str() {
            smackdebt_analysis::DECLARING_FILE_CANDIDATE => index.get(source).copied(),
            smackdebt_analysis::CRATE_ROOT_CANDIDATE => rust_crate_root_file(source, index),
            smackdebt_analysis::PARENT_MODULE_CANDIDATE => rust_parent_module_file(source, index),
            _ => None,
        };
        matches.extend(file);
    }
    matches
}

/// The file a Rust package presents as the root of its module tree.
fn rust_crate_root_file(source: &Path, index: &BTreeMap<PathBuf, FileId>) -> Option<FileId> {
    let root = rust_source_root(source)?;
    index
        .get(&root.join("lib.rs"))
        .or_else(|| index.get(&root.join("main.rs")))
        .copied()
}

/// The file declaring the module that declares a Rust file.
///
/// The enclosing module is the directory holding the file's own module
/// directory, and Rust spells that module in three places: `a/mod.rs` inside
/// it, `a.rs` beside it, and the crate root when the module is the source root
/// itself.  The first spelling the repository holds is the answer.
fn rust_parent_module_file(source: &Path, index: &BTreeMap<PathBuf, FileId>) -> Option<FileId> {
    let directory = rust_module_directory(source)?;
    let parent = directory.parent()?;
    let name = parent.file_name()?.to_str()?;
    let root = rust_source_root(source).filter(|root| root.as_path() == parent);
    [
        parent.join("mod.rs"),
        parent.with_file_name(format!("{name}.rs")),
    ]
    .into_iter()
    .chain(
        root.into_iter()
            .flat_map(|root| [root.join("lib.rs"), root.join("main.rs")]),
    )
    .find_map(|path| index.get(&path).copied())
}

/// The directory a Rust file's own modules live in.
///
/// `mod.rs`, `lib.rs`, and `main.rs` are the module of their directory; every
/// other file is a module that owns a directory named after it.
fn rust_module_directory(source: &Path) -> Option<PathBuf> {
    let parent = source.parent()?;
    let stem = source.file_stem()?.to_str()?;
    Some(if matches!(stem, "mod" | "lib" | "main") {
        parent.to_path_buf()
    } else {
        parent.join(stem)
    })
}

fn rust_source_root(source: &Path) -> Option<PathBuf> {
    let mut root = PathBuf::new();
    for component in source.parent()?.components() {
        let std::path::Component::Normal(value) = component else {
            return None;
        };
        root.push(value);
        if value == "src" {
            return Some(root);
        }
    }
    None
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

fn nearest_package_root(path: &Path, package_roots: &[PathBuf]) -> PathBuf {
    package_roots
        .iter()
        .filter(|root| path.starts_with(root))
        .max_by_key(|root| root.components().count())
        .cloned()
        .unwrap_or_default()
}

fn report_package_path(path: &Path) -> String {
    if path.as_os_str().is_empty() {
        ".".to_owned()
    } else {
        path.display().to_string()
    }
}

struct HierarchyBuilder {
    scopes: Vec<Scope>,
    package_scopes: Vec<ScopeId>,
    file_scopes: BTreeMap<PathBuf, ScopeId>,
    directories: BTreeMap<(PathBuf, PathBuf), ScopeId>,
    package_roots: Vec<PathBuf>,
}

impl HierarchyBuilder {
    fn new(label: String, package_roots: &[PathBuf]) -> Self {
        let root = ScopeId::from_index(0);
        let mut scopes = vec![Scope::new(root, ScopeKind::Repository, label, None)];
        let mut package_scopes = Vec::with_capacity(package_roots.len());
        for package_root in package_roots {
            let id = ScopeId::from_index(scopes.len());
            package_scopes.push(id);
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
            package_scopes,
            file_scopes: BTreeMap::new(),
            directories: BTreeMap::new(),
            package_roots: package_roots.to_vec(),
        }
    }

    fn add_file(&mut self, path: &Path, package_index: usize) {
        let package_root = self.package_roots[package_index].clone();
        let package_scope = self.package_scopes[package_index];
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
    use smackdebt_analysis::{
        CodebaseTier, DiffTier, ProblemPattern, WorstOffenderReason, duplicate_claim,
    };
    use std::process::Command;

    /// A history file list the diff flow could produce: file 1 was filtered
    /// out, so the second entry's identity is 2, not 1.
    #[test]
    fn history_paths_are_placed_by_file_identity_rather_than_pushed_in_order() {
        let files = [
            (PathBuf::from("left/a.rs"), 0),
            (PathBuf::from("right/b.rs"), 2),
        ]
        .map(|(path, index)| {
            (
                path,
                FileId::from_index(index),
                PackageId::from_index(0),
                SourceRole::Primary,
                SourceTrust::Trusted,
            )
        });
        let paths = history_directory_paths(&files);
        assert_eq!(paths, ["left/a.rs", "", "right/b.rs"]);

        // Pushing in order would file `right/b.rs` under identity 1 and answer
        // the root for identity 2, so both directories and every distance drawn
        // from them would be wrong with no wrong-looking value to notice.
        let tree = DirectoryTree::from_file_paths(paths);
        let directory = |index| tree.directory_of(FileId::from_index(index));
        assert_eq!(directory(1), Some(DirectoryTree::ROOT));
        assert_ne!(directory(2), Some(DirectoryTree::ROOT));
        assert_eq!(
            tree.distance(directory(0).unwrap(), directory(2).unwrap()),
            2
        );
    }

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

    /// Commit with the authored and landed (committer) dates both pinned, so
    /// window fixtures describe the instant history filters compare.
    fn git_dated<const N: usize>(root: &Path, date: &str, args: [&str; N]) {
        let output = Command::new("git")
            .args(args)
            .env("GIT_AUTHOR_DATE", date)
            .env("GIT_COMMITTER_DATE", date)
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

    fn workspace_with_declared_names() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        for (path, source) in [
            (
                "crates/core/Cargo.toml",
                "[package]\nname='acme-core'\nversion='0.1.0'\n",
            ),
            (
                "crates/core/src/lib.rs",
                "pub fn core(value: i32) -> i32 { value }\n",
            ),
            (
                "crates/app/Cargo.toml",
                "[package]\nname='acme-app'\nversion='0.1.0'\n[lib]\nname='acme_renamed'\n",
            ),
            (
                "crates/app/src/lib.rs",
                "use acme_core::core;\npub fn app(value: i32) -> i32 { core(value) }\n",
            ),
            (
                "ui/package.json",
                "{\"name\":\"@acme/ui\",\"private\":true}\n",
            ),
            ("ui/index.js", "export const ui = 1;\n"),
            (
                "web/package.json",
                "{\"name\":\"@acme/web\",\"private\":true}\n",
            ),
            (
                "web/index.js",
                "import { ui } from '@acme/ui/button';\nexport const web = ui;\n",
            ),
        ] {
            let file = root.path().join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, source).unwrap();
        }
        root
    }

    fn package_pairs(report: &Report) -> Vec<(String, String)> {
        report
            .package_edges()
            .iter()
            .map(|edge| {
                (
                    report.packages()[edge.source().index()].path().to_owned(),
                    report.packages()[edge.target().index()].path().to_owned(),
                )
            })
            .collect()
    }

    #[test]
    fn declared_manifest_names_resolve_cross_package_references() {
        let root = workspace_with_declared_names();
        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        assert_eq!(
            package_pairs(report),
            [
                ("crates/app".to_owned(), "crates/core".to_owned()),
                ("web".to_owned(), "ui".to_owned()),
            ]
        );
        assert!(
            report
                .external_dependencies()
                .iter()
                .all(|external| external.target() != "acme_core"
                    && external.target() != "@acme/ui/button"),
            "{:?}",
            report.external_dependencies()
        );
        let edges: Vec<_> = report
            .dependency_edges()
            .iter()
            .map(|edge| {
                (
                    report.files()[edge.source().index()].path(),
                    report.files()[edge.target().index()].path(),
                )
            })
            .collect();
        assert!(
            edges.contains(&("crates/app/src/lib.rs", "crates/core/src/lib.rs")),
            "{edges:?}"
        );
        assert!(
            edges.contains(&("web/index.js", "ui/index.js")),
            "{edges:?}"
        );
    }

    #[test]
    fn a_shadowed_manifest_name_stays_ambiguous_with_its_diagnostic() {
        let root = workspace_with_declared_names();
        fs::create_dir_all(root.path().join("mirror/core/src")).unwrap();
        fs::write(
            root.path().join("mirror/core/Cargo.toml"),
            "[package]\nname='acme-core'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            root.path().join("mirror/core/src/lib.rs"),
            "pub fn core(value: i32) -> i32 { value }\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        assert!(
            !package_pairs(report).contains(&("crates/app".to_owned(), "crates/core".to_owned())),
            "{:?}",
            package_pairs(report)
        );
        let ambiguous: Vec<_> = report
            .resolution_diagnostics()
            .iter()
            .filter(|value| value.kind() == ResolutionIssueKind::Ambiguous)
            .map(|value| (value.target(), value.span().start_line()))
            .collect();
        assert_eq!(ambiguous, [("acme_core::core", 1)]);
    }

    #[test]
    fn symbolic_candidates_are_the_last_resort_of_one_reference() {
        let root = tempfile::tempdir().unwrap();
        for (path, source) in [
            (
                "Cargo.toml",
                "[package]\nname='symbolic'\nversion='0.1.0'\n",
            ),
            (
                "src/lib.rs",
                "mod deep;\nmod helper;\nmod registries;\nmod report;\npub struct Item;\n",
            ),
            (
                "src/report.rs",
                "use crate::Item;\nuse crate::registries::traits::ToolExt;\nmod tests {\n    use super::*;\n}\n",
            ),
            ("src/deep/mod.rs", "mod inner;\n"),
            (
                "src/deep/inner.rs",
                "mod tests {\n    use super::helper::work;\n}\n",
            ),
            ("src/helper.rs", "pub fn work() -> i32 { 1 }\n"),
            ("src/registries.rs", "pub mod traits;\n"),
            ("src/registries/traits.rs", "pub struct ToolExt;\n"),
            ("standalone/loose.rs", "use crate::Missing;\n"),
        ] {
            let file = root.path().join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, source).unwrap();
        }

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let edges: Vec<_> = report
            .dependency_edges()
            .iter()
            .filter(|edge| edge.relation() == smackdebt_analysis::StaticRelationKind::Uses)
            .map(|edge| {
                (
                    report.files()[edge.source().index()].path(),
                    report.files()[edge.target().index()].path(),
                )
            })
            .collect();
        assert!(
            edges.contains(&("src/report.rs", "src/lib.rs")),
            "a crate-root item resolves to the crate root: {edges:?}"
        );
        assert!(
            edges.contains(&("src/deep/inner.rs", "src/helper.rs")),
            "a matching module path wins over the declaring file: {edges:?}"
        );
        assert!(
            edges.contains(&("src/report.rs", "src/registries/traits.rs")),
            "the nearest matching module wins over its parent: {edges:?}"
        );
        assert!(
            !edges
                .iter()
                .any(|(source, target)| source == target || *target == "src/deep/inner.rs"),
            "a reference to the declaring file creates no edge: {edges:?}"
        );
        let unresolved: Vec<_> = report
            .resolution_diagnostics()
            .iter()
            .filter(|value| value.kind() == ResolutionIssueKind::Unresolved)
            .map(ResolutionDiagnostic::target)
            .collect();
        assert_eq!(
            unresolved,
            ["crate::Missing"],
            "a symbolic candidate that matches nothing stays unresolved"
        );
    }

    #[test]
    fn a_super_rooted_item_resolves_to_the_file_declaring_the_parent_module() {
        let root = tempfile::tempdir().unwrap();
        for (path, source) in [
            ("Cargo.toml", "[package]\nname='rooted'\nversion='0.1.0'\n"),
            (
                "src/lib.rs",
                "mod builder;\nmod edge;\nmod widget;\npub struct Root;\n",
            ),
            // The parent module lives inside its own directory.
            (
                "src/builder/mod.rs",
                "mod manifests;\npub struct DockerMode;\n",
            ),
            // A chain of several `super` segments would name this file's own
            // parent, two levels below the module it counts from.
            (
                "src/builder/manifests.rs",
                "use super::DockerMode;\nuse super::super::*;\n",
            ),
            // The parent module lives beside its directory, 2018 style.
            (
                "src/widget.rs",
                "mod deep;\nmod parts;\npub struct Frame;\n",
            ),
            (
                "src/widget/parts.rs",
                "use super::Frame;\nuse self::helper;\npub fn helper() -> u32 { 1 }\n",
            ),
            // The declaring file is the module of its own directory, so its
            // parent is the directory above rather than beside it.
            ("src/widget/deep/mod.rs", "use super::Frame;\n"),
            // The parent of a source-root module is the crate root.
            ("src/edge.rs", "use super::Root;\n"),
            // Nothing declares this file, so nothing can be named.
            ("standalone/loose.rs", "use super::Nothing;\n"),
        ] {
            let file = root.path().join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, source).unwrap();
        }

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let mut edges: Vec<_> = report
            .dependency_edges()
            .iter()
            .filter(|edge| edge.relation() == smackdebt_analysis::StaticRelationKind::Uses)
            .map(|edge| {
                (
                    report.files()[edge.source().index()].path(),
                    report.files()[edge.target().index()].path(),
                )
            })
            .collect();
        edges.sort_unstable();
        // Every edge, so a fallback that names a module the reference did not
        // ask for fails here instead of hiding among the ones it did.
        assert_eq!(
            edges,
            [
                // A directory module declares its children.
                ("src/builder/manifests.rs", "src/builder/mod.rs"),
                // The crate root declares the modules of the source root.
                ("src/edge.rs", "src/lib.rs"),
                // A module file beside its directory declares the children of
                // that directory, whether they are files or directories.
                ("src/widget/deep/mod.rs", "src/widget.rs"),
                ("src/widget/parts.rs", "src/widget.rs"),
            ],
            "a reference resolves to the module that declares its root"
        );
        let unresolved: Vec<_> = report
            .resolution_diagnostics()
            .iter()
            .filter(|value| value.kind() == ResolutionIssueKind::Unresolved)
            .map(ResolutionDiagnostic::target)
            .collect();
        assert_eq!(
            unresolved,
            ["super::super::*", "super::Nothing"],
            "a module the walk cannot name exactly keeps its absence: {edges:?}"
        );
    }

    #[test]
    fn an_import_of_a_non_source_file_is_an_asset_rather_than_a_hole() {
        let root = tempfile::tempdir().unwrap();
        for (path, source) in [
            ("app/package.json", "{\"name\":\"app\"}\n"),
            // Three assets an importer reads for their bytes: one carries a
            // query suffix, one a fragment, one neither.
            (
                "app/main.ts",
                "import raw from './config.yaml?raw';\nimport icon from './logo.svg#glyph';\nimport theme from './theme.css';\nimport helper from './helper';\nexport default [raw, icon, theme, helper];\n",
            ),
            ("app/helper.ts", "export default 1;\n"),
            ("app/config.yaml", "name: fixture\n"),
            ("app/logo.svg", "<svg />\n"),
            ("app/theme.css", ".a { color: red; }\n"),
            ("core/package.json", "{\"name\":\"core\"}\n"),
            // The scope guard: a source extension that matches nothing, and a
            // target that names no extension at all, are still holes.
            (
                "core/main.ts",
                "import gone from './gone.ts';\nimport absent from './absent';\nexport default [gone, absent];\n",
            ),
        ] {
            let file = root.path().join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, source).unwrap();
        }

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let rows: Vec<_> = report
            .resolution_diagnostics()
            .iter()
            .map(|value| {
                format!(
                    "{} · {:?} · {}",
                    value.target(),
                    value.kind(),
                    value.reason()
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                "./config.yaml?raw · Asset · target is an asset",
                "./logo.svg#glyph · Asset · target is an asset",
                "./theme.css · Asset · target is an asset",
                "./absent · Unresolved · no repository file matches",
                "./gone.ts · Unresolved · no repository file matches",
            ],
            "an asset keeps its disclosure under its own reason"
        );

        let coverage = report.dependency_coverage();
        assert_eq!(
            coverage.unresolved_internal_uses(),
            2,
            "only the two holes are unresolved"
        );
        assert_eq!(
            coverage.context_relations(),
            3,
            "the assets stay counted, outside the verdict graph"
        );
        assert_eq!(coverage.resolved_internal_uses(), 1);

        let evidence = report.graph_evidence();
        assert_eq!(evidence.unresolved_internal(), 2);
        let core = report
            .files()
            .iter()
            .find(|file| file.path() == "core/main.ts")
            .and_then(smackdebt_analysis::FileRecord::package)
            .expect("the hole belongs to a package");
        assert_eq!(
            evidence.incomplete_packages(),
            [core],
            "importing an asset leaves its package complete"
        );
    }

    #[test]
    fn an_asset_is_read_from_the_paths_a_reference_was_looked_for_under() {
        // The spellings each language hands the resolver. A path language
        // offers the written name plus the extensions it knows; a language
        // that reads dotted module notation offers only the paths it derived
        // from that name, and the written form never appears at all.
        let path = |target: &str| {
            let mut values = vec![target.to_owned()];
            for extension in [".js", ".ts"] {
                values.push(format!("{target}{extension}"));
                values.push(format!("{target}/index{extension}"));
            }
            values
        };
        let module = |values: &[&str]| {
            values
                .iter()
                .map(|value| (*value).to_owned())
                .collect::<Vec<_>>()
        };

        for (target, candidates, asset, reading) in [
            (
                "./x.yaml?raw",
                path("./x.yaml?raw"),
                true,
                "a query suffix is stripped before the extension is read",
            ),
            (
                "./x.md",
                path("./x.md"),
                true,
                "a plain unclaimed extension needs no suffix",
            ),
            (
                "./capabilities",
                path("./capabilities"),
                false,
                "a spelling that names no extension claims nothing",
            ),
            (
                "./missing.ts",
                path("./missing.ts"),
                false,
                "a source extension that matched nothing is still a hole",
            ),
            // `from ..core import thing`. Read as a path the target carries
            // the extension `core`, so only the candidates show it is a module
            // name and that the file it misses is a real hole.
            (
                "..core",
                module(&["../core.py", "../core/__init__.py"]),
                false,
                "dotted module notation never reaches here as a path",
            ),
            (
                ".missing.thing",
                module(&["./missing/thing.py", "./missing/thing/__init__.py"]),
                false,
                "a dotted module chain is not a path either",
            ),
            // Pinned rather than preferred: an absent `./webpack.config.js`
            // imported as `./webpack.config` reads as an asset, because
            // `config` is an extension no language claims. The candidates
            // cannot settle it — the literal spelling is one of them. It costs
            // a reader nothing: the row keeps its target and its line in JSON,
            // and is only held out of a count that would otherwise claim a
            // broken graph on a guess.
            (
                "./webpack.config",
                path("./webpack.config"),
                true,
                "an unclaimed extension on a written path reads as an asset",
            ),
        ] {
            assert_eq!(
                candidates_name_an_asset(&candidates),
                asset,
                "{target}: {reading}"
            );
        }
    }

    #[test]
    fn a_python_relative_import_of_a_missing_module_is_still_a_hole() {
        let root = tempfile::tempdir().unwrap();
        for (path, source) in [
            ("pyproject.toml", "[project]\nname='service'\n"),
            ("src/__init__.py", "\n"),
            ("src/api/__init__.py", "\n"),
            // `..core` names the module `src/core`, which nothing declares.
            // Read as a path it would carry the extension `core` and vanish
            // under an asset row, taking the package's incompleteness with it.
            (
                "src/api/handler.py",
                "from ..core import thing\n\n\ndef handle():\n    return thing\n",
            ),
        ] {
            let file = root.path().join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, source).unwrap();
        }

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let rows: Vec<_> = report
            .resolution_diagnostics()
            .iter()
            .map(|value| {
                format!(
                    "{} · {:?} · {}",
                    value.target(),
                    value.kind(),
                    value.reason()
                )
            })
            .collect();
        assert_eq!(rows, ["..core · Unresolved · no repository file matches"]);
        assert_eq!(report.dependency_coverage().unresolved_internal_uses(), 1);

        let evidence = report.graph_evidence();
        assert!(
            !evidence.is_complete(),
            "a missing Python module leaves the graph incomplete"
        );
        assert_eq!(evidence.unresolved_internal(), 1);
        assert_eq!(evidence.incomplete_packages().len(), 1);
    }

    #[test]
    fn a_package_without_an_entry_file_keeps_a_package_scoped_edge() {
        let root = tempfile::tempdir().unwrap();
        for (path, source) in [
            (
                "crates/tool/Cargo.toml",
                "[package]\nname='acme-tool'\nversion='0.1.0'\n",
            ),
            (
                "crates/tool/other/thing.rs",
                "pub fn thing(value: i32) -> i32 { value }\n",
            ),
            (
                "crates/app/Cargo.toml",
                "[package]\nname='acme-app'\nversion='0.1.0'\n",
            ),
            (
                "crates/app/src/lib.rs",
                "use acme_tool::thing;\nuse acme_tool::other::more;\npub fn app(value: i32) -> i32 { thing(more(value)) }\n",
            ),
            (
                "crates/app/src/second.rs",
                "use acme_tool::thing;\npub fn second(value: i32) -> i32 { thing(value) }\n",
            ),
        ] {
            let file = root.path().join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, source).unwrap();
        }

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        assert_eq!(
            package_pairs(report),
            [("crates/app".to_owned(), "crates/tool".to_owned())]
        );
        let edge = &report.package_edges()[0];
        assert_eq!(
            (edge.file_pairs(), edge.references(), edge.file_edges()),
            (2, 3, [].as_slice()),
            "two files make three package-scoped references"
        );
        assert!(
            report
                .dependency_edges()
                .iter()
                .all(|edge| !report.files()[edge.target().index()]
                    .path()
                    .starts_with("crates/tool")),
            "a package without an entry file has no file-level target"
        );
        assert_eq!(report.dependency_coverage().resolved_internal_uses(), 3);
        assert!(
            report
                .external_dependencies()
                .iter()
                .all(|external| !external.target().starts_with("acme_tool")),
            "{:?}",
            report.external_dependencies()
        );
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
    fn source_role_precedence_keeps_explicit_rules_ahead_of_generated_javascript() {
        let generated = b"// @generated\nexport function work() {}\n";
        assert_eq!(
            classify_source_role(
                Path::new("tests/work.js"),
                generated,
                &[SourceRoleRule::example("tests/*.js")],
            ),
            Ok(SourceRole::Example)
        );
        assert_eq!(
            classify_source_role(Path::new("tests/work.js"), generated, &[]),
            Ok(SourceRole::Generated)
        );
        assert_eq!(
            classify_source_role(Path::new("tests/work.js"), b"function work() {}", &[]),
            Ok(SourceRole::Test)
        );
        assert_eq!(
            classify_source_role(Path::new("src/work.js"), b"function work() {}", &[]),
            Ok(SourceRole::Primary)
        );
        assert_eq!(
            classify_source_role(
                Path::new("src/vendor.min.js"),
                b"function work() {}",
                &[SourceRoleRule::primary("src/vendor.min.js")],
            ),
            Ok(SourceRole::Primary)
        );
        assert_eq!(
            classify_source_role(Path::new("tests/vendor.min.js"), b"function work() {}", &[],),
            Ok(SourceRole::Generated)
        );
        assert_eq!(
            classify_source_role(
                Path::new("src/client.bundle.ts"),
                b"function work() {}",
                &[],
            ),
            Ok(SourceRole::Primary)
        );
    }

    /// The guard that decides which files the dormancy rule may look at.
    ///
    /// Its two errors are not symmetric: missing module syntax lets a module be
    /// called dormant, while seeing it where there is none only spares a file.
    /// The table therefore leans on the spellings that could be missed.
    #[test]
    fn module_syntax_is_read_from_the_first_word_of_a_line() {
        let script = Path::new("public/js/widget.js");
        for (source, expected, why) in MODULE_SYNTAX_CASES {
            assert_eq!(
                declares_module_syntax(script, source.as_bytes()),
                *expected,
                "{why}: {source:?}"
            );
        }
        assert!(
            !declares_module_syntax(Path::new("src/widget.ts"), b"export const a = 1;\n"),
            "only the JavaScript a runtime loads as written is read at all"
        );
    }

    /// One source, the answer it must produce, and why that answer is right.
    const MODULE_SYNTAX_CASES: &[(&str, bool, &str)] = &[
        (
            "function a() {}\nexport function b() {}\n",
            true,
            "an export anywhere in the file counts",
        ),
        (
            "const a = 1;\nexport {a};\n",
            true,
            "a brace after the keyword counts",
        ),
        (
            "import\"./x\"\n",
            true,
            "a double quote with no space counts",
        ),
        ("import'./x'\n", true, "a single quote with no space counts"),
        (
            "import {\n  thing,\n} from './x';\n",
            true,
            "a multi-line import counts",
        ),
        (
            "import\n  { thing }\nfrom './x';\n",
            true,
            "the keyword ending its own line counts",
        ),
        ("export * from './x';\n", true, "a star counts"),
        ("  export default 1;\n", true, "leading indent is trimmed"),
        (
            "export {a};\r\n",
            true,
            "a carriage return does not hide the keyword",
        ),
        (
            "#!/usr/bin/env node\n(function () {\n  module.exports = 1;\n})();\n",
            false,
            "a UMD or CommonJS wrapper is still a script",
        ),
        (
            "const x = require('./x');\nwindow.x = x;\n",
            false,
            "a plain require is not module syntax the graph reads here",
        ),
        (
            "const exported = 1;\nconst important = 2;\n",
            false,
            "a longer word starting with the keyword is not the keyword",
        ),
        (
            "// export function b() {}\n",
            false,
            "the keyword is not the first word of that line",
        ),
    ];

    #[test]
    fn generated_javascript_content_uses_exact_size_and_density_edges() {
        let exact_edge = vec![b'x'; 65_536];
        for path in [
            "src/client.js",
            "src/client.mjs",
            "src/client.cjs",
            "src/client.jsx",
            "src/client.ts",
            "src/client.tsx",
        ] {
            assert!(
                has_generated_javascript_content(Path::new(path), &exact_edge),
                "{path}",
            );
        }

        let mut exact_lines = Vec::with_capacity(65_536);
        for _ in 0..127 {
            exact_lines.extend(std::iter::repeat_n(b'x', 511));
            exact_lines.push(b'\n');
        }
        exact_lines.extend(std::iter::repeat_n(b'x', 512));
        assert_eq!(exact_lines.len(), 65_536);
        assert!(has_generated_javascript_content(
            Path::new("src/client.js"),
            &exact_lines,
        ));

        let below_size = vec![b'x'; 65_535];
        assert!(!has_generated_javascript_content(
            Path::new("src/client.js"),
            &below_size,
        ));

        let mut below_density = exact_lines;
        below_density[255] = b'\n';
        assert!(!has_generated_javascript_content(
            Path::new("src/client.js"),
            &below_density,
        ));
        assert!(!has_generated_javascript_content(
            Path::new("src/client.vue"),
            &exact_edge,
        ));
    }

    #[test]
    fn ordinary_javascript_content_and_common_directories_remain_primary() {
        let mut multiline = Vec::with_capacity(65_536);
        for _ in 0..256 {
            multiline.extend(std::iter::repeat_n(b'x', 255));
            multiline.push(b'\n');
        }
        assert_eq!(multiline.len(), 65_536);
        assert!(!has_generated_javascript_content(
            Path::new("src/large.js"),
            &multiline,
        ));

        for path in ["public/app.js", "share/tool.js", "assets/editor.js"] {
            assert_eq!(
                classify_source_role(Path::new(path), b"export const value = 1;", &[]),
                Ok(SourceRole::Primary),
                "{path}",
            );
        }
        assert_eq!(
            classify_source_role(
                Path::new("src/authored.js"),
                b"export const compact = true;",
                &[],
            ),
            Ok(SourceRole::Primary)
        );
    }

    #[test]
    fn explicit_role_conflicts_report_every_disagreeing_role() {
        let error = classify_source_role(
            Path::new("src/work.js"),
            b"function work() {}",
            &[
                SourceRoleRule::test("src/*.js"),
                SourceRoleRule::fixture("src/work.js"),
            ],
        )
        .unwrap_err();
        assert_eq!(error, "test, fixture");
    }

    #[test]
    fn every_source_role_is_retained_and_only_verdict_roles_change_health() {
        let mut analyzer = Analyzer::default();
        let source = b"function work(a, b) { if (a) { if (b) { return 1; } } return 0; }\n";
        let policy = HealthPolicy::new(
            smackdebt_analysis::Thresholds::new(1, 2),
            smackdebt_analysis::Thresholds::new(1, 2),
            smackdebt_analysis::Thresholds::new(1, 2),
            smackdebt_analysis::Thresholds::new(4, 7),
            smackdebt_analysis::Thresholds::new(6, 9),
        );
        for role in [
            SourceRole::Primary,
            SourceRole::Test,
            SourceRole::Example,
            SourceRole::Benchmark,
            SourceRole::Fixture,
            SourceRole::Generated,
        ] {
            let analysis = analyzer
                .analyze(Path::new("src/work.js"), source.to_vec())
                .unwrap();
            let rated = rate_file(analysis, role, policy, false);
            assert_eq!(rated.role, role);
            assert!(!rated.debt.is_empty());
            if role.affects_verdict() {
                assert!(rated.health.debt() > 0, "{role:?}");
            } else {
                assert_eq!(rated.health.debt(), 0, "{role:?}");
            }
        }
    }

    #[test]
    fn workspace_test_fixture_has_one_role_and_stays_outside_the_verdict() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("crates/cli/tests/fixtures")).unwrap();
        fs::write(
            root.path().join("crates/cli/Cargo.toml"),
            "[package]\nname='fixture'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            root.path().join("crates/cli/tests/fixtures/complex.js"),
            "export function fixture(a, b) { if (a) { if (b) { return 1; } } return 0; }\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path()).with_thresholds(
            (1, 2),
            (1, 2),
            (1, 2),
            (4, 7),
            (6, 9),
        ))
        .unwrap();
        let report = result.report();
        let fixture = report
            .files()
            .iter()
            .find(|file| file.path() == "crates/cli/tests/fixtures/complex.js")
            .unwrap();

        assert_eq!(fixture.role(), SourceRole::Fixture);
        assert_eq!(fixture.health(), HealthCounts::default());
        assert_eq!(fixture.coverage().context_files(), 1);
        assert_eq!(
            report.scopes()[report.root().unwrap().index()].health(),
            HealthCounts::default()
        );
    }

    #[test]
    fn recovered_findings_are_advisory_and_do_not_change_health() {
        let analysis = Analyzer::default()
            .analyze(
                Path::new("src/work.py"),
                b"def broken(:\n    if yes:\n        if more:\n            pass\n".to_vec(),
            )
            .unwrap();
        assert_eq!(analysis.parse_status().trust(), SourceTrust::Advisory);
        let rated = rate_file(
            analysis,
            SourceRole::Primary,
            HealthPolicy::new(
                smackdebt_analysis::Thresholds::new(1, 2),
                smackdebt_analysis::Thresholds::new(1, 2),
                smackdebt_analysis::Thresholds::new(1, 2),
                smackdebt_analysis::Thresholds::new(4, 7),
                smackdebt_analysis::Thresholds::new(6, 9),
            ),
            false,
        );
        assert_eq!(rated.health, HealthCounts::default());
        assert!(!rated.debt.is_empty());
    }

    #[test]
    fn recovered_dependency_is_retained_but_cannot_enter_the_verdict_graph() {
        let root = tempfile::tempdir().unwrap();
        for package in ["app", "core"] {
            fs::create_dir_all(root.path().join(package)).unwrap();
            fs::write(root.path().join(package).join("package.json"), "{}").unwrap();
        }
        fs::write(
            root.path().join("app/main.js"),
            "import core from '../core/main';\nfunction broken( { if (a) { if (b) { core(); } }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("core/main.js"),
            "export default function core() {}\n",
        )
        .unwrap();
        fs::create_dir_all(root.path().join("app/fixtures")).unwrap();
        fs::write(
            root.path().join("app/fixtures/context.js"),
            "export function context() {}\n",
        )
        .unwrap();
        fs::write(
            root.path().join("app/unsupported.kt"),
            "fun unsupported() = Unit\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path()).with_thresholds(
            (1, 2),
            (1, 2),
            (1, 2),
            (4, 7),
            (6, 9),
        ))
        .unwrap();
        let report = result.report();
        let recovered = report
            .files()
            .iter()
            .find(|file| file.path() == "app/main.js")
            .unwrap();
        assert_eq!(recovered.trust(), SourceTrust::Advisory);
        assert_eq!(recovered.health(), HealthCounts::default());
        assert_eq!(recovered.coverage().clean_files(), 0);
        assert_eq!(recovered.coverage().recovered_files(), 1);
        let root_coverage = report.scopes()[report.root().unwrap().index()].coverage();
        assert_eq!(root_coverage.clean_files(), 1);
        assert_eq!(root_coverage.recovered_files(), 1);
        assert_eq!(root_coverage.unsupported_files(), 1);
        assert_eq!(root_coverage.failed_files(), 0);
        assert_eq!(root_coverage.context_files(), 1);
        assert_eq!(root_coverage.selected_files(), 4);
        assert!(report.findings().iter().any(|finding| {
            finding.file() == recovered.id() && finding.trust() == SourceTrust::Advisory
        }));
        assert_eq!(report.dependency_edges().len(), 1);
        assert_eq!(report.dependency_edges()[0].trust(), SourceTrust::Advisory);
        assert!(report.package_edges().is_empty());
        assert!(report.architecture_findings().is_empty());
        assert_eq!(report.dependency_coverage().context_relations(), 1);
        assert_eq!(report.dependency_coverage().total(), 1);
    }

    #[test]
    fn fixture_dependency_is_visible_without_affecting_architecture_health() {
        let root = tempfile::tempdir().unwrap();
        for package in ["app", "core"] {
            fs::create_dir_all(root.path().join(package)).unwrap();
            fs::write(root.path().join(package).join("package.json"), "{}").unwrap();
        }
        fs::create_dir_all(root.path().join("app/fixtures")).unwrap();
        fs::write(
            root.path().join("app/fixtures/main.js"),
            "import core from '../../core/main';\nimport { helper } from './helper';\nexport function fixture() { helper(); core(); }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("app/fixtures/helper.js"),
            "import { fixture } from './main';\nexport function helper() { fixture(); }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("core/main.js"),
            "export default function core() {}\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        assert_eq!(report.dependency_edges().len(), 3);
        assert!(
            report
                .dependency_edges()
                .iter()
                .all(|edge| edge.role() == SourceRole::Fixture && !edge.affects_verdict())
        );
        assert!(report.package_edges().is_empty());
        assert!(report.architecture_findings().is_empty());
        assert_eq!(report.dependency_coverage().context_relations(), 3);
        assert_eq!(report.dependency_coverage().total(), 3);
        let root_coverage = report.scopes()[report.root().unwrap().index()].coverage();
        assert_eq!(root_coverage.clean_files(), 1);
        assert_eq!(root_coverage.context_files(), 2);
        assert_eq!(root_coverage.recovered_files(), 0);
    }

    #[test]
    fn generated_dependency_is_visible_without_affecting_architecture_health() {
        let root = tempfile::tempdir().unwrap();
        for package in ["app", "core"] {
            fs::create_dir_all(root.path().join(package)).unwrap();
            fs::write(root.path().join(package).join("package.json"), "{}").unwrap();
        }
        fs::create_dir_all(root.path().join("app/generated")).unwrap();
        fs::write(
            root.path().join("app/generated/main.js"),
            "// @generated\nimport core from '../../core/main';\nimport { helper } from './helper';\nexport function generated() { helper(); core(); }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("app/generated/helper.js"),
            "// @generated\nimport { generated } from './main';\nexport function helper() { generated(); }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("core/main.js"),
            "export default function core() {}\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        assert_eq!(report.dependency_edges().len(), 3);
        assert!(report.dependency_edges().iter().all(|edge| {
            edge.role() == SourceRole::Generated
                && edge.relation() == smackdebt_analysis::StaticRelationKind::Uses
                && !edge.affects_verdict()
        }));
        assert!(report.package_edges().is_empty());
        assert!(report.architecture_findings().is_empty());
        assert_eq!(report.dependency_coverage().context_relations(), 3);
        assert_eq!(report.dependency_coverage().total(), 3);
    }

    #[test]
    fn recovered_worktree_units_remain_advisory_without_diff_verdicts() {
        let root = tempfile::tempdir().unwrap();
        git(root.path(), ["init", "-q"]);
        git(
            root.path(),
            ["config", "user.email", "test@example.invalid"],
        );
        git(root.path(), ["config", "user.name", "Smackdebt Test"]);
        fs::write(
            root.path().join("main.js"),
            "export function work() { return 1; }\n",
        )
        .unwrap();
        git(root.path(), ["add", "-A"]);
        git(root.path(), ["commit", "-qm", "base"]);
        fs::write(
            root.path().join("main.js"),
            "export function work( { if (a) { if (b) { return 1; } }\n",
        )
        .unwrap();

        let result = analyze_diff(
            &DiffRequest::new(root.path())
                .with_reference("HEAD")
                .with_thresholds((1, 2), (1, 2), (1, 2), (4, 7), (6, 9)),
        )
        .unwrap();
        let report = result.report();
        assert!(report.comparisons().is_empty());
        assert!(!report.findings().is_empty());
        assert!(
            report
                .findings()
                .iter()
                .all(|finding| finding.trust() == SourceTrust::Advisory)
        );
        assert_eq!(report.files()[0].health(), HealthCounts::default());
        assert_eq!(report.files()[0].coverage().clean_files(), 0);
        assert_eq!(report.files()[0].coverage().recovered_files(), 1);
        let root_coverage = report.scopes()[report.root().unwrap().index()].coverage();
        assert_eq!(root_coverage.clean_files(), 0);
        assert_eq!(root_coverage.recovered_files(), 1);
        assert_eq!(root_coverage.unsupported_files(), 0);
        assert_eq!(root_coverage.failed_files(), 0);
        assert_eq!(root_coverage.context_files(), 0);
        assert_eq!(root_coverage.selected_files(), 1);
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
    fn concentrated_package_knowledge_is_a_watch_finding_of_counts_only() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "owner@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Sole Owner"]);
        for revision in 0..10 {
            fs::write(
                repository_path.join("owned.rs"),
                format!("pub fn owned() -> i32 {{ {revision} }}\n"),
            )
            .unwrap();
            git(repository_path, ["add", "."]);
            git(repository_path, ["commit", "-qm", "change"]);
        }

        let analyzed = analyze_codebase(&CodebaseRequest::new(repository_path)).unwrap();
        let report = analyzed.report();
        let findings = report.knowledge_concentration_findings();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rating(), Rating::Watch);
        let concentration = findings[0].concentration();
        assert_eq!(
            (
                concentration.contributor_count(),
                concentration.numerator(),
                concentration.denominator()
            ),
            (1, 10, 10)
        );
        assert!(report.evolutionary_findings().is_empty());
        // No contributor identity reaches any retained report value.
        let retained = format!("{report:?}");
        assert!(!retained.contains("Sole Owner"));
        assert!(!retained.contains("owner@example.invalid"));
    }

    #[test]
    fn serial_and_parallel_runs_derive_identical_signal_tables() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        // More files than the parallel cutover, so the automatic run really
        // splits the work across workers.
        for index in 0..120 {
            fs::write(
                repository_path.join(format!("file{index}.rs")),
                format!("pub fn work{index}(value: i32) -> i32 {{ value + {index} }}\n"),
            )
            .unwrap();
        }
        for revision in 0..6 {
            fs::write(
                repository_path.join("file0.rs"),
                format!("pub fn work0(value: i32) -> i32 {{ value + {revision} }}\n"),
            )
            .unwrap();
            git(repository_path, ["add", "."]);
            git(repository_path, ["commit", "-qm", "change"]);
        }

        let request = CodebaseRequest::new(repository_path).with_size_thresholds((1, 2), (1, 2));
        let serial = request
            .clone()
            .with_width(ExecutionWidth::fixed(1).unwrap())
            .analyze()
            .unwrap();
        let parallel = request
            .with_width(ExecutionWidth::Automatic)
            .analyze()
            .unwrap();
        assert!(!serial.report().hotspots().is_empty());
        assert!(!serial.report().size_findings().is_empty());
        assert!(!serial.report().orphan_files().is_empty());
        assert_eq!(serial.report().hotspots(), parallel.report().hotspots());
        assert_eq!(
            serial.report().size_findings(),
            parallel.report().size_findings()
        );
        assert_eq!(
            serial.report().orphan_files(),
            parallel.report().orphan_files()
        );
        assert_eq!(
            serial.report().stable_dependency_findings(),
            parallel.report().stable_dependency_findings()
        );
        assert_eq!(
            serial.report().knowledge_concentration_findings(),
            parallel.report().knowledge_concentration_findings()
        );
        // The verdict is derived from those tables, so both widths answer with
        // the same tier, counts, selection, and worst offender.
        assert!(serial.report().verdict().is_some());
        assert_eq!(serial.report().verdict(), parallel.report().verdict());
        assert!(
            !serial
                .report()
                .verdict()
                .unwrap()
                .selection()
                .has_duplicate_identity()
        );
    }

    /// A function whose nesting alone rates High on cognitive complexity.
    fn nested_source(seed: usize) -> String {
        let mut source = format!("pub fn work{seed}(value: i32) -> i32 {{\n");
        for depth in 0..8 {
            source.push_str(&format!(
                "{}if value > {depth} {{\n",
                "    ".repeat(depth + 1)
            ));
        }
        source.push_str(&format!("{}return 1;\n", "    ".repeat(9)));
        for depth in (0..8).rev() {
            source.push_str(&format!("{}}}\n", "    ".repeat(depth + 1)));
        }
        source.push_str("    value\n}\n");
        source
    }

    #[test]
    fn serial_and_parallel_runs_cluster_identical_problem_cards() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        // More files than the parallel cutover, so the automatic run really
        // splits the work across workers, and every sixth file carries High
        // debt so cards exist to compare.
        for index in 0..120 {
            let source = if index % 6 == 0 {
                nested_source(index)
            } else {
                format!("pub fn work{index}(value: i32) -> i32 {{ value + {index} }}\n")
            };
            fs::write(repository_path.join(format!("file{index}.rs")), source).unwrap();
        }
        // One file changes often enough to be hot, so heat reaches a card too.
        for revision in 0..6 {
            fs::write(repository_path.join("file0.rs"), nested_source(revision)).unwrap();
            git(repository_path, ["add", "."]);
            git(repository_path, ["commit", "-qm", "change"]);
        }

        let request = CodebaseRequest::new(repository_path).with_size_thresholds((1, 2), (1, 2));
        let serial = request
            .clone()
            .with_width(ExecutionWidth::fixed(1).unwrap())
            .analyze()
            .unwrap();
        let parallel = request
            .with_width(ExecutionWidth::Automatic)
            .analyze()
            .unwrap();
        let cards = serial.report().problems();
        assert!(!cards.is_empty());
        // Clustering reads report tables only, so width cannot move a card or
        // its position.
        assert_eq!(cards, parallel.report().problems());
        assert_eq!(duplicate_claim(cards), None);
        assert!(
            cards
                .iter()
                .any(|card| card.pattern() == ProblemPattern::HotMess),
            "the file that changes often carries its heat into a card"
        );
        // Coverage over a real report: every retained finding of every
        // claimable table reaches exactly one card, so no table can be dropped
        // from the clustering input without this failing.
        let report = serial.report();
        let claimed: BTreeSet<_> = cards
            .iter()
            .flat_map(|card| card.claimed_findings().iter().copied())
            .collect();
        assert_eq!(
            claimed.len(),
            report.findings().len()
                + report.size_findings().len()
                + report.architecture_findings().len()
                + report.evolutionary_findings().len()
                + report.knowledge_concentration_findings().len()
                + report.stable_dependency_findings().len()
        );
        assert!(!report.size_findings().is_empty());
    }

    #[test]
    fn stable_dependency_violations_and_orphan_files_are_derived_from_the_graph() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        for (path, source) in [
            ("a/package.json", "{\"name\":\"a\"}\n"),
            (
                "a/index.js",
                "import { b } from '../b/index.js';\nimport { other } from '../b/index.js';\nexport const a = b + other;\n",
            ),
            ("a/orphan.js", "export function orphan() { return 1; }\n"),
            ("b/package.json", "{\"name\":\"b\"}\n"),
            (
                "b/index.js",
                "import { e } from '../e/index.js';\nexport const b = e;\nexport const other = e;\n",
            ),
            ("c/package.json", "{\"name\":\"c\"}\n"),
            (
                "c/index.js",
                "import { a } from '../a/index.js';\nexport const c = a;\n",
            ),
            ("d/package.json", "{\"name\":\"d\"}\n"),
            (
                "d/index.js",
                "import { a } from '../a/index.js';\nexport const d = a;\n",
            ),
            ("e/package.json", "{\"name\":\"e\"}\n"),
            ("e/index.js", "export const e = 1;\n"),
        ] {
            let file = repository_path.join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, source).unwrap();
        }

        let analyzed = analyze_codebase(&CodebaseRequest::new(repository_path)).unwrap();
        let report = analyzed.report();
        let package_path =
            |package: PackageId| report.packages()[package.index()].path().to_owned();

        // Package `a` is more stable than `b`, so depending on it with two
        // references reverses the intended direction.
        let violations: Vec<_> = report
            .stable_dependency_findings()
            .iter()
            .map(|finding| {
                let evidence = finding.evidence();
                (
                    package_path(finding.source()),
                    package_path(finding.target()),
                    (
                        evidence.source().fan_in(),
                        evidence.source().fan_out(),
                        evidence.target().fan_in(),
                        evidence.target().fan_out(),
                    ),
                    evidence.references(),
                    finding.rating(),
                )
            })
            .collect();
        assert_eq!(
            violations,
            [(
                "a".to_owned(),
                "b".to_owned(),
                (2, 1, 1, 1),
                2,
                Rating::Watch
            )]
        );

        let orphans: Vec<_> = report
            .orphan_files()
            .iter()
            .map(|orphan| report.files()[orphan.file().index()].path().to_owned())
            .collect();
        assert_eq!(orphans, ["a/orphan.js".to_owned()]);
    }

    #[test]
    fn advisory_and_context_source_produce_no_size_finding_or_hotspot() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(
            repository_path.join("trusted.rs"),
            "struct Worker;\nimpl Worker {\n    fn work(&self) {\n        let a = 1;\n        let b = 2;\n    }\n}\n",
        )
        .unwrap();
        // Recovered source is advisory, so it contributes no default finding.
        fs::write(
            repository_path.join("recovered.rs"),
            "struct Broken;\nimpl Broken {\n    fn work(&self) {\n        let a = 1;\n        let b = 2;\n    }\n}\nfn unterminated(\n",
        )
        .unwrap();
        // Generated source is context, so it never affects a verdict.
        fs::write(
            repository_path.join("generated.rs"),
            "// @generated\nstruct Made;\nimpl Made {\n    fn work(&self) {\n        let a = 1;\n        let b = 2;\n    }\n}\n",
        )
        .unwrap();
        for revision in 0..5 {
            fs::write(
                repository_path.join("touch.txt"),
                format!("change {revision}\n"),
            )
            .unwrap();
            fs::write(
                repository_path.join("recovered.rs"),
                format!(
                    "struct Broken;\nimpl Broken {{\n    fn work(&self) {{\n        let a = {revision};\n        let b = 2;\n    }}\n}}\nfn unterminated(\n"
                ),
            )
            .unwrap();
            fs::write(
                repository_path.join("generated.rs"),
                format!(
                    "// @generated\nstruct Made;\nimpl Made {{\n    fn work(&self) {{\n        let a = {revision};\n        let b = 2;\n    }}\n}}\n"
                ),
            )
            .unwrap();
            fs::write(
                repository_path.join("trusted.rs"),
                format!(
                    "struct Worker;\nimpl Worker {{\n    fn work(&self) {{\n        let a = {revision};\n        let b = 2;\n    }}\n}}\n"
                ),
            )
            .unwrap();
            git(repository_path, ["add", "."]);
            git(repository_path, ["commit", "-qm", "change"]);
        }

        let analyzed = analyze_codebase(
            &CodebaseRequest::new(repository_path).with_size_thresholds((5, 10), (1, 2)),
        )
        .unwrap();
        let report = analyzed.report();
        let path = |file: FileId| report.files()[file.index()].path().to_owned();
        let sized: Vec<_> = report
            .size_findings()
            .iter()
            .map(|finding| path(finding.file()))
            .collect();
        assert_eq!(
            sized,
            ["trusted.rs".to_owned(), "trusted.rs".to_owned()],
            "only trusted verdict source is sized"
        );
        let hot: Vec<_> = report
            .hotspots()
            .iter()
            .map(|hotspot| path(hotspot.file()))
            .collect();
        assert_eq!(hot, ["trusted.rs".to_owned()]);
        // Every file was touched five times, so activity alone did not decide.
        for name in ["recovered.rs", "generated.rs"] {
            let file = report
                .files()
                .iter()
                .find(|file| file.path() == name)
                .expect("selected file");
            assert_eq!(
                file.activity().map(FileActivity::touches),
                Some(5),
                "{name} still records its activity"
            );
        }
    }

    #[test]
    fn hotspots_cross_rated_files_with_their_windowed_touch_count() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("cold.rs"), "pub fn cold() {}\n").unwrap();
        for revision in 0..5 {
            fs::write(
                repository_path.join("hot.rs"),
                format!("pub fn hot(value: i32) -> i32 {{ value + {revision} }}\n"),
            )
            .unwrap();
            git(repository_path, ["add", "."]);
            git(repository_path, ["commit", "-qm", "change"]);
        }

        let report = analyze_codebase(&CodebaseRequest::new(repository_path)).unwrap();
        let report = report.report();
        let named = |file: FileId| report.files()[file.index()].path().to_owned();
        let hotspots: Vec<_> = report
            .hotspots()
            .iter()
            .map(|hotspot| (named(hotspot.file()), hotspot.touches()))
            .collect();
        assert_eq!(hotspots, [("hot.rs".to_owned(), 5)]);
        assert!(
            report.is_hotspot(
                report
                    .files()
                    .iter()
                    .find(|file| file.path() == "hot.rs")
                    .unwrap()
                    .id()
            )
        );

        let below_boundary = analyze_codebase(
            &CodebaseRequest::new(repository_path).with_minimum_hotspot_touches(6),
        )
        .unwrap();
        assert!(below_boundary.report().hotspots().is_empty());
    }

    #[test]
    fn file_and_container_size_are_rated_outside_the_unit_health_counts() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(
            repository_path.join("big.rs"),
            "struct Worker;\nimpl Worker {\n    fn one(&self) {\n        let a = 1;\n        let b = 2;\n        let c = 3;\n    }\n    fn two(&self) {\n        let d = 4;\n        let e = 5;\n    }\n}\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "size"]);

        let report = analyze_codebase(
            &CodebaseRequest::new(repository_path).with_size_thresholds((10, 20), (4, 6)),
        )
        .unwrap();
        let report = report.report();
        let findings: Vec<_> = report
            .size_findings()
            .iter()
            .map(|finding| {
                (
                    report.files()[finding.file().index()].path().to_owned(),
                    finding.subject(),
                    finding.container().map(str::to_owned),
                    finding.value(),
                    finding.rating(),
                )
            })
            .collect();
        assert_eq!(
            findings,
            [
                (
                    "big.rs".to_owned(),
                    smackdebt_analysis::SizeSubject::File,
                    None,
                    12,
                    Rating::Watch
                ),
                (
                    "big.rs".to_owned(),
                    smackdebt_analysis::SizeSubject::Container,
                    Some("Worker".to_owned()),
                    5,
                    Rating::Watch
                ),
            ]
        );
        // Size findings never enter the unit verdict counts.
        let root_scope = report.root().unwrap();
        assert_eq!(
            report.scopes()[root_scope.index()].health(),
            HealthCounts::new(2, 0, 0)
        );
    }

    #[test]
    fn the_history_window_excludes_older_commits_and_coverage_states_it() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("old.rs"), "pub fn old() {}\n").unwrap();
        git(repository_path, ["add", "."]);
        git_dated(
            repository_path,
            "2001-02-03T04:05:06+00:00",
            ["commit", "-qm", "old"],
        );
        fs::write(repository_path.join("recent.rs"), "pub fn recent() {}\n").unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "recent"]);

        let windowed = analyze_codebase(&CodebaseRequest::new(repository_path)).unwrap();
        let coverage = windowed.report().history_coverage();
        assert_eq!(coverage.window_days(), Some(90));
        // The window filter runs inside the history stream, so the streamed
        // set is the windowed set and no boundary reject is counted.
        assert_eq!(coverage.commits(), 1);
        assert_eq!(coverage.window_excluded_commits(), 0);
        assert_eq!(coverage.eligible_commits(), 1);
        let touches = |report: &Report, path: &str| {
            let file = report
                .files()
                .iter()
                .find(|file| file.path() == path)
                .expect("selected file");
            report
                .file_history()
                .iter()
                .filter(|history| history.file() == file.id())
                .map(|history| history.touches())
                .sum::<u32>()
        };
        assert_eq!(touches(windowed.report(), "old.rs"), 0);
        assert_eq!(touches(windowed.report(), "recent.rs"), 1);

        let complete =
            analyze_codebase(&CodebaseRequest::new(repository_path).with_history_days(36_500))
                .unwrap();
        let coverage = complete.report().history_coverage();
        assert_eq!(coverage.window_days(), Some(36_500));
        assert_eq!(coverage.window_excluded_commits(), 0);
        assert_eq!(coverage.eligible_commits(), 2);
        assert_eq!(touches(complete.report(), "old.rs"), 1);
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
        fs::write(
            root.path().join(".smackdebt.toml"),
            "[source_roles]\ngenerated = ['old.js', 'new.js']\n",
        )
        .unwrap();
        fs::write(root.path().join("old.js"), "export const value = 1;\n").unwrap();
        git(root.path(), ["add", "-A"]);
        git(root.path(), ["commit", "-qm", "initial old path"]);
        fs::rename(root.path().join("old.js"), root.path().join("new.js")).unwrap();
        git(root.path(), ["add", "-A"]);
        git(root.path(), ["commit", "-qm", "rename old to new"]);
        fs::write(root.path().join("old.js"), "export const reused = 2;\n").unwrap();
        git(root.path(), ["add", "-A"]);
        git(root.path(), ["commit", "-qm", "reuse old path"]);

        let result = analyze_codebase(
            &CodebaseRequest::new(root.path())
                .with_history_days(36_500)
                .with_role_rules(vec![
                    SourceRoleRule::generated("old.js"),
                    SourceRoleRule::generated("new.js"),
                ]),
        )
        .unwrap();
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
        assert!(report.file_history().iter().all(|history| {
            history.role() == SourceRole::Generated && history.trust() == SourceTrust::Trusted
        }));
        assert_eq!(report.history_coverage().eligible_commits(), 0);
        assert_eq!(report.history_coverage().mapped_eligible_changes(), 0);
        assert_eq!(report.history_coverage().context_changes(), 2);
        assert_eq!(report.history_coverage().rename_gaps(), 1);
        assert_eq!(report.history_coverage().excluded_changes(), 2);
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
        assert_eq!(result.report().packages().len(), 1);
        assert_eq!(result.report().packages()[0].path(), ".");
    }

    #[test]
    fn package_rows_keep_empty_packages_and_discovery_ids() {
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
        assert_eq!(result.report().package_graph().len(), 3);
        let packages: Vec<_> = result
            .report()
            .packages()
            .iter()
            .map(|package| {
                let scope = &result.report().scopes()[package.scope().index()];
                assert_eq!(scope.kind(), ScopeKind::Package);
                assert_eq!(scope.name(), package.path());
                (
                    package.id().index(),
                    package.path(),
                    package.presence(),
                    package.scope(),
                )
            })
            .collect();
        assert_eq!(
            packages,
            [
                (
                    0,
                    "a",
                    smackdebt_analysis::PackagePresence::Current,
                    ScopeId::from_index(1)
                ),
                (
                    1,
                    "b",
                    smackdebt_analysis::PackagePresence::Current,
                    ScopeId::from_index(2)
                ),
                (
                    2,
                    "c",
                    smackdebt_analysis::PackagePresence::Current,
                    ScopeId::from_index(3)
                ),
            ]
        );
        let package_ids: Vec<_> = result
            .report()
            .files()
            .iter()
            .map(|file| file.package().unwrap().index())
            .collect();
        assert_eq!(package_ids, [0, 2]);
    }

    /// A path view is a scope of the repository report, so it carries the
    /// repository's package table and points at one row of it.
    #[test]
    fn path_view_keeps_the_repository_package_table() {
        let root = repository();
        let repo = root.path().join("repo");
        for package in ["a", "b"] {
            fs::create_dir_all(repo.join(package)).unwrap();
            fs::write(repo.join(package).join("package.json"), "{}").unwrap();
            fs::write(
                repo.join(package).join("main.js"),
                "export function work() { return 1; }\n",
            )
            .unwrap();
        }

        let codebase = analyze_codebase(&CodebaseRequest::new(&repo)).unwrap();
        let path = analyze_codebase(&CodebaseRequest::new(repo.join("b"))).unwrap();
        let package_paths = |report: &Report| {
            report
                .packages()
                .iter()
                .map(|package| (package.id(), package.path().to_owned()))
                .collect::<Vec<_>>()
        };
        assert_eq!(package_paths(codebase.report()).len(), 2);
        assert_eq!(
            package_paths(path.report()),
            package_paths(codebase.report())
        );
        let selected = path.selected_scope().unwrap();
        assert_eq!(path.report().scopes()[selected.index()].name(), "b");
    }

    #[test]
    fn diff_appends_base_only_packages_after_current_ids() {
        let root = tempfile::tempdir().unwrap();
        git(root.path(), ["init", "-q"]);
        git(
            root.path(),
            ["config", "user.email", "test@example.invalid"],
        );
        git(root.path(), ["config", "user.name", "Smackdebt Test"]);
        for package in ["a", "m"] {
            fs::create_dir_all(root.path().join(package)).unwrap();
            fs::write(root.path().join(package).join("package.json"), "{}").unwrap();
            fs::write(
                root.path().join(package).join("main.js"),
                "export function work() { return 1; }\n",
            )
            .unwrap();
        }
        git(root.path(), ["add", "-A"]);
        git(root.path(), ["commit", "-qm", "base"]);
        fs::remove_dir_all(root.path().join("m")).unwrap();
        fs::create_dir_all(root.path().join("z")).unwrap();
        fs::write(root.path().join("z/package.json"), "{}").unwrap();
        fs::write(
            root.path().join("z/main.js"),
            "export function work() { return 1; }\n",
        )
        .unwrap();

        let codebase = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let diff = analyze_diff(
            &DiffRequest::new(root.path())
                .with_reference("HEAD")
                .with_history_days(0),
        )
        .unwrap();
        let current: Vec<_> = codebase
            .report()
            .packages()
            .iter()
            .map(|package| (package.id(), package.path()))
            .collect();
        assert_eq!(
            current,
            [
                (PackageId::from_index(0), "a"),
                (PackageId::from_index(1), "z")
            ]
        );
        let packages: Vec<_> = diff
            .report()
            .packages()
            .iter()
            .map(|package| {
                let scope = &diff.report().scopes()[package.scope().index()];
                assert_eq!(scope.kind(), ScopeKind::Package);
                assert_eq!(scope.name(), package.path());
                (package.id(), package.path(), package.presence())
            })
            .collect();
        assert_eq!(
            packages,
            [
                (
                    PackageId::from_index(0),
                    "a",
                    smackdebt_analysis::PackagePresence::Current
                ),
                (
                    PackageId::from_index(1),
                    "z",
                    smackdebt_analysis::PackagePresence::Current
                ),
                (
                    PackageId::from_index(2),
                    "m",
                    smackdebt_analysis::PackagePresence::BaseOnly
                ),
            ]
        );
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
            "import core from '../core/b';\nimport ext from 'external';\nconst late = require(name);\nconst later = require(name);\nfunction app() {}\n",
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
            DependencyCoverage::new(2, 0, 0, 1, 2, 0, 0)
        );
        assert_eq!(report.dependency_coverage().total(), 5);
        assert_eq!(report.architecture_findings().len(), 1);
        assert_eq!(
            report.architecture_findings()[0].kind(),
            ArchitectureFindingKind::PackageCycle
        );
        assert_eq!(report.architecture_findings()[0].rating(), Rating::High);
        let external = &report.external_dependencies()[0];
        assert_eq!(
            external.relation(),
            smackdebt_analysis::StaticRelationKind::Uses
        );
        assert_eq!(external.role(), SourceRole::Primary);
        assert_eq!(external.trust(), SourceTrust::Trusted);
        assert_eq!(external.references(), 1);
        assert_eq!(
            external.locations(),
            &[smackdebt_analysis::SourceSpan::new(2, 2)]
        );
        let unresolved = report
            .resolution_diagnostics()
            .iter()
            .find(|value| value.kind() == ResolutionIssueKind::Unresolved)
            .expect("dynamic reference stays unresolved");
        assert_eq!(
            unresolved.relation(),
            smackdebt_analysis::StaticRelationKind::Uses
        );
        assert_eq!(unresolved.role(), SourceRole::Primary);
        assert_eq!(unresolved.trust(), SourceTrust::Trusted);
        assert_eq!(unresolved.references(), 2);
        assert_eq!(unresolved.span(), smackdebt_analysis::SourceSpan::new(3, 3));
        assert_eq!(
            unresolved.locations(),
            &[
                smackdebt_analysis::SourceSpan::new(3, 3),
                smackdebt_analysis::SourceSpan::new(4, 4),
            ]
        );
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
    fn package_aliases_jsonc_and_runtime_extensions_resolve_within_their_package() {
        let root = tempfile::tempdir().unwrap();
        for package in ["app", "core"] {
            fs::create_dir_all(root.path().join(package).join("src")).unwrap();
            fs::write(root.path().join(package).join("package.json"), "{}").unwrap();
            fs::write(
                root.path().join(package).join("src/main.ts"),
                "import value from '@/value.js?raw';\nexport default value;\n",
            )
            .unwrap();
            fs::write(
                root.path().join(package).join("src/value.ts"),
                "export default 1;\n",
            )
            .unwrap();
        }
        fs::write(
            root.path().join("app/tsconfig.json"),
            "{ extends: './tsconfig.base.json', }",
        )
        .unwrap();
        fs::write(
            root.path().join("app/tsconfig.base.json"),
            "{ compilerOptions: { paths: { '@/*': ['./src/*'], }, }, }",
        )
        .unwrap();
        fs::write(
            root.path().join("core/tsconfig.json"),
            "{ // package-local alias\n compilerOptions: { paths: { '@/*': ['./src/*'], }, }, }",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        assert_eq!(report.dependency_edges().len(), 2);
        for edge in report.dependency_edges() {
            let source = &report.files()[edge.source().index()];
            let target = &report.files()[edge.target().index()];
            assert_eq!(source.package(), target.package());
            assert!(target.path().ends_with("src/value.ts"));
        }
    }

    /// A recovered parse whose errors sit beside every fact still discloses
    /// itself, but it no longer costs its package the completeness that
    /// reach, core, and leakage are published from.
    #[test]
    fn recovery_beside_every_fact_leaves_the_package_complete() {
        let evidence = recovery_fixture("struct Broken {\n");
        assert!(evidence.complete);
        assert_eq!(evidence.parse_failures, 0);
        assert_eq!(evidence.incomplete_packages, 0);
        assert_eq!(evidence.unresolved_internal, 0);
        assert!(evidence.disclosed);
    }

    #[test]
    fn recovery_over_a_measured_unit_marks_the_package_incomplete() {
        let evidence = recovery_fixture("fn late(value: i32) -> i32 { helper(value broken( }\n");
        assert!(!evidence.complete);
        assert_eq!(evidence.parse_failures, 1);
        assert_eq!(evidence.incomplete_packages, 1);
        assert!(evidence.disclosed);
    }

    struct RecoveryEvidence {
        complete: bool,
        parse_failures: u32,
        incomplete_packages: usize,
        unresolved_internal: u32,
        disclosed: bool,
    }

    /// One package whose only Primary file recovers from `tail`.
    fn recovery_fixture(tail: &str) -> RecoveryEvidence {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname='recovery'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            root.path().join("helper.rs"),
            "pub fn helper(value: i32) -> i32 {\n    value\n}\n",
        )
        .unwrap();
        fs::write(
            root.path().join("main.rs"),
            format!(
                "mod helper;\nuse crate::helper::helper;\n\nfn work(value: i32) -> i32 {{\n    helper(value)\n}}\n\n{tail}"
            ),
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let evidence = report.graph_evidence();
        RecoveryEvidence {
            complete: evidence.is_complete(),
            parse_failures: evidence.parse_failures(),
            incomplete_packages: evidence.incomplete_packages().len(),
            unresolved_internal: evidence.unresolved_internal(),
            disclosed: report.diagnostics().iter().any(|diagnostic| {
                diagnostic.kind() == DiagnosticKind::ParseFailure
                    && diagnostic.message() == "parser recovered from syntax errors"
            }),
        }
    }

    /// A package closes over its own files, so its own evidence decides
    /// whether its reach may be stated.
    #[test]
    fn a_complete_package_states_the_reach_an_incomplete_one_withholds() {
        let root = tempfile::tempdir().unwrap();
        let files = smackdebt_analysis::PACKAGE_REACH_FILES;
        for package in ["app", "core"] {
            fs::create_dir_all(root.path().join(package)).unwrap();
            fs::write(root.path().join(package).join("package.json"), "{}").unwrap();
            for index in 0..files {
                let source = if index + 1 < files {
                    format!(
                        "import next from './unit{}';\nexport default next;\n",
                        index + 1
                    )
                } else {
                    "export default 1;\n".to_owned()
                };
                fs::write(
                    root.path().join(package).join(format!("unit{index}.js")),
                    source,
                )
                .unwrap();
            }
        }
        // One import of one primary file in `core` names nothing, which is
        // what the two packages' evidence differs by.
        fs::write(
            root.path().join("core/unread.js"),
            "import absent from './absent';\nexport default absent;\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let scope = |path: &str| {
            report
                .packages()
                .iter()
                .find(|package| package.path() == path)
                .expect("both packages are reported")
                .scope()
        };
        assert_eq!(report.graph_evidence().incomplete_packages().len(), 1);
        assert_eq!(report.graph_evidence().suppressed_reach(), 1);
        assert!(report.scope_verdict(scope("app")).reach().is_some());
        assert!(report.scope_verdict(scope("core")).reach().is_none());
    }

    /// A leakage finding is about two files, so one unread package withholds
    /// only the findings that name it.
    #[test]
    fn leakage_between_two_complete_packages_survives_a_third_incomplete_one() {
        let file = |index: usize, package: usize| {
            FileRecord::new(
                FileId::from_index(index),
                ScopeId::from_index(0),
                "src/unit.js",
                Coverage::default(),
                HealthCounts::default(),
            )
            .with_package(PackageId::from_index(package))
        };
        let files = [file(0, 0), file(1, 1), file(2, 2)];
        let evidence = GraphEvidence::new(vec![PackageId::from_index(2)], 0, 0, 0, Vec::new());
        let pair = |left: usize, right: usize| {
            smackdebt_analysis::FileChangeCoupling::new(
                FileId::from_index(left),
                FileId::from_index(right),
                4,
                5,
                2,
            )
        };
        let pairs = [pair(0, 1), pair(0, 2)];

        assert!(leakage_evidence_is_complete(&evidence, pairs[0], &files));
        assert!(!leakage_evidence_is_complete(&evidence, pairs[1], &files));
        let suppressed = pairs
            .into_iter()
            .filter(|pair| !leakage_evidence_is_complete(&evidence, *pair, &files))
            .count();
        assert_eq!(suppressed, 1);
    }

    /// A file the file dependency graph never reads cannot leave a hole in it.
    ///
    /// A fixture and a test are outside the graph the reach, core, and leakage
    /// facts are proved over, so an import either of them leaves unresolved
    /// hides nothing from those facts. The diagnostic is still published:
    /// what changes is only whether the package's evidence is called
    /// incomplete.
    #[test]
    fn an_unread_import_outside_the_graph_leaves_the_package_complete() {
        for path in ["tests/fixtures/dynamic.js", "tests/dynamic.test.js"] {
            let evidence = unread_import_evidence(path);
            assert!(evidence.complete, "{path}");
            assert_eq!(evidence.incomplete_packages, 0, "{path}");
            assert_eq!(evidence.diagnostics, 2, "{path}");
            assert_eq!(evidence.suppressed_reach, 0, "{path}");
        }
    }

    #[test]
    fn an_unread_import_in_a_graph_file_marks_its_package_incomplete() {
        let evidence = unread_import_evidence("src/dynamic.js");
        assert!(!evidence.complete);
        assert_eq!(evidence.incomplete_packages, 1);
        assert_eq!(evidence.diagnostics, 2);
    }

    struct UnreadImportEvidence {
        complete: bool,
        incomplete_packages: usize,
        diagnostics: usize,
        suppressed_reach: u32,
    }

    /// One JavaScript package whose file at `path` leaves two imports
    /// unresolved: a dynamic `require` and a name no file matches. Only that
    /// path differs between the cases, so only the role it carries can explain
    /// a difference in the evidence.
    fn unread_import_evidence(path: &str) -> UnreadImportEvidence {
        let root = tempfile::tempdir().unwrap();
        let unread = root.path().join(path);
        fs::create_dir_all(unread.parent().unwrap()).unwrap();
        fs::write(root.path().join("package.json"), "{}").unwrap();
        fs::write(
            root.path().join("main.js"),
            "import helper from './helper';\nexport default helper;\n",
        )
        .unwrap();
        fs::write(root.path().join("helper.js"), "export default 1;\n").unwrap();
        fs::write(
            unread,
            "import absent from './absent';\nconst late = require(moduleName);\nexport default [absent, late];\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let evidence = report.graph_evidence();
        UnreadImportEvidence {
            complete: evidence.is_complete(),
            incomplete_packages: evidence.incomplete_packages().len(),
            diagnostics: report
                .resolution_diagnostics()
                .iter()
                .filter(|diagnostic| diagnostic.kind() == ResolutionIssueKind::Unresolved)
                .count(),
            suppressed_reach: evidence.suppressed_reach(),
        }
    }

    #[test]
    fn invalid_resolution_configuration_marks_the_package_graph_incomplete() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("package.json"), "{}").unwrap();
        fs::write(root.path().join("tsconfig.json"), "{ compilerOptions:").unwrap();
        fs::write(
            root.path().join("main.ts"),
            "import value from '@/value';\nexport default value;\n",
        )
        .unwrap();
        fs::write(root.path().join("value.ts"), "export default 1;\n").unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let evidence = result.report().graph_evidence();
        assert!(!evidence.is_complete());
        assert_eq!(evidence.incomplete_packages().len(), 1);
        assert_eq!(evidence.configuration_failures().len(), 1);
        assert!(
            evidence.configuration_failures()[0]
                .reason()
                .contains("cannot parse")
        );
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
    fn rust_root_use_prefers_the_nearest_matching_module() {
        for (name, leaf, parent, expected_internal, expected_ambiguous) in [
            ("leaf", true, false, 1, 0),
            ("parent", false, true, 1, 0),
            ("both", true, true, 1, 0),
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
    fn rust_crate_qualified_use_resolves_from_the_crate_source_root() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("crates/app/src/core")).unwrap();
        fs::write(
            root.path().join("crates/app/Cargo.toml"),
            "[package]\nname='app'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            root.path().join("crates/app/src/lib.rs"),
            "use crate::core::work;\npub fn run() { work(); }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("crates/app/src/core.rs"),
            "pub fn work() {}\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        assert_eq!(result.report().dependency_edges().len(), 1);
        assert_eq!(
            result
                .report()
                .dependency_coverage()
                .resolved_internal_uses(),
            1
        );
        assert_eq!(result.report().dependency_coverage().total(), 1);
    }

    #[test]
    fn a_rust_test_scope_demotes_a_primary_reference_while_the_file_keeps_its_own_role() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname='scoped'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            root.path().join("helper.rs"),
            "pub fn work() -> u32 { 1 }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("shipped.rs"),
            "use crate::helper::work;\n#[cfg(test)]\nmod tests {\n    use crate::helper::work;\n    #[test]\n    fn covers() { assert_eq!(work(), 1); }\n}\npub fn ship() -> u32 { work() }\n",
        )
        .unwrap();
        fs::create_dir(root.path().join("fixtures")).unwrap();
        fs::write(
            root.path().join("fixtures/kept.rs"),
            "#[cfg(test)]\nmod tests {\n    use crate::helper::work;\n}\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let helper = file_id(report, "helper.rs");
        let shipped = file_id(report, "shipped.rs");
        let fixture = file_id(report, "fixtures/kept.rs");

        let mut shipped_roles: Vec<_> = report
            .dependency_edges()
            .iter()
            .filter(|edge| edge.source() == shipped && edge.target() == helper)
            .map(|edge| (edge.role(), edge.relation(), edge.references()))
            .collect();
        shipped_roles.sort();
        assert_eq!(
            shipped_roles,
            [
                (
                    SourceRole::Primary,
                    smackdebt_analysis::StaticRelationKind::Uses,
                    1
                ),
                (
                    SourceRole::Test,
                    smackdebt_analysis::StaticRelationKind::Uses,
                    1
                ),
            ]
        );
        assert!(
            report
                .dependency_edges()
                .iter()
                .filter(|edge| edge.source() == shipped && edge.target() == helper)
                .all(DependencyEdge::affects_verdict)
        );
        assert_eq!(
            report
                .dependency_edges()
                .iter()
                .filter(|edge| edge.source() == fixture && edge.target() == helper)
                .map(DependencyEdge::role)
                .collect::<Vec<_>>(),
            [SourceRole::Fixture]
        );
        assert_eq!(report.dependency_coverage().resolved_internal_uses(), 2);
        assert_eq!(report.dependency_coverage().context_relations(), 1);
    }

    fn file_id(report: &smackdebt_analysis::Report, path: &str) -> FileId {
        report
            .files()
            .iter()
            .find(|file| file.path() == path)
            .unwrap_or_else(|| panic!("missing {path}"))
            .id()
    }

    fn write_crate(root: &Path, name: &str, entry: &str) {
        fs::create_dir_all(root.join(format!("crates/{name}/src"))).unwrap();
        fs::write(
            root.join(format!("crates/{name}/Cargo.toml")),
            format!("[package]\nname='{name}'\nversion='0.1.0'\n"),
        )
        .unwrap();
        fs::write(root.join(format!("crates/{name}/src/lib.rs")), entry).unwrap();
    }

    #[test]
    fn a_test_role_relation_stays_evidence_and_never_closes_a_package_cycle() {
        let root = tempfile::tempdir().unwrap();
        write_crate(
            root.path(),
            "alpha",
            "use beta::helper;\npub fn run() -> u32 { helper() }\n",
        );
        write_crate(
            root.path(),
            "beta",
            "pub fn helper() -> u32 { 1 }\n#[cfg(test)]\nmod tests {\n    use alpha::run;\n    #[test]\n    fn covers() { assert_eq!(run(), 1); }\n}\n",
        );
        fs::create_dir_all(root.path().join("crates/beta/tests")).unwrap();
        fs::write(
            root.path().join("crates/beta/tests/it.rs"),
            "use alpha::run;\n#[test]\nfn integrates() { assert_eq!(run(), 1); }\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let alpha = file_id(report, "crates/alpha/src/lib.rs");
        let beta = file_id(report, "crates/beta/src/lib.rs");

        let mut back_edges: Vec<_> = report
            .dependency_edges()
            .iter()
            .filter(|edge| edge.target() == alpha)
            .map(|edge| (edge.source() == beta, edge.role()))
            .collect();
        back_edges.sort();
        assert_eq!(
            back_edges,
            [(false, SourceRole::Test), (true, SourceRole::Test)],
            "the test-role relations into alpha must stay in the machine report"
        );

        let package_edges: Vec<_> = report
            .package_edges()
            .iter()
            .map(|edge| (edge.source(), edge.target()))
            .collect();
        let alpha_package = report.files()[alpha.index()].package().unwrap();
        let beta_package = report.files()[beta.index()].package().unwrap();
        assert_eq!(package_edges, [(alpha_package, beta_package)]);
        assert!(
            report
                .architecture_findings()
                .iter()
                .all(|finding| finding.kind() != ArchitectureFindingKind::PackageCycle)
        );
        assert!(report.stable_dependency_findings().is_empty());
    }

    #[test]
    fn a_rust_module_file_owns_a_directory_named_after_it() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("src/outer")).unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname='modules'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(root.path().join("src/lib.rs"), "mod outer;\nmod sibling;\n").unwrap();
        fs::write(root.path().join("src/outer.rs"), "mod inner;\n").unwrap();
        fs::write(
            root.path().join("src/outer/inner.rs"),
            "use super::super::sibling::shared;\npub fn work() -> u32 { shared() }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("src/sibling.rs"),
            "pub fn shared() -> u32 { 1 }\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let mut edges: Vec<_> = report
            .dependency_edges()
            .iter()
            .map(|edge| {
                (
                    report.files()[edge.source().index()].path().to_owned(),
                    report.files()[edge.target().index()].path().to_owned(),
                    edge.relation(),
                )
            })
            .collect();
        edges.sort();
        let ownership = smackdebt_analysis::StaticRelationKind::ModuleOwnership;
        let uses = smackdebt_analysis::StaticRelationKind::Uses;
        assert_eq!(
            edges,
            [
                (
                    "src/lib.rs".to_owned(),
                    "src/outer.rs".to_owned(),
                    ownership
                ),
                (
                    "src/lib.rs".to_owned(),
                    "src/sibling.rs".to_owned(),
                    ownership
                ),
                // `mod inner;` in `outer.rs` names `outer/inner.rs`.
                (
                    "src/outer.rs".to_owned(),
                    "src/outer/inner.rs".to_owned(),
                    ownership
                ),
                // `super::super` from `outer/inner.rs` is the crate root's module.
                (
                    "src/outer/inner.rs".to_owned(),
                    "src/sibling.rs".to_owned(),
                    uses
                ),
            ]
        );
        assert_eq!(report.resolution_diagnostics().len(), 0);
    }

    #[test]
    fn a_module_declared_only_under_a_test_configuration_is_test_source() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("crates/main/src/worker")).unwrap();
        fs::write(
            root.path().join("crates/main/Cargo.toml"),
            "[package]\nname='main'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            root.path().join("crates/main/src/lib.rs"),
            "mod worker;\npub fn run() -> u32 { worker::work() }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("crates/main/src/worker.rs"),
            "#[cfg(test)]\nmod tests;\npub fn work() -> u32 { 1 }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("crates/main/src/worker/tests.rs"),
            "use support::probe;\n#[test]\nfn covers() { assert_eq!(probe(), 1); }\n",
        )
        .unwrap();
        write_crate(
            root.path(),
            "support",
            "use main::run;\npub fn probe() -> u32 { run() }\n",
        );

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let declared = file_id(report, "crates/main/src/worker/tests.rs");
        assert_eq!(
            report.files()[declared.index()].role(),
            SourceRole::Test,
            "rustc compiles a cfg(test) module file only under test"
        );
        assert!(
            report
                .dependency_edges()
                .iter()
                .filter(|edge| edge.source() == declared)
                .all(|edge| edge.role() == SourceRole::Test && !edge.enters_verdict_graph())
        );
        assert_eq!(
            package_pairs(report),
            [("crates/support".to_owned(), "crates/main".to_owned())]
        );
        assert!(
            report
                .architecture_findings()
                .iter()
                .all(|finding| finding.kind() != ArchitectureFindingKind::PackageCycle)
        );
    }

    #[test]
    fn a_test_declaration_demotes_only_a_file_no_other_rule_and_no_other_declaration_claims() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("src/harness")).unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname='declared'\nversion='0.1.0'\n",
        )
        .unwrap();
        // `shared` is declared twice, once outside the test scope.
        fs::write(
            root.path().join("src/lib.rs"),
            "#[cfg(test)]\nmod shared;\n#[cfg(test)]\nmod harness;\n#[cfg(test)]\nmod kept;\n",
        )
        .unwrap();
        fs::write(
            root.path().join("src/main.rs"),
            "mod shared;\nfn main() {}\n",
        )
        .unwrap();
        fs::write(root.path().join("src/shared.rs"), "pub fn shared() {}\n").unwrap();
        // `harness` inherits the test scope and passes it to its own module.
        fs::write(
            root.path().join("src/harness.rs"),
            "mod helpers;\npub fn harness() {}\n",
        )
        .unwrap();
        fs::write(
            root.path().join("src/harness/helpers.rs"),
            "pub fn helper() {}\n",
        )
        .unwrap();
        // `kept` is claimed by configuration, which keeps precedence.
        fs::write(root.path().join("src/kept.rs"), "pub fn kept() {}\n").unwrap();

        let result = analyze_codebase(
            &CodebaseRequest::new(root.path())
                .with_role_rules(vec![SourceRoleRule::primary("src/kept.rs")]),
        )
        .unwrap();
        let report = result.report();
        let role = |path: &str| report.files()[file_id(report, path).index()].role();
        assert_eq!(role("src/shared.rs"), SourceRole::Primary);
        assert_eq!(role("src/harness.rs"), SourceRole::Test);
        assert_eq!(role("src/harness/helpers.rs"), SourceRole::Test);
        assert_eq!(role("src/kept.rs"), SourceRole::Primary);
        assert_eq!(role("src/lib.rs"), SourceRole::Primary);
    }

    #[test]
    fn a_primary_file_imported_only_by_tests_is_not_an_orphan() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("src")).unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname='orphans'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            root.path().join("src/lib.rs"),
            "#[cfg(test)]\nmod tests {\n    use crate::only_tests::sample;\n    #[test]\n    fn runs() { assert!(sample()); }\n}\n",
        )
        .unwrap();
        fs::write(
            root.path().join("src/only_tests.rs"),
            "pub fn sample() -> bool { true }\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let only_tests = file_id(report, "src/only_tests.rs");
        assert!(
            report
                .dependency_edges()
                .iter()
                .any(|edge| edge.target() == only_tests && edge.role() == SourceRole::Test)
        );
        assert!(
            report
                .orphan_files()
                .iter()
                .all(|orphan| orphan.file() != only_tests),
            "a file its own tests import is used"
        );
    }

    #[test]
    fn a_primary_edge_reports_a_stable_dependency_violation_that_test_edges_do_not() {
        let violating = tempfile::tempdir().unwrap();
        write_crate(
            violating.path(),
            "x",
            "use a::run;\npub fn x() -> u32 { run() }\n",
        );
        write_crate(
            violating.path(),
            "a",
            "use b::one;\nuse b::two;\npub fn run() -> u32 { one() + two() }\n",
        );
        write_crate(
            violating.path(),
            "b",
            "use c::cee;\nuse d::dee;\npub fn one() -> u32 { cee() }\npub fn two() -> u32 { dee() }\n",
        );
        write_crate(violating.path(), "c", "pub fn cee() -> u32 { 1 }\n");
        write_crate(violating.path(), "d", "pub fn dee() -> u32 { 2 }\n");

        let result = analyze_codebase(&CodebaseRequest::new(violating.path())).unwrap();
        let report = result.report();
        let package_name = |package: smackdebt_analysis::PackageId| {
            report.packages()[package.index()].path().to_owned()
        };
        let findings: Vec<_> = report
            .stable_dependency_findings()
            .iter()
            .map(|finding| {
                (
                    package_name(finding.source()),
                    package_name(finding.target()),
                    finding.evidence().references(),
                )
            })
            .collect();
        assert_eq!(
            findings,
            [("crates/a".to_owned(), "crates/b".to_owned(), 2)]
        );

        let scoped = tempfile::tempdir().unwrap();
        write_crate(
            scoped.path(),
            "x",
            "use a::run;\npub fn x() -> u32 { run() }\n",
        );
        write_crate(
            scoped.path(),
            "a",
            "pub fn run() -> u32 { 3 }\n#[cfg(test)]\nmod tests {\n    use b::one;\n    use b::two;\n    #[test]\n    fn covers() { assert_eq!(one() + two(), 3); }\n}\n",
        );
        write_crate(
            scoped.path(),
            "b",
            "use c::cee;\nuse d::dee;\npub fn one() -> u32 { cee() }\npub fn two() -> u32 { dee() }\n",
        );
        write_crate(scoped.path(), "c", "pub fn cee() -> u32 { 1 }\n");
        write_crate(scoped.path(), "d", "pub fn dee() -> u32 { 2 }\n");

        let result = analyze_codebase(&CodebaseRequest::new(scoped.path())).unwrap();
        assert!(
            result.report().stable_dependency_findings().is_empty(),
            "a test-scoped import is not a production dependency direction"
        );
    }

    #[test]
    fn rust_module_ownership_cycle_is_context_while_mutual_uses_are_a_verdict() {
        let ownership = tempfile::tempdir().unwrap();
        fs::write(
            ownership.path().join("Cargo.toml"),
            "[package]\nname='ownership'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(ownership.path().join("a.rs"), "mod b;\npub fn a() {}\n").unwrap();
        fs::write(ownership.path().join("b.rs"), "mod a;\npub fn b() {}\n").unwrap();
        let result = analyze_codebase(&CodebaseRequest::new(ownership.path())).unwrap();
        let report = result.report();
        assert_eq!(report.dependency_edges().len(), 2);
        assert!(report.dependency_edges().iter().all(|edge| {
            edge.relation() == smackdebt_analysis::StaticRelationKind::ModuleOwnership
                && !edge.affects_verdict()
        }));
        assert!(report.package_edges().is_empty());
        assert!(report.architecture_findings().is_empty());
        assert_eq!(report.dependency_coverage().module_ownership_relations(), 2);
        assert_eq!(report.dependency_coverage().total(), 2);

        let uses = tempfile::tempdir().unwrap();
        fs::write(
            uses.path().join("Cargo.toml"),
            "[package]\nname='uses'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            uses.path().join("a.rs"),
            "use crate::b::b;\npub fn a() { b(); }\n",
        )
        .unwrap();
        fs::write(
            uses.path().join("b.rs"),
            "use crate::a::a;\npub fn b() { a(); }\n",
        )
        .unwrap();
        let result = analyze_codebase(&CodebaseRequest::new(uses.path())).unwrap();
        let report = result.report();
        assert_eq!(report.dependency_edges().len(), 2);
        assert!(report.dependency_edges().iter().all(|edge| {
            edge.relation() == smackdebt_analysis::StaticRelationKind::Uses
                && edge.affects_verdict()
        }));
        assert!(
            report
                .architecture_findings()
                .iter()
                .any(|finding| { finding.kind() == ArchitectureFindingKind::FileCycle })
        );
        assert_eq!(report.dependency_coverage().resolved_internal_uses(), 2);
        assert_eq!(report.dependency_coverage().total(), 2);

        let wiring = tempfile::tempdir().unwrap();
        fs::write(
            wiring.path().join("Cargo.toml"),
            "[package]\nname='wiring'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::create_dir_all(wiring.path().join("thing")).unwrap();
        fs::write(
            wiring.path().join("thing/mod.rs"),
            "mod child;\npub use self::child::x;\npub fn y() {}\n",
        )
        .unwrap();
        fs::write(
            wiring.path().join("thing/child.rs"),
            "use super::*;\npub fn x() { y(); }\n",
        )
        .unwrap();
        let result = analyze_codebase(&CodebaseRequest::new(wiring.path())).unwrap();
        let report = result.report();
        let relations: Vec<_> = report
            .dependency_edges()
            .iter()
            .map(|edge| {
                (
                    report.files()[edge.source().index()].path().to_owned(),
                    report.files()[edge.target().index()].path().to_owned(),
                    edge.relation(),
                )
            })
            .collect();
        let uses = smackdebt_analysis::StaticRelationKind::Uses;
        let owns = smackdebt_analysis::StaticRelationKind::ModuleOwnership;
        assert_eq!(
            relations,
            [
                ("thing/child.rs".to_owned(), "thing/mod.rs".to_owned(), uses),
                ("thing/mod.rs".to_owned(), "thing/child.rs".to_owned(), uses),
                ("thing/mod.rs".to_owned(), "thing/child.rs".to_owned(), owns),
            ],
            "the wiring relations stay complete in the machine report"
        );
        assert!(
            report.architecture_findings().is_empty(),
            "module wiring between an owning pair is not a file cycle"
        );
        assert!(
            report.orphan_files().is_empty(),
            "the exclusion is scoped to the cycle graph, so orphan facts are unchanged"
        );

        let siblings = tempfile::tempdir().unwrap();
        fs::write(
            siblings.path().join("Cargo.toml"),
            "[package]\nname='siblings'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::create_dir_all(siblings.path().join("thing")).unwrap();
        fs::write(
            siblings.path().join("thing/mod.rs"),
            "mod one;\nmod two;\nmod three;\n",
        )
        .unwrap();
        fs::write(
            siblings.path().join("thing/one.rs"),
            "use super::two::two;\npub fn one() { two() }\n",
        )
        .unwrap();
        fs::write(
            siblings.path().join("thing/two.rs"),
            "use super::three::three;\npub fn two() { three() }\n",
        )
        .unwrap();
        fs::write(
            siblings.path().join("thing/three.rs"),
            "use super::one::one;\npub fn three() { one() }\n",
        )
        .unwrap();
        let result = analyze_codebase(&CodebaseRequest::new(siblings.path())).unwrap();
        let report = result.report();
        let cycles: Vec<_> = report
            .architecture_findings()
            .iter()
            .filter(|finding| finding.kind() == ArchitectureFindingKind::FileCycle)
            .map(|finding| {
                let mut files: Vec<_> = finding
                    .files()
                    .iter()
                    .map(|file| report.files()[file.index()].path().to_owned())
                    .collect();
                files.sort();
                files
            })
            .collect();
        assert_eq!(
            cycles,
            [vec![
                "thing/one.rs".to_owned(),
                "thing/three.rs".to_owned(),
                "thing/two.rs".to_owned(),
            ]],
            "a cycle between owned siblings is not wiring and survives"
        );
        assert!(
            report
                .architecture_findings()
                .iter()
                .find(|finding| finding.kind() == ArchitectureFindingKind::FileCycle)
                .is_some_and(|finding| !finding.witness_edges().is_empty()),
            "a surviving cycle still names the relations that remain in the graph"
        );
    }

    #[test]
    fn a_module_component_collapses_while_a_cycle_between_its_children_survives() {
        let collapsing = tempfile::tempdir().unwrap();
        write_module_component(collapsing.path(), "");
        let result = analyze_codebase(&CodebaseRequest::new(collapsing.path())).unwrap();
        let report = result.report();
        assert_eq!(
            report
                .dependency_edges()
                .iter()
                .filter(|edge| edge.enters_verdict_graph())
                .count(),
            4,
            "every wiring relation stays eligible evidence"
        );
        assert!(
            report.architecture_findings().is_empty(),
            "a parent and its children are one wiring relationship, not a cycle"
        );

        let surviving = tempfile::tempdir().unwrap();
        write_module_component(surviving.path(), "use super::second::second;\n");
        fs::write(
            surviving.path().join("thing/second.rs"),
            "use super::*;\nuse super::first::first;\npub fn second() { y(); first() }\n",
        )
        .unwrap();
        let result = analyze_codebase(&CodebaseRequest::new(surviving.path())).unwrap();
        let report = result.report();
        let cycles: Vec<_> = report
            .architecture_findings()
            .iter()
            .filter(|finding| finding.kind() == ArchitectureFindingKind::FileCycle)
            .map(|finding| {
                let mut files: Vec<_> = finding
                    .files()
                    .iter()
                    .map(|file| report.files()[file.index()].path().to_owned())
                    .collect();
                files.sort();
                files
            })
            .collect();
        assert_eq!(
            cycles,
            [vec![
                "thing/first.rs".to_owned(),
                "thing/second.rs".to_owned()
            ]],
            "the sibling cycle survives while the parent's wiring is excluded"
        );
    }

    #[test]
    fn a_cycle_that_passes_through_an_owning_pair_by_other_files_survives() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname='through'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::create_dir_all(root.path().join("a")).unwrap();
        fs::write(
            root.path().join("a.rs"),
            "mod child;\nuse self::child::step;\nuse self::other::other;\npub fn a() -> u32 { other() + step() }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("a/child.rs"),
            "use super::back::back;\npub fn step() -> u32 { back() }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("a/other.rs"),
            "use super::child::step;\npub fn other() -> u32 { step() }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("a/back.rs"),
            "use super::a;\npub fn back() -> u32 { a() }\n",
        )
        .unwrap();
        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let cycles: Vec<_> = report
            .architecture_findings()
            .iter()
            .filter(|finding| finding.kind() == ArchitectureFindingKind::FileCycle)
            .map(|finding| {
                let mut files: Vec<_> = finding
                    .files()
                    .iter()
                    .map(|file| report.files()[file.index()].path().to_owned())
                    .collect();
                files.sort();
                files
            })
            .collect();
        assert_eq!(
            cycles,
            [vec![
                "a.rs".to_owned(),
                "a/back.rs".to_owned(),
                "a/child.rs".to_owned(),
                "a/other.rs".to_owned(),
            ]],
            "only the owning pair's own relations leave the graph"
        );
        let witnessed: Vec<_> = report.architecture_findings()[0]
            .witness_edges()
            .iter()
            .map(|edge| {
                let edge = &report.dependency_edges()[edge.index()];
                (
                    report.files()[edge.source().index()].path().to_owned(),
                    report.files()[edge.target().index()].path().to_owned(),
                )
            })
            .collect();
        assert!(
            !witnessed.contains(&("a.rs".to_owned(), "a/child.rs".to_owned())),
            "a witness can only name a relation the cycle graph kept: {witnessed:?}"
        );
    }

    /// A `mod.rs` that re-exports two children which import it back.
    fn write_module_component(root: &Path, first_extra: &str) {
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='component'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("thing")).unwrap();
        fs::write(
            root.join("thing/mod.rs"),
            "mod first;\nmod second;\npub use self::first::first;\npub use self::second::second;\npub fn y() {}\n",
        )
        .unwrap();
        fs::write(
            root.join("thing/first.rs"),
            format!("use super::*;\n{first_extra}pub fn first() {{ y() }}\n"),
        )
        .unwrap();
        fs::write(
            root.join("thing/second.rs"),
            "use super::*;\npub fn second() { y() }\n",
        )
        .unwrap();
    }

    #[test]
    fn a_module_declaration_reads_the_module_directory_before_a_sibling_file() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname='precedence'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::create_dir_all(root.path().join("a")).unwrap();
        fs::write(root.path().join("a.rs"), "mod child;\npub fn a() {}\n").unwrap();
        fs::write(root.path().join("a/child.rs"), "pub fn owned() {}\n").unwrap();
        fs::write(root.path().join("child.rs"), "pub fn sibling() {}\n").unwrap();
        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let declarations: Vec<_> = report
            .dependency_edges()
            .iter()
            .filter(|edge| {
                edge.relation() == smackdebt_analysis::StaticRelationKind::ModuleOwnership
            })
            .map(|edge| {
                (
                    report.files()[edge.source().index()].path().to_owned(),
                    report.files()[edge.target().index()].path().to_owned(),
                )
            })
            .collect();
        assert_eq!(
            declarations,
            [("a.rs".to_owned(), "a/child.rs".to_owned())],
            "the module directory a file owns wins over a sibling of the same name"
        );
    }

    #[test]
    fn a_real_diff_answers_from_moved_debt_and_never_from_healthy_additions() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(
            repository_path.join("Cargo.toml"),
            "[package]\nname='verdicts'\nversion='0.1.0'\n",
        )
        .unwrap();
        let complex = "pub fn work(a: i32) -> i32 { if a > 0 { if a > 1 { return 1; } } 0 }\n";
        fs::write(repository_path.join("work.rs"), complex).unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "complex base"]);
        let request = DiffRequest::new(repository_path)
            .with_reference("HEAD")
            .with_thresholds((1, 2), (5, 10), (50, 100), (4, 7), (6, 9));

        fs::write(
            repository_path.join("work.rs"),
            format!("{complex}pub fn helper() -> i32 {{ 1 }}\n"),
        )
        .unwrap();
        let added = analyze_diff(&request).unwrap();
        let verdict = added.report().verdict().unwrap();
        assert_eq!(verdict.diff_tier(), Some(DiffTier::NoDebtChange));
        assert_eq!(verdict.sentence(), "No debt changed.");
        assert!(verdict.selection().is_empty());
        // The healthy addition stays in the machine report while it moves
        // nothing.
        assert!(
            added
                .report()
                .comparisons()
                .iter()
                .any(|comparison| comparison.kind() == smackdebt_analysis::ComparisonKind::Added)
        );

        fs::write(
            repository_path.join("work.rs"),
            "pub fn work(a: i32) -> i32 { a }\n",
        )
        .unwrap();
        let improved = analyze_diff(&request).unwrap();
        assert_eq!(
            improved.report().verdict().unwrap().diff_tier(),
            Some(DiffTier::Better)
        );

        fs::write(
            repository_path.join("work.rs"),
            format!(
                "pub fn work(a: i32) -> i32 {{ a }}\n{}",
                complex.replace("work", "later")
            ),
        )
        .unwrap();
        let mixed = analyze_diff(&request).unwrap();
        let verdict = mixed.report().verdict().unwrap();
        assert_eq!(verdict.diff_tier(), Some(DiffTier::Mixed));
        assert_eq!(
            verdict.sentence(),
            "Debt increased in some places and decreased in others."
        );
        assert_eq!(verdict.facts().source().worse(), 1);
        assert_eq!(verdict.facts().source().better(), 1);
        assert!(
            verdict
                .facts()
                .moved(smackdebt_analysis::DebtFamily::Source)
        );
        assert!(
            !verdict
                .facts()
                .moved(smackdebt_analysis::DebtFamily::Architecture)
        );
        assert!(!verdict.selection().has_duplicate_identity());
    }

    #[test]
    fn a_real_diff_pairs_only_safe_anonymous_units() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(
            repository_path.join("callbacks.js"),
            "watch('ready', () => work());\nrepeat(() => same());\ngone(() => old());\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "callback base"]);
        fs::write(
            repository_path.join("callbacks.js"),
            "\nwatch('ready', () => { if (ready) work(); });\nrepeat(() => same());\nrepeat(() => same());\nonly(() => new_one());\nonly(() => new_one());\n",
        )
        .unwrap();

        let report =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let report = report.report();
        let kinds: Vec<_> = report
            .comparisons()
            .iter()
            .map(smackdebt_analysis::Comparison::kind)
            .collect();
        assert_eq!(
            kinds
                .iter()
                .filter(|kind| **kind == smackdebt_analysis::ComparisonKind::Ambiguous)
                .count(),
            1
        );
        assert_eq!(
            kinds
                .iter()
                .filter(|kind| **kind == smackdebt_analysis::ComparisonKind::Added)
                .count(),
            2
        );
        assert_eq!(
            kinds
                .iter()
                .filter(|kind| **kind == smackdebt_analysis::ComparisonKind::Removed)
                .count(),
            1
        );
        assert_eq!(
            kinds
                .iter()
                .filter(|kind| **kind == smackdebt_analysis::ComparisonKind::MetricChanged)
                .count(),
            1
        );
        let diagnostics: Vec<_> = report
            .diagnostics()
            .iter()
            .filter(|value| value.kind() == DiagnosticKind::AmbiguousIdentity)
            .collect();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].file(), Some(FileId::from_index(0)));
        assert!(
            report
                .comparisons()
                .iter()
                .any(Comparison::is_anonymous_ambiguity)
        );
        assert_eq!(
            report.verdict().unwrap().diff_tier(),
            Some(DiffTier::NoDebtChange)
        );
        assert!(
            !report
                .verdict()
                .unwrap()
                .selection()
                .has_duplicate_identity()
        );
    }

    #[test]
    fn repeated_declared_names_do_not_create_an_anonymous_warning() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(
            repository_path.join("duplicate.js"),
            "function same() { return 1; }\nfunction same() { return 2; }\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "duplicate base"]);
        fs::write(
            repository_path.join("duplicate.js"),
            "function same() { return 1; }\nfunction same() { if (ready) return 2; }\n",
        )
        .unwrap();

        let report =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let report = report.report();
        assert_eq!(report.comparisons().len(), 1);
        assert_eq!(
            report.comparisons()[0].kind(),
            smackdebt_analysis::ComparisonKind::Ambiguous
        );
        assert!(!report.comparisons()[0].is_anonymous_ambiguity());
        assert!(
            report
                .diagnostics()
                .iter()
                .all(|value| value.kind() != DiagnosticKind::AmbiguousIdentity)
        );
    }

    #[test]
    fn a_codebase_verdict_states_the_tier_and_names_the_worst_offender() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname='verdicts'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            root.path().join("work.rs"),
            "pub fn work(a: i32) -> i32 { if a > 0 { if a > 1 { return 1; } } 0 }\n",
        )
        .unwrap();
        let report = CodebaseRequest::new(root.path())
            .with_thresholds((1, 2), (5, 10), (50, 100), (4, 7), (6, 9))
            .analyze()
            .unwrap();
        let verdict = report.report().verdict().unwrap();
        assert_eq!(verdict.counts().checked(), 1);
        assert_eq!(verdict.counts().high(), 1);
        assert_eq!(verdict.counts().high_permille(), 1000);
        // One checked unit is far below the density evidence threshold, so
        // the saturated permille is capped at worn.
        assert_eq!(verdict.tier(), CodebaseTier::Worn);
        assert_eq!(verdict.sentence(), "Worn in the usual places.");
        assert_eq!(verdict.diff_tier(), None);
        let offender = verdict.worst_offender().unwrap();
        assert_eq!(offender.path(), "work.rs");
        assert_eq!(offender.reason(), WorstOffenderReason::MostComplex);
    }

    #[test]
    fn rust_relation_diffs_keep_kind_role_and_trust_as_independent_evidence() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(
            repository_path.join("Cargo.toml"),
            "[package]\nname='relations'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(repository_path.join("child.rs"), "pub fn work() {}\n").unwrap();
        fs::write(
            repository_path.join("main.rs"),
            "use crate::child::work;\nfn main() { work(); }\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "base use"]);
        fs::write(
            repository_path.join("main.rs"),
            "mod child;\nuse crate::child::work;\nfn main() { work(); }\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "add ownership"]);

        let clean =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD~1")).unwrap();
        let ownership = clean
            .report()
            .architecture_comparisons()
            .iter()
            .find(|comparison| {
                comparison.kind() == smackdebt_analysis::ArchitectureComparisonKind::EdgeAdded
                    && comparison.relation()
                        == Some(smackdebt_analysis::StaticRelationKind::ModuleOwnership)
            })
            .expect("clean ref diff retains ownership evidence");
        assert_eq!(ownership.role(), Some(SourceRole::Primary));
        assert_eq!(ownership.trust(), Some(SourceTrust::Trusted));
        assert_eq!(
            ownership.direction(),
            smackdebt_analysis::ComparisonDirection::Changed
        );
        assert!(
            clean
                .report()
                .architecture_comparisons()
                .iter()
                .all(|comparison| {
                    comparison.direction() != smackdebt_analysis::ComparisonDirection::Worse
                })
        );

        fs::write(
            repository_path.join("main.rs"),
            "mod child;\nuse crate::child::work;\nfn main() { work(); broken( }\n",
        )
        .unwrap();
        let worktree =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        assert!(
            worktree
                .report()
                .architecture_comparisons()
                .iter()
                .any(|comparison| {
                    comparison.relation() == Some(smackdebt_analysis::StaticRelationKind::Uses)
                        && comparison.kind()
                            == smackdebt_analysis::ArchitectureComparisonKind::EdgeAdded
                        && comparison.role() == Some(SourceRole::Primary)
                        && comparison.trust() == Some(SourceTrust::Advisory)
                })
        );
        assert!(
            worktree
                .report()
                .architecture_comparisons()
                .iter()
                .any(|comparison| {
                    comparison.relation() == Some(smackdebt_analysis::StaticRelationKind::Uses)
                        && comparison.kind()
                            == smackdebt_analysis::ArchitectureComparisonKind::EdgeRemoved
                        && comparison.role() == Some(SourceRole::Primary)
                        && comparison.trust() == Some(SourceTrust::Trusted)
                })
        );

        fs::create_dir_all(repository_path.join("tests")).unwrap();
        fs::write(
            repository_path.join("tests/main.rs"),
            "use crate::child::work;\nfn main() { work(); }\n",
        )
        .unwrap();
        fs::remove_file(repository_path.join("main.rs")).unwrap();
        let role_change =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        assert!(
            role_change
                .report()
                .architecture_comparisons()
                .iter()
                .any(|comparison| {
                    comparison.relation() == Some(smackdebt_analysis::StaticRelationKind::Uses)
                        && comparison.kind()
                            == smackdebt_analysis::ArchitectureComparisonKind::EdgeAdded
                        && comparison.role() == Some(SourceRole::Test)
                        && comparison.trust() == Some(SourceTrust::Trusted)
                })
        );
        assert!(
            role_change
                .report()
                .architecture_comparisons()
                .iter()
                .any(|comparison| {
                    comparison.relation() == Some(smackdebt_analysis::StaticRelationKind::Uses)
                        && comparison.kind()
                            == smackdebt_analysis::ArchitectureComparisonKind::EdgeRemoved
                        && comparison.role() == Some(SourceRole::Primary)
                        && comparison.trust() == Some(SourceTrust::Trusted)
                })
        );
    }

    #[test]
    fn relation_diff_retains_reference_count_changes_for_the_same_file_pair() {
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
            "import value from './value';\nfunction main() { return value(); }\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("value.js"),
            "export default function value() {}\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "one reference"]);
        fs::write(
            repository_path.join("main.js"),
            "import value from './value';\nimport second from './value';\nfunction main() { value(); second(); }\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let comparison = result
            .report()
            .architecture_comparisons()
            .iter()
            .find(|comparison| {
                comparison.relation() == Some(smackdebt_analysis::StaticRelationKind::Uses)
                    && comparison.before_references() == Some(1)
                    && comparison.after_references() == Some(2)
            })
            .expect("reference count change is retained");
        assert_eq!(
            comparison.direction(),
            smackdebt_analysis::ComparisonDirection::Changed
        );
        assert_eq!(result.report().dependency_edges()[0].references(), 2);
    }

    /// A package cannot say who imports it from its own files, so selecting
    /// one reads the repository that answers the question and shows the
    /// package.
    #[test]
    fn package_selection_reads_the_sibling_source_that_imports_it() {
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
        assert_eq!(result.stats().inventory_walks, 1);
        assert_eq!(result.stats().source_reads, 2);
        let selected = result.selected_scope().unwrap();
        assert_eq!(result.report().scopes()[selected.index()].name(), "core");
        assert_eq!(result.report().scopes()[0].name(), ".");
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

    /// A file selection answers one file of the repository report, so the
    /// walk is the repository and the answered scope is that file.
    #[test]
    fn explicit_file_selection_answers_one_scope_of_the_repository() {
        let root = repository();
        let repository_path = root.path().join("repo");
        fs::write(
            repository_path.join("outside.rs"),
            "fn outside() { if true {} }\n",
        )
        .unwrap();

        let result =
            analyze_codebase(&CodebaseRequest::new(repository_path.join("sample.rs"))).unwrap();

        assert_eq!(result.report().files().len(), 2);
        assert_eq!(result.stats().inventory_walks, 1);
        assert_eq!(result.stats().source_reads, 2);
        let selected = result.selected_scope().unwrap();
        assert_eq!(
            result.report().scopes()[selected.index()].name(),
            "sample.rs"
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
    fn diff_reports_a_named_file_when_branch_reach_grows() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}").unwrap();
        for index in 0..20 {
            fs::write(
                repository_path.join(format!("f{index}.ts")),
                format!("export const f{index} = {index};\n"),
            )
            .unwrap();
        }
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "initial"]);
        fs::write(
            repository_path.join("f1.ts"),
            "import { f0 } from './f0.js';\nexport const f1 = f0 + 1;\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let comparison = result
            .report()
            .propagation_comparisons()
            .iter()
            .find(|comparison| {
                matches!(
                    comparison.subject(),
                    smackdebt_analysis::PropagationSubject::File { .. }
                )
            })
            .expect("file reach movement is retained");
        assert_eq!(
            comparison.direction(),
            smackdebt_analysis::ComparisonDirection::Worse
        );
        assert_eq!(comparison.before(), (1, 20));
        assert_eq!(comparison.after(), (2, 20));
        assert_eq!(
            result
                .report()
                .scope_verdict(result.report().root().unwrap())
                .selection()
                .facts()
                .architecture()
                .worse(),
            1
        );
    }

    #[test]
    fn diff_withholds_reach_movement_when_only_current_graph_evidence_is_incomplete() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}").unwrap();
        for index in 0..20 {
            fs::write(
                repository_path.join(format!("f{index}.ts")),
                format!("export const f{index} = {index};\n"),
            )
            .unwrap();
        }
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "initial"]);
        fs::write(
            repository_path.join("f1.ts"),
            "import { f0 } from './f0.js';\nimport missing from './missing.js';\nexport const f1 = f0 + missing;\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let evidence = result.report().diff_graph_evidence().unwrap();
        assert!(!evidence.current().is_complete());
        assert!(evidence.base().is_complete());
        assert_eq!(evidence.propagation().total(), 1);
        assert_eq!(evidence.propagation().current(), 1);
        assert_eq!(evidence.propagation().base(), 0);
        assert!(result.report().propagation_comparisons().is_empty());
        assert_eq!(
            result
                .report()
                .scope_verdict(result.report().root().unwrap())
                .selection()
                .facts()
                .architecture()
                .total(),
            0
        );
    }

    #[test]
    fn diff_withholds_reach_movement_when_only_base_graph_evidence_is_incomplete() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}").unwrap();
        for index in 0..20 {
            fs::write(
                repository_path.join(format!("f{index}.ts")),
                format!("export const f{index} = {index};\n"),
            )
            .unwrap();
        }
        fs::write(
            repository_path.join("f1.ts"),
            "import { f0 } from './f0.js';\nimport missing from './missing.js';\nexport const f1 = f0 + missing;\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "initial"]);
        fs::write(repository_path.join("f1.ts"), "export const f1 = 1;\n").unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let evidence = result.report().diff_graph_evidence().unwrap();
        assert!(evidence.current().is_complete());
        assert!(!evidence.base().is_complete());
        assert_eq!(evidence.propagation().total(), 1);
        assert_eq!(evidence.propagation().current(), 0);
        assert_eq!(evidence.propagation().base(), 1);
        assert!(result.report().propagation_comparisons().is_empty());
    }

    fn astro_diff_result(change: &str) -> ProjectReport {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}\n").unwrap();
        if change != "added" {
            fs::write(repository_path.join("page.astro"), "<h1>Before</h1>\n").unwrap();
        }
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "base"]);
        match change {
            "added" => fs::write(repository_path.join("page.astro"), "<h1>Added</h1>\n").unwrap(),
            "modified" => {
                fs::write(repository_path.join("page.astro"), "<h1>After</h1>\n").unwrap()
            }
            "deleted" => fs::remove_file(repository_path.join("page.astro")).unwrap(),
            _ => unreachable!("test chooses an Astro change"),
        }
        analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap()
    }

    fn assert_astro_diff_is_retained(result: &ProjectReport) {
        let report = result.report();
        let root = report.root().unwrap();
        let coverage = report.scopes()[root.index()].coverage();
        assert_eq!(coverage.selected_files(), 1);
        assert_eq!(coverage.unsupported_files(), 1);
        assert_eq!(report.files().len(), 1);
        assert_eq!(report.files()[0].language(), Some(Language::Astro));
        assert_eq!(report.files()[0].trust(), SourceTrust::Failed);
        assert!(report.findings().is_empty());
        assert!(report.dependency_edges().is_empty());
    }

    #[test]
    fn added_astro_makes_only_current_diff_graph_evidence_incomplete() {
        let result = astro_diff_result("added");
        assert_astro_diff_is_retained(&result);
        let evidence = result.report().diff_graph_evidence().unwrap();
        assert!(!evidence.current().is_complete());
        assert!(evidence.base().is_complete());
    }

    #[test]
    fn modified_astro_makes_both_diff_graph_evidence_sides_incomplete() {
        let result = astro_diff_result("modified");
        assert_astro_diff_is_retained(&result);
        let evidence = result.report().diff_graph_evidence().unwrap();
        assert!(!evidence.current().is_complete());
        assert!(!evidence.base().is_complete());
    }

    #[test]
    fn deleted_astro_makes_only_base_diff_graph_evidence_incomplete() {
        let result = astro_diff_result("deleted");
        assert_astro_diff_is_retained(&result);
        let evidence = result.report().diff_graph_evidence().unwrap();
        assert!(evidence.current().is_complete());
        assert!(!evidence.base().is_complete());
    }

    #[test]
    fn diff_assigns_base_graph_failures_to_the_base_package_owner() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::create_dir_all(repository_path.join("sub")).unwrap();
        fs::write(repository_path.join("package.json"), "{}").unwrap();
        fs::write(
            repository_path.join("sub/a.ts"),
            "import missing from './missing.js';\nexport default missing;\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "root package"]);
        fs::write(repository_path.join("sub/package.json"), "{}").unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let evidence = result.report().diff_graph_evidence().unwrap();
        assert_eq!(
            evidence.base().incomplete_packages(),
            &[PackageId::from_index(0)]
        );
        assert_eq!(
            evidence.current().incomplete_packages(),
            &[PackageId::from_index(1)]
        );
    }

    #[test]
    fn unchanged_rust_module_uses_each_graph_sides_test_declaration_role() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::create_dir_all(repository_path.join("src")).unwrap();
        fs::write(
            repository_path.join("Cargo.toml"),
            "[package]\nname='roles'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("src/lib.rs"),
            "#[cfg(test)]\nmod helper;\npub fn run() {}\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("src/helper.rs"),
            "mod missing;\npub fn help() {}\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "test-only helper"]);
        fs::write(
            repository_path.join("src/lib.rs"),
            "mod helper;\npub fn run() { helper::help(); }\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let evidence = result.report().diff_graph_evidence().unwrap();
        assert!(evidence.base().is_complete());
        assert!(!evidence.current().is_complete());
        let helper = file_id(result.report(), "src/helper.rs");
        assert_eq!(
            result.report().files()[helper.index()].role(),
            SourceRole::Primary
        );
    }

    #[test]
    fn diff_resolves_manifest_names_from_each_graph_side() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        for package in ["a", "b"] {
            fs::create_dir_all(repository_path.join(package)).unwrap();
        }
        fs::write(
            repository_path.join("a/package.json"),
            "{\"name\":\"old-name\",\"main\":\"main.js\"}\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("b/package.json"),
            "{\"name\":\"b\",\"main\":\"main.js\"}\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("a/main.js"),
            "import value from 'b';\nexport default value;\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("b/main.js"),
            "import value from 'old-name';\nexport default value;\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "old package name"]);
        fs::write(
            repository_path.join("a/package.json"),
            "{\"name\":\"new-name\",\"main\":\"main.js\"}\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("b/main.js"),
            "import value from 'new-name';\nexport default value;\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        assert_eq!(result.report().architecture_findings().len(), 1);
        assert!(result.report().architecture_comparisons().is_empty());
        assert!(
            result
                .report()
                .resolution_diagnostics()
                .iter()
                .all(|diagnostic| diagnostic.kind() != ResolutionIssueKind::Unresolved)
        );
    }

    #[test]
    fn unchanged_gemspec_names_resolve_on_both_sides_of_an_unrelated_diff() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        for package in ["a", "b"] {
            fs::create_dir_all(repository_path.join(package)).unwrap();
        }
        fs::write(
            repository_path.join("a/a.gemspec"),
            "Gem::Specification.new do |spec|\n  spec.name = 'a-gem'\nend\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("b/b.gemspec"),
            "Gem::Specification.new do |spec|\n  spec.name = 'b-gem'\nend\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("a/main.js"),
            "import value from 'b-gem';\nexport default value;\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("b/main.js"),
            "import value from 'a-gem';\nexport default value;\n",
        )
        .unwrap();
        fs::write(repository_path.join("note.md"), "before\n").unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "gemspec packages"]);
        fs::write(repository_path.join("note.md"), "after\n").unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        assert_eq!(result.report().architecture_findings().len(), 1);
        assert!(result.report().architecture_comparisons().is_empty());
    }

    #[test]
    fn changed_ignore_rules_select_unchanged_tracked_sources_per_side() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}\n").unwrap();
        fs::write(repository_path.join(".gitignore"), "").unwrap();
        fs::write(
            repository_path.join("hidden.ts"),
            "import missing from './missing.js';\nexport default missing;\n",
        )
        .unwrap();
        fs::write(repository_path.join("main.ts"), "export default 1;\n").unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "visible source"]);
        fs::write(repository_path.join(".gitignore"), "hidden.ts\n").unwrap();
        fs::write(
            repository_path.join("hidden.ts"),
            "import changed from './missing.js';\nexport default changed;\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let evidence = result.report().diff_graph_evidence().unwrap();
        assert!(!evidence.base().is_complete());
        assert!(evidence.current().is_complete());
        assert!(
            result
                .report()
                .files()
                .iter()
                .any(|file| file.path() == "hidden.ts")
        );
    }

    #[test]
    fn a_modified_source_ignored_on_both_sides_never_enters_diff_analysis() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}\n").unwrap();
        fs::write(repository_path.join(".gitignore"), "hidden.ts\n").unwrap();
        fs::write(
            repository_path.join("hidden.ts"),
            "import missing from './missing.js';\nexport default missing;\n",
        )
        .unwrap();
        fs::write(repository_path.join("main.ts"), "export default 1;\n").unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["add", "-f", "hidden.ts"]);
        git(repository_path, ["commit", "-qm", "ignored tracked source"]);
        fs::write(
            repository_path.join("hidden.ts"),
            "import changed from './still-missing.js';\nexport default changed;\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let evidence = result.report().diff_graph_evidence().unwrap();
        assert!(evidence.base().is_complete());
        assert!(evidence.current().is_complete());
        assert!(
            result
                .report()
                .files()
                .iter()
                .all(|file| file.path() != "hidden.ts")
        );
    }

    #[test]
    fn diff_fails_when_a_reachable_base_manifest_object_is_missing() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(
            repository_path.join("package.json"),
            "{\"name\":\"example\"}\n",
        )
        .unwrap();
        fs::write(repository_path.join("main.ts"), "export default 1;\n").unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "base"]);
        fs::write(repository_path.join("main.ts"), "export default 2;\n").unwrap();

        let output = Command::new("git")
            .args(["rev-parse", "HEAD:package.json"])
            .current_dir(repository_path)
            .output()
            .unwrap();
        assert!(output.status.success());
        let object = String::from_utf8(output.stdout).unwrap();
        let object = object.trim();
        fs::remove_file(
            repository_path
                .join(".git/objects")
                .join(&object[..2])
                .join(&object[2..]),
        )
        .unwrap();

        let error =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap_err();
        assert!(matches!(error, ProjectError::Inspect { .. }));
    }

    #[test]
    fn propagation_compares_each_named_package_instead_of_unrelated_maxima() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        for package in ["a", "b", "c", "d"] {
            fs::create_dir_all(repository_path.join(package)).unwrap();
            fs::write(repository_path.join(package).join("package.json"), "{}").unwrap();
        }
        fs::write(repository_path.join("a/main.js"), "export default 1;\n").unwrap();
        for package in ["b", "c", "d"] {
            fs::write(
                repository_path.join(package).join("main.js"),
                "import value from '../a/main.js';\nexport default value;\n",
            )
            .unwrap();
        }
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "a reaches four"]);
        fs::write(repository_path.join("a/main.js"), "export default 1;\n").unwrap();
        fs::write(repository_path.join("b/main.js"), "export default 1;\n").unwrap();
        for package in ["c", "d"] {
            fs::write(
                repository_path.join(package).join("main.js"),
                "import value from '../b/main.js';\nexport default value;\n",
            )
            .unwrap();
        }

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let report = result.report();
        let package_id = |path: &str| {
            report
                .packages()
                .iter()
                .find(|package| package.path() == path)
                .unwrap()
                .id()
        };
        let a = package_id("a");
        let b = package_id("b");
        let package_rows: Vec<_> = report
            .propagation_comparisons()
            .iter()
            .filter_map(|comparison| match comparison.subject() {
                smackdebt_analysis::PropagationSubject::Package { source } => {
                    Some((source, comparison.before(), comparison.after()))
                }
                smackdebt_analysis::PropagationSubject::File { .. } => None,
            })
            .collect();
        assert!(package_rows.contains(&(a, (4, 4), (1, 4))));
        assert!(package_rows.contains(&(b, (1, 4), (3, 4))));
        assert!(!package_rows.contains(&(a, (4, 4), (3, 4))));
    }

    #[test]
    fn diff_reports_a_new_material_core_from_the_existing_two_graphs() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}").unwrap();
        for index in 0..20 {
            fs::write(
                repository_path.join(format!("f{index}.ts")),
                format!("export const f{index} = {index};\n"),
            )
            .unwrap();
        }
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "initial"]);
        for index in 0..5 {
            let next = (index + 1) % 5;
            fs::write(
                repository_path.join(format!("f{index}.ts")),
                format!(
                    "import {{ f{next} }} from './f{next}.js';\nexport const f{index} = f{next} + 1;\n"
                ),
            )
            .unwrap();
        }

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let comparison = &result.report().core_comparisons()[0];
        assert_eq!(
            comparison.direction(),
            smackdebt_analysis::ComparisonDirection::Worse
        );
        assert_eq!(comparison.before(), (1, 20));
        assert_eq!(comparison.after(), (5, 20));
        assert_eq!(comparison.after_members().len(), 5);
    }

    #[test]
    fn diff_counts_a_core_candidate_before_incomplete_evidence_withholds_it() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}").unwrap();
        for index in 0..20 {
            fs::write(
                repository_path.join(format!("f{index}.ts")),
                format!("export const f{index} = {index};\n"),
            )
            .unwrap();
        }
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "initial"]);
        for index in 0..5 {
            let next = (index + 1) % 5;
            fs::write(
                repository_path.join(format!("f{index}.ts")),
                format!(
                    "import {{ f{next} }} from './f{next}.js';\nexport const f{index} = f{next} + 1;\n"
                ),
            )
            .unwrap();
        }
        fs::write(
            repository_path.join("f5.ts"),
            "import missing from './missing.js';\nexport const f5 = missing;\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let evidence = result.report().diff_graph_evidence().unwrap();
        assert_eq!(evidence.core().total(), 1);
        assert_eq!(evidence.core().current(), 1);
        assert_eq!(evidence.core().base(), 0);
        assert!(result.report().core_comparisons().is_empty());
    }

    #[test]
    fn diff_does_not_compare_disjoint_largest_dependency_cycles() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}").unwrap();
        for index in 0..20 {
            fs::write(
                repository_path.join(format!("f{index}.ts")),
                format!("export const f{index} = {index};\n"),
            )
            .unwrap();
        }
        for index in 0..5 {
            let next = (index + 1) % 5;
            fs::write(
                repository_path.join(format!("f{index}.ts")),
                format!(
                    "import {{ f{next} }} from './f{next}.js';\nexport const f{index} = f{next};\n"
                ),
            )
            .unwrap();
        }
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "first core"]);
        for index in 0..5 {
            fs::write(
                repository_path.join(format!("f{index}.ts")),
                format!("export const f{index} = {index};\n"),
            )
            .unwrap();
        }
        for index in 5..11 {
            let next = if index == 10 { 5 } else { index + 1 };
            fs::write(
                repository_path.join(format!("f{index}.ts")),
                format!(
                    "import {{ f{next} }} from './f{next}.js';\nexport const f{index} = f{next};\n"
                ),
            )
            .unwrap();
        }

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let rows = result.report().core_comparisons();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| {
            row.before_members().contains(&row.anchor())
                && row.after_members().contains(&row.anchor())
        }));
        assert!(
            rows.iter()
                .any(|row| row.before() == (5, 20) && row.after() == (1, 20))
        );
        assert!(
            rows.iter()
                .any(|row| row.before() == (1, 20) && row.after() == (6, 20))
        );
        assert!(
            !rows
                .iter()
                .any(|row| row.before() == (5, 20) && row.after() == (6, 20))
        );
    }

    #[test]
    fn adding_a_code_link_resolves_hidden_coupling_in_the_diff() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}").unwrap();
        fs::create_dir_all(repository_path.join("left")).unwrap();
        fs::create_dir_all(repository_path.join("right")).unwrap();
        for revision in 0..5 {
            fs::write(
                repository_path.join("left/a.ts"),
                format!("export const a = {revision};\n"),
            )
            .unwrap();
            fs::write(
                repository_path.join("right/b.ts"),
                format!("export const b = {revision};\n"),
            )
            .unwrap();
            git(repository_path, ["add", "."]);
            git(
                repository_path,
                ["commit", "-qm", &format!("change {revision}")],
            );
        }
        fs::write(
            repository_path.join("left/a.ts"),
            "import { b } from '../right/b.js';\nexport const a = b + 1;\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let comparison = result
            .report()
            .change_leakage_comparisons()
            .iter()
            .find(|comparison| {
                comparison.kind() == smackdebt_analysis::ChangeLeakageKind::HiddenCoupling
            })
            .expect("the new code link resolves the hidden pair");
        assert_eq!(
            comparison.direction(),
            smackdebt_analysis::ComparisonDirection::Better
        );
    }

    #[test]
    fn diff_counts_a_leakage_candidate_before_incomplete_evidence_withholds_it() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}").unwrap();
        fs::create_dir_all(repository_path.join("left")).unwrap();
        fs::create_dir_all(repository_path.join("right")).unwrap();
        for revision in 0..5 {
            fs::write(
                repository_path.join("left/a.ts"),
                format!("export const a = {revision};\n"),
            )
            .unwrap();
            fs::write(
                repository_path.join("right/b.ts"),
                format!("export const b = {revision};\n"),
            )
            .unwrap();
            git(repository_path, ["add", "."]);
            git(
                repository_path,
                ["commit", "-qm", &format!("change {revision}")],
            );
        }
        fs::write(
            repository_path.join("left/a.ts"),
            "import { b } from '../right/b.js';\nexport const a = b + 1;\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("other.ts"),
            "import missing from './missing.js';\nexport default missing;\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let evidence = result.report().diff_graph_evidence().unwrap();
        // The new dependency resolves one hidden-coupling candidate and creates
        // one leaky-interface candidate. Both are counted before the incomplete
        // current graph withholds them.
        assert_eq!(evidence.leakage().total(), 2);
        assert_eq!(evidence.leakage().current(), 2);
        assert_eq!(evidence.leakage().base(), 0);
        assert!(result.report().change_leakage_comparisons().is_empty());
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
        assert!(
            result.report().architecture_comparisons().is_empty(),
            "{:?}",
            result.report().architecture_comparisons()
        );
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
