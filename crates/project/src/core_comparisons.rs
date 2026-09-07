//! The core: whether the largest dependency cycle moved.

use smackdebt_analysis::{CoreSize, FileId};

use crate::architecture::ArchitectureBuild;
use crate::propagation_comparisons::fraction_direction;

pub(crate) fn core_comparisons(
    current: &ArchitectureBuild,
    before: &ArchitectureBuild,
) -> (
    Vec<smackdebt_analysis::CoreComparison>,
    smackdebt_analysis::ComparisonSuppression,
) {
    if current.core_size.is_none() && before.core_size.is_none() {
        return (
            Vec::new(),
            smackdebt_analysis::ComparisonSuppression::default(),
        );
    }
    let anchors = core_comparison_anchors(current, before);
    let mut comparisons = Vec::new();
    let mut suppression = smackdebt_analysis::ComparisonSuppression::default();
    for anchor in anchors {
        append_core_comparison(current, before, anchor, &mut comparisons, &mut suppression);
    }
    (comparisons, suppression)
}
pub(crate) fn core_comparison_anchors(
    current: &ArchitectureBuild,
    before: &ArchitectureBuild,
) -> Vec<FileId> {
    let shared = current
        .core_members
        .iter()
        .find(|file| before.core_members.contains(file))
        .copied();
    if let Some(anchor) = shared {
        return vec![anchor];
    }
    let mut anchors = Vec::new();
    if before.core_size.is_some()
        && let Some(anchor) = before
            .core_members
            .iter()
            .find(|file| graph_contains(current, **file))
    {
        anchors.push(*anchor);
    }
    if current.core_size.is_some()
        && let Some(anchor) = current
            .core_members
            .iter()
            .find(|file| graph_contains(before, **file))
    {
        anchors.push(*anchor);
    }
    anchors.sort_unstable();
    anchors.dedup();
    anchors
}
pub(crate) fn graph_contains(architecture: &ArchitectureBuild, file: FileId) -> bool {
    architecture
        .graph_packages
        .get(file.index())
        .is_some_and(Option::is_some)
}
pub(crate) fn component_containing(
    architecture: &ArchitectureBuild,
    anchor: FileId,
) -> Option<&[FileId]> {
    architecture
        .file_components
        .iter()
        .find(|component| component.contains(&anchor))
        .map(Vec::as_slice)
}
pub(crate) fn append_core_comparison(
    current: &ArchitectureBuild,
    before: &ArchitectureBuild,
    anchor: FileId,
    comparisons: &mut Vec<smackdebt_analysis::CoreComparison>,
    suppression: &mut smackdebt_analysis::ComparisonSuppression,
) {
    let (Some(current_members), Some(before_members)) = (
        component_containing(current, anchor),
        component_containing(before, anchor),
    ) else {
        return;
    };
    let before_counts = (before_members.len() as u32, before.file_graph_count);
    let after_counts = (current_members.len() as u32, current.file_graph_count);
    let material = CoreSize::from_counts(before_counts.0, before_counts.1).is_some()
        || CoreSize::from_counts(after_counts.0, after_counts.1).is_some();
    if !material || (before_counts == after_counts && before_members == current_members) {
        return;
    }
    let current_incomplete = !current.graph_evidence.is_complete();
    let base_incomplete = !before.graph_evidence.is_complete();
    if current_incomplete || base_incomplete {
        suppression.record(current_incomplete, base_incomplete);
        return;
    }
    let direction = fraction_direction(before_counts, after_counts)
        .unwrap_or(smackdebt_analysis::ComparisonDirection::Changed);
    comparisons.push(smackdebt_analysis::CoreComparison::new(
        smackdebt_analysis::CoreComparisonId::from_index(comparisons.len()),
        anchor,
        direction,
        (before_counts.0, before_counts.1, before_members.to_vec()),
        (after_counts.0, after_counts.1, current_members.to_vec()),
    ));
}
