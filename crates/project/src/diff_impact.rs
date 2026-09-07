//! The impact verdict of one diff: propagation, the core, and change
//! leakage, each compared across the two graphs.

use std::collections::BTreeSet;

use smackdebt_analysis::{ChangeLeakageFinding, FileId, FileRecord};

use crate::architecture::{ArchitectureBuild, leakage_evidence_is_complete, leakage_findings};
use crate::core_comparisons::core_comparisons;
use crate::dependencies::DiffTables;
use crate::diff_changes::DiffPackages;
use crate::diff_graphs::DiffArchitectures;
use crate::propagation_comparisons::propagation_comparisons;

/// The impact comparisons, which both the report facts and the scope links
/// read.
#[derive(Clone)]
pub(crate) struct ImpactComparisons {
    pub(crate) propagation: Vec<smackdebt_analysis::PropagationComparison>,
    pub(crate) core: Vec<smackdebt_analysis::CoreComparison>,
    pub(crate) leakage: Vec<smackdebt_analysis::ChangeLeakageComparison>,
}
/// The impact verdict of one diff, and the evidence each comparison stood
/// down on.
pub(crate) struct DiffImpact {
    pub(crate) leakage_findings: Vec<ChangeLeakageFinding>,
    pub(crate) suppressed_leakage: u32,
    pub(crate) comparisons: ImpactComparisons,
    pub(crate) propagation_suppression: smackdebt_analysis::ComparisonSuppression,
    pub(crate) core_suppression: smackdebt_analysis::ComparisonSuppression,
    pub(crate) leakage_suppression: smackdebt_analysis::ComparisonSuppression,
}
/// Compares how far change travels in the two graphs: propagation, the core,
/// and change leakage.
pub(crate) fn compare_diff_impact(
    architectures: &DiffArchitectures,
    tables: &DiffTables,
    packages: &DiffPackages,
    evolution: &smackdebt_analysis::EvolutionaryReportFacts,
) -> DiffImpact {
    let (current_leakage_candidates, current_leakage, suppressed_leakage) =
        leakage_findings(&architectures.current, evolution, &tables.current.files);
    let (before_leakage_candidates, _, _) =
        leakage_findings(&architectures.before, evolution, &tables.before.files);
    let (propagation, propagation_suppression) = propagation_comparisons(
        &architectures.current,
        &architectures.before,
        &packages.current_roots,
        &packages.before_roots,
        &packages.roots,
    );
    let (core, core_suppression) = core_comparisons(&architectures.current, &architectures.before);
    let evidence = LeakageComparisonEvidence {
        pairs: evolution.file_coupling(),
        current: &architectures.current,
        base: &architectures.before,
        current_files: &tables.current.files,
        base_files: &tables.before.files,
    };
    let (leakage, leakage_suppression) = leakage_comparisons(
        &current_leakage_candidates,
        &before_leakage_candidates,
        &evidence,
    );
    DiffImpact {
        leakage_findings: current_leakage,
        suppressed_leakage,
        comparisons: ImpactComparisons {
            propagation,
            core,
            leakage,
        },
        propagation_suppression,
        core_suppression,
        leakage_suppression,
    }
}
pub(crate) fn leakage_comparisons(
    current: &[ChangeLeakageFinding],
    before: &[ChangeLeakageFinding],
    evidence: &LeakageComparisonEvidence<'_>,
) -> (
    Vec<smackdebt_analysis::ChangeLeakageComparison>,
    smackdebt_analysis::ComparisonSuppression,
) {
    type Key = (
        smackdebt_analysis::ChangeLeakageKind,
        smackdebt_analysis::FileChangeCouplingId,
        Option<FileId>,
    );
    let current: BTreeSet<Key> = current
        .iter()
        .map(|finding| (finding.kind(), finding.coupling(), finding.interface()))
        .collect();
    let before: BTreeSet<Key> = before
        .iter()
        .map(|finding| (finding.kind(), finding.coupling(), finding.interface()))
        .collect();
    let mut comparisons = Vec::new();
    let mut suppression = smackdebt_analysis::ComparisonSuppression::default();
    for key in current.symmetric_difference(&before) {
        let &(kind, coupling, _) = key;
        let pair = evidence.pairs[coupling.index()];
        let (current_incomplete, base_incomplete) = evidence.incomplete_sides(pair);
        if current_incomplete || base_incomplete {
            suppression.record(current_incomplete, base_incomplete);
            continue;
        }
        let direction = if current.contains(key) {
            smackdebt_analysis::ComparisonDirection::Worse
        } else {
            smackdebt_analysis::ComparisonDirection::Better
        };
        comparisons.push(smackdebt_analysis::ChangeLeakageComparison::new(
            smackdebt_analysis::ChangeLeakageComparisonId::from_index(comparisons.len()),
            kind,
            pair.left(),
            pair.right(),
            direction,
        ));
    }
    (comparisons, suppression)
}
pub(crate) struct LeakageComparisonEvidence<'a> {
    pub(crate) pairs: &'a [smackdebt_analysis::FileChangeCoupling],
    pub(crate) current: &'a ArchitectureBuild,
    pub(crate) base: &'a ArchitectureBuild,
    pub(crate) current_files: &'a [FileRecord],
    pub(crate) base_files: &'a [FileRecord],
}
impl LeakageComparisonEvidence<'_> {
    pub(crate) fn incomplete_sides(
        &self,
        pair: smackdebt_analysis::FileChangeCoupling,
    ) -> (bool, bool) {
        (
            !leakage_evidence_is_complete(&self.current.graph_evidence, pair, self.current_files),
            !leakage_evidence_is_complete(&self.base.graph_evidence, pair, self.base_files),
        )
    }
}
