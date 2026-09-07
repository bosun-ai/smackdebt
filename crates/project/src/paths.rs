//! Repository-relative path arithmetic and package placement.

use std::path::{Path, PathBuf};

use smackdebt_analysis::PackageId;

/// Returns the report package that owns one repository path.
pub(crate) fn package_of(
    path: &Path,
    side_package_roots: &[PathBuf],
    package_roots: &[PathBuf],
) -> PackageId {
    let root = nearest_package_root(path, side_package_roots);
    PackageId::from_index(
        package_roots
            .iter()
            .position(|candidate| candidate == &root)
            .unwrap_or(0),
    )
}
pub(crate) fn clean_relative(path: &Path) -> Option<PathBuf> {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::Normal(value) => result.push(value),
            std::path::Component::ParentDir => {
                if !result.pop() {
                    return None;
                }
            }
            std::path::Component::RootDir | std::path::Component::Prefix(_) => return None,
        }
    }
    Some(result)
}
pub(crate) fn nearest_package_root(path: &Path, package_roots: &[PathBuf]) -> PathBuf {
    package_roots
        .iter()
        .filter(|root| path.starts_with(root))
        .max_by_key(|root| root.components().count())
        .cloned()
        .unwrap_or_default()
}
pub(crate) fn report_package_path(path: &Path) -> String {
    if path.as_os_str().is_empty() {
        ".".to_owned()
    } else {
        path.display().to_string()
    }
}
