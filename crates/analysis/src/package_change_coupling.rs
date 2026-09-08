//! Package change coupling: packages repeatedly changed in the same commits.
//!
//! Also called temporal or logical coupling. Similarity is the Jaccard ratio:
//! shared commits divided by commits touching either package (the union). Each
//! package pair is counted once per commit. A package and its own nested package
//! are excluded. Descriptive rows need two shared commits; a Watch finding needs
//! at least three and a similarity of at least 20%, without a direct explaining
//! code dependency in either direction. The finding test uses integer products.
//!
//! Descriptive counts retain all roles; finding counts use trusted verdict roles.
//! History composition suppresses findings and comparisons when coverage cannot
//! support them. Comparisons hold coupling fixed and observe changes to its code
//! dependency explanation. A transitive presentation link does not explain it.
//!
//! ```
//! use smackdebt_analysis::{ChangeCoupling, PackageId, qualifies_for_finding};
//! let pair = ChangeCoupling::new(PackageId::from_index(0), PackageId::from_index(1), 3, 15);
//! assert!(qualifies_for_finding(pair)); // Exactly three shared commits and 20%.
//! ```
//!
//! Terminology: [CodeScene change coupling](https://www.codescene.io/docs/configuration/project-config/index.html).

#![deny(missing_docs)]

use crate::{ComparisonDirection, EvolutionaryFindingKind};
use crate::{HistoryCommitFact, PackageId, SourceRole, SourceTrust};
use std::collections::{BTreeMap, BTreeSet};

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
    ancestor == "."
        || descendant
            .strip_prefix(ancestor)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Accumulates descriptive package-pair counts, excluding ancestor/descendant pairs.
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

/// Selects sufficiently strong package pairs without a direct explaining dependency.
pub fn unexplained_coupling(
    coupling: &[ChangeCoupling],
    explanation_pairs: &BTreeSet<(PackageId, PackageId)>,
) -> Vec<EvolutionaryFinding> {
    coupling
        .iter()
        .filter(|pair| qualifies_for_finding(**pair))
        .filter(|pair| !pair_is_explained(explanation_pairs, pair.left(), pair.right()))
        .enumerate()
        .map(|(index, pair)| {
            EvolutionaryFinding::new(EvolutionaryFindingId::from_index(index), *pair)
        })
        .collect()
}

/// Whether one coupling pair is strong enough to be reported at all.
///
/// The strength rule lives here so no consumer re-derives it: a report shows
/// weak coupling only in JSON.
pub fn qualifies_for_finding(pair: ChangeCoupling) -> bool {
    pair.shared_commits() >= 3
        && u64::from(pair.shared_commits()) * 5 >= u64::from(pair.union_commits())
}

/// A package pair with exact shared and union commit counts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChangeCoupling {
    left: PackageId,
    right: PackageId,
    shared_commits: u32,
    union_commits: u32,
    evidence: Option<CouplingEvidence>,
}

impl ChangeCoupling {
    /// Retains the package identities and shared/union commit counts without rating them.
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
    /// Attaches the observed source role and trust without changing the measurements.
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
    /// The first subject in the retained pair.
    pub const fn left(self) -> PackageId {
        self.left
    }
    /// The second subject in the retained pair.
    pub const fn right(self) -> PackageId {
        self.right
    }
    /// Distinct commits touching both subjects in this pair's history population.
    pub const fn shared_commits(self) -> u32 {
        self.shared_commits
    }
    /// Distinct commits touching either subject, counting shared commits once.
    pub const fn union_commits(self) -> u32 {
        self.union_commits
    }
    /// The shared/union commit ratio as a convenience float.
    ///
    /// Finding policy uses the integer operands. A zero union follows IEEE floating
    /// point division; this accessor does not validate supplied counts.
    pub fn similarity(self) -> f64 {
        f64::from(self.shared_commits) / f64::from(self.union_commits)
    }
    /// The source evidence retained with this observation.
    pub const fn evidence(self) -> Option<CouplingEvidence> {
        self.evidence
    }
}

/// The strongest observed source role and trust for each side of a package pair.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CouplingEvidence {
    left_role: SourceRole,
    left_trust: SourceTrust,
    right_role: SourceRole,
    right_trust: SourceTrust,
}

