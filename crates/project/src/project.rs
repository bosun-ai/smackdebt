//! Repository use cases. This crate is the only place that composes discovery,
//! parsers, Git, health policy, and parallel execution.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use smackdebt_analysis::{
    DiagnosticKind, DirectoryTree, FileId, HealthPolicy, HotspotPolicy, Report, Scope, ScopeId,
    ScopeKind, SizePolicy, SourceTrust, Thresholds,
};
use smackdebt_discovery::{DiscoveredFile, Inventory, SnapshotInventory, is_source_path};
use smackdebt_git::GitRepository;

#[cfg(test)]
use crate::architecture::leakage_evidence_is_complete;
#[cfg(test)]
use crate::candidates::candidates_name_an_asset;
use crate::codebase_report::{CodebaseReportBuilder, SignalPolicies};
use crate::dependencies::DiffTables;
use crate::diff_changes::{
    DiffAliases, DiffChangeSet, DiffHierarchy, DiffPackages, build_diff_hierarchy,
    discover_base_tree, discover_current_tree, resolve_diff_refs, select_diff_changes,
};
use crate::diff_comparisons::{attribute_comparison_files, compare_diff_architecture};
use crate::diff_graphs::{ArchitectureLinks, build_diff_architectures};
use crate::diff_impact::compare_diff_impact;
use crate::diff_report::{
    DiffPlacement, link_architecture_scopes, link_evolution_scopes, link_file_comparison_scopes,
    link_propagation_scopes, new_diff_builder, record_changed_files, record_unchanged_files,
    set_diff_architecture_facts, set_diff_history_facts, stream_diff_evolution,
};
use crate::diff_source::{DiffObjects, analyze_diff_files};
#[cfg(test)]
use crate::history_stream::failed_history_availability;
#[cfg(test)]
use crate::history_stream::history_directory_paths;
use crate::history_stream::load_evolution;
use crate::rating::FileResult;
#[cfg(test)]
use crate::rating::rate_file;
use crate::requests::{
    CodebaseRequest, DEFAULT_HISTORY_DAYS, DiffRequest, ExecutionWidth, ProjectError,
    ProjectReport, SourceRoleRule, WorkStats,
};
use crate::resolution_config::{load_base_resolution_aliases, load_resolution_aliases};
#[cfg(test)]
use crate::roles::classify_source_role;
#[cfg(test)]
use crate::roles::declares_module_syntax;
#[cfg(test)]
use crate::roles::has_generated_javascript_content;
use crate::selection::Selection;
use crate::source_units::analyze_current_files;
use crate::test_scope::{demote_diff_roles, demote_test_declared_roles};
use crate::work::AnalysisWork;
#[cfg(test)]
use smackdebt_analysis::{
    ArchitectureFindingKind, Comparison, DependencyCoverage, DependencyEdge, FileActivity,
    GraphEvidence, HistoryAvailability, Language, Rating, ResolutionDiagnostic,
    ResolutionIssueKind,
};
#[cfg(test)]
use smackdebt_analysis::{Coverage, FileRecord, HealthCounts, PackageId, SourceRole};
#[cfg(test)]
use smackdebt_languages::Analyzer;
#[cfg(test)]
use std::collections::BTreeSet;
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::fs;

