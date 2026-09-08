//! File change coupling across directories, measured from shared commits.
//!
//! Similarity is shared commits divided by their union (Jaccard similarity).
//! Only trusted primary source enters this population. A commit with more than
//! 25 distinct eligible files contributes neither pairs nor file touch counts
//! here; other history metrics still count it. Files in one directory do not
//! form a pair. Retention requires three shared commits and at least 10%
//! similarity; retained rows are evidence, not rated findings.
//!
//! At most one million pair keys are accumulated. Existing pairs keep counting
//! when new keys are declined. History coverage records both these omissions and
//! bulk commits. Directory distance is retained with the pair for change-leakage
//! rules; the same inventory directory tree must be used throughout accumulation.

#![deny(missing_docs)]

use crate::{DirectoryId, DirectoryTree, FileId, HistoryCommitFact};
use std::collections::{BTreeMap, BTreeSet};

/// The distinct change-graph files a commit may hold before it contributes no
/// pair at all.
///
/// A sweeping rename, a formatting pass, or a dependency bump changes many
/// files at once and says nothing about design, while the pairs it would create
/// grow with the square of its size. Such a commit is counted as a bulk commit
/// and disclosed; every other signal — churn, touches, package change coupling,
/// contributor concentration, hotspots — still counts it in full.
pub const BULK_COMMIT_FILES: usize = 25;

/// The commits a pair must share before it is worth retaining at all.
pub const RETAINED_FILE_PAIR_SHARED_COMMITS: u32 = 3;

/// The share of a pair's union, in permille, its shared commits must reach
/// before it is retained.
///
/// Retention is deliberately far below any detector's bar: it decides which
/// pairs a detector may read, not which pairs are worth a word.
pub const RETAINED_FILE_PAIR_SIMILARITY_PERMILLE: u32 = 100;

/// The pair keys the accumulator may hold before it stops creating new ones.
///
/// Past the limit no new key is created and each declined pair is counted, so
/// the approximation only ever loses pairs the stream had not yet seen while
/// every key already held keeps accumulating.
pub const RETAINED_FILE_PAIR_LIMIT: usize = 1_000_000;

/// One accumulating pair: how many commits it was seen in, and how far apart
/// its two files sit, which is fixed the moment the pair is first seen.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PairCounts {
    shared_commits: u32,
    distance: u32,
}

/// Accumulates the file pairs that change together, one commit at a time.
///
/// The history stream delivers one commit at a time and retains no per-commit
/// file list, so pairs are counted inline. The accumulator keeps its own
/// per-file commit count over exactly the commits pair accumulation ran on, and
/// derives every union from it, so the shared and union operands of a pair
/// describe one population. That count is separate from the file history touch
/// count, which stays over every eligible commit.
#[derive(Debug, Default)]
pub(crate) struct FileChangeCouplingAccumulator {
    touches: BTreeMap<FileId, u32>,
    shared: BTreeMap<(FileId, FileId), PairCounts>,
    bulk_commits: u32,
    declined_pairs: u32,
}

impl FileChangeCouplingAccumulator {
    /// Counts one commit's cross-directory pairs.
    ///
    /// The same directory tree must be passed for every commit of one report:
    /// a file's identity is a position in the tree the report built once, and
    /// two trees would answer two different distances for one pair.
    pub(crate) fn accept(&mut self, commit: &HistoryCommitFact, directories: &DirectoryTree) {
        let files: BTreeSet<FileId> = commit
            .changes()
            .iter()
            .filter(|change| change.enters_change_graph())
            .map(|change| change.file())
            .collect();
        if files.len() > BULK_COMMIT_FILES {
            self.bulk_commits += 1;
            return;
        }
        let files = located(files, directories);
        for (file, _) in &files {
            *self.touches.entry(*file).or_default() += 1;
        }
        for (index, (left, left_directory)) in files.iter().enumerate() {
            for (right, right_directory) in &files[index + 1..] {
                if left_directory == right_directory {
                    continue;
                }
                self.observe(
                    *left,
                    *right,
                    directories.distance(*left_directory, *right_directory),
                );
            }
        }
    }

    /// The commits the bulk-commit guard excluded from pair accumulation.
    pub(crate) const fn bulk_commits(&self) -> u32 {
        self.bulk_commits
    }

    /// The pairs the storage limit declined to create.
    pub(crate) const fn declined_pairs(&self) -> u32 {
        self.declined_pairs
    }

    /// The pairs whose support and similarity clear the retention floors, in
    /// file order.
    pub(crate) fn finish(self) -> Vec<FileChangeCoupling> {
        let Self {
            touches, shared, ..
        } = self;
        shared
            .into_iter()
            .filter_map(|((left, right), counts)| {
                let union = touches[&left] + touches[&right] - counts.shared_commits;
                is_retained(counts.shared_commits, union).then(|| {
                    FileChangeCoupling::new(
                        left,
                        right,
                        counts.shared_commits,
                        union,
                        counts.distance,
                    )
                })
            })
            .collect()
    }

