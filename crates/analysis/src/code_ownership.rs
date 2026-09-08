//! Code ownership measured by the top contributor's share of package commits.
//!
//! For each package, source role, and trust partition, count each contributor's
//! touch once per commit. Ownership is the largest contributor count divided by
//! all contributor counts. The exact numerator, denominator, and contributor
//! count are retained. This describes observed work in the selected history
//! window, not an assigned reviewer or a line-based authorship calculation.
//!
//! Trusted verdict roles produce a Watch knowledge-concentration finding at
//! 10 commits and 90% ownership, both inclusive. Weak or context observations
//! remain descriptive. History composition controls coverage eligibility.
//!
//! ```
//! use smackdebt_analysis::{ContributorConcentration, PackageId, knowledge_concentration};
//! let ownership = ContributorConcentration::new(PackageId::from_index(0), 2, 9, 10);
//! assert_eq!(knowledge_concentration(&[ownership]).len(), 1);
//! ```
//!
//! Terminology: [Bird et al., code ownership](https://www.microsoft.com/en-us/research/uploads/prod/2016/02/bird2011dtm.pdf).

#![deny(missing_docs)]

use crate::{ComparisonDirection, EvolutionaryFindingKind};
use crate::{ContributorId, HistoryCommitFact, PackageId, SourceRole, SourceTrust};
use std::collections::{BTreeMap, BTreeSet};

/// The windowed commits a package needs before its top share is a finding.
pub const MINIMUM_CONCENTRATION_COMMITS: u32 = 10;

/// The top-contributor share, in percent, that concentrates knowledge.
pub const MINIMUM_CONCENTRATION_PERCENT: u32 = 90;

/// Turns eligible concentration rows into Watch findings.
///
/// The share is compared with integer operands only, and the finding keeps the
/// package, the contributor count, the numerator, and the denominator. Weaker
/// observations stay descriptive. Context rows never qualify, so generated or
/// fixture history cannot create a finding.
pub fn knowledge_concentration(
    rows: &[ContributorConcentration],
) -> Vec<KnowledgeConcentrationFinding> {
    rows.iter()
        .filter(|row| row.affects_findings() && qualifies_for_finding(**row))
        .enumerate()
        .map(|(index, row)| {
            KnowledgeConcentrationFinding::new(
                KnowledgeConcentrationFindingId::from_index(index),
                *row,
            )
        })
        .collect()
}

/// Names every package whose knowledge concentration the change moved.
///
/// A package qualifies or it does not, on each side, by exactly the rule that
/// states a standing finding — so a change that pushed a package over the bar
/// introduced the concentration, and one that pulled it back dissolved it. A
/// package that qualified on both sides states nothing: its concentration is
/// standing history, which a change did not make.
pub fn compare_concentration(
    before: &[ContributorConcentration],
    after: &[ContributorConcentration],
) -> Vec<ConcentrationComparison> {
    let qualified = |rows: &[ContributorConcentration], package: PackageId, role: SourceRole| {
        rows.iter()
            .find(|row| row.package() == package && row.role() == role)
            .copied()
            .filter(|row| row.affects_findings() && qualifies_for_finding(*row))
    };
    let mut comparisons = Vec::new();
    for row in after {
        let now = row.affects_findings() && qualifies_for_finding(*row);
        let then = qualified(before, row.package(), row.role());
        if now && then.is_none() {
            comparisons.push(ConcentrationComparison::new(
                ConcentrationComparisonId::from_index(comparisons.len()),
                ConcentrationComparisonKind::Introduced,
                *row,
            ));
        }
    }
    for row in before {
        let then = row.affects_findings() && qualifies_for_finding(*row);
        if then && qualified(after, row.package(), row.role()).is_none() {
            comparisons.push(ConcentrationComparison::new(
                ConcentrationComparisonId::from_index(comparisons.len()),
                ConcentrationComparisonKind::Dissolved,
                *row,
            ));
        }
    }
    comparisons
}

fn qualifies_for_finding(row: ContributorConcentration) -> bool {
    row.denominator() >= MINIMUM_CONCENTRATION_COMMITS
        && u64::from(row.numerator()) * 100
            >= u64::from(row.denominator()) * u64::from(MINIMUM_CONCENTRATION_PERCENT)
}

type PackageEvidence = (PackageId, SourceRole, SourceTrust);

/// Counts commit-based ownership per package, role, and trust in stable key order.
pub fn contributor_concentration(commits: &[HistoryCommitFact]) -> Vec<ContributorConcentration> {
    let mut accumulator = ContributorConcentrationAccumulator::default();
    for commit in commits {
        accumulator.accept(commit);
    }
    accumulator.finish()
}

