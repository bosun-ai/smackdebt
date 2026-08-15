use std::collections::{BTreeMap, BTreeSet};

use crate::evolution::has_static_edge;
use crate::{
    ChangeCoupling, EvolutionaryFinding, EvolutionaryFindingId, HistoryCommitFact, PackageEdge,
    PackageId, SourceRole, SourceTrust,
};

/// The strongest source evidence one package contributed to a commit.
type Evidence = (SourceRole, SourceTrust);

/// Which package scopes contain which other package scopes.
///
/// Shared commits between a scope and its own descendant are structural, so
/// such a pair is never coupling evidence and never forms a coupling row.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackageContainment {
    nested: BTreeSet<(PackageId, PackageId)>,
}

impl PackageContainment {
    /// Relates packages by their repository paths, where `.` is the root.
    pub fn from_paths(paths: &[String]) -> Self {
        let mut nested = BTreeSet::new();
        for (left, left_path) in paths.iter().enumerate() {
            for (right, right_path) in paths.iter().enumerate().skip(left + 1) {
                if contains(left_path, right_path) || contains(right_path, left_path) {
                    nested.insert((PackageId::from_index(left), PackageId::from_index(right)));
                }
            }
        }
        Self { nested }
    }

    /// Whether one package scope contains the other.
    pub fn is_nested(&self, left: PackageId, right: PackageId) -> bool {
        let pair = if left <= right {
            (left, right)
        } else {
            (right, left)
        };
        self.nested.contains(&pair)
    }
}

/// Whether the first repository path is an ancestor scope of the second.
fn contains(ancestor: &str, descendant: &str) -> bool {
    if ancestor == descendant {
        return false;
    }
    ancestor == "." || descendant.starts_with(&format!("{ancestor}/"))
}

pub fn change_coupling(
    commits: &[HistoryCommitFact],
    containment: &PackageContainment,
) -> Vec<ChangeCoupling> {
    let mut accumulator = ChangeCouplingAccumulator::default();
    for commit in commits {
        accumulator.accept(commit);
    }
    accumulator.finish(containment).0
}

#[derive(Default)]
pub(crate) struct ChangeCouplingAccumulator {
    touches: BTreeMap<PackageId, u32>,
    shared: BTreeMap<(PackageId, PackageId), u32>,
    evidence: BTreeMap<(PackageId, PackageId), (Evidence, Evidence)>,
    eligible_touches: BTreeMap<PackageId, u32>,
    eligible_shared: BTreeMap<(PackageId, PackageId), u32>,
}

impl ChangeCouplingAccumulator {
    pub(crate) fn accept(&mut self, commit: &HistoryCommitFact) {
        let mut packages: BTreeMap<PackageId, Evidence> = BTreeMap::new();
        for change in commit.changes() {
            let evidence = (change.role(), change.trust());
            packages
                .entry(change.package())
                .and_modify(|value| *value = (*value).min(evidence))
                .or_insert(evidence);
        }
        let packages: Vec<_> = packages.into_iter().collect();
        for (package, _) in &packages {
            *self.touches.entry(*package).or_default() += 1;
        }
        for left_index in 0..packages.len() {
            let (left, left_evidence) = packages[left_index];
            for (right, right_evidence) in &packages[left_index + 1..] {
                *self.shared.entry((left, *right)).or_default() += 1;
                self.evidence
                    .entry((left, *right))
                    .and_modify(|value| {
                        value.0 = value.0.min(left_evidence);
                        value.1 = value.1.min(*right_evidence);
                    })
                    .or_insert((left_evidence, *right_evidence));
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
    pub(crate) fn finish(
        self,
        containment: &PackageContainment,
    ) -> (Vec<ChangeCoupling>, Vec<ChangeCoupling>) {
        (
            evidence_coupling_rows(self.shared, &self.touches, &self.evidence, containment),
            eligible_coupling_rows(self.eligible_shared, &self.eligible_touches, containment),
        )
    }
}

fn evidence_coupling_rows(
    shared: BTreeMap<(PackageId, PackageId), u32>,
    touches: &BTreeMap<PackageId, u32>,
    evidence: &BTreeMap<(PackageId, PackageId), (Evidence, Evidence)>,
    containment: &PackageContainment,
) -> Vec<ChangeCoupling> {
    shared
        .into_iter()
        .filter(|((left, right), count)| *count >= 2 && !containment.is_nested(*left, *right))
        .map(|((left, right), count)| {
            let ((left_role, left_trust), (right_role, right_trust)) = evidence[&(left, right)];
            ChangeCoupling::new(left, right, count, touches[&left] + touches[&right] - count)
                .with_evidence(left_role, left_trust, right_role, right_trust)
        })
        .collect()
}

fn eligible_coupling_rows(
    shared: BTreeMap<(PackageId, PackageId), u32>,
    touches: &BTreeMap<PackageId, u32>,
    containment: &PackageContainment,
) -> Vec<ChangeCoupling> {
    shared
        .into_iter()
        .filter(|((left, right), count)| *count >= 2 && !containment.is_nested(*left, *right))
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
