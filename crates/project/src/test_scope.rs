//! Test-declared demotion: a file the build compiles only under `test`
//! carries a test role on the side that declares it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use smackdebt_analysis::{
    DependencySyntax, DependencySyntaxState, FileId, ModuleDeclaration, SourceRole,
    test_declared_files,
};
use smackdebt_discovery::DiscoveredFile;

use crate::candidates::resolve_candidates;
use crate::dependencies::DiffSideSelector;
use crate::diff_changes::DiffAliases;
use crate::diff_source::{DiffResult, DiffUnchanged};
use crate::rating::FileResult;
use crate::requests::SourceRoleRule;
use crate::resolution_config::ResolutionRules;
use crate::roles::matching_role_rules;

/// Reclassifies test-declared files on both sides of the diff.
pub(crate) fn demote_diff_roles(
    results: &mut [DiffResult],
    unchanged: &mut DiffUnchanged<'_>,
    aliases: DiffAliases<'_>,
    rules: &[SourceRoleRule],
) {
    for side in [DiffSideSelector::Current, DiffSideSelector::Before] {
        demote_test_declared_diff_roles(
            side,
            unchanged.first_file_index,
            results,
            &unchanged.candidates,
            &mut unchanged.results,
            &mut unchanged.before_roles,
            DiffRolePolicy {
                aliases: aliases.select(side),
                rules,
            },
        );
    }
}
/// The module declarations one side of an analysis states.
///
/// A declaration is a `module_ownership` relation, so this reads the same
/// references the architecture pass resolves later, before any role is read.
pub(crate) fn module_declarations<'a>(
    sources: impl Iterator<Item = (usize, &'a Path, &'a [DependencySyntax])>,
    index: &BTreeMap<PathBuf, FileId>,
    aliases: &ResolutionRules,
) -> BTreeSet<ModuleDeclaration> {
    let mut declarations = BTreeSet::new();
    for (declarer, path, references) in sources {
        for reference in references {
            if reference.relation() != smackdebt_analysis::StaticRelationKind::ModuleOwnership {
                continue;
            }
            let DependencySyntaxState::Candidates(candidates) = reference.state() else {
                continue;
            };
            let test_scoped = reference.scope() == smackdebt_analysis::DependencyScope::Test;
            for target in resolve_candidates(path, candidates, index, aliases) {
                if target.index() != declarer {
                    declarations.insert((declarer, target.index(), test_scoped));
                }
            }
        }
    }
    declarations
}
/// The files a pass reclassifies because the build only compiles them when
/// `test` is set.
///
/// This resolves the module declarations of one file table, runs the
/// test-declared fixpoint over them, and drops every file explicit
/// configuration already claims — configuration keeps precedence over every
/// later rule.
pub(crate) fn test_declared_demotions<P: AsRef<Path>>(
    paths: &[P],
    references: &[&[DependencySyntax]],
    aliases: &ResolutionRules,
    rules: &[SourceRoleRule],
) -> BTreeSet<usize> {
    debug_assert_eq!(paths.len(), references.len());
    let index: BTreeMap<PathBuf, FileId> = paths
        .iter()
        .enumerate()
        .map(|(index, path)| (path.as_ref().to_path_buf(), FileId::from_index(index)))
        .collect();
    let declarations = module_declarations(
        paths
            .iter()
            .zip(references)
            .enumerate()
            .map(|(index, (path, references))| (index, path.as_ref(), *references)),
        &index,
        aliases,
    );
    test_declared_files(&declarations)
        .into_iter()
        .filter(|file| {
            matching_role_rules(paths[*file].as_ref(), rules)
                .next()
                .is_none()
        })
        .collect()
}
/// Reclassifies every file the build compiles only when `test` is set.
///
/// This runs before any role reaches the report builder, so findings, ratings,
/// coverage, history evidence, and the architecture graphs all read one role
/// per file.
pub(crate) fn demote_test_declared_roles(
    paths: &[&Path],
    results: &mut [FileResult],
    aliases: &ResolutionRules,
    rules: &[SourceRoleRule],
) {
    let demotions = {
        let references: Vec<&[DependencySyntax]> =
            results.iter().map(FileResult::references).collect();
        test_declared_demotions(paths, &references, aliases, rules)
    };
    for file in demotions {
        results[file].demote_to_test();
    }
}
/// Reclassifies test-declared files on one side of a diff.
///
/// Changed and unchanged files retain a role for each tree independently.
pub(crate) struct DiffRolePolicy<'a> {
    pub(crate) aliases: &'a ResolutionRules,
    pub(crate) rules: &'a [SourceRoleRule],
}
pub(crate) fn demote_test_declared_diff_roles(
    side: DiffSideSelector,
    changed_count: usize,
    results: &mut [DiffResult],
    unchanged_candidates: &[&DiscoveredFile],
    unchanged: &mut [FileResult],
    before_unchanged_roles: &mut [SourceRole],
    policy: DiffRolePolicy<'_>,
) {
    debug_assert_eq!(results.len(), changed_count);
    debug_assert!(
        results
            .iter()
            .enumerate()
            .all(|(offset, result)| result.index == offset),
        "diff results are dense and ordered by file index"
    );
    let demotions =
        {
            let (paths, references): (Vec<PathBuf>, Vec<&[DependencySyntax]>) =
                results
                    .iter()
                    .map(|result| match side {
                        DiffSideSelector::Current => (
                            result.change.current_path().to_path_buf(),
                            result.current.references(),
                        ),
                        DiffSideSelector::Before => (
                            result.change.base_path().to_path_buf(),
                            result.before.references(),
                        ),
                    })
                    .chain(unchanged_candidates.iter().zip(unchanged.iter()).map(
                        |(file, result)| (file.path().as_path().to_path_buf(), result.references()),
                    ))
                    .unzip();
            test_declared_demotions(&paths, &references, policy.aliases, policy.rules)
        };
    for file in demotions {
        if file < changed_count {
            match side {
                DiffSideSelector::Current => results[file].current.demote_to_test(),
                DiffSideSelector::Before => results[file].before.demote_to_test(),
            }
        } else {
            let unchanged_index = file - changed_count;
            match side {
                DiffSideSelector::Current => unchanged[unchanged_index].demote_to_test(),
                DiffSideSelector::Before => {
                    before_unchanged_roles[unchanged_index] =
                        before_unchanged_roles[unchanged_index].demoted_by_test_scope();
                }
            }
        }
    }
}
