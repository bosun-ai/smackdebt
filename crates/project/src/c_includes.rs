//! Compilation database data and ordered C-family include lookup.
mod command;
mod paths;
mod search_paths;
mod standard_headers;
#[cfg(test)]
mod tests;

use command::command_words;
use paths::{relative_path, repository_path};
use search_paths::{SearchPaths, search_paths};
use smackdebt_analysis::IncludeStyle;
use standard_headers::standard_header;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Default)]
pub(crate) struct CIncludeConfig {
    files: BTreeMap<PathBuf, Vec<SearchPaths>>,
    shared: Option<SearchPaths>,
    conflicts: bool,
}
impl CIncludeConfig {
    pub(crate) fn parse(root: &Path, source: &str) -> Result<Self, String> {
        let entries: serde_json::Value =
            serde_json::from_str(source).map_err(|_| "compilation database is not valid JSON")?;
        let entries = entries
            .as_array()
            .ok_or("compilation database must be an array")?;
        let mut result = Self::default();
        for entry in entries {
            let directory = entry["directory"]
                .as_str()
                .ok_or("compile command has no directory")?;
            let directory = repository_path(root, Path::new(""), directory)
                .ok_or("compile command directory is outside the repository")?;
            let file = entry["file"]
                .as_str()
                .ok_or("compile command has no source file")?;
            let file = repository_path(root, &directory, file)
                .ok_or("compile command source is outside the repository")?;
            let args = if let Some(args) = entry["arguments"].as_array() {
                args.iter()
                    .map(|arg| {
                        arg.as_str()
                            .map(str::to_owned)
                            .ok_or("compile arguments must be strings".to_owned())
                    })
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                command_words(
                    entry["command"]
                        .as_str()
                        .ok_or("compile command has no arguments")?,
                )?
            };
            let paths = search_paths(root, &directory, &args)?;
            let configs = result.files.entry(file).or_default();
            if !configs.contains(&paths) {
                configs.push(paths);
            }
        }
        let mut all = result.files.values().flatten();
        result.shared = all.next().cloned();
        result.conflicts = all.any(|paths| Some(paths) != result.shared.as_ref());
        Ok(result)
    }
}

pub(crate) fn resolve(
    configs: &[CIncludeConfig],
    source: &Path,
    target: &str,
    style: IncludeStyle,
    mut exists: impl FnMut(&Path) -> bool,
) -> Result<Option<PathBuf>, String> {
    if Path::new(target).is_absolute() || target.contains('\\') {
        return Err("include target is not a repository path".to_owned());
    }
    let first = select_configuration(configs, source)?;
    if let Some(path) = find_beside_source(source, target, style, &mut exists) {
        return Ok(Some(path));
    }
    if let Some(paths) = first
        && let Some(path) = find_in_search_paths(paths, target, style, &mut exists)?
    {
        return Ok(Some(path));
    }

    missing_target(first, target, style)
}
fn missing_target(
    first: Option<&SearchPaths>,
    target: &str,
    style: IncludeStyle,
) -> Result<Option<PathBuf>, String> {
    if style == IncludeStyle::Angle && standard_header(target) {
        return Ok(None);
    }
    if first.is_some_and(|paths| !paths.after.is_empty()) {
        return Err("include search after compiler system paths cannot be verified".to_owned());
    }
    Err(if first.is_none() {
        "include has no compilation database search paths"
    } else {
        "include target was not found in configured repository paths"
    }
    .to_owned())
}
fn find_beside_source(
    source: &Path,
    target: &str,
    style: IncludeStyle,
    exists: &mut impl FnMut(&Path) -> bool,
) -> Option<PathBuf> {
    if style != IncludeStyle::Quoted {
        return None;
    }
    let path = relative_path(&source.parent().unwrap_or(Path::new("")).join(target))?;
    exists(&path).then_some(path)
}

fn select_configuration<'a>(
    configs: &'a [CIncludeConfig],
    source: &Path,
) -> Result<Option<&'a SearchPaths>, String> {
    let mut direct = configs
        .iter()
        .filter_map(|config| config.files.get(source))
        .flatten();
    if let Some(first) = direct.next() {
        if direct.any(|paths| paths != first) {
            return Err("include has conflicting compile configurations".to_owned());
        }
        return Ok(Some(first));
    }
    let is_header = source
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| matches!(extension, "h" | "hh" | "hpp" | "hxx"));
    if !is_header {
        return Ok(None);
    }
    shared_configuration(configs)
}

fn shared_configuration(configs: &[CIncludeConfig]) -> Result<Option<&SearchPaths>, String> {
    // Header lookup uses the search order shared by every translation unit.
    let mut shared = configs.iter().filter_map(|config| config.shared.as_ref());
    let first = shared.next();
    if configs.iter().any(|config| config.conflicts) || shared.any(|paths| Some(paths) != first) {
        return Err("include has conflicting compile configurations".to_owned());
    }
    Ok(first)
}

fn find_in_search_paths(
    paths: &SearchPaths,
    target: &str,
    style: IncludeStyle,
    exists: &mut impl FnMut(&Path) -> bool,
) -> Result<Option<PathBuf>, String> {
    let quotes = if style == IncludeStyle::Quoted {
        paths.quote.as_slice()
    } else {
        &[]
    };
    for directory in quotes.iter().chain(&paths.include).chain(&paths.system) {
        let Some(directory) = directory else {
            return external_directory(target, style);
        };
        if let Some(path) = relative_path(&directory.join(target))
            && exists(&path)
        {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

fn external_directory(target: &str, style: IncludeStyle) -> Result<Option<PathBuf>, String> {
    if style == IncludeStyle::Angle && standard_header(target) {
        Ok(None)
    } else {
        Err("include search reaches a directory outside the repository".to_owned())
    }
}
