//! Which scope a codebase path names, and the repository behind it.

use std::path::{Path, PathBuf};

use smackdebt_discovery::is_source_path;
use smackdebt_git::GitRepository;

use crate::requests::ProjectError;

pub(crate) struct Selection {
    /// The root every recorded path is relative to.
    pub(crate) inventory_root: PathBuf,
    /// The tree walked when no repository stands behind the selection.
    pub(crate) discovery_root: PathBuf,
    pub(crate) exact_file: Option<PathBuf>,
    pub(crate) prefix: Option<PathBuf>,
    pub(crate) label: String,
    /// Whether a repository stands behind the selection, which is what makes
    /// the selection a scope of one repository report rather than a report of
    /// its own.
    pub(crate) repository: bool,
}
impl Selection {
    pub(crate) fn resolve(path: &Path, automatic_scope: bool) -> Result<Self, ProjectError> {
        let absolute = std::path::absolute(path).map_err(|source| ProjectError::Inspect {
            path: path.to_path_buf(),
            source,
        })?;
        if !automatic_scope && absolute.is_file() && !is_source_path(&absolute) {
            return Err(ProjectError::NotSourceFile(path.to_path_buf()));
        }
        if let Ok(repository) = GitRepository::discover(&absolute) {
            let root = repository
                .root()
                .canonicalize()
                .unwrap_or_else(|_| repository.root().to_path_buf());
            let selected_absolute = absolute.canonicalize().unwrap_or_else(|_| absolute.clone());
            let prefix = if automatic_scope {
                None
            } else {
                Some(
                    selected_absolute
                        .strip_prefix(&root)
                        .unwrap_or(Path::new(""))
                        .to_path_buf(),
                )
            };
            let exact_file = selected_absolute.is_file().then(|| {
                selected_absolute
                    .strip_prefix(&root)
                    .unwrap_or(Path::new(""))
                    .to_path_buf()
            });
            // The report is the repository however little of it is answered,
            // so its root scope carries the repository's own name.
            return Ok(Self {
                inventory_root: root,
                discovery_root: selected_absolute,
                exact_file,
                prefix,
                label: ".".to_owned(),
                repository: true,
            });
        }
        if absolute.is_file() {
            let root = absolute.parent().unwrap_or(Path::new(".")).to_path_buf();
            let exact_file = absolute.file_name().map(PathBuf::from);
            return Ok(Self {
                inventory_root: root,
                discovery_root: absolute,
                exact_file,
                prefix: None,
                label: path.display().to_string(),
                repository: false,
            });
        }
        Ok(Self {
            inventory_root: absolute.clone(),
            discovery_root: absolute,
            exact_file: None,
            prefix: None,
            label: path.display().to_string(),
            repository: false,
        })
    }

    /// The tree the inventory walk covers, which the reader is shown if the
    /// walk fails.
    pub(crate) fn walk_root(&self) -> &Path {
        if self.repository {
            &self.inventory_root
        } else {
            &self.discovery_root
        }
    }

    pub(crate) fn includes(&self, path: &Path) -> bool {
        self.exact_file.as_deref().map_or_else(
            || {
                self.prefix
                    .as_deref()
                    .is_none_or(|prefix| path.starts_with(prefix))
            },
            |file| path == file,
        )
    }
}

#[cfg(test)]
mod tests {

    use crate::codebase::analyze_codebase;
    use crate::requests::CodebaseRequest;
    use crate::test_support::{git, repository};
    use std::fs;

    /// A package cannot say who imports it from its own files, so selecting
    /// one reads the repository that answers the question and shows the
    /// package.
    #[test]
    fn package_selection_reads_the_sibling_source_that_imports_it() {
        let root = tempfile::tempdir().unwrap();
        git(root.path(), ["init", "-q"]);
        for package in ["app", "core"] {
            fs::create_dir_all(root.path().join(package)).unwrap();
            fs::write(root.path().join(package).join("package.json"), "{}").unwrap();
        }
        fs::write(
            root.path().join("app/a.js"),
            "import core from '../core/b';\nfunction app() {}\n",
        )
        .unwrap();
        fs::write(root.path().join("core/b.js"), "function core() {}\n").unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path().join("core"))).unwrap();
        assert_eq!(result.report().files().len(), 2);
        assert_eq!(result.report().dependency_edges().len(), 1);
        assert_eq!(result.stats().inventory_walks, 1);
        assert_eq!(result.stats().source_reads, 2);
        let selected = result.selected_scope().unwrap();
        assert_eq!(result.report().scopes()[selected.index()].name(), "core");
        assert_eq!(result.report().scopes()[0].name(), ".");
    }
    #[test]
    fn automatic_scope_uses_the_git_root_from_a_nested_directory() {
        let root = repository();
        let repository_path = root.path().join("repo");
        let nested = repository_path.join("nested/deeper");
        fs::create_dir_all(&nested).unwrap();
        let result = analyze_codebase(&CodebaseRequest::automatic(&nested)).unwrap();
        assert!(
            result
                .report()
                .files()
                .iter()
                .any(|file| file.path() == "sample.rs")
        );
        assert_eq!(result.report().scopes()[0].name(), ".");
    }
    /// A file selection answers one file of the repository report, so the
    /// walk is the repository and the answered scope is that file.
    #[test]
    fn explicit_file_selection_answers_one_scope_of_the_repository() {
        let root = repository();
        let repository_path = root.path().join("repo");
        fs::write(
            repository_path.join("outside.rs"),
            "fn outside() { if true {} }\n",
        )
        .unwrap();

        let result =
            analyze_codebase(&CodebaseRequest::new(repository_path.join("sample.rs"))).unwrap();

        assert_eq!(result.report().files().len(), 2);
        assert_eq!(result.stats().inventory_walks, 1);
        assert_eq!(result.stats().source_reads, 2);
        let selected = result.selected_scope().unwrap();
        assert_eq!(
            result.report().scopes()[selected.index()].name(),
            "sample.rs"
        );
        assert_eq!(result.report().scopes()[0].name(), ".");
    }
    #[test]
    fn codebase_path_selection_keeps_repository_relative_scope_identity() {
        let root = repository();
        let repository_path = root.path().join("repo");
        fs::create_dir_all(repository_path.join("src")).unwrap();
        fs::write(
            repository_path.join("src/lib.rs"),
            "fn selected() { if true {} }\n",
        )
        .unwrap();
        let result = analyze_codebase(&CodebaseRequest::new(repository_path.join("src"))).unwrap();
        assert_ne!(result.selected_scope(), result.report().root());
        assert!(
            result
                .report()
                .files()
                .iter()
                .any(|file| file.path() == "src/lib.rs")
        );
        assert!(
            result
                .report()
                .scopes()
                .iter()
                .any(|scope| scope.name() == "src")
        );
    }
    #[test]
    fn source_outside_manifest_roots_uses_discoverys_fallback_package() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("app/src")).unwrap();
        fs::write(
            root.path().join("app/Cargo.toml"),
            "[package]\nname='app'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(root.path().join("app/src/lib.rs"), "fn app() {}\n").unwrap();
        fs::write(root.path().join("outside.rs"), "fn outside() {}\n").unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();

        assert_eq!(result.report().files().len(), 2);
        assert!(
            result
                .report()
                .files()
                .iter()
                .any(|file| file.path() == "outside.rs")
        );
    }
}
