use crate::{ComparisonDirection, FileId, PackageId, SourceRole, SourceTrust};
use std::collections::BTreeSet;

macro_rules! evolution_index {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u32);

        impl $name {
            pub const fn from_index(index: usize) -> Self {
                Self(index as u32)
            }
            pub const fn index(self) -> usize {
                self.0 as usize
            }
            pub const fn get(self) -> u32 {
                self.0
            }
        }
    };
}

evolution_index!(ContributorId);
evolution_index!(EvolutionaryFindingId);
evolution_index!(FileChangeCouplingId);
evolution_index!(EvolutionaryComparisonId);
evolution_index!(HistoryComparisonSuppressionId);
evolution_index!(KnowledgeConcentrationFindingId);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HistoryAvailability {
    Complete,
    Incomplete,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryCoverage {
    availability: HistoryAvailability,
    revision: Option<String>,
    commits: u32,
    eligible_commits: u32,
    mapped_eligible_changes: u32,
    context_changes: u32,
    newest_timestamp: Option<i64>,
    oldest_timestamp: Option<i64>,
    textual_changes: u32,
    uncounted_changes: u32,
    excluded_changes: u32,
    rename_gaps: u32,
    reason: Option<String>,
    window_days: Option<u32>,
    window_excluded_commits: u32,
    bulk_commits: u32,
    declined_pairs: u32,
}

impl HistoryCoverage {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        availability: HistoryAvailability,
        revision: Option<String>,
        commits: u32,
        eligible_commits: u32,
        mapped_eligible_changes: u32,
        context_changes: u32,
        newest_timestamp: Option<i64>,
        oldest_timestamp: Option<i64>,
        textual_changes: u32,
        uncounted_changes: u32,
        excluded_changes: u32,
        rename_gaps: u32,
        reason: Option<String>,
    ) -> Self {
        assert!(
            eligible_commits <= commits,
            "eligible history commits cannot exceed streamed commits"
        );
        assert_eq!(
            u64::from(textual_changes) + u64::from(uncounted_changes),
            u64::from(mapped_eligible_changes) + u64::from(context_changes),
            "mapped history changes must be classified once as textual or uncounted"
        );
        Self {
            availability,
            revision,
            commits,
            eligible_commits,
            mapped_eligible_changes,
            context_changes,
            newest_timestamp,
            oldest_timestamp,
            textual_changes,
            uncounted_changes,
            excluded_changes,
            rename_gaps,
            reason,
            window_days: None,
            window_excluded_commits: 0,
            bulk_commits: 0,
            declined_pairs: 0,
        }
    }
    /// Records the selected window and the boundary rejects the defensive
    /// in-process check excluded. The stream itself is windowed, so streamed
    /// commits and the newest and oldest timestamps describe the windowed set
    /// and the excluded count is normally zero.
    pub fn with_window(mut self, days: Option<u32>, excluded_commits: u32) -> Self {
        assert!(
            excluded_commits <= self.commits,
            "window-excluded commits cannot exceed streamed commits"
        );
        self.window_days = days;
        self.window_excluded_commits = excluded_commits;
        self
    }
    /// Records what pair accumulation approximated: the commits the bulk-commit
    /// guard held back from it and the pairs its storage limit declined.
    ///
    /// Both counters describe pair accumulation alone. A bulk-excluded commit
    /// stays a fully counted commit everywhere else, so it still contributes
    /// churn, touches, package coupling, concentration, and one amplification
    /// observation. They are machine-report facts: no terminal view states
    /// either, at any scope or detail level, because they are processing totals
    /// of the kind human output already omits.
    pub fn with_change_graph(mut self, bulk_commits: u32, declined_pairs: u32) -> Self {
        assert!(
            bulk_commits <= self.commits,
            "bulk commits cannot exceed streamed commits"
        );
        self.bulk_commits = bulk_commits;
        self.declined_pairs = declined_pairs;
        self
    }
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(
            HistoryAvailability::Unavailable,
            None,
            0,
            0,
            0,
            0,
            None,
            None,
            0,
            0,
            0,
            0,
            Some(reason.into()),
        )
    }
    pub const fn availability(&self) -> HistoryAvailability {
        self.availability
    }
    pub fn revision(&self) -> Option<&str> {
        self.revision.as_deref()
    }
    pub const fn commits(&self) -> u32 {
        self.commits
    }
    pub const fn eligible_commits(&self) -> u32 {
        self.eligible_commits
    }
    pub const fn mapped_eligible_changes(&self) -> u32 {
        self.mapped_eligible_changes
    }
    pub const fn context_changes(&self) -> u32 {
        self.context_changes
    }
    pub const fn mapped_changes(&self) -> u32 {
        self.mapped_eligible_changes
            .saturating_add(self.context_changes)
    }
    pub const fn observed_changes(&self) -> u32 {
        self.mapped_changes().saturating_add(self.excluded_changes)
    }
    pub const fn newest_timestamp(&self) -> Option<i64> {
        self.newest_timestamp
    }
    pub const fn oldest_timestamp(&self) -> Option<i64> {
        self.oldest_timestamp
    }
    pub const fn textual_changes(&self) -> u32 {
        self.textual_changes
    }
    pub const fn uncounted_changes(&self) -> u32 {
        self.uncounted_changes
    }
    pub const fn excluded_changes(&self) -> u32 {
        self.excluded_changes
    }
    pub const fn rename_gaps(&self) -> u32 {
        self.rename_gaps
    }
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
    /// The selected history window length in days, when a window is selected.
    pub const fn window_days(&self) -> Option<u32> {
        self.window_days
    }
    /// Streamed commits the defensive boundary check rejected at the window
    /// edge, counted separately from changes excluded for other reasons.
    /// Normally zero, because out-of-window history is not streamed at all.
    pub const fn window_excluded_commits(&self) -> u32 {
        self.window_excluded_commits
    }
    /// Streamed commits the bulk-commit guard excluded from file pair
    /// accumulation, which every other history signal still counts.
    pub const fn bulk_commits(&self) -> u32 {
        self.bulk_commits
    }
    /// File pairs the storage limit declined to create.
    pub const fn declined_pairs(&self) -> u32 {
        self.declined_pairs
    }
}

