//! Change amplification: how many files a typical commit touching a scope changes.
//!
//! Smackdebt measures this design symptom with the nearest-rank median of
//! repository-wide eligible file counts, one observation per touched directory
//! and ancestor. For an even sample the lower middle observation is used.
//! Only trusted primary files count. File counts are capped at 1,000; bulk commits
//! excluded from file-pair coupling still contribute here.
//!
//! A result needs at least ten commits and a median of three files. Each scope
//! reads its directory's own sample, never an average of child medians. File
//! scopes have no amplification value. History composition withholds unreliable
//! samples. Amplification is descriptive and does not change the health tier.
//!
//! Terminology: [Ousterhout, change amplification](https://web.stanford.edu/~ouster/cgi-bin/cs190-winter18/lecture.php?topic=complexity).

#![deny(missing_docs)]

use crate::median::nearest_rank_median_of_counts;
use crate::{DirectoryId, DirectoryTree, HistoryCommitFact};
use crate::{Scope, ScopeKind};
use std::collections::{BTreeMap, BTreeSet};
use std::iter::once;

/// The most files one commit may be observed to have touched.
///
/// A repository-wide sweep — a formatting pass, a license header, a generated
/// rewrite — says nothing about what a change costs, and left unclamped it
/// would put one histogram key per distinct sweep size in every directory it
/// touched. The clamp bounds the key space of every histogram to this many
/// keys, which is what keeps the median a walk over counts rather than over
/// commits.
pub const AMPLIFICATION_MAX_FILES: u32 = 1_000;

/// The observations a directory needs before its median is called typical.
pub const AMPLIFICATION_MIN_COMMITS: u32 = 10;

/// The median a directory needs before stating it says anything.
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

/// How many files a typical change to one scope touches.
///
/// The value is the nearest-rank median of the scope's directory histogram, so
/// it is a member of the sample rather than an average of it: a repository
/// whose changes touch three files usually says three, whatever one sweeping
/// commit did. The fact is descriptive — it is never rated, creates no finding,
/// and changes no verdict — and its sentence is copy owned by analysis, so a
/// terminal renderer and a machine consumer print the same bytes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ChangeAmplification {
    median: u32,
    commits: u32,
}

impl ChangeAmplification {
    /// Completes an amplification fact when the sample is both long enough and
    /// wide enough to be worth stating.
    ///
    /// A handful of commits is an anecdote rather than a typical change, and a
    /// median of one or two files is what a directory is for, so each floor
    /// rules out one of those and a scope below either states nothing rather
    /// than stating noise. Whether the history stream was complete enough to
    /// hold a sample at all is decided before a histogram is finished, so it
    /// never reaches here.
    pub const fn from_counts(median: u32, commits: u32) -> Option<Self> {
        if commits < AMPLIFICATION_MIN_COMMITS || median < AMPLIFICATION_MIN_MEDIAN {
            return None;
        }
        Some(Self { median, commits })
    }

    /// The exact sentence every consumer prints for this amplification.
    ///
    /// The median is at least the floor, so the plural is always the correct
    /// form and the sentence needs no singular arm.
    pub fn sentence(self) -> String {
        format!("A typical change here touches {} files.", self.median)
    }

    /// The files a typical change to this scope touches.
    pub const fn median(self) -> u32 {
        self.median
    }

    /// The commits the median was computed from.
    pub const fn commits(self) -> u32 {
        self.commits
    }
}

/// The change amplification of every directory whose value is material.
///
/// A directory whose sample is too short, or whose median is below the floor,
/// has no entry at all rather than a weak one, so a reader of this table never
/// has to know the materiality rule to use it. The table is keyed by directory
/// because that is what the history stream observed; turning it into the answer
/// a scope states is the join below.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DirectoryAmplification(BTreeMap<DirectoryId, ChangeAmplification>);

impl DirectoryAmplification {
    /// The table these material facts make, keyed by directory.
    pub(crate) const fn new(facts: BTreeMap<DirectoryId, ChangeAmplification>) -> Self {
        Self(facts)
    }

    /// What a typical change to this directory touches, when the fact is
    /// material.
    ///
    /// A table stating nothing anywhere is the default one, so a caller asking
    /// whether a stream observed anything compares against that rather than
    /// counting rows.
    pub(crate) fn get(&self, directory: DirectoryId) -> Option<ChangeAmplification> {
        self.0.get(&directory).copied()
    }
}

