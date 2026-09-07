//! What changed between the two trees: the selected changes, the package
//! positions both trees share, and each side's resolution rules.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use smackdebt_analysis::{Language, PackageId, PackageRecord, Scope, ScopeId};
use smackdebt_discovery::{Inventory, SnapshotInventory, discover_snapshot};
use smackdebt_git::{Change, GitRepository, ObjectReader};
use smackdebt_languages::Analyzer;

use crate::dependencies::DiffSideSelector;
use crate::hierarchy::HierarchyBuilder;
use crate::manifest_names::declared_manifest_name;
use crate::paths::{package_of, report_package_path};
use crate::requests::ProjectError;
use crate::resolution_config::ResolutionRules;

/// Resolves the reference a diff answers against and the merge base the
/// changes are stated from.
pub(crate) fn resolve_diff_refs(
    repository: &GitRepository,
    requested: Option<&str>,
) -> Result<(String, String), ProjectError> {
    let reference = match requested {
        Some(reference) => reference.to_owned(),
        None => repository
            .default_ref()?
            .ok_or(ProjectError::MissingReference)?,
    };
    let base = repository
        .merge_base(&reference, "HEAD")
        .map_err(|error| match error {
            // Git reports an unknown ref through a failed command, so the
            // failure is restated as the fixable value the user supplied.
            smackdebt_git::GitError::Command { .. } | smackdebt_git::GitError::MissingObject(_) => {
                ProjectError::UnknownReference(reference.clone())
            }
            other => ProjectError::Git(other),
        })?;
    Ok((reference, base))
}
/// The one ignore-aware walk of the current tree.
pub(crate) fn discover_current_tree(repository: &GitRepository) -> Result<Inventory, ProjectError> {
    let inventory =
        Inventory::discover_sources(repository.root(), Vec::new()).map_err(|source| {
            ProjectError::Inspect {
                path: repository.root().to_path_buf(),
                source,
            }
        })?;
    #[cfg(feature = "evidence-stats")]
    crate::evidence::record_inventory(inventory.visited_entries());
    Ok(inventory)
}
/// The base tree's inventory, read through the batched object reader.
pub(crate) fn discover_base_tree(
    repository: &GitRepository,
    batch: &mut ObjectReader,
    base: &str,
) -> Result<SnapshotInventory, ProjectError> {
    let base_tree_files = batch.tree_files(base)?;
    let mut base_metadata: BTreeMap<PathBuf, Result<Vec<u8>, String>> = BTreeMap::new();
    discover_snapshot(repository.root(), &base_tree_files, |path| {
        if let Some(source) = base_metadata.get(path) {
            return source.clone().map_err(std::io::Error::other);
        }
        let source = batch
            .read_path(base, path)
            .map_err(|error| error.to_string());
        base_metadata.insert(path.to_path_buf(), source.clone());
        source.map_err(std::io::Error::other)
    })
    .map_err(|source| ProjectError::Inspect {
        path: repository.root().to_path_buf(),
        source,
    })
}
/// The files the two trees disagree about, in one ordered table.
pub(crate) struct DiffChangeSet {
    pub(crate) changed: Vec<SelectedChange>,
    pub(crate) all_changed: Vec<SelectedChange>,
    pub(crate) selected_paths: BTreeSet<PathBuf>,
}
/// Merges the changes Git reports with the paths only one snapshot holds,
/// keeps the analyzable ones, and notes which fall inside the selected scope.
pub(crate) fn select_diff_changes(
    inventory: &Inventory,
    base_inventory: &SnapshotInventory,
    changes: Vec<Change>,
    path_filter: Option<&Path>,
) -> DiffChangeSet {
    let current_sources = inventory
        .source_files()
        .map(|file| file.path().as_path().to_path_buf())
        .collect::<BTreeSet<_>>();
    let base_sources = base_inventory
        .source_paths()
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let changed_paths = changes
        .iter()
        .flat_map(|change| {
            [
                change.current_path().to_path_buf(),
                change.base_path().to_path_buf(),
            ]
        })
        .collect::<BTreeSet<_>>();
    let mut changed = changes
        .into_iter()
        .filter_map(|change| SelectedChange::from_git(change, &current_sources, &base_sources))
        .collect::<Vec<_>>();
    for path in current_sources.symmetric_difference(&base_sources) {
        if !changed_paths.contains(path.as_path()) {
            changed.push(SelectedChange::from_snapshot(
                path.clone(),
                current_sources.contains(path),
                base_sources.contains(path),
            ));
        }
    }
    changed.sort_by(|left, right| left.current_path().cmp(right.current_path()));
    let all_changed = changed.clone();
    changed.retain(|entry| Analyzer::language(entry.current_path()) != Language::Unknown);
    let selected_paths: BTreeSet<_> = changed
        .iter()
        .filter(|entry| path_filter.is_none_or(|path| entry.current_path().starts_with(path)))
        .map(|entry| entry.current_path().to_path_buf())
        .collect();
    DiffChangeSet {
        changed,
        all_changed,
        selected_paths,
    }
}
/// Every resolution configuration either tree declares or the change touched.
pub(crate) fn base_resolution_configs(
    inventory: &Inventory,
    base_inventory: &SnapshotInventory,
    all_changed: &[SelectedChange],
) -> Vec<PathBuf> {
    let mut candidates: Vec<_> = inventory
        .packages()
        .iter()
        .filter_map(|package| package.resolution_config())
        .map(|path| path.as_path().to_path_buf())
        .collect();
    candidates.extend(base_inventory.resolution_configs().iter().cloned());
    for change in all_changed {
        for path in [change.current_path(), change.base_path()] {
            if matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some("tsconfig.json" | "jsconfig.json")
            ) {
                candidates.push(path.to_path_buf());
            }
        }
    }
    candidates.sort();
    candidates.dedup();
    candidates
}
/// The package positions the two trees share, current roots first, and the
/// resolution configuration either tree declares.
pub(crate) struct DiffPackages {
    pub(crate) roots: Vec<PathBuf>,
    pub(crate) current_roots: Vec<PathBuf>,
    pub(crate) before_roots: Vec<PathBuf>,
    pub(crate) before_manifest_names: Vec<Option<String>>,
    pub(crate) resolution_configs: Vec<PathBuf>,
}
impl DiffPackages {
    pub(crate) fn of(
        inventory: &Inventory,
        base_inventory: &SnapshotInventory,
        all_changed: &[SelectedChange],
    ) -> Self {
        let current_roots: Vec<_> = inventory
            .packages()
            .iter()
            .map(|package| package.root().as_path().to_path_buf())
            .collect();
        let before_roots = base_inventory
            .packages()
            .iter()
            .map(|(root, _)| root.clone())
            .collect::<Vec<_>>();
        let resolution_configs = base_resolution_configs(inventory, base_inventory, all_changed);
        let mut base_only_roots = before_roots.clone();
        base_only_roots.retain(|root| !current_roots.contains(root));
        base_only_roots.sort();
        base_only_roots.dedup();
        let mut roots = current_roots.clone();
        roots.extend(base_only_roots.iter().cloned());
        let before_manifest_names = roots
            .iter()
            .map(|root| {
                base_inventory
                    .packages()
                    .iter()
                    .find(|(candidate, _)| candidate == root)
                    .and_then(|(_, name)| name.clone())
            })
            .collect::<Vec<_>>();
        Self {
            roots,
            current_roots,
            before_roots,
            before_manifest_names,
            resolution_configs,
        }
    }
}
/// The scopes a diff places its files in, and the packages it will publish.
pub(crate) struct DiffHierarchy {
    pub(crate) scopes: Vec<Scope>,
    pub(crate) file_scopes: BTreeMap<PathBuf, ScopeId>,
    pub(crate) packages: Vec<PackageRecord>,
}
/// Builds the repository hierarchy from the shared package positions, placing
/// changed files first and every other analyzable current file after them.
pub(crate) fn build_diff_hierarchy(
    inventory: &Inventory,
    changed: &[SelectedChange],
    packages: &DiffPackages,
) -> DiffHierarchy {
    let mut hierarchy = HierarchyBuilder::new(".".to_owned(), &packages.roots);
    let package_records: Vec<_> = packages
        .roots
        .iter()
        .enumerate()
        .map(|(index, root)| {
            let id = PackageId::from_index(index);
            let scope = hierarchy.package_scopes[index];
            let path = report_package_path(root);
            let record = if index < packages.current_roots.len() {
                PackageRecord::current(id, scope, path)
            } else {
                PackageRecord::base_only(id, scope, path)
            };
            record.with_manifest_name(declared_manifest_name(inventory, root))
        })
        .collect();
    let changed_paths: BTreeSet<_> = changed
        .iter()
        .map(|entry| entry.current_path().to_path_buf())
        .collect();
    for entry in changed {
        let package = package_of(entry.current_path(), &packages.roots, &packages.roots);
        hierarchy.add_file(entry.current_path(), package.index());
    }
    for file in inventory
        .source_files()
        .filter(|file| Analyzer::language(file.path().as_path()) != Language::Unknown)
    {
        if changed_paths.contains(file.path().as_path()) {
            continue;
        }
        let package = package_of(file.path().as_path(), &packages.roots, &packages.roots);
        hierarchy.add_file(file.path().as_path(), package.index());
    }
    DiffHierarchy {
        scopes: hierarchy.scopes,
        file_scopes: hierarchy.file_scopes,
        packages: package_records,
    }
}
/// Both trees' resolution rules, selected by side.
#[derive(Clone, Copy)]
pub(crate) struct DiffAliases<'a> {
    pub(crate) current: &'a ResolutionRules,
    pub(crate) before: &'a ResolutionRules,
}
impl DiffAliases<'_> {
    pub(crate) const fn select(&self, side: DiffSideSelector) -> &ResolutionRules {
        match side {
            DiffSideSelector::Current => self.current,
            DiffSideSelector::Before => self.before,
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) struct SelectedChange {
    pub(crate) current_path: PathBuf,
    pub(crate) base_path: PathBuf,
    pub(crate) current_exists: bool,
    pub(crate) base_exists: bool,
}
impl SelectedChange {
    pub(crate) fn from_git(
        change: Change,
        current_sources: &BTreeSet<PathBuf>,
        base_sources: &BTreeSet<PathBuf>,
    ) -> Option<Self> {
        let current_exists =
            change.current_exists() && current_sources.contains(change.current_path());
        let base_exists = change.base_exists() && base_sources.contains(change.base_path());
        (current_exists || base_exists).then(|| Self {
            current_path: change.current_path().to_path_buf(),
            base_path: change.base_path().to_path_buf(),
            current_exists,
            base_exists,
        })
    }

    pub(crate) fn from_snapshot(path: PathBuf, current_exists: bool, base_exists: bool) -> Self {
        debug_assert!(current_exists || base_exists);
        Self {
            current_path: path.clone(),
            base_path: path,
            current_exists,
            base_exists,
        }
    }

    pub(crate) fn current_path(&self) -> &Path {
        &self.current_path
    }

    pub(crate) fn base_path(&self) -> &Path {
        &self.base_path
    }

    pub(crate) const fn current_exists(&self) -> bool {
        self.current_exists
    }

    pub(crate) const fn base_exists(&self) -> bool {
        self.base_exists
    }
}
