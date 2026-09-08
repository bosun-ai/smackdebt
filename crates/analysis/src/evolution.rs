//! Shared history facts, coverage, and composition of history-derived metrics.
//!
//! One accumulator feeds each metric from the same streamed commits. This module
//! owns coverage-based suppression and completed history tables, while metric
//! values and rules live in their named modules.

use crate::package_change_coupling::pair_is_explained;
use crate::{
    ChangeCoupling, ConcentrationComparison, ContributorConcentration, EvolutionaryComparison,
    EvolutionaryFinding, FileChangeCoupling, FileHistory, KnowledgeConcentrationFinding,
    PackageHistory,
};
use crate::{FileId, PackageId, SourceRole, SourceTrust};
use std::collections::BTreeSet;

crate::table_index::table_index!(
    /// A contributor identity within the selected history.
    ContributorId
);
crate::table_index::table_index!(
    /// The position of one history comparison suppression in its report table.
    HistoryComparisonSuppressionId
);

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

/// How one history stream's mapped changes were classified, on both axes.
///
/// Every mapped change is classified exactly once as eligible or context,
/// and exactly once as textual or uncounted, so the two axes always sum to
/// the same total.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HistoryChangeCounts {
    pub mapped_eligible: u32,
    pub context: u32,
    pub textual: u32,
    pub uncounted: u32,
    pub excluded: u32,
    pub rename_gaps: u32,
}

impl HistoryCoverage {
    pub fn new(
        availability: HistoryAvailability,
        revision: Option<String>,
        commits: u32,
        eligible_commits: u32,
        reason: Option<String>,
    ) -> Self {
        assert!(
            eligible_commits <= commits,
            "eligible history commits cannot exceed streamed commits"
        );
        Self {
            availability,
            revision,
            commits,
            eligible_commits,
            mapped_eligible_changes: 0,
            context_changes: 0,
            newest_timestamp: None,
            oldest_timestamp: None,
            textual_changes: 0,
            uncounted_changes: 0,
            excluded_changes: 0,
            rename_gaps: 0,
            reason,
            window_days: None,
            window_excluded_commits: 0,
            bulk_commits: 0,
            declined_pairs: 0,
        }
    }

    /// Records how the stream's mapped changes were classified.
    pub fn with_changes(mut self, counts: HistoryChangeCounts) -> Self {
        assert_eq!(
            u64::from(counts.textual) + u64::from(counts.uncounted),
            u64::from(counts.mapped_eligible) + u64::from(counts.context),
            "mapped history changes must be classified once as textual or uncounted"
        );
        self.mapped_eligible_changes = counts.mapped_eligible;
        self.context_changes = counts.context;
        self.textual_changes = counts.textual;
        self.uncounted_changes = counts.uncounted;
        self.excluded_changes = counts.excluded;
        self.rename_gaps = counts.rename_gaps;
        self
    }

