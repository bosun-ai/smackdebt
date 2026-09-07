//! Retaining one changed file's comparisons, findings, and diagnostics.

use smackdebt_analysis::{
    Comparison, ComparisonId, Coverage, Diagnostic, DiagnosticId, DiagnosticKind, FileId,
    FileRecord, Finding, FindingId, HealthCounts, HealthPolicy, Language, PackageId, ParseStatus,
    Rating, ReportBuilder as AnalysisReportBuilder, ScopeId, SourceCoverageOutcome, SourceRole,
};

use crate::diff_source::{DiffResult, DiffSide};
use crate::rating::{source_coverage, verdict_eligible};

/// One matched comparison as the report keeps it: the same measurements,
/// placed in the file it came from, told what its source roles allow, and
/// carrying whatever the matcher could not settle about its identity.
pub(crate) fn retained_comparison(
    comparison: &Comparison,
    id: ComparisonId,
    file: FileId,
    roles: (Option<SourceRole>, Option<SourceRole>),
) -> Comparison {
    let mut retained = Comparison::new(
        id,
        comparison.identity().clone(),
        comparison.kind(),
        comparison.before(),
        comparison.after(),
        comparison.before_rating(),
        comparison.after_rating(),
    )
    .with_file(file)
    .with_source_roles(roles.0, roles.1);
    if let Some(span) = comparison.span() {
        retained = retained.with_span(span);
    }
    if comparison.is_anonymous_ambiguity() {
        retained = retained.with_anonymous_ambiguity();
    }
    if comparison.is_unpaired_anonymous() {
        retained = retained.with_unpaired_anonymous();
    }
    retained
}
/// Where one changed file sits in the report and whether the change
/// selected it.
#[derive(Clone, Copy)]
pub(crate) struct DiffSpot {
    pub(crate) scope: ScopeId,
    pub(crate) package: PackageId,
    pub(crate) selected: bool,
}

/// One changed file's landing context while its rows are added.
struct ResultSink<'a> {
    report: &'a mut AnalysisReportBuilder,
    indexes: &'a mut DiffIndexes,
    file: FileId,
    spot: DiffSpot,
}

impl ResultSink<'_> {
    /// Retains the file's comparisons, and one ambiguity diagnostic when
    /// anonymous units could not be matched safely.
    fn add_comparisons(
        &mut self,
        comparisons: &[Comparison],
        roles: (Option<SourceRole>, Option<SourceRole>),
    ) {
        let mut has_ambiguous_identity = false;
        let mut ambiguity_affects_verdict = false;
        for comparison in comparisons.iter().filter(|_| self.spot.selected) {
            let comparison_id = ComparisonId::from_index(self.indexes.comparison);
            let retained = retained_comparison(comparison, comparison_id, self.file, roles);
            if retained.is_anonymous_ambiguity() {
                has_ambiguous_identity = true;
                ambiguity_affects_verdict |= retained.source_moves_debt();
            }
            self.report.add_comparison(retained);
            self.report.link_comparison(self.spot.scope, comparison_id);
            self.indexes.comparison += 1;
        }
        if has_ambiguous_identity {
            let mut diagnostic = Diagnostic::new(
                DiagnosticId::from_index(self.report.diagnostic_count()),
                Some(self.file),
                DiagnosticKind::AmbiguousIdentity,
                "anonymous units could not be matched safely",
                0,
            );
            if !ambiguity_affects_verdict {
                diagnostic = diagnostic.as_context();
            }
            self.report.add_diagnostic(diagnostic);
        }
    }

    /// Retains the advisory findings of a side the verdict never reads.
    fn add_advisory_findings(&mut self, selected: &DiffSide, policy: HealthPolicy) {
        let DiffSide::Analyzed { analysis, role, .. } = selected else {
            return;
        };
        if verdict_eligible(analysis, *role) {
            return;
        }
        for unit in analysis.units() {
            let assessment = policy.assess(unit.measurements());
            if assessment.rating() == Rating::Healthy {
                continue;
            }
            let id = FindingId::from_index(self.indexes.finding);
            self.report.add_finding(
                Finding::new(
                    id,
                    self.file,
                    unit.identity().clone(),
                    unit.span(),
                    unit.measurements(),
                    assessment,
                )
                .with_evidence(*role, analysis.parse_status().trust()),
            );
            self.report.link_finding(self.spot.scope, id);
            self.indexes.finding += 1;
        }
    }

    /// Adds the file record the selected side settles.
    fn add_file(&mut self, result: &DiffResult, selected: &DiffSide) {
        let (coverage, health, language) = if self.spot.selected {
            diff_side_summary(selected)
        } else {
            let (_, _, language) = diff_side_summary(selected);
            (Coverage::default(), HealthCounts::default(), language)
        };
        let mut file = FileRecord::new(
            self.file,
            self.spot.scope,
            result.change.current_path().to_string_lossy(),
            coverage,
            health,
        )
        .with_package(self.spot.package);
        if let Some(language) = language {
            file = file.with_language(language);
        }
        if let DiffSide::Analyzed { analysis, role, .. } = selected {
            file = file.with_source_state(*role, analysis.parse_status().clone());
        } else if let DiffSide::Unsupported { role, .. } | DiffSide::Failed { role, .. } = selected
        {
            file = file.with_source_state(*role, ParseStatus::Failed);
        }
        // A deleted file keeps its record, because its removed units and its
        // before measurements are half of every comparison it appears in. The
        // path is gone all the same, and the record says so.
        if !result.change.current_exists() {
            file = file.base_only();
        }
        self.report.add_file(file);
        self.report.link_file(self.spot.scope, self.file);
    }
}

