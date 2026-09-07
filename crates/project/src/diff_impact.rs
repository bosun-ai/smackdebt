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

#[cfg(test)]
mod tests {

    use crate::diff::analyze_diff;
    use crate::requests::DiffRequest;
    use crate::test_support::git;
    use std::fs;

    #[test]
    fn adding_a_code_link_resolves_hidden_coupling_in_the_diff() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}").unwrap();
        fs::create_dir_all(repository_path.join("left")).unwrap();
        fs::create_dir_all(repository_path.join("right")).unwrap();
        for revision in 0..5 {
            fs::write(
                repository_path.join("left/a.ts"),
                format!("export const a = {revision};\n"),
            )
            .unwrap();
            fs::write(
                repository_path.join("right/b.ts"),
                format!("export const b = {revision};\n"),
            )
            .unwrap();
            git(repository_path, ["add", "."]);
            git(
                repository_path,
                ["commit", "-qm", &format!("change {revision}")],
            );
        }
        fs::write(
            repository_path.join("left/a.ts"),
            "import { b } from '../right/b.js';\nexport const a = b + 1;\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let comparison = result
            .report()
            .change_leakage_comparisons()
            .iter()
            .find(|comparison| {
                comparison.kind() == smackdebt_analysis::ChangeLeakageKind::HiddenCoupling
            })
            .expect("the new code link resolves the hidden pair");
        assert_eq!(
            comparison.direction(),
            smackdebt_analysis::ComparisonDirection::Better
        );
    }
    #[test]
    fn diff_counts_a_leakage_candidate_before_incomplete_evidence_withholds_it() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(repository_path.join("package.json"), "{}").unwrap();
        fs::create_dir_all(repository_path.join("left")).unwrap();
        fs::create_dir_all(repository_path.join("right")).unwrap();
        for revision in 0..5 {
            fs::write(
                repository_path.join("left/a.ts"),
                format!("export const a = {revision};\n"),
            )
            .unwrap();
            fs::write(
                repository_path.join("right/b.ts"),
                format!("export const b = {revision};\n"),
            )
            .unwrap();
            git(repository_path, ["add", "."]);
            git(
                repository_path,
                ["commit", "-qm", &format!("change {revision}")],
            );
        }
        fs::write(
            repository_path.join("left/a.ts"),
            "import { b } from '../right/b.js';\nexport const a = b + 1;\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("other.ts"),
            "import missing from './missing.js';\nexport default missing;\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let evidence = result.report().diff_graph_evidence().unwrap();
        // The new dependency resolves one hidden-coupling candidate and creates
        // one leaky-interface candidate. Both are counted before the incomplete
        // current graph withholds them.
        assert_eq!(evidence.leakage().total(), 2);
        assert_eq!(evidence.leakage().current(), 2);
        assert_eq!(evidence.leakage().base(), 0);
        assert!(result.report().change_leakage_comparisons().is_empty());
    }
}
