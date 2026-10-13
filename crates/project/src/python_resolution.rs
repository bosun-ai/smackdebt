//! Python module lookup over the repository inventory, without loading Python.
use crate::named_dependencies::NamedResolution;
use smackdebt_analysis::FileId;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub(crate) fn resolve(
    source: &Path,
    module: &str,
    names: &[String],
    index: &BTreeMap<PathBuf, FileId>,
    package_roots: &[PathBuf],
) -> NamedResolution {
    let relative = module.starts_with('.');
    let Some(base) = import_path(source, module) else {
        return NamedResolution::Unresolved("relative import leaves the repository".to_owned());
    };
    let roots = if relative {
        vec![PathBuf::new()]
    } else {
        import_roots(source, index, package_roots)
    };
    let mut matched_roots = Vec::new();
    for root in roots {
        match resolve_at_root(&root, &base, names, index) {
            NamedResolution::Files(files) if !files.is_empty() => matched_roots.push(files),
            NamedResolution::Files(_) | NamedResolution::External => {}
            issue => return issue,
        }
    }
    matched_roots.sort();
    matched_roots.dedup();
    match matched_roots.len() {
        0 if relative => NamedResolution::Unresolved("no repository file matches".to_owned()),
        0 => NamedResolution::External,
        1 => NamedResolution::Files(matched_roots.pop().unwrap()),
        _ => NamedResolution::Ambiguous,
    }
}

fn module_files(path: &Path, index: &BTreeMap<PathBuf, FileId>) -> Vec<FileId> {
    [path.with_extension("py"), path.join("__init__.py")]
        .iter()
        .filter_map(|path| index.get(path).copied())
        .collect()
}

fn import_path(source: &Path, module: &str) -> Option<PathBuf> {
    if !module.starts_with('.') {
        return Some(PathBuf::from(module.replace('.', "/")));
    }
    let mut directory = source.parent().unwrap_or(Path::new(""));
    for _ in 1..module.chars().take_while(|c| *c == '.').count() {
        directory = directory.parent()?;
    }
    Some(directory.join(module.trim_start_matches('.').replace('.', "/")))
}

fn import_roots(
    source: &Path,
    index: &BTreeMap<PathBuf, FileId>,
    package_roots: &[PathBuf],
) -> Vec<PathBuf> {
    let mut roots = BTreeSet::from([PathBuf::new()]);
    if let Some(root) = package_roots
        .iter()
        .filter(|root| source.starts_with(root))
        .max_by_key(|root| root.components().count())
    {
        roots.insert(root.clone());
        roots.insert(root.join("src"));
    }
    let mut directory = source.parent().unwrap_or(Path::new(""));
    let regular_package = index.contains_key(&directory.join("__init__.py"));
    while index.contains_key(&directory.join("__init__.py")) {
        let Some(parent) = directory.parent() else {
            break;
        };
        directory = parent;
    }
    if regular_package {
        roots.insert(directory.to_path_buf());
    }
    roots.into_iter().collect()
}

fn resolve_at_root(
    root: &Path,
    base: &Path,
    names: &[String],
    index: &BTreeMap<PathBuf, FileId>,
) -> NamedResolution {
    let path = root.join(base);
    let parent = module_files(&path, index);
    if parent.len() > 1 {
        return NamedResolution::Ambiguous;
    }
    let mut files: BTreeSet<_> = parent.into_iter().collect();
    if !index.contains_key(&path.with_extension("py")) {
        for name in names {
            let members = module_files(&path.join(name), index);
            if members.len() > 1 {
                return NamedResolution::Ambiguous;
            }
            files.extend(members);
        }
    }
    if !files.is_empty() && !add_initializers(&path, root, index, &mut files) {
        return NamedResolution::Unresolved("a module file cannot contain submodules".to_owned());
    }
    NamedResolution::Files(files.into_iter().collect())
}

fn add_initializers(
    path: &Path,
    root: &Path,
    index: &BTreeMap<PathBuf, FileId>,
    files: &mut BTreeSet<FileId>,
) -> bool {
    let mut parent = path.parent();
    while let Some(directory) =
        parent.filter(|directory| directory.starts_with(root) && *directory != root)
    {
        if index.contains_key(&directory.with_extension("py")) {
            return false;
        }
        files.extend(index.get(&directory.join("__init__.py")).copied());
        parent = directory.parent();
    }
    true
}
