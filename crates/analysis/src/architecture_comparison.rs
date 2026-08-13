use crate::{
    ArchitectureComparison, ArchitectureComparisonId, ArchitectureComparisonKind, PackageId,
};
use std::collections::BTreeSet;

/// A cyclic component and one stable directed witness whose last package is
/// the first package again.
pub type PackageCycle = (Vec<PackageId>, Vec<PackageId>);

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
