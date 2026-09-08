//! Parsed resolution rules shared by current and base analysis.
use std::path::{Path, PathBuf};
#[derive(Clone)]
pub(crate) struct ResolutionAlias {
    pub(crate) prefix: String,
    pub(crate) suffix: String,
    pub(crate) replacement: String,
}
#[derive(Clone, Default)]
pub(crate) struct ResolutionRules {
    pub(crate) metadata: crate::project_metadata::ProjectMetadata,
    pub(crate) packages: Vec<PackageResolution>,
}
#[derive(Clone)]
pub(crate) struct PackageResolution {
    pub(crate) root: PathBuf,
    pub(crate) aliases: Vec<ResolutionAlias>,
    pub(crate) issue: Option<String>,
}
impl ResolutionRules {
    pub(crate) fn aliases_for(&self, source: &Path) -> &[ResolutionAlias] {
        self.packages
            .iter()
            .filter(|package| source.starts_with(&package.root))
            .max_by_key(|package| package.root.components().count())
            .map_or(&[], |package| package.aliases.as_slice())
    }
}
impl ResolutionAlias {
    pub(crate) fn expand(&self, candidate: &str) -> Option<String> {
        let middle = candidate
            .strip_prefix(&self.prefix)?
            .strip_suffix(&self.suffix)?;
        Some(self.replacement.replace('*', middle))
    }
}

impl ResolutionRules {
    pub(crate) fn new(
        metadata: crate::project_metadata::ProjectMetadata,
        mut packages: Vec<PackageResolution>,
    ) -> ResolutionRules {
        for issue in &metadata.issues {
            for package in packages
                .iter_mut()
                .filter(|package| package.root.starts_with(&issue.root))
            {
                match &mut package.issue {
                    Some(message) => {
                        message.push_str("; ");
                        message.push_str(&issue.reason);
                    }
                    None => package.issue = Some(issue.reason.clone()),
                }
            }
        }
        ResolutionRules { packages, metadata }
    }
}