impl Default for HistoryCoverage {
    fn default() -> Self {
        Self::unavailable("history was not collected")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryChangeFact {
    file: FileId,
    package: PackageId,
    role: SourceRole,
    trust: SourceTrust,
    added_lines: Option<u32>,
    deleted_lines: Option<u32>,
}

impl HistoryChangeFact {
    pub const fn new(
        file: FileId,
        package: PackageId,
        added_lines: Option<u32>,
        deleted_lines: Option<u32>,
    ) -> Self {
        Self {
            file,
            package,
            role: SourceRole::Primary,
            trust: SourceTrust::Trusted,
            added_lines,
            deleted_lines,
        }
    }
    pub const fn with_source_evidence(mut self, role: SourceRole, trust: SourceTrust) -> Self {
        self.role = role;
        self.trust = trust;
        self
    }
    pub const fn file(self) -> FileId {
        self.file
    }
    pub const fn package(self) -> PackageId {
        self.package
    }
    pub const fn role(self) -> SourceRole {
        self.role
    }
    pub const fn trust(self) -> SourceTrust {
        self.trust
    }
    pub const fn affects_findings(self) -> bool {
        self.role.affects_verdict() && matches!(self.trust, SourceTrust::Trusted)
    }
    /// Whether this change may enter the change graph, which is the population
    /// file change coupling and change amplification are accumulated over.
    ///
    /// The name mirrors [`crate::DependencyEdge::enters_verdict_graph`], and so
    /// does the reason for a second, narrower predicate beside
    /// [`Self::affects_findings`]. That one admits test, example, and benchmark
    /// roles, which is right for churn, for package change coupling, and for
    /// concentration: they describe how the whole repository moves. It is wrong
    /// here. A test file and the file it exercises change in the same commit by
    /// construction, so admitting the test role would manufacture the strongest
    /// possible co-change pair for every well-tested file and name good
    /// practice as leakage. Fixture, generated, recovered, and failed source
    /// contributes no pair for the same reason and stays complete in the
    /// machine report as history context.
    pub const fn enters_change_graph(self) -> bool {
        matches!(self.role, SourceRole::Primary) && matches!(self.trust, SourceTrust::Trusted)
    }
    pub const fn added_lines(self) -> Option<u32> {
        self.added_lines
    }
    pub const fn deleted_lines(self) -> Option<u32> {
        self.deleted_lines
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryCommitFact {
    contributor: ContributorId,
    changes: Vec<HistoryChangeFact>,
}

impl HistoryCommitFact {
    pub fn new(contributor: ContributorId, changes: Vec<HistoryChangeFact>) -> Self {
        Self {
            contributor,
            changes,
        }
    }
    pub const fn contributor(&self) -> ContributorId {
        self.contributor
    }
    pub fn changes(&self) -> &[HistoryChangeFact] {
        &self.changes
    }
}

#[derive(Default)]
pub struct EvolutionAccumulator {
    churn: crate::churn::ChurnAccumulator,
    coupling: crate::change_coupling::ChangeCouplingAccumulator,
    concentration: crate::contributor_concentration::ContributorConcentrationAccumulator,
    file_coupling: crate::file_change_coupling::FileChangeCouplingAccumulator,
    amplification: crate::change_amplification::ChangeAmplificationAccumulator,
}

impl EvolutionAccumulator {
    pub fn register_source(
        &mut self,
        file: FileId,
        package: PackageId,
        role: SourceRole,
        trust: SourceTrust,
    ) {
        self.churn.register_source(file, package, role, trust);
    }
    /// Fans one commit out to every signal the one history stream feeds.
    ///
    /// The directory tree is borrowed rather than owned because the report
    /// builds it once, over every file in file order, and composition keeps it:
    /// file pair accumulation and change amplification read directories from it
    /// here, and the scope join reads the same tree later. The same tree must be
    /// passed for every commit of one report.
    pub fn accept(&mut self, commit: HistoryCommitFact, directories: &crate::DirectoryTree) {
        self.churn.accept(&commit);
        self.coupling.accept(&commit);
        self.concentration.accept(&commit);
        self.file_coupling.accept(&commit, directories);
        self.amplification.accept(&commit, directories);
    }
    pub fn finish(
        self,
        coverage: HistoryCoverage,
        file_count: usize,
        package_count: usize,
        containment: &crate::PackageContainment,
        explanation_pairs: &BTreeSet<(PackageId, PackageId)>,
        before_explanation_pairs: Option<&BTreeSet<(PackageId, PackageId)>>,
    ) -> (EvolutionaryReportFacts, crate::DirectoryAmplification) {
        let coverage = coverage.with_change_graph(
            self.file_coupling.bulk_commits(),
            self.file_coupling.declined_pairs(),
        );
        let file_coupling = self.file_coupling.finish();
        // A shallow or unavailable stream holds a sample of the commits it
        // happened to reach, which is not the sample the median is about, so
        // an incomplete stream states no amplification anywhere rather than a
        // median of what it saw.
        let amplification = if coverage.availability() == HistoryAvailability::Complete {
            self.amplification.finish()
        } else {
            crate::DirectoryAmplification::default()
        };
        let (file_history, package_history) = self.churn.finish(file_count, package_count);
        let (coupling, eligible_coupling) = self.coupling.finish(containment);
        let concentration = self.concentration.finish();
        let history_is_sufficient = coverage.availability() == HistoryAvailability::Complete
            && coverage.eligible_commits() > 0
            && coverage.mapped_eligible_changes() > 0;
        let findings = if history_is_sufficient {
            crate::unexplained_coupling(&eligible_coupling, explanation_pairs)
        } else {
            Vec::new()
        };
        let comparison_history_is_trusted = history_is_sufficient && coverage.rename_gaps() == 0;
        let comparisons = if comparison_history_is_trusted {
            before_explanation_pairs.map_or_else(Vec::new, |before| {
                crate::compare_evolution(&eligible_coupling, before, explanation_pairs)
            })
        } else {
            Vec::new()
        };
        let comparison_suppressions = if comparison_history_is_trusted {
            Vec::new()
        } else {
            before_explanation_pairs.map_or_else(Vec::new, |before| {
                eligible_coupling
                    .iter()
                    .filter(|pair| crate::change_coupling::qualifies_for_finding(**pair))
                    .filter(|pair| {
                        pair_is_explained(before, pair.left(), pair.right())
                            != pair_is_explained(explanation_pairs, pair.left(), pair.right())
                    })
                    .enumerate()
                    .map(|(index, pair)| {
                        HistoryComparisonSuppression::new(
                            HistoryComparisonSuppressionId::from_index(index),
                            pair.left(),
                            pair.right(),
                        )
                    })
                    .collect()
            })
        };
        let concentration_findings = if history_is_sufficient {
            crate::knowledge_concentration(&concentration)
        } else {
            Vec::new()
        };
        let facts = EvolutionaryReportFacts::new(
            coverage,
            file_history,
            package_history,
            coupling,
            concentration,
            findings,
            comparisons,
        )
        .with_concentration_findings(concentration_findings)
        .with_comparison_suppressions(comparison_suppressions)
        .with_file_coupling(file_coupling);
        // The amplification travels beside the facts rather than inside them:
        // it is keyed by directory, and only the composition that owns the
        // directory tree can turn it into the answer a scope states.
        (facts, amplification)
    }
}

/// One package-pair comparison withheld because its history evidence cannot be
/// trusted. The pair changed explanation state across the diff, so history is
/// required to decide whether that change added or removed evolutionary debt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HistoryComparisonSuppression {
    id: HistoryComparisonSuppressionId,
    left: PackageId,
    right: PackageId,
}

impl HistoryComparisonSuppression {
    pub const fn new(
        id: HistoryComparisonSuppressionId,
        left: PackageId,
        right: PackageId,
    ) -> Self {
        Self { id, left, right }
    }
    pub const fn id(self) -> HistoryComparisonSuppressionId {
        self.id
    }
    pub const fn left(self) -> PackageId {
        self.left
    }
    pub const fn right(self) -> PackageId {
        self.right
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DirectoryTree, HistoryChangeFact, change_coupling, contributor_concentration};

    /// A tree giving each of the first `count` files its own directory, so a
    /// pair of them is always cross-directory.
    fn directories(count: usize) -> DirectoryTree {
        let paths: Vec<String> = (0..count)
            .map(|file| format!("dir{file}/main.js"))
            .collect();
        DirectoryTree::from_file_paths(&paths)
    }

    fn commit(
        contributor: usize,
        changes: &[(usize, usize, Option<u32>, Option<u32>)],
    ) -> HistoryCommitFact {
        HistoryCommitFact::new(
            ContributorId::from_index(contributor),
            changes
                .iter()
                .map(|(file, package, added, deleted)| {
                    HistoryChangeFact::new(
                        FileId::from_index(*file),
                        PackageId::from_index(*package),
                        *added,
                        *deleted,
                    )
                })
                .collect(),
        )
    }

    fn qualifying_pair_accumulator() -> EvolutionAccumulator {
        let mut accumulator = EvolutionAccumulator::default();
        let directories = directories(2);
        for contributor in 0..3 {
            accumulator.accept(
                commit(
                    contributor,
                    &[(0, 0, Some(1), Some(0)), (1, 1, Some(1), Some(0))],
                ),
                &directories,
            );
        }
        accumulator
    }

    #[test]
    fn a_window_governs_churn_coupling_and_concentration_and_coverage_states_it() {
        let old = 100_000;
        let recent = 150_000;
        let streamed = [
            (old, commit(0, &[(0, 0, None, None), (1, 1, None, None)])),
            (old, commit(0, &[(0, 0, None, None), (1, 1, None, None)])),
            (recent, commit(0, &[(0, 0, None, None)])),
            (recent, commit(1, &[(0, 0, None, None), (1, 1, None, None)])),
        ];
        let window = crate::HistoryWindow::of_days(1, 200_000);
        let windowed: Vec<_> = streamed
            .iter()
            .filter(|(timestamp, _)| window.includes(*timestamp))
            .map(|(_, commit)| commit.clone())
            .collect();
        let every: Vec<_> = streamed.iter().map(|(_, commit)| commit.clone()).collect();
        let excluded = (every.len() - windowed.len()) as u32;

        let (all_files, _) = crate::churn(2, 2, &every);
        let (window_files, _) = crate::churn(2, 2, &windowed);
        assert_eq!((all_files[0].touches(), window_files[0].touches()), (4, 2));

        let containment = crate::PackageContainment::default();
        assert_eq!(change_coupling(&every, &containment)[0].shared_commits(), 3);
        assert!(change_coupling(&windowed, &containment).is_empty());

        let all_concentration = contributor_concentration(&every);
        let window_concentration = contributor_concentration(&windowed);
        assert_eq!(
            (
                all_concentration[0].numerator(),
                all_concentration[0].denominator()
            ),
            (3, 4)
        );
        assert_eq!(
            (
                window_concentration[0].numerator(),
                window_concentration[0].denominator()
            ),
            (1, 2)
        );

        // The stream itself is windowed, so coverage describes the windowed
        // set and carries no boundary rejects.
        let coverage = HistoryCoverage::new(
            HistoryAvailability::Complete,
            None,
            windowed.len() as u32,
            windowed.len() as u32,
            3,
            0,
            None,
            None,
            0,
            3,
            0,
            0,
            None,
        )
        .with_window(window.days(), 0);
        assert_eq!(coverage.window_days(), Some(1));
        assert_eq!(coverage.window_excluded_commits(), 0);
        assert_eq!(coverage.commits(), 2);
        assert_eq!(excluded, 2);
    }

    #[test]
    fn coverage_retains_boundary_rejects_from_the_defensive_window_check() {
        let coverage = HistoryCoverage::new(
            HistoryAvailability::Complete,
            None,
            2,
            1,
            1,
            0,
            None,
            None,
            1,
            0,
            0,
            0,
            None,
        )
        .with_window(Some(90), 1);
        assert_eq!(coverage.window_days(), Some(90));
        assert_eq!(coverage.window_excluded_commits(), 1);
    }

    #[test]
    fn changed_explanations_record_comparisons_withheld_by_incomplete_history() {
        let left = PackageId::from_index(0);
        let right = PackageId::from_index(1);
        let before = BTreeSet::from([(left, right)]);
        let after = BTreeSet::new();
        let (facts, _) = qualifying_pair_accumulator().finish(
            HistoryCoverage::new(
                HistoryAvailability::Incomplete,
                None,
                3,
                3,
                6,
                0,
                None,
                None,
                6,
                0,
                0,
                0,
                Some("history is shallow".to_owned()),
            ),
            2,
            2,
            &crate::PackageContainment::default(),
            &after,
            Some(&before),
        );

        assert!(facts.comparisons().is_empty());
        assert_eq!(
            facts.comparison_suppressions(),
            [HistoryComparisonSuppression::new(
                HistoryComparisonSuppressionId::from_index(0),
                left,
                right,
            )]
        );
    }

    #[test]
    fn unchanged_explanations_need_no_history_comparison_warning() {
        let pair = (PackageId::from_index(0), PackageId::from_index(1));
        let explanations = BTreeSet::from([pair]);
        let (facts, _) = EvolutionAccumulator::default().finish(
            HistoryCoverage::default(),
            0,
            2,
            &crate::PackageContainment::default(),
            &explanations,
            Some(&explanations),
        );

        assert!(facts.comparison_suppressions().is_empty());
    }

    #[test]
    fn changed_explanations_without_qualifying_coupling_need_no_warning() {
        let pair = (PackageId::from_index(0), PackageId::from_index(1));
        let (facts, _) = EvolutionAccumulator::default().finish(
            HistoryCoverage::default(),
            0,
            2,
            &crate::PackageContainment::default(),
            &BTreeSet::new(),
            Some(&BTreeSet::from([pair])),
        );

        assert!(facts.comparison_suppressions().is_empty());
    }

    #[test]
    fn rename_gaps_withhold_changed_explanation_comparisons() {
        let pair = (PackageId::from_index(0), PackageId::from_index(1));
        let (facts, _) = qualifying_pair_accumulator().finish(
            HistoryCoverage::new(
                HistoryAvailability::Complete,
                None,
                3,
                3,
                6,
                0,
                None,
                None,
                6,
                0,
                0,
                1,
                None,
            ),
            2,
            2,
            &crate::PackageContainment::default(),
            &BTreeSet::new(),
            Some(&BTreeSet::from([pair])),
        );

        assert!(facts.comparisons().is_empty());
        assert_eq!(facts.comparison_suppressions().len(), 1);
    }

    /// The change graph is narrower than the population evolutionary findings
    /// read, and the whole of the difference is the role and trust matrix.
    #[test]
    fn only_primary_trusted_source_enters_the_change_graph() {
        let change = HistoryChangeFact::new(
            FileId::from_index(0),
            PackageId::from_index(0),
            Some(1),
            Some(0),
        );
        assert!(change.enters_change_graph());
        assert!(change.affects_findings());
        for (role, trust) in [
            (SourceRole::Test, SourceTrust::Trusted),
            (SourceRole::Example, SourceTrust::Trusted),
            (SourceRole::Benchmark, SourceTrust::Trusted),
            (SourceRole::Fixture, SourceTrust::Trusted),
            (SourceRole::Generated, SourceTrust::Trusted),
            (SourceRole::Primary, SourceTrust::Advisory),
            (SourceRole::Primary, SourceTrust::Failed),
        ] {
            let contextual = change.with_source_evidence(role, trust);
            assert!(
                !contextual.enters_change_graph(),
                "{role:?} {trust:?} contributes no pair and no amplification"
            );
        }
        // The three roles churn and package coupling still count are exactly
        // the ones the two predicates disagree about.
        for role in [SourceRole::Test, SourceRole::Example, SourceRole::Benchmark] {
            let evidence = change.with_source_evidence(role, SourceTrust::Trusted);
            assert!(evidence.affects_findings());
            assert!(!evidence.enters_change_graph());
        }
    }

    /// The bulk-commit guard belongs to pair accumulation alone: the commit it
    /// holds back still moves churn, touches, and package coupling, and only
    /// the pair table and the population its unions describe skip it.
    #[test]
    fn a_bulk_commit_is_counted_everywhere_but_in_the_pair_table() {
        let files = crate::BULK_COMMIT_FILES + 1;
        let directories = directories(files);
        let mut accumulator = EvolutionAccumulator::default();
        for contributor in 0..3 {
            accumulator.accept(
                commit(
                    contributor,
                    &[(0, 0, Some(1), Some(0)), (1, 1, Some(1), Some(0))],
                ),
                &directories,
            );
        }
        let sweep: Vec<_> = (0..files)
            .map(|file| (file, file % 2, Some(1), Some(0)))
            .collect();
        accumulator.accept(commit(3, &sweep), &directories);

        let changes = (6 + files) as u32;
        let (report, _) = accumulator.finish(
            HistoryCoverage::new(
                HistoryAvailability::Complete,
                None,
                4,
                4,
                changes,
                0,
                None,
                None,
                changes,
                0,
                0,
                0,
                None,
            ),
            files,
            2,
            &crate::PackageContainment::default(),
            &BTreeSet::new(),
            None,
        );

        assert_eq!(
            (
                report.coverage.bulk_commits(),
                report.coverage.declined_pairs()
            ),
            (1, 0)
        );
        assert_eq!(report.file_history[0].touches(), 4);
        assert_eq!(report.file_history[files - 1].touches(), 1);
        assert_eq!(report.package_history[0].touches(), 4);
        assert_eq!(
            (
                report.coupling[0].shared_commits(),
                report.coupling[0].union_commits()
            ),
            (4, 4)
        );
        assert_eq!(
            report.file_coupling(),
            [FileChangeCoupling::new(
                FileId::from_index(0),
                FileId::from_index(1),
                3,
                3,
                2
            )],
            "the sweep is outside both the shared count and the union it is compared with"
        );
    }

    /// Change amplification rides the same stream as the pairs and is bounded
    /// by neither the pair guard nor the pair floors: the sweeping commit the
    /// guard holds back is the tenth observation that makes the fact material
    /// at all. An incomplete stream states nothing anywhere.
    #[test]
    fn amplification_counts_the_commit_the_pair_guard_holds_back() {
        let files = crate::BULK_COMMIT_FILES + 1;
        let directories = directories(files);
        let sweep: Vec<_> = (0..files)
            .map(|file| (file, file % 2, Some(1), Some(0)))
            .collect();
        let accumulated = || {
            let mut accumulator = EvolutionAccumulator::default();
            for contributor in 0..9 {
                accumulator.accept(
                    commit(
                        contributor,
                        &[
                            (0, 0, Some(1), Some(0)),
                            (1, 1, Some(1), Some(0)),
                            (2, 0, Some(1), Some(0)),
                        ],
                    ),
                    &directories,
                );
            }
            accumulator.accept(commit(9, &sweep), &directories);
            accumulator
        };
        let coverage = |availability| {
            HistoryCoverage::new(
                availability,
                None,
                10,
                10,
                10,
                0,
                None,
                None,
                10,
                0,
                0,
                0,
                None,
            )
        };
        let finish = |availability| {
            accumulated()
                .finish(
                    coverage(availability),
                    files,
                    2,
                    &crate::PackageContainment::default(),
                    &BTreeSet::new(),
                    None,
                )
                .1
        };

        let complete = finish(HistoryAvailability::Complete);
        let root = complete
            .get(crate::DirectoryTree::ROOT)
            .expect("nine ordinary commits and one sweep are ten observations");
        assert_eq!(
            (root.median(), root.commits()),
            (3, 10),
            "the guard bounds the pair table alone; without the sweep the sample \
             is one commit short of material"
        );
        assert_eq!(
            finish(HistoryAvailability::Incomplete),
            crate::DirectoryAmplification::default(),
            "a stream that saw part of the history has no typical change to state"
        );
    }

    #[test]
    fn churn_deduplicates_package_touches_and_sums_lines() {
        let commits = vec![
            commit(0, &[(0, 0, Some(3), Some(1)), (1, 0, Some(2), Some(4))]),
            commit(1, &[(0, 0, None, None)]),
        ];
        let (files, packages) = crate::churn(2, 1, &commits);
        assert_eq!(
            (
                files[0].touches(),
                files[0].added_lines(),
                files[0].deleted_lines(),
                files[0].uncounted_changes()
            ),
            (2, 3, 1, 1)
        );
        assert_eq!(
            (
                packages[0].touches(),
                packages[0].added_lines(),
                packages[0].deleted_lines(),
                packages[0].uncounted_changes()
            ),
            (2, 5, 5, 1)
        );
    }

    #[test]
    fn churn_counts_duplicate_file_facts_as_one_commit_touch() {
        let commits = vec![commit(
            0,
            &[(0, 0, Some(3), Some(1)), (0, 0, Some(2), Some(4))],
        )];
        let (files, packages) = crate::churn(1, 1, &commits);
        assert_eq!(files[0].touches(), 1);
        assert_eq!(files[0].added_lines(), 5);
        assert_eq!(files[0].deleted_lines(), 5);
        assert_eq!(packages[0].touches(), 1);
        assert_eq!(packages[0].added_lines(), 5);
        assert_eq!(packages[0].deleted_lines(), 5);
    }

    #[test]
    fn coupling_counts_a_package_pair_once_per_commit() {
        let commits = vec![
            commit(
                0,
                &[
                    (0, 0, Some(1), Some(0)),
                    (1, 0, Some(1), Some(0)),
                    (2, 1, Some(1), Some(0)),
                ],
            ),
            commit(1, &[(0, 0, Some(1), Some(0)), (2, 1, Some(1), Some(0))]),
            commit(1, &[(0, 0, Some(1), Some(0))]),
            commit(2, &[(2, 1, Some(1), Some(0))]),
        ];
        let values = change_coupling(&commits, &crate::PackageContainment::default());
        assert_eq!(
            values,
            vec![
                ChangeCoupling::new(PackageId::from_index(0), PackageId::from_index(1), 2, 4)
                    .with_evidence(
                        SourceRole::Primary,
                        SourceTrust::Trusted,
                        SourceRole::Primary,
                        SourceTrust::Trusted,
                    )
            ]
        );
        assert_eq!(values[0].similarity(), 0.5);
    }

    #[test]
    fn concentration_preserves_only_operands() {
        let mut commits = Vec::new();
        for contributor in [0, 0, 0, 1, 1, 2] {
            commits.push(commit(contributor, &[(0, 0, Some(1), Some(0))]));
        }
        let values = contributor_concentration(&commits);
        assert_eq!(
            values,
            vec![ContributorConcentration::new(
                PackageId::from_index(0),
                3,
                3,
                6
            )]
        );
        assert_eq!(values[0].ratio(), 0.5);
    }

    #[test]
    fn an_explaining_pair_suppresses_finding_and_changes_diff_context() {
        let pair = ChangeCoupling::new(PackageId::from_index(0), PackageId::from_index(1), 3, 3);
        let explained: BTreeSet<_> = [(PackageId::from_index(0), PackageId::from_index(1))].into();
        assert_eq!(
            crate::unexplained_coupling(&[pair], &BTreeSet::new()).len(),
            1
        );
        assert!(crate::unexplained_coupling(&[pair], &explained).is_empty());
        let reversed: BTreeSet<_> = [(PackageId::from_index(1), PackageId::from_index(0))].into();
        assert!(crate::unexplained_coupling(&[pair], &reversed).is_empty());
        let comparisons = crate::compare_evolution(&[pair], &BTreeSet::new(), &explained);
        assert_eq!(
            comparisons[0].kind(),
            EvolutionaryComparisonKind::FindingRemoved
        );
        assert_eq!(comparisons[0].direction(), ComparisonDirection::Better);
    }

    #[test]
    fn coupling_finding_requires_both_thresholds() {
        let package_a = PackageId::from_index(0);
        let package_b = PackageId::from_index(1);
        let at_boundary = ChangeCoupling::new(package_a, package_b, 3, 15);
        let too_few_shared = ChangeCoupling::new(package_a, package_b, 2, 10);
        let below_similarity = ChangeCoupling::new(package_a, package_b, 3, 16);

        assert_eq!(
            crate::unexplained_coupling(&[at_boundary], &BTreeSet::new()).len(),
            1
        );
        assert!(crate::unexplained_coupling(&[too_few_shared], &BTreeSet::new()).is_empty());
        assert!(crate::unexplained_coupling(&[below_similarity], &BTreeSet::new()).is_empty());
    }

    #[test]
    fn source_role_and_trust_remain_explicit_and_context_cannot_create_findings() {
        let contextual = HistoryChangeFact::new(
            FileId::from_index(0),
            PackageId::from_index(0),
            Some(1),
            Some(0),
        )
        .with_source_evidence(SourceRole::Fixture, SourceTrust::Trusted);
        assert_eq!(contextual.role(), SourceRole::Fixture);
        assert_eq!(contextual.trust(), SourceTrust::Trusted);
        assert!(!contextual.affects_findings());

        let mut accumulator = EvolutionAccumulator::default();
        for contributor in 0..3 {
            accumulator.accept(
                HistoryCommitFact::new(
                    ContributorId::from_index(contributor),
                    vec![
                        contextual,
                        HistoryChangeFact::new(
                            FileId::from_index(1),
                            PackageId::from_index(1),
                            Some(1),
                            Some(0),
                        ),
                    ],
                ),
                &directories(2),
            );
        }
        let (report, _) = accumulator.finish(
            HistoryCoverage::new(
                HistoryAvailability::Complete,
                None,
                3,
                3,
                3,
                3,
                None,
                None,
                6,
                0,
                0,
                0,
                None,
            ),
            2,
            2,
            &crate::PackageContainment::default(),
            &BTreeSet::new(),
            None,
        );
        assert_eq!(report.coupling.len(), 1);
        assert_eq!(report.coupling[0].shared_commits(), 3);
        assert!(report.findings().is_empty());
    }

    #[test]
    fn incomplete_history_cannot_create_findings_or_diff_verdicts() {
        let mut accumulator = EvolutionAccumulator::default();
        for contributor in 0..3 {
            accumulator.accept(
                commit(
                    contributor,
                    &[(0, 0, Some(1), Some(0)), (1, 1, Some(1), Some(0))],
                ),
                &directories(2),
            );
        }
        let (report, _) = accumulator.finish(
            HistoryCoverage::new(
                HistoryAvailability::Incomplete,
                None,
                3,
                3,
                6,
                0,
                None,
                None,
                6,
                0,
                0,
                0,
                Some("repository history is shallow".to_owned()),
            ),
            2,
            2,
            &crate::PackageContainment::default(),
            &BTreeSet::new(),
            Some(&[(PackageId::from_index(0), PackageId::from_index(1))].into()),
        );
        assert_eq!(report.coupling.len(), 1);
        assert!(report.findings().is_empty());
        assert!(report.comparisons().is_empty());
    }

    #[test]
    fn complete_history_without_eligible_mapping_cannot_create_findings() {
        let mut accumulator = EvolutionAccumulator::default();
        for contributor in 0..3 {
            accumulator.accept(
                HistoryCommitFact::new(
                    ContributorId::from_index(contributor),
                    vec![
                        HistoryChangeFact::new(
                            FileId::from_index(0),
                            PackageId::from_index(0),
                            Some(1),
                            Some(0),
                        )
                        .with_source_evidence(SourceRole::Generated, SourceTrust::Trusted),
                        HistoryChangeFact::new(
                            FileId::from_index(1),
                            PackageId::from_index(1),
                            Some(1),
                            Some(0),
                        )
                        .with_source_evidence(SourceRole::Generated, SourceTrust::Trusted),
                    ],
                ),
                &directories(2),
            );
        }
        let (report, _) = accumulator.finish(
            HistoryCoverage::new(
                HistoryAvailability::Complete,
                None,
                3,
                0,
                0,
                6,
                None,
                None,
                6,
                0,
                0,
                0,
                None,
            ),
            2,
            2,
            &crate::PackageContainment::default(),
            &BTreeSet::new(),
            None,
        );

        assert_eq!(report.coupling.len(), 1);
        assert!(report.findings().is_empty());
        assert!(report.comparisons().is_empty());
    }

    #[test]
    fn contextual_history_changes_descriptive_operands_not_finding_operands() {
        let mut accumulator = EvolutionAccumulator::default();
        for contributor in 0..3 {
            accumulator.accept(
                commit(
                    contributor,
                    &[(0, 0, Some(1), Some(0)), (1, 1, Some(1), Some(0))],
                ),
                &directories(3),
            );
        }
        for contributor in 3..5 {
            accumulator.accept(
                HistoryCommitFact::new(
                    ContributorId::from_index(contributor),
                    vec![
                        HistoryChangeFact::new(
                            FileId::from_index(2),
                            PackageId::from_index(0),
                            Some(1),
                            Some(0),
                        )
                        .with_source_evidence(SourceRole::Generated, SourceTrust::Trusted),
                    ],
                ),
                &directories(3),
            );
        }
        let (report, _) = accumulator.finish(
            HistoryCoverage::new(
                HistoryAvailability::Complete,
                None,
                5,
                3,
                6,
                2,
                None,
                None,
                8,
                0,
                0,
                0,
                None,
            ),
            3,
            2,
            &crate::PackageContainment::default(),
            &BTreeSet::new(),
            None,
        );

        assert_eq!(report.coupling[0].shared_commits(), 3);
        assert_eq!(report.coupling[0].union_commits(), 5);
        assert_eq!(report.findings()[0].coupling().shared_commits(), 3);
        assert_eq!(report.findings()[0].coupling().union_commits(), 3);
    }

    #[test]
    fn one_package_pair_keeps_one_row_across_role_and_trust_variants() {
        let primary = |file, package| {
            HistoryChangeFact::new(
                FileId::from_index(file),
                PackageId::from_index(package),
                None,
                None,
            )
        };
        let test_role = |file, package| {
            primary(file, package).with_source_evidence(SourceRole::Test, SourceTrust::Trusted)
        };
        let commits = vec![
            HistoryCommitFact::new(
                ContributorId::from_index(0),
                vec![primary(0, 0), test_role(1, 0), primary(2, 1)],
            ),
            HistoryCommitFact::new(
                ContributorId::from_index(0),
                vec![test_role(1, 0), test_role(3, 1)],
            ),
        ];
        let values = change_coupling(&commits, &crate::PackageContainment::default());
        assert_eq!(values.len(), 1);
        assert_eq!(
            (values[0].shared_commits(), values[0].union_commits()),
            (2, 2)
        );
        let evidence = values[0].evidence().unwrap();
        assert_eq!(
            (evidence.left_role(), evidence.right_role()),
            (SourceRole::Primary, SourceRole::Primary)
        );
    }

    #[test]
    fn a_scope_and_its_own_descendant_are_not_a_coupling_pair() {
        let containment = crate::PackageContainment::from_paths(&[
            ".".to_owned(),
            "crates/project".to_owned(),
            "other".to_owned(),
        ]);
        assert!(containment.is_nested(PackageId::from_index(0), PackageId::from_index(1)));
        assert!(!containment.is_nested(PackageId::from_index(1), PackageId::from_index(2)));
        let commits = vec![
            commit(
                0,
                &[(0, 0, None, None), (1, 1, None, None), (2, 2, None, None)],
            ),
            commit(
                1,
                &[(0, 0, None, None), (1, 1, None, None), (2, 2, None, None)],
            ),
        ];
        let values = change_coupling(&commits, &containment);
        let pairs: Vec<_> = values
            .iter()
            .map(|pair| (pair.left().index(), pair.right().index()))
            .collect();
        assert_eq!(pairs, [(1, 2)]);
    }

    #[test]
    fn dense_commits_store_each_observed_pair_once() {
        let changes = (0..50)
            .flat_map(|package| {
                [
                    (package * 2, package, Some(1), Some(0)),
                    (package * 2 + 1, package, Some(1), Some(0)),
                ]
            })
            .collect::<Vec<_>>();
        let commits = vec![commit(0, &changes), commit(1, &changes)];
        let values = change_coupling(&commits, &crate::PackageContainment::default());
        assert_eq!(values.len(), 50 * 49 / 2);
        assert!(
            values
                .iter()
                .all(|pair| pair.shared_commits() == 2 && pair.union_commits() == 2)
        );
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileHistory {
    file: FileId,
    role: SourceRole,
    trust: SourceTrust,
    touches: u32,
    added_lines: u64,
    deleted_lines: u64,
    uncounted_changes: u32,
}

impl FileHistory {
    pub const fn new(
        file: FileId,
        touches: u32,
        added_lines: u64,
        deleted_lines: u64,
        uncounted_changes: u32,
    ) -> Self {
        Self {
            file,
            role: SourceRole::Primary,
            trust: SourceTrust::Trusted,
            touches,
            added_lines,
            deleted_lines,
            uncounted_changes,
        }
    }
    pub const fn with_evidence(mut self, role: SourceRole, trust: SourceTrust) -> Self {
        self.role = role;
        self.trust = trust;
        self
    }
    pub const fn file(self) -> FileId {
        self.file
    }
    pub const fn role(self) -> SourceRole {
        self.role
    }
    pub const fn trust(self) -> SourceTrust {
        self.trust
    }
    pub const fn affects_findings(self) -> bool {
        self.role.affects_verdict() && matches!(self.trust, SourceTrust::Trusted)
    }
    pub const fn touches(self) -> u32 {
        self.touches
    }
    pub const fn added_lines(self) -> u64 {
        self.added_lines
    }
    pub const fn deleted_lines(self) -> u64 {
        self.deleted_lines
    }
    pub const fn uncounted_changes(self) -> u32 {
        self.uncounted_changes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageHistory {
    package: PackageId,
    role: SourceRole,
    trust: SourceTrust,
    touches: u32,
    added_lines: u64,
    deleted_lines: u64,
    uncounted_changes: u32,
}

impl PackageHistory {
    pub const fn new(
        package: PackageId,
        touches: u32,
        added_lines: u64,
        deleted_lines: u64,
        uncounted_changes: u32,
    ) -> Self {
        Self {
            package,
            role: SourceRole::Primary,
            trust: SourceTrust::Trusted,
            touches,
            added_lines,
            deleted_lines,
            uncounted_changes,
        }
    }
    pub const fn with_evidence(mut self, role: SourceRole, trust: SourceTrust) -> Self {
        self.role = role;
        self.trust = trust;
        self
    }
    pub const fn package(self) -> PackageId {
        self.package
    }
    pub const fn role(self) -> SourceRole {
        self.role
    }
    pub const fn trust(self) -> SourceTrust {
        self.trust
    }
    pub const fn affects_findings(self) -> bool {
        self.role.affects_verdict() && matches!(self.trust, SourceTrust::Trusted)
    }
    pub const fn touches(self) -> u32 {
        self.touches
    }
    pub const fn added_lines(self) -> u64 {
        self.added_lines
    }
    pub const fn deleted_lines(self) -> u64 {
        self.deleted_lines
    }
    pub const fn uncounted_changes(self) -> u32 {
        self.uncounted_changes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChangeCoupling {
    left: PackageId,
    right: PackageId,
    shared_commits: u32,
    union_commits: u32,
    evidence: Option<CouplingEvidence>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CouplingEvidence {
    left_role: SourceRole,
    left_trust: SourceTrust,
    right_role: SourceRole,
    right_trust: SourceTrust,
}

impl ChangeCoupling {
    pub const fn new(
        left: PackageId,
        right: PackageId,
        shared_commits: u32,
        union_commits: u32,
    ) -> Self {
        Self {
            left,
            right,
            shared_commits,
            union_commits,
            evidence: None,
        }
    }
    pub const fn with_evidence(
        mut self,
        left_role: SourceRole,
        left_trust: SourceTrust,
        right_role: SourceRole,
        right_trust: SourceTrust,
    ) -> Self {
        self.evidence = Some(CouplingEvidence {
            left_role,
            left_trust,
            right_role,
            right_trust,
        });
        self
    }
    pub const fn left(self) -> PackageId {
        self.left
    }
    pub const fn right(self) -> PackageId {
        self.right
    }
    pub const fn shared_commits(self) -> u32 {
        self.shared_commits
    }
    pub const fn union_commits(self) -> u32 {
        self.union_commits
    }
    pub fn similarity(self) -> f64 {
        f64::from(self.shared_commits) / f64::from(self.union_commits)
    }
    pub const fn evidence(self) -> Option<CouplingEvidence> {
        self.evidence
    }
}

impl CouplingEvidence {
    pub const fn left_role(self) -> SourceRole {
        self.left_role
    }
    pub const fn left_trust(self) -> SourceTrust {
        self.left_trust
    }
    pub const fn right_role(self) -> SourceRole {
        self.right_role
    }
    pub const fn right_trust(self) -> SourceTrust {
        self.right_trust
    }
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
    pub const fn left(self) -> FileId {
        self.left
    }
    pub const fn right(self) -> FileId {
        self.right
    }
    pub const fn shared_commits(self) -> u32 {
        self.shared_commits
    }
    pub const fn union_commits(self) -> u32 {
        self.union_commits
    }
    /// The integer directory distance between the two files.
    pub const fn distance(self) -> u32 {
        self.distance
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContributorConcentration {
    package: PackageId,
    role: SourceRole,
    trust: SourceTrust,
    contributor_count: u32,
    numerator: u32,
    denominator: u32,
}

impl ContributorConcentration {
    pub const fn new(
        package: PackageId,
        contributor_count: u32,
        numerator: u32,
        denominator: u32,
    ) -> Self {
        Self {
            package,
            role: SourceRole::Primary,
            trust: SourceTrust::Trusted,
            contributor_count,
            numerator,
            denominator,
        }
    }
    pub const fn with_evidence(mut self, role: SourceRole, trust: SourceTrust) -> Self {
        self.role = role;
        self.trust = trust;
        self
    }
    pub const fn package(self) -> PackageId {
        self.package
    }
    pub const fn role(self) -> SourceRole {
        self.role
    }
    pub const fn trust(self) -> SourceTrust {
        self.trust
    }
    pub const fn affects_findings(self) -> bool {
        self.role.affects_verdict() && matches!(self.trust, SourceTrust::Trusted)
    }
    pub const fn contributor_count(self) -> u32 {
        self.contributor_count
    }
    pub const fn numerator(self) -> u32 {
        self.numerator
    }
    pub const fn denominator(self) -> u32 {
        self.denominator
    }
    pub fn ratio(self) -> f64 {
        f64::from(self.numerator) / f64::from(self.denominator)
    }
}

/// What an evolutionary finding observed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvolutionaryFindingKind {
    UnexplainedCoupling,
    KnowledgeConcentration,
}

/// One Watch observation of packages that change together without a code
/// dependency.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvolutionaryFinding {
    id: EvolutionaryFindingId,
    coupling: ChangeCoupling,
}

impl EvolutionaryFinding {
    pub const fn new(id: EvolutionaryFindingId, coupling: ChangeCoupling) -> Self {
        Self { id, coupling }
    }
    pub const fn id(self) -> EvolutionaryFindingId {
        self.id
    }
    pub const fn kind(self) -> EvolutionaryFindingKind {
        EvolutionaryFindingKind::UnexplainedCoupling
    }
    pub const fn rating(self) -> crate::Rating {
        crate::Rating::Watch
    }
    pub const fn coupling(self) -> ChangeCoupling {
        self.coupling
    }
}

/// One Watch observation of a package whose knowledge sits with one
/// contributor.
///
/// The finding carries counts alone: no name, address, raw author field, or
/// internal contributor identifier. It owns its table and its own index type,
/// so a row can never be mistaken for a coupling finding position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KnowledgeConcentrationFinding {
    id: KnowledgeConcentrationFindingId,
    concentration: ContributorConcentration,
}

impl KnowledgeConcentrationFinding {
    pub const fn new(
        id: KnowledgeConcentrationFindingId,
        concentration: ContributorConcentration,
    ) -> Self {
        Self { id, concentration }
    }
    pub const fn id(self) -> KnowledgeConcentrationFindingId {
        self.id
    }
    pub const fn kind(self) -> EvolutionaryFindingKind {
        EvolutionaryFindingKind::KnowledgeConcentration
    }
    pub const fn rating(self) -> crate::Rating {
        crate::Rating::Watch
    }
    pub const fn concentration(self) -> ContributorConcentration {
        self.concentration
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvolutionaryComparisonKind {
    FindingIntroduced,
    FindingRemoved,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvolutionaryComparison {
    id: EvolutionaryComparisonId,
    kind: EvolutionaryComparisonKind,
    direction: ComparisonDirection,
    coupling: ChangeCoupling,
}

impl EvolutionaryComparison {
    pub const fn new(
        id: EvolutionaryComparisonId,
        kind: EvolutionaryComparisonKind,
        direction: ComparisonDirection,
        coupling: ChangeCoupling,
    ) -> Self {
        Self {
            id,
            kind,
            direction,
            coupling,
        }
    }
    pub const fn id(self) -> EvolutionaryComparisonId {
        self.id
    }
    pub const fn kind(self) -> EvolutionaryComparisonKind {
        self.kind
    }
    pub const fn direction(self) -> ComparisonDirection {
        self.direction
    }
    pub const fn coupling(self) -> ChangeCoupling {
        self.coupling
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EvolutionaryReportFacts {
    pub(crate) coverage: HistoryCoverage,
    pub(crate) file_history: Vec<FileHistory>,
    pub(crate) package_history: Vec<PackageHistory>,
    pub(crate) coupling: Vec<ChangeCoupling>,
    pub(crate) concentration: Vec<ContributorConcentration>,
    pub(crate) findings: Vec<EvolutionaryFinding>,
    pub(crate) comparisons: Vec<EvolutionaryComparison>,
    pub(crate) comparison_suppressions: Vec<HistoryComparisonSuppression>,
    pub(crate) concentration_findings: Vec<KnowledgeConcentrationFinding>,
    pub(crate) file_coupling: Vec<FileChangeCoupling>,
}

impl EvolutionaryReportFacts {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        coverage: HistoryCoverage,
        file_history: Vec<FileHistory>,
        package_history: Vec<PackageHistory>,
        coupling: Vec<ChangeCoupling>,
        concentration: Vec<ContributorConcentration>,
        findings: Vec<EvolutionaryFinding>,
        comparisons: Vec<EvolutionaryComparison>,
    ) -> Self {
        Self {
            coverage,
            file_history,
            package_history,
            coupling,
            concentration,
            findings,
            comparisons,
            comparison_suppressions: Vec::new(),
            concentration_findings: Vec::new(),
            file_coupling: Vec::new(),
        }
    }
    /// Adds the knowledge-concentration findings, kept in their own table.
    pub fn with_concentration_findings(
        mut self,
        findings: Vec<KnowledgeConcentrationFinding>,
    ) -> Self {
        self.concentration_findings = findings;
        self
    }
    pub fn with_comparison_suppressions(
        mut self,
        suppressions: Vec<HistoryComparisonSuppression>,
    ) -> Self {
        self.comparison_suppressions = suppressions;
        self
    }
    /// Adds the retained file change coupling pairs, kept in their own table.
    ///
    /// A retained pair is the population a change-leakage detector reads. It is
    /// not a finding and reaches no human view.
    pub fn with_file_coupling(mut self, pairs: Vec<FileChangeCoupling>) -> Self {
        self.file_coupling = pairs;
        self
    }
    pub fn file_coupling(&self) -> &[FileChangeCoupling] {
        &self.file_coupling
    }
    pub fn concentration_findings(&self) -> &[KnowledgeConcentrationFinding] {
        &self.concentration_findings
    }
    pub fn findings(&self) -> &[EvolutionaryFinding] {
        &self.findings
    }
    pub fn comparisons(&self) -> &[EvolutionaryComparison] {
        &self.comparisons
    }
    pub fn comparison_suppressions(&self) -> &[HistoryComparisonSuppression] {
        &self.comparison_suppressions
    }
}

/// How the package dependency graph links a coupled package pair.
///
/// The classification informs presentation only: finding creation never reads
/// it, so an indirect link does not suppress or explain an
/// unexplained-coupling Watch finding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CouplingLink {
    /// A trusted eligible `uses` relation links the pair in either direction.
    Direct,
    /// No direct relation exists, but a dependency path connects the pair in
    /// one direction; the payload names the first intermediate package on a
    /// shortest such path.
    Indirect(PackageId),
    /// No dependency path connects the pair in either direction.
    None,
}

/// Whether a code dependency explains why two packages change together.
///
/// The pairs are direction-carrying, so a pair is explained when either
/// direction is present. They are wider than the verdict graph on purpose: a
/// dev-dependency test import is still a code dependency.
pub(crate) fn pair_is_explained(
    pairs: &BTreeSet<(PackageId, PackageId)>,
    left: PackageId,
    right: PackageId,
) -> bool {
    pairs.contains(&(left, right)) || pairs.contains(&(right, left))
}
