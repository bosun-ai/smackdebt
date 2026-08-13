use crate::evolution::has_static_edge;
use crate::{
    ChangeCoupling, ComparisonDirection, EvolutionaryComparison, EvolutionaryComparisonId,
    EvolutionaryComparisonKind, PackageEdge,
};

pub fn compare_evolution(
    coupling: &[ChangeCoupling],
    before: &[PackageEdge],
    after: &[PackageEdge],
) -> Vec<EvolutionaryComparison> {
    let mut result = Vec::new();
    for pair in coupling {
        let before_explained = has_static_edge(before, pair.left(), pair.right());
        let after_explained = has_static_edge(after, pair.left(), pair.right());
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