    /// Counts one more shared commit for a cross-directory pair, creating its
    /// key while the storage limit allows one.
    fn observe(&mut self, left: FileId, right: FileId, distance: u32) {
        if let Some(counts) = self.shared.get_mut(&(left, right)) {
            counts.shared_commits += 1;
            return;
        }
        if self.shared.len() >= RETAINED_FILE_PAIR_LIMIT {
            self.declined_pairs += 1;
            return;
        }
        self.shared.insert(
            (left, right),
            PairCounts {
                shared_commits: 1,
                distance,
            },
        );
    }
}

/// The commit's files beside the directory each sits in, in file order.
///
/// A file the tree does not hold is a composition mistake rather than a history
/// fact, so it trips a debug assertion and is dropped where assertions are off:
/// a pair must never carry a distance guessed from a missing directory.
fn located(files: BTreeSet<FileId>, directories: &DirectoryTree) -> Vec<(FileId, DirectoryId)> {
    files
        .into_iter()
        .filter_map(|file| {
            let directory = directories.directory_of(file);
            debug_assert!(
                directory.is_some(),
                "the change graph names files the directory tree holds"
            );
            Some((file, directory?))
        })
        .collect()
}

/// Whether a pair's support and similarity clear both retention floors.
fn is_retained(shared_commits: u32, union_commits: u32) -> bool {
    shared_commits >= RETAINED_FILE_PAIR_SHARED_COMMITS
        && u64::from(shared_commits) * 1_000
            >= u64::from(union_commits) * u64::from(RETAINED_FILE_PAIR_SIMILARITY_PERMILLE)
}

/// Two files that change in the same commits, named lower identity first.
///
/// Every operand is an integer. The distance is the directory distance of the
/// pair, which is at least 1 because a pair inside one directory is never
/// stored. Similarity is derived by a reader, exactly as it is for package
/// change coupling, so no ratio is stored or serialized.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileChangeCoupling {
    left: FileId,
    right: FileId,
    shared_commits: u32,
    union_commits: u32,
    distance: u32,
}

impl FileChangeCoupling {
    /// Creates one retained pair, which names the lower file identity first.
    pub fn new(
        left: FileId,
        right: FileId,
        shared_commits: u32,
        union_commits: u32,
        distance: u32,
    ) -> Self {
        assert!(
            left < right,
            "a file pair names the lower file identity first"
        );
        assert!(
            shared_commits <= union_commits,
            "shared commits are part of the union"
        );
        assert!(distance >= 1, "a stored pair crosses a directory boundary");
        Self {
            left,
            right,
            shared_commits,
            union_commits,
            distance,
        }
    }
    /// The first subject in the retained pair.
    pub const fn left(self) -> FileId {
        self.left
    }
    /// The second subject in the retained pair.
    pub const fn right(self) -> FileId {
        self.right
    }
    /// Distinct commits touching both subjects in this pair's history population.
    pub const fn shared_commits(self) -> u32 {
        self.shared_commits
    }
    /// Distinct commits touching either subject, counting shared commits once.
    pub const fn union_commits(self) -> u32 {
        self.union_commits
    }
    /// The integer directory distance between the two files.
    pub const fn distance(self) -> u32 {
        self.distance
    }
}

