//! Repository-relative normalization without filesystem access.
use std::path::{Component, Path, PathBuf};

pub(super) fn repository_path(root: &Path, directory: &Path, value: &str) -> Option<PathBuf> {
    let path = Path::new(value);
    if path.is_absolute() {
        relative_path(path.strip_prefix(root).ok()?)
    } else {
        relative_path(&directory.join(path))
    }
}
pub(super) fn relative_path(path: &Path) -> Option<PathBuf> {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => result.push(value),
            Component::CurDir => {}
            Component::ParentDir => {
                if !result.pop() {
                    return None;
                }
            }
            _ => return None,
        }
    }
    Some(result)
}
