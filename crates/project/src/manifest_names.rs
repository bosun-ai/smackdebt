//! Declared manifest names: the index that resolves them and the entry files
//! they answer with.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use smackdebt_analysis::{FileId, Language};
use smackdebt_discovery::Inventory;

/// The name the current inventory declares for one package root.
///
/// A base-only package root the working tree no longer has declares nothing,
/// which is exactly what the report states for it.
pub(crate) fn declared_manifest_name(inventory: &Inventory, root: &Path) -> Option<String> {
    inventory
        .packages()
        .iter()
        .find(|package| package.root().as_path() == root)
        .and_then(|package| package.manifest_name().map(str::to_owned))
}
/// Aligns discovered manifest names with the report's package positions.
///
/// A package root the working tree no longer has keeps no declared name: the
/// diff sides read the names the current inventory declares.
pub(crate) fn manifest_names_for(
    inventory: &Inventory,
    package_roots: &[PathBuf],
) -> Vec<Option<String>> {
    let declared: BTreeMap<_, _> = inventory
        .packages()
        .iter()
        .map(|package| {
            (
                package.root().as_path().to_path_buf(),
                package.manifest_name().map(str::to_owned),
            )
        })
        .collect();
    package_roots
        .iter()
        .map(|root| declared.get(root).cloned().flatten())
        .collect()
}
/// The paths each package's manifest names, aligned with `package_roots`.
pub(crate) fn manifest_paths_for(
    inventory: &Inventory,
    package_roots: &[PathBuf],
) -> Vec<Vec<String>> {
    let declared: BTreeMap<_, _> = inventory
        .packages()
        .iter()
        .map(|package| {
            (
                package.root().as_path().to_path_buf(),
                package.declared_paths().to_vec(),
            )
        })
        .collect();
    package_roots
        .iter()
        .map(|root| declared.get(root).cloned().unwrap_or_default())
        .collect()
}
/// What a declared manifest name means inside this repository.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ManifestNameMatch {
    /// No internal package declares the name.
    Absent,
    /// Several internal packages declare the name.
    Ambiguous,
    /// Exactly one internal package declares the name.
    Package(usize),
}
/// A read-only index from declared package names to internal packages.
///
/// Positions are package-root positions, so a match names the same package the
/// rest of the build already knows.  The index is consulted only for references
/// that path candidates would otherwise classify external.
#[derive(Default)]
pub(crate) struct ManifestNameIndex {
    exact: BTreeMap<String, ManifestNameMatch>,
    rust: BTreeMap<String, ManifestNameMatch>,
}
impl ManifestNameIndex {
    pub(crate) fn new(names: &[Option<String>]) -> Self {
        let mut index = Self::default();
        for (position, name) in names.iter().enumerate() {
            let Some(name) = name else {
                continue;
            };
            index.insert_key(name.clone(), position, false);
            index.insert_key(rust_manifest_key(name), position, true);
        }
        index
    }

    pub(crate) fn insert_key(&mut self, key: String, position: usize, rust: bool) {
        let keys = if rust {
            &mut self.rust
        } else {
            &mut self.exact
        };
        keys.entry(key)
            .and_modify(|value| {
                if *value != ManifestNameMatch::Package(position) {
                    *value = ManifestNameMatch::Ambiguous;
                }
            })
            .or_insert(ManifestNameMatch::Package(position));
    }

    pub(crate) fn resolve(&self, target: &str, language: Language) -> ManifestNameMatch {
        let Some(root) = reference_root(target, language) else {
            return ManifestNameMatch::Absent;
        };
        if language == Language::Rust {
            return self
                .rust
                .get(&rust_manifest_key(root))
                .copied()
                .unwrap_or(ManifestNameMatch::Absent);
        }
        self.exact
            .get(root)
            .copied()
            .unwrap_or(ManifestNameMatch::Absent)
    }
}
/// Normalizes the Rust equivalence of hyphens and underscores in a name.
pub(crate) fn rust_manifest_key(name: &str) -> String {
    name.replace('-', "_")
}
/// Returns the package-naming first segment of an unresolved reference.
pub(crate) fn reference_root(target: &str, language: Language) -> Option<&str> {
    let root = match language {
        Language::Rust => target.split("::").next(),
        Language::Python | Language::Java => target.split(['.', '/']).next(),
        _ if target.starts_with('@') => {
            let mut parts = target.splitn(3, '/');
            match (parts.next(), parts.next()) {
                (Some(scope), Some(name)) => Some(&target[..scope.len() + name.len() + 1]),
                _ => Some(target),
            }
        }
        _ => target.split('/').next(),
    }?;
    (!root.is_empty()).then_some(root)
}
/// Returns the file a package presents as its entry point, when it has one.
pub(crate) fn package_entry_file(
    root: &Path,
    name: &str,
    index: &BTreeMap<PathBuf, FileId>,
) -> Option<FileId> {
    let module = name.rsplit('/').next().unwrap_or(name).replace('-', "_");
    [
        "src/lib.rs".to_owned(),
        "src/main.rs".to_owned(),
        "index.js".to_owned(),
        "index.mjs".to_owned(),
        "index.ts".to_owned(),
        "src/index.js".to_owned(),
        "src/index.mjs".to_owned(),
        "src/index.ts".to_owned(),
        "lib/index.js".to_owned(),
        "__init__.py".to_owned(),
        format!("{module}/__init__.py"),
        format!("src/{module}/__init__.py"),
        format!("lib/{module}.rb"),
    ]
    .into_iter()
    .find_map(|candidate| index.get(&root.join(candidate)).copied())
}

