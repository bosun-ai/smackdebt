//! Rating one analyzed file: the health its measurements earn under the
//! selected policy, and the result shapes the flows share.

use std::path::PathBuf;

use crate::requests::SourceRoleRule;
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
/// How one tree's source is judged: the health thresholds every unit is
/// rated under and the configured role rules.
#[derive(Clone, Copy)]
pub(crate) struct SourcePolicy<'a> {
    pub(crate) health: HealthPolicy,
    pub(crate) rules: &'a [SourceRoleRule],
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebase::analyze_codebase;
    use crate::requests::CodebaseRequest;
    use crate::test_support::git;
    use smackdebt_analysis::FileActivity;
    use smackdebt_analysis::FileId;
    use smackdebt_analysis::SourceTrust;
    use smackdebt_analysis::{HealthPolicy, Thresholds};
    use smackdebt_languages::Analyzer;
    use std::fs;
    use std::path::Path;

    #[test]
    fn every_source_role_is_retained_and_only_verdict_roles_change_health() {
        let mut analyzer = Analyzer::default();
        let source = b"function work(a, b) { if (a) { if (b) { return 1; } } return 0; }\n";
        let policy = HealthPolicy::new(
            smackdebt_analysis::Thresholds::new(1, 2),
            smackdebt_analysis::Thresholds::new(1, 2),
            smackdebt_analysis::Thresholds::new(1, 2),
            smackdebt_analysis::Thresholds::new(4, 7),
            smackdebt_analysis::Thresholds::new(6, 9),
        );
        for role in [
            SourceRole::Primary,
            SourceRole::Test,
            SourceRole::Example,
            SourceRole::Benchmark,
            SourceRole::Fixture,
            SourceRole::Generated,
        ] {
            let analysis = analyzer
                .analyze(Path::new("src/work.js"), source.to_vec())
                .unwrap();
            let rated = rate_file(analysis, role, policy, false);
            assert_eq!(rated.role, role);
            assert!(!rated.debt.is_empty());
            if role.affects_verdict() {
                assert!(rated.health.debt() > 0, "{role:?}");
            } else {
                assert_eq!(rated.health.debt(), 0, "{role:?}");
            }
        }
    }
    #[test]
    fn workspace_test_fixture_has_one_role_and_stays_outside_the_verdict() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("crates/cli/tests/fixtures")).unwrap();
        fs::write(
            root.path().join("crates/cli/Cargo.toml"),
            "[package]\nname='fixture'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            root.path().join("crates/cli/tests/fixtures/complex.js"),
            "export function fixture(a, b) { if (a) { if (b) { return 1; } } return 0; }\n",
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
        let fixture = report
            .files()
            .iter()
            .find(|file| file.path() == "crates/cli/tests/fixtures/complex.js")
            .unwrap();

        assert_eq!(fixture.role(), SourceRole::Fixture);
        assert_eq!(fixture.health(), HealthCounts::default());
        assert_eq!(fixture.coverage().context_files(), 1);
        assert_eq!(
            report.scopes()[report.root().unwrap().index()].health(),
            HealthCounts::default()
        );
    }
    #[test]
    fn recovered_findings_are_advisory_and_do_not_change_health() {
        let analysis = Analyzer::default()
            .analyze(
                Path::new("src/work.py"),
                b"def broken(:\n    if yes:\n        if more:\n            pass\n".to_vec(),
            )
            .unwrap();
        assert_eq!(analysis.parse_status().trust(), SourceTrust::Advisory);
        let rated = rate_file(
            analysis,
            SourceRole::Primary,
            HealthPolicy::new(
                smackdebt_analysis::Thresholds::new(1, 2),
                smackdebt_analysis::Thresholds::new(1, 2),
                smackdebt_analysis::Thresholds::new(1, 2),
                smackdebt_analysis::Thresholds::new(4, 7),
                smackdebt_analysis::Thresholds::new(6, 9),
            ),
            false,
        );
        assert_eq!(rated.health, HealthCounts::default());
        assert!(!rated.debt.is_empty());
    }
    #[test]
    fn advisory_and_context_source_produce_no_size_finding_or_hotspot() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::write(
            repository_path.join("trusted.rs"),
            "struct Worker;\nimpl Worker {\n    fn work(&self) {\n        let a = 1;\n        let b = 2;\n    }\n}\n",
        )
        .unwrap();
        // Recovered source is advisory, so it contributes no default finding.
        fs::write(
            repository_path.join("recovered.rs"),
            "struct Broken;\nimpl Broken {\n    fn work(&self) {\n        let a = 1;\n        let b = 2;\n    }\n}\nfn unterminated(\n",
        )
        .unwrap();
        // Generated source is context, so it never affects a verdict.
        fs::write(
            repository_path.join("generated.rs"),
            "// @generated\nstruct Made;\nimpl Made {\n    fn work(&self) {\n        let a = 1;\n        let b = 2;\n    }\n}\n",
        )
        .unwrap();
        for revision in 0..5 {
            fs::write(
                repository_path.join("touch.txt"),
                format!("change {revision}\n"),
            )
            .unwrap();
            fs::write(
                repository_path.join("recovered.rs"),
                format!(
                    "struct Broken;\nimpl Broken {{\n    fn work(&self) {{\n        let a = {revision};\n        let b = 2;\n    }}\n}}\nfn unterminated(\n"
                ),
            )
            .unwrap();
            fs::write(
                repository_path.join("generated.rs"),
                format!(
                    "// @generated\nstruct Made;\nimpl Made {{\n    fn work(&self) {{\n        let a = {revision};\n        let b = 2;\n    }}\n}}\n"
                ),
            )
            .unwrap();
            fs::write(
                repository_path.join("trusted.rs"),
                format!(
                    "struct Worker;\nimpl Worker {{\n    fn work(&self) {{\n        let a = {revision};\n        let b = 2;\n    }}\n}}\n"
                ),
            )
            .unwrap();
            git(repository_path, ["add", "."]);
            git(repository_path, ["commit", "-qm", "change"]);
        }

        let analyzed = analyze_codebase(
            &CodebaseRequest::new(repository_path).with_size_thresholds((5, 10), (1, 2)),
        )
        .unwrap();
        let report = analyzed.report();
        let path = |file: FileId| report.files()[file.index()].path().to_owned();
        let sized: Vec<_> = report
            .size_findings()
            .iter()
            .map(|finding| path(finding.file()))
            .collect();
        assert_eq!(
            sized,
            ["trusted.rs".to_owned(), "trusted.rs".to_owned()],
            "only trusted verdict source is sized"
        );
        let hot: Vec<_> = report
            .hotspots()
            .iter()
            .map(|hotspot| path(hotspot.file()))
            .collect();
        assert_eq!(hot, ["trusted.rs".to_owned()]);
        // Every file was touched five times, so activity alone did not decide.
        for name in ["recovered.rs", "generated.rs"] {
            let file = report
                .files()
                .iter()
                .find(|file| file.path() == name)
                .expect("selected file");
            assert_eq!(
                file.activity().map(FileActivity::touches),
                Some(5),
                "{name} still records its activity"
            );
        }
    }
}
