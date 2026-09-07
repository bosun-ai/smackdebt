//! The dependency vocabulary both flows share: what one file's references
//! state and the tables the graph builds read them from.

use std::path::PathBuf;

use smackdebt_analysis::{DependencySyntax, FileId, FileRecord, Language, SourceRole, SourceTrust};

/// One tree's file and dependency tables, as an architecture build reads
/// them.
pub(crate) struct SideTables {
    pub(crate) files: Vec<FileRecord>,
    pub(crate) dependencies: Vec<SourceDependencies>,
}
/// Both trees' tables, filled in the same file order.
pub(crate) struct DiffTables {
    pub(crate) current: SideTables,
    pub(crate) before: SideTables,
}
impl DiffTables {
    pub(crate) fn with_capacity(selected_count: usize) -> Self {
        Self {
            current: SideTables {
                files: Vec::with_capacity(selected_count),
                dependencies: Vec::new(),
            },
            before: SideTables {
                files: Vec::with_capacity(selected_count),
                dependencies: Vec::new(),
            },
        }
    }
}
#[derive(Clone)]
pub(crate) struct SourceDependencies {
    pub(crate) file: FileId,
    pub(crate) path: PathBuf,
    pub(crate) references: Vec<DependencySyntax>,
    pub(crate) role: SourceRole,
    pub(crate) trust: SourceTrust,
    pub(crate) language: Language,
    /// Whether the file is written as a module rather than a plain script.
    pub(crate) module_syntax: bool,
}
/// What each package's own manifest states about itself, in package order.
///
/// The two tables are read together wherever a package is asked what it owns,
/// so a build can never hold one without the other.
#[derive(Clone, Copy)]
pub(crate) struct ManifestFacts<'a> {
    pub(crate) names: &'a [Option<String>],
    /// The paths each manifest names as something it publishes, installs, or
    /// runs, spelled relative to the package directory.
    pub(crate) paths: &'a [Vec<String>],
}
/// The package facts one graph side is built against.
///
/// `side_roots` are the package roots of the tree being read; `roots` are the
/// report's own package positions, which the two sides of a diff share.
#[derive(Clone, Copy)]
pub(crate) struct PackageTables<'a> {
    pub(crate) side_roots: &'a [PathBuf],
    pub(crate) roots: &'a [PathBuf],
    pub(crate) manifests: ManifestFacts<'a>,
}

