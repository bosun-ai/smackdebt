//! Compiler search flags and their ordered directory classes.
use super::paths::repository_path;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct SearchPaths {
    pub(super) quote: Vec<Option<PathBuf>>,
    pub(super) include: Vec<Option<PathBuf>>,
    pub(super) system: Vec<Option<PathBuf>>,
    pub(super) after: Vec<Option<PathBuf>>,
}

pub(super) fn search_paths(
    root: &Path,
    directory: &Path,
    args: &[String],
) -> Result<SearchPaths, String> {
    let mut result = SearchPaths::default();
    let mut args = args.iter().skip(1);
    while let Some(arg) = args.next() {
        if unsupported_configuration(arg) {
            return Err("compile command uses unsupported include configuration".to_owned());
        }
        let Some((flag, value)) = search_flag(arg) else {
            continue;
        };
        let value = if value.is_empty() {
            args.next()
                .ok_or("include search flag has no directory")?
                .as_str()
        } else {
            value
        };
        let path = repository_path(root, directory, value);
        result.directories(flag).push(path);
    }
    result
        .include
        .retain(|path| path.is_none() || !result.system.contains(path));
    Ok(result)
}

fn unsupported_configuration(argument: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "@",
        "-nostdinc",
        "--sysroot",
        "-isysroot",
        "-iprefix",
        "-iwithprefix",
        "-include",
        "-imacros",
        "--include",
        "--imacros",
        "-Wp,",
        "/FI",
        "-FI",
        "/I",
    ];
    matches!(argument, "-I-" | "-Xpreprocessor" | "-Xclang")
        || PREFIXES.iter().any(|prefix| argument.starts_with(prefix))
}

fn search_flag(argument: &str) -> Option<(&str, &str)> {
    ["-iquote", "-isystem", "-idirafter", "-I"]
        .into_iter()
        .find_map(|flag| argument.strip_prefix(flag).map(|value| (flag, value)))
}

impl SearchPaths {
    fn directories(&mut self, flag: &str) -> &mut Vec<Option<std::path::PathBuf>> {
        match flag {
            "-iquote" => &mut self.quote,
            "-isystem" => &mut self.system,
            "-idirafter" => &mut self.after,
            _ => &mut self.include,
        }
    }
}