/// The amplification each scope states, by scope position.
///
/// Every scope maps to exactly one directory, stated rather than inferred:
///
/// - a repository scope reads the root directory, which every tree holds;
/// - a package scope reads its own root directory, which is the path its scope
///   is named by — a package rooted at the repository root therefore reads the
///   root histogram and states the same median the repository states, which is
///   one fact stated at two scopes rather than two numbers;
/// - a directory scope reads itself;
/// - a file scope reads nothing, because a per-file histogram would state
///   sample noise as a fact.
///
/// The join is computed once, while the report is composed, so rendering a
/// scope reads one table position rather than walking a tree.
pub fn scope_amplification(
    scopes: &[Scope],
    directories: &DirectoryTree,
    amplification: &DirectoryAmplification,
) -> Vec<Option<ChangeAmplification>> {
    scopes
        .iter()
        .map(|scope| {
            let directory = match scope.kind() {
                ScopeKind::Repository => Some(DirectoryTree::ROOT),
                ScopeKind::Package | ScopeKind::Directory => {
                    directories.directory_of_path(scope.name())
                }
                ScopeKind::File => None,
            };
            directory.and_then(|directory| amplification.get(directory))
        })
        .collect()
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

    #[test]
    fn an_amplification_is_material_only_above_both_of_its_floors() {
        assert!(
            ChangeAmplification::from_counts(3, AMPLIFICATION_MIN_COMMITS - 1).is_none(),
            "nine commits are too few to call a median typical"
        );
        assert!(
            ChangeAmplification::from_counts(AMPLIFICATION_MIN_MEDIAN - 1, 40).is_none(),
            "a typical change of two files is what a directory is for"
        );
        let exactly =
            ChangeAmplification::from_counts(AMPLIFICATION_MIN_MEDIAN, AMPLIFICATION_MIN_COMMITS)
                .expect("both floors are inclusive");
        assert_eq!(exactly.median(), 3);
        assert_eq!(exactly.commits(), 10);
        assert_eq!(exactly.sentence(), "A typical change here touches 3 files.");
        let wide = ChangeAmplification::from_counts(4, 40).expect("a busy directory");
        assert_eq!(wide.sentence(), "A typical change here touches 4 files.");
    }
}

#[cfg(test)]
mod scope_tests {
    use crate::ScopeId;
    /// Three directories stating three different medians, so a scope reading
    /// the wrong one is a wrong number rather than the same number.
    fn table(directories: &DirectoryTree) -> DirectoryAmplification {
        let directory = |path| {
            directories
                .directory_of_path(path)
                .expect("the tree holds this directory")
        };
        let fact =
            |median| ChangeAmplification::from_counts(median, 12).expect("a material sample");
        DirectoryAmplification::new(BTreeMap::from([
            (DirectoryTree::ROOT, fact(3)),
            (directory("src"), fact(4)),
            (directory("src/deep"), fact(5)),
        ]))
    }

    use super::*;

    #[test]
    fn every_scope_reads_the_one_directory_its_kind_names() {
        let directories = DirectoryTree::from_file_paths(["src/deep/one.js", "src/other.js"]);
        let root = ScopeId::from_index(0);
        let below =
            |index, kind, name| Scope::new(ScopeId::from_index(index), kind, name, Some(root));
        let scopes = [
            Scope::new(root, ScopeKind::Repository, ".", None),
            below(1, ScopeKind::Package, "."),
            below(2, ScopeKind::Package, "src"),
            below(3, ScopeKind::Directory, "src/deep"),
            below(4, ScopeKind::File, "src/deep/one.js"),
            below(5, ScopeKind::Directory, "gone"),
        ];
        let medians: Vec<_> = scope_amplification(&scopes, &directories, &table(&directories))
            .into_iter()
            .map(|amplification| amplification.map(ChangeAmplification::median))
            .collect();
        assert_eq!(
            medians,
            [Some(3), Some(3), Some(4), Some(5), None, None],
            "a package rooted at the repository root states the root's own median, \
             a deeper scope states its own, a file scope states nothing, and a \
             directory the tree never saw states nothing"
        );
    }

    #[test]
    fn a_stream_that_observed_nothing_leaves_every_scope_stating_nothing() {
        let directories = DirectoryTree::from_file_paths(["src/deep/one.js"]);
        let root = Scope::new(ScopeId::from_index(0), ScopeKind::Repository, ".", None);
        let stated = scope_amplification(&[root], &directories, &DirectoryAmplification::default());
        assert_eq!(stated, [None]);
    }
}
