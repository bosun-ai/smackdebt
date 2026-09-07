//! Rust module layout: which files a symbolic path may resolve to.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use smackdebt_analysis::FileId;

/// Resolves candidates that name a file by role instead of by path.
///
/// Symbolic candidates are the last resort of one reference: they are consulted
/// only when no path candidate of the same reference matched a discovered file.
pub(crate) fn resolve_symbolic_candidates(
    source: &Path,
    candidates: &[String],
    index: &BTreeMap<PathBuf, FileId>,
) -> Vec<FileId> {
    let mut matches = Vec::new();
    for candidate in candidates {
        let file = match candidate.as_str() {
            smackdebt_analysis::DECLARING_FILE_CANDIDATE => index.get(source).copied(),
            smackdebt_analysis::CRATE_ROOT_CANDIDATE => rust_crate_root_file(source, index),
            smackdebt_analysis::PARENT_MODULE_CANDIDATE => rust_parent_module_file(source, index),
            _ => None,
        };
        matches.extend(file);
    }
    matches
}
/// The file a Rust package presents as the root of its module tree.
pub(crate) fn rust_crate_root_file(
    source: &Path,
    index: &BTreeMap<PathBuf, FileId>,
) -> Option<FileId> {
    let root = rust_source_root(source)?;
    index
        .get(&root.join("lib.rs"))
        .or_else(|| index.get(&root.join("main.rs")))
        .copied()
}
/// The file declaring the module that declares a Rust file.
///
/// The enclosing module is the directory holding the file's own module
/// directory, and Rust spells that module in three places: `a/mod.rs` inside
/// it, `a.rs` beside it, and the crate root when the module is the source root
/// itself.  The first spelling the repository holds is the answer.
pub(crate) fn rust_parent_module_file(
    source: &Path,
    index: &BTreeMap<PathBuf, FileId>,
) -> Option<FileId> {
    let directory = rust_module_directory(source)?;
    let parent = directory.parent()?;
    let name = parent.file_name()?.to_str()?;
    let root = rust_source_root(source).filter(|root| root.as_path() == parent);
    [
        parent.join("mod.rs"),
        parent.with_file_name(format!("{name}.rs")),
    ]
    .into_iter()
    .chain(
        root.into_iter()
            .flat_map(|root| [root.join("lib.rs"), root.join("main.rs")]),
    )
    .find_map(|path| index.get(&path).copied())
}
/// The directory a Rust file's own modules live in.
///
/// `mod.rs`, `lib.rs`, and `main.rs` are the module of their directory; every
/// other file is a module that owns a directory named after it.
pub(crate) fn rust_module_directory(source: &Path) -> Option<PathBuf> {
    let parent = source.parent()?;
    let stem = source.file_stem()?.to_str()?;
    Some(if matches!(stem, "mod" | "lib" | "main") {
        parent.to_path_buf()
    } else {
        parent.join(stem)
    })
}
pub(crate) fn rust_source_root(source: &Path) -> Option<PathBuf> {
    let mut root = PathBuf::new();
    for component in source.parent()?.components() {
        let std::path::Component::Normal(value) = component else {
            return None;
        };
        root.push(value);
        if value == "src" {
            return Some(root);
        }
    }
    None
}

#[cfg(test)]
mod tests {

    use crate::codebase::analyze_codebase;
    use crate::requests::CodebaseRequest;
    use smackdebt_analysis::ResolutionDiagnostic;
    use smackdebt_analysis::ResolutionIssueKind;
    use std::fs;

