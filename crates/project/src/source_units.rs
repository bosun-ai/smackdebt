//! Reading and analyzing the current tree's files, serially or across the
//! one worker pool.

use std::fs;
use std::path::Path;
use std::sync::atomic::Ordering;

use rayon::prelude::*;
use smackdebt_analysis::{FileAnalysis, FileId, Language};
use smackdebt_discovery::{DiscoveredFile, Inventory};
use smackdebt_languages::{AnalysisError as LanguageError, Analyzer};

use crate::rating::{FileResult, SourcePolicy, rate_file};
use crate::requests::{ExecutionWidth, ProjectError, SourceRoleRule};
use crate::roles::{classify_source_role, declares_module_syntax, role_for_unavailable_source};
use crate::work::AnalysisWork;

pub(crate) const PARALLEL_FILE_CUTOVER: usize = 100;

/// One worker's analysis session: its reusable parser and the work counters
/// it reports into.
pub(crate) struct AnalysisSession<'a> {
    pub(crate) analyzer: &'a mut Analyzer,
    pub(crate) work: &'a AnalysisWork,
}

/// Reads and rates one discovered file.
fn analyze_one_file(
    session: &mut AnalysisSession<'_>,
    (index, file): (usize, &DiscoveredFile),
    inventory: &Inventory,
    policy: SourcePolicy<'_>,
) -> FileResult {
    let path = file.path().as_path();
    let language = Analyzer::language(path);
    if matches!(
        language,
        Language::Astro | Language::Kotlin | Language::Unknown
    ) {
        return match role_for_unavailable_source(path, policy.rules) {
            Ok(role) => FileResult::Unsupported { language, role },
            Err(roles) => conflict(path, roles),
        };
    }
    let Some(absolute) = inventory.absolute_path(file.path()) else {
        return failed_result(
            path,
            policy.rules,
            language,
            "source path escaped the selected root".to_owned(),
        );
    };
    match fs::read(&absolute) {
        Ok(source) => {
            session.work.source_reads.fetch_add(1, Ordering::Relaxed);
            #[cfg(feature = "evidence-stats")]
            crate::evidence::record_source_read();
            rate_source(session, FileId::from_index(index), path, source, policy)
        }
        Err(error) => failed_result(path, policy.rules, language, error.to_string()),
    }
}

/// The conflict result that stops composition when rules disagree.
fn conflict(path: &Path, roles: String) -> FileResult {
    FileResult::RoleConflict {
        path: path.to_path_buf(),
        roles,
    }
}

/// A failure result carrying the role the path rules settle.
fn failed_result(
    path: &Path,
    rules: &[SourceRoleRule],
    language: Language,
    message: String,
) -> FileResult {
    match role_for_unavailable_source(path, rules) {
        Ok(role) => FileResult::Failed {
            message,
            role,
            language,
        },
        Err(roles) => conflict(path, roles),
    }
}

/// Classifies and parses one read source.
fn rate_source(
    session: &mut AnalysisSession<'_>,
    file: FileId,
    path: &Path,
    source: Vec<u8>,
    policy: SourcePolicy<'_>,
) -> FileResult {
    let role = match classify_source_role(path, &source, policy.rules) {
        Ok(role) => role,
        Err(roles) => {
            return FileResult::RoleConflict {
                path: path.to_path_buf(),
                roles,
            };
        }
    };
    let module_syntax = declares_module_syntax(path, &source);
    match analyze_bytes(session.analyzer, file, path, source, session.work) {
        Ok(value) => FileResult::Analyzed(rate_file(value, role, policy.health, module_syntax)),
        Err(LanguageError::Unsupported(language)) => FileResult::Unsupported { language, role },
        Err(error) => FileResult::Failed {
            message: error.to_string(),
            role,
            language: Analyzer::language(path),
        },
    }
}

