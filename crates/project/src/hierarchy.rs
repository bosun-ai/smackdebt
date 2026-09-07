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