#[derive(Default)]
pub(crate) struct ContributorConcentrationAccumulator {
    counts: BTreeMap<PackageEvidence, BTreeMap<ContributorId, u32>>,
}

impl ContributorConcentrationAccumulator {
    pub(crate) fn accept(&mut self, commit: &HistoryCommitFact) {
        let packages = commit
            .changes()
            .iter()
            .map(|change| (change.package(), change.role(), change.trust()))
            .collect::<BTreeSet<_>>();
        for evidence in packages {
            *self
                .counts
                .entry(evidence)
                .or_default()
                .entry(commit.contributor())
                .or_default() += 1;
        }
    }
    pub(crate) fn finish(self) -> Vec<ContributorConcentration> {
        self.counts
            .into_iter()
            .map(|((package, role, trust), contributors)| {
                let numerator = contributors.values().copied().max().unwrap_or(0);
                let denominator = contributors.values().sum();
                ContributorConcentration::new(
                    package,
                    contributors.len() as u32,
                    numerator,
                    denominator,
                )
                .with_evidence(role, trust)
            })
            .collect()
    }
}

/// Top-contributor ownership, stored as commit counts for a package evidence partition.
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
    /// Retains contributor count and top/total commit counts for one package.
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
    /// Attaches the observed source role and trust without changing the measurements.
    pub const fn with_evidence(mut self, role: SourceRole, trust: SourceTrust) -> Self {
        self.role = role;
        self.trust = trust;
        self
    }
    /// The package-table identity this observation describes.
    pub const fn package(self) -> PackageId {
        self.package
    }
    /// The source role of the evidence behind this observation.
    pub const fn role(self) -> SourceRole {
        self.role
    }
    /// Whether the source facts behind this observation are trusted or advisory.
    pub const fn trust(self) -> SourceTrust {
        self.trust
    }
    /// Whether trusted source in a verdict role may contribute to findings.
    pub const fn affects_findings(self) -> bool {
        self.role.affects_verdict() && matches!(self.trust, SourceTrust::Trusted)
    }
    /// The number of contributors with at least one commit in this partition.
    pub const fn contributor_count(self) -> u32 {
        self.contributor_count
    }
    /// The top contributor's commit count in this package evidence partition.
    pub const fn numerator(self) -> u32 {
        self.numerator
    }
    /// The total commit count in this package evidence partition.
    pub const fn denominator(self) -> u32 {
        self.denominator
    }
    /// The top/total commit ratio as a convenience float.
    ///
    /// Finding policy compares integer operands; this accessor does not validate
    /// a zero denominator supplied to the constructor.
    pub fn ratio(self) -> f64 {
        f64::from(self.numerator) / f64::from(self.denominator)
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
    /// Records ownership already selected by the knowledge-concentration rule.
    pub const fn new(
        id: KnowledgeConcentrationFindingId,
        concentration: ContributorConcentration,
    ) -> Self {
        Self { id, concentration }
    }
    /// The row's typed position in its owning report table.
    pub const fn id(self) -> KnowledgeConcentrationFindingId {
        self.id
    }
    /// The finding or movement category represented by this row.
    pub const fn kind(self) -> EvolutionaryFindingKind {
        EvolutionaryFindingKind::KnowledgeConcentration
    }
    /// The health rating carried by this observation.
    pub const fn rating(self) -> crate::Rating {
        crate::Rating::Watch
    }
    /// The exact ownership operands supporting this finding.
    pub const fn concentration(self) -> ContributorConcentration {
        self.concentration
    }
}

/// What a change did to one package's knowledge concentration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConcentrationComparisonKind {
    /// The package now rests on one contributor and did not before.
    Introduced,
    /// The package no longer rests on one contributor.
    Dissolved,
}

/// One package whose knowledge concentration the change under review moved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConcentrationComparison {
    id: ConcentrationComparisonId,
    kind: ConcentrationComparisonKind,
    concentration: ContributorConcentration,
}

impl ConcentrationComparison {
    /// Retains ownership evidence and whether its finding appeared or disappeared.
    pub const fn new(
        id: ConcentrationComparisonId,
        kind: ConcentrationComparisonKind,
        concentration: ContributorConcentration,
    ) -> Self {
        Self {
            id,
            kind,
            concentration,
        }
    }
    /// The row's typed position in its owning report table.
    pub const fn id(self) -> ConcentrationComparisonId {
        self.id
    }
    /// The finding or movement category represented by this row.
    pub const fn kind(self) -> ConcentrationComparisonKind {
        self.kind
    }
    /// The side that states the fact: the change's own tally when knowledge
    /// concentrated, and the tally it dissolved when it did not.
    pub const fn concentration(self) -> ContributorConcentration {
        self.concentration
    }
    /// Whether the comparison represents worse, better, or neutral debt movement.
    pub const fn direction(self) -> ComparisonDirection {
        match self.kind {
            ConcentrationComparisonKind::Introduced => ComparisonDirection::Worse,
            ConcentrationComparisonKind::Dissolved => ComparisonDirection::Better,
        }
    }
}

