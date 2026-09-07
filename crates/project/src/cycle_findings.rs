//! Cycle findings over both graphs: the file cycle graph, its package-level
//! counterpart, and the witnesses each finding carries.

use std::collections::BTreeSet;

use smackdebt_analysis::{
    ArchitectureFinding, ArchitectureFindingId, ArchitectureFindingKind, DependencyEdge, FileId,
    FileRecord, PackageId, ScopeId, cycle_witness, strongly_connected_components,
};

use crate::package_graph::PackageGraph;

/// The cycle findings one build accumulates. File-cycle identities continue
/// the package-cycle numbering, so both loops share one table.
#[derive(Default)]
pub(crate) struct ArchitectureFindings {
    pub(crate) findings: Vec<ArchitectureFinding>,
    pub(crate) links: Vec<(ScopeId, ArchitectureFindingId)>,
    pub(crate) cycles: Vec<smackdebt_analysis::PackageCycle>,
}
impl ArchitectureFindings {
    /// Records one finding, witness, and scope links per package cycle.
    pub(crate) fn record_package_cycles(
        &mut self,
        graph: &PackageGraph,
        files: &[FileRecord],
        file_edges: &[DependencyEdge],
    ) {
        for component in strongly_connected_components(graph.count, &graph.pairs)
            .into_iter()
            .filter(|component| component.len() > 1)
        {
            let witness = cycle_witness(&component, &graph.pairs).unwrap_or_default();
            let witness_edges: Vec<_> = witness
                .iter()
                .filter_map(|&(source, target)| {
                    graph.edges.iter().find(|edge| {
                        edge.source().index() == source && edge.target().index() == target
                    })
                })
                .flat_map(|edge| edge.file_edges().iter().copied().take(1))
                .collect();
            let involved_files: Vec<_> = witness_edges
                .iter()
                .flat_map(|id| {
                    let edge = &file_edges[id.index()];
                    [edge.source(), edge.target()]
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let packages: Vec<_> = component.into_iter().map(PackageId::from_index).collect();
            let mut package_witness: Vec<_> = witness
                .iter()
                .map(|(source, _)| PackageId::from_index(*source))
                .collect();
            if let Some((_, target)) = witness.last() {
                package_witness.push(PackageId::from_index(*target));
            }
            self.cycles.push((packages.clone(), package_witness));
            let id = ArchitectureFindingId::from_index(self.findings.len());
            for package in &packages {
                if let Some(scope) = files
                    .iter()
                    .find(|file| file.package() == Some(*package))
                    .map(FileRecord::scope)
                {
                    self.links.push((scope, id));
                }
            }
            self.findings.push(ArchitectureFinding::new(
                id,
                ArchitectureFindingKind::PackageCycle,
                packages,
                involved_files,
                witness_edges,
            ));
        }
    }

    /// Records one finding and scope links per single-package file cycle.
    pub(crate) fn record_file_cycles(
        &mut self,
        graph: &FileCycleGraph,
        files: &[FileRecord],
        file_edges: &[DependencyEdge],
    ) {
        for component in graph
            .components
            .iter()
            .filter(|component| component.len() > 1)
        {
            let packages: BTreeSet<_> = component
                .iter()
                .filter_map(|file| files[*file].package())
                .collect();
            if packages.len() != 1 {
                continue;
            }
            let witness = cycle_witness(component, &graph.pairs).unwrap_or_default();
            let witness_edges: Vec<_> = witness
                .iter()
                .filter_map(|&(source, target)| {
                    file_edges
                        .iter()
                        .find(|edge| {
                            enters_cycle_graph(edge, &graph.ownership_pairs)
                                && edge.source().index() == source
                                && edge.target().index() == target
                        })
                        .map(DependencyEdge::id)
                })
                .collect();
            let id = ArchitectureFindingId::from_index(self.findings.len());
            for file in component {
                self.links.push((files[*file].scope(), id));
            }
            self.findings.push(ArchitectureFinding::new(
                id,
                ArchitectureFindingKind::FileCycle,
                packages.into_iter().collect(),
                component.iter().copied().map(FileId::from_index).collect(),
                witness_edges,
            ));
        }
    }
}
/// The file cycle graph: the ownership pairs it drops, the pairs that enter
/// it, and the components those pairs form.
pub(crate) struct FileCycleGraph {
    pub(crate) ownership_pairs: BTreeSet<(usize, usize)>,
    pub(crate) pairs: Vec<(usize, usize)>,
    pub(crate) components: Vec<Vec<usize>>,
}
impl FileCycleGraph {
    pub(crate) fn of(file_edges: &[DependencyEdge], file_count: usize) -> Self {
        // A Rust `mod` declaration and the imports that accompany it are one
        // wiring relationship rather than a cycle, so the cycle graph drops the
        // uses between an owning pair. The exclusion is pairwise: every other
        // relation of the same component stays, and only this graph sees it.
        let ownership_pairs: BTreeSet<(usize, usize)> = file_edges
            .iter()
            .filter(|edge| {
                edge.relation() == smackdebt_analysis::StaticRelationKind::ModuleOwnership
            })
            .map(|edge| unordered_pair(edge.source().index(), edge.target().index()))
            .collect();
        let pairs: Vec<_> = file_edges
            .iter()
            .filter(|edge| enters_cycle_graph(edge, &ownership_pairs))
            .map(|edge| (edge.source().index(), edge.target().index()))
            .collect();
        // The components the cycle findings are made of are also the core and
        // the reach candidates, so they are retained rather than recomputed.
        let components = strongly_connected_components(file_count, &pairs);
        Self {
            ownership_pairs,
            pairs,
            components,
        }
    }
}
/// Whether one relation enters the file cycle graph.
pub(crate) fn enters_cycle_graph(
    edge: &DependencyEdge,
    ownership_pairs: &BTreeSet<(usize, usize)>,
) -> bool {
    edge.enters_verdict_graph()
        && !ownership_pairs.contains(&unordered_pair(
            edge.source().index(),
            edge.target().index(),
        ))
}
/// Orders one file pair so a relation and its reverse read as the same pair.
pub(crate) const fn unordered_pair(source: usize, target: usize) -> (usize, usize) {
    if source <= target {
        (source, target)
    } else {
        (target, source)
    }
}