#[cfg(test)]
mod tests {

    use crate::codebase::analyze_codebase;
    use crate::requests::CodebaseRequest;
    use crate::test_support::{package_pairs, workspace_with_declared_names};
    use smackdebt_analysis::ResolutionIssueKind;
    use std::fs;

    #[test]
    fn declared_manifest_names_resolve_cross_package_references() {
        let root = workspace_with_declared_names();
        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        assert_eq!(
            package_pairs(report),
            [
                ("crates/app".to_owned(), "crates/core".to_owned()),
                ("web".to_owned(), "ui".to_owned()),
            ]
        );
        assert!(
            report
                .external_dependencies()
                .iter()
                .all(|external| external.target() != "acme_core"
                    && external.target() != "@acme/ui/button"),
            "{:?}",
            report.external_dependencies()
        );
        let edges: Vec<_> = report
            .dependency_edges()
            .iter()
            .map(|edge| {
                (
                    report.files()[edge.source().index()].path(),
                    report.files()[edge.target().index()].path(),
                )
            })
            .collect();
        assert!(
            edges.contains(&("crates/app/src/lib.rs", "crates/core/src/lib.rs")),
            "{edges:?}"
        );
        assert!(
            edges.contains(&("web/index.js", "ui/index.js")),
            "{edges:?}"
        );
    }
    #[test]
    fn a_shadowed_manifest_name_stays_ambiguous_with_its_diagnostic() {
        let root = workspace_with_declared_names();
        fs::create_dir_all(root.path().join("mirror/core/src")).unwrap();
        fs::write(
            root.path().join("mirror/core/Cargo.toml"),
            "[package]\nname='acme-core'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            root.path().join("mirror/core/src/lib.rs"),
            "pub fn core(value: i32) -> i32 { value }\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        assert!(
            !package_pairs(report).contains(&("crates/app".to_owned(), "crates/core".to_owned())),
            "{:?}",
            package_pairs(report)
        );
        let ambiguous: Vec<_> = report
            .resolution_diagnostics()
            .iter()
            .filter(|value| value.kind() == ResolutionIssueKind::Ambiguous)
            .map(|value| (value.target(), value.span().start_line()))
            .collect();
        assert_eq!(ambiguous, [("acme_core::core", 1)]);
    }
    #[test]
    fn a_package_without_an_entry_file_keeps_a_package_scoped_edge() {
        let root = tempfile::tempdir().unwrap();
        for (path, source) in [
            (
                "crates/tool/Cargo.toml",
                "[package]\nname='acme-tool'\nversion='0.1.0'\n",
            ),
            (
                "crates/tool/other/thing.rs",
                "pub fn thing(value: i32) -> i32 { value }\n",
            ),
            (
                "crates/app/Cargo.toml",
                "[package]\nname='acme-app'\nversion='0.1.0'\n",
            ),
            (
                "crates/app/src/lib.rs",
                "use acme_tool::thing;\nuse acme_tool::other::more;\npub fn app(value: i32) -> i32 { thing(more(value)) }\n",
            ),
            (
                "crates/app/src/second.rs",
                "use acme_tool::thing;\npub fn second(value: i32) -> i32 { thing(value) }\n",
            ),
        ] {
            let file = root.path().join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, source).unwrap();
        }

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        assert_eq!(
            package_pairs(report),
            [("crates/app".to_owned(), "crates/tool".to_owned())]
        );
        let edge = &report.package_edges()[0];
        assert_eq!(
            (edge.file_pairs(), edge.references(), edge.file_edges()),
            (2, 3, [].as_slice()),
            "two files make three package-scoped references"
        );
        assert!(
            report
                .dependency_edges()
                .iter()
                .all(|edge| !report.files()[edge.target().index()]
                    .path()
                    .starts_with("crates/tool")),
            "a package without an entry file has no file-level target"
        );
        assert_eq!(report.dependency_coverage().resolved_internal_uses(), 3);
        assert!(
            report
                .external_dependencies()
                .iter()
                .all(|external| !external.target().starts_with("acme_tool")),
            "{:?}",
            report.external_dependencies()
        );
    }
}
