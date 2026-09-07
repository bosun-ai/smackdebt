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
