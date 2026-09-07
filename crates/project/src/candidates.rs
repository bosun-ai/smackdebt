//! Path-candidate resolution: which repository file, if any, a reference's
//! spelled candidates name.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use smackdebt_analysis::FileId;
use smackdebt_discovery::is_source_path;

use crate::paths::clean_relative;
use crate::resolution_config::ResolutionRules;
use crate::rust_layout::{resolve_symbolic_candidates, rust_module_directory, rust_source_root};

pub(crate) fn resolve_candidates(
    source: &Path,
    candidates: &[String],
    index: &BTreeMap<PathBuf, FileId>,
    aliases: &ResolutionRules,
) -> Vec<FileId> {
    let parent = source.parent().unwrap_or(Path::new(""));
    let rust = source
        .extension()
        .is_some_and(|extension| extension == "rs");
    // A Rust file that is not `mod.rs`, `lib.rs`, or `main.rs` owns a directory
    // of its own name, so `mod child;` in `a.rs` names `a/child.rs` rather than
    // a sibling. That reading wins where it resolves; the sibling reading stays
    // for every other layout.
    let module_directory = rust.then(|| rust_module_directory(source)).flatten();
    let source_root = rust.then(|| rust_source_root(source)).flatten();
    let aliases = aliases.aliases_for(source);
    let resolve = |expanded: &[String], matches: &mut BTreeSet<FileId>| {
        for candidate in expanded {
            let path = Path::new(&candidate);
            let relative = candidate.starts_with("./") || candidate.starts_with("../");
            let module = module_directory
                .as_ref()
                .filter(|_| relative)
                .and_then(|directory| clean_relative(&directory.join(path)))
                .and_then(|clean| index.get(&clean));
            if let Some(file) = module {
                matches.insert(*file);
                continue;
            }
            let mut joined = if relative {
                vec![parent.join(path)]
            } else {
                vec![path.to_path_buf()]
            };
            if !relative && let Some(source_root) = &source_root {
                joined.push(source_root.join(path));
            }
            for joined in joined {
                if let Some(clean) = clean_relative(&joined)
                    && let Some(file) = index.get(&clean)
                {
                    matches.insert(*file);
                }
            }
        }
    };
    if !rust {
        let mut expanded = Vec::new();
        for candidate in candidates
            .iter()
            .filter(|candidate| !smackdebt_analysis::is_symbolic_candidate(candidate))
        {
            let candidate = strip_path_suffix(candidate);
            expanded.push(candidate.to_owned());
            expanded.extend(aliases.iter().filter_map(|alias| alias.expand(candidate)));
        }
        let mut matches = BTreeSet::new();
        resolve(&expanded, &mut matches);
        if matches.is_empty() {
            let source_spellings: Vec<_> = expanded
                .iter()
                .flat_map(|candidate| runtime_source_spellings(candidate))
                .collect();
            resolve(&source_spellings, &mut matches);
        }
        if matches.is_empty() {
            matches.extend(resolve_symbolic_candidates(source, candidates, index));
        }
        return matches.into_iter().collect();
    }
    for candidate in candidates
        .iter()
        .filter(|candidate| !smackdebt_analysis::is_symbolic_candidate(candidate))
    {
        let candidate = strip_path_suffix(candidate);
        let mut expanded = vec![candidate.to_owned()];
        expanded.extend(aliases.iter().filter_map(|alias| alias.expand(candidate)));
        let mut matches = BTreeSet::new();
        resolve(&expanded, &mut matches);
        if matches.is_empty() {
            let source_spellings: Vec<_> = expanded
                .iter()
                .flat_map(|candidate| runtime_source_spellings(candidate))
                .collect();
            resolve(&source_spellings, &mut matches);
        }
        if !matches.is_empty() {
            return matches.into_iter().collect();
        }
    }
    resolve_symbolic_candidates(source, candidates, index)
        .into_iter()
        .collect()
}
pub(crate) fn strip_path_suffix(candidate: &str) -> &str {
    candidate
        .find(['?', '#'])
        .map_or(candidate, |index| &candidate[..index])
}
/// Whether the paths a reference was looked for under spell a file the
/// source languages never analyze.
///
/// The question is asked of the candidate spellings and never of the written
/// target, because an extension only means what it looks like once a language
/// has read its target as a path — and a language says so by handing that path
/// back. Dotted module notation is read as a name instead, and arrives here
/// already turned into paths: Python offers `../core.py` for `..core`, Java
/// offers `app/Local.java` for `app.Local`. Judged on the written target those
/// two would carry the extensions `core` and `Local`, and every broken module
/// import in the repository would vanish under an asset row — the inverse of
/// the false hole this classification exists to remove.
///
/// Nothing is read from the filesystem. Discovery walks once and inventories
/// source only, so an extension it does not claim could never have been
/// indexed whether the file is on disk or not. A spelling with no extension
/// claims nothing, because a bare `./config` is an unwritten source path far
/// more often than it is an asset.
pub(crate) fn candidates_name_an_asset(candidates: &[String]) -> bool {
    candidates
        .iter()
        .filter(|candidate| !smackdebt_analysis::is_symbolic_candidate(candidate))
        .any(|candidate| {
            let path = Path::new(strip_path_suffix(candidate));
            path.extension().is_some() && !is_source_path(path)
        })
}
pub(crate) fn runtime_source_spellings(candidate: &str) -> Vec<String> {
    let path = Path::new(candidate);
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return Vec::new();
    };
    let replacements: &[&str] = match extension {
        "js" => &["ts", "tsx"],
        "jsx" => &["tsx"],
        "mjs" => &["mts", "ts"],
        "cjs" => &["cts", "ts"],
        _ => return Vec::new(),
    };
    replacements
        .iter()
        .map(|extension| {
            path.with_extension(extension)
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}
