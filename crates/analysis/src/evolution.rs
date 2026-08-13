use crate::{ComparisonDirection, FileId, PackageEdge, PackageId};

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
evolution_index!(EvolutionaryComparisonId);

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
    newest_timestamp: Option<i64>,
    oldest_timestamp: Option<i64>,
    textual_changes: u32,
    uncounted_changes: u32,
    excluded_paths: u32,
    rename_gaps: u32,
    reason: Option<String>,
}

impl HistoryCoverage {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        availability: HistoryAvailability,
        revision: Option<String>,
        commits: u32,
        newest_timestamp: Option<i64>,
        oldest_timestamp: Option<i64>,
        textual_changes: u32,
        uncounted_changes: u32,
        excluded_paths: u32,
        rename_gaps: u32,
        reason: Option<String>,
    ) -> Self {
        Self {
            availability,
            revision,
            commits,
            newest_timestamp,
            oldest_timestamp,
            textual_changes,
            uncounted_changes,
            excluded_paths,
            rename_gaps,
            reason,
        }
    }
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(
            HistoryAvailability::Unavailable,
            None,
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
    pub const fn excluded_paths(&self) -> u32 {
        self.excluded_paths
    }
    pub const fn rename_gaps(&self) -> u32 {
        self.rename_gaps
    }
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
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
            added_lines,
            deleted_lines,
        }
    }
    pub const fn file(self) -> FileId {
        self.file
    }
    pub const fn package(self) -> PackageId {
        self.package
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
}

impl EvolutionAccumulator {
    pub fn accept(&mut self, commit: HistoryCommitFact) {
        self.churn.accept(&commit);
        self.coupling.accept(&commit);
        self.concentration.accept(&commit);
    }
    pub fn finish(
        self,
        coverage: HistoryCoverage,
        file_count: usize,
        package_count: usize,
        static_edges: &[PackageEdge],
        before_static_edges: Option<&[PackageEdge]>,
    ) -> EvolutionaryReportFacts {
        let (file_history, package_history) = self.churn.finish(file_count, package_count);
        let coupling = self.coupling.finish();
        let concentration = self.concentration.finish();
        let findings = crate::unexplained_coupling(&coupling, static_edges);
        let comparisons = before_static_edges.map_or_else(Vec::new, |before| {
            crate::compare_evolution(&coupling, before, static_edges)
        });
        EvolutionaryReportFacts::new(
            coverage,
            file_history,
            package_history,
            coupling,
            concentration,
            findings,
            comparisons,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DependencyEdgeId, HistoryChangeFact, PackageEdgeId, change_coupling,
        contributor_concentration,
    };

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
        let values = change_coupling(&commits);
        assert_eq!(
            values,
            vec![ChangeCoupling::new(
                PackageId::from_index(0),
                PackageId::from_index(1),
                2,
                4
            )]
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
    fn static_edge_suppresses_finding_and_changes_diff_context() {
        let pair = ChangeCoupling::new(PackageId::from_index(0), PackageId::from_index(1), 2, 2);
        let edge = PackageEdge::new(
            PackageEdgeId::from_index(0),
            PackageId::from_index(0),
            PackageId::from_index(1),
            1,
            1,
            vec![DependencyEdgeId::from_index(0)],
        );
        assert_eq!(crate::unexplained_coupling(&[pair], &[]).len(), 1);
        assert!(crate::unexplained_coupling(&[pair], std::slice::from_ref(&edge)).is_empty());
        let comparisons = crate::compare_evolution(&[pair], &[], &[edge]);
        assert_eq!(
            comparisons[0].kind(),
            EvolutionaryComparisonKind::FindingRemoved
        );
        assert_eq!(comparisons[0].direction(), ComparisonDirection::Better);
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
        let values = change_coupling(&commits);
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
            touches,
            added_lines,
            deleted_lines,
            uncounted_changes,
        }
    }
    pub const fn file(self) -> FileId {
        self.file
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
            touches,
            added_lines,
            deleted_lines,
            uncounted_changes,
        }
    }
    pub const fn package(self) -> PackageId {
        self.package
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
        }
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContributorConcentration {
    package: PackageId,
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
            contributor_count,
            numerator,
            denominator,
        }
    }
    pub const fn package(self) -> PackageId {
        self.package
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
    pub const fn coupling(self) -> ChangeCoupling {
        self.coupling
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
        }
    }
    pub fn findings(&self) -> &[EvolutionaryFinding] {
        &self.findings
    }
    pub fn comparisons(&self) -> &[EvolutionaryComparison] {
        &self.comparisons
    }
}

pub(crate) fn has_static_edge(edges: &[PackageEdge], left: PackageId, right: PackageId) -> bool {
    edges.iter().any(|edge| {
        (edge.source() == left && edge.target() == right)
            || (edge.source() == right && edge.target() == left)
    })
}
