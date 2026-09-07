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

#[cfg(test)]
mod tests {

    use crate::codebase::analyze_codebase;
    use crate::requests::CodebaseRequest;
    use crate::test_support::{file_id, write_crate};
    use smackdebt_analysis::ArchitectureFindingKind;
    use smackdebt_analysis::DependencyCoverage;
    use smackdebt_analysis::Rating;
    use smackdebt_analysis::ResolutionIssueKind;
    use smackdebt_analysis::SourceRole;
    use smackdebt_analysis::SourceTrust;
    use std::fs;

    #[test]
    fn codebase_builds_package_cycles_and_exact_dependency_coverage() {
        let root = tempfile::tempdir().unwrap();
        for package in ["app", "core"] {
            fs::create_dir_all(root.path().join(package)).unwrap();
            fs::write(root.path().join(package).join("package.json"), "{}").unwrap();
        }
        fs::write(
            root.path().join("app/a.js"),
            "import core from '../core/b';\nimport ext from 'external';\nconst late = require(name);\nconst later = require(name);\nfunction app() {}\n",
        ).unwrap();
        fs::write(
            root.path().join("core/b.js"),
            "import app from '../app/a';\nfunction core() {}\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        assert_eq!(report.dependency_edges().len(), 2);
        assert_eq!(report.package_edges().len(), 2);
        assert_eq!(
            report.dependency_coverage(),
            DependencyCoverage::new(2, 0, 0, 1, 2, 0, 0)
        );
        assert_eq!(report.dependency_coverage().total(), 5);
        assert_eq!(report.architecture_findings().len(), 1);
        assert_eq!(
            report.architecture_findings()[0].kind(),
            ArchitectureFindingKind::PackageCycle
        );
        assert_eq!(report.architecture_findings()[0].rating(), Rating::High);
        let external = &report.external_dependencies()[0];
        assert_eq!(
            external.relation(),
            smackdebt_analysis::StaticRelationKind::Uses
        );
        assert_eq!(external.role(), SourceRole::Primary);
        assert_eq!(external.trust(), SourceTrust::Trusted);
        assert_eq!(external.references(), 1);
        assert_eq!(
            external.locations(),
            &[smackdebt_analysis::SourceSpan::new(2, 2)]
        );
        let unresolved = report
            .resolution_diagnostics()
            .iter()
            .find(|value| value.kind() == ResolutionIssueKind::Unresolved)
            .expect("dynamic reference stays unresolved");
        assert_eq!(
            unresolved.relation(),
            smackdebt_analysis::StaticRelationKind::Uses
        );
        assert_eq!(unresolved.role(), SourceRole::Primary);
        assert_eq!(unresolved.trust(), SourceTrust::Trusted);
        assert_eq!(unresolved.references(), 2);
        assert_eq!(unresolved.span(), smackdebt_analysis::SourceSpan::new(3, 3));
        assert_eq!(
            unresolved.locations(),
            &[
                smackdebt_analysis::SourceSpan::new(3, 3),
                smackdebt_analysis::SourceSpan::new(4, 4),
            ]
        );
    }
    #[test]
    fn repeated_references_share_file_and_package_edges_with_exact_counts() {
        let root = tempfile::tempdir().unwrap();
        for package in ["app", "core"] {
            fs::create_dir_all(root.path().join(package)).unwrap();
            fs::write(root.path().join(package).join("package.json"), "{}").unwrap();
        }
        fs::write(
            root.path().join("app/a.js"),
            "import first from '../core/b';\nimport second from '../core/b';\nfunction app() {}\n",
        )
        .unwrap();
        fs::write(
            root.path().join("app/other.js"),
            "import core from '../core/b';\nfunction other() {}\n",
        )
        .unwrap();
        fs::write(
            root.path().join("core/b.js"),
            "export default function core() {}\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        assert_eq!(report.dependency_edges().len(), 2);
        assert_eq!(report.package_edges().len(), 1);
        assert_eq!(report.package_edges()[0].file_pairs(), 2);
        assert_eq!(report.package_edges()[0].references(), 3);
    }
    #[test]
    fn a_primary_edge_reports_a_stable_dependency_violation_that_test_edges_do_not() {
        let violating = tempfile::tempdir().unwrap();
        write_crate(
            violating.path(),
            "x",
            "use a::run;\npub fn x() -> u32 { run() }\n",
        );
        write_crate(
            violating.path(),
            "a",
            "use b::one;\nuse b::two;\npub fn run() -> u32 { one() + two() }\n",
        );
        write_crate(
            violating.path(),
            "b",
            "use c::cee;\nuse d::dee;\npub fn one() -> u32 { cee() }\npub fn two() -> u32 { dee() }\n",
        );
        write_crate(violating.path(), "c", "pub fn cee() -> u32 { 1 }\n");
        write_crate(violating.path(), "d", "pub fn dee() -> u32 { 2 }\n");

        let result = analyze_codebase(&CodebaseRequest::new(violating.path())).unwrap();
        let report = result.report();
        let package_name = |package: smackdebt_analysis::PackageId| {
            report.packages()[package.index()].path().to_owned()
        };
        let findings: Vec<_> = report
            .stable_dependency_findings()
            .iter()
            .map(|finding| {
                (
                    package_name(finding.source()),
                    package_name(finding.target()),
                    finding.evidence().references(),
                )
            })
            .collect();
        assert_eq!(
            findings,
            [("crates/a".to_owned(), "crates/b".to_owned(), 2)]
        );

        let scoped = tempfile::tempdir().unwrap();
        write_crate(
            scoped.path(),
            "x",
            "use a::run;\npub fn x() -> u32 { run() }\n",
        );
        write_crate(
            scoped.path(),
            "a",
            "pub fn run() -> u32 { 3 }\n#[cfg(test)]\nmod tests {\n    use b::one;\n    use b::two;\n    #[test]\n    fn covers() { assert_eq!(one() + two(), 3); }\n}\n",
        );
        write_crate(
            scoped.path(),
            "b",
            "use c::cee;\nuse d::dee;\npub fn one() -> u32 { cee() }\npub fn two() -> u32 { dee() }\n",
        );
        write_crate(scoped.path(), "c", "pub fn cee() -> u32 { 1 }\n");
        write_crate(scoped.path(), "d", "pub fn dee() -> u32 { 2 }\n");

        let result = analyze_codebase(&CodebaseRequest::new(scoped.path())).unwrap();
        assert!(
            result.report().stable_dependency_findings().is_empty(),
            "a test-scoped import is not a production dependency direction"
        );
    }
    #[test]
    fn a_test_role_relation_stays_evidence_and_never_closes_a_package_cycle() {
        let root = tempfile::tempdir().unwrap();
        write_crate(
            root.path(),
            "alpha",
            "use beta::helper;\npub fn run() -> u32 { helper() }\n",
        );
        write_crate(
            root.path(),
            "beta",
            "pub fn helper() -> u32 { 1 }\n#[cfg(test)]\nmod tests {\n    use alpha::run;\n    #[test]\n    fn covers() { assert_eq!(run(), 1); }\n}\n",
        );
        fs::create_dir_all(root.path().join("crates/beta/tests")).unwrap();
        fs::write(
            root.path().join("crates/beta/tests/it.rs"),
            "use alpha::run;\n#[test]\nfn integrates() { assert_eq!(run(), 1); }\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let alpha = file_id(report, "crates/alpha/src/lib.rs");
        let beta = file_id(report, "crates/beta/src/lib.rs");

        let mut back_edges: Vec<_> = report
            .dependency_edges()
            .iter()
            .filter(|edge| edge.target() == alpha)
            .map(|edge| (edge.source() == beta, edge.role()))
            .collect();
        back_edges.sort();
        assert_eq!(
            back_edges,
            [(false, SourceRole::Test), (true, SourceRole::Test)],
            "the test-role relations into alpha must stay in the machine report"
        );

        let package_edges: Vec<_> = report
            .package_edges()
            .iter()
            .map(|edge| (edge.source(), edge.target()))
            .collect();
        let alpha_package = report.files()[alpha.index()].package().unwrap();
        let beta_package = report.files()[beta.index()].package().unwrap();
        assert_eq!(package_edges, [(alpha_package, beta_package)]);
        assert!(
            report
                .architecture_findings()
                .iter()
                .all(|finding| finding.kind() != ArchitectureFindingKind::PackageCycle)
        );
        assert!(report.stable_dependency_findings().is_empty());
    }
}
