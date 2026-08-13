use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ContributorConcentration, ContributorId, HistoryCommitFact, PackageId, SourceRole, SourceTrust,
};

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
