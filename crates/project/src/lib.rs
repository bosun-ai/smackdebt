#![forbid(unsafe_code)]

//! Repository use cases. This crate is the only place that composes discovery,
//! parsers, Git, health policy, and parallel execution.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use rayon::prelude::*;
use smackdebt_analysis::{
    Comparison, ComparisonId, Coverage, Diagnostic, DiagnosticId, DiagnosticKind, FileActivity,
    FileAnalysis, FileId, FileRecord, Finding, FindingId, HealthAssessment, HealthCounts,
    HealthPolicy, Language, ParseStatus, Rating, Report, ReportMode, Scope, ScopeId, ScopeKind,
    Thresholds, UnitId, aggregate_scopes, compare_units,
};
use smackdebt_discovery::{DiscoveredFile, Inventory, InventoryOptions};
use smackdebt_git::{ChangeStatus, GitRepository, HistoryWindow};
use smackdebt_languages::{AnalysisError as LanguageError, Analyzer, SourceFile};
use thiserror::Error;

const DEFAULT_HISTORY_DAYS: u32 = 90;
const PARALLEL_FILE_CUTOVER: usize = 100;

/// The requested execution width.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionWidth {
    Automatic,
    Fixed(NonZeroUsize),
}

impl ExecutionWidth {
    pub fn fixed(width: usize) -> Option<Self> {
        NonZeroUsize::new(width).map(Self::Fixed)
    }

    fn threads(self) -> usize {
        match self {
            Self::Automatic => std::thread::available_parallelism().map_or(1, NonZeroUsize::get),
            Self::Fixed(value) => value.get(),
        }
    }
}

/// A request to inspect the current source tree.
#[derive(Clone, Debug)]
pub struct CodebaseRequest {
    path: PathBuf,
    automatic_scope: bool,
    width: ExecutionWidth,
    history_days: u32,
    excludes: Vec<String>,
    policy: HealthPolicy,
}

impl CodebaseRequest {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            automatic_scope: false,
            width: ExecutionWidth::Automatic,
            history_days: DEFAULT_HISTORY_DAYS,
            excludes: Vec::new(),
            policy: HealthPolicy::default(),
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

    pub fn with_excludes(mut self, excludes: Vec<String>) -> Self {
        self.excludes = excludes;
        self
    }

    pub fn with_policy(mut self, policy: HealthPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn with_thresholds(
        mut self,
        cognitive: (u32, u32),
        cyclomatic: (u32, u32),
        logical_lines: (u32, u32),
    ) -> Self {
        self.policy = HealthPolicy::new(
            Thresholds::new(cognitive.0, cognitive.1),
            Thresholds::new(cyclomatic.0, cyclomatic.1),
            Thresholds::new(logical_lines.0, logical_lines.1),
        );
        self
    }
}

/// A request to compare the worktree with a ref.
#[derive(Clone, Debug)]
pub struct DiffRequest {
    path: PathBuf,
    automatic_scope: bool,
    reference: Option<String>,
    width: ExecutionWidth,
    policy: HealthPolicy,
}

impl DiffRequest {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            automatic_scope: false,
            reference: None,
            width: ExecutionWidth::Automatic,
            policy: HealthPolicy::default(),
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

    pub fn with_policy(mut self, policy: HealthPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn with_thresholds(
        mut self,
        cognitive: (u32, u32),
        cyclomatic: (u32, u32),
        logical_lines: (u32, u32),
    ) -> Self {
        self.policy = HealthPolicy::new(
            Thresholds::new(cognitive.0, cognitive.1),
            Thresholds::new(cyclomatic.0, cyclomatic.1),
            Thresholds::new(logical_lines.0, logical_lines.1),
        );
        self
    }
}

/// Observable work counts used by acceptance and performance checks.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorkStats {
    pub inventory_walks: usize,
    pub inventory_visits: usize,
    pub source_reads: usize,
    pub git_processes: usize,
}

/// A completed report and the structural work used to produce it.
#[derive(Debug)]
pub struct ProjectReport {
    report: Report,
    stats: WorkStats,
}

impl ProjectReport {
    pub fn report(&self) -> &Report {
        &self.report
    }

