//! The codebase use case: one walk, one analysis, one report.

use std::path::PathBuf;
use std::sync::atomic::Ordering;

use smackdebt_analysis::{
    DiagnosticKind, DirectoryTree, FileId, HealthPolicy, HotspotPolicy, Scope, ScopeKind,
    SizePolicy, SourceTrust, Thresholds,
};
use smackdebt_discovery::{DiscoveredFile, Inventory};
use std::path::Path;

use crate::codebase_report::{CodebaseReportBuilder, SignalPolicies};
use crate::history_stream::load_evolution;
use crate::rating::FileResult;
use crate::requests::{
    CodebaseRequest, DEFAULT_HISTORY_DAYS, ExecutionWidth, ProjectError, ProjectReport,
    SourceRoleRule, WorkStats,
};
use crate::resolution_config::load_resolution_aliases;
use crate::selection::Selection;
use crate::source_units::analyze_current_files;
use crate::test_scope::demote_test_declared_roles;
use crate::work::AnalysisWork;

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
pub(crate) fn analyze_codebase(request: &CodebaseRequest) -> Result<ProjectReport, ProjectError> {
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
