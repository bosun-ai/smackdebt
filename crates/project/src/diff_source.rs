//! Reading and analyzing both sides of every changed file.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, mpsc};

use smackdebt_analysis::{
    Comparison, DependencySyntax, FileAnalysis, FileId, HealthCounts, HealthPolicy, Language,
    ParseStatus, SourceRole, compare_units,
};
use smackdebt_discovery::{DiscoveredFile, Inventory};
use smackdebt_git::ObjectReader;
use smackdebt_languages::{AnalysisError as LanguageError, Analyzer};

use crate::diff_changes::SelectedChange;
use crate::rating::{FileResult, SourcePolicy, file_result_role, rated_health};
use crate::requests::{DiffRequest, ProjectError, SourceRoleRule};
use crate::roles::{classify_source_role, role_for_unavailable_source};
use crate::source_units::{AnalysisSession, analyze_bytes, analyze_current_files};
use crate::work::AnalysisWork;

/// The Git objects one diff analysis reads from.
pub(crate) struct DiffObjects {
    pub(crate) root: PathBuf,
    pub(crate) base: String,
    pub(crate) reader: ObjectReader,
}
/// The files the change left alone, the analysis each produced, and the role
/// the base tree gives each one — index-aligned tables starting after the
/// changed files.
pub(crate) struct DiffUnchanged<'a> {
    pub(crate) candidates: Vec<&'a DiscoveredFile>,
    pub(crate) results: Vec<FileResult>,
    pub(crate) before_roles: Vec<SourceRole>,
    pub(crate) first_file_index: usize,
}
/// Analyzes the changed files against the base tree and the unchanged files
/// in place.
pub(crate) fn analyze_diff_files<'a>(
    request: &DiffRequest,
    inventory: &'a Inventory,
    changed: Vec<SelectedChange>,
    objects: DiffObjects,
    work: &AnalysisWork,
) -> Result<(Vec<DiffResult>, DiffUnchanged<'a>), ProjectError> {
    let changed_count = changed.len();
    let width = request.width.threads().min(changed_count.max(1));
    let results = analyze_diff_inputs(
        changed,
        objects,
        DiffAnalysisPolicy {
            health: request.policy,
            roles: request.role_rules.clone(),
        },
        width,
        work.clone(),
    )?;
    let changed_paths: BTreeSet<_> = results
        .iter()
        .filter(|result| result.change.current_exists())
        .map(|result| result.change.current_path().to_path_buf())
        .collect();
    let candidates: Vec<_> = inventory
        .source_files()
        .filter(|file| {
            !changed_paths.contains(file.path().as_path())
                && Analyzer::language(file.path().as_path()) != Language::Unknown
        })
        .collect();
    let unchanged = analyze_current_files(
        inventory,
        &candidates,
        request.width,
        SourcePolicy {
            health: request.policy,
            rules: &request.role_rules,
        },
        work,
    )?;
    let before_roles = unchanged.iter().map(file_result_role).collect();
    Ok((
        results,
        DiffUnchanged {
            candidates,
            results: unchanged,
            before_roles,
            first_file_index: changed_count,
        },
    ))
}
pub(crate) struct InputSide {
    pub(crate) bytes: Option<Vec<u8>>,
    pub(crate) error: Option<String>,
}
pub(crate) struct DiffInput {
    pub(crate) index: usize,
    pub(crate) change: SelectedChange,
    pub(crate) current: InputSide,
    pub(crate) before: InputSide,
}
pub(crate) struct DiffResult {
    pub(crate) index: usize,
    pub(crate) change: SelectedChange,
    pub(crate) current: DiffSide,
    pub(crate) before: DiffSide,
    pub(crate) comparisons: Vec<Comparison>,
}
pub(crate) enum DiffSide {
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
pub(crate) struct DiffAnalysisPolicy {
    pub(crate) health: HealthPolicy,
    pub(crate) roles: Vec<SourceRoleRule>,
}

pub(crate) fn analyze_diff_inputs(
    changes: Vec<SelectedChange>,
    objects: DiffObjects,
    policy: DiffAnalysisPolicy,
    width: usize,
    work: AnalysisWork,
) -> Result<Vec<DiffResult>, ProjectError> {
    let mut objects = objects;
    if changes.len() <= 1 {
        let mut analyzer = Analyzer::default();
        let mut session = AnalysisSession {
            analyzer: &mut analyzer,
            work: &work,
        };
        let results = changes
            .into_iter()
            .enumerate()
            .map(|(index, change)| {
                let input = read_diff_input(index, change, &mut objects, session.work);
                analyze_diff_input(input, &policy, &mut session)
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
                let input = read_diff_input(index, change, &mut objects, &producer_work);
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
                    let mut session = AnalysisSession {
                        analyzer: &mut analyzer,
                        work: &worker_work,
                    };
                    loop {
                        let input = {
                            let receiver = input_rx.lock().expect("diff input queue poisoned");
                            receiver.recv()
                        };
                        let Ok(input) = input else { break };
                        if result_tx
                            .send(analyze_diff_input(input, &policy, &mut session))
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
pub(crate) fn diff_role_conflict(result: &DiffResult) -> Option<(PathBuf, String)> {
    [&result.current, &result.before]
        .into_iter()
        .find_map(|side| match side {
            DiffSide::RoleConflict { path, roles } => Some((path.clone(), roles.clone())),
            _ => None,
        })
}
pub(crate) fn read_diff_input(
    index: usize,
    change: SelectedChange,
    objects: &mut DiffObjects,
    work: &AnalysisWork,
) -> DiffInput {
    let current = if !change.current_exists() {
        InputSide::missing()
    } else {
        match safe_worktree_path(&objects.root, change.current_path())
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
        match objects.reader.read_path(&objects.base, change.base_path()) {
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
pub(crate) fn analyze_diff_input(
    input: DiffInput,
    policy: &DiffAnalysisPolicy,
    worker: &mut AnalysisSession<'_>,
) -> DiffResult {
    let file_id = FileId::from_index(input.index);
    let current = analyze_diff_side(
        worker,
        file_id,
        input.change.current_path(),
        input.current,
        policy,
    );
    let before_path = input.change.base_path();
    let before = analyze_diff_side(worker, file_id, before_path, input.before, policy);
    worker.work.record_algorithm_pass();
    let mut comparisons = match (diff_units(&before), diff_units(&current)) {
        (Some(before), Some(current)) => compare_units(before, current, policy.health),
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
pub(crate) fn diff_units(side: &DiffSide) -> Option<&[smackdebt_analysis::UnitFact]> {
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
pub(crate) fn analyze_diff_side(
    worker: &mut AnalysisSession<'_>,
    file: FileId,
    path: &Path,
    input: InputSide,
    policy: &DiffAnalysisPolicy,
) -> DiffSide {
    if let Some(error) = input.error {
        return role_for_unavailable_source(path, &policy.roles).map_or_else(
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
    let role = match classify_source_role(path, &bytes, &policy.roles) {
        Ok(role) => role,
        Err(roles) => {
            return DiffSide::RoleConflict {
                path: path.to_path_buf(),
                roles,
            };
        }
    };
    match analyze_bytes(worker.analyzer, file, path, bytes, worker.work) {
        Ok(analysis) => {
            let health = rated_health(&analysis, role, policy.health);
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
pub(crate) fn safe_worktree_path(root: &Path, relative: &Path) -> Result<PathBuf, String> {
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
impl DiffSide {
    pub(crate) fn role(&self) -> Option<SourceRole> {
        match self {
            Self::Analyzed { role, .. }
            | Self::Unsupported { role, .. }
            | Self::Failed { role, .. } => Some(*role),
            Self::Missing | Self::RoleConflict { .. } => None,
        }
    }

    pub(crate) fn references(&self) -> &[DependencySyntax] {
        match self {
            Self::Analyzed { analysis, .. } => analysis.dependencies(),
            _ => &[],
        }
    }

    pub(crate) fn demote_to_test(&mut self) {
        let role = match self {
            Self::Analyzed { role, .. }
            | Self::Unsupported { role, .. }
            | Self::Failed { role, .. } => role,
            Self::Missing | Self::RoleConflict { .. } => return,
        };
        *role = role.demoted_by_test_scope();
    }
}

impl InputSide {
    pub(crate) fn missing() -> Self {
        Self {
            bytes: None,
            error: None,
        }
    }

    pub(crate) fn bytes(bytes: Vec<u8>) -> Self {
        Self {
            bytes: Some(bytes),
            error: None,
        }
    }

    pub(crate) fn failed(error: String) -> Self {
        Self {
            bytes: None,
            error: Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::analyze_diff;
    use crate::requests::DiffRequest;
    use crate::requests::ExecutionWidth;
    use crate::test_support::{git, repository};
    use smackdebt_analysis::ScopeId;
    use smackdebt_analysis::SourceTrust;
    use smackdebt_analysis::{HealthPolicy, Thresholds};
    use std::fs;

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
                .with_thresholds(HealthPolicy::new(
                    Thresholds::new(1, 2),
                    Thresholds::new(1, 2),
                    Thresholds::new(1, 2),
                    Thresholds::new(4, 7),
                    Thresholds::new(6, 9),
                )),
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
}
