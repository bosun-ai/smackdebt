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
pub(crate) fn add_diff_result(
    report: &mut AnalysisReportBuilder,
    result: DiffResult,
    indexes: &mut DiffIndexes,
    scope_id: ScopeId,
    package: PackageId,
    included_in_code_diff: bool,
    policy: HealthPolicy,
) {
    let file_id = FileId::from_index(result.index);
    let before_role = result.before.role();
    let current_role = result.current.role();
    let mut has_ambiguous_identity = false;
    let mut ambiguity_affects_verdict = false;

    for comparison in result.comparisons.iter().filter(|_| included_in_code_diff) {
        let comparison_id = ComparisonId::from_index(indexes.comparison);
        let retained = retained_comparison(
            comparison,
            comparison_id,
            file_id,
            (before_role, current_role),
        );
        if retained.is_anonymous_ambiguity() {
            has_ambiguous_identity = true;
            ambiguity_affects_verdict |= retained.source_moves_debt();
        }
        report.add_comparison(retained);
        report.link_comparison(scope_id, comparison_id);
        indexes.comparison += 1;
    }
    if has_ambiguous_identity {
        let mut diagnostic = Diagnostic::new(
            DiagnosticId::from_index(report.diagnostic_count()),
            Some(file_id),
            DiagnosticKind::AmbiguousIdentity,
            "anonymous units could not be matched safely",
            0,
        );
        if !ambiguity_affects_verdict {
            diagnostic = diagnostic.as_context();
        }
        report.add_diagnostic(diagnostic);
    }

    let selected = match &result.current {
        DiffSide::Missing => &result.before,
        current => current,
    };
    if let DiffSide::Analyzed { analysis, role, .. } = selected
        && !verdict_eligible(analysis, *role)
    {
        for unit in analysis.units() {
            let assessment = policy.assess(unit.measurements());
            if assessment.rating() == Rating::Healthy {
                continue;
            }
            let id = FindingId::from_index(indexes.finding);
            report.add_finding(
                Finding::new(
                    id,
                    file_id,
                    unit.identity().clone(),
                    unit.span(),
                    unit.measurements(),
                    assessment,
                )
                .with_evidence(*role, analysis.parse_status().trust()),
            );
            report.link_finding(scope_id, id);
            indexes.finding += 1;
        }
    }
    let (coverage, health, language) = if included_in_code_diff {
        diff_side_summary(selected)
    } else {
        let (_, _, language) = diff_side_summary(selected);
        (Coverage::default(), HealthCounts::default(), language)
    };
    let mut file = FileRecord::new(
        file_id,
        scope_id,
        result.change.current_path().to_string_lossy(),
        coverage,
        health,
    )
    .with_package(package);
    if let Some(language) = language {
        file = file.with_language(language);
    }
    if let DiffSide::Analyzed { analysis, role, .. } = selected {
        file = file.with_source_state(*role, analysis.parse_status().clone());
    } else if let DiffSide::Unsupported { role, .. } | DiffSide::Failed { role, .. } = selected {
        file = file.with_source_state(*role, ParseStatus::Failed);
    }
    // A deleted file keeps its record, because its removed units and its before
    // measurements are half of every comparison it appears in. The path is gone
    // all the same, and the record says so.
    if !result.change.current_exists() {
        file = file.base_only();
    }
    report.add_file(file);
    report.link_file(scope_id, file_id);
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
