use crate::{ComparisonDirection, FileId, PackageEdge, PackageId, SourceRole, SourceTrust};

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
        }
    }
    /// Records the selected window and the commits it excluded.
    pub fn with_window(mut self, days: Option<u32>, excluded_commits: u32) -> Self {
        assert!(
            excluded_commits <= self.commits,
            "window-excluded commits cannot exceed streamed commits"
        );
        self.window_days = days;
        self.window_excluded_commits = excluded_commits;
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
    /// Streamed commits the selected window excluded, counted separately from
    /// changes excluded for other reasons.
    pub const fn window_excluded_commits(&self) -> u32 {
        self.window_excluded_commits
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
    pub fn register_source(
        &mut self,
        file: FileId,
        package: PackageId,
        role: SourceRole,
        trust: SourceTrust,
    ) {
        self.churn.register_source(file, package, role, trust);
    }
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
        containment: &crate::PackageContainment,
        static_edges: &[PackageEdge],
        before_static_edges: Option<&[PackageEdge]>,
    ) -> EvolutionaryReportFacts {
        let (file_history, package_history) = self.churn.finish(file_count, package_count);
        let (coupling, eligible_coupling) = self.coupling.finish(containment);
        let concentration = self.concentration.finish();
        let history_is_sufficient = coverage.availability() == HistoryAvailability::Complete
            && coverage.eligible_commits() > 0
            && coverage.mapped_eligible_changes() > 0;
        let findings = if history_is_sufficient {
            crate::unexplained_coupling(&eligible_coupling, static_edges)
        } else {
            Vec::new()
        };
        let comparisons = if history_is_sufficient {
            before_static_edges.map_or_else(Vec::new, |before| {
                crate::compare_evolution(&eligible_coupling, before, static_edges)
            })
        } else {
            Vec::new()
        };
        let concentration_findings = if history_is_sufficient {
            crate::knowledge_concentration(&concentration)
        } else {
            Vec::new()
        };
        EvolutionaryReportFacts::new(
            coverage,
            file_history,
            package_history,
            coupling,
            concentration,
            findings,
            comparisons,
        )
        .with_concentration_findings(concentration_findings)
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

        let coverage = HistoryCoverage::new(
            HistoryAvailability::Complete,
            None,
            every.len() as u32,
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
        .with_window(window.days(), excluded);
        assert_eq!(coverage.window_days(), Some(1));
        assert_eq!(coverage.window_excluded_commits(), 2);
        assert_eq!(coverage.commits(), 4);
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
    fn static_edge_suppresses_finding_and_changes_diff_context() {
        let pair = ChangeCoupling::new(PackageId::from_index(0), PackageId::from_index(1), 3, 3);
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
    fn coupling_finding_requires_both_thresholds() {
        let package_a = PackageId::from_index(0);
        let package_b = PackageId::from_index(1);
        let at_boundary = ChangeCoupling::new(package_a, package_b, 3, 15);
        let too_few_shared = ChangeCoupling::new(package_a, package_b, 2, 10);
        let below_similarity = ChangeCoupling::new(package_a, package_b, 3, 16);

        assert_eq!(crate::unexplained_coupling(&[at_boundary], &[]).len(), 1);
        assert!(crate::unexplained_coupling(&[too_few_shared], &[]).is_empty());
        assert!(crate::unexplained_coupling(&[below_similarity], &[]).is_empty());
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
            accumulator.accept(HistoryCommitFact::new(
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
            ));
        }
        let report = accumulator.finish(
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
            &[],
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
            accumulator.accept(commit(
                contributor,
                &[(0, 0, Some(1), Some(0)), (1, 1, Some(1), Some(0))],
            ));
        }
        let report = accumulator.finish(
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
            &[],
            Some(&[PackageEdge::new(
                PackageEdgeId::from_index(0),
                PackageId::from_index(0),
                PackageId::from_index(1),
                1,
                1,
                vec![DependencyEdgeId::from_index(0)],
            )]),
        );
        assert_eq!(report.coupling.len(), 1);
        assert!(report.findings().is_empty());
        assert!(report.comparisons().is_empty());
    }

    #[test]
    fn complete_history_without_eligible_mapping_cannot_create_findings() {
        let mut accumulator = EvolutionAccumulator::default();
        for contributor in 0..3 {
            accumulator.accept(HistoryCommitFact::new(
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
            ));
        }
        let report = accumulator.finish(
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
            &[],
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
            accumulator.accept(commit(
                contributor,
                &[(0, 0, Some(1), Some(0)), (1, 1, Some(1), Some(0))],
            ));
        }
        for contributor in 3..5 {
            accumulator.accept(HistoryCommitFact::new(
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
            ));
        }
        let report = accumulator.finish(
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
            &[],
            None,
        );

        assert_eq!(report.coupling[0].shared_commits(), 3);
        assert_eq!(report.coupling[0].union_commits(), 5);
        let finding = report.findings()[0].coupling().expect("a coupling finding");
        assert_eq!(finding.shared_commits(), 3);
        assert_eq!(finding.union_commits(), 3);
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

/// One Watch observation about how the code changed.
///
/// The kind discriminates the two observations, and each kind carries only its
/// own operands. A knowledge-concentration finding carries counts alone: no
/// name, address, raw author field, or internal contributor identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvolutionaryFinding {
    id: EvolutionaryFindingId,
    kind: EvolutionaryFindingKind,
    coupling: Option<ChangeCoupling>,
    concentration: Option<ContributorConcentration>,
}

impl EvolutionaryFinding {
    pub const fn new(id: EvolutionaryFindingId, coupling: ChangeCoupling) -> Self {
        Self {
            id,
            kind: EvolutionaryFindingKind::UnexplainedCoupling,
            coupling: Some(coupling),
            concentration: None,
        }
    }
    pub const fn knowledge_concentration(
        id: EvolutionaryFindingId,
        concentration: ContributorConcentration,
    ) -> Self {
        Self {
            id,
            kind: EvolutionaryFindingKind::KnowledgeConcentration,
            coupling: None,
            concentration: Some(concentration),
        }
    }
    pub const fn id(self) -> EvolutionaryFindingId {
        self.id
    }
    pub const fn kind(self) -> EvolutionaryFindingKind {
        self.kind
    }
    /// Both kinds are Watch observations.
    pub const fn rating(self) -> crate::Rating {
        crate::Rating::Watch
    }
    pub const fn coupling(self) -> Option<ChangeCoupling> {
        self.coupling
    }
    pub const fn concentration(self) -> Option<ContributorConcentration> {
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
    pub(crate) concentration_findings: Vec<EvolutionaryFinding>,
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
            concentration_findings: Vec::new(),
        }
    }
    /// Adds the knowledge-concentration findings, kept in their own table.
    pub fn with_concentration_findings(mut self, findings: Vec<EvolutionaryFinding>) -> Self {
        self.concentration_findings = findings;
        self
    }
    pub fn concentration_findings(&self) -> &[EvolutionaryFinding] {
        &self.concentration_findings
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
