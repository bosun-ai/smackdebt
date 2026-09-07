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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebase::analyze_codebase;
    use crate::requests::CodebaseRequest;
    use crate::test_support::write_module_component;
    use std::fs;

    #[test]
    fn rust_module_ownership_cycle_is_context_while_mutual_uses_are_a_verdict() {
        let ownership = tempfile::tempdir().unwrap();
        fs::write(
            ownership.path().join("Cargo.toml"),
            "[package]\nname='ownership'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(ownership.path().join("a.rs"), "mod b;\npub fn a() {}\n").unwrap();
        fs::write(ownership.path().join("b.rs"), "mod a;\npub fn b() {}\n").unwrap();
        let result = analyze_codebase(&CodebaseRequest::new(ownership.path())).unwrap();
        let report = result.report();
        assert_eq!(report.dependency_edges().len(), 2);
        assert!(report.dependency_edges().iter().all(|edge| {
            edge.relation() == smackdebt_analysis::StaticRelationKind::ModuleOwnership
                && !edge.affects_verdict()
        }));
        assert!(report.package_edges().is_empty());
        assert!(report.architecture_findings().is_empty());
        assert_eq!(report.dependency_coverage().module_ownership_relations(), 2);
        assert_eq!(report.dependency_coverage().total(), 2);

        let uses = tempfile::tempdir().unwrap();
        fs::write(
            uses.path().join("Cargo.toml"),
            "[package]\nname='uses'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            uses.path().join("a.rs"),
            "use crate::b::b;\npub fn a() { b(); }\n",
        )
        .unwrap();
        fs::write(
            uses.path().join("b.rs"),
            "use crate::a::a;\npub fn b() { a(); }\n",
        )
        .unwrap();
        let result = analyze_codebase(&CodebaseRequest::new(uses.path())).unwrap();
        let report = result.report();
        assert_eq!(report.dependency_edges().len(), 2);
        assert!(report.dependency_edges().iter().all(|edge| {
            edge.relation() == smackdebt_analysis::StaticRelationKind::Uses
                && edge.affects_verdict()
        }));
        assert!(
            report
                .architecture_findings()
                .iter()
                .any(|finding| { finding.kind() == ArchitectureFindingKind::FileCycle })
        );
        assert_eq!(report.dependency_coverage().resolved_internal_uses(), 2);
        assert_eq!(report.dependency_coverage().total(), 2);

        let wiring = tempfile::tempdir().unwrap();
        fs::write(
            wiring.path().join("Cargo.toml"),
            "[package]\nname='wiring'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::create_dir_all(wiring.path().join("thing")).unwrap();
        fs::write(
            wiring.path().join("thing/mod.rs"),
            "mod child;\npub use self::child::x;\npub fn y() {}\n",
        )
        .unwrap();
        fs::write(
            wiring.path().join("thing/child.rs"),
            "use super::*;\npub fn x() { y(); }\n",
        )
        .unwrap();
        let result = analyze_codebase(&CodebaseRequest::new(wiring.path())).unwrap();
        let report = result.report();
        let relations: Vec<_> = report
            .dependency_edges()
            .iter()
            .map(|edge| {
                (
                    report.files()[edge.source().index()].path().to_owned(),
                    report.files()[edge.target().index()].path().to_owned(),
                    edge.relation(),
                )
            })
            .collect();
        let uses = smackdebt_analysis::StaticRelationKind::Uses;
        let owns = smackdebt_analysis::StaticRelationKind::ModuleOwnership;
        assert_eq!(
            relations,
            [
                ("thing/child.rs".to_owned(), "thing/mod.rs".to_owned(), uses),
                ("thing/mod.rs".to_owned(), "thing/child.rs".to_owned(), uses),
                ("thing/mod.rs".to_owned(), "thing/child.rs".to_owned(), owns),
            ],
            "the wiring relations stay complete in the machine report"
        );
        assert!(
            report.architecture_findings().is_empty(),
            "module wiring between an owning pair is not a file cycle"
        );
        assert!(
            report.orphan_files().is_empty(),
            "the exclusion is scoped to the cycle graph, so orphan facts are unchanged"
        );

        let siblings = tempfile::tempdir().unwrap();
        fs::write(
            siblings.path().join("Cargo.toml"),
            "[package]\nname='siblings'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::create_dir_all(siblings.path().join("thing")).unwrap();
        fs::write(
            siblings.path().join("thing/mod.rs"),
            "mod one;\nmod two;\nmod three;\n",
        )
        .unwrap();
        fs::write(
            siblings.path().join("thing/one.rs"),
            "use super::two::two;\npub fn one() { two() }\n",
        )
        .unwrap();
        fs::write(
            siblings.path().join("thing/two.rs"),
            "use super::three::three;\npub fn two() { three() }\n",
        )
        .unwrap();
        fs::write(
            siblings.path().join("thing/three.rs"),
            "use super::one::one;\npub fn three() { one() }\n",
        )
        .unwrap();
        let result = analyze_codebase(&CodebaseRequest::new(siblings.path())).unwrap();
        let report = result.report();
        let cycles: Vec<_> = report
            .architecture_findings()
            .iter()
            .filter(|finding| finding.kind() == ArchitectureFindingKind::FileCycle)
            .map(|finding| {
                let mut files: Vec<_> = finding
                    .files()
                    .iter()
                    .map(|file| report.files()[file.index()].path().to_owned())
                    .collect();
                files.sort();
                files
            })
            .collect();
        assert_eq!(
            cycles,
            [vec![
                "thing/one.rs".to_owned(),
                "thing/three.rs".to_owned(),
                "thing/two.rs".to_owned(),
            ]],
            "a cycle between owned siblings is not wiring and survives"
        );
        assert!(
            report
                .architecture_findings()
                .iter()
                .find(|finding| finding.kind() == ArchitectureFindingKind::FileCycle)
                .is_some_and(|finding| !finding.witness_edges().is_empty()),
            "a surviving cycle still names the relations that remain in the graph"
        );
    }
    #[test]
    fn a_module_component_collapses_while_a_cycle_between_its_children_survives() {
        let collapsing = tempfile::tempdir().unwrap();
        write_module_component(collapsing.path(), "");
        let result = analyze_codebase(&CodebaseRequest::new(collapsing.path())).unwrap();
        let report = result.report();
        assert_eq!(
            report
                .dependency_edges()
                .iter()
                .filter(|edge| edge.enters_verdict_graph())
                .count(),
            4,
            "every wiring relation stays eligible evidence"
        );
        assert!(
            report.architecture_findings().is_empty(),
            "a parent and its children are one wiring relationship, not a cycle"
        );

        let surviving = tempfile::tempdir().unwrap();
        write_module_component(surviving.path(), "use super::second::second;\n");
        fs::write(
            surviving.path().join("thing/second.rs"),
            "use super::*;\nuse super::first::first;\npub fn second() { y(); first() }\n",
        )
        .unwrap();
        let result = analyze_codebase(&CodebaseRequest::new(surviving.path())).unwrap();
        let report = result.report();
        let cycles: Vec<_> = report
            .architecture_findings()
            .iter()
            .filter(|finding| finding.kind() == ArchitectureFindingKind::FileCycle)
            .map(|finding| {
                let mut files: Vec<_> = finding
                    .files()
                    .iter()
                    .map(|file| report.files()[file.index()].path().to_owned())
                    .collect();
                files.sort();
                files
            })
            .collect();
        assert_eq!(
            cycles,
            [vec![
                "thing/first.rs".to_owned(),
                "thing/second.rs".to_owned()
            ]],
            "the sibling cycle survives while the parent's wiring is excluded"
        );
    }
    #[test]
    fn a_cycle_that_passes_through_an_owning_pair_by_other_files_survives() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname='through'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::create_dir_all(root.path().join("a")).unwrap();
        fs::write(
            root.path().join("a.rs"),
            "mod child;\nuse self::child::step;\nuse self::other::other;\npub fn a() -> u32 { other() + step() }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("a/child.rs"),
            "use super::back::back;\npub fn step() -> u32 { back() }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("a/other.rs"),
            "use super::child::step;\npub fn other() -> u32 { step() }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("a/back.rs"),
            "use super::a;\npub fn back() -> u32 { a() }\n",
        )
        .unwrap();
        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let cycles: Vec<_> = report
            .architecture_findings()
            .iter()
            .filter(|finding| finding.kind() == ArchitectureFindingKind::FileCycle)
            .map(|finding| {
                let mut files: Vec<_> = finding
                    .files()
                    .iter()
                    .map(|file| report.files()[file.index()].path().to_owned())
                    .collect();
                files.sort();
                files
            })
            .collect();
        assert_eq!(
            cycles,
            [vec![
                "a.rs".to_owned(),
                "a/back.rs".to_owned(),
                "a/child.rs".to_owned(),
                "a/other.rs".to_owned(),
            ]],
            "only the owning pair's own relations leave the graph"
        );
        let witnessed: Vec<_> = report.architecture_findings()[0]
            .witness_edges()
            .iter()
            .map(|edge| {
                let edge = &report.dependency_edges()[edge.index()];
                (
                    report.files()[edge.source().index()].path().to_owned(),
                    report.files()[edge.target().index()].path().to_owned(),
                )
            })
            .collect();
        assert!(
            !witnessed.contains(&("a.rs".to_owned(), "a/child.rs".to_owned())),
            "a witness can only name a relation the cycle graph kept: {witnessed:?}"
        );
    }
}
