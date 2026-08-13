use std::collections::{BTreeMap, BTreeSet};

use crate::evolution::has_static_edge;
use crate::{
    ChangeCoupling, EvolutionaryFinding, EvolutionaryFindingId, HistoryCommitFact, PackageEdge,
    PackageId, SourceRole, SourceTrust,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Endpoint {
    package: PackageId,
    role: SourceRole,
    trust: SourceTrust,
}

pub fn change_coupling(commits: &[HistoryCommitFact]) -> Vec<ChangeCoupling> {
    let mut accumulator = ChangeCouplingAccumulator::default();
    for commit in commits {
        accumulator.accept(commit);
    }
    accumulator.finish().0
}

#[derive(Default)]
pub(crate) struct ChangeCouplingAccumulator {
    touches: BTreeMap<Endpoint, u32>,
    shared: BTreeMap<(Endpoint, Endpoint), u32>,
    eligible_touches: BTreeMap<PackageId, u32>,
    eligible_shared: BTreeMap<(PackageId, PackageId), u32>,
}

impl ChangeCouplingAccumulator {
    pub(crate) fn accept(&mut self, commit: &HistoryCommitFact) {
        let endpoints = commit
            .changes()
            .iter()
            .map(|change| Endpoint {
                package: change.package(),
                role: change.role(),
                trust: change.trust(),
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        for endpoint in &endpoints {
            *self.touches.entry(*endpoint).or_default() += 1;
        }
        for left_index in 0..endpoints.len() {
            for right in &endpoints[left_index + 1..] {
                if endpoints[left_index].package == right.package {
                    continue;
                }
                *self
                    .shared
                    .entry((endpoints[left_index], *right))
                    .or_default() += 1;
            }
        }
        let eligible_packages = commit
            .changes()
            .iter()
            .filter(|change| change.affects_findings())
            .map(|change| change.package())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        for package in &eligible_packages {
            *self.eligible_touches.entry(*package).or_default() += 1;
        }
        for left_index in 0..eligible_packages.len() {
            for right in &eligible_packages[left_index + 1..] {
                *self
                    .eligible_shared
                    .entry((eligible_packages[left_index], *right))
                    .or_default() += 1;
            }
        }
    }
    pub(crate) fn finish(self) -> (Vec<ChangeCoupling>, Vec<ChangeCoupling>) {
        (
            evidence_coupling_rows(self.shared, &self.touches),
            eligible_coupling_rows(self.eligible_shared, &self.eligible_touches),
        )
    }
}

fn evidence_coupling_rows(
    shared: BTreeMap<(Endpoint, Endpoint), u32>,
    touches: &BTreeMap<Endpoint, u32>,
) -> Vec<ChangeCoupling> {
    shared
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .map(|((left, right), count)| {
            ChangeCoupling::new(
                left.package,
                right.package,
                count,
                touches[&left] + touches[&right] - count,
            )
            .with_evidence(left.role, left.trust, right.role, right.trust)
        })
        .collect()
}

fn eligible_coupling_rows(
    shared: BTreeMap<(PackageId, PackageId), u32>,
    touches: &BTreeMap<PackageId, u32>,
) -> Vec<ChangeCoupling> {
    shared
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .map(|((left, right), count)| {
            ChangeCoupling::new(left, right, count, touches[&left] + touches[&right] - count)
        })
        .collect()
}

pub fn unexplained_coupling(
    coupling: &[ChangeCoupling],
    static_edges: &[PackageEdge],
) -> Vec<EvolutionaryFinding> {
    coupling
        .iter()
        .filter(|pair| qualifies_for_finding(**pair))
        .filter(|pair| !has_static_edge(static_edges, pair.left(), pair.right()))
        .enumerate()
        .map(|(index, pair)| {
            EvolutionaryFinding::new(EvolutionaryFindingId::from_index(index), *pair)
        })
        .collect()
}

pub(crate) fn qualifies_for_finding(pair: ChangeCoupling) -> bool {
    pair.shared_commits() >= 3
        && u64::from(pair.shared_commits()) * 5 >= u64::from(pair.union_commits())
}
