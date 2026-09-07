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
