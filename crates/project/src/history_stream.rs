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
use smackdebt_git::{ContributorIdentity, GitRepository, HistoryChange, HistoryCommit};

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
/// Everything one streamed history window accumulates: the identity tables
/// the paths resolve through and the counters the coverage will state.
struct HistoryAccumulation<'a> {
    relative_root: &'a Path,
    directories: &'a DirectoryTree,
    window: HistoryWindow,
    aliases: HashMap<PathBuf, HistoryAlias>,
    file_paths: HashMap<FileId, PathBuf>,
    contributors: HashMap<ContributorIdentity, ContributorId>,
    accumulator: EvolutionAccumulator,
    activity: HashMap<PathBuf, u32>,
    textual_changes: u32,
    uncounted_changes: u32,
    eligible_commits: u32,
    mapped_eligible_changes: u32,
    context_changes: u32,
    excluded_changes: u32,
    rename_gaps: u32,
    streamed_commits: u32,
    window_excluded_commits: u32,
}

impl HistoryAccumulation<'_> {
    /// Accepts one streamed commit into the tables and counters.
    fn accept(&mut self, commit: &HistoryCommit) {
        self.streamed_commits += 1;
        // The window filter runs inside the streamed history process on landed
        // dates, so out-of-window history is never streamed. This defensive
        // boundary check compares the same landed instant and keeps any
        // straggler from becoming a fact; it counts boundary rejects only.
        if !self.window.includes(commit.timestamp()) {
            self.window_excluded_commits += 1;
            return;
        }
        let contributor = self.contributor(commit.contributor());
        let mut changes = Vec::new();
        let mut contains_eligible_source = false;
        for change in commit.changes() {
            let Some((fact, eligible)) = self.change_fact(change) else {
                continue;
            };
            contains_eligible_source |= eligible;
            changes.push(fact);
        }
        if !changes.is_empty() {
            self.accumulator.accept(
                HistoryCommitFact::new(contributor, changes),
                self.directories,
            );
        }
        self.eligible_commits += u32::from(contains_eligible_source);
    }

    /// The identity one contributor streams under, minted on first sight.
    fn contributor(&mut self, identity: &ContributorIdentity) -> ContributorId {
        let next = ContributorId::from_index(self.contributors.len());
        *self.contributors.entry(identity.clone()).or_insert(next)
    }

    /// The fact one streamed change states, and whether it is verdict
    /// eligible — or nothing when no mapped file answers for its path.
    fn change_fact(&mut self, change: &HistoryChange) -> Option<(HistoryChangeFact, bool)> {
        let path = change
            .path()
            .strip_prefix(self.relative_root)
            .unwrap_or(change.path());
        let identity = self.aliases.get(path).copied();
        let Some(HistoryAlias::Resolved(file, package, role, trust)) = identity else {
            self.excluded_changes += 1;
            return None;
        };
        self.track_rename(change, HistoryAlias::Resolved(file, package, role, trust));
        if change.added_lines().is_some() && change.deleted_lines().is_some() {
            self.textual_changes += 1;
        } else {
            self.uncounted_changes += 1;
        }
        if let Some(path) = self.file_paths.get(&file) {
            *self.activity.entry(path.clone()).or_default() += 1;
        }
        let eligible = role.affects_verdict() && trust == SourceTrust::Trusted;
        if eligible {
            self.mapped_eligible_changes += 1;
        } else {
            self.context_changes += 1;
        }
        let fact =
            HistoryChangeFact::new(file, package, change.added_lines(), change.deleted_lines())
                .with_source_evidence(role, trust);
        Some((fact, eligible))
    }

    /// Follows one rename backward, or marks the previous path unusable when
    /// two identities claim it.
    fn track_rename(&mut self, change: &HistoryChange, resolved: HistoryAlias) {
        let HistoryAlias::Resolved(file, package, role, trust) = resolved else {
            unreachable!("only resolved identities reach rename tracking");
        };
        let Some(previous) = change.previous_path() else {
            return;
        };
        let previous = previous
            .strip_prefix(self.relative_root)
            .unwrap_or(previous)
            .to_path_buf();
        match self.aliases.get(&previous) {
            Some(HistoryAlias::Resolved(existing_file, existing_package, _, _))
                if (*existing_file, *existing_package) != (file, package) =>
            {
                self.rename_gaps += 1;
                self.aliases.insert(previous, HistoryAlias::Unusable);
            }
            Some(HistoryAlias::Unusable) => {}
            _ => {
                self.aliases
                    .insert(previous, HistoryAlias::Resolved(file, package, role, trust));
            }
        }
    }
}

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
    let mut accumulator = EvolutionAccumulator::default();
    for (_, file, package, role, trust) in files {
        accumulator.register_source(*file, *package, *role, *trust);
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(i64::MIN, |duration| duration.as_secs() as i64);
    let window = HistoryWindow::of_days(history_days, now);
    let mut state = HistoryAccumulation {
        relative_root,
        directories,
        window,
        aliases: files
            .iter()
            .map(|(path, file, package, role, trust)| {
                (
                    path.clone(),
                    HistoryAlias::Resolved(*file, *package, *role, *trust),
                )
            })
            .collect(),
        file_paths: files
            .iter()
            .map(|(path, file, _, _, _)| (*file, path.clone()))
            .collect(),
        contributors: HashMap::new(),
        accumulator,
        activity: HashMap::new(),
        textual_changes: 0,
        uncounted_changes: 0,
        eligible_commits: 0,
        mapped_eligible_changes: 0,
        context_changes: 0,
        excluded_changes: 0,
        rename_gaps: 0,
        streamed_commits: 0,
        window_excluded_commits: 0,
    };
    let history = repository.stream_history(Some(window.cutoff()), |commit| {
        state.accept(&commit);
        Ok(())
    });
    let process_count = repository.git_processes();
    let accumulator = state.accumulator;
    let activity = state.activity;
    let counts = HistoryChangeCounts {
        mapped_eligible: state.mapped_eligible_changes,
        context: state.context_changes,
        textual: state.textual_changes,
        uncounted: state.uncounted_changes,
        excluded: state.excluded_changes,
        rename_gaps: state.rename_gaps,
    };
    let eligible_commits = state.eligible_commits;
    let streamed_commits = state.streamed_commits;
    let window_excluded_commits = state.window_excluded_commits;
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
                .with_changes(counts)
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
                    .with_changes(counts)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebase::analyze_codebase;
    use crate::requests::CodebaseRequest;
    use crate::requests::SourceRoleRule;
    use crate::test_support::{git, git_dated};
    use smackdebt_analysis::Report;
    use std::fs;
    use std::path::PathBuf;

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
}
