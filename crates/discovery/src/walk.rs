//! Walk configuration with git ignore semantics.
//!
//! The walk is serial and deterministic: entries are yielded in file-name
//! order per directory, git ignore files apply with their full semantics
//! (root and nested `.gitignore`, `.git/info/exclude`, the global gitignore,
//! and ancestor ignore files when a subpath is analyzed), and dependency
//! directories are always pruned regardless of ignore-file content.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::{DirEntry, Walk, WalkBuilder};

/// Whether a directory name is an always-excluded dependency directory.
///
/// This rule applies regardless of ignore-file content and wins over `!`
/// negations, because dependency directories are promised excluded by
/// default.
pub(crate) fn is_dependency_dir(name: &OsStr) -> bool {
    matches!(
        name.to_str(),
        Some(".git" | ".hg" | ".svn" | "target" | "node_modules" | "vendor")
    )
}

/// Compiles configuration exclude patterns as gitignore syntax anchored at
/// the analyzed root.
///
/// An invalid pattern excludes nothing rather than failing the walk.
pub(crate) fn config_excludes(root: &Path, patterns: &[String]) -> Gitignore {
    let mut builder = GitignoreBuilder::new(root);
    for pattern in patterns {
        let _ignored_invalid_pattern = builder.add_line(None, pattern);
    }
    builder.build().unwrap_or_else(|_| Gitignore::empty())
}

/// Whether a directory below the analyzed root is its own git checkout.
///
/// A `.git` entry of either shape marks one: a directory for submodules and
/// embedded clones, a file for linked worktrees. One existence check per
/// directory is the whole cost.
fn is_nested_checkout(entry: &DirEntry) -> bool {
    entry.path().join(".git").exists()
}

/// Builds the one serial source walk over the selected root.
///
/// Every nested checkout the walk prunes is pushed onto `nested_checkouts`
/// in walk order so inventory can disclose it.
pub(crate) fn source_walk(
    root: &Path,
    excludes: Gitignore,
    nested_checkouts: Arc<Mutex<Vec<PathBuf>>>,
) -> Walk {
    WalkBuilder::new(root)
        .hidden(false)
        .ignore(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .require_git(false)
        .parents(true)
        .follow_links(false)
        .sort_by_file_name(OsStr::cmp)
        .filter_entry(move |entry| keep_entry(entry, &excludes, &nested_checkouts))
        .build()
}

/// Decides whether the walk keeps one yielded entry.
///
/// The analyzed root itself is always kept, even when it is a checkout. A
/// dependency directory is always pruned, even when an ignore-file negation
/// re-includes it. A nested checkout is pruned next and recorded, so it is
/// disclosed even when a configuration pattern would also exclude it.
/// Configuration excludes apply last, so their own `!` negations can
/// re-include candidates their earlier patterns excluded.
fn keep_entry(
    entry: &DirEntry,
    excludes: &Gitignore,
    nested_checkouts: &Mutex<Vec<PathBuf>>,
) -> bool {
    if entry.depth() == 0 {
        return true;
    }
    let is_dir = entry.file_type().is_some_and(|kind| kind.is_dir());
    if is_dir && is_dependency_dir(entry.file_name()) {
        return false;
    }
    if is_dir && is_nested_checkout(entry) {
        nested_checkouts
            .lock()
            .expect("the serial walk never poisons the nested-checkout list")
            .push(entry.path().to_path_buf());
        return false;
    }
    !excludes.matched(entry.path(), is_dir).is_ignore()
}

/// Returns the path a walk error names, when it names one.
pub(crate) fn error_path(error: &ignore::Error) -> Option<&Path> {
    match error {
        ignore::Error::WithPath { path, .. } => Some(path),
        ignore::Error::WithDepth { err, .. } | ignore::Error::WithLineNumber { err, .. } => {
            error_path(err)
        }
        ignore::Error::Partial(errors) => errors.iter().find_map(error_path),
        _ => None,
    }
}
