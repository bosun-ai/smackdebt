//! One streamed history window: the commits Git answers with and the
//! evolutionary accumulation they feed.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use smackdebt_analysis::{
    ContributorId, DirectoryTree, EvolutionAccumulator, FileId, HistoryAvailability,
    HistoryChangeCounts, HistoryChangeFact, HistoryCommitFact, HistoryCoverage, HistoryWindow,
    PackageId, SourceRole, SourceTrust,
};
use smackdebt_git::{ContributorIdentity, GitRepository};

pub(crate) struct EvolutionInput {
    pub(crate) accumulator: EvolutionAccumulator,
    pub(crate) coverage: HistoryCoverage,
}
pub(crate) struct LoadedEvolution {
    pub(crate) evolution: EvolutionInput,
    pub(crate) activity: HashMap<PathBuf, u32>,
    pub(crate) processes: usize,
    pub(crate) diagnostic: Option<String>,
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
pub(crate) fn history_directory_paths(
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
pub(crate) fn load_evolution(
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
                    summary
                        .is_shallow()
                        .then(|| "repository history is shallow".to_owned()),
                )
                .with_changes(HistoryChangeCounts {
                    mapped_eligible: mapped_eligible_changes,
                    context: context_changes,
                    textual: textual_changes,
                    uncounted: uncounted_changes,
                    excluded: excluded_changes,
                    rename_gaps,
                })
                .with_timestamps(summary.newest_timestamp(), summary.oldest_timestamp())
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
                        Some(reason.clone()),
                    )
                    .with_changes(HistoryChangeCounts {
                        mapped_eligible: mapped_eligible_changes,
                        context: context_changes,
                        textual: textual_changes,
                        uncounted: uncounted_changes,
                        excluded: excluded_changes,
                        rename_gaps,
                    })
                    .with_window(window.days(), window_excluded_commits),
                },
                activity,
                processes: process_count,
                diagnostic: Some(format!("Git history {label}: {reason}")),
            }
        }
    }
}
pub(crate) fn failed_history_availability(
    error: &smackdebt_git::GitError,
    streamed_commits: u32,
) -> HistoryAvailability {
    if matches!(error, smackdebt_git::GitError::InvalidOutput(_)) || streamed_commits > 0 {
        HistoryAvailability::Incomplete
    } else {
        HistoryAvailability::Unavailable
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HistoryAlias {
    Resolved(FileId, PackageId, SourceRole, SourceTrust),
    Unusable,
}
