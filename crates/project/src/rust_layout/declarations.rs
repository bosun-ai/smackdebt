use super::{RustModules, resolve_explicit_module, rust_module_directory};
use crate::dependencies::SourceDependencies;
use smackdebt_analysis::{DependencySyntax, DependencySyntaxState, FileId, StaticRelationKind};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub(super) struct ModuleDeclarations<'a> {
    declarations: Vec<(FileId, &'a str, FileId)>,
    parents: BTreeMap<FileId, (FileId, &'a str)>,
    sources: BTreeMap<FileId, &'a Path>,
    explicit_files: BTreeSet<FileId>,
}
impl<'a> ModuleDeclarations<'a> {
    pub(super) fn collect(
        sources: &'a [SourceDependencies],
        index: &BTreeMap<PathBuf, FileId>,
    ) -> Self {
        let mut result = Self {
            declarations: Vec::new(),
            parents: BTreeMap::new(),
            explicit_files: BTreeSet::new(),
            sources: sources
                .iter()
                .map(|source| (source.file, source.path.as_path()))
                .collect(),
        };
        result.collect_explicit(sources, index);
        if !result.parents.is_empty() {
            result.collect_ordinary(sources, index);
        }
        result
    }

    fn collect_explicit(
        &mut self,
        sources: &'a [SourceDependencies],
        index: &BTreeMap<PathBuf, FileId>,
    ) {
        for source in sources {
            for reference in &source.references {
                let Some(module) = reference.module_path() else {
                    continue;
                };
                for file in
                    resolve_explicit_module(&source.path, reference, index).unwrap_or_default()
                {
                    self.declarations.push((source.file, module, file));
                    self.parents.insert(file, (source.file, module));
                    self.explicit_files.insert(file);
                }
            }
        }
    }

    fn collect_ordinary(
        &mut self,
        sources: &'a [SourceDependencies],
        index: &BTreeMap<PathBuf, FileId>,
    ) {
        for source in sources {
            for reference in &source.references {
                if let Some(file) = self.ordinary_target(source, reference, index) {
                    self.declarations
                        .push((source.file, reference.target(), file));
                    self.parents
                        .entry(file)
                        .or_insert((source.file, reference.target()));
                }
            }
        }
    }

    fn ordinary_target(
        &self,
        source: &SourceDependencies,
        reference: &DependencySyntax,
        index: &BTreeMap<PathBuf, FileId>,
    ) -> Option<FileId> {
        if reference.relation() != StaticRelationKind::ModuleOwnership
            || reference.module_path().is_some()
        {
            return None;
        }
        let DependencySyntaxState::Candidates(candidates) = reference.state() else {
            return None;
        };
        let directory = if self.explicit_files.contains(&source.file) {
            source.path.parent().map(Path::to_path_buf)
        } else {
            rust_module_directory(&source.path)
        }?;
        candidates
            .iter()
            .filter_map(|candidate| crate::paths::clean_relative(&directory.join(candidate)))
            .find_map(|path| index.get(&path).copied())
    }

    fn explicit_descendants(&self) -> BTreeSet<FileId> {
        let mut children: BTreeMap<FileId, Vec<FileId>> = BTreeMap::new();
        for (source, _, target) in &self.declarations {
            children.entry(*source).or_default().push(*target);
        }
        let mut selected = BTreeSet::new();
        let mut pending: Vec<_> = self.explicit_files.iter().copied().collect();
        while let Some(file) = pending.pop() {
            if selected.insert(file)
                && let Some(children) = children.get(&file)
            {
                pending.extend(children);
            }
        }
        selected
    }

    pub(super) fn finish(self) -> RustModules {
        if self.parents.is_empty() {
            return RustModules::default();
        }
        let selected = self.explicit_descendants();
        let mut logical_sources = BTreeMap::new();
        let mut active = BTreeSet::new();
        let mut paths = BTreeMap::new();
        for (source, module, target) in self.declarations {
            if !selected.contains(&target) {
                continue;
            }
            if let Some(base) = logical_directory(
                source,
                &self.sources,
                &self.parents,
                &mut logical_sources,
                &mut active,
            ) {
                paths.insert(base.join(module.replace("::", "/")), target);
            }
        }
        for file in self.parents.keys() {
            logical_directory(
                *file,
                &self.sources,
                &self.parents,
                &mut logical_sources,
                &mut active,
            );
        }
        RustModules {
            paths,
            logical_sources,
        }
    }
}

fn logical_directory(
    file: FileId,
    sources: &BTreeMap<FileId, &Path>,
    parents: &BTreeMap<FileId, (FileId, &str)>,
    cache: &mut BTreeMap<FileId, PathBuf>,
    active: &mut std::collections::BTreeSet<FileId>,
) -> Option<PathBuf> {
    if let Some(path) = cache.get(&file) {
        return Some(path.clone());
    }
    if !active.insert(file) {
        return None;
    }
    let path = if let Some((parent, module)) = parents.get(&file) {
        logical_directory(*parent, sources, parents, cache, active)
            .map(|path| path.join(module.replace("::", "/")))
    } else {
        sources
            .get(&file)
            .and_then(|source| rust_module_directory(source))
    };
    active.remove(&file);
    if let Some(path) = &path {
        cache.insert(file, path.clone());
    }
    path
}
