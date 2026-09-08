//! Before/after dependency edge and cycle comparisons.
//!
//! Cycle matching retains a component when either witness still connects its
//! overlapping members. Edge membership changes remain separate from rated cycle
//! movement; comparison values retain the source evidence needed to explain both.

#![deny(missing_docs)]

use crate::PackageId;
use crate::{ComparisonDirection, FileId, SourceRole, SourceTrust, StaticRelationKind};
use std::collections::BTreeSet;

/// A cyclic component and one stable directed witness whose last package is
/// the first package again.
pub type PackageCycle = (Vec<PackageId>, Vec<PackageId>);

/// Compares edge membership and cyclic components while preserving valid overlapping witnesses.
pub fn compare_architecture(
    before_edges: &[(PackageId, PackageId)],
    after_edges: &[(PackageId, PackageId)],
    before_cycles: &[PackageCycle],
    after_cycles: &[PackageCycle],
) -> Vec<ArchitectureComparison> {
    let before: BTreeSet<_> = before_edges.iter().copied().collect();
    let after: BTreeSet<_> = after_edges.iter().copied().collect();
    let mut facts = Vec::new();

    // A component that grows or shrinks is still the same finding when it
    // shares cyclic packages. This prevents one retained cycle from appearing
    // as both an improvement and a regression.
    for (packages, witness) in after_cycles {
        if !before_cycles.iter().any(|(old, old_witness)| {
            intersects(old, packages)
                && (witness_exists(witness, &before) || witness_exists(old_witness, &after))
        }) {
            facts.push((
                ArchitectureComparisonKind::CycleIntroduced,
                packages.clone(),
                witness.clone(),
            ));
        }
    }
    for (packages, witness) in before_cycles {
        if !after_cycles.iter().any(|(new, new_witness)| {
            intersects(new, packages)
                && (witness_exists(witness, &after) || witness_exists(new_witness, &before))
        }) {
            facts.push((
                ArchitectureComparisonKind::CycleRemoved,
                packages.clone(),
                witness.clone(),
            ));
        }
    }
    for &(source, target) in after.difference(&before) {
        facts.push((
            ArchitectureComparisonKind::EdgeAdded,
            vec![source, target],
            Vec::new(),
        ));
    }
    for &(source, target) in before.difference(&after) {
        facts.push((
            ArchitectureComparisonKind::EdgeRemoved,
            vec![source, target],
            Vec::new(),
        ));
    }
    facts
        .into_iter()
        .enumerate()
        .map(|(index, (kind, packages, witness))| {
            ArchitectureComparison::new(ArchitectureComparisonId::from_index(index), kind, packages)
                .with_witness(witness)
        })
        .collect()
}

fn intersects(left: &[PackageId], right: &[PackageId]) -> bool {
    left.iter().any(|package| right.contains(package))
}

fn witness_exists(witness: &[PackageId], edges: &BTreeSet<(PackageId, PackageId)>) -> bool {
    witness.len() > 1
        && witness
            .windows(2)
            .all(|step| edges.contains(&(step[0], step[1])))
}

/// The dependency edge or cycle change described by a comparison.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchitectureComparisonKind {
    /// A dependency edge is present only after the change.
    EdgeAdded,
    /// A dependency edge is present only before the change.
    EdgeRemoved,
    /// A cyclic component has no retained witness before the change.
    CycleIntroduced,
    /// A cyclic component has no retained witness after the change.
    CycleRemoved,
}

/// Dependency movement with optional file, relation, and reference evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchitectureComparison {
    id: ArchitectureComparisonId,
    kind: ArchitectureComparisonKind,
    direction: ComparisonDirection,
    packages: Vec<PackageId>,
    witness: Vec<PackageId>,
    files: Vec<FileId>,
    relation: Option<StaticRelationKind>,
    role: Option<SourceRole>,
    trust: Option<SourceTrust>,
    before_references: Option<u32>,
    after_references: Option<u32>,
}

