use std::num::NonZeroUsize;
use std::path::PathBuf;

use smackdebt_analysis::{
    HealthPolicy, HotspotPolicy, Report, ScopeId, SizePolicy, SourceRole, Thresholds,
};
use thiserror::Error;

use crate::project::{analyze_codebase, analyze_diff};

const DEFAULT_HISTORY_DAYS: u32 = 90;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceRoleRule {
    role: SourceRole,
    pattern: String,
}

impl SourceRoleRule {
    fn new(role: SourceRole, pattern: impl Into<String>) -> Self {
        Self {
            role,
            pattern: pattern.into(),
        }
    }

    pub fn primary(pattern: impl Into<String>) -> Self {
        Self::new(SourceRole::Primary, pattern)
    }

    pub fn test(pattern: impl Into<String>) -> Self {
        Self::new(SourceRole::Test, pattern)
    }

    pub fn example(pattern: impl Into<String>) -> Self {
        Self::new(SourceRole::Example, pattern)
    }

    pub fn benchmark(pattern: impl Into<String>) -> Self {
        Self::new(SourceRole::Benchmark, pattern)
    }

    pub fn fixture(pattern: impl Into<String>) -> Self {
        Self::new(SourceRole::Fixture, pattern)
    }

    pub fn generated(pattern: impl Into<String>) -> Self {
        Self::new(SourceRole::Generated, pattern)
    }

    pub(super) const fn role(&self) -> SourceRole {
        self.role
    }

    pub fn role_name(&self) -> &'static str {
        match self.role {
            SourceRole::Primary => "primary",
            SourceRole::Test => "test",
            SourceRole::Example => "example",
            SourceRole::Benchmark => "benchmark",
            SourceRole::Fixture => "fixture",
            SourceRole::Generated => "generated",
        }
    }

    pub fn pattern(&self) -> &str {
        &self.pattern
    }
}

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

    pub(super) fn threads(self) -> usize {
        match self {
            Self::Automatic => std::thread::available_parallelism().map_or(1, NonZeroUsize::get),
            Self::Fixed(value) => value.get(),
        }
    }
}

/// A request to inspect the current source tree.
#[derive(Clone, Debug)]
pub struct CodebaseRequest {
    pub(super) path: PathBuf,
    pub(super) automatic_scope: bool,
    pub(super) width: ExecutionWidth,
    pub(super) history_days: u32,
    pub(super) excludes: Vec<String>,
    pub(super) policy: HealthPolicy,
    pub(super) role_rules: Vec<SourceRoleRule>,
    pub(super) hotspots: HotspotPolicy,
    pub(super) size: SizePolicy,
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

/// A request to compare the worktree with a ref.
#[derive(Clone, Debug)]
pub struct DiffRequest {
    pub(super) path: PathBuf,
    pub(super) automatic_scope: bool,
    pub(super) reference: Option<String>,
    pub(super) width: ExecutionWidth,
    pub(super) history_days: u32,
    pub(super) policy: HealthPolicy,
    pub(super) role_rules: Vec<SourceRoleRule>,
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

/// Observable work counts used by acceptance and performance checks.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorkStats {
    pub(super) inventory_walks: usize,
    pub(super) inventory_visits: usize,
    pub(super) source_reads: usize,
    pub(super) git_processes: usize,
}

impl WorkStats {
    pub const fn inventory_walks(self) -> usize {
        self.inventory_walks
    }
    pub const fn inventory_visits(self) -> usize {
        self.inventory_visits
    }
    pub const fn source_reads(self) -> usize {
        self.source_reads
    }
    pub const fn git_processes(self) -> usize {
        self.git_processes
    }
}

/// A completed report and the structural work used to produce it.
#[derive(Debug)]
pub struct ProjectReport {
    pub(super) report: Report,
    pub(super) selected_scope: Option<ScopeId>,
    pub(super) stats: WorkStats,
}

impl ProjectReport {
    pub fn report(&self) -> &Report {
        &self.report
    }

    pub fn selected_scope(&self) -> Option<ScopeId> {
        self.selected_scope
    }

    #[doc(hidden)]
    pub const fn stats(&self) -> WorkStats {
        self.stats
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
    /// The selected Git ref does not name anything in this repository.
    #[error("Git ref not found: {0}")]
    UnknownReference(String),
    #[error("source role conflict for {path}: {roles}")]
    SourceRoleConflict { path: PathBuf, roles: String },
    #[error("no source files found under: {0}")]
    NoSourceFiles(PathBuf),
    #[error("not a source file: {0}")]
    NotSourceFile(PathBuf),
}
