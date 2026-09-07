//! Both trees' architecture builds and the rows their findings restate.

use smackdebt_analysis::{
    ArchitectureComparison, ArchitectureComparisonId, ArchitectureFinding, ArchitectureFindingId,
};

use crate::architecture::{ArchitectureBuild, GraphInputs, build_architecture};
use crate::dependencies::{DiffTables, ManifestFacts, PackageTables};
use crate::diff_changes::{DiffAliases, DiffPackages};
use crate::dormancy::WindowedHistory;
use crate::manifest_names::{manifest_names_for, manifest_paths_for};
use crate::work::AnalysisWork;
use smackdebt_discovery::Inventory;

/// Both trees' architecture builds.
pub(crate) struct DiffArchitectures {
    pub(crate) current: ArchitectureBuild,
    pub(crate) before: ArchitectureBuild,
}
/// Builds the architecture graph each tree states over the shared package
/// positions.
pub(crate) fn build_diff_architectures(
    work: &AnalysisWork,
    tables: &DiffTables,
    packages: &DiffPackages,
    aliases: DiffAliases<'_>,
    inventory: &Inventory,
) -> DiffArchitectures {
    let current_manifest_names = manifest_names_for(inventory, &packages.roots);
    let current_manifest_paths = manifest_paths_for(inventory, &packages.roots);
    let current = build_architecture(
        work,
        GraphInputs {
            files: &tables.current.files,
            dependencies: &tables.current.dependencies,
        },
        aliases.current,
        PackageTables {
            side_roots: &packages.current_roots,
            roots: &packages.roots,
            manifests: ManifestFacts {
                names: &current_manifest_names,
                paths: &current_manifest_paths,
            },
        },
        // A diff answers what two trees say about the changed units. Neither
        // tree carries the per-file window activity the dormancy rule reads, so
        // the rule stands down and both sides keep the roles their names and
        // markers state.
        WindowedHistory::Absent,
    );
    let before = build_architecture(
        work,
        GraphInputs {
            files: &tables.before.files,
            dependencies: &tables.before.dependencies,
        },
        aliases.before,
        PackageTables {
            side_roots: &packages.before_roots,
            roots: &packages.roots,
            manifests: ManifestFacts {
                names: &packages.before_manifest_names,
                // The base tree is read from Git objects, which the walk never
                // opens, so no manifest of that tree was read.
                paths: &[],
            },
        },
        WindowedHistory::Absent,
    );
    DiffArchitectures { current, before }
}
/// The architecture rows the scope links restate, taken because the report
/// facts consume the originals.
pub(crate) struct ArchitectureLinks {
    pub(crate) comparison_ids: Vec<ArchitectureComparisonId>,
    pub(crate) comparisons: Vec<ArchitectureComparison>,
    pub(crate) finding_ids: Vec<ArchitectureFindingId>,
    pub(crate) findings: Vec<ArchitectureFinding>,
}
impl ArchitectureLinks {
    pub(crate) fn of(comparisons: &[ArchitectureComparison], current: &ArchitectureBuild) -> Self {
        Self {
            comparison_ids: comparisons
                .iter()
                .map(|comparison| comparison.id())
                .collect(),
            comparisons: comparisons.to_vec(),
            finding_ids: current
                .findings
                .iter()
                .map(|finding| finding.id())
                .collect(),
            findings: current.findings.clone(),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::diff::analyze_diff;
    use crate::requests::DiffRequest;
    use crate::test_support::git;
    use smackdebt_analysis::ResolutionIssueKind;
    use std::fs;

    #[test]
    fn diff_uses_unchanged_edges_to_find_an_introduced_package_cycle() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        for package in ["app", "core"] {
            fs::create_dir_all(repository_path.join(package)).unwrap();
            fs::write(repository_path.join(package).join("package.json"), "{}").unwrap();
        }
        fs::create_dir_all(repository_path.join("app/src")).unwrap();
        fs::write(
            repository_path.join("app/src/a.js"),
            "import core from '../../core/b';\nfunction app() {}\n",
        )
        .unwrap();
        fs::write(repository_path.join("core/b.js"), "function core() {}\n").unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "initial"]);
        fs::write(
            repository_path.join("core/b.js"),
            "import app from '../app/src/a';\nfunction core() {}\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        assert!(
            result
                .report()
                .architecture_comparisons()
                .iter()
                .any(|comparison| {
                    comparison.kind()
                        == smackdebt_analysis::ArchitectureComparisonKind::CycleIntroduced
                        && comparison.direction() == smackdebt_analysis::ComparisonDirection::Worse
                })
        );
        assert_eq!(result.stats().inventory_walks, 1);
        assert_eq!(result.stats().source_reads, 2);
    }
    #[test]
    fn selected_package_diff_uses_an_outside_change_for_incoming_cycle_evidence() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        for package in ["app", "core"] {
            fs::create_dir_all(repository_path.join(package)).unwrap();
            fs::write(repository_path.join(package).join("package.json"), "{}").unwrap();
        }
        fs::create_dir_all(repository_path.join("app/src")).unwrap();
        fs::write(
            repository_path.join("app/src/a.js"),
            "import core from '../../core/b';\nfunction app() {}\n",
        )
        .unwrap();
        fs::write(repository_path.join("core/b.js"), "function core() {}\n").unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "initial"]);
        fs::write(
            repository_path.join("core/b.js"),
            "import app from '../app/src/a';\nfunction core() {}\n",
        )
        .unwrap();

        for selected in ["app", "app/src", "app/src/a.js"] {
            let result = analyze_diff(
                &DiffRequest::new(repository_path.join(selected)).with_reference("HEAD"),
            )
            .unwrap();
            let scope = result.selected_scope().unwrap();
            assert_eq!(result.report().scopes()[scope.index()].name(), selected);
            let comparisons: Vec<_> = result.report().scopes()[scope.index()]
                .architecture_comparisons()
                .iter()
                .map(|id| &result.report().architecture_comparisons()[id.index()])
                .collect();
            assert!(comparisons.iter().any(|value| value.kind()
                == smackdebt_analysis::ArchitectureComparisonKind::CycleIntroduced));
            assert!(
                comparisons.iter().any(|value| value.kind()
                    == smackdebt_analysis::ArchitectureComparisonKind::EdgeAdded)
            );
        }
    }
    #[test]
    fn diff_resolves_manifest_names_from_each_graph_side() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        for package in ["a", "b"] {
            fs::create_dir_all(repository_path.join(package)).unwrap();
        }
        fs::write(
            repository_path.join("a/package.json"),
            "{\"name\":\"old-name\",\"main\":\"main.js\"}\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("b/package.json"),
            "{\"name\":\"b\",\"main\":\"main.js\"}\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("a/main.js"),
            "import value from 'b';\nexport default value;\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("b/main.js"),
            "import value from 'old-name';\nexport default value;\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "old package name"]);
        fs::write(
            repository_path.join("a/package.json"),
            "{\"name\":\"new-name\",\"main\":\"main.js\"}\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("b/main.js"),
            "import value from 'new-name';\nexport default value;\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        assert_eq!(result.report().architecture_findings().len(), 1);
        assert!(result.report().architecture_comparisons().is_empty());
        assert!(
            result
                .report()
                .resolution_diagnostics()
                .iter()
                .all(|diagnostic| diagnostic.kind() != ResolutionIssueKind::Unresolved)
        );
    }
    #[test]
    fn unchanged_gemspec_names_resolve_on_both_sides_of_an_unrelated_diff() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        for package in ["a", "b"] {
            fs::create_dir_all(repository_path.join(package)).unwrap();
        }
        fs::write(
            repository_path.join("a/a.gemspec"),
            "Gem::Specification.new do |spec|\n  spec.name = 'a-gem'\nend\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("b/b.gemspec"),
            "Gem::Specification.new do |spec|\n  spec.name = 'b-gem'\nend\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("a/main.js"),
            "import value from 'b-gem';\nexport default value;\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("b/main.js"),
            "import value from 'a-gem';\nexport default value;\n",
        )
        .unwrap();
        fs::write(repository_path.join("note.md"), "before\n").unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "gemspec packages"]);
        fs::write(repository_path.join("note.md"), "after\n").unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        assert_eq!(result.report().architecture_findings().len(), 1);
        assert!(result.report().architecture_comparisons().is_empty());
    }
}