pub(crate) fn add_diff_result(
    report: &mut AnalysisReportBuilder,
    result: DiffResult,
    indexes: &mut DiffIndexes,
    spot: DiffSpot,
    policy: HealthPolicy,
) {
    let file_id = FileId::from_index(result.index);
    let roles = (result.before.role(), result.current.role());
    let mut sink = ResultSink {
        report,
        indexes,
        file: file_id,
        spot,
    };
    sink.add_comparisons(&result.comparisons, roles);
    let selected = match &result.current {
        DiffSide::Missing => &result.before,
        current => current,
    };
    sink.add_advisory_findings(selected, policy);
    sink.add_file(&result, selected);
    add_diff_diagnostic(report, file_id, &result.current, "current");
    add_diff_diagnostic(report, file_id, &result.before, "base");
}
#[derive(Default)]
pub(crate) struct DiffIndexes {
    pub(crate) comparison: usize,
    pub(crate) finding: usize,
}
pub(crate) fn diff_side_summary(side: &DiffSide) -> (Coverage, HealthCounts, Option<Language>) {
    match side {
        DiffSide::Missing => (
            Coverage::new(1, 0, 0, 0, 0, 0),
            HealthCounts::default(),
            None,
        ),
        DiffSide::Unsupported {
            language,
            size_bytes,
            ..
        } => (
            Coverage::classified(1, SourceCoverageOutcome::Unsupported, 0, 0)
                .with_bytes(*size_bytes, *size_bytes),
            HealthCounts::default(),
            Some(*language),
        ),
        DiffSide::Failed { size_bytes, .. } => (
            Coverage::classified(1, SourceCoverageOutcome::Failed, 0, 0).with_bytes(*size_bytes, 0),
            HealthCounts::default(),
            None,
        ),
        DiffSide::RoleConflict { .. } => (
            Coverage::classified(1, SourceCoverageOutcome::Failed, 0, 0),
            HealthCounts::default(),
            None,
        ),
        DiffSide::Analyzed {
            analysis,
            health,
            role,
            size_bytes,
        } => (
            source_coverage(analysis, *role).with_bytes(*size_bytes, 0),
            *health,
            Some(analysis.language()),
        ),
    }
}
pub(crate) fn add_diff_diagnostic(
    report: &mut AnalysisReportBuilder,
    file: FileId,
    side: &DiffSide,
    label: &str,
) {
    let (kind, message) = match side {
        DiffSide::Unsupported { language, .. } => (
            DiagnosticKind::UnsupportedLanguage,
            format!("{label} side uses unsupported language: {language:?}"),
        ),
        DiffSide::Failed { message, .. } => {
            (DiagnosticKind::Other, format!("{label} side: {message}"))
        }
        DiffSide::Analyzed { analysis, .. }
            if matches!(analysis.parse_status(), ParseStatus::Failed) =>
        {
            (
                DiagnosticKind::ParseFailure,
                format!("{label} side parser failed"),
            )
        }
        DiffSide::Analyzed { analysis, .. }
            if matches!(analysis.parse_status(), ParseStatus::Recovered(_)) =>
        {
            (
                DiagnosticKind::ParseFailure,
                format!("{label} side parser recovered from syntax errors"),
            )
        }
        DiffSide::RoleConflict { .. } => return,
        DiffSide::Missing | DiffSide::Analyzed { .. } => return,
    };
    report.add_diagnostic(Diagnostic::new(
        DiagnosticId::from_index(report.diagnostic_count()),
        Some(file),
        kind,
        message,
        0,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::analyze_diff;
    use crate::requests::DiffRequest;
    use crate::test_support::git;
    use smackdebt_analysis::DiffTier;
    use smackdebt_analysis::{HealthPolicy, Thresholds};
    use std::fs;

    #[test]
    fn a_real_diff_answers_from_moved_debt_and_never_from_healthy_additions() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(
            repository_path.join("Cargo.toml"),
            "[package]\nname='verdicts'\nversion='0.1.0'\n",
        )
        .unwrap();
        let complex = "pub fn work(a: i32) -> i32 { if a > 0 { if a > 1 { return 1; } } 0 }\n";
        fs::write(repository_path.join("work.rs"), complex).unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "complex base"]);
        let request = DiffRequest::new(repository_path)
            .with_reference("HEAD")
            .with_thresholds(HealthPolicy::new(
                Thresholds::new(1, 2),
                Thresholds::new(5, 10),
                Thresholds::new(50, 100),
                Thresholds::new(4, 7),
                Thresholds::new(6, 9),
            ));

        fs::write(
            repository_path.join("work.rs"),
            format!("{complex}pub fn helper() -> i32 {{ 1 }}\n"),
        )
        .unwrap();
        let added = analyze_diff(&request).unwrap();
        let verdict = added.report().verdict().unwrap();
        assert_eq!(verdict.diff_tier(), Some(DiffTier::NoDebtChange));
        assert_eq!(verdict.sentence(), "No debt changed.");
        assert!(verdict.selection().is_empty());
        // The healthy addition stays in the machine report while it moves
        // nothing.
        assert!(
            added
                .report()
                .comparisons()
                .iter()
                .any(|comparison| comparison.kind() == smackdebt_analysis::ComparisonKind::Added)
        );

        fs::write(
            repository_path.join("work.rs"),
            "pub fn work(a: i32) -> i32 { a }\n",
        )
        .unwrap();
        let improved = analyze_diff(&request).unwrap();
        assert_eq!(
            improved.report().verdict().unwrap().diff_tier(),
            Some(DiffTier::Better)
        );

        fs::write(
            repository_path.join("work.rs"),
            format!(
                "pub fn work(a: i32) -> i32 {{ a }}\n{}",
                complex.replace("work", "later")
            ),
        )
        .unwrap();
        let mixed = analyze_diff(&request).unwrap();
        let verdict = mixed.report().verdict().unwrap();
        assert_eq!(verdict.diff_tier(), Some(DiffTier::Mixed));
        assert_eq!(
            verdict.sentence(),
            "Debt increased in some places and decreased in others."
        );
        assert_eq!(verdict.facts().source().worse(), 1);
        assert_eq!(verdict.facts().source().better(), 1);
        assert!(
            verdict
                .facts()
                .moved(smackdebt_analysis::DebtFamily::Source)
        );
        assert!(
            !verdict
                .facts()
                .moved(smackdebt_analysis::DebtFamily::Architecture)
        );
        assert!(!verdict.selection().has_duplicate_identity());
    }
    #[test]
    fn a_real_diff_pairs_only_safe_anonymous_units() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(
            repository_path.join("callbacks.js"),
            "watch('ready', () => work());\nrepeat(() => same());\ngone(() => old());\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "callback base"]);
        fs::write(
            repository_path.join("callbacks.js"),
            "\nwatch('ready', () => { if (ready) work(); });\nrepeat(() => same());\nrepeat(() => same());\nonly(() => new_one());\nonly(() => new_one());\n",
        )
        .unwrap();

        let report =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let report = report.report();
        let kinds: Vec<_> = report
            .comparisons()
            .iter()
            .map(smackdebt_analysis::Comparison::kind)
            .collect();
        assert_eq!(
            kinds
                .iter()
                .filter(|kind| **kind == smackdebt_analysis::ComparisonKind::Ambiguous)
                .count(),
            1
        );
        assert_eq!(
            kinds
                .iter()
                .filter(|kind| **kind == smackdebt_analysis::ComparisonKind::Added)
                .count(),
            2
        );
        assert_eq!(
            kinds
                .iter()
                .filter(|kind| **kind == smackdebt_analysis::ComparisonKind::Removed)
                .count(),
            1
        );
        assert_eq!(
            kinds
                .iter()
                .filter(|kind| **kind == smackdebt_analysis::ComparisonKind::MetricChanged)
                .count(),
            1
        );
        let diagnostics: Vec<_> = report
            .diagnostics()
            .iter()
            .filter(|value| value.kind() == DiagnosticKind::AmbiguousIdentity)
            .collect();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].file(), Some(FileId::from_index(0)));
        assert!(
            report
                .comparisons()
                .iter()
                .any(Comparison::is_anonymous_ambiguity)
        );
        assert_eq!(
            report.verdict().unwrap().diff_tier(),
            Some(DiffTier::NoDebtChange)
        );
        assert!(
            !report
                .verdict()
                .unwrap()
                .selection()
                .has_duplicate_identity()
        );
    }
    #[test]
    fn repeated_declared_names_do_not_create_an_anonymous_warning() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(
            repository_path.join("duplicate.js"),
            "function same() { return 1; }\nfunction same() { return 2; }\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "duplicate base"]);
        fs::write(
            repository_path.join("duplicate.js"),
            "function same() { return 1; }\nfunction same() { if (ready) return 2; }\n",
        )
        .unwrap();

        let report =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let report = report.report();
        assert_eq!(report.comparisons().len(), 1);
        assert_eq!(
            report.comparisons()[0].kind(),
            smackdebt_analysis::ComparisonKind::Ambiguous
        );
        assert!(!report.comparisons()[0].is_anonymous_ambiguity());
        assert!(
            report
                .diagnostics()
                .iter()
                .all(|value| value.kind() != DiagnosticKind::AmbiguousIdentity)
        );
    }
}