crate::table_index::table_index!(
    /// The position of one file change coupling in its report table.
    FileChangeCouplingId
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContributorId, HistoryChangeFact, PackageId, SourceRole, SourceTrust};

    /// A tree over `count` files, each in its own directory under one parent,
    /// so every pair is two directories apart.
    fn siblings(count: usize) -> DirectoryTree {
        let paths: Vec<String> = (0..count)
            .map(|file| format!("src/dir{file:03}/unit{file:03}.js"))
            .collect();
        DirectoryTree::from_file_paths(&paths)
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

    /// Accumulates the commits over one tree and returns the retained pairs.
    fn retained(
        commits: &[HistoryCommitFact],
        directories: &DirectoryTree,
    ) -> Vec<FileChangeCoupling> {
        let mut accumulator = FileChangeCouplingAccumulator::default();
        for commit in commits {
            accumulator.accept(commit, directories);
        }
        accumulator.finish()
    }

    #[test]
    fn a_commit_at_the_bulk_boundary_pairs_and_one_file_more_pairs_nothing() {
        let directories = siblings(BULK_COMMIT_FILES + 1);
        let at_boundary: Vec<usize> = (0..BULK_COMMIT_FILES).collect();
        let over_boundary: Vec<usize> = (0..=BULK_COMMIT_FILES).collect();

        let mut inside = FileChangeCouplingAccumulator::default();
        inside.accept(&commit(&at_boundary), &directories);
        assert_eq!(
            inside.shared.len(),
            BULK_COMMIT_FILES * (BULK_COMMIT_FILES - 1) / 2
        );
        assert_eq!(inside.bulk_commits(), 0);

        let mut outside = FileChangeCouplingAccumulator::default();
        outside.accept(&commit(&over_boundary), &directories);
        assert!(
            outside.shared.is_empty(),
            "a bulk commit contributes no pair"
        );
        assert_eq!(outside.bulk_commits(), 1);
        assert!(
            outside.touches.is_empty(),
            "a bulk commit is outside the population the union describes"
        );
    }

    #[test]
    fn a_pair_is_retained_at_three_shared_commits_and_not_at_two() {
        let directories = siblings(2);
        assert!(retained(&vec![commit(&[0, 1]); 2], &directories).is_empty());
        let three = retained(&vec![commit(&[0, 1]); 3], &directories);
        assert_eq!(
            three,
            [FileChangeCoupling::new(
                FileId::from_index(0),
                FileId::from_index(1),
                3,
                3,
                2
            )]
        );
    }

    #[test]
    fn a_pair_is_retained_at_exactly_one_tenth_of_its_union_and_not_below() {
        let directories = siblings(2);
        // Three shared commits and twenty-seven single-file ones make a union
        // of thirty, of which three is exactly one tenth.
        let alone = |count| (0..count).map(|index| commit(&[index % 2]));
        let mut commits = vec![commit(&[0, 1]); 3];
        commits.extend(alone(27));
        let at_floor = retained(&commits, &directories);
        assert_eq!(at_floor.len(), 1);
        assert_eq!(at_floor[0].union_commits(), 30);

        commits.extend(alone(1));
        assert!(
            retained(&commits, &directories).is_empty(),
            "one commit more of union puts the pair below the similarity floor"
        );
    }

    #[test]
    fn two_files_of_one_directory_never_form_a_pair() {
        let directories = DirectoryTree::from_file_paths(["src/a.js", "src/b.js", "other/c.js"]);
        let commits: Vec<_> = (0..8).map(|_| commit(&[0, 1, 2])).collect();
        let pairs: Vec<_> = retained(&commits, &directories)
            .into_iter()
            .map(|pair| (pair.left().index(), pair.right().index()))
            .collect();
        assert_eq!(
            pairs,
            [(0, 2), (1, 2)],
            "co-change inside one directory is what a directory is for"
        );
    }

    #[test]
    fn a_test_file_and_the_subject_it_exercises_produce_no_pair() {
        let directories = DirectoryTree::from_file_paths(["src/unit.js", "tests/unit.test.js"]);
        let commits: Vec<_> = (0..20)
            .map(|_| {
                HistoryCommitFact::new(
                    ContributorId::from_index(0),
                    vec![
                        change(0),
                        change(1).with_source_evidence(SourceRole::Test, SourceTrust::Trusted),
                    ],
                )
            })
            .collect();
        assert!(
            retained(&commits, &directories).is_empty(),
            "a test changes with its subject by construction, which is good practice"
        );
    }

    #[test]
    fn a_pair_takes_its_distance_from_the_directory_tree() {
        let directories =
            DirectoryTree::from_file_paths(["one/two/three/a.js", "four/five/b.js", "six/c.js"]);
        let commits: Vec<_> = (0..3).map(|_| commit(&[0, 1, 2])).collect();
        let distances: Vec<_> = retained(&commits, &directories)
            .into_iter()
            .map(|pair| (pair.left().index(), pair.right().index(), pair.distance()))
            .collect();
        assert_eq!(distances, [(0, 1, 5), (0, 2, 4), (1, 2, 3)]);
    }

    #[test]
    fn pairs_are_stored_in_file_order_whatever_order_a_commit_lists_them() {
        let directories = siblings(4);
        let listed = |files: [usize; 4]| {
            HistoryCommitFact::new(
                ContributorId::from_index(0),
                files.into_iter().map(change).collect(),
            )
        };
        let forward = vec![listed([0, 1, 2, 3]); 3];
        let reversed = vec![listed([3, 2, 1, 0]); 3];
        let ordered: Vec<_> = retained(&forward, &directories)
            .into_iter()
            .map(|pair| (pair.left().index(), pair.right().index()))
            .collect();
        assert_eq!(
            ordered,
            [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)],
            "a stored pair names the lower file identity first"
        );
        assert_eq!(
            retained(&reversed, &directories),
            retained(&forward, &directories)
        );
    }

    #[test]
    fn the_storage_limit_declines_new_keys_and_keeps_counting_the_keys_it_holds() {
        let directories = siblings(4);
        let mut accumulator = FileChangeCouplingAccumulator::default();
        accumulator.accept(&commit(&[0, 1]), &directories);
        // The map is filled to the limit around the one key the stream created,
        // which no commit can widen and every commit can still deepen.
        accumulator
            .shared
            .extend((0..RETAINED_FILE_PAIR_LIMIT).map(|index| {
                (
                    (FileId::from_index(index + 4), FileId::from_index(index + 5)),
                    PairCounts {
                        shared_commits: 1,
                        distance: 2,
                    },
                )
            }));
        let held = accumulator.shared.len();
        assert!(held > RETAINED_FILE_PAIR_LIMIT);

        accumulator.accept(&commit(&[0, 1]), &directories);
        assert_eq!(accumulator.declined_pairs(), 0, "an existing key is free");
        assert_eq!(
            accumulator.shared[&(FileId::from_index(0), FileId::from_index(1))].shared_commits,
            2
        );

        accumulator.accept(&commit(&[2, 3]), &directories);
        assert_eq!(accumulator.declined_pairs(), 1);
        assert_eq!(accumulator.shared.len(), held, "no key was created");
    }
}
