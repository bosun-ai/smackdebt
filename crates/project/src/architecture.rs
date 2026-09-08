//! One tree's architecture build: the resolved graph, its completeness
//! evidence, and what leaks from it.

use std::collections::{BTreeMap, BTreeSet};

use smackdebt_analysis::{
    ArchitectureFinding, ArchitectureFindingId, ChangeGraph, ChangeLeakageFinding, ConnectionGraph,
    CoreSize, DependencyCoverage, DependencyEdge, ExternalDependency, FileId, FileReach,
    FileRecord, GraphConfigurationFailure, GraphEvidence, OrphanFile, PackageClosure, PackageEdge,
    PackageFileReach, PackageGraphMeasurement, PackageId, ResolutionDiagnostic, ScopeId,
    SourceRole, SourceTrust, StableDependencyFinding, change_leakage, close_over_packages,
    enters_connection_graph, enters_file_graph, file_reaches, graph_file_count,
    stable_dependency_findings,
};

use crate::cycle_findings::{ArchitectureFindings, FileCycleGraph};
use crate::dependencies::{PackageTables, SourceDependencies};
use crate::dormancy::{
    PackageEntries, WindowedHistory, declared_entry_files, derive_orphans, dormant_javascript,
    restate_dormant, restate_dormant_relations,
};
use crate::manifest_names::ManifestNameIndex;
use crate::package_graph::{graph_packages, package_graph};
use crate::paths::package_of;
use crate::reference_tables::{ReferenceResolver, ReferenceRows};
use crate::resolution_rules::ResolutionRules;
use crate::work::AnalysisWork;

