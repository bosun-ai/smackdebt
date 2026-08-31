use crate::architecture::DependencyEdge;
use crate::path_probe::PathProbe;
use crate::propagation::enters_file_graph;
use crate::report::{FileRecord, PackageId};
use crate::source::StaticRelationKind;
use crate::strongly_connected_components::strongly_connected_components;

/// Whether one relation joins two files over the connection graph.
///
/// The graph admits every `uses` and every `module_ownership` relation whose
/// two endpoint files are primary-role and trusted. It is deliberately wider
/// than the file dependency cycle graph the leaky-interface rule reads, and the
/// contrast is the point: that rule wants a production import whose changes
/// still follow through it, while a claim that *no* code dependency explains a
/// co-change must count the wiring a Rust parent and the child it declares
/// share. Reading the cycle graph here would name exactly the pair that owns
/// itself.
pub fn enters_connection_graph(edge: &DependencyEdge, files: &[FileRecord]) -> bool {
    matches!(
        edge.relation(),
        StaticRelationKind::Uses | StaticRelationKind::ModuleOwnership
    ) && [edge.source(), edge.target()]
        .into_iter()
        .all(|file| files.get(file.index()).is_some_and(enters_file_graph))
}

/// The graph a hidden-coupling proof is run against, with its package-level
/// closure derived once.
///
/// Every admitted relation is held in both directions of travel, because the
/// claim under proof is that no path connects two files *in either direction*.
/// That symmetry is what lets one walk answer for both directions and what
/// collapses the package closure to component identity: over a symmetric graph
/// the condensation has no edge between components, so two packages reach each
/// other exactly when they share one.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConnectionGraph {
    files: usize,
    travel: Vec<(usize, usize)>,
    /// The package connection matrix, held as one component label per package:
    /// two packages are connected exactly when their labels agree.
    package_component: Vec<u32>,
}

impl ConnectionGraph {
    /// Builds the graph from the relations it admits, where `file_packages`
    /// names the package of every file that enters the file dependency graph
    /// and holds nothing for a file outside it.
    pub fn new(
        files: usize,
        packages: usize,
        relations: &[(usize, usize)],
        file_packages: &[Option<PackageId>],
    ) -> Self {
        let mut travel = Vec::with_capacity(relations.len() * 2);
        for &(source, target) in relations {
            travel.push((source, target));
            travel.push((target, source));
        }
        Self {
            files,
            travel,
            package_component: package_components(packages, relations, file_packages),
        }
    }

    /// Whether the package connection matrix proves two packages cannot reach
    /// each other.
    ///
    /// This is the first stage of the absence proof and answers only
    /// *separate*: a pair the matrix leaves connected falls through to the
    /// file-level probe rather than being decided here. A file without a
    /// package, and two files of one package, are never separated by it.
    pub fn separates(&self, left: Option<PackageId>, right: Option<PackageId>) -> bool {
        let (Some(left), Some(right)) = (left, right) else {
            return false;
        };
        match (
            self.package_component.get(left.index()),
            self.package_component.get(right.index()),
        ) {
            (Some(left), Some(right)) => left != right,
            _ => false,
        }
    }

    /// A walk over the connection graph, prepared once for many probes.
    pub fn probe(&self) -> PathProbe {
        PathProbe::over(self.files, &self.travel)
    }
}

/// The connected component of every package over the package-level projection
/// of the connection graph.
///
/// The projection is symmetric, so its strongly connected components are its
/// connected components and each one is its own transitive closure: the
/// condensation machinery the propagation closures use answers the whole
/// matrix in one pass, with no bit set per package.
fn package_components(
    packages: usize,
    relations: &[(usize, usize)],
    file_packages: &[Option<PackageId>],
) -> Vec<u32> {
    let mut projection = Vec::new();
    for &(source, target) in relations {
        let from = file_packages.get(source).copied().flatten();
        let to = file_packages.get(target).copied().flatten();
        if let (Some(from), Some(to)) = (from, to)
            && from != to
        {
            projection.push((from.index(), to.index()));
            projection.push((to.index(), from.index()));
        }
    }
    let mut component = vec![0; packages];
    for (label, members) in strongly_connected_components(packages, &projection)
        .into_iter()
        .enumerate()
    {
        for package in members {
            component[package] = label as u32;
        }
    }
    component
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::HealthCounts;
    use crate::report::{Coverage, FileId, ScopeId};
    use crate::source::{ParseStatus, SourceRole};

    fn file(index: usize, role: SourceRole, status: ParseStatus) -> FileRecord {
        FileRecord::new(
            FileId::from_index(index),
            ScopeId::from_index(0),
            "src/unit.js",
            Coverage::default(),
            HealthCounts::default(),
        )
        .with_source_state(role, status)
    }

    fn edge(source: usize, target: usize, relation: StaticRelationKind) -> DependencyEdge {
        DependencyEdge::new(
            crate::architecture::DependencyEdgeId::from_index(0),
            FileId::from_index(source),
            FileId::from_index(target),
            1,
            Vec::new(),
        )
        .with_relation(relation)
    }

    #[test]
    fn ownership_joins_two_files_the_cycle_graph_would_leave_apart() {
        let files = [
            file(0, SourceRole::Primary, ParseStatus::Parsed),
            file(1, SourceRole::Primary, ParseStatus::Parsed),
        ];
        for relation in [
            StaticRelationKind::Uses,
            StaticRelationKind::ModuleOwnership,
        ] {
            assert!(enters_connection_graph(&edge(0, 1, relation), &files));
        }
    }

    #[test]
    fn a_relation_touching_a_file_outside_the_graph_is_not_a_connection() {
        let files = [
            file(0, SourceRole::Primary, ParseStatus::Parsed),
            file(1, SourceRole::Test, ParseStatus::Parsed),
            file(2, SourceRole::Primary, ParseStatus::Failed),
        ];
        let uses = StaticRelationKind::Uses;
        assert!(!enters_connection_graph(&edge(0, 1, uses), &files));
        assert!(!enters_connection_graph(&edge(0, 2, uses), &files));
        assert!(!enters_connection_graph(&edge(0, 7, uses), &files));
    }

    #[test]
    fn packages_joined_by_one_relation_share_a_component_whichever_way_it_runs() {
        let packages = [
            Some(PackageId::from_index(0)),
            Some(PackageId::from_index(1)),
            Some(PackageId::from_index(2)),
        ];
        let graph = ConnectionGraph::new(3, 3, &[(0, 1)], &packages);
        for (left, right) in [(0, 1), (1, 0)] {
            assert!(
                !graph.separates(
                    Some(PackageId::from_index(left)),
                    Some(PackageId::from_index(right))
                ),
                "one relation connects both directions of travel"
            );
        }
        assert!(graph.separates(
            Some(PackageId::from_index(0)),
            Some(PackageId::from_index(2))
        ));
        // A package is never separate from itself, and a file without a
        // package is never separated from anything.
        assert!(!graph.separates(
            Some(PackageId::from_index(2)),
            Some(PackageId::from_index(2))
        ));
        assert!(!graph.separates(None, Some(PackageId::from_index(2))));
    }

    #[test]
    fn two_packages_joined_through_a_third_are_connected() {
        let packages = [
            Some(PackageId::from_index(0)),
            Some(PackageId::from_index(1)),
            Some(PackageId::from_index(2)),
        ];
        let graph = ConnectionGraph::new(3, 3, &[(0, 1), (2, 1)], &packages);
        assert!(!graph.separates(
            Some(PackageId::from_index(0)),
            Some(PackageId::from_index(2))
        ));
    }
}