pub(crate) fn analyze_current_files(
    inventory: &Inventory,
    candidates: &[&DiscoveredFile],
    width: ExecutionWidth,
    policy: SourcePolicy<'_>,
    work: &AnalysisWork,
) -> Result<Vec<FileResult>, ProjectError> {
    let analyze = |analyzer: &mut Analyzer, (index, file): (usize, &&DiscoveredFile)| {
        let mut session = AnalysisSession { analyzer, work };
        analyze_one_file(&mut session, (index, file), inventory, policy)
    };

    if candidates.len() < PARALLEL_FILE_CUTOVER || width.threads() == 1 {
        let mut analyzer = Analyzer::default();
        let results = candidates
            .iter()
            .enumerate()
            .map(|entry| analyze(&mut analyzer, entry))
            .collect::<Vec<_>>();
        return role_results(results);
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(width.threads())
        .build()?;
    let results = pool.install(|| {
        candidates
            .par_iter()
            .enumerate()
            .map_init(Analyzer::default, analyze)
            .collect()
    });
    role_results(results)
}
pub(crate) fn role_results(results: Vec<FileResult>) -> Result<Vec<FileResult>, ProjectError> {
    if let Some((path, roles)) = results.iter().find_map(|result| match result {
        FileResult::RoleConflict { path, roles } => Some((path.clone(), roles.clone())),
        _ => None,
    }) {
        Err(ProjectError::SourceRoleConflict { path, roles })
    } else {
        Ok(results)
    }
}
pub(crate) fn analyze_bytes(
    analyzer: &mut Analyzer,
    file: FileId,
    path: &Path,
    source: Vec<u8>,
    work: &AnalysisWork,
) -> Result<FileAnalysis, LanguageError> {
    let _ = file;
    let result = analyzer.analyze(path, source);
    if !matches!(result, Err(LanguageError::Unsupported(_))) {
        work.record_parser_visit();
        work.record_algorithm_pass();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebase::analyze_codebase;
    use crate::requests::CodebaseRequest;
    use crate::test_support::git;
    use smackdebt_analysis::{ProblemPattern, duplicate_claim};
    use std::collections::BTreeSet;
    use std::fs;

    #[test]
    fn serial_and_parallel_runs_derive_identical_signal_tables() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        // More files than the parallel cutover, so the automatic run really
        // splits the work across workers.
        for index in 0..120 {
            fs::write(
                repository_path.join(format!("file{index}.rs")),
                format!("pub fn work{index}(value: i32) -> i32 {{ value + {index} }}\n"),
            )
            .unwrap();
        }
        for revision in 0..6 {
            fs::write(
                repository_path.join("file0.rs"),
                format!("pub fn work0(value: i32) -> i32 {{ value + {revision} }}\n"),
            )
            .unwrap();
            git(repository_path, ["add", "."]);
            git(repository_path, ["commit", "-qm", "change"]);
        }

        let request = CodebaseRequest::new(repository_path).with_size_thresholds((1, 2), (1, 2));
        let serial = request
            .clone()
            .with_width(ExecutionWidth::fixed(1).unwrap())
            .analyze()
            .unwrap();
        let parallel = request
            .with_width(ExecutionWidth::Automatic)
            .analyze()
            .unwrap();
        assert!(!serial.report().hotspots().is_empty());
        assert!(!serial.report().size_findings().is_empty());
        assert!(!serial.report().orphan_files().is_empty());
        assert_eq!(serial.report().hotspots(), parallel.report().hotspots());
        assert_eq!(
            serial.report().size_findings(),
            parallel.report().size_findings()
        );
        assert_eq!(
            serial.report().orphan_files(),
            parallel.report().orphan_files()
        );
        assert_eq!(
            serial.report().stable_dependency_findings(),
            parallel.report().stable_dependency_findings()
        );
        assert_eq!(
            serial.report().knowledge_concentration_findings(),
            parallel.report().knowledge_concentration_findings()
        );
        // The verdict is derived from those tables, so both widths answer with
        // the same tier, counts, selection, and worst offender.
        assert!(serial.report().verdict().is_some());
        assert_eq!(serial.report().verdict(), parallel.report().verdict());
        assert!(
            !serial
                .report()
                .verdict()
                .unwrap()
                .selection()
                .has_duplicate_identity()
        );
    }
    /// A function whose nesting alone rates High on cognitive complexity.
    fn nested_source(seed: usize) -> String {
        let mut source = format!("pub fn work{seed}(value: i32) -> i32 {{\n");
        for depth in 0..8 {
            source.push_str(&format!(
                "{}if value > {depth} {{\n",
                "    ".repeat(depth + 1)
            ));
        }
        source.push_str(&format!("{}return 1;\n", "    ".repeat(9)));
        for depth in (0..8).rev() {
            source.push_str(&format!("{}}}\n", "    ".repeat(depth + 1)));
        }
        source.push_str("    value\n}\n");
        source
    }
    #[test]
    fn serial_and_parallel_runs_cluster_identical_problem_cards() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        // More files than the parallel cutover, so the automatic run really
        // splits the work across workers, and every sixth file carries High
        // debt so cards exist to compare.
        for index in 0..120 {
            let source = if index % 6 == 0 {
                nested_source(index)
            } else {
                format!("pub fn work{index}(value: i32) -> i32 {{ value + {index} }}\n")
            };
            fs::write(repository_path.join(format!("file{index}.rs")), source).unwrap();
        }
        // One file changes often enough to be hot, so heat reaches a card too.
        for revision in 0..6 {
            fs::write(repository_path.join("file0.rs"), nested_source(revision)).unwrap();
            git(repository_path, ["add", "."]);
            git(repository_path, ["commit", "-qm", "change"]);
        }

        let request = CodebaseRequest::new(repository_path).with_size_thresholds((1, 2), (1, 2));
        let serial = request
            .clone()
            .with_width(ExecutionWidth::fixed(1).unwrap())
            .analyze()
            .unwrap();
        let parallel = request
            .with_width(ExecutionWidth::Automatic)
            .analyze()
            .unwrap();
        let cards = serial.report().problems();
        assert!(!cards.is_empty());
        // Clustering reads report tables only, so width cannot move a card or
        // its position.
        assert_eq!(cards, parallel.report().problems());
        assert_eq!(duplicate_claim(cards), None);
        assert!(
            cards
                .iter()
                .any(|card| card.pattern() == ProblemPattern::HotMess),
            "the file that changes often carries its heat into a card"
        );
        // Coverage over a real report: every retained finding of every
        // claimable table reaches exactly one card, so no table can be dropped
        // from the clustering input without this failing.
        let report = serial.report();
        let claimed: BTreeSet<_> = cards
            .iter()
            .flat_map(|card| card.claimed_findings().iter().copied())
            .collect();
        assert_eq!(
            claimed.len(),
            report.findings().len()
                + report.size_findings().len()
                + report.architecture_findings().len()
                + report.evolutionary_findings().len()
                + report.knowledge_concentration_findings().len()
                + report.stable_dependency_findings().len()
        );
        assert!(!report.size_findings().is_empty());
    }
    #[test]
    fn one_file_is_read_once_and_produces_a_report() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("sample.rb"),
            "def work\n  if ready\n    go\n  end\nend\n",
        )
        .unwrap();
        let result = analyze_codebase(
            &CodebaseRequest::new(root.path()).with_width(ExecutionWidth::fixed(1).unwrap()),
        )
        .unwrap();
        assert_eq!(result.stats().inventory_walks, 1);
        assert_eq!(result.stats().source_reads, 1);
        assert_eq!(result.report().files().len(), 1);
        assert_eq!(result.report().packages().len(), 1);
        assert_eq!(result.report().packages()[0].path(), ".");
    }
}