impl CodebaseRequest {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            automatic_scope: false,
            width: ExecutionWidth::Automatic,
            history_days: DEFAULT_HISTORY_DAYS,
            excludes: Vec::new(),
            policy: HealthPolicy::default(),
            role_rules: Vec::new(),
            hotspots: HotspotPolicy::default(),
            size: SizePolicy::default(),
        }
    }

    pub fn automatic(path: impl Into<PathBuf>) -> Self {
        let mut request = Self::new(path);
        request.automatic_scope = true;
        request
    }

    pub fn with_width(mut self, width: ExecutionWidth) -> Self {
        self.width = width;
        self
    }

    pub fn with_history_days(mut self, days: u32) -> Self {
        self.history_days = days;
        self
    }

    /// Sets the minimum windowed touch count a rated file needs to be hot.
    pub fn with_minimum_hotspot_touches(mut self, touches: u32) -> Self {
        self.hotspots = HotspotPolicy::new(touches);
        self
    }

    /// Sets the file and container size thresholds.
    pub fn with_size_thresholds(
        mut self,
        file_lines: (u32, u32),
        container_lines: (u32, u32),
    ) -> Self {
        self.size = SizePolicy::new(
            Thresholds::new(file_lines.0, file_lines.1),
            Thresholds::new(container_lines.0, container_lines.1),
        );
        self
    }

    pub fn with_excludes(mut self, excludes: Vec<String>) -> Self {
        self.excludes = excludes;
        self
    }

    pub fn with_role_rules(mut self, rules: Vec<SourceRoleRule>) -> Self {
        self.role_rules = rules;
        self
    }

    pub fn with_thresholds(
        mut self,
        cognitive: (u32, u32),
        cyclomatic: (u32, u32),
        logical_lines: (u32, u32),
        nesting: (u32, u32),
        parameters: (u32, u32),
    ) -> Self {
        self.policy = HealthPolicy::new(
            Thresholds::new(cognitive.0, cognitive.1),
            Thresholds::new(cyclomatic.0, cyclomatic.1),
            Thresholds::new(logical_lines.0, logical_lines.1),
            Thresholds::new(nesting.0, nesting.1),
            Thresholds::new(parameters.0, parameters.1),
        );
        self
    }

    /// Analyzes the selected codebase.
    pub fn analyze(&self) -> Result<ProjectReport, ProjectError> {
        analyze_codebase(self)
    }
}

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

impl DiffRequest {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            automatic_scope: false,
            reference: None,
            width: ExecutionWidth::Automatic,
            history_days: DEFAULT_HISTORY_DAYS,
            policy: HealthPolicy::default(),
            role_rules: Vec::new(),
        }
    }

    pub fn automatic(path: impl Into<PathBuf>) -> Self {
        let mut request = Self::new(path);
        request.automatic_scope = true;
        request
    }

    pub fn with_reference(mut self, reference: impl Into<String>) -> Self {
        self.reference = Some(reference.into());
        self
    }

    pub fn with_width(mut self, width: ExecutionWidth) -> Self {
        self.width = width;
        self
    }

    pub fn with_history_days(mut self, days: u32) -> Self {
        self.history_days = days;
        self
    }

    pub fn with_role_rules(mut self, rules: Vec<SourceRoleRule>) -> Self {
        self.role_rules = rules;
        self
    }

    pub fn with_thresholds(
        mut self,
        cognitive: (u32, u32),
        cyclomatic: (u32, u32),
        logical_lines: (u32, u32),
        nesting: (u32, u32),
        parameters: (u32, u32),
    ) -> Self {
        self.policy = HealthPolicy::new(
            Thresholds::new(cognitive.0, cognitive.1),
            Thresholds::new(cyclomatic.0, cyclomatic.1),
            Thresholds::new(logical_lines.0, logical_lines.1),
            Thresholds::new(nesting.0, nesting.1),
            Thresholds::new(parameters.0, parameters.1),
        );
        self
    }

    /// Compares the worktree with the selected ref.
    pub fn analyze(&self) -> Result<ProjectReport, ProjectError> {
        analyze_diff(self)
    }
}

