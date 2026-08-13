use std::collections::{BTreeMap, BTreeSet};

use crate::evolution::has_static_edge;
use crate::{
    ChangeCoupling, EvolutionaryFinding, EvolutionaryFindingId, HistoryCommitFact, PackageEdge,
    PackageId,
};

pub fn change_coupling(commits: &[HistoryCommitFact]) -> Vec<ChangeCoupling> {
    let mut accumulator = ChangeCouplingAccumulator::default();
    for commit in commits {
        accumulator.accept(commit);
    }
    accumulator.finish()
}

#[derive(Default)]
pub(crate) struct ChangeCouplingAccumulator {
    touches: BTreeMap<PackageId, u32>,
    shared: BTreeMap<(PackageId, PackageId), u32>,
}

impl ChangeCouplingAccumulator {
    pub(crate) fn accept(&mut self, commit: &HistoryCommitFact) {
        let packages = commit
            .changes()
            .iter()
            .map(|change| change.package())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        for package in &packages {
            *self.touches.entry(*package).or_default() += 1;
        }
        for left_index in 0..packages.len() {
            for right in &packages[left_index + 1..] {
                *self
                    .shared
                    .entry((packages[left_index], *right))
                    .or_default() += 1;
            }
        }
    }
    pub(crate) fn finish(self) -> Vec<ChangeCoupling> {
        self.shared
            .into_iter()
            .filter(|(_, count)| *count >= 2)
            .map(|((left, right), count)| {
                ChangeCoupling::new(
                    left,
                    right,
                    count,
                    self.touches[&left] + self.touches[&right] - count,
                )
            })
            .collect()
    }
}

pub fn unexplained_coupling(
    coupling: &[ChangeCoupling],
    static_edges: &[PackageEdge],
) -> Vec<EvolutionaryFinding> {
    coupling
        .iter()
        .filter(|pair| !has_static_edge(static_edges, pair.left(), pair.right()))
        .enumerate()
        .map(|(index, pair)| {
            EvolutionaryFinding::new(EvolutionaryFindingId::from_index(index), *pair)
        })
        .collect()
}
