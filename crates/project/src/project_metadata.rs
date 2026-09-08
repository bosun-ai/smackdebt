//! Static Go project declarations shared by codebase and diff resolution.
use crate::go_project::{GoProject, package_directory};
use smackdebt_analysis::Language;
use std::path::{Path, PathBuf};

#[derive(Clone, Default)]
pub(crate) struct ProjectMetadata {
    go: Vec<GoProject>,
    pub(crate) issues: Vec<ProjectIssue>,
}

#[derive(Clone)]
pub(crate) struct ProjectIssue {
    pub(crate) root: PathBuf,
    pub(crate) reason: String,
}

impl ProjectMetadata {
    pub(crate) fn load(
        paths: &[PathBuf],
        mut read: impl FnMut(&Path) -> Result<Vec<u8>, String>,
    ) -> Self {
        let mut result = Self::default();
        for path in paths.iter().filter(|path| {
            matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some("go.mod" | "go.work")
            )
        }) {
            let parsed = read(path).and_then(|bytes| {
                let source = std::str::from_utf8(&bytes)
                    .map_err(|_| "Go configuration is not UTF-8".to_owned())?;
                result.go.push(GoProject::parse(path, source)?);
                Ok(())
            });
            if let Err(error) = parsed {
                result.issues.push(ProjectIssue {
                    root: path.parent().unwrap_or(Path::new("")).to_path_buf(),
                    reason: format!("{}: {error}", path.display()),
                });
            }
        }
        result
    }

    pub(crate) fn issue_for(&self, language: Language, source: &Path) -> Option<&str> {
        if language != Language::Go {
            return None;
        }
        self.issues
            .iter()
            .find(|issue| source.starts_with(&issue.root))
            .map(|issue| issue.reason.as_str())
    }

    pub(crate) fn package_directory(
        &self,
        source: &Path,
        target: &str,
    ) -> Result<Option<PathBuf>, String> {
        if target
            .split('/')
            .any(|part| matches!(part, "" | "." | ".."))
            || target.contains('\\')
        {
            return Err("Go import is not a package path".to_owned());
        }
        let Some(module) = self
            .go
            .iter()
            .filter(|module| !module.workspace && source.starts_with(&module.root))
            .max_by_key(|module| module.root.components().count())
        else {
            return Err("Go import has no module declaration".to_owned());
        };
        if let Some(directory) = module.directory(target) {
            return Ok(Some(directory));
        }
        let workspace = self
            .go
            .iter()
            .filter(|workspace| {
                workspace.workspace
                    && source.starts_with(&workspace.root)
                    && workspace.uses.contains(&module.root)
            })
            .max_by_key(|workspace| workspace.root.components().count());
        let replacement = workspace
            .into_iter()
            .flat_map(|workspace| workspace.replacements.iter())
            .chain(module.replacements.iter().filter(|(name, _)| {
                !workspace.is_some_and(|workspace| workspace.replacements.contains_key(*name))
            }))
            .filter_map(|(prefix, root)| {
                package_directory(prefix, root, target).map(|directory| (prefix.len(), directory))
            })
            .max_by_key(|(length, _)| *length);
        if let Some((_, directory)) = replacement {
            return Ok(Some(directory));
        }
        if let Some(workspace) = workspace {
            for root in &workspace.uses {
                if let Some(other) = self
                    .go
                    .iter()
                    .find(|module| !module.workspace && &module.root == root)
                    && let Some(directory) = other.directory(target)
                {
                    return Ok(Some(directory));
                }
            }
        }
        Ok(None)
    }

    pub(crate) fn visible(
        &self,
        _language: Language,
        _source: &Path,
        _target: &Path,
        _name: &str,
    ) -> bool {
        true
    }

    pub(crate) fn claims_name(&self, _language: Language, _source: &Path, _name: &str) -> bool {
        false
    }

    pub(crate) fn imports(&self, _source: &Path) -> impl Iterator<Item = &str> {
        std::iter::empty()
    }

    pub(crate) fn is_test_source(&self, _source: &Path) -> bool {
        false
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = &Path> {
        std::iter::empty()
    }

    pub(crate) fn is_external_bootstrap(
        &self,
        _language: Language,
        _source: &Path,
        _candidate: &str,
    ) -> bool {
        false
    }
}
