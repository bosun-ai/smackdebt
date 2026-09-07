//! The repository scope hierarchy every report is stated over.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use smackdebt_analysis::{Scope, ScopeId, ScopeKind};

pub(crate) struct HierarchyBuilder {
    pub(crate) scopes: Vec<Scope>,
    pub(crate) package_scopes: Vec<ScopeId>,
    pub(crate) file_scopes: BTreeMap<PathBuf, ScopeId>,
    pub(crate) directories: BTreeMap<(PathBuf, PathBuf), ScopeId>,
    pub(crate) package_roots: Vec<PathBuf>,
}
impl HierarchyBuilder {
    pub(crate) fn new(label: String, package_roots: &[PathBuf]) -> Self {
        let root = ScopeId::from_index(0);
        let mut scopes = vec![Scope::new(root, ScopeKind::Repository, label, None)];
        let mut package_scopes = Vec::with_capacity(package_roots.len());
        for package_root in package_roots {
            let id = ScopeId::from_index(scopes.len());
            package_scopes.push(id);
            scopes[root.index()].add_child(id);
            scopes.push(Scope::new(
                id,
                ScopeKind::Package,
                if package_root.as_os_str().is_empty() {
                    ".".to_owned()
                } else {
                    package_root.display().to_string()
                },
                Some(root),
            ));
        }
        Self {
            scopes,
            package_scopes,
            file_scopes: BTreeMap::new(),
            directories: BTreeMap::new(),
            package_roots: package_roots.to_vec(),
        }
    }

    pub(crate) fn add_file(&mut self, path: &Path, package_index: usize) {
        let package_root = self.package_roots[package_index].clone();
        let package_scope = self.package_scopes[package_index];
        let relative_directory = path
            .parent()
            .unwrap_or(Path::new(""))
            .strip_prefix(&package_root)
            .unwrap_or(Path::new(""));
        let mut parent = package_scope;
        let mut accumulated = package_root.clone();
        for component in relative_directory.components() {
            accumulated.push(component);
            let key = (package_root.clone(), accumulated.clone());
            parent = if let Some(id) = self.directories.get(&key) {
                *id
            } else {
                let id = ScopeId::from_index(self.scopes.len());
                self.scopes[parent.index()].add_child(id);
                self.scopes.push(Scope::new(
                    id,
                    ScopeKind::Directory,
                    accumulated.display().to_string(),
                    Some(parent),
                ));
                self.directories.insert(key, id);
                id
            };
        }
        let file_scope = ScopeId::from_index(self.scopes.len());
        self.scopes[parent.index()].add_child(file_scope);
        self.scopes.push(Scope::new(
            file_scope,
            ScopeKind::File,
            path.display().to_string(),
            Some(parent),
        ));
        self.file_scopes.insert(path.to_path_buf(), file_scope);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebase::analyze_codebase;
    use crate::requests::CodebaseRequest;
    use crate::test_support::repository;
    use smackdebt_analysis::Report;
    use std::fs;

    #[test]
    fn package_rows_keep_empty_packages_and_discovery_ids() {
        let root = tempfile::tempdir().unwrap();
        for package in ["a", "b", "c"] {
            fs::create_dir_all(root.path().join(package)).unwrap();
            fs::write(
                root.path().join(package).join("package.json"),
                format!("{{\"name\":\"{package}\",\"private\":true}}\n"),
            )
            .unwrap();
        }
        fs::write(
            root.path().join("a/main.js"),
            "export function a() { return 1; }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("c/main.js"),
            "export function c() { return 1; }\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        assert_eq!(result.report().package_graph().len(), 3);
        let packages: Vec<_> = result
            .report()
            .packages()
            .iter()
            .map(|package| {
                let scope = &result.report().scopes()[package.scope().index()];
                assert_eq!(scope.kind(), ScopeKind::Package);
                assert_eq!(scope.name(), package.path());
                (
                    package.id().index(),
                    package.path(),
                    package.presence(),
                    package.scope(),
                )
            })
            .collect();
        assert_eq!(
            packages,
            [
                (
                    0,
                    "a",
                    smackdebt_analysis::PackagePresence::Current,
                    ScopeId::from_index(1)
                ),
                (
                    1,
                    "b",
                    smackdebt_analysis::PackagePresence::Current,
                    ScopeId::from_index(2)
                ),
                (
                    2,
                    "c",
                    smackdebt_analysis::PackagePresence::Current,
                    ScopeId::from_index(3)
                ),
            ]
        );
        let package_ids: Vec<_> = result
            .report()
            .files()
            .iter()
            .map(|file| file.package().unwrap().index())
            .collect();
        assert_eq!(package_ids, [0, 2]);
    }
    /// A path view is a scope of the repository report, so it carries the
    /// repository's package table and points at one row of it.
    #[test]
    fn path_view_keeps_the_repository_package_table() {
        let root = repository();
        let repo = root.path().join("repo");
        for package in ["a", "b"] {
            fs::create_dir_all(repo.join(package)).unwrap();
            fs::write(repo.join(package).join("package.json"), "{}").unwrap();
            fs::write(
                repo.join(package).join("main.js"),
                "export function work() { return 1; }\n",
            )
            .unwrap();
        }

        let codebase = analyze_codebase(&CodebaseRequest::new(&repo)).unwrap();
        let path = analyze_codebase(&CodebaseRequest::new(repo.join("b"))).unwrap();
        let package_paths = |report: &Report| {
            report
                .packages()
                .iter()
                .map(|package| (package.id(), package.path().to_owned()))
                .collect::<Vec<_>>()
        };
        assert_eq!(package_paths(codebase.report()).len(), 2);
        assert_eq!(
            package_paths(path.report()),
            package_paths(codebase.report())
        );
        let selected = path.selected_scope().unwrap();
        assert_eq!(path.report().scopes()[selected.index()].name(), "b");
    }
}