    #[test]
    fn a_super_rooted_item_resolves_to_the_file_declaring_the_parent_module() {
        let root = tempfile::tempdir().unwrap();
        for (path, source) in [
            ("Cargo.toml", "[package]\nname='rooted'\nversion='0.1.0'\n"),
            (
                "src/lib.rs",
                "mod builder;\nmod edge;\nmod widget;\npub struct Root;\n",
            ),
            // The parent module lives inside its own directory.
            (
                "src/builder/mod.rs",
                "mod manifests;\npub struct DockerMode;\n",
            ),
            // A chain of several `super` segments would name this file's own
            // parent, two levels below the module it counts from.
            (
                "src/builder/manifests.rs",
                "use super::DockerMode;\nuse super::super::*;\n",
            ),
            // The parent module lives beside its directory, 2018 style.
            (
                "src/widget.rs",
                "mod deep;\nmod parts;\npub struct Frame;\n",
            ),
            (
                "src/widget/parts.rs",
                "use super::Frame;\nuse self::helper;\npub fn helper() -> u32 { 1 }\n",
            ),
            // The declaring file is the module of its own directory, so its
            // parent is the directory above rather than beside it.
            ("src/widget/deep/mod.rs", "use super::Frame;\n"),
            // The parent of a source-root module is the crate root.
            ("src/edge.rs", "use super::Root;\n"),
            // Nothing declares this file, so nothing can be named.
            ("standalone/loose.rs", "use super::Nothing;\n"),
        ] {
            let file = root.path().join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, source).unwrap();
        }

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let mut edges: Vec<_> = report
            .dependency_edges()
            .iter()
            .filter(|edge| edge.relation() == smackdebt_analysis::StaticRelationKind::Uses)
            .map(|edge| {
                (
                    report.files()[edge.source().index()].path(),
                    report.files()[edge.target().index()].path(),
                )
            })
            .collect();
        edges.sort_unstable();
        // Every edge, so a fallback that names a module the reference did not
        // ask for fails here instead of hiding among the ones it did.
        assert_eq!(
            edges,
            [
                // A directory module declares its children.
                ("src/builder/manifests.rs", "src/builder/mod.rs"),
                // The crate root declares the modules of the source root.
                ("src/edge.rs", "src/lib.rs"),
                // A module file beside its directory declares the children of
                // that directory, whether they are files or directories.
                ("src/widget/deep/mod.rs", "src/widget.rs"),
                ("src/widget/parts.rs", "src/widget.rs"),
            ],
            "a reference resolves to the module that declares its root"
        );
        let unresolved: Vec<_> = report
            .resolution_diagnostics()
            .iter()
            .filter(|value| value.kind() == ResolutionIssueKind::Unresolved)
            .map(ResolutionDiagnostic::target)
            .collect();
        assert_eq!(
            unresolved,
            ["super::super::*", "super::Nothing"],
            "a module the walk cannot name exactly keeps its absence: {edges:?}"
        );
    }
    #[test]
    fn rust_root_use_prefers_the_nearest_matching_module() {
        for (name, leaf, parent, expected_internal, expected_ambiguous) in [
            ("leaf", true, false, 1, 0),
            ("parent", false, true, 1, 0),
            ("both", true, true, 1, 0),
        ] {
            let root = tempfile::tempdir().unwrap();
            fs::write(
                root.path().join("main.rs"),
                "use crate::core::work;\nfn main() { work(); }\n",
            )
            .unwrap();
            if leaf {
                fs::create_dir_all(root.path().join("core")).unwrap();
                fs::write(root.path().join("core/work.rs"), "pub fn work() {}\n").unwrap();
            }
            if parent {
                fs::write(root.path().join("core.rs"), "pub fn work() {}\n").unwrap();
            }

            let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
            assert_eq!(
                result.report().dependency_coverage().internal(),
                expected_internal,
                "{name}"
            );
            assert_eq!(
                result.report().dependency_coverage().ambiguous(),
                expected_ambiguous,
                "{name}"
            );
            assert_eq!(
                result.report().dependency_edges().len(),
                expected_internal as usize,
                "{name}"
            );
            assert_eq!(
                result.report().resolution_diagnostics().len(),
                expected_ambiguous as usize,
                "{name}"
            );
        }
    }
    #[test]
    fn rust_crate_qualified_use_resolves_from_the_crate_source_root() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("crates/app/src/core")).unwrap();
        fs::write(
            root.path().join("crates/app/Cargo.toml"),
            "[package]\nname='app'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            root.path().join("crates/app/src/lib.rs"),
            "use crate::core::work;\npub fn run() { work(); }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("crates/app/src/core.rs"),
            "pub fn work() {}\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        assert_eq!(result.report().dependency_edges().len(), 1);
        assert_eq!(
            result
                .report()
                .dependency_coverage()
                .resolved_internal_uses(),
            1
        );
        assert_eq!(result.report().dependency_coverage().total(), 1);
    }
    #[test]
    fn a_rust_module_file_owns_a_directory_named_after_it() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("src/outer")).unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname='modules'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(root.path().join("src/lib.rs"), "mod outer;\nmod sibling;\n").unwrap();
        fs::write(root.path().join("src/outer.rs"), "mod inner;\n").unwrap();
        fs::write(
            root.path().join("src/outer/inner.rs"),
            "use super::super::sibling::shared;\npub fn work() -> u32 { shared() }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("src/sibling.rs"),
            "pub fn shared() -> u32 { 1 }\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let mut edges: Vec<_> = report
            .dependency_edges()
            .iter()
            .map(|edge| {
                (
                    report.files()[edge.source().index()].path().to_owned(),
                    report.files()[edge.target().index()].path().to_owned(),
                    edge.relation(),
                )
            })
            .collect();
        edges.sort();
        let ownership = smackdebt_analysis::StaticRelationKind::ModuleOwnership;
        let uses = smackdebt_analysis::StaticRelationKind::Uses;
        assert_eq!(
            edges,
            [
                (
                    "src/lib.rs".to_owned(),
                    "src/outer.rs".to_owned(),
                    ownership
                ),
                (
                    "src/lib.rs".to_owned(),
                    "src/sibling.rs".to_owned(),
                    ownership
                ),
                // `mod inner;` in `outer.rs` names `outer/inner.rs`.
                (
                    "src/outer.rs".to_owned(),
                    "src/outer/inner.rs".to_owned(),
                    ownership
                ),
                // `super::super` from `outer/inner.rs` is the crate root's module.
                (
                    "src/outer/inner.rs".to_owned(),
                    "src/sibling.rs".to_owned(),
                    uses
                ),
            ]
        );
        assert_eq!(report.resolution_diagnostics().len(), 0);
    }
    #[test]
    fn a_module_declaration_reads_the_module_directory_before_a_sibling_file() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nname='precedence'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::create_dir_all(root.path().join("a")).unwrap();
        fs::write(root.path().join("a.rs"), "mod child;\npub fn a() {}\n").unwrap();
        fs::write(root.path().join("a/child.rs"), "pub fn owned() {}\n").unwrap();
        fs::write(root.path().join("child.rs"), "pub fn sibling() {}\n").unwrap();
        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let declarations: Vec<_> = report
            .dependency_edges()
            .iter()
            .filter(|edge| {
                edge.relation() == smackdebt_analysis::StaticRelationKind::ModuleOwnership
            })
            .map(|edge| {
                (
                    report.files()[edge.source().index()].path().to_owned(),
                    report.files()[edge.target().index()].path().to_owned(),
                )
            })
            .collect();
        assert_eq!(
            declarations,
            [("a.rs".to_owned(), "a/child.rs".to_owned())],
            "the module directory a file owns wins over a sibling of the same name"
        );
    }
}
