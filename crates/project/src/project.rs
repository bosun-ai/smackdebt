//! Repository use cases. This crate is the only place that composes discovery,
//! parsers, Git, health policy, and parallel execution.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use rayon::prelude::*;
use smackdebt_analysis::{
    Comparison, ComparisonId, Coverage, Diagnostic, DiagnosticId, DiagnosticKind, FileActivity,
    FileAnalysis, FileId, FileRecord, Finding, FindingId, HealthAssessment, HealthCounts,
    HealthPolicy, Language, ParseStatus, Rating, Report, ReportBuilder as AnalysisReportBuilder,
    ReportMode, Scope, ScopeId, ScopeKind, compare_units,
};
use smackdebt_discovery::{DiscoveredFile, Inventory};
use smackdebt_git::{Change, GitRepository};
use smackdebt_languages::{AnalysisError as LanguageError, Analyzer};

#[cfg(test)]
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
    let candidates: Vec<&DiscoveredFile> = inventory
        .source_files()
        .filter(|file| selection.includes(file.path().as_path()))
        .collect();
    let source_reads = AtomicUsize::new(0);
    let analyses = analyze_current_files(
        &inventory,
        &candidates,
        request.width,
        request.policy,
        &source_reads,
    )?;

    let (activity, _git_processes, history_error) =
        load_activity(&selection.inventory_root, request.history_days);
    let mut builder = CodebaseReportBuilder::new(
        ReportMode::Codebase,
        selection.label,
        &inventory,
        &candidates,
        &activity,
    );
    if let Some(message) = history_error {
        builder.add_general_diagnostic(
            DiagnosticKind::Other,
            format!("Git history unavailable: {message}"),
        );
    }
    for (file_index, analysis) in analyses.into_iter().enumerate() {
        builder.add_analysis(file_index, analysis);
    }
    let report = builder.finish();
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
        #[cfg(test)]
        stats: WorkStats {
            inventory_walks: 1,
            inventory_visits: _inventory_visits,
            source_reads: source_reads.load(Ordering::Relaxed),
            git_processes: _git_processes,
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

    let path_filter = (!request.automatic_scope)
        .then(|| diff_filter(repository.root(), &request.path))
        .flatten();
    changed.retain(|entry| {
        path_filter
            .as_ref()
            .is_none_or(|path| entry.current_path().starts_with(path))
    });
    let all_changed = changed.clone();
    changed.retain(|entry| Analyzer::language(entry.current_path()) != Language::Unknown);
    let selected_count = changed.len();
    let package_roots = diff_package_roots(repository.root(), &all_changed);
    let mut hierarchy = HierarchyBuilder::new(".".to_owned(), &package_roots);
    for entry in &changed {
        let package_root = nearest_package_root(entry.current_path(), &package_roots);
        let package_index = package_roots
            .iter()
            .position(|root| root == &package_root)
            .unwrap_or(0);
        hierarchy.add_file(entry.current_path(), package_index);
    }
    let file_scopes = hierarchy.file_scopes.clone();
    let source_reads = Arc::new(AtomicUsize::new(0));
    let width = request.width.threads().min(selected_count.max(1));
    let batch = repository.object_reader(width * 2)?;
    let results = analyze_diff_inputs(
        changed,
        repository.root().to_path_buf(),
        base,
        batch,
        request.policy,
        width,
        source_reads.clone(),
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
    for result in results {
        let scope_id = file_scopes
            .get(result.change.current_path())
            .copied()
            .expect("diff hierarchy contains every changed file");
        add_diff_result(&mut builder, result, &mut comparison_index, scope_id);
    }
    let report = builder.finish();
    Ok(ProjectReport {
        report,
        selected_scope: Some(root),
        #[cfg(test)]
        stats: WorkStats {
            inventory_walks: 0,
            inventory_visits: 0,
            source_reads: source_reads.load(Ordering::Relaxed),
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
    source_reads: Arc<AtomicUsize>,
) -> Result<Vec<DiffResult>, ProjectError> {
    if changes.len() <= 1 {
        let mut analyzer = Analyzer::default();
        return Ok(changes
            .into_iter()
            .enumerate()
            .map(|(index, change)| {
                let input =
                    read_diff_input(index, change, &root_path, &base, &mut batch, &source_reads);
                analyze_diff_input(input, policy, &mut analyzer)
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
    std::thread::scope(|threads| {
        let producer = threads.spawn(move || {
            for (index, change) in changes.into_iter().enumerate() {
                let input =
                    read_diff_input(index, change, &root_path, &base, &mut batch, &source_reads);
                if input_tx.send(input).is_err() {
                    break;
                }
            }
        });
        pool.scope(|scope| {
            for _ in 0..worker_count {
                let input_rx = Arc::clone(&input_rx);
                let result_tx = result_tx.clone();
                scope.spawn(move |_| {
                    let mut analyzer = Analyzer::default();
                    loop {
                        let input = {
                            let receiver = input_rx.lock().expect("diff input queue poisoned");
                            receiver.recv()
                        };
                        let Ok(input) = input else { break };
                        if result_tx
                            .send(analyze_diff_input(input, policy, &mut analyzer))
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
    source_reads: &AtomicUsize,
) -> DiffInput {
    let current = if !change.current_exists() {
        InputSide::missing()
    } else {
        match safe_worktree_path(root_path, change.current_path())
            .and_then(|path| fs::read(path).map_err(|error| error.to_string()))
        {
            Ok(bytes) => {
                source_reads.fetch_add(1, Ordering::Relaxed);
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
) -> DiffResult {
    let file_id = FileId::from_index(input.index);
    let current = analyze_diff_side(
        analyzer,
        file_id,
        input.change.current_path(),
        input.current,
        policy,
    );
    let before_path = input.change.base_path();
    let before = analyze_diff_side(analyzer, file_id, before_path, input.before, policy);
    let current_units = match &current {
        DiffSide::Analyzed { analysis, .. } => analysis.units(),
        _ => &[],
    };
    let before_units = match &before {
        DiffSide::Analyzed { analysis, .. } => analysis.units(),
        _ => &[],
    };
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
) -> DiffSide {
    if let Some(error) = input.error {
        return DiffSide::Failed(error);
    }
    let Some(bytes) = input.bytes else {
        return DiffSide::Missing;
    };
    match analyze_bytes(analyzer, file, path, bytes) {
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
) {
    let file_id = FileId::from_index(result.index);

    for comparison in &result.comparisons {
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
    let (coverage, health, language) = diff_side_summary(selected);
    let mut file = FileRecord::new(
        file_id,
        scope_id,
        result.change.current_path().to_string_lossy(),
        coverage,
        health,
    );
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
    reads: &AtomicUsize,
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
                reads.fetch_add(1, Ordering::Relaxed);
                match analyze_bytes(analyzer, FileId::from_index(index), path, source) {
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
) -> Result<FileAnalysis, LanguageError> {
    let _ = file;
    analyzer.analyze(path, source)
}

fn load_activity(
    inventory_root: &Path,
    history_days: u32,
) -> (HashMap<PathBuf, u32>, usize, Option<String>) {
    let Ok(repository) = GitRepository::discover(inventory_root) else {
        return (HashMap::new(), 0, None);
    };
    let relative_root = inventory_root
        .strip_prefix(repository.root())
        .unwrap_or(Path::new(""));
    let history = repository.history(history_days);
    let process_count = repository.git_processes();
    match history {
        Ok(values) => {
            let activity = values
                .into_iter()
                .filter_map(|value| {
                    value
                        .path()
                        .strip_prefix(relative_root)
                        .ok()
                        .map(|path| (path.to_path_buf(), value.touches()))
                })
                .collect();
            (activity, process_count, None)
        }
        Err(error) => (HashMap::new(), process_count, Some(error.to_string())),
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

    fn includes(&self, path: &Path) -> bool {
        if let Some(exact) = &self.exact_file {
            return exact == path;
        }
        self.prefix
            .as_ref()
            .is_none_or(|prefix| path.starts_with(prefix))
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
}

impl<'a> CodebaseReportBuilder<'a> {
    fn new(
        mode: ReportMode,
        label: String,
        inventory: &Inventory,
        candidates: &[&DiscoveredFile],
        activity: &'a HashMap<PathBuf, u32>,
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
        for file in candidates {
            let included_package_index = included_packages[..file.package().index()]
                .iter()
                .filter(|included| **included)
                .count();
            hierarchy.add_file(file.path().as_path(), included_package_index);
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
                let failed = matches!(analysis.parse_status(), ParseStatus::Failed);
                if failed {
                    self.add_diagnostic(
                        file_id,
                        DiagnosticKind::ParseFailure,
                        "parser failed",
                        analysis.source_lines(),
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

    fn finish(self) -> Report {
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
        builder.finish()
    }
}

fn diff_filter(root: &Path, selected: &Path) -> Option<PathBuf> {
    let absolute = std::path::absolute(selected).ok()?;
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