    /// Records the newest and oldest instants the stream visited.
    pub fn with_timestamps(mut self, newest: Option<i64>, oldest: Option<i64>) -> Self {
        self.newest_timestamp = newest;
        self.oldest_timestamp = oldest;
        self
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
    ahead_of_base: bool,
}

impl HistoryCommitFact {
    pub fn new(contributor: ContributorId, changes: Vec<HistoryChangeFact>) -> Self {
        Self {
            contributor,
            changes,
            ahead_of_base: false,
        }
    }
    /// Records that the change under review contains this commit, so the
    /// history without it is the history the change started from.
    #[must_use]
    pub fn made_by_the_change(mut self) -> Self {
        self.ahead_of_base = true;
        self
    }
    /// Whether the change under review made this commit.
    pub const fn made_by_change(&self) -> bool {
        self.ahead_of_base
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
    churn: crate::code_churn::ChurnAccumulator,
    coupling: crate::package_change_coupling::ChangeCouplingAccumulator,
    concentration: crate::code_ownership::ContributorConcentrationAccumulator,
    /// The same tally without the commits the change under review made, which
    /// is what the packages looked like before it.
    base_concentration: crate::code_ownership::ContributorConcentrationAccumulator,
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
        if !commit.made_by_change() {
            self.base_concentration.accept(&commit);
        }
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
                    .filter(|pair| crate::package_change_coupling::qualifies_for_finding(**pair))
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
        // A codebase report streams no base side, so its two tallies are the
        // same one and nothing moved.
        let base_concentration = self.base_concentration.finish();
        let concentration_comparisons = if history_is_sufficient {
            crate::compare_concentration(&base_concentration, &concentration)
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
        .with_concentration_comparisons(concentration_comparisons)
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

/// What an evolutionary finding observed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvolutionaryFindingKind {
    UnexplainedCoupling,
    KnowledgeConcentration,
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
    pub(crate) concentration_comparisons: Vec<ConcentrationComparison>,
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
            concentration_comparisons: Vec::new(),
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
    /// Adds the concentration movements this change made, if any.
    pub fn with_concentration_comparisons(
        mut self,
        comparisons: Vec<ConcentrationComparison>,
    ) -> Self {
        self.concentration_comparisons = comparisons;
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
    pub fn concentration_comparisons(&self) -> &[ConcentrationComparison] {
        &self.concentration_comparisons
    }
    pub fn comparisons(&self) -> &[EvolutionaryComparison] {
        &self.comparisons
    }
    pub fn comparison_suppressions(&self) -> &[HistoryComparisonSuppression] {
        &self.comparison_suppressions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DirectoryTree, change_coupling, contributor_concentration};

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
            None,
        )
        .with_changes(HistoryChangeCounts {
            mapped_eligible: 3,
            context: 0,
            textual: 0,
            uncounted: 3,
            excluded: 0,
            rename_gaps: 0,
        })
        .with_window(window.days(), 0);
        assert_eq!(coverage.window_days(), Some(1));
        assert_eq!(coverage.window_excluded_commits(), 0);
        assert_eq!(coverage.commits(), 2);
        assert_eq!(excluded, 2);
    }

    #[test]
    fn coverage_retains_boundary_rejects_from_the_defensive_window_check() {
        let coverage = HistoryCoverage::new(HistoryAvailability::Complete, None, 2, 1, None)
            .with_changes(HistoryChangeCounts {
                mapped_eligible: 1,
                context: 0,
                textual: 1,
                uncounted: 0,
                excluded: 0,
                rename_gaps: 0,
            })
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
                Some("history is shallow".to_owned()),
            )
            .with_changes(HistoryChangeCounts {
                mapped_eligible: 6,
                context: 0,
                textual: 6,
                uncounted: 0,
                excluded: 0,
                rename_gaps: 0,
            }),
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
            HistoryCoverage::new(HistoryAvailability::Complete, None, 3, 3, None).with_changes(
                HistoryChangeCounts {
                    mapped_eligible: 6,
                    context: 0,
                    textual: 6,
                    uncounted: 0,
                    excluded: 0,
                    rename_gaps: 1,
                },
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
            HistoryCoverage::new(HistoryAvailability::Complete, None, 4, 4, None).with_changes(
                HistoryChangeCounts {
                    mapped_eligible: changes,
                    context: 0,
                    textual: changes,
                    uncounted: 0,
                    excluded: 0,
                    rename_gaps: 0,
                },
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

    /// Change amplification rides the same stream as the pairs and is limited
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
            HistoryCoverage::new(availability, None, 10, 10, None).with_changes(
                HistoryChangeCounts {
                    mapped_eligible: 10,
                    context: 0,
                    textual: 10,
                    uncounted: 0,
                    excluded: 0,
                    rename_gaps: 0,
                },
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
            HistoryCoverage::new(HistoryAvailability::Complete, None, 3, 3, None).with_changes(
                HistoryChangeCounts {
                    mapped_eligible: 3,
                    context: 3,
                    textual: 6,
                    uncounted: 0,
                    excluded: 0,
                    rename_gaps: 0,
                },
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
                Some("repository history is shallow".to_owned()),
            )
            .with_changes(HistoryChangeCounts {
                mapped_eligible: 6,
                context: 0,
                textual: 6,
                uncounted: 0,
                excluded: 0,
                rename_gaps: 0,
            }),
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
            HistoryCoverage::new(HistoryAvailability::Complete, None, 3, 0, None).with_changes(
                HistoryChangeCounts {
                    mapped_eligible: 0,
                    context: 6,
                    textual: 6,
                    uncounted: 0,
                    excluded: 0,
                    rename_gaps: 0,
                },
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
            HistoryCoverage::new(HistoryAvailability::Complete, None, 5, 3, None).with_changes(
                HistoryChangeCounts {
                    mapped_eligible: 6,
                    context: 2,
                    textual: 8,
                    uncounted: 0,
                    excluded: 0,
                    rename_gaps: 0,
                },
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
}
