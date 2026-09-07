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
