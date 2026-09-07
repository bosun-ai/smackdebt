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

#[cfg(test)]
mod tests {

    use crate::diff::analyze_diff;
    use crate::requests::DiffRequest;
    use crate::test_support::git;
    use std::fs;

    #[test]
    fn diff_reports_a_new_material_core_from_the_existing_two_graphs() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}").unwrap();
        for index in 0..20 {
            fs::write(
                repository_path.join(format!("f{index}.ts")),
                format!("export const f{index} = {index};\n"),
            )
            .unwrap();
        }
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "initial"]);
        for index in 0..5 {
            let next = (index + 1) % 5;
            fs::write(
                repository_path.join(format!("f{index}.ts")),
                format!(
                    "import {{ f{next} }} from './f{next}.js';\nexport const f{index} = f{next} + 1;\n"
                ),
            )
            .unwrap();
        }

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let comparison = &result.report().core_comparisons()[0];
        assert_eq!(
            comparison.direction(),
            smackdebt_analysis::ComparisonDirection::Worse
        );
        assert_eq!(comparison.before(), (1, 20));
        assert_eq!(comparison.after(), (5, 20));
        assert_eq!(comparison.after_members().len(), 5);
    }
    #[test]
    fn diff_counts_a_core_candidate_before_incomplete_evidence_withholds_it() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}").unwrap();
        for index in 0..20 {
            fs::write(
                repository_path.join(format!("f{index}.ts")),
                format!("export const f{index} = {index};\n"),
            )
            .unwrap();
        }
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "initial"]);
        for index in 0..5 {
            let next = (index + 1) % 5;
            fs::write(
                repository_path.join(format!("f{index}.ts")),
                format!(
                    "import {{ f{next} }} from './f{next}.js';\nexport const f{index} = f{next} + 1;\n"
                ),
            )
            .unwrap();
        }
        fs::write(
            repository_path.join("f5.ts"),
            "import missing from './missing.js';\nexport const f5 = missing;\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let evidence = result.report().diff_graph_evidence().unwrap();
        assert_eq!(evidence.core().total(), 1);
        assert_eq!(evidence.core().current(), 1);
        assert_eq!(evidence.core().base(), 0);
        assert!(result.report().core_comparisons().is_empty());
    }
    #[test]
    fn diff_does_not_compare_disjoint_largest_dependency_cycles() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}").unwrap();
        for index in 0..20 {
            fs::write(
                repository_path.join(format!("f{index}.ts")),
                format!("export const f{index} = {index};\n"),
            )
            .unwrap();
        }
        for index in 0..5 {
            let next = (index + 1) % 5;
            fs::write(
                repository_path.join(format!("f{index}.ts")),
                format!(
                    "import {{ f{next} }} from './f{next}.js';\nexport const f{index} = f{next};\n"
                ),
            )
            .unwrap();
        }
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "first core"]);
        for index in 0..5 {
            fs::write(
                repository_path.join(format!("f{index}.ts")),
                format!("export const f{index} = {index};\n"),
            )
            .unwrap();
        }
        for index in 5..11 {
            let next = if index == 10 { 5 } else { index + 1 };
            fs::write(
                repository_path.join(format!("f{index}.ts")),
                format!(
                    "import {{ f{next} }} from './f{next}.js';\nexport const f{index} = f{next};\n"
                ),
            )
            .unwrap();
        }

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let rows = result.report().core_comparisons();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| {
            row.before_members().contains(&row.anchor())
                && row.after_members().contains(&row.anchor())
        }));
        assert!(
            rows.iter()
                .any(|row| row.before() == (5, 20) && row.after() == (1, 20))
        );
        assert!(
            rows.iter()
                .any(|row| row.before() == (1, 20) && row.after() == (6, 20))
        );
        assert!(
            !rows
                .iter()
                .any(|row| row.before() == (5, 20) && row.after() == (6, 20))
        );
    }
}
