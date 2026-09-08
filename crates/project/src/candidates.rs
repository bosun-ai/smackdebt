//! Path-candidate resolution: which repository file, if any, a reference's
//! spelled candidates name.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use smackdebt_analysis::FileId;
use smackdebt_discovery::is_source_path;

use crate::paths::clean_relative;
use crate::resolution_rules::ResolutionRules;
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::codebase::analyze_codebase;

    #[test]
    fn an_import_of_a_non_source_file_is_an_asset_rather_than_a_hole() {
        let root = tempfile::tempdir().unwrap();
        for (path, source) in [
            ("app/package.json", "{\"name\":\"app\"}\n"),
            // Three assets an importer reads for their bytes: one carries a
            // query suffix, one a fragment, one neither.
            (
                "app/main.ts",
                "import raw from './config.yaml?raw';\nimport icon from './logo.svg#glyph';\nimport theme from './theme.css';\nimport helper from './helper';\nexport default [raw, icon, theme, helper];\n",
            ),
            ("app/helper.ts", "export default 1;\n"),
            ("app/config.yaml", "name: fixture\n"),
            ("app/logo.svg", "<svg />\n"),
            ("app/theme.css", ".a { color: red; }\n"),
            ("core/package.json", "{\"name\":\"core\"}\n"),
            // The scope guard: a source extension that matches nothing, and a
            // target that names no extension at all, are still holes.
            (
                "core/main.ts",
                "import gone from './gone.ts';\nimport absent from './absent';\nexport default [gone, absent];\n",
            ),
        ] {
            let file = root.path().join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, source).unwrap();
        }

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let rows: Vec<_> = report
            .resolution_diagnostics()
            .iter()
            .map(|value| {
                format!(
                    "{} · {:?} · {}",
                    value.target(),
                    value.kind(),
                    value.reason()
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                "./config.yaml?raw · Asset · target is an asset",
                "./logo.svg#glyph · Asset · target is an asset",
                "./theme.css · Asset · target is an asset",
                "./absent · Unresolved · no repository file matches",
                "./gone.ts · Unresolved · no repository file matches",
            ],
            "an asset keeps its disclosure under its own reason"
        );

        let coverage = report.dependency_coverage();
        assert_eq!(
            coverage.unresolved_internal_uses(),
            2,
            "only the two holes are unresolved"
        );
        assert_eq!(
            coverage.context_relations(),
            3,
            "the assets stay counted, outside the verdict graph"
        );
        assert_eq!(coverage.resolved_internal_uses(), 1);

        let evidence = report.graph_evidence();
        assert_eq!(evidence.unresolved_internal(), 2);
        let core = report
            .files()
            .iter()
            .find(|file| file.path() == "core/main.ts")
            .and_then(smackdebt_analysis::FileRecord::package)
            .expect("the hole belongs to a package");
        assert_eq!(
            evidence.incomplete_packages(),
            [core],
            "importing an asset leaves its package complete"
        );
    }

    #[test]
    fn an_asset_is_read_from_the_paths_a_reference_was_looked_for_under() {
        // The spellings each language hands the resolver. A path language
        // offers the written name plus the extensions it knows; a language
        // that reads dotted module notation offers only the paths it derived
        // from that name, and the written form never appears at all.
        let path = |target: &str| {
            let mut values = vec![target.to_owned()];
            for extension in [".js", ".ts"] {
                values.push(format!("{target}{extension}"));
                values.push(format!("{target}/index{extension}"));
            }
            values
        };
        let module = |values: &[&str]| {
            values
                .iter()
                .map(|value| (*value).to_owned())
                .collect::<Vec<_>>()
        };

        for (target, candidates, asset, reading) in [
            (
                "./x.yaml?raw",
                path("./x.yaml?raw"),
                true,
                "a query suffix is stripped before the extension is read",
            ),
            (
                "./x.md",
                path("./x.md"),
                true,
                "a plain unclaimed extension needs no suffix",
            ),
            (
                "./capabilities",
                path("./capabilities"),
                false,
                "a spelling that names no extension claims nothing",
            ),
            (
                "./missing.ts",
                path("./missing.ts"),
                false,
                "a source extension that matched nothing is still a hole",
            ),
            // `from ..core import thing`. Read as a path the target carries
            // the extension `core`, so only the candidates show it is a module
            // name and that the file it misses is a real hole.
            (
                "..core",
                module(&["../core.py", "../core/__init__.py"]),
                false,
                "dotted module notation never reaches here as a path",
            ),
            (
                ".missing.thing",
                module(&["./missing/thing.py", "./missing/thing/__init__.py"]),
                false,
                "a dotted module chain is not a path either",
            ),
            // Pinned rather than preferred: an absent `./webpack.config.js`
            // imported as `./webpack.config` reads as an asset, because
            // `config` is an extension no language claims. The candidates
            // cannot settle it — the literal spelling is one of them. It costs
            // a reader nothing: the row keeps its target and its line in JSON,
            // and is only held out of a count that would otherwise claim a
            // broken graph on a guess.
            (
                "./webpack.config",
                path("./webpack.config"),
                true,
                "an unclaimed extension on a written path reads as an asset",
            ),
        ] {
            assert_eq!(
                candidates_name_an_asset(&candidates),
                asset,
                "{target}: {reading}"
            );
        }
    }

    #[test]
    fn a_python_relative_import_of_a_missing_module_is_still_a_hole() {
        let root = tempfile::tempdir().unwrap();
        for (path, source) in [
            ("pyproject.toml", "[project]\nname='service'\n"),
            ("src/__init__.py", "\n"),
            ("src/api/__init__.py", "\n"),
            // `..core` names the module `src/core`, which nothing declares.
            // Read as a path it would carry the extension `core` and vanish
            // under an asset row, taking the package's incompleteness with it.
            (
                "src/api/handler.py",
                "from ..core import thing\n\n\ndef handle():\n    return thing\n",
            ),
        ] {
            let file = root.path().join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, source).unwrap();
        }

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let rows: Vec<_> = report
            .resolution_diagnostics()
            .iter()
            .map(|value| {
                format!(
                    "{} · {:?} · {}",
                    value.target(),
                    value.kind(),
                    value.reason()
                )
            })
            .collect();
        assert_eq!(rows, ["..core · Unresolved · no repository file matches"]);
        assert_eq!(report.dependency_coverage().unresolved_internal_uses(), 1);

        let evidence = report.graph_evidence();
        assert!(
            !evidence.is_complete(),
            "a missing Python module leaves the graph incomplete"
        );
        assert_eq!(evidence.unresolved_internal(), 1);
        assert_eq!(evidence.incomplete_packages().len(), 1);
    }

    use crate::requests::CodebaseRequest;
    use smackdebt_analysis::ResolutionDiagnostic;
    use smackdebt_analysis::ResolutionIssueKind;
    use std::fs;

    #[test]
    fn symbolic_candidates_are_the_last_resort_of_one_reference() {
        let root = tempfile::tempdir().unwrap();
        for (path, source) in [
            (
                "Cargo.toml",
                "[package]\nname='symbolic'\nversion='0.1.0'\n",
            ),
            (
                "src/lib.rs",
                "mod deep;\nmod helper;\nmod registries;\nmod report;\npub struct Item;\n",
            ),
            (
                "src/report.rs",
                "use crate::Item;\nuse crate::registries::traits::ToolExt;\nmod tests {\n    use super::*;\n}\n",
            ),
            ("src/deep/mod.rs", "mod inner;\n"),
            (
                "src/deep/inner.rs",
                "mod tests {\n    use super::helper::work;\n}\n",
            ),
            ("src/helper.rs", "pub fn work() -> i32 { 1 }\n"),
            ("src/registries.rs", "pub mod traits;\n"),
            ("src/registries/traits.rs", "pub struct ToolExt;\n"),
            ("standalone/loose.rs", "use crate::Missing;\n"),
        ] {
            let file = root.path().join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, source).unwrap();
        }

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        let edges: Vec<_> = report
            .dependency_edges()
            .iter()
            .filter(|edge| edge.relation() == smackdebt_analysis::StaticRelationKind::Uses)
            .map(|edge| {
                (
                    report.files()[edge.source().index()].path(),
                    report.files()[edge.target().index()].path(),
                )
            })
            .collect();
        assert!(
            edges.contains(&("src/report.rs", "src/lib.rs")),
            "a crate-root item resolves to the crate root: {edges:?}"
        );
        assert!(
            edges.contains(&("src/deep/inner.rs", "src/helper.rs")),
            "a matching module path wins over the declaring file: {edges:?}"
        );
        assert!(
            edges.contains(&("src/report.rs", "src/registries/traits.rs")),
            "the nearest matching module wins over its parent: {edges:?}"
        );
        assert!(
            !edges
                .iter()
                .any(|(source, target)| source == target || *target == "src/deep/inner.rs"),
            "a reference to the declaring file creates no edge: {edges:?}"
        );
        let unresolved: Vec<_> = report
            .resolution_diagnostics()
            .iter()
            .filter(|value| value.kind() == ResolutionIssueKind::Unresolved)
            .map(ResolutionDiagnostic::target)
            .collect();
        assert_eq!(
            unresolved,
            ["crate::Missing"],
            "a symbolic candidate that matches nothing stays unresolved"
        );
    }
    #[test]
    fn java_source_root_import_resolves_to_a_repository_file() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("src/main/java/app")).unwrap();
        fs::create_dir_all(root.path().join("src/main/java/usecase")).unwrap();
        fs::write(root.path().join("pom.xml"), "<project />").unwrap();
        fs::write(
            root.path().join("src/main/java/app/Local.java"),
            "package app; public class Local {}\n",
        )
        .unwrap();
        fs::write(
            root.path().join("src/main/java/usecase/Main.java"),
            "package usecase; import app.Local; public class Main {}\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        assert_eq!(result.report().dependency_edges().len(), 1);
        assert_eq!(result.report().dependency_coverage().internal(), 1);
    }
}
