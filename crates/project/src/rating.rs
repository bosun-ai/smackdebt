//! Rating one analyzed file: the health its measurements earn under the
//! selected policy, and the result shapes the flows share.

use std::path::PathBuf;

use smackdebt_analysis::{
    Coverage, DependencySyntax, FileAnalysis, HealthAssessment, HealthCounts, HealthPolicy,
    Language, ParseStatus, Rating, SourceCoverageOutcome, SourceRole,
};

pub(crate) fn source_coverage(analysis: &FileAnalysis, role: SourceRole) -> Coverage {
    let outcome = match analysis.parse_status() {
        ParseStatus::Parsed if role.affects_verdict() => SourceCoverageOutcome::Clean,
        ParseStatus::Parsed => SourceCoverageOutcome::Context,
        ParseStatus::Recovered(_) => SourceCoverageOutcome::Recovered,
        ParseStatus::Failed => SourceCoverageOutcome::Failed,
    };
    Coverage::classified(
        1,
        outcome,
        analysis.source_lines(),
        u32::from(matches!(outcome, SourceCoverageOutcome::Failed)) * analysis.source_lines(),
    )
}
pub(crate) struct RatedFile {
    pub(crate) analysis: FileAnalysis,
    pub(crate) role: SourceRole,
    /// Whether the source states its own imports and exports.
    pub(crate) module_syntax: bool,
    pub(crate) health: HealthCounts,
    pub(crate) debt: Vec<(usize, HealthAssessment)>,
    /// Whether this file's facts may produce default signals.
    ///
    /// Only trusted source in a verdict role feeds the hotspot and size
    /// tables: recovered facts stay advisory, and fixture or generated source
    /// stays context. Retained findings are unaffected, so advisory and context
    /// debt remains visible.
    pub(crate) signals_verdict: bool,
    pub(crate) rated_units: u32,
    pub(crate) max_rating: Rating,
    pub(crate) container_statements: Vec<(String, u32)>,
}
pub(crate) fn rate_file(
    analysis: FileAnalysis,
    role: SourceRole,
    policy: HealthPolicy,
    module_syntax: bool,
) -> RatedFile {
    let mut health = HealthCounts::default();
    let mut debt = Vec::new();
    let mut max_rating = Rating::Healthy;
    // Container totals accumulate while units are rated, so no healthy unit is
    // retained to compute container size later.
    let mut container_statements: Vec<(String, u32)> = Vec::new();
    let signals_verdict = verdict_eligible(&analysis, role);
    for (index, unit) in analysis.units().iter().enumerate() {
        let assessment = policy.assess(unit.measurements());
        if assessment.rating() != Rating::Healthy {
            debt.push((index, assessment));
        }
        if !signals_verdict {
            continue;
        }
        health.add_rating(assessment.rating());
        if assessment.rating() > max_rating {
            max_rating = assessment.rating();
        }
        if let Some(container) = unit.identity().container() {
            let statements = unit.measurements().logical_lines();
            match container_statements
                .iter_mut()
                .find(|(name, _)| name == container)
            {
                Some(total) => total.1 += statements,
                None => container_statements.push((container.to_owned(), statements)),
            }
        }
    }
    let rated_units = if signals_verdict {
        u32::try_from(analysis.units().len()).unwrap_or(u32::MAX)
    } else {
        0
    };
    RatedFile {
        analysis,
        role,
        module_syntax,
        health,
        debt,
        signals_verdict,
        rated_units,
        max_rating,
        container_statements,
    }
}
pub(crate) enum FileResult {
    Analyzed(RatedFile),
    Unsupported {
        language: Language,
        role: SourceRole,
    },
    Failed {
        message: String,
        role: SourceRole,
        language: Language,
    },
    RoleConflict {
        path: PathBuf,
        roles: String,
    },
}
impl FileResult {
    pub(crate) fn references(&self) -> &[DependencySyntax] {
        match self {
            Self::Analyzed(rated) => rated.analysis.dependencies(),
            _ => &[],
        }
    }

    /// Reclassifies a file the build compiles only under `test`.
    ///
    /// Rating never changes: a test file is still verdict eligible, so the
    /// health and debt already computed for it stay correct.
    pub(crate) fn demote_to_test(&mut self) {
        let role = match self {
            Self::Analyzed(rated) => &mut rated.role,
            Self::Unsupported { role, .. } | Self::Failed { role, .. } => role,
            Self::RoleConflict { .. } => return,
        };
        *role = role.demoted_by_test_scope();
    }
}
pub(crate) fn file_result_role(result: &FileResult) -> SourceRole {
    match result {
        FileResult::Analyzed(rated) => rated.role,
        FileResult::Unsupported { role, .. } | FileResult::Failed { role, .. } => *role,
        FileResult::RoleConflict { .. } => unreachable!("role conflicts stop composition"),
    }
}
pub(crate) fn verdict_eligible(analysis: &FileAnalysis, role: SourceRole) -> bool {
    role.affects_verdict() && matches!(analysis.parse_status(), ParseStatus::Parsed)
}
pub(crate) fn rated_health(
    analysis: &FileAnalysis,
    role: SourceRole,
    policy: HealthPolicy,
) -> HealthCounts {
    let mut health = HealthCounts::default();
    if verdict_eligible(analysis, role) {
        for unit in analysis.units() {
            health.add_rating(policy.assess(unit.measurements()).rating());
        }
    }
    health
}