/// Which side of a diff a classification pass reads.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum DiffSideSelector {
    Current,
    Before,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebase::analyze_codebase;
    use crate::requests::CodebaseRequest;
    use smackdebt_analysis::HealthCounts;
    use smackdebt_analysis::{HealthPolicy, Thresholds};
    use std::fs;

    #[test]
    fn recovered_dependency_is_retained_but_cannot_enter_the_verdict_graph() {
        let root = tempfile::tempdir().unwrap();
        for package in ["app", "core"] {
            fs::create_dir_all(root.path().join(package)).unwrap();
            fs::write(root.path().join(package).join("package.json"), "{}").unwrap();
        }
        fs::write(
            root.path().join("app/main.js"),
            "import core from '../core/main';\nfunction broken( { if (a) { if (b) { core(); } }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("core/main.js"),
            "export default function core() {}\n",
        )
        .unwrap();
        fs::create_dir_all(root.path().join("app/fixtures")).unwrap();
        fs::write(
            root.path().join("app/fixtures/context.js"),
            "export function context() {}\n",
        )
        .unwrap();
        fs::write(
            root.path().join("app/unsupported.kt"),
            "fun unsupported() = Unit\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path()).with_thresholds(
            HealthPolicy::new(
                Thresholds::new(1, 2),
                Thresholds::new(1, 2),
                Thresholds::new(1, 2),
                Thresholds::new(4, 7),
                Thresholds::new(6, 9),
            ),
        ))
        .unwrap();
        let report = result.report();
        let recovered = report
            .files()
            .iter()
            .find(|file| file.path() == "app/main.js")
            .unwrap();
        assert_eq!(recovered.trust(), SourceTrust::Advisory);
        assert_eq!(recovered.health(), HealthCounts::default());
        assert_eq!(recovered.coverage().clean_files(), 0);
        assert_eq!(recovered.coverage().recovered_files(), 1);
        let root_coverage = report.scopes()[report.root().unwrap().index()].coverage();
        assert_eq!(root_coverage.clean_files(), 1);
        assert_eq!(root_coverage.recovered_files(), 1);
        assert_eq!(root_coverage.unsupported_files(), 1);
        assert_eq!(root_coverage.failed_files(), 0);
        assert_eq!(root_coverage.context_files(), 1);
        assert_eq!(root_coverage.selected_files(), 4);
        assert!(report.findings().iter().any(|finding| {
            finding.file() == recovered.id() && finding.trust() == SourceTrust::Advisory
        }));
        assert_eq!(report.dependency_edges().len(), 1);
        assert_eq!(report.dependency_edges()[0].trust(), SourceTrust::Advisory);
        assert!(report.package_edges().is_empty());
        assert!(report.architecture_findings().is_empty());
        assert_eq!(report.dependency_coverage().context_relations(), 1);
        assert_eq!(report.dependency_coverage().total(), 1);
    }
    #[test]
    fn fixture_dependency_is_visible_without_affecting_architecture_health() {
        let root = tempfile::tempdir().unwrap();
        for package in ["app", "core"] {
            fs::create_dir_all(root.path().join(package)).unwrap();
            fs::write(root.path().join(package).join("package.json"), "{}").unwrap();
        }
        fs::create_dir_all(root.path().join("app/fixtures")).unwrap();
        fs::write(
            root.path().join("app/fixtures/main.js"),
            "import core from '../../core/main';\nimport { helper } from './helper';\nexport function fixture() { helper(); core(); }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("app/fixtures/helper.js"),
            "import { fixture } from './main';\nexport function helper() { fixture(); }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("core/main.js"),
            "export default function core() {}\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        assert_eq!(report.dependency_edges().len(), 3);
        assert!(
            report
                .dependency_edges()
                .iter()
                .all(|edge| edge.role() == SourceRole::Fixture && !edge.affects_verdict())
        );
        assert!(report.package_edges().is_empty());
        assert!(report.architecture_findings().is_empty());
        assert_eq!(report.dependency_coverage().context_relations(), 3);
        assert_eq!(report.dependency_coverage().total(), 3);
        let root_coverage = report.scopes()[report.root().unwrap().index()].coverage();
        assert_eq!(root_coverage.clean_files(), 1);
        assert_eq!(root_coverage.context_files(), 2);
        assert_eq!(root_coverage.recovered_files(), 0);
    }
    #[test]
    fn generated_dependency_is_visible_without_affecting_architecture_health() {
        let root = tempfile::tempdir().unwrap();
        for package in ["app", "core"] {
            fs::create_dir_all(root.path().join(package)).unwrap();
            fs::write(root.path().join(package).join("package.json"), "{}").unwrap();
        }
        fs::create_dir_all(root.path().join("app/generated")).unwrap();
        fs::write(
            root.path().join("app/generated/main.js"),
            "// @generated\nimport core from '../../core/main';\nimport { helper } from './helper';\nexport function generated() { helper(); core(); }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("app/generated/helper.js"),
            "// @generated\nimport { generated } from './main';\nexport function helper() { generated(); }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("core/main.js"),
            "export default function core() {}\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        assert_eq!(report.dependency_edges().len(), 3);
        assert!(report.dependency_edges().iter().all(|edge| {
            edge.role() == SourceRole::Generated
                && edge.relation() == smackdebt_analysis::StaticRelationKind::Uses
                && !edge.affects_verdict()
        }));
        assert!(report.package_edges().is_empty());
        assert!(report.architecture_findings().is_empty());
        assert_eq!(report.dependency_coverage().context_relations(), 3);
        assert_eq!(report.dependency_coverage().total(), 3);
    }
}
