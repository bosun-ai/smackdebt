use std::collections::{BTreeMap, BTreeSet};
use std::iter::once;

use crate::median::nearest_rank_median_of_counts;
use crate::{
    ChangeAmplification, DirectoryAmplification, DirectoryId, DirectoryTree, HistoryCommitFact,
};

/// The most files one commit may be observed to have touched.
///
/// A repository-wide sweep — a formatting pass, a license header, a generated
/// rewrite — says nothing about what a change costs, and left unclamped it
/// would put one histogram key per distinct sweep size in every directory it
/// touched. The clamp bounds the key space of every histogram to this many
/// keys, which is what keeps the median a walk over counts rather than over
/// commits.
///
/// This is a proposed constant under review.
pub const AMPLIFICATION_MAX_FILES: u32 = 1_000;

/// The observations a directory needs before its median is called typical.
///
/// This is a proposed constant under review.
pub const AMPLIFICATION_MIN_COMMITS: u32 = 10;

/// The median a directory needs before stating it says anything.
///
/// This is a proposed constant under review.
pub const AMPLIFICATION_MIN_MEDIAN: u32 = 3;

/// Accumulates how many files a commit touched, per directory, one commit at a
/// time.
///
/// The history stream delivers one commit at a time and retains no per-commit
/// file list, so the histograms are counted inline beside the file pairs. Each
/// histogram is sparse: it holds one key per distinct observed file count, and
/// the clamp bounds that key space however wide a commit was.
///
/// The observation a commit contributes is repository-wide rather than local:
/// the question a scope answers is what a change that touched this directory
/// cost altogether, not how much of it happened to land here. A commit is
/// counted once per directory, never once per file, so a commit that rewrote
/// eight files of one directory is one observation there and not eight.
#[derive(Debug, Default)]
pub(crate) struct ChangeAmplificationAccumulator {
    histograms: BTreeMap<DirectoryId, BTreeMap<u32, u32>>,
}

impl ChangeAmplificationAccumulator {
    /// Counts one commit in every directory it touched and in their ancestors.
    ///
    /// The same directory tree must be passed for every commit of one report:
    /// a file's identity is a position in the tree the report built once, and
    /// two trees would file one commit under two different directories.
    ///
    /// The bulk-commit guard that holds a sweeping commit back from pair
    /// accumulation never reaches here: a sweep is exactly the change whose
    /// cost this fact is about, so it contributes its one observation like any
    /// other commit.
    pub(crate) fn accept(&mut self, commit: &HistoryCommitFact, directories: &DirectoryTree) {
        let files: BTreeSet<_> = commit
            .changes()
            .iter()
            .filter(|change| change.enters_change_graph())
            .map(|change| change.file())
            .collect();
        let observation = u32::try_from(files.len())
            .unwrap_or(AMPLIFICATION_MAX_FILES)
            .min(AMPLIFICATION_MAX_FILES);
        if observation == 0 {
            return;
        }
        let mut counted: BTreeSet<DirectoryId> = BTreeSet::new();
        for file in files {
            let directory = directories.directory_of(file);
            debug_assert!(
                directory.is_some(),
                "the change graph names files the directory tree holds"
            );
            // `ancestors` excludes the directory itself, which is the one the
            // file actually sits in, so the walk starts at it.
            if let Some(directory) = directory {
                counted.extend(once(directory).chain(directories.ancestors(directory)));
            }
        }
        for directory in counted {
            *self
                .histograms
                .entry(directory)
                .or_default()
                .entry(observation)
                .or_default() += 1;
        }
    }

