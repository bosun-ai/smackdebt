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

#[cfg(test)]
mod tests {

    use crate::diff::analyze_diff;
    use crate::requests::DiffRequest;
    use crate::test_support::git;
    use std::fs;

    #[test]
    fn propagation_compares_each_named_package_instead_of_unrelated_maxima() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        for package in ["a", "b", "c", "d"] {
            fs::create_dir_all(repository_path.join(package)).unwrap();
            fs::write(repository_path.join(package).join("package.json"), "{}").unwrap();
        }
        fs::write(repository_path.join("a/main.js"), "export default 1;\n").unwrap();
        for package in ["b", "c", "d"] {
            fs::write(
                repository_path.join(package).join("main.js"),
                "import value from '../a/main.js';\nexport default value;\n",
            )
            .unwrap();
        }
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "a reaches four"]);
        fs::write(repository_path.join("a/main.js"), "export default 1;\n").unwrap();
        fs::write(repository_path.join("b/main.js"), "export default 1;\n").unwrap();
        for package in ["c", "d"] {
            fs::write(
                repository_path.join(package).join("main.js"),
                "import value from '../b/main.js';\nexport default value;\n",
            )
            .unwrap();
        }

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let report = result.report();
        let package_id = |path: &str| {
            report
                .packages()
                .iter()
                .find(|package| package.path() == path)
                .unwrap()
                .id()
        };
        let a = package_id("a");
        let b = package_id("b");
        let package_rows: Vec<_> = report
            .propagation_comparisons()
            .iter()
            .filter_map(|comparison| match comparison.subject() {
                smackdebt_analysis::PropagationSubject::Package { source } => {
                    Some((source, comparison.before(), comparison.after()))
                }
                smackdebt_analysis::PropagationSubject::File { .. } => None,
            })
            .collect();
        assert!(package_rows.contains(&(a, (4, 4), (1, 4))));
        assert!(package_rows.contains(&(b, (1, 4), (3, 4))));
        assert!(!package_rows.contains(&(a, (4, 4), (3, 4))));
    }
    #[test]
    fn diff_reports_a_named_file_when_branch_reach_grows() {
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
        fs::write(
            repository_path.join("f1.ts"),
            "import { f0 } from './f0.js';\nexport const f1 = f0 + 1;\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let comparison = result
            .report()
            .propagation_comparisons()
            .iter()
            .find(|comparison| {
                matches!(
                    comparison.subject(),
                    smackdebt_analysis::PropagationSubject::File { .. }
                )
            })
            .expect("file reach movement is retained");
        assert_eq!(
            comparison.direction(),
            smackdebt_analysis::ComparisonDirection::Worse
        );
        assert_eq!(comparison.before(), (1, 20));
        assert_eq!(comparison.after(), (2, 20));
        assert_eq!(
            result
                .report()
                .scope_verdict(result.report().root().unwrap())
                .selection()
                .facts()
                .architecture()
                .worse(),
            1
        );
    }
    #[test]
    fn diff_withholds_reach_movement_when_only_current_graph_evidence_is_incomplete() {
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
        fs::write(
            repository_path.join("f1.ts"),
            "import { f0 } from './f0.js';\nimport missing from './missing.js';\nexport const f1 = f0 + missing;\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let evidence = result.report().diff_graph_evidence().unwrap();
        assert!(!evidence.current().is_complete());
        assert!(evidence.base().is_complete());
        assert_eq!(evidence.propagation().total(), 1);
        assert_eq!(evidence.propagation().current(), 1);
        assert_eq!(evidence.propagation().base(), 0);
        assert!(result.report().propagation_comparisons().is_empty());
        assert_eq!(
            result
                .report()
                .scope_verdict(result.report().root().unwrap())
                .selection()
                .facts()
                .architecture()
                .total(),
            0
        );
    }
    #[test]
    fn diff_withholds_reach_movement_when_only_base_graph_evidence_is_incomplete() {
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
        fs::write(
            repository_path.join("f1.ts"),
            "import { f0 } from './f0.js';\nimport missing from './missing.js';\nexport const f1 = f0 + missing;\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "initial"]);
        fs::write(repository_path.join("f1.ts"), "export const f1 = 1;\n").unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let evidence = result.report().diff_graph_evidence().unwrap();
        assert!(evidence.current().is_complete());
        assert!(!evidence.base().is_complete());
        assert_eq!(evidence.propagation().total(), 1);
        assert_eq!(evidence.propagation().current(), 0);
        assert_eq!(evidence.propagation().base(), 1);
        assert!(result.report().propagation_comparisons().is_empty());
    }
}
