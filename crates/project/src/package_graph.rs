//! Aggregating the file graph to package edges, coupling explanations, and
//! per-package measurements.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use smackdebt_analysis::dependency_degree;
use smackdebt_analysis::enters_file_graph;
use smackdebt_analysis::{
    DependencyEdge, DependencyEdgeId, FileId, FileRecord, PackageEdge, PackageEdgeId,
    PackageGraphMeasurement, PackageId, reach_in_counts,
};

use crate::dependencies::{PackageTables, SourceDependencies};
use crate::paths::package_of;
use crate::reference_tables::ManifestJoins;

/// The path one file-graph endpoint answers to, preferring the dependency
/// table's path over the record's.
pub(crate) fn edge_file_path<'a>(
    file: FileId,
    dependencies: &'a [SourceDependencies],
    files: &'a [FileRecord],
) -> &'a Path {
    dependencies
        .iter()
        .find(|source| source.file == file)
        .map(|source| source.path.as_path())
        .unwrap_or_else(|| Path::new(files[file.index()].path()))
}
/// The package-level graph the file edges and manifest joins aggregate to.
pub(crate) struct PackageGraph {
    pub(crate) edges: Vec<PackageEdge>,
    pub(crate) measurements: Vec<PackageGraphMeasurement>,
    pub(crate) explanation_pairs: BTreeSet<(PackageId, PackageId)>,
    pub(crate) pairs: Vec<(usize, usize)>,
    pub(crate) count: usize,
}
/// Aggregates the file edges to package edges, coupling explanations, and
/// per-package graph measurements.
pub(crate) fn package_graph(
    file_edges: &[DependencyEdge],
    files: &[FileRecord],
    dependencies: &[SourceDependencies],
    packages: PackageTables<'_>,
    manifest: ManifestJoins,
) -> PackageGraph {
    let mut package_values: BTreeMap<(PackageId, PackageId), (u32, u32, Vec<DependencyEdgeId>)> =
        BTreeMap::new();
    let mut explanation_pairs: BTreeSet<(PackageId, PackageId)> = BTreeSet::new();
    for edge in file_edges {
        if !edge.affects_verdict() {
            continue;
        }
        let source_path = edge_file_path(edge.source(), dependencies, files);
        let target_path = edge_file_path(edge.target(), dependencies, files);
        let source = package_of(source_path, packages.side_roots, packages.roots);
        let target = package_of(target_path, packages.side_roots, packages.roots);
        if source == target {
            continue;
        }
        explanation_pairs.insert((source, target));
        if !edge.enters_verdict_graph() {
            continue;
        }
        let value = package_values.entry((source, target)).or_default();
        value.0 += 1;
        value.1 += edge.references();
        value.2.push(edge.id());
    }
    explanation_pairs.extend(manifest.explanation_pairs);
    for ((source, target), (files, references)) in manifest.package_values {
        let value = package_values.entry((source, target)).or_default();
        value.0 += u32::try_from(files.len()).unwrap_or(u32::MAX);
        value.1 += references;
    }
    let edges: Vec<_> = package_values
        .into_iter()
        .enumerate()
        .map(|(index, ((source, target), (pairs, references, edges)))| {
            PackageEdge::new(
                PackageEdgeId::from_index(index),
                source,
                target,
                pairs,
                references,
                edges,
            )
        })
        .collect();
    let count = packages.roots.len();
    let pairs: Vec<_> = edges
        .iter()
        .map(|edge| (edge.source().index(), edge.target().index()))
        .collect();
    let degrees = dependency_degree(count, &pairs);
    // The package graph is small enough to close over whole: its node count is
    // the package count, so no limit gates it.
    let package_reach = reach_in_counts(count, &pairs);
    let measurements: Vec<_> = degrees
        .into_iter()
        .enumerate()
        .map(|(index, (incoming, outgoing))| {
            PackageGraphMeasurement::new(PackageId::from_index(index), incoming, outgoing)
                .with_reach_in(package_reach[index])
        })
        .collect();
    PackageGraph {
        edges,
        measurements,
        explanation_pairs,
        pairs,
        count,
    }
}
/// The package of every file that enters the file dependency graph, by file
/// table position.
///
/// A file outside the graph belongs to no package closure, so the fraction a
/// package states is a fraction of one population.
pub(crate) fn graph_packages(files: &[FileRecord]) -> Vec<Option<PackageId>> {
    files
        .iter()
        .map(|file| file.package().filter(|_| enters_file_graph(file)))
        .collect()
}
