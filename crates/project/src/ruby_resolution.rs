//! Gem load paths read as data; gemspec code is never executed.
use smackdebt_analysis::FileId;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default)]
pub(crate) struct RubyLoadPaths {
    gems: BTreeMap<PathBuf, Option<Vec<PathBuf>>>,
}
impl RubyLoadPaths {
    pub(crate) fn insert_gemspec(&mut self, path: &Path, source: &str) {
        let root = path.parent().unwrap_or(Path::new(""));
        let paths = static_require_paths(source)
            .map(|paths| paths.into_iter().map(|path| root.join(path)).collect());
        self.gems
            .entry(root.to_path_buf())
            .and_modify(|existing| {
                *existing = match (existing.take(), paths.clone()) {
                    (Some(mut first), Some(second)) => {
                        first.extend(second);
                        first.sort();
                        first.dedup();
                        Some(first)
                    }
                    _ => None,
                };
            })
            .or_insert(paths);
    }
    pub(crate) fn resolve(
        &self,
        source: &Path,
        target: &str,
        index: &BTreeMap<PathBuf, FileId>,
    ) -> Result<Vec<FileId>, &'static str> {
        let gem = self
            .gems
            .iter()
            .filter(|(root, _)| source.starts_with(root))
            .max_by_key(|(root, _)| root.components().count());
        let Some((_, paths)) = gem else {
            return Ok(Vec::new());
        };
        let Some(paths) = paths else {
            return Err("Ruby gem require_paths are not static literals");
        };
        let target = if target.ends_with(".rb") {
            target.to_owned()
        } else {
            format!("{target}.rb")
        };
        if Path::new(&target).is_absolute() || target.split('/').any(|part| part == "..") {
            return Err("Ruby require target is not a safe load-path name");
        }
        let matches: BTreeSet<_> = paths
            .iter()
            .filter_map(|root| crate::paths::clean_relative(&root.join(&target)))
            .filter_map(|path| index.get(&path).copied())
            .collect();
        Ok(matches.into_iter().collect())
    }
}

