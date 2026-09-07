//! How far change travels: the reach comparisons a diff states.

use std::path::PathBuf;

use smackdebt_analysis::{FileId, PackageId};

use crate::architecture::ArchitectureBuild;

pub(crate) fn fraction_direction(
    before: (u32, u32),
    after: (u32, u32),
) -> Option<smackdebt_analysis::ComparisonDirection> {
    if before == after {
        return None;
    }
    let before_cross = u64::from(before.0) * u64::from(after.1);
    let after_cross = u64::from(after.0) * u64::from(before.1);
    Some(match after_cross.cmp(&before_cross) {
        std::cmp::Ordering::Greater => smackdebt_analysis::ComparisonDirection::Worse,
        std::cmp::Ordering::Less => smackdebt_analysis::ComparisonDirection::Better,
        std::cmp::Ordering::Equal => smackdebt_analysis::ComparisonDirection::Changed,
    })
}
pub(crate) fn propagation_comparisons(
    current: &ArchitectureBuild,
    before: &ArchitectureBuild,
    current_roots: &[PathBuf],
    before_roots: &[PathBuf],
    package_roots: &[PathBuf],
) -> (
    Vec<smackdebt_analysis::PropagationComparison>,
    smackdebt_analysis::ComparisonSuppression,
) {
    let mut comparisons = Vec::new();
    let mut suppression = smackdebt_analysis::ComparisonSuppression::default();
    let mut package_subjects = [
        package_reach_subject(current, current_roots.len() as u32),
        package_reach_subject(before, before_roots.len() as u32),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    package_subjects.sort_unstable();
    package_subjects.dedup();
    for package in package_subjects {
        let root = &package_roots[package.index()];
        if !current_roots.contains(root) || !before_roots.contains(root) {
            continue;
        }
        let current_reach = current
            .measurements
            .iter()
            .find(|measurement| measurement.package() == package)
            .map(|measurement| measurement.reach_in());
        let before_reach = before
            .measurements
            .iter()
            .find(|measurement| measurement.package() == package)
            .map(|measurement| measurement.reach_in());
        let (Some(current_reach), Some(before_reach)) = (current_reach, before_reach) else {
            continue;
        };
        let before_counts = (before_reach, before_roots.len() as u32);
        let after_counts = (current_reach, current_roots.len() as u32);
        let material =
            smackdebt_analysis::PropagationReach::packages(before_counts.0, before_counts.1)
                .is_some()
                || smackdebt_analysis::PropagationReach::packages(after_counts.0, after_counts.1)
                    .is_some();
        let Some(direction) = material
            .then(|| fraction_direction(before_counts, after_counts))
            .flatten()
        else {
            continue;
        };
        let current_incomplete = !current.graph_evidence.is_complete();
        let base_incomplete = !before.graph_evidence.is_complete();
        if current_incomplete || base_incomplete {
            suppression.record(current_incomplete, base_incomplete);
        } else {
            comparisons.push(smackdebt_analysis::PropagationComparison::new(
                smackdebt_analysis::PropagationComparisonId::from_index(comparisons.len()),
                smackdebt_analysis::PropagationSubject::Package { source: package },
                direction,
                before_counts,
                after_counts,
            ));
        }
    }

    let mut file_subjects = current
        .package_closures
        .iter()
        .chain(&before.package_closures)
        .map(|closure| (closure.package(), closure.source()))
        .collect::<Vec<_>>();
    file_subjects.sort_unstable();
    file_subjects.dedup();
    for (package, source) in file_subjects {
        append_file_reach_comparison(
            FileReachComparisonInput {
                current,
                before,
                package,
                source,
            },
            &mut comparisons,
            &mut suppression,
        );
    }
    (comparisons, suppression)
}
pub(crate) fn package_reach_subject(
    architecture: &ArchitectureBuild,
    packages: u32,
) -> Option<PackageId> {
    let winner = architecture.measurements.iter().max_by(|left, right| {
        left.reach_in()
            .cmp(&right.reach_in())
            .then_with(|| right.package().cmp(&left.package()))
    })?;
    smackdebt_analysis::PropagationReach::packages(winner.reach_in(), packages)
        .map(|_| winner.package())
}
pub(crate) struct FileReachComparisonInput<'a> {
    pub(crate) current: &'a ArchitectureBuild,
    pub(crate) before: &'a ArchitectureBuild,
    pub(crate) package: PackageId,
    pub(crate) source: FileId,
}
pub(crate) fn append_file_reach_comparison(
    input: FileReachComparisonInput<'_>,
    comparisons: &mut Vec<smackdebt_analysis::PropagationComparison>,
    suppression: &mut smackdebt_analysis::ComparisonSuppression,
) {
    let value = |architecture: &ArchitectureBuild| {
        architecture
            .package_file_reach
            .iter()
            .find(|value| value.package() == input.package && value.source() == input.source)
            .copied()
    };
    let (Some(current_value), Some(before_value)) = (value(input.current), value(input.before))
    else {
        return;
    };
    let before_counts = (before_value.reach(), before_value.files());
    let after_counts = (current_value.reach(), current_value.files());
    let Some(direction) = fraction_direction(before_counts, after_counts) else {
        return;
    };
    let current_incomplete = !input
        .current
        .graph_evidence
        .package_is_complete(input.package);
    let base_incomplete = !input
        .before
        .graph_evidence
        .package_is_complete(input.package);
    if current_incomplete || base_incomplete {
        suppression.record(current_incomplete, base_incomplete);
        return;
    }
    comparisons.push(smackdebt_analysis::PropagationComparison::new(
        smackdebt_analysis::PropagationComparisonId::from_index(comparisons.len()),
        smackdebt_analysis::PropagationSubject::File {
            package: input.package,
            source: input.source,
        },
        direction,
        before_counts,
        after_counts,
    ));
}
