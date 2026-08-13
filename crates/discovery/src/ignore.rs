use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub(super) struct Pattern {
    base: PathBuf,
    pattern: String,
    negated: bool,
    directory_only: bool,
}

pub(super) fn read_ignore_file(directory: &Path, relative: &Path) -> Vec<Pattern> {
    let path = directory.join(".gitignore");
    let Ok(contents) = fs::read_to_string(path) else {
        return Vec::new();
    };
    contents
        .lines()
        .filter_map(|line| parse_pattern(line, relative))
        .collect()
}

pub(super) fn parse_pattern(line: &str, base: &Path) -> Option<Pattern> {
    let mut value = line.trim();
    if value.is_empty() || value.starts_with('#') {
        return None;
    }
    let negated = value.starts_with('!');
    if negated {
        value = &value[1..];
    }
    let directory_only = value.ends_with('/');
    value = value.trim_end_matches('/');
    if value.is_empty() {
        return None;
    }
    Some(Pattern {
        base: base.to_path_buf(),
        pattern: value.trim_start_matches('/').replace('\\', "/"),
        negated,
        directory_only,
    })
}

pub(super) fn should_ignore(path: &Path, is_dir: bool, patterns: &[Pattern]) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    let mut ignored = false;
    for pattern in patterns {
        let base = pattern.base.to_string_lossy().replace('\\', "/");
        let candidate = if base.is_empty() {
            normalized.as_str()
        } else if normalized == base {
            ""
        } else if let Some(rest) = normalized.strip_prefix(&(base + "/")) {
            rest
        } else {
            continue;
        };
        if pattern.directory_only && !is_dir {
            continue;
        }
        let matches = glob_matches(&pattern.pattern, candidate)
            || (!pattern.pattern.contains('/')
                && candidate
                    .split('/')
                    .any(|part| glob_matches(&pattern.pattern, part)));
        if matches {
            ignored = !pattern.negated;
        }
    }
    ignored
}

pub fn glob_matches(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    let mut states = vec![false; text.len() + 1];
    states[0] = true;
    let mut pattern_index = 0;
    while pattern_index < pattern.len() {
        let character = pattern[pattern_index];
        let mut next = vec![false; text.len() + 1];
        for (index, active) in states.iter().copied().enumerate() {
            if !active {
                continue;
            }
            match character {
                '*' => {
                    next[index] = true;
                    let crosses_directories = pattern.get(pattern_index + 1) == Some(&'*');
                    for (text_index, slot) in next.iter_mut().enumerate().skip(index + 1) {
                        if !crosses_directories && text[text_index - 1] == '/' {
                            break;
                        }
                        *slot = true;
                    }
                }
                '?' if index < text.len() && text[index] != '/' => next[index + 1] = true,
                character if index < text.len() && character == text[index] => {
                    next[index + 1] = true
                }
                _ => {}
            }
        }
        states = next;
        if character == '*' && pattern.get(pattern_index + 1) == Some(&'*') {
            pattern_index += 1;
        }
        pattern_index += 1;
    }
    states[text.len()]
}

pub(super) fn is_generated_dir(name: &std::ffi::OsStr) -> bool {
    matches!(
        name.to_str(),
        Some(".git" | ".hg" | ".svn" | "target" | "node_modules" | "vendor")
    )
}
