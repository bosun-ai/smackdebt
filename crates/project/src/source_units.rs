//! Reading and analyzing the current tree's files, serially or across the
//! one worker pool.

use std::fs;
use std::path::Path;
use std::sync::atomic::Ordering;

use rayon::prelude::*;
use smackdebt_analysis::{FileAnalysis, FileId, HealthPolicy, Language};
use smackdebt_discovery::{DiscoveredFile, Inventory};
use smackdebt_languages::{AnalysisError as LanguageError, Analyzer};

use crate::rating::{FileResult, rate_file};
use crate::requests::{ExecutionWidth, ProjectError, SourceRoleRule};
use crate::roles::{classify_source_role, declares_module_syntax, role_for_unavailable_source};
use crate::work::AnalysisWork;

pub(crate) const PARALLEL_FILE_CUTOVER: usize = 100;

pub(crate) fn analyze_current_files(
    inventory: &Inventory,
    candidates: &[&DiscoveredFile],
    width: ExecutionWidth,
    policy: HealthPolicy,
    role_rules: &[SourceRoleRule],
    work: &AnalysisWork,
) -> Result<Vec<FileResult>, ProjectError> {
    let analyze = |analyzer: &mut Analyzer, (index, file): (usize, &&DiscoveredFile)| {
        let path = file.path().as_path();
        let language = Analyzer::language(path);
        if matches!(
            language,
            Language::Astro | Language::Kotlin | Language::Unknown
        ) {
            return match role_for_unavailable_source(path, role_rules) {
                Ok(role) => FileResult::Unsupported { language, role },
                Err(roles) => FileResult::RoleConflict {
                    path: path.to_path_buf(),
                    roles,
                },
            };
        }
        let Some(absolute) = inventory.absolute_path(file.path()) else {
            return match role_for_unavailable_source(path, role_rules) {
                Ok(role) => FileResult::Failed {
                    message: "source path escaped the selected root".to_owned(),
                    role,
                    language,
                },
                Err(roles) => FileResult::RoleConflict {
                    path: path.to_path_buf(),
                    roles,
                },
            };
        };
        match fs::read(&absolute) {
            Ok(source) => {
                work.source_reads.fetch_add(1, Ordering::Relaxed);
                #[cfg(feature = "evidence-stats")]
                crate::evidence::record_source_read();
                let role = match classify_source_role(path, &source, role_rules) {
                    Ok(role) => role,
                    Err(roles) => {
                        return FileResult::RoleConflict {
                            path: path.to_path_buf(),
                            roles,
                        };
                    }
                };
                let module_syntax = declares_module_syntax(path, &source);
                match analyze_bytes(analyzer, FileId::from_index(index), path, source, work) {
                    Ok(value) => {
                        FileResult::Analyzed(rate_file(value, role, policy, module_syntax))
                    }
                    Err(LanguageError::Unsupported(language)) => {
                        FileResult::Unsupported { language, role }
                    }
                    Err(error) => FileResult::Failed {
                        message: error.to_string(),
                        role,
                        language,
                    },
                }
            }
            Err(error) => match role_for_unavailable_source(path, role_rules) {
                Ok(role) => FileResult::Failed {
                    message: error.to_string(),
                    role,
                    language,
                },
                Err(roles) => FileResult::RoleConflict {
                    path: path.to_path_buf(),
                    roles,
                },
            },
        }
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