impl ArchitectureComparison {
    /// Retains an edge or cycle change; cycle changes set debt direction while edge changes remain neutral.
    pub fn new(
        id: ArchitectureComparisonId,
        kind: ArchitectureComparisonKind,
        packages: Vec<PackageId>,
    ) -> Self {
        let direction = match kind {
            ArchitectureComparisonKind::CycleIntroduced => ComparisonDirection::Worse,
            ArchitectureComparisonKind::CycleRemoved => ComparisonDirection::Better,
            ArchitectureComparisonKind::EdgeAdded | ArchitectureComparisonKind::EdgeRemoved => {
                ComparisonDirection::Changed
            }
        };
        Self {
            id,
            kind,
            direction,
            packages,
            witness: Vec::new(),
            files: Vec::new(),
            relation: None,
            role: None,
            trust: None,
            before_references: None,
            after_references: None,
        }
    }
    /// Attaches the closed directed package path that witnesses the cycle.
    pub fn with_witness(mut self, witness: Vec<PackageId>) -> Self {
        self.witness = witness;
        self
    }
    /// Attaches the file identities affected by the dependency movement.
    pub fn with_files(mut self, files: Vec<FileId>) -> Self {
        self.files = files;
        self
    }
    /// Attaches the dependency relation, source role, and trust for the changed edge.
    pub fn with_relation_evidence(
        mut self,
        relation: StaticRelationKind,
        role: SourceRole,
        trust: SourceTrust,
    ) -> Self {
        self.relation = Some(relation);
        self.role = Some(role);
        self.trust = Some(trust);
        self
    }
    /// Retains the exact before and after reference counts for the relation.
    pub fn with_reference_counts(mut self, before: u32, after: u32) -> Self {
        self.before_references = Some(before);
        self.after_references = Some(after);
        self
    }
    /// The row's typed position in its owning report table.
    pub const fn id(&self) -> ArchitectureComparisonId {
        self.id
    }
    /// The finding or movement category represented by this row.
    pub const fn kind(&self) -> ArchitectureComparisonKind {
        self.kind
    }
    /// Whether the comparison represents worse, better, or neutral debt movement.
    pub const fn direction(&self) -> ComparisonDirection {
        self.direction
    }
    /// The package identities participating in this observation.
    pub fn packages(&self) -> &[PackageId] {
        &self.packages
    }
    /// The closed directed package path retained as cycle evidence.
    pub fn witness(&self) -> &[PackageId] {
        &self.witness
    }
    /// The file identities participating in this observation.
    pub fn files(&self) -> &[FileId] {
        &self.files
    }
    /// The dependency relation kind when this row describes an edge.
    pub const fn relation(&self) -> Option<StaticRelationKind> {
        self.relation
    }
    /// The source role of the evidence behind this observation.
    pub const fn role(&self) -> Option<SourceRole> {
        self.role
    }
    /// Whether the source facts behind this observation are trusted or advisory.
    pub const fn trust(&self) -> Option<SourceTrust> {
        self.trust
    }
    /// The reference count before the change, when supplied.
    pub const fn before_references(&self) -> Option<u32> {
        self.before_references
    }
    /// The reference count after the change, when supplied.
    pub const fn after_references(&self) -> Option<u32> {
        self.after_references
    }
}

crate::table_index::table_index!(
    /// The position of one architecture comparison in its report table.
    ArchitectureComparisonId
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ComparisonDirection;

    #[test]
    fn introduced_and_removed_cycles_keep_closed_directed_witnesses() {
        let a = PackageId::from_index(0);
        let b = PackageId::from_index(1);
        let introduced = compare_architecture(&[], &[], &[], &[(vec![a, b], vec![a, b, a])]);
        assert_eq!(introduced[0].direction(), ComparisonDirection::Worse);
        assert_eq!(introduced[0].witness(), &[a, b, a]);
        let removed = compare_architecture(&[], &[], &[(vec![a, b], vec![a, b, a])], &[]);
        assert_eq!(removed[0].direction(), ComparisonDirection::Better);
        assert_eq!(removed[0].witness(), &[a, b, a]);
    }

    #[test]
    fn retained_cycle_with_a_changed_component_is_not_better_and_worse() {
        let a = PackageId::from_index(0);
        let b = PackageId::from_index(1);
        let c = PackageId::from_index(2);
        let values = compare_architecture(
            &[(a, b), (b, a)],
            &[(a, b), (b, a), (b, c), (c, a)],
            &[(vec![a, b], vec![a, b, a])],
            &[(vec![a, b, c], vec![a, b, c, a])],
        );
        assert!(values.iter().all(|value| matches!(
            value.kind(),
            ArchitectureComparisonKind::EdgeAdded | ArchitectureComparisonKind::EdgeRemoved
        )));
    }
}