/// Compares changed source units with the selected ref.
pub(super) fn analyze_diff(request: &DiffRequest) -> Result<ProjectReport, ProjectError> {
    let repository = GitRepository::discover(&request.path)?;
    let (reference, base) = resolve_diff_refs(&repository, request.reference.as_deref())?;
    let changes = repository.changes_from(&base)?;
    let inventory = discover_current_tree(&repository)?;
    let aliases = load_resolution_aliases(repository.root(), &inventory);
    let path_filter = diff_selection_filter(request, repository.root())?;
    let work = AnalysisWork::default();
    let width = request.width.threads().min(changes.len().max(1));
    let mut batch = repository.object_reader(width * 2)?;
    let base_inventory = discover_base_tree(&repository, &mut batch, &base)?;
    require_selected_source(request, &inventory, &base_inventory, path_filter.as_deref())?;
    let DiffChangeSet {
        changed,
        all_changed,
        selected_paths,
    } = select_diff_changes(&inventory, &base_inventory, changes, path_filter.as_deref());
    let selected_count = selected_paths.len();
    let packages = DiffPackages::of(&inventory, &base_inventory, &all_changed);
    let before_aliases = load_base_resolution_aliases(
        &mut batch,
        &base,
        &packages.before_roots,
        &packages.resolution_configs,
    );
    let DiffHierarchy {
        scopes: hierarchy_scopes,
        file_scopes,
        packages: package_records,
    } = build_diff_hierarchy(&inventory, &changed, &packages);
    let objects = DiffObjects {
        root: repository.root().to_path_buf(),
        base,
        reader: batch,
    };
    let (mut results, mut unchanged) =
        analyze_diff_files(request, &inventory, changed, objects, &work)?;
    let side_aliases = DiffAliases {
        current: &aliases,
        before: &before_aliases,
    };
    demote_diff_roles(
        &mut results,
        &mut unchanged,
        side_aliases,
        &request.role_rules,
    );
    let root = ScopeId::from_index(0);
    let mut builder = new_diff_builder(hierarchy_scopes, selected_count);
    let placement = DiffPlacement {
        packages: &packages,
        file_scopes: &file_scopes,
        selected_paths: &selected_paths,
    };
    let mut tables = DiffTables::with_capacity(selected_count);
    record_changed_files(
        &mut builder,
        results,
        &placement,
        &mut tables,
        request.policy,
    );
    record_unchanged_files(&mut builder, unchanged, &placement, &mut tables);
    let architectures =
        build_diff_architectures(&work, &tables, &packages, side_aliases, &inventory);
    let mut architecture_comparisons = compare_diff_architecture(&architectures, &tables, &work);
    attribute_comparison_files(&mut architecture_comparisons, &architectures);
    let architecture_links =
        ArchitectureLinks::of(&architecture_comparisons, &architectures.current);
    work.record_algorithm_pass();
    let (evolution, evolution_links) =
        stream_diff_evolution(request, &repository, &builder, &packages, &architectures);
    let impact = compare_diff_impact(&architectures, &tables, &packages, &evolution.facts);
    let impact_comparisons = set_diff_architecture_facts(
        &mut builder,
        architectures,
        architecture_comparisons,
        impact,
    );
    set_diff_history_facts(&mut builder, evolution, reference);
    link_evolution_scopes(&mut builder, root, &package_records, &evolution_links);
    link_propagation_scopes(
        &mut builder,
        root,
        &package_records,
        &impact_comparisons.propagation,
    );
    link_file_comparison_scopes(
        &mut builder,
        root,
        &impact_comparisons.core,
        &impact_comparisons.leakage,
    );
    link_architecture_scopes(&mut builder, root, &package_records, &architecture_links);
    builder.set_packages(package_records);
    let report = builder.finish();
    let selected_scope = diff_selected_scope(&report, request, path_filter.as_deref(), root)?;
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

/// Resolves the scope an explicit path selection names in the finished
/// report.
fn diff_selected_scope(
    report: &Report,
    request: &DiffRequest,
    path_filter: Option<&Path>,
    root: ScopeId,
) -> Result<Option<ScopeId>, ProjectError> {
    let selected = path_filter.and_then(|path| {
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
    if request.automatic_scope {
        Ok(Some(root))
    } else {
        Ok(Some(selected.ok_or_else(|| {
            ProjectError::NoSourceFiles(request.path.clone())
        })?))
    }
}

/// The path filter an explicit diff scope selects, refused when it points at
/// a file no language claims.
fn diff_selection_filter(
    request: &DiffRequest,
    root: &Path,
) -> Result<Option<PathBuf>, ProjectError> {
    let path_filter = (!request.automatic_scope)
        .then(|| diff_filter(root, &request.path))
        .flatten();
    if !request.automatic_scope && request.path.is_file() && !is_source_path(&request.path) {
        return Err(ProjectError::NotSourceFile(request.path.clone()));
    }
    Ok(path_filter)
}

/// Fails an explicit scope that selects no source in either tree.
fn require_selected_source(
    request: &DiffRequest,
    inventory: &Inventory,
    base_inventory: &SnapshotInventory,
    path_filter: Option<&Path>,
) -> Result<(), ProjectError> {
    if request.automatic_scope {
        return Ok(());
    }
    let includes = |path: &Path| path_filter.is_some_and(|selected| path.starts_with(selected));
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
    Ok(())
}

fn diff_filter(root: &Path, selected: &Path) -> Option<PathBuf> {
    let absolute = std::path::absolute(selected).ok()?;
    let absolute = absolute.canonicalize().unwrap_or(absolute);
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    absolute.strip_prefix(root).ok().map(Path::to_path_buf)
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

    /// An unchanged file in an unsupported language keeps its language and
    /// failed parse on both sides of a diff, so neither graph side claims a
    /// completeness it does not have.
    #[test]
    fn an_unchanged_unsupported_file_is_counted_by_both_diff_sides() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(
            repository_path.join("page.astro"),
            "---\nconst title = 'page';\n---\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("work.rs"),
            "pub fn work() -> i32 { 1 }\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "base"]);
        fs::write(
            repository_path.join("work.rs"),
            "pub fn work() -> i32 { 2 }\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let report = result.report();
        let page = report
            .files()
            .iter()
            .find(|file| file.path() == "page.astro")
            .expect("an unchanged unsupported file stays in the file table");
        assert_eq!(page.language(), Some(Language::Astro));
        assert_eq!(page.role(), SourceRole::Primary);
        assert_ne!(page.trust(), SourceTrust::Trusted);
        let evidence = report
            .diff_graph_evidence()
            .expect("a diff states both graph sides");
        assert_eq!(
            evidence.current().parse_failures(),
            1,
            "the current side counts the unsupported file as its one parse failure"
        );
        assert_eq!(
            evidence.base().parse_failures(),
            1,
            "the base side states the same failure for the same unchanged file"
        );
    }

    /// A diff without a reference compares against the default branch, so the
    /// everyday `smackdebt diff` answers without the user naming anything.
    #[test]
    fn a_diff_without_a_reference_compares_against_the_default_branch() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q", "-b", "master"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(
            repository_path.join("work.rs"),
            "pub fn work() -> i32 { 1 }\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "base"]);
        git(repository_path, ["checkout", "-q", "-b", "feature"]);
        fs::write(
            repository_path.join("work.rs"),
            "pub fn work() -> i32 { 2 }\n",
        )
        .unwrap();

        let result = analyze_diff(&DiffRequest::new(repository_path)).unwrap();
        assert_eq!(
            result.report().comparison_ref(),
            Some("master"),
            "the report names the branch it answered against"
        );
        assert!(
            result
                .report()
                .files()
                .iter()
                .any(|file| file.path() == "work.rs"),
            "the worktree change against the default branch is the diff's subject"
        );
    }

    /// Without a recognizable default branch the diff stops and asks, rather
    /// than comparing against something the user never chose.
    #[test]
    fn a_diff_with_no_default_branch_reports_the_missing_reference() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q", "-b", "trunk"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(
            repository_path.join("work.rs"),
            "pub fn work() -> i32 { 1 }\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "base"]);

        let error = analyze_diff(&DiffRequest::new(repository_path)).unwrap_err();
        assert!(matches!(error, ProjectError::MissingReference));
    }

    /// A diff scoped to a non-source file or a source-free directory fails
    /// with the same exact errors the codebase flow states, carrying the path
    /// the user typed.
    #[test]
    fn a_diff_scoped_to_a_non_source_target_fails_like_the_codebase_flow() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(
            repository_path.join("work.rs"),
            "pub fn work() -> i32 { 1 }\n",
        )
        .unwrap();
        fs::write(repository_path.join("README.txt"), "notes\n").unwrap();
        fs::create_dir_all(repository_path.join("docs")).unwrap();
        fs::write(repository_path.join("docs/notes.txt"), "notes\n").unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "base"]);

        let target = repository_path.join("README.txt");
        let error = analyze_diff(&DiffRequest::new(&target).with_reference("HEAD")).unwrap_err();
        assert!(matches!(error, ProjectError::NotSourceFile(path) if path == target));

        let target = repository_path.join("docs");
        let error = analyze_diff(&DiffRequest::new(&target).with_reference("HEAD")).unwrap_err();
        assert!(matches!(error, ProjectError::NoSourceFiles(path) if path == target));
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
