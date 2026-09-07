//! The diff use case: what one change did, stated from two trees.

use std::path::{Path, PathBuf};

use smackdebt_analysis::{HealthPolicy, Report, Scope, ScopeId};
use smackdebt_discovery::{Inventory, SnapshotInventory, is_source_path};
use smackdebt_git::GitRepository;
use std::sync::atomic::Ordering;

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
use crate::requests::{
    DEFAULT_HISTORY_DAYS, DiffRequest, ExecutionWidth, ProjectError, ProjectReport, SourceRoleRule,
    WorkStats,
};
use crate::resolution_config::{load_base_resolution_aliases, load_resolution_aliases};
use crate::test_scope::demote_diff_roles;
use crate::work::AnalysisWork;

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

    /// Sets the health policy every unit is rated under.
    pub fn with_thresholds(mut self, policy: HealthPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Compares the worktree with the selected ref.
    pub fn analyze(&self) -> Result<ProjectReport, ProjectError> {
        analyze_diff(self)
    }
}
/// Compares changed source units with the selected ref.
pub(crate) fn analyze_diff(request: &DiffRequest) -> Result<ProjectReport, ProjectError> {
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
pub(crate) fn diff_selected_scope(
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
pub(crate) fn diff_selection_filter(
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
pub(crate) fn require_selected_source(
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
pub(crate) fn diff_filter(root: &Path, selected: &Path) -> Option<PathBuf> {
    let absolute = std::path::absolute(selected).ok()?;
    let absolute = absolute.canonicalize().unwrap_or(absolute);
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    absolute.strip_prefix(root).ok().map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{git, repository};
    use std::fs;

    use std::process::Command;

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
}
