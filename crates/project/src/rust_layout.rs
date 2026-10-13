//! Rust module layout: which files a symbolic path may resolve to.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use smackdebt_analysis::FileId;

mod declarations;

/// Resolves candidates that name a file by role instead of by path.
///
/// Symbolic candidates are the last resort of one reference: they are consulted
/// only when no path candidate of the same reference matched a discovered file.
pub(crate) fn resolve_symbolic_candidates(
    source: &Path,
    candidates: &[String],
    index: &BTreeMap<PathBuf, FileId>,
) -> Vec<FileId> {
    let mut matches = Vec::new();
    for candidate in candidates {
        let file = match candidate.as_str() {
            smackdebt_analysis::DECLARING_FILE_CANDIDATE => index.get(source).copied(),
            smackdebt_analysis::CRATE_ROOT_CANDIDATE => rust_crate_root_file(source, index),
            smackdebt_analysis::PARENT_MODULE_CANDIDATE => rust_parent_module_file(source, index),
            _ => None,
        };
        matches.extend(file);
    }
    matches
}
/// The file a Rust package presents as the root of its module tree.
pub(crate) fn rust_crate_root_file(
    source: &Path,
    index: &BTreeMap<PathBuf, FileId>,
) -> Option<FileId> {
    let root = rust_source_root(source)?;
    index
        .get(&root.join("lib.rs"))
        .or_else(|| index.get(&root.join("main.rs")))
        .copied()
}
/// The file declaring the module that declares a Rust file.
///
/// The enclosing module is the directory holding the file's own module
/// directory, and Rust spells that module in three places: `a/mod.rs` inside
/// it, `a.rs` beside it, and the crate root when the module is the source root
/// itself.  The first spelling the repository holds is the answer.
pub(crate) fn rust_parent_module_file(
    source: &Path,
    index: &BTreeMap<PathBuf, FileId>,
) -> Option<FileId> {
    let directory = rust_module_directory(source)?;
    let parent = directory.parent()?;
    let name = parent.file_name()?.to_str()?;
    let root = rust_source_root(source).filter(|root| root.as_path() == parent);
    [
        parent.join("mod.rs"),
        parent.with_file_name(format!("{name}.rs")),
    ]
    .into_iter()
    .chain(
        root.into_iter()
            .flat_map(|root| [root.join("lib.rs"), root.join("main.rs")]),
    )
    .find_map(|path| index.get(&path).copied())
}
/// The directory a Rust file's own modules live in.
///
/// `mod.rs`, `lib.rs`, and `main.rs` are the module of their directory; every
/// other file is a module that owns a directory named after it.
pub(crate) fn rust_module_directory(source: &Path) -> Option<PathBuf> {
    let parent = source.parent()?;
    let stem = source.file_stem()?.to_str()?;
    Some(if matches!(stem, "mod" | "lib" | "main") {
        parent.to_path_buf()
    } else {
        parent.join(stem)
    })
}
pub(crate) fn rust_source_root(source: &Path) -> Option<PathBuf> {
    let mut root = PathBuf::new();
    for component in source.parent()?.components() {
        let std::path::Component::Normal(value) = component else {
            return None;
        };
        root.push(value);
        if value == "src" {
            return Some(root);
        }
    }
    None
}

pub(crate) fn resolve_explicit_module(
    source: &Path,
    reference: &smackdebt_analysis::DependencySyntax,
    index: &BTreeMap<PathBuf, FileId>,
) -> Option<Vec<FileId>> {
    if reference.relation() != smackdebt_analysis::StaticRelationKind::ModuleOwnership {
        return None;
    }
    let module = reference.module_path()?;
    let smackdebt_analysis::DependencySyntaxState::Candidates(candidates) = reference.state()
    else {
        return None;
    };
    let inline = module
        .rsplit_once("::")
        .map(|(prefix, _)| prefix.replace("::", "/"))
        .unwrap_or_default();
    let parent = if inline.is_empty() {
        source.parent().unwrap_or(Path::new("")).to_path_buf()
    } else {
        rust_module_directory(source)?
    };
    Some(
        candidates
            .iter()
            .filter_map(|candidate| {
                crate::paths::clean_relative(&parent.join(&inline).join(candidate))
            })
            .filter_map(|path| index.get(&path).copied())
            .collect(),
    )
}

/// Explicit module files and their declared children, indexed by logical module path.
#[derive(Default)]
pub(crate) struct RustModules {
    paths: BTreeMap<PathBuf, FileId>,
    logical_sources: BTreeMap<FileId, PathBuf>,
}
impl RustModules {
    pub(crate) fn new(
        sources: &[crate::dependencies::SourceDependencies],
        index: &BTreeMap<PathBuf, FileId>,
    ) -> Self {
        declarations::ModuleDeclarations::collect(sources, index).finish()
    }

    pub(crate) fn resolve(
        &self,
        source: &crate::dependencies::SourceDependencies,
        reference: &smackdebt_analysis::DependencySyntax,
    ) -> Option<Vec<FileId>> {
        if source.language != smackdebt_analysis::Language::Rust {
            return None;
        }
        let inline = reference.module_path().unwrap_or("");
        let base = self
            .logical_sources
            .get(&source.file)
            .cloned()
            .or_else(|| rust_module_directory(&source.path))?;
        if reference.relation() == smackdebt_analysis::StaticRelationKind::ModuleOwnership {
            return self.resolve_declaration(&base, reference);
        }
        let mut path = reference_path(&source.path, &base, inline, reference.target())?;
        loop {
            if let Some(file) = self.paths.get(&path) {
                return Some(vec![*file]);
            }
            if !path.pop() {
                return None;
            }
        }
    }
    fn resolve_declaration(
        &self,
        base: &Path,
        reference: &smackdebt_analysis::DependencySyntax,
    ) -> Option<Vec<FileId>> {
        let module = reference
            .module_path()
            .unwrap_or_else(|| reference.target());
        let target = self
            .paths
            .get(&base.join(module.replace("::", "/")))
            .copied();
        if reference.module_path().is_some() {
            Some(target.into_iter().collect())
        } else {
            target.map(|file| vec![file])
        }
    }
}

#[cfg(test)]
mod tests;

fn reference_path(source: &Path, base: &Path, inline: &str, target: &str) -> Option<PathBuf> {
    let mut segments = target.split("::");
    let mut path = match segments.next()? {
        "crate" => rust_source_root(source)
            .unwrap_or_else(|| source.parent().unwrap_or(Path::new("")).to_path_buf()),
        "self" => base.join(inline.replace("::", "/")),
        "super" => {
            let mut path = base.join(inline.replace("::", "/"));
            path.pop();
            path
        }
        _ => return None,
    };
    for segment in segments {
        if segment == "super" {
            path.pop();
        } else {
            path.push(segment);
        }
    }
    Some(path)
}
