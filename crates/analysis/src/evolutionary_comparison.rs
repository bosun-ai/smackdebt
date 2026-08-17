use crate::change_coupling::qualifies_for_finding;
use crate::evolution::pair_is_explained;
use crate::{
    ChangeCoupling, ComparisonDirection, EvolutionaryComparison, EvolutionaryComparisonId,
    EvolutionaryComparisonKind, PackageId,
};
use std::collections::BTreeSet;

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
