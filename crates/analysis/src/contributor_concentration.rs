use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ContributorConcentration, ContributorId, HistoryCommitFact, KnowledgeConcentrationFinding,
    KnowledgeConcentrationFindingId, PackageId, SourceRole, SourceTrust,
};

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

fn qualifies_for_finding(row: ContributorConcentration) -> bool {
    row.denominator() >= MINIMUM_CONCENTRATION_COMMITS
        && u64::from(row.numerator()) * 100
            >= u64::from(row.denominator()) * u64::from(MINIMUM_CONCENTRATION_PERCENT)
}

type PackageEvidence = (PackageId, SourceRole, SourceTrust);

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

#[cfg(test)]
mod tests {
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
}