impl CouplingEvidence {
    /// The strongest observed source role of the first package.
    pub const fn left_role(self) -> SourceRole {
        self.left_role
    }
    /// The trust associated with the first package's selected role.
    pub const fn left_trust(self) -> SourceTrust {
        self.left_trust
    }
    /// The strongest observed source role of the second package.
    pub const fn right_role(self) -> SourceRole {
        self.right_role
    }
    /// The trust associated with the second package's selected role.
    pub const fn right_trust(self) -> SourceTrust {
        self.right_trust
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

/// One Watch observation of packages that change together without a code
/// dependency.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvolutionaryFinding {
    id: EvolutionaryFindingId,
    coupling: ChangeCoupling,
}

impl EvolutionaryFinding {
    /// Records a pair already selected by the unexplained-coupling rule.
    pub const fn new(id: EvolutionaryFindingId, coupling: ChangeCoupling) -> Self {
        Self { id, coupling }
    }
    /// The row's typed position in its owning report table.
    pub const fn id(self) -> EvolutionaryFindingId {
        self.id
    }
    /// The finding or movement category represented by this row.
    pub const fn kind(self) -> EvolutionaryFindingKind {
        EvolutionaryFindingKind::UnexplainedCoupling
    }
    /// The health rating carried by this observation.
    pub const fn rating(self) -> crate::Rating {
        crate::Rating::Watch
    }
    /// The exact package-pair counts supporting this row.
    pub const fn coupling(self) -> ChangeCoupling {
        self.coupling
    }
}

/// Whether a package coupling finding was introduced or removed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvolutionaryComparisonKind {
    /// The change introduces an unexplained-coupling finding.
    FindingIntroduced,
    /// The change removes an unexplained-coupling finding.
    FindingRemoved,
}

/// A change to the code dependency that explains a package coupling pair.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvolutionaryComparison {
    id: EvolutionaryComparisonId,
    kind: EvolutionaryComparisonKind,
    direction: ComparisonDirection,
    coupling: ChangeCoupling,
}

impl EvolutionaryComparison {
    /// Retains a classified explanation change and the coupling operands it concerns.
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
    /// The row's typed position in its owning report table.
    pub const fn id(self) -> EvolutionaryComparisonId {
        self.id
    }
    /// The finding or movement category represented by this row.
    pub const fn kind(self) -> EvolutionaryComparisonKind {
        self.kind
    }
    /// Whether the comparison represents worse, better, or neutral debt movement.
    pub const fn direction(self) -> ComparisonDirection {
        self.direction
    }
    /// The exact package-pair counts supporting this row.
    pub const fn coupling(self) -> ChangeCoupling {
        self.coupling
    }
}

crate::table_index::table_index!(
    /// The position of one evolutionary finding in its report table.
    EvolutionaryFindingId
);

crate::table_index::table_index!(
    /// The position of one evolutionary comparison in its report table.
    EvolutionaryComparisonId
);

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

/// Compares the direct dependency explanations for retained, qualifying coupling pairs.
///
/// Losing an explanation introduces debt; adding one removes it. The coupling
/// counts themselves are shared by both sides.
pub fn compare_evolution(
    coupling: &[ChangeCoupling],
    before: &BTreeSet<(PackageId, PackageId)>,
    after: &BTreeSet<(PackageId, PackageId)>,
) -> Vec<EvolutionaryComparison> {
    let mut result = Vec::new();
    for pair in coupling.iter().filter(|pair| qualifies_for_finding(**pair)) {
        let before_explained = pair_is_explained(before, pair.left(), pair.right());
        let after_explained = pair_is_explained(after, pair.left(), pair.right());
        let value = match (before_explained, after_explained) {
            (true, false) => Some((
                EvolutionaryComparisonKind::FindingIntroduced,
                ComparisonDirection::Worse,
            )),
            (false, true) => Some((
                EvolutionaryComparisonKind::FindingRemoved,
                ComparisonDirection::Better,
            )),
            _ => None,
        };
        if let Some((kind, direction)) = value {
            result.push(EvolutionaryComparison::new(
                EvolutionaryComparisonId::from_index(result.len()),
                kind,
                direction,
                *pair,
            ));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use crate::{ContributorId, FileId, HistoryChangeFact};
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

    use super::*;

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