crate::table_index::table_index!(
    /// The position of one knowledge concentration finding in its report table.
    KnowledgeConcentrationFindingId
);

crate::table_index::table_index!(
    /// The position of one concentration comparison in its report table.
    ConcentrationComparisonId
);

#[cfg(test)]
mod tests {
    fn commit(
        contributor: usize,
        changes: &[(usize, usize, Option<u32>, Option<u32>)],
    ) -> crate::HistoryCommitFact {
        crate::HistoryCommitFact::new(
            crate::ContributorId::from_index(contributor),
            changes
                .iter()
                .map(|(file, package, added, deleted)| {
                    crate::HistoryChangeFact::new(
                        crate::FileId::from_index(*file),
                        crate::PackageId::from_index(*package),
                        *added,
                        *deleted,
                    )
                })
                .collect(),
        )
    }

    fn concentrated(package: usize, numerator: u32, denominator: u32) -> ContributorConcentration {
        ContributorConcentration::new(PackageId::from_index(package), 1, numerator, denominator)
    }

    /// A change that pushed a package onto one contributor made that
    /// concentration, so the diff may say so.
    #[test]
    fn concentration_the_change_created_is_introduced() {
        let before = [concentrated(0, 1, 10)];
        let after = [concentrated(0, 10, 10)];
        let comparisons = compare_concentration(&before, &after);
        assert_eq!(comparisons.len(), 1);
        assert_eq!(
            comparisons[0].kind(),
            ConcentrationComparisonKind::Introduced
        );
        assert_eq!(comparisons[0].concentration().numerator(), 10);
    }

    /// A change that spread the work back out dissolved the concentration.
    #[test]
    fn concentration_the_change_ended_is_dissolved() {
        let before = [concentrated(0, 10, 10)];
        let after = [concentrated(0, 1, 10)];
        let comparisons = compare_concentration(&before, &after);
        assert_eq!(comparisons.len(), 1);
        assert_eq!(
            comparisons[0].kind(),
            ConcentrationComparisonKind::Dissolved
        );
    }

    /// Concentration both sides hold is standing history, which no change
    /// made and no diff claims.
    #[test]
    fn standing_concentration_states_no_movement() {
        let rows = [concentrated(0, 10, 10)];
        assert!(compare_concentration(&rows, &rows).is_empty());
    }

    use super::*;
    use crate::{EvolutionaryFindingKind, Rating};

    fn row(contributors: u32, numerator: u32, denominator: u32) -> ContributorConcentration {
        ContributorConcentration::new(
            PackageId::from_index(0),
            contributors,
            numerator,
            denominator,
        )
    }

    #[test]
    fn one_contributor_owning_a_package_is_a_watch_finding_of_counts_only() {
        let findings = knowledge_concentration(&[row(3, 19, 20)]);
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].id(),
            KnowledgeConcentrationFindingId::from_index(0)
        );
        assert_eq!(findings[0].rating(), Rating::Watch);
        assert_eq!(
            findings[0].kind(),
            EvolutionaryFindingKind::KnowledgeConcentration
        );
        let concentration = findings[0].concentration();
        assert_eq!(concentration.package(), PackageId::from_index(0));
        assert_eq!(concentration.contributor_count(), 3);
        assert_eq!(concentration.numerator(), 19);
        assert_eq!(concentration.denominator(), 20);
    }

    #[test]
    fn the_commit_and_share_boundaries_use_integer_comparison() {
        assert_eq!(MINIMUM_CONCENTRATION_COMMITS, 10);
        assert_eq!(MINIMUM_CONCENTRATION_PERCENT, 90);
        // Nine commits stay descriptive even at a complete share.
        assert!(knowledge_concentration(&[row(1, 9, 9)]).is_empty());
        assert_eq!(knowledge_concentration(&[row(1, 10, 10)]).len(), 1);
        // Eighty-nine percent stays descriptive; ninety percent qualifies.
        assert!(knowledge_concentration(&[row(2, 89, 100)]).is_empty());
        assert_eq!(knowledge_concentration(&[row(2, 90, 100)]).len(), 1);
    }

    #[test]
    fn context_rows_cannot_create_a_finding() {
        let context = row(1, 20, 20).with_evidence(SourceRole::Generated, SourceTrust::Trusted);
        assert!(knowledge_concentration(&[context]).is_empty());
        let advisory = row(1, 20, 20).with_evidence(SourceRole::Primary, SourceTrust::Advisory);
        assert!(knowledge_concentration(&[advisory]).is_empty());
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
}
