//! Static project declarations shared by codebase and diff resolution.
use crate::csharp_project::CSharpProject;
use crate::go_project::{GoProject, package_directory};
use crate::php_project::PhpProject;
use smackdebt_analysis::Language;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Clone, Default)]
pub(crate) struct ProjectMetadata {
    pub(crate) go: Vec<GoProject>,
    pub(crate) php: Vec<PhpProject>,
    pub(crate) csharp: Vec<CSharpProject>,
    pub(crate) issues: Vec<ProjectIssue>,
}
#[derive(Clone)]
pub(crate) struct ProjectIssue {
    pub(crate) root: PathBuf,
    pub(crate) language: Language,
    pub(crate) reason: String,
}
impl ProjectMetadata {
    pub(crate) fn load(
        paths: &[PathBuf],
        mut read: impl FnMut(&Path) -> Result<Vec<u8>, String>,
    ) -> Self {
        let mut result = Self::default();
        for path in paths {
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !matches!(
                name,
                "go.mod"
                    | "go.work"
                    | "composer.json"
                    | "Directory.Build.props"
                    | "Directory.Build.targets"
            ) && !name.ends_with(".csproj")
            {
                continue;
            }
            let parsed = read(path).and_then(|bytes| {
                let source = std::str::from_utf8(&bytes)
                    .map_err(|_| "project configuration is not UTF-8".to_owned())?;
                match name {
                    "go.mod" | "go.work" => result.go.push(GoProject::parse(path, source)?),
                    "composer.json" => result.php.push(PhpProject::parse(path, source)?),
                    _ if name.ends_with(".csproj") => {
                        result.csharp.push(CSharpProject::parse(path, source)?)
                    }
                    _ => return Err("C# project uses shared build configuration".to_owned()),
                }
                Ok(())
            });
            if let Err(error) = parsed {
                result.issues.push(ProjectIssue {
                    root: path.parent().unwrap_or(Path::new("")).to_path_buf(),
                    language: match name {
                        "go.mod" | "go.work" => Language::Go,
                        "composer.json" => Language::Php,
                        _ => Language::CSharp,
                    },
                    reason: format!("{}: {error}", path.display()),
                });
            }
        }
        result
    }
    pub(crate) fn issue_for(&self, language: Language, source: &Path) -> Option<&str> {
        self.issues
            .iter()
            .find(|issue| issue.language == language && source.starts_with(&issue.root))
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
        language: Language,
        source: &Path,
        target: &Path,
        name: &str,
    ) -> bool {
        match language {
            Language::Php => {
                let Some(owner) = self
                    .php
                    .iter()
                    .filter(|package| source.starts_with(&package.root))
                    .max_by_key(|package| package.root.components().count())
                else {
                    return true;
                };
                self.php
                    .iter()
                    .filter(|package| {
                        package.root == owner.root
                            || package
                                .name
                                .as_ref()
                                .is_some_and(|name| owner.requires.contains(name))
                    })
                    .any(|package| package.permits(name, target))
            }
            Language::CSharp => self.csharp_visible(source, target),
            _ => true,
        }
    }
    fn csharp_visible(&self, source: &Path, target: &Path) -> bool {
        let owners: Vec<_> = self
            .csharp
            .iter()
            .filter(|project| project.contains(source))
            .collect();
        if owners.is_empty() {
            return true;
        }
        let mut todo: Vec<_> = owners
            .into_iter()
            .map(|project| project.path.as_path())
            .collect();
        let mut seen = BTreeSet::new();
        while let Some(path) = todo.pop() {
            if !seen.insert(path) {
                continue;
            }
            let Some(project) = self.csharp.iter().find(|project| project.path == path) else {
                continue;
            };
            if project.contains(target) {
                return true;
            }
            todo.extend(project.references.iter().map(PathBuf::as_path));
        }
        false
    }
    pub(crate) fn is_external_bootstrap(
        &self,
        language: Language,
        source: &Path,
        candidate: &str,
    ) -> bool {
        language == Language::Php
            && self
                .php
                .iter()
                .any(|package| package.is_bootstrap(source, candidate))
    }
    pub(crate) fn claims_name(&self, language: Language, source: &Path, name: &str) -> bool {
        language == Language::Php
            && self
                .php
                .iter()
                .filter(|package| source.starts_with(&package.root))
                .any(|package| package.claims(name))
    }
    pub(crate) fn imports(&self, source: &Path) -> impl Iterator<Item = &str> {
        self.csharp
            .iter()
            .filter(move |project| project.contains(source))
            .flat_map(|project| project.imports.iter().map(String::as_str))
    }
    pub(crate) fn is_test_source(&self, source: &Path) -> bool {
        self.csharp
            .iter()
            .any(|project| project.test && project.contains(source))
    }
    pub(crate) fn entries(&self) -> impl Iterator<Item = &Path> {
        self.php
            .iter()
            .flat_map(|package| package.files.iter().map(PathBuf::as_path))
    }
}