/// Joins the retained file pairs with the graphs the architecture build
/// carried out, which is the whole of what the change-leakage capability adds
/// to composition: no table is walked twice and no graph is rebuilt.
pub(crate) fn leakage_findings(
    architecture: &ArchitectureBuild,
    evolution: &smackdebt_analysis::EvolutionaryReportFacts,
    files: &[FileRecord],
) -> (Vec<ChangeLeakageFinding>, Vec<ChangeLeakageFinding>, u32) {
    let candidates = change_leakage(
        evolution.file_coupling(),
        &ChangeGraph::new(
            &architecture.cycle_pairs,
            &architecture.connections,
            &architecture.graph_packages,
            files,
        ),
    );
    let before = candidates.len();
    let findings = candidates
        .iter()
        .copied()
        .filter(|finding| {
            leakage_evidence_is_complete(
                &architecture.graph_evidence,
                evolution.file_coupling()[finding.coupling().index()],
                files,
            )
        })
        .collect::<Vec<_>>();
    let suppressed = before.saturating_sub(findings.len()) as u32;
    (candidates, findings, suppressed)
}
/// Whether the code a leakage finding names was read completely enough to
/// state it.
///
/// A finding is about two files, so the gate asks about their two packages,
/// whichever rule decided it. A package every import of which resolved cannot
/// be hiding the dependency that would explain a co-change inside it, and a
/// package that could not be read might be.
///
/// The hidden-coupling rule proves its absence over the whole connection
/// graph, so a hole in a third package could in principle carry a path the
/// proof never saw. Withholding every finding because some unrelated corner of
/// the repository could not be read states nothing at all, and the pair's own
/// two packages are what a reader checks the claim against, so that is what
/// this asks. The withheld findings are counted, so what the gate dropped
/// stays visible.
pub(crate) fn leakage_evidence_is_complete(
    evidence: &GraphEvidence,
    pair: smackdebt_analysis::FileChangeCoupling,
    files: &[FileRecord],
) -> bool {
    [pair.left(), pair.right()].into_iter().all(|file| {
        files[file.index()]
            .package()
            .is_some_and(|package| evidence.package_is_complete(package))
    })
}
pub(crate) struct ArchitectureBuild {
    pub(crate) coverage: DependencyCoverage,
    pub(crate) graph_evidence: GraphEvidence,
    pub(crate) orphans: Vec<OrphanFile>,
    pub(crate) stable_dependencies: Vec<StableDependencyFinding>,
    pub(crate) file_edges: Vec<DependencyEdge>,
    pub(crate) package_edges: Vec<PackageEdge>,
    pub(crate) external: Vec<ExternalDependency>,
    pub(crate) diagnostics: Vec<ResolutionDiagnostic>,
    pub(crate) measurements: Vec<PackageGraphMeasurement>,
    pub(crate) findings: Vec<ArchitectureFinding>,
    pub(crate) finding_links: Vec<(ScopeId, ArchitectureFindingId)>,
    pub(crate) cycles: Vec<smackdebt_analysis::PackageCycle>,
    /// Cross-package pairs that explain change coupling, whether or not they
    /// enter a verdict graph.
    pub(crate) explanation_pairs: BTreeSet<(PackageId, PackageId)>,
    /// How far a change reaches inside each package whose value is material.
    pub(crate) package_closures: Vec<PackageClosure>,
    /// Each computed file value behind the selected package closure subjects.
    pub(crate) package_file_reach: Vec<PackageFileReach>,
    /// The packages whose closure the node limit skipped, which the machine
    /// report discloses rather than leaving silently absent.
    pub(crate) skipped_closures: Vec<PackageId>,
    /// The exact repository-wide reach of the bounded candidate set.
    pub(crate) file_reach: Vec<FileReach>,
    /// The largest file dependency cycle, when it is material.
    pub(crate) core_size: Option<CoreSize>,
    pub(crate) core_members: Vec<FileId>,
    pub(crate) file_components: Vec<Vec<FileId>>,
    pub(crate) file_graph_count: u32,
    /// The imports that enter the file dependency cycle graph, which the
    /// leaky-interface rule reads.
    pub(crate) cycle_pairs: Vec<(usize, usize)>,
    /// The wider graph the hidden-coupling rule proves absence against, with
    /// its package closure derived once.
    pub(crate) connections: ConnectionGraph,
    /// The package of every file that enters the file dependency graph.
    pub(crate) graph_packages: Vec<Option<PackageId>>,
    /// The files the resolved relations and the streamed window together
    /// proved nobody is working on, which the caller restates in the tables it
    /// owns.
    pub(crate) dormant: BTreeSet<FileId>,
}
pub(crate) fn build_architecture(
    work: &AnalysisWork,
    inputs: GraphInputs<'_>,
    aliases: &ResolutionRules,
    packages: PackageTables<'_>,
    history: WindowedHistory,
) -> ArchitectureBuild {
    let GraphInputs {
        files,
        dependencies,
    } = inputs;
    let PackageTables {
        side_roots: side_package_roots,
        roots: package_roots,
        manifests,
    } = packages;
    work.record_algorithm_pass();
    let mut index = BTreeMap::new();
    for source in dependencies {
        index.insert(source.path.clone(), source.file);
    }
    let manifest_index = ManifestNameIndex::new(manifests.names);
    let resolver = ReferenceResolver {
        index: &index,
        manifest_index: &manifest_index,
        manifest_names: manifests.names,
        side_package_roots,
        package_roots,
        aliases,
    };
    let ReferenceRows {
        coverage,
        mut diagnostics,
        mut file_edges,
        mut external,
        internal_issue_files,
        manifest,
    } = resolver.resolve(dependencies).into_rows();

    // Computed once, here, and read by both the dormancy rule below and the
    // orphan table further down, so the two can never disagree about what a
    // package owns.
    let mut declared_entries = declared_entry_files(PackageEntries {
        package_roots,
        manifest_names: manifests.names,
        manifest_paths: manifests.paths,
        index: &index,
    });
    declared_entries.extend(
        aliases
            .metadata
            .entries()
            .filter_map(|path| index.get(path).copied()),
    );
    // The last role this build settles, and the first point at which it can be:
    // dormancy is recognized by what nothing does with a file, so the resolved
    // relations above are its evidence. Every graph fact below is derived from
    // the restated table, so the role a file carries in the report is the role
    // its package closure, its core, and its orphan state were computed under.
    let dormant = dormant_javascript(files, dependencies, &file_edges, history, &declared_entries);
    let restated;
    let files = if dormant.is_empty() {
        files
    } else {
        restated = restate_dormant(files, &dormant);
        &restated
    };
    restate_dormant_relations(&dormant, &mut file_edges, &mut external, &mut diagnostics);

    let graph_evidence =
        architecture_graph_evidence(files, &internal_issue_files, aliases, packages, &coverage);

    let package_graph = package_graph(&file_edges, files, dependencies, packages, manifest);
    let mut findings = ArchitectureFindings::default();
    findings.record_package_cycles(&package_graph, files, &file_edges);
    let stable_dependencies =
        stable_dependency_findings(&package_graph.edges, &package_graph.measurements);

    // Orphan fan-in asks whether anything uses a file at all, so it keeps the
    // wider evidence predicate: a file its own tests import is used. The cycle
    // graph is a verdict, so it keeps primary relations only.
    let orphan_pairs: Vec<_> = file_edges
        .iter()
        .filter(|edge| edge.affects_verdict())
        .map(|edge| (edge.source().index(), edge.target().index()))
        .collect();
    let orphans = derive_orphans(files, dependencies, &orphan_pairs, &declared_entries);
    let cycle_graph = FileCycleGraph::of(&file_edges, files.len());
    // Absence is proved against a wider graph than the cycle graph: every
    // `uses` and every `module_ownership` relation between two graph files, in
    // both directions of travel. It is built here, beside the cycle graph it
    // must never be confused with, and carried to the change-leakage join.
    let connection_relations: Vec<_> = file_edges
        .iter()
        .filter(|edge| enters_connection_graph(edge, files))
        .map(|edge| (edge.source().index(), edge.target().index()))
        .collect();
    let package_count = package_roots.len();
    let graph_packages = graph_packages(files);
    let connections = ConnectionGraph::new(
        files.len(),
        package_count,
        &connection_relations,
        &graph_packages,
    );
    let largest_component = cycle_graph
        .components
        .iter()
        .max_by(|left, right| left.len().cmp(&right.len()).then_with(|| right.cmp(left)));
    let file_graph_count = graph_file_count(files);
    let core_size = CoreSize::from_counts(
        largest_component.map_or(0, Vec::len) as u32,
        file_graph_count,
    );
    let core_members = largest_component
        .into_iter()
        .flatten()
        .copied()
        .map(FileId::from_index)
        .collect();
    let retained_file_components = cycle_graph
        .components
        .iter()
        .map(|component| component.iter().copied().map(FileId::from_index).collect())
        .collect();
    let closures = close_over_packages(package_count, &graph_packages, &cycle_graph.pairs);
    let file_reach = file_reaches(files, &cycle_graph.components, &cycle_graph.pairs);
    findings.record_file_cycles(&cycle_graph, files, &file_edges);

    ArchitectureBuild {
        coverage,
        graph_evidence,
        orphans,
        stable_dependencies,
        file_edges,
        package_edges: package_graph.edges,
        external,
        diagnostics,
        measurements: package_graph.measurements,
        findings: findings.findings,
        finding_links: findings.links,
        cycles: findings.cycles,
        explanation_pairs: package_graph.explanation_pairs,
        package_closures: closures.closures().to_vec(),
        package_file_reach: closures.file_reaches().to_vec(),
        skipped_closures: closures.skipped().to_vec(),
        file_reach,
        core_size,
        core_members,
        file_components: retained_file_components,
        file_graph_count,
        cycle_pairs: cycle_graph.pairs,
        connections,
        graph_packages,
        dormant,
    }
}
/// One tree's file records and the dependencies they stated, borrowed as the
/// graph build reads them.
#[derive(Clone, Copy)]
pub(crate) struct GraphInputs<'a> {
    pub(crate) files: &'a [FileRecord],
    pub(crate) dependencies: &'a [SourceDependencies],
}
/// The completeness evidence the graph publishes: which packages hold a hole
/// and why.
pub(crate) fn architecture_graph_evidence(
    files: &[FileRecord],
    internal_issue_files: &BTreeSet<FileId>,
    aliases: &ResolutionRules,
    packages: PackageTables<'_>,
    coverage: &DependencyCoverage,
) -> GraphEvidence {
    let parse_failure_files: Vec<_> = files
        .iter()
        .filter(|file| {
            file.package().is_some()
                && file.role() == SourceRole::Primary
                && file.trust() != SourceTrust::Trusted
        })
        .map(FileRecord::id)
        .collect();
    // Only a file the graph reads can leave a hole in it. `enters_file_graph`
    // is the predicate the closures, the core, and the connection graph are
    // built with, so asking it here keeps what withholds a fact and what
    // produces it the same rule: a fixture, a test, or a generated file may
    // publish every diagnostic its unread imports earned without costing its
    // package the completeness those facts are stated from.
    let mut incomplete_packages: Vec<_> = parse_failure_files
        .iter()
        .chain(
            internal_issue_files
                .iter()
                .filter(|file| enters_file_graph(&files[file.index()])),
        )
        .filter_map(|file| files[file.index()].package())
        .collect();
    let configuration_failures: Vec<_> = aliases
        .packages
        .iter()
        .filter_map(|package| {
            let issue = package.issue.as_ref()?;
            let owner = package_of(
                &package.root.join("resolution-config"),
                packages.side_roots,
                packages.roots,
            );
            incomplete_packages.push(owner);
            Some(GraphConfigurationFailure::new(owner, issue.clone()))
        })
        .collect();
    GraphEvidence::new(
        incomplete_packages,
        parse_failure_files.len() as u32,
        coverage.unresolved_internal_uses(),
        coverage.ambiguous_internal_uses(),
        configuration_failures,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebase::analyze_codebase;
    use crate::diff::analyze_diff;
    use crate::requests::ProjectReport;
    use crate::requests::{CodebaseRequest, DiffRequest};
    use crate::test_support::git;
    use smackdebt_analysis::Coverage;
    use smackdebt_analysis::DiagnosticKind;
    use smackdebt_analysis::HealthCounts;
    use smackdebt_analysis::Language;
    use smackdebt_analysis::ResolutionIssueKind;
    use std::fs;

    /// A recovered parse whose errors sit beside every fact still discloses
    /// itself, but it no longer costs its package the completeness that
    /// reach, core, and leakage are published from.
    #[test]
    fn recovery_beside_every_fact_leaves_the_package_complete() {
        let evidence = recovery_fixture("struct Broken {\n");
        assert!(evidence.complete);
        assert_eq!(evidence.parse_failures, 0);
        assert_eq!(evidence.incomplete_packages, 0);
        assert_eq!(evidence.unresolved_internal, 0);
        assert!(evidence.disclosed);
    }
    #[test]
    fn recovery_over_a_measured_unit_marks_the_package_incomplete() {
        let evidence = recovery_fixture("fn late(value: i32) -> i32 { helper(value broken( }\n");
        assert!(!evidence.complete);
        assert_eq!(evidence.parse_failures, 1);
        assert_eq!(evidence.incomplete_packages, 1);
        assert!(evidence.disclosed);
    }
    struct RecoveryEvidence {
        complete: bool,
        parse_failures: u32,
        incomplete_packages: usize,
        unresolved_internal: u32,
        disclosed: bool,
    }
    /// One package whose only Primary file recovers from `tail`.
    fn recovery_fixture(tail: &str) -> RecoveryEvidence {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname='recovery'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            root.path().join("helper.rs"),
            "pub fn helper(value: i32) -> i32 {\n    value\n}\n",
        )
        .unwrap();
        fs::write(
            root.path().join("main.rs"),
            format!(
                "mod helper;\nuse crate::helper::helper;\n\nfn work(value: i32) -> i32 {{\n    helper(value)\n}}\n\n{tail}"
            ),
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let evidence = report.graph_evidence();
        RecoveryEvidence {
            complete: evidence.is_complete(),
            parse_failures: evidence.parse_failures(),
            incomplete_packages: evidence.incomplete_packages().len(),
            unresolved_internal: evidence.unresolved_internal(),
            disclosed: report.diagnostics().iter().any(|diagnostic| {
                diagnostic.kind() == DiagnosticKind::ParseFailure
                    && diagnostic.message() == "parser recovered from syntax errors"
            }),
        }
    }
    /// A package closes over its own files, so its own evidence decides
    /// whether its reach may be stated.
    #[test]
    fn a_complete_package_states_the_reach_an_incomplete_one_withholds() {
        let root = tempfile::tempdir().unwrap();
        let files = smackdebt_analysis::PACKAGE_REACH_FILES;
        for package in ["app", "core"] {
            fs::create_dir_all(root.path().join(package)).unwrap();
            fs::write(root.path().join(package).join("package.json"), "{}").unwrap();
            for index in 0..files {
                let source = if index + 1 < files {
                    format!(
                        "import next from './unit{}';\nexport default next;\n",
                        index + 1
                    )
                } else {
                    "export default 1;\n".to_owned()
                };
                fs::write(
                    root.path().join(package).join(format!("unit{index}.js")),
                    source,
                )
                .unwrap();
            }
        }
        // One import of one primary file in `core` names nothing, which is
        // what the two packages' evidence differs by.
        fs::write(
            root.path().join("core/unread.js"),
            "import absent from './absent';\nexport default absent;\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let scope = |path: &str| {
            report
                .packages()
                .iter()
                .find(|package| package.path() == path)
                .expect("both packages are reported")
                .scope()
        };
        assert_eq!(report.graph_evidence().incomplete_packages().len(), 1);
        assert_eq!(report.graph_evidence().suppressed_reach(), 1);
        assert!(report.scope_verdict(scope("app")).reach().is_some());
        assert!(report.scope_verdict(scope("core")).reach().is_none());
    }
    /// A leakage finding is about two files, so one unread package withholds
    /// only the findings that name it.
    #[test]
    fn leakage_between_two_complete_packages_survives_a_third_incomplete_one() {
        let file = |index: usize, package: usize| {
            FileRecord::new(
                FileId::from_index(index),
                ScopeId::from_index(0),
                "src/unit.js",
                Coverage::default(),
                HealthCounts::default(),
            )
            .with_package(PackageId::from_index(package))
        };
        let files = [file(0, 0), file(1, 1), file(2, 2)];
        let evidence = GraphEvidence::new(vec![PackageId::from_index(2)], 0, 0, 0, Vec::new());
        let pair = |left: usize, right: usize| {
            smackdebt_analysis::FileChangeCoupling::new(
                FileId::from_index(left),
                FileId::from_index(right),
                4,
                5,
                2,
            )
        };
        let pairs = [pair(0, 1), pair(0, 2)];

        assert!(leakage_evidence_is_complete(&evidence, pairs[0], &files));
        assert!(!leakage_evidence_is_complete(&evidence, pairs[1], &files));
        let suppressed = pairs
            .into_iter()
            .filter(|pair| !leakage_evidence_is_complete(&evidence, *pair, &files))
            .count();
        assert_eq!(suppressed, 1);
    }
    /// A file the file dependency graph never reads cannot leave a hole in it.
    ///
    /// A fixture and a test are outside the graph the reach, core, and leakage
    /// facts are proved over, so an import either of them leaves unresolved
    /// hides nothing from those facts. The diagnostic is still published:
    /// what changes is only whether the package's evidence is called
    /// incomplete.
    #[test]
    fn an_unread_import_outside_the_graph_leaves_the_package_complete() {
        for path in ["tests/fixtures/dynamic.js", "tests/dynamic.test.js"] {
            let evidence = unread_import_evidence(path);
            assert!(evidence.complete, "{path}");
            assert_eq!(evidence.incomplete_packages, 0, "{path}");
            assert_eq!(evidence.diagnostics, 2, "{path}");
            assert_eq!(evidence.suppressed_reach, 0, "{path}");
        }
    }
    #[test]
    fn an_unread_import_in_a_graph_file_marks_its_package_incomplete() {
        let evidence = unread_import_evidence("src/dynamic.js");
        assert!(!evidence.complete);
        assert_eq!(evidence.incomplete_packages, 1);
        assert_eq!(evidence.diagnostics, 2);
    }
    struct UnreadImportEvidence {
        complete: bool,
        incomplete_packages: usize,
        diagnostics: usize,
        suppressed_reach: u32,
    }
    /// One JavaScript package whose file at `path` leaves two imports
    /// unresolved: a dynamic `require` and a name no file matches. Only that
    /// path differs between the cases, so only the role it carries can explain
    /// a difference in the evidence.
    fn unread_import_evidence(path: &str) -> UnreadImportEvidence {
        let root = tempfile::tempdir().unwrap();
        let unread = root.path().join(path);
        fs::create_dir_all(unread.parent().unwrap()).unwrap();
        fs::write(root.path().join("package.json"), "{}").unwrap();
        fs::write(
            root.path().join("main.js"),
            "import helper from './helper';\nexport default helper;\n",
        )
        .unwrap();
        fs::write(root.path().join("helper.js"), "export default 1;\n").unwrap();
        fs::write(
            unread,
            "import absent from './absent';\nconst late = require(moduleName);\nexport default [absent, late];\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let evidence = report.graph_evidence();
        UnreadImportEvidence {
            complete: evidence.is_complete(),
            incomplete_packages: evidence.incomplete_packages().len(),
            diagnostics: report
                .resolution_diagnostics()
                .iter()
                .filter(|diagnostic| diagnostic.kind() == ResolutionIssueKind::Unresolved)
                .count(),
            suppressed_reach: evidence.suppressed_reach(),
        }
    }
    #[test]
    fn invalid_resolution_configuration_marks_the_package_graph_incomplete() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("package.json"), "{}").unwrap();
        fs::write(root.path().join("tsconfig.json"), "{ compilerOptions:").unwrap();
        fs::write(
            root.path().join("main.ts"),
            "import value from '@/value';\nexport default value;\n",
        )
        .unwrap();
        fs::write(root.path().join("value.ts"), "export default 1;\n").unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let evidence = result.report().graph_evidence();
        assert!(!evidence.is_complete());
        assert_eq!(evidence.incomplete_packages().len(), 1);
        assert_eq!(evidence.configuration_failures().len(), 1);
        assert!(
            evidence.configuration_failures()[0]
                .reason()
                .contains("cannot parse")
        );
    }
    #[test]
    fn diff_assigns_base_graph_failures_to_the_base_package_owner() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::create_dir_all(repository_path.join("sub")).unwrap();
        fs::write(repository_path.join("package.json"), "{}").unwrap();
        fs::write(
            repository_path.join("sub/a.ts"),
            "import missing from './missing.js';\nexport default missing;\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "root package"]);
        fs::write(repository_path.join("sub/package.json"), "{}").unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let evidence = result.report().diff_graph_evidence().unwrap();
        assert_eq!(
            evidence.base().incomplete_packages(),
            &[PackageId::from_index(0)]
        );
        assert_eq!(
            evidence.current().incomplete_packages(),
            &[PackageId::from_index(1)]
        );
    }
    fn astro_diff_result(change: &str) -> ProjectReport {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}\n").unwrap();
        if change != "added" {
            fs::write(repository_path.join("page.astro"), "<h1>Before</h1>\n").unwrap();
        }
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "base"]);
        match change {
            "added" => fs::write(repository_path.join("page.astro"), "<h1>Added</h1>\n").unwrap(),
            "modified" => {
                fs::write(repository_path.join("page.astro"), "<h1>After</h1>\n").unwrap()
            }
            "deleted" => fs::remove_file(repository_path.join("page.astro")).unwrap(),
            _ => unreachable!("test chooses an Astro change"),
        }
        analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap()
    }
    fn assert_astro_diff_is_retained(result: &ProjectReport) {
        let report = result.report();
        let root = report.root().unwrap();
        let coverage = report.scopes()[root.index()].coverage();
        assert_eq!(coverage.selected_files(), 1);
        assert_eq!(coverage.unsupported_files(), 1);
        assert_eq!(report.files().len(), 1);
        assert_eq!(report.files()[0].language(), Some(Language::Astro));
        assert_eq!(report.files()[0].trust(), SourceTrust::Failed);
        assert!(report.findings().is_empty());
        assert!(report.dependency_edges().is_empty());
    }
    #[test]
    fn added_astro_makes_only_current_diff_graph_evidence_incomplete() {
        let result = astro_diff_result("added");
        assert_astro_diff_is_retained(&result);
        let evidence = result.report().diff_graph_evidence().unwrap();
        assert!(!evidence.current().is_complete());
        assert!(evidence.base().is_complete());
    }
    #[test]
    fn modified_astro_makes_both_diff_graph_evidence_sides_incomplete() {
        let result = astro_diff_result("modified");
        assert_astro_diff_is_retained(&result);
        let evidence = result.report().diff_graph_evidence().unwrap();
        assert!(!evidence.current().is_complete());
        assert!(!evidence.base().is_complete());
    }
    #[test]
    fn deleted_astro_makes_only_base_diff_graph_evidence_incomplete() {
        let result = astro_diff_result("deleted");
        assert_astro_diff_is_retained(&result);
        let evidence = result.report().diff_graph_evidence().unwrap();
        assert!(evidence.current().is_complete());
        assert!(!evidence.base().is_complete());
    }
}