    /// The material amplification of every directory the stream observed.
    ///
    /// The median is the nearest-rank median of the directory's histogram,
    /// which is an integer of the sample and needs no interpolation.
    pub(crate) fn finish(self) -> DirectoryAmplification {
        DirectoryAmplification::new(
            self.histograms
                .into_iter()
                .filter_map(|(directory, histogram)| {
                    let commits: u32 = histogram.values().sum();
                    let median = nearest_rank_median_of_counts(&histogram);
                    ChangeAmplification::from_counts(median, commits)
                        .map(|amplification| (directory, amplification))
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContributorId, FileId, HistoryChangeFact, PackageId, SourceRole, SourceTrust};

    /// The four files of the tree every case below counts over: two in one
    /// directory, one beside them, and one at the root.
    fn directories() -> DirectoryTree {
        DirectoryTree::from_file_paths([
            "src/deep/one.js",
            "src/deep/two.js",
            "src/other.js",
            "top.js",
        ])
    }

    fn change(file: usize) -> HistoryChangeFact {
        HistoryChangeFact::new(
            FileId::from_index(file),
            PackageId::from_index(0),
            Some(1),
            Some(0),
        )
    }

    fn commit(files: &[usize]) -> HistoryCommitFact {
        HistoryCommitFact::new(
            ContributorId::from_index(0),
            files.iter().copied().map(change).collect(),
        )
    }

    /// The accumulator these commits leave behind, over one tree.
    fn accumulated(
        commits: &[HistoryCommitFact],
        directories: &DirectoryTree,
    ) -> ChangeAmplificationAccumulator {
        let mut accumulator = ChangeAmplificationAccumulator::default();
        for commit in commits {
            accumulator.accept(commit, directories);
        }
        accumulator
    }

    /// The raw histograms, which the materiality floors are not applied to.
    fn histograms(
        commits: &[HistoryCommitFact],
        directories: &DirectoryTree,
    ) -> BTreeMap<DirectoryId, BTreeMap<u32, u32>> {
        accumulated(commits, directories).histograms
    }

    /// The material facts the commits leave behind.
    fn finished(
        commits: &[HistoryCommitFact],
        directories: &DirectoryTree,
    ) -> DirectoryAmplification {
        accumulated(commits, directories).finish()
    }

    /// The directory of a file of the fixture tree.
    fn directory(directories: &DirectoryTree, file: usize) -> DirectoryId {
        directories
            .directory_of(FileId::from_index(file))
            .expect("the tree holds every file it was built from")
    }

    #[test]
    fn a_commit_counts_once_per_directory_and_once_per_ancestor() {
        let directories = directories();
        let counted = histograms(&[commit(&[0, 1, 2])], &directories);
        let deep = directory(&directories, 0);
        let source = directory(&directories, 2);
        assert_eq!(
            counted,
            BTreeMap::from([
                (deep, BTreeMap::from([(3, 1)])),
                (source, BTreeMap::from([(3, 1)])),
                (DirectoryTree::ROOT, BTreeMap::from([(3, 1)])),
            ]),
            "two files of one directory are one observation there, not two, \
             and the observation is the whole commit rather than the local half"
        );
    }

    #[test]
    fn a_commit_holding_no_change_graph_file_is_no_observation() {
        let directories = directories();
        let tests_only = HistoryCommitFact::new(
            ContributorId::from_index(0),
            vec![change(0).with_source_evidence(SourceRole::Test, SourceTrust::Trusted)],
        );
        assert!(histograms(&[tests_only], &directories).is_empty());
        assert!(histograms(&[commit(&[])], &directories).is_empty());
    }

    #[test]
    fn an_observation_is_clamped_at_the_maximum_file_count() {
        let paths: Vec<String> = (0..=AMPLIFICATION_MAX_FILES as usize)
            .map(|file| format!("src/unit{file:04}.js"))
            .collect();
        let directories = DirectoryTree::from_file_paths(&paths);
        let at_limit: Vec<usize> = (0..AMPLIFICATION_MAX_FILES as usize).collect();
        let over_limit: Vec<usize> = (0..=AMPLIFICATION_MAX_FILES as usize).collect();
        let counted = histograms(&[commit(&at_limit), commit(&over_limit)], &directories);
        assert_eq!(
            counted[&DirectoryTree::ROOT],
            BTreeMap::from([(AMPLIFICATION_MAX_FILES, 2)]),
            "one file more than the clamp is the same key rather than a new one"
        );
    }

    /// The bulk-commit guard bounds pair accumulation alone. A sweeping commit
    /// is exactly the change this fact is about, so it is observed in full.
    #[test]
    fn a_commit_the_pair_guard_holds_back_still_contributes_one_observation() {
        let files = crate::BULK_COMMIT_FILES + 5;
        let paths: Vec<String> = (0..files)
            .map(|file| format!("src/dir{file:03}/unit.js"))
            .collect();
        let directories = DirectoryTree::from_file_paths(&paths);
        let counted = histograms(&[commit(&(0..files).collect::<Vec<_>>())], &directories);
        assert_eq!(
            counted[&DirectoryTree::ROOT],
            BTreeMap::from([(files as u32, 1)]),
            "the sweeping commit is one observation of the files it changed"
        );
        assert_eq!(
            counted.len(),
            files + 2,
            "every touched directory, their one parent, and the root saw it once"
        );
    }

    #[test]
    fn a_directory_states_the_nearest_rank_median_of_its_own_observations() {
        let directories = directories();
        // Ten commits reaching the deep directory: two of two files, five of
        // three, and three of four, whose nearest-rank median is 3. Two more
        // commits touch the root file alone, which only the root sees.
        let mut commits = vec![commit(&[0, 1]); 2];
        commits.extend(vec![commit(&[0, 1, 2]); 5]);
        commits.extend(vec![commit(&[0, 1, 2, 3]); 3]);
        commits.extend(vec![commit(&[3]); 2]);
        let finished = finished(&commits, &directories);
        let deep = finished
            .get(directory(&directories, 0))
            .expect("ten observations at a median of three are material");
        assert_eq!((deep.median(), deep.commits()), (3, 10));
        assert_eq!(deep.sentence(), "A typical change here touches 3 files.");
        let root = finished
            .get(DirectoryTree::ROOT)
            .expect("every commit reaches the root");
        assert_eq!(
            (root.median(), root.commits()),
            (3, 12),
            "the root saw two commits the deep directory never did, and its \
             own median is its own sample"
        );
        // The tree holds three directories and every one of them states its
        // own fact, the middle one from the same ten commits the deepest saw.
        let source = finished
            .get(directory(&directories, 2))
            .expect("an ancestor states its own fact");
        assert_eq!((source.median(), source.commits()), (3, 10));
    }

    #[test]
    fn a_sample_one_commit_short_and_a_median_one_file_short_state_nothing() {
        let directories = directories();
        let nothing = DirectoryAmplification::default();
        assert_eq!(
            finished(
                &vec![commit(&[0, 1, 2]); AMPLIFICATION_MIN_COMMITS as usize - 1],
                &directories
            ),
            nothing,
            "nine commits are an anecdote rather than a typical change"
        );
        assert_ne!(
            finished(
                &vec![commit(&[0, 1, 2]); AMPLIFICATION_MIN_COMMITS as usize],
                &directories
            ),
            nothing,
            "one commit more is material"
        );
        assert_eq!(
            finished(
                &vec![commit(&[0, 2]); AMPLIFICATION_MIN_COMMITS as usize],
                &directories
            ),
            nothing,
            "a typical change of two files is below the median floor"
        );
    }
}
