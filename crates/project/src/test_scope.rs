//! Test-declared demotion: a file the build compiles only under `test`
//! carries a test role on the side that declares it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use smackdebt_analysis::{
    DependencySyntax, DependencySyntaxState, FileId, ModuleDeclaration, test_declared_files,
};

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
            results,
            unchanged,
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
    for source in sources {
        record_declarer(&mut declarations, source, index, aliases);
    }
    declarations
}

/// Records the module files one declarer's ownership references resolve to.
fn record_declarer(
    declarations: &mut BTreeSet<ModuleDeclaration>,
    (declarer, path, references): (usize, &Path, &[DependencySyntax]),
    index: &BTreeMap<PathBuf, FileId>,
    aliases: &ResolutionRules,
) {
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
    results: &mut [DiffResult],
    unchanged: &mut DiffUnchanged<'_>,
    policy: DiffRolePolicy<'_>,
) {
    let changed_count = unchanged.first_file_index;
    debug_assert_eq!(results.len(), changed_count);
    debug_assert!(
        results
            .iter()
            .enumerate()
            .all(|(offset, result)| result.index == offset),
        "diff results are dense and ordered by file index"
    );
    let demotions = {
        let (paths, references): (Vec<PathBuf>, Vec<&[DependencySyntax]>) = results
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
            .chain(
                unchanged
                    .candidates
                    .iter()
                    .zip(unchanged.results.iter())
                    .map(|(file, result)| {
                        (file.path().as_path().to_path_buf(), result.references())
                    }),
            )
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
                DiffSideSelector::Current => unchanged.results[unchanged_index].demote_to_test(),
                DiffSideSelector::Before => {
                    unchanged.before_roles[unchanged_index] =
                        unchanged.before_roles[unchanged_index].demoted_by_test_scope();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebase::analyze_codebase;
    use crate::diff::analyze_diff;
    use crate::requests::{CodebaseRequest, DiffRequest};
    use crate::test_support::{file_id, git, package_pairs, write_crate};
    use smackdebt_analysis::ArchitectureFindingKind;
    use smackdebt_analysis::DependencyEdge;
    use smackdebt_analysis::SourceRole;
    use std::fs;

    #[test]
    fn a_rust_test_scope_demotes_a_primary_reference_while_the_file_keeps_its_own_role() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname='scoped'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            root.path().join("helper.rs"),
            "pub fn work() -> u32 { 1 }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("shipped.rs"),
            "use crate::helper::work;\n#[cfg(test)]\nmod tests {\n    use crate::helper::work;\n    #[test]\n    fn covers() { assert_eq!(work(), 1); }\n}\npub fn ship() -> u32 { work() }\n",
        )
        .unwrap();
        fs::create_dir(root.path().join("fixtures")).unwrap();
        fs::write(
            root.path().join("fixtures/kept.rs"),
            "#[cfg(test)]\nmod tests {\n    use crate::helper::work;\n}\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let helper = file_id(report, "helper.rs");
        let shipped = file_id(report, "shipped.rs");
        let fixture = file_id(report, "fixtures/kept.rs");

        let mut shipped_roles: Vec<_> = report
            .dependency_edges()
            .iter()
            .filter(|edge| edge.source() == shipped && edge.target() == helper)
            .map(|edge| (edge.role(), edge.relation(), edge.references()))
            .collect();
        shipped_roles.sort();
        assert_eq!(
            shipped_roles,
            [
                (
                    SourceRole::Primary,
                    smackdebt_analysis::StaticRelationKind::Uses,
                    1
                ),
                (
                    SourceRole::Test,
                    smackdebt_analysis::StaticRelationKind::Uses,
                    1
                ),
            ]
        );
        assert!(
            report
                .dependency_edges()
                .iter()
                .filter(|edge| edge.source() == shipped && edge.target() == helper)
                .all(DependencyEdge::affects_verdict)
        );
        assert_eq!(
            report
                .dependency_edges()
                .iter()
                .filter(|edge| edge.source() == fixture && edge.target() == helper)
                .map(DependencyEdge::role)
                .collect::<Vec<_>>(),
            [SourceRole::Fixture]
        );
        assert_eq!(report.dependency_coverage().resolved_internal_uses(), 2);
        assert_eq!(report.dependency_coverage().context_relations(), 1);
    }
    #[test]
    fn a_module_declared_only_under_a_test_configuration_is_test_source() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("crates/main/src/worker")).unwrap();
        fs::write(
            root.path().join("crates/main/Cargo.toml"),
            "[package]\nname='main'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            root.path().join("crates/main/src/lib.rs"),
            "mod worker;\npub fn run() -> u32 { worker::work() }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("crates/main/src/worker.rs"),
            "#[cfg(test)]\nmod tests;\npub fn work() -> u32 { 1 }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("crates/main/src/worker/tests.rs"),
            "use support::probe;\n#[test]\nfn covers() { assert_eq!(probe(), 1); }\n",
        )
        .unwrap();
        write_crate(
            root.path(),
            "support",
            "use main::run;\npub fn probe() -> u32 { run() }\n",
        );

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let declared = file_id(report, "crates/main/src/worker/tests.rs");
        assert_eq!(
            report.files()[declared.index()].role(),
            SourceRole::Test,
            "rustc compiles a cfg(test) module file only under test"
        );
        assert!(
            report
                .dependency_edges()
                .iter()
                .filter(|edge| edge.source() == declared)
                .all(|edge| edge.role() == SourceRole::Test && !edge.enters_verdict_graph())
        );
        assert_eq!(
            package_pairs(report),
            [("crates/support".to_owned(), "crates/main".to_owned())]
        );
        assert!(
            report
                .architecture_findings()
                .iter()
                .all(|finding| finding.kind() != ArchitectureFindingKind::PackageCycle)
        );
    }
    #[test]
    fn a_test_declaration_demotes_only_a_file_no_other_rule_and_no_other_declaration_claims() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("src/harness")).unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname='declared'\nversion='0.1.0'\n",
        )
        .unwrap();
        // `shared` is declared twice, once outside the test scope.
        fs::write(
            root.path().join("src/lib.rs"),
            "#[cfg(test)]\nmod shared;\n#[cfg(test)]\nmod harness;\n#[cfg(test)]\nmod kept;\n",
        )
        .unwrap();
        fs::write(
            root.path().join("src/main.rs"),
            "mod shared;\nfn main() {}\n",
        )
        .unwrap();
        fs::write(root.path().join("src/shared.rs"), "pub fn shared() {}\n").unwrap();
        // `harness` inherits the test scope and passes it to its own module.
        fs::write(
            root.path().join("src/harness.rs"),
            "mod helpers;\npub fn harness() {}\n",
        )
        .unwrap();
        fs::write(
            root.path().join("src/harness/helpers.rs"),
            "pub fn helper() {}\n",
        )
        .unwrap();
        // `kept` is claimed by configuration, which keeps precedence.
        fs::write(root.path().join("src/kept.rs"), "pub fn kept() {}\n").unwrap();

        let result = analyze_codebase(
            &CodebaseRequest::new(root.path())
                .with_role_rules(vec![SourceRoleRule::primary("src/kept.rs")]),
        )
        .unwrap();
        let report = result.report();
        let role = |path: &str| report.files()[file_id(report, path).index()].role();
        assert_eq!(role("src/shared.rs"), SourceRole::Primary);
        assert_eq!(role("src/harness.rs"), SourceRole::Test);
        assert_eq!(role("src/harness/helpers.rs"), SourceRole::Test);
        assert_eq!(role("src/kept.rs"), SourceRole::Primary);
        assert_eq!(role("src/lib.rs"), SourceRole::Primary);
    }
    #[test]
    fn unchanged_rust_module_uses_each_graph_sides_test_declaration_role() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        fs::create_dir_all(repository_path.join("src")).unwrap();
        fs::write(
            repository_path.join("Cargo.toml"),
            "[package]\nname='roles'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("src/lib.rs"),
            "#[cfg(test)]\nmod helper;\npub fn run() {}\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("src/helper.rs"),
            "mod missing;\npub fn help() {}\n",
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "test-only helper"]);
        fs::write(
            repository_path.join("src/lib.rs"),
            "mod helper;\npub fn run() { helper::help(); }\n",
        )
        .unwrap();

        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        let evidence = result.report().diff_graph_evidence().unwrap();
        assert!(evidence.base().is_complete());
        assert!(!evidence.current().is_complete());
        let helper = file_id(result.report(), "src/helper.rs");
        assert_eq!(
            result.report().files()[helper.index()].role(),
            SourceRole::Primary
        );
    }
}
