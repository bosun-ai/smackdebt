//! Rust module layout: which files a symbolic path may resolve to.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use smackdebt_analysis::FileId;

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