fn static_require_paths(source: &str) -> Option<Vec<String>> {
    smackdebt_languages::ruby_gem_require_paths(source).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conditional_gem_load_paths_do_not_invent_decoy_edges() {
        let root = tempfile::tempdir().unwrap();
        for (path, source) in [
            (
                "sample.gemspec",
                "Gem::Specification.new do |spec|\n if false\n spec.require_paths = ['decoy']\n end\nend\n",
            ),
            ("main.rb", "require 'widget'\n"),
            ("lib/widget.rb", "def real; end\n"),
            ("decoy/widget.rb", "def decoy; end\n"),
        ] {
            let path = root.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, source).unwrap();
        }
        let result = crate::requests::CodebaseRequest::new(root.path())
            .analyze()
            .unwrap();
        assert!(result.report().dependency_edges().is_empty());
        assert!(!result.report().graph_evidence().is_complete());
    }
    #[test]
    fn dynamic_ruby_loading_leaves_the_graph_incomplete() {
        for source in ["require $target\n", "$LOAD_PATH.unshift($target)\n"] {
            let root = tempfile::tempdir().unwrap();
            std::fs::write(root.path().join("main.rb"), source).unwrap();
            let result = crate::codebase::analyze_codebase(&crate::requests::CodebaseRequest::new(
                root.path(),
            ))
            .unwrap();
            assert!(!result.report().graph_evidence().is_complete(), "{source}");
            assert_eq!(
                result.report().graph_evidence().unresolved_internal(),
                1,
                "{source}"
            );
        }
    }
    #[test]
    fn object_require_methods_do_not_claim_gem_dependencies() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("lib")).unwrap();
        std::fs::write(
            root.path().join("sample.gemspec"),
            "Gem::Specification.new do |s|\nend\n",
        )
        .unwrap();
        std::fs::write(
            root.path().join("main.rb"),
            "object.require('widget')\nobject.require_relative('lib/widget')\n",
        )
        .unwrap();
        std::fs::write(root.path().join("lib/widget.rb"), "def work; end\n").unwrap();
        let result =
            crate::codebase::analyze_codebase(&crate::requests::CodebaseRequest::new(root.path()))
                .unwrap();
        assert!(result.report().dependency_edges().is_empty());
        assert_eq!(result.report().dependency_coverage().total(), 0);
    }
    #[test]
    fn changed_load_paths_leave_ordinary_requires_unresolved() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("lib")).unwrap();
        std::fs::write(
            root.path().join("sample.gemspec"),
            "Gem::Specification.new do |s|\nend\n",
        )
        .unwrap();
        std::fs::write(
            root.path().join("main.rb"),
            "$LOAD_PATH.unshift(dynamic_path)\nrequire 'widget'\n",
        )
        .unwrap();
        std::fs::write(root.path().join("lib/widget.rb"), "def work; end\n").unwrap();
        let result =
            crate::codebase::analyze_codebase(&crate::requests::CodebaseRequest::new(root.path()))
                .unwrap();
        assert!(result.report().dependency_edges().is_empty());
        assert!(
            result
                .report()
                .resolution_diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.target() == "widget"
                    && diagnostic.kind() == smackdebt_analysis::ResolutionIssueKind::Unresolved)
        );
    }
    #[test]
    fn ruby_requires_resolve_gem_paths_and_keep_relative_paths() {
        let root = tempfile::tempdir().unwrap();
        for (path, source) in [
            (
                "sample.gemspec",
                "Gem::Specification.new do |s|\n s.require_paths = ['code']\nend\n",
            ),
            (
                "main.rb",
                "require 'widget'\nrequire_relative 'helper'\nrequire 'missing'\n",
            ),
            ("code/widget.rb", "def work; end\n"),
            ("helper.rb", "def help; end\n"),
            ("widget.rb", "def decoy; end\n"),
        ] {
            let file = root.path().join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, source).unwrap();
        }
        let result =
            crate::codebase::analyze_codebase(&crate::requests::CodebaseRequest::new(root.path()))
                .unwrap();
        let report = result.report();
        let mut edges: Vec<_> = report
            .dependency_edges()
            .iter()
            .map(|edge| {
                (
                    report.files()[edge.source().index()].path(),
                    report.files()[edge.target().index()].path(),
                )
            })
            .collect();
        edges.sort();
        assert_eq!(
            edges,
            [("main.rb", "code/widget.rb"), ("main.rb", "helper.rb")]
        );
    }
    #[test]
    fn gem_load_paths_use_literals_and_default_to_lib() {
        assert_eq!(
            static_require_paths("Gem::Specification.new do |s|\nend"),
            Some(vec!["lib".to_owned()])
        );
        assert_eq!(
            static_require_paths(
                "Gem::Specification.new do |s|\n s.require_paths = ['app', 'shared']\nend"
            ),
            Some(vec!["app".to_owned(), "shared".to_owned()])
        );
        assert_eq!(
            static_require_paths("s.require_paths = compute_paths()"),
            None
        );
        assert_eq!(static_require_paths("s.require_paths << 'extra'"), None);
    }
    #[test]
    fn ordinary_require_uses_only_the_gems_declared_load_paths() {
        let mut paths = RubyLoadPaths::default();
        paths.insert_gemspec(
            Path::new("gem/sample.gemspec"),
            "Gem::Specification.new do |s|\n s.require_paths = ['code']\nend",
        );
        let index = BTreeMap::from([
            (PathBuf::from("gem/code/widget.rb"), FileId::from_index(0)),
            (PathBuf::from("widget.rb"), FileId::from_index(1)),
        ]);
        assert_eq!(
            paths.resolve(Path::new("gem/main.rb"), "widget", &index),
            Ok(vec![FileId::from_index(0)])
        );
        assert_eq!(
            paths.resolve(Path::new("other/main.rb"), "widget", &index),
            Ok(vec![])
        );
    }
}