    pub const fn stats(&self) -> WorkStats {
        self.stats
    }

    pub fn into_report(self) -> Report {
        self.report
    }
}

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("cannot inspect {path}: {source}")]
    Inspect {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot create analysis workers: {0}")]
    WorkerPool(#[from] rayon::ThreadPoolBuildError),
    #[error("Git comparison failed: {0}")]
    Git(#[from] smackdebt_git::GitError),
    #[error("no default Git ref was found; pass a ref explicitly")]
    MissingReference,
}

/// Analyzes the selected codebase.
pub fn analyze_codebase(request: &CodebaseRequest) -> Result<ProjectReport, ProjectError> {
    let selection = Selection::resolve(&request.path, request.automatic_scope)?;
    let inventory = Inventory::discover_with(
        &selection.inventory_root,
        InventoryOptions {
            excludes: request.excludes.clone(),
            include_other_files: false,
        },
    )
    .map_err(|source| ProjectError::Inspect {
        path: selection.inventory_root.clone(),
        source,
    })?;
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

    let (activity, git_processes, history_error) =
        load_activity(&selection.inventory_root, request.history_days);
    let mut builder = ReportBuilder::new(
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
    let inventory_stats = inventory.stats();
    Ok(ProjectReport {
        report,
        stats: WorkStats {
            inventory_walks: 1,
            inventory_visits: inventory_stats.directories_visited + inventory_stats.files_visited,
            source_reads: source_reads.load(Ordering::Relaxed),
            git_processes,
        },
    })
}

/// Compares changed source units with the selected ref.
pub fn analyze_diff(request: &DiffRequest) -> Result<ProjectReport, ProjectError> {
    let repository = GitRepository::discover(&request.path)?;
    let reference = match &request.reference {
        Some(reference) => reference.clone(),
        None => repository
            .default_ref()?
            .ok_or(ProjectError::MissingReference)?,
    };
    let base = repository.merge_base(&reference, "HEAD")?;
    let mut changed = repository.changed_paths(&base)?;
    for entry in repository.status()?.entries {
        if entry.status == ChangeStatus::Untracked
            && !changed.iter().any(|known| known.path == entry.path)
        {
            changed.push(entry);
        }
    }
    changed.sort_by(|left, right| left.path.cmp(&right.path));
    changed.dedup_by(|left, right| left.path == right.path);

    let path_filter = (!request.automatic_scope)
        .then(|| diff_filter(repository.root(), &request.path))
        .flatten();
    changed.retain(|entry| {
        path_filter
            .as_ref()
            .is_none_or(|path| entry.path.starts_with(path))
    });
    let changed_path_count = changed.len();
    changed.retain(|entry| smackdebt_languages::detect(&entry.path) != Language::Unknown);
    let skipped_non_source = changed_path_count - changed.len();
    let selected_count = changed.len();
    let source_reads = Arc::new(AtomicUsize::new(0));
    let width = request.width.threads().min(selected_count.max(1));
    let batch = repository.batch_reader(width * 2)?;
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
    let mut report = Report::with_capacity(
        ReportMode::Diff,
        selected_count + 1,
        selected_count,
        0,
        selected_count * 2,
        selected_count,
    );
    report.add_scope(Scope::new(root, ScopeKind::Repository, ".", None));
    report.set_root(root);
    let mut comparison_index = 0usize;
    for result in results {
        add_diff_result(&mut report, result, &mut comparison_index);
    }
    if skipped_non_source > 0 {
        report.add_diagnostic(Diagnostic::new(
            DiagnosticId::from_index(report.diagnostics().len()),
            None,
            DiagnosticKind::Other,
            format!("{skipped_non_source} changed non-source files were skipped"),
            0,
        ));
    }
    let files = report.files().to_vec();
    aggregate_scopes(report.scopes_mut(), &files, root);

    Ok(ProjectReport {
        report,
        stats: WorkStats {
            inventory_walks: 0,
            inventory_visits: 0,
            source_reads: source_reads.load(Ordering::Relaxed),
            git_processes: repository.process_stats().git_processes,
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
    change: smackdebt_git::ChangedPath,
    current: InputSide,
    before: InputSide,
}

struct DiffResult {
    index: usize,
    change: smackdebt_git::ChangedPath,
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
    changes: Vec<smackdebt_git::ChangedPath>,
    root_path: PathBuf,
    base: String,
    mut batch: smackdebt_git::BatchObjectReader,
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
    change: smackdebt_git::ChangedPath,
    root_path: &Path,
    base: &str,
    batch: &mut smackdebt_git::BatchObjectReader,
    source_reads: &AtomicUsize,
) -> DiffInput {
    let current = if change.status == ChangeStatus::Deleted {
        InputSide::missing()
    } else {
        match safe_worktree_path(root_path, &change.path)
            .and_then(|path| fs::read(path).map_err(|error| error.to_string()))
        {
            Ok(bytes) => {
                source_reads.fetch_add(1, Ordering::Relaxed);
                InputSide::bytes(bytes)
            }
            Err(error) => InputSide::failed(format!("could not read current file: {error}")),
        }
    };
    let previous_path = change.previous_path.as_ref().unwrap_or(&change.path);
    let before = if matches!(change.status, ChangeStatus::Added | ChangeStatus::Untracked) {
        InputSide::missing()
    } else {
        match batch.read_path(base, previous_path) {
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
    let current = analyze_diff_side(analyzer, file_id, &input.change.path, input.current, policy);
    let before_path = input
        .change
        .previous_path
        .as_ref()
        .unwrap_or(&input.change.path);
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

fn add_diff_result(report: &mut Report, result: DiffResult, comparison_index: &mut usize) {
    let file_id = FileId::from_index(result.index);
    let scope_id = ScopeId::from_index(report.scopes().len());
    report.scopes_mut()[0].add_child(scope_id);
    report.add_scope(Scope::new(
        scope_id,
        ScopeKind::File,
        result.change.path.to_string_lossy(),
        Some(ScopeId::from_index(0)),
    ));

    for comparison in &result.comparisons {
        report.add_comparison(Comparison::new(
            ComparisonId::from_index(*comparison_index),
            comparison.identity().clone(),
            comparison.kind(),
            comparison.before(),
            comparison.after(),
            comparison.before_rating(),
            comparison.after_rating(),
        ));
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
        result.change.path.to_string_lossy(),
        coverage,
        health,
    );
    if let Some(language) = language {
        file = file.with_language(language);
    }
    report.add_file(file);
    report.scopes_mut()[scope_id.index()].add_file(file_id);
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

fn add_diff_diagnostic(report: &mut Report, file: FileId, side: &DiffSide, label: &str) {
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
        DiagnosticId::from_index(report.diagnostics().len()),
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
        let language = smackdebt_languages::detect(path);
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
    analyzer.analyze(SourceFile { file, path, source })
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
    let history = repository.history(HistoryWindow::days(history_days));
    let process_count = repository.process_stats().git_processes;
    match history {
        Ok(values) => {
            let activity = values
                .into_iter()
                .filter_map(|value| {
                    value
                        .path
                        .strip_prefix(relative_root)
                        .ok()
                        .map(|path| (path.to_path_buf(), value.touches))
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
    label: String,
}

impl Selection {
    fn resolve(path: &Path, automatic_scope: bool) -> Result<Self, ProjectError> {
        if automatic_scope && let Ok(repository) = GitRepository::discover(path) {
            return Ok(Self {
                inventory_root: repository.root().to_path_buf(),
                exact_file: None,
                label: ".".to_owned(),
            });
        }
        let absolute = std::path::absolute(path).map_err(|source| ProjectError::Inspect {
            path: path.to_path_buf(),
            source,
        })?;
        if absolute.is_file() {
            let root = absolute.parent().unwrap_or(Path::new(".")).to_path_buf();
            let exact_file = absolute.file_name().map(PathBuf::from);
            return Ok(Self {
                inventory_root: root,
                exact_file,
                label: path.display().to_string(),
            });
        }
        Ok(Self {
            inventory_root: absolute,
            exact_file: None,
            label: path.display().to_string(),
        })
    }

    fn includes(&self, path: &Path) -> bool {
        self.exact_file.as_ref().is_none_or(|exact| exact == path)
    }
}

struct ReportBuilder<'a> {
    mode: ReportMode,
    scopes: Vec<Scope>,
    files: Vec<FileRecord>,
    findings: Vec<Finding>,
    diagnostics: Vec<Diagnostic>,
    file_scopes: Vec<ScopeId>,
    activity: &'a HashMap<PathBuf, u32>,
}

impl<'a> ReportBuilder<'a> {
    fn new(
        mode: ReportMode,
        label: String,
        inventory: &Inventory,
        candidates: &[&DiscoveredFile],
        activity: &'a HashMap<PathBuf, u32>,
    ) -> Self {
        let root = ScopeId::from_index(0);
        let mut scopes = vec![Scope::new(root, ScopeKind::Repository, label, None)];
        let mut package_scopes = Vec::with_capacity(inventory.packages().len());
        for package in inventory.packages() {
            let id = ScopeId::from_index(scopes.len());
            scopes[root.index()].add_child(id);
            scopes.push(Scope::new(
                id,
                ScopeKind::Package,
                package.root().to_string(),
                Some(root),
            ));
            package_scopes.push(id);
        }

        let mut directory_scopes: BTreeMap<(usize, PathBuf), ScopeId> = BTreeMap::new();
        let mut file_scopes = Vec::with_capacity(candidates.len());
        for file in candidates {
            let package_index = file.package().index();
            let package_scope = package_scopes[package_index];
            let package_root = inventory.packages()[package_index].root().as_path();
            let relative_directory = file
                .path()
                .as_path()
                .parent()
                .unwrap_or(Path::new(""))
                .strip_prefix(package_root)
                .unwrap_or(Path::new(""));
            let mut parent = package_scope;
            let mut accumulated = PathBuf::new();
            for component in relative_directory.components() {
                accumulated.push(component);
                let key = (package_index, accumulated.clone());
                parent = if let Some(id) = directory_scopes.get(&key) {
                    *id
                } else {
                    let id = ScopeId::from_index(scopes.len());
                    scopes[parent.index()].add_child(id);
                    scopes.push(Scope::new(
                        id,
                        ScopeKind::Directory,
                        accumulated.display().to_string(),
                        Some(parent),
                    ));
                    directory_scopes.insert(key, id);
                    id
                };
            }
            let file_scope = ScopeId::from_index(scopes.len());
            scopes[parent.index()].add_child(file_scope);
            scopes.push(Scope::new(
                file_scope,
                ScopeKind::File,
                file.path().to_string(),
                Some(parent),
            ));
            file_scopes.push(file_scope);
        }

        Self {
            mode,
            scopes,
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
                        UnitId::from_index(self.findings.len()),
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

    fn finish(mut self) -> Report {
        let root = ScopeId::from_index(0);
        aggregate_scopes(&mut self.scopes, &self.files, root);
        let mut report = Report::with_capacity(
            self.mode,
            self.scopes.len(),
            self.files.len(),
            self.findings.len(),
            self.diagnostics.len(),
            0,
        );
        for scope in self.scopes {
            report.add_scope(scope);
        }
        report.set_root(root);
        for file in self.files {
            report.add_file(file);
        }
        for finding in self.findings {
            report.add_finding(finding);
        }
        for diagnostic in self.diagnostics {
            report.add_diagnostic(diagnostic);
        }
        report
    }
}

fn diff_filter(root: &Path, selected: &Path) -> Option<PathBuf> {
    let absolute = std::path::absolute(selected).ok()?;
    absolute.strip_prefix(root).ok().map(Path::to_path_buf)
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
        assert_eq!(result.report().scopes().len(), 3);
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
