//! The cycle and relation comparisons the two graphs disagree on.

use std::collections::BTreeSet;

use smackdebt_analysis::{
    ArchitectureComparison, ArchitectureComparisonId, ArchitectureFindingKind, DependencyEdge,
    FileId, FileRecord, PackageId, SourceRole, SourceTrust, compare_architecture,
};
use std::collections::BTreeMap;

use crate::architecture::ArchitectureBuild;
use crate::dependencies::DiffTables;
use crate::diff_graphs::DiffArchitectures;
use crate::work::AnalysisWork;

/// The cycle and relation comparisons the two graphs disagree on.
pub(crate) fn compare_diff_architecture(
    architectures: &DiffArchitectures,
    tables: &DiffTables,
    work: &AnalysisWork,
) -> Vec<ArchitectureComparison> {
    let before_edges: Vec<_> = architectures
        .before
        .package_edges
        .iter()
        .map(|edge| (edge.source(), edge.target()))
        .collect();
    let current_edges: Vec<_> = architectures
        .current
        .package_edges
        .iter()
        .map(|edge| (edge.source(), edge.target()))
        .collect();
    work.record_algorithm_pass();
    let mut comparisons = compare_architecture(
        &before_edges,
        &current_edges,
        &architectures.before.cycles,
        &architectures.current.cycles,
    );
    comparisons.retain(|comparison| {
        matches!(
            comparison.kind(),
            smackdebt_analysis::ArchitectureComparisonKind::CycleIntroduced
                | smackdebt_analysis::ArchitectureComparisonKind::CycleRemoved
        )
    });
    append_relation_comparisons(
        &mut comparisons,
        &architectures.before.file_edges,
        &architectures.current.file_edges,
        &tables.before.files,
        &tables.current.files,
    );
    comparisons
}
/// Attaches the file evidence each comparison's explaining side holds.
pub(crate) fn attribute_comparison_files(
    comparisons: &mut [ArchitectureComparison],
    architectures: &DiffArchitectures,
) {
    for comparison in comparisons.iter_mut() {
        if comparison.relation().is_some() {
            continue;
        }
        let source = comparison_explaining_side(architectures, comparison.kind());
        let files = comparison_files(source, comparison);
        *comparison = comparison.clone().with_files(files);
    }
}
/// The side whose graph explains one comparison kind: removals answer from
/// the base tree, everything else from the current tree.
pub(crate) fn comparison_explaining_side(
    architectures: &DiffArchitectures,
    kind: smackdebt_analysis::ArchitectureComparisonKind,
) -> &ArchitectureBuild {
    if kind == smackdebt_analysis::ArchitectureComparisonKind::CycleRemoved
        || kind == smackdebt_analysis::ArchitectureComparisonKind::EdgeRemoved
    {
        &architectures.before
    } else {
        &architectures.current
    }
}
/// The distinct files one comparison's explaining side attributes to it.
pub(crate) fn comparison_files(
    source: &ArchitectureBuild,
    comparison: &ArchitectureComparison,
) -> Vec<FileId> {
    let files: BTreeSet<_> = match comparison.kind() {
        smackdebt_analysis::ArchitectureComparisonKind::EdgeAdded
        | smackdebt_analysis::ArchitectureComparisonKind::EdgeRemoved => source
            .package_edges
            .iter()
            .filter(|edge| comparison.packages() == [edge.source(), edge.target()])
            .flat_map(|edge| {
                edge.file_edges().iter().flat_map(|id| {
                    let edge = &source.file_edges[id.index()];
                    [edge.source(), edge.target()]
                })
            })
            .collect(),
        _ => source
            .findings
            .iter()
            .filter(|finding| {
                finding.kind() == ArchitectureFindingKind::PackageCycle
                    && finding
                        .packages()
                        .iter()
                        .any(|package| comparison.packages().contains(package))
            })
            .flat_map(|finding| finding.files().iter().copied())
            .collect(),
    };
    files.into_iter().collect()
}
pub(crate) fn append_relation_comparisons(
    comparisons: &mut Vec<ArchitectureComparison>,
    before_edges: &[DependencyEdge],
    current_edges: &[DependencyEdge],
    before_files: &[FileRecord],
    current_files: &[FileRecord],
) {
    type RelationKey = (
        PackageId,
        PackageId,
        smackdebt_analysis::StaticRelationKind,
        SourceRole,
        SourceTrust,
    );
    pub(crate) fn relations_by_evidence(
        edges: &[DependencyEdge],
        files: &[FileRecord],
    ) -> BTreeMap<RelationKey, (u32, (FileId, FileId))> {
        let mut values: BTreeMap<RelationKey, (u32, (FileId, FileId))> = BTreeMap::new();
        for edge in edges {
            let Some(source_package) = files[edge.source().index()].package() else {
                continue;
            };
            let Some(target_package) = files[edge.target().index()].package() else {
                continue;
            };
            values
                .entry((
                    source_package,
                    target_package,
                    edge.relation(),
                    edge.role(),
                    edge.trust(),
                ))
                .and_modify(|value| {
                    value.0 += edge.references();
                    value.1 = (edge.source(), edge.target());
                })
                .or_insert((edge.references(), (edge.source(), edge.target())));
        }
        values
    }
    let before = relations_by_evidence(before_edges, before_files);
    let current = relations_by_evidence(current_edges, current_files);
    let keys: std::collections::BTreeSet<_> =
        before.keys().chain(current.keys()).copied().collect();
    for (source_package, target_package, relation, role, trust) in keys {
        let key = (source_package, target_package, relation, role, trust);
        let before_references = before.get(&key).map_or(0, |value| value.0);
        let after_references = current.get(&key).map_or(0, |value| value.0);
        if before_references == after_references {
            continue;
        }
        let kind = if after_references > before_references {
            smackdebt_analysis::ArchitectureComparisonKind::EdgeAdded
        } else {
            smackdebt_analysis::ArchitectureComparisonKind::EdgeRemoved
        };
        let files = current
            .get(&key)
            .or_else(|| before.get(&key))
            .expect("changed relation has file evidence")
            .1;
        let id = ArchitectureComparisonId::from_index(comparisons.len());
        let packages = if source_package == target_package {
            vec![source_package]
        } else {
            vec![source_package, target_package]
        };
        comparisons.push(
            ArchitectureComparison::new(id, kind, packages)
                .with_files(vec![files.0, files.1])
                .with_relation_evidence(relation, role, trust)
                .with_reference_counts(before_references, after_references),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::analyze_diff;
    use crate::requests::DiffRequest;
    use crate::test_support::git;
    use std::fs;

    #[test]
    fn rust_relation_diffs_keep_kind_role_and_trust_as_independent_evidence() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(
            repository_path.join("Cargo.toml"),
            "[package]\nname='relations'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(repository_path.join("child.rs"), "pub fn work() {}\n").unwrap();
        fs::write(
            repository_path.join("main.rs"),
            "use crate::child::work;\nfn main() { work(); }\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "base use"]);
        fs::write(
            repository_path.join("main.rs"),
            "mod child;\nuse crate::child::work;\nfn main() { work(); }\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "add ownership"]);

        let clean =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD~1")).unwrap();
        let ownership = clean
            .report()
            .architecture_comparisons()
            .iter()
            .find(|comparison| {
                comparison.kind() == smackdebt_analysis::ArchitectureComparisonKind::EdgeAdded
                    && comparison.relation()
                        == Some(smackdebt_analysis::StaticRelationKind::ModuleOwnership)
            })
            .expect("clean ref diff retains ownership evidence");
        assert_eq!(ownership.role(), Some(SourceRole::Primary));
        assert_eq!(ownership.trust(), Some(SourceTrust::Trusted));
        assert_eq!(
            ownership.direction(),
            smackdebt_analysis::ComparisonDirection::Changed
        );
        assert!(
            clean
                .report()
                .architecture_comparisons()
                .iter()
                .all(|comparison| {
                    comparison.direction() != smackdebt_analysis::ComparisonDirection::Worse
                })
        );

        fs::write(
            repository_path.join("main.rs"),
            "mod child;\nuse crate::child::work;\nfn main() { work(); broken( }\n",
        )
        .unwrap();
        let worktree =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        assert!(
            worktree
                .report()
                .architecture_comparisons()
                .iter()
                .any(|comparison| {
                    comparison.relation() == Some(smackdebt_analysis::StaticRelationKind::Uses)
                        && comparison.kind()
                            == smackdebt_analysis::ArchitectureComparisonKind::EdgeAdded
                        && comparison.role() == Some(SourceRole::Primary)
                        && comparison.trust() == Some(SourceTrust::Advisory)
                })
        );
        assert!(
            worktree
                .report()
                .architecture_comparisons()
                .iter()
                .any(|comparison| {
                    comparison.relation() == Some(smackdebt_analysis::StaticRelationKind::Uses)
                        && comparison.kind()
                            == smackdebt_analysis::ArchitectureComparisonKind::EdgeRemoved
                        && comparison.role() == Some(SourceRole::Primary)
                        && comparison.trust() == Some(SourceTrust::Trusted)
                })
        );

        fs::create_dir_all(repository_path.join("tests")).unwrap();
        fs::write(
            repository_path.join("tests/main.rs"),
            "use crate::child::work;\nfn main() { work(); }\n",
        )
        .unwrap();
        fs::remove_file(repository_path.join("main.rs")).unwrap();
        let role_change =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        assert!(
            role_change
                .report()
                .architecture_comparisons()
                .iter()
                .any(|comparison| {
                    comparison.relation() == Some(smackdebt_analysis::StaticRelationKind::Uses)
                        && comparison.kind()
                            == smackdebt_analysis::ArchitectureComparisonKind::EdgeAdded
                        && comparison.role() == Some(SourceRole::Test)
                        && comparison.trust() == Some(SourceTrust::Trusted)
                })
        );
        assert!(
            role_change
                .report()
                .architecture_comparisons()
                .iter()
                .any(|comparison| {
                    comparison.relation() == Some(smackdebt_analysis::StaticRelationKind::Uses)
                        && comparison.kind()
                            == smackdebt_analysis::ArchitectureComparisonKind::EdgeRemoved
                        && comparison.role() == Some(SourceRole::Primary)
                        && comparison.trust() == Some(SourceTrust::Trusted)
                })
        );
    }
    #[test]
    fn relation_diff_retains_reference_count_changes_for_the_same_file_pair() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}").unwrap();
        fs::write(
            repository_path.join("main.js"),
            "import value from './value';\nfunction main() { return value(); }\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("value.js"),
            "export default function value() {}\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "one reference"]);
        fs::write(
            repository_path.join("main.js"),
            "import value from './value';\nimport second from './value';\nfunction main() { value(); second(); }\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let comparison = result
            .report()
            .architecture_comparisons()
            .iter()
            .find(|comparison| {
                comparison.relation() == Some(smackdebt_analysis::StaticRelationKind::Uses)
                    && comparison.before_references() == Some(1)
                    && comparison.after_references() == Some(2)
            })
            .expect("reference count change is retained");
        assert_eq!(
            comparison.direction(),
            smackdebt_analysis::ComparisonDirection::Changed
        );
        assert_eq!(result.report().dependency_edges()[0].references(), 2);
    }
    #[test]
    fn added_package_manifests_change_package_cycle_identity_between_sides() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}").unwrap();
        for package in ["app", "core"] {
            fs::create_dir_all(repository_path.join(package)).unwrap();
        }
        fs::write(
            repository_path.join("app/a.js"),
            "import core from '../core/b';\nfunction app() {}\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("core/b.js"),
            "import app from '../app/a';\nfunction core() {}\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "initial"]);
        for package in ["app", "core"] {
            fs::write(repository_path.join(package).join("package.json"), "{}").unwrap();
        }
        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let cycle = result
            .report()
            .architecture_comparisons()
            .iter()
            .find(|value| {
                value.kind() == smackdebt_analysis::ArchitectureComparisonKind::CycleIntroduced
            })
            .unwrap();
        assert_eq!(cycle.witness().first(), cycle.witness().last());
    }
    #[test]
    fn diff_applies_rename_identity_before_architecture_comparison() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}").unwrap();
        fs::write(
            repository_path.join("main.js"),
            "import value from './old';\nfunction main() { return value; }\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("old.js"),
            "export default function value() {}\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "initial"]);
        fs::rename(
            repository_path.join("old.js"),
            repository_path.join("new.js"),
        )
        .unwrap();
        fs::write(
            repository_path.join("main.js"),
            "import value from './new';\nfunction main() { return value; }\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        assert!(
            result.report().architecture_comparisons().is_empty(),
            "{:?}",
            result.report().architecture_comparisons()
        );
        assert_eq!(result.report().dependency_edges().len(), 1);
    }
}
