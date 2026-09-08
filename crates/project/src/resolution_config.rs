//! Import-resolution configuration: the alias rules tsconfig and jsconfig
//! files declare, read from the worktree or from base Git objects.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use smackdebt_discovery::Inventory;

use crate::paths::clean_relative;

#[derive(Clone)]
pub(crate) struct ResolutionAlias {
    prefix: String,
    suffix: String,
    replacement: String,
}
#[derive(Clone, Default)]
pub(crate) struct ResolutionRules {
    pub(crate) packages: Vec<PackageResolution>,
}
#[derive(Clone)]
pub(crate) struct PackageResolution {
    pub(crate) root: PathBuf,
    pub(crate) aliases: Vec<ResolutionAlias>,
    pub(crate) issue: Option<String>,
}
impl ResolutionRules {
    pub(crate) fn aliases_for(&self, source: &Path) -> &[ResolutionAlias] {
        self.packages
            .iter()
            .filter(|package| source.starts_with(&package.root))
            .max_by_key(|package| package.root.components().count())
            .map_or(&[], |package| package.aliases.as_slice())
    }
}
impl ResolutionAlias {
    pub(crate) fn expand(&self, candidate: &str) -> Option<String> {
        let middle = candidate
            .strip_prefix(&self.prefix)?
            .strip_suffix(&self.suffix)?;
        Some(self.replacement.replace('*', middle))
    }
}
pub(crate) fn load_resolution_aliases(root: &Path, inventory: &Inventory) -> ResolutionRules {
    let packages = inventory
        .packages()
        .iter()
        .map(|package| {
            let package_root = package.root().as_path().to_path_buf();
            let Some(config) = package.resolution_config() else {
                return PackageResolution {
                    root: package_root,
                    aliases: Vec::new(),
                    issue: None,
                };
            };
            let config = config.as_path().to_path_buf();
            let mut read =
                |path: &Path| fs::read(root.join(path)).map_err(|error| error.to_string());
            let (aliases, issue) = match load_resolution_chain(&config, &mut read) {
                Ok(aliases) => (aliases, None),
                Err(issue) => (Vec::new(), Some(issue)),
            };
            PackageResolution {
                root: package_root,
                aliases,
                issue,
            }
        })
        .collect();
    ResolutionRules { packages }
}
pub(crate) fn load_base_resolution_aliases(
    reader: &mut smackdebt_git::ObjectReader,
    base: &str,
    package_roots: &[PathBuf],
    config_candidates: &[PathBuf],
) -> ResolutionRules {
    let mut sources = BTreeMap::new();
    for path in config_candidates {
        sources.insert(
            path.clone(),
            reader
                .read_path(base, path)
                .map_err(|error| error.to_string()),
        );
    }
    let mut packages = Vec::with_capacity(package_roots.len());
    for root in package_roots {
        let config = config_candidates
            .iter()
            .filter(|path| {
                path.parent()
                    .is_some_and(|directory| root.starts_with(directory))
            })
            .filter(|path| sources.get(*path).is_some_and(Result::is_ok))
            .min_by_key(|path| {
                root.components()
                    .count()
                    .saturating_sub(path.parent().map_or(0, |value| value.components().count()))
            })
            .cloned();
        let Some(config) = config else {
            packages.push(PackageResolution {
                root: root.clone(),
                aliases: Vec::new(),
                issue: None,
            });
            continue;
        };
        let mut read = |path: &Path| match sources.get(path) {
            Some(source) => source.clone(),
            None => {
                let source = reader
                    .read_path(base, path)
                    .map_err(|error| error.to_string());
                sources.insert(path.to_path_buf(), source.clone());
                source
            }
        };
        let (aliases, issue) = match load_resolution_chain(&config, &mut read) {
            Ok(aliases) => (aliases, None),
            Err(issue) => (Vec::new(), Some(issue)),
        };
        packages.push(PackageResolution {
            root: root.clone(),
            aliases,
            issue,
        });
    }
    ResolutionRules { packages }
}
pub(crate) fn load_resolution_chain(
    config: &Path,
    read: &mut impl FnMut(&Path) -> Result<Vec<u8>, String>,
) -> Result<Vec<ResolutionAlias>, String> {
    pub(crate) fn visit(
        config: &Path,
        read: &mut impl FnMut(&Path) -> Result<Vec<u8>, String>,
        stack: &mut BTreeSet<PathBuf>,
        depth: usize,
    ) -> Result<Vec<ResolutionAlias>, String> {
        if depth >= 16 {
            return Err(format!(
                "configuration inheritance is too deep at {}",
                config.display()
            ));
        }
        let config = clean_relative(config).ok_or_else(|| {
            format!(
                "configuration path leaves the repository: {}",
                config.display()
            )
        })?;
        if !stack.insert(config.clone()) {
            return Err(format!(
                "configuration inheritance cycles at {}",
                config.display()
            ));
        }
        let source =
            read(&config).map_err(|error| format!("cannot read {}: {error}", config.display()))?;
        let text = std::str::from_utf8(&source)
            .map_err(|_| format!("{} is not UTF-8", config.display()))?;
        let value = json5::from_str::<serde_json::Value>(text)
            .map_err(|error| format!("cannot parse {}: {error}", config.display()))?;
        let parent = config.parent().unwrap_or(Path::new(""));
        let mut aliases = Vec::new();
        for inherited in inherited_configs(&value, parent)? {
            aliases.extend(visit(&inherited, read, stack, depth + 1)?);
        }
        let local = parse_resolution_aliases(&value, parent)?;
        for alias in &local {
            aliases.retain(|inherited| {
                inherited.prefix != alias.prefix || inherited.suffix != alias.suffix
            });
        }
        aliases.extend(local);
        aliases.sort_by(|left, right| {
            (&left.prefix, &left.suffix, &left.replacement).cmp(&(
                &right.prefix,
                &right.suffix,
                &right.replacement,
            ))
        });
        stack.remove(&config);
        Ok(aliases)
    }

    visit(config, read, &mut BTreeSet::new(), 0)
}
pub(crate) fn inherited_configs(
    value: &serde_json::Value,
    parent: &Path,
) -> Result<Vec<PathBuf>, String> {
    let inherited: Vec<&str> = match &value["extends"] {
        serde_json::Value::Null => Vec::new(),
        serde_json::Value::String(value) => vec![value],
        serde_json::Value::Array(values) => {
            values.iter().filter_map(|value| value.as_str()).collect()
        }
        _ => return Err("configuration extends must be a path or path list".to_owned()),
    };
    inherited
        .into_iter()
        .map(|value| {
            if !value.starts_with('.') {
                return Err(format!(
                    "configuration inheritance is not repository-relative: {value}"
                ));
            }
            let mut path = parent.join(value);
            if path.extension().is_none() {
                path.set_extension("json");
            }
            clean_relative(&path)
                .ok_or_else(|| format!("configuration inheritance leaves the repository: {value}"))
        })
        .collect()
}
pub(crate) fn parse_resolution_aliases(
    value: &serde_json::Value,
    config_parent: &Path,
) -> Result<Vec<ResolutionAlias>, String> {
    let base = value["compilerOptions"]["baseUrl"]
        .as_str()
        .unwrap_or("")
        .trim_matches('/');
    let base = if base == "." { "" } else { base };
    let Some(paths) = value["compilerOptions"]["paths"].as_object() else {
        return Ok(Vec::new());
    };
    let mut aliases = Vec::new();
    for (pattern, replacements) in paths {
        let (prefix, suffix) = pattern
            .split_once('*')
            .map_or((pattern.as_str(), ""), |parts| parts);
        for replacement in replacements.as_array().into_iter().flatten() {
            let Some(replacement) = replacement.as_str() else {
                continue;
            };
            let replacement = if base.is_empty() {
                config_parent.join(replacement)
            } else {
                config_parent.join(base).join(replacement)
            };
            let replacement = clean_relative(&replacement).ok_or_else(|| {
                format!(
                    "alias target leaves the repository: {}",
                    replacement.display()
                )
            })?;
            aliases.push(ResolutionAlias {
                prefix: prefix.to_owned(),
                suffix: suffix.to_owned(),
                replacement: replacement.to_string_lossy().replace('\\', "/"),
            });
        }
    }
    aliases.sort_by(|left, right| {
        (&left.prefix, &left.suffix, &left.replacement).cmp(&(
            &right.prefix,
            &right.suffix,
            &right.replacement,
        ))
    });
    Ok(aliases)
}

#[cfg(test)]
mod tests {

    use crate::codebase::analyze_codebase;
    use crate::diff::analyze_diff;
    use crate::requests::{CodebaseRequest, DiffRequest};
    use crate::test_support::git;
    use std::fs;

    #[test]
    fn project_configuration_aliases_resolve_as_data_without_execution() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("src/core")).unwrap();
        fs::write(root.path().join("package.json"), "{}").unwrap();
        fs::write(
            root.path().join("tsconfig.json"),
            r#"{"compilerOptions":{"baseUrl":".","paths":{"@/*":["src/*"]}}}"#,
        )
        .unwrap();
        fs::write(
            root.path().join("src/main.ts"),
            "import core from '@/core/index';\nfunction main() { return core(); }\n",
        )
        .unwrap();
        fs::write(
            root.path().join("src/core/index.ts"),
            "export default function core() {}\n",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        assert_eq!(result.report().dependency_edges().len(), 1);
        assert_eq!(result.report().dependency_coverage().internal(), 1);
    }
    #[test]
    fn package_aliases_jsonc_and_runtime_extensions_resolve_within_their_package() {
        let root = tempfile::tempdir().unwrap();
        for package in ["app", "core"] {
            fs::create_dir_all(root.path().join(package).join("src")).unwrap();
            fs::write(root.path().join(package).join("package.json"), "{}").unwrap();
            fs::write(
                root.path().join(package).join("src/main.ts"),
                "import value from '@/value.js?raw';\nexport default value;\n",
            )
            .unwrap();
            fs::write(
                root.path().join(package).join("src/value.ts"),
                "export default 1;\n",
            )
            .unwrap();
        }
        fs::write(
            root.path().join("app/tsconfig.json"),
            "{ extends: './tsconfig.base.json', }",
        )
        .unwrap();
        fs::write(
            root.path().join("app/tsconfig.base.json"),
            "{ compilerOptions: { paths: { '@/*': ['./src/*'], }, }, }",
        )
        .unwrap();
        fs::write(
            root.path().join("core/tsconfig.json"),
            "{ // package-local alias\n compilerOptions: { paths: { '@/*': ['./src/*'], }, }, }",
        )
        .unwrap();

        let result = analyze_codebase(&CodebaseRequest::new(root.path())).unwrap();
        let report = result.report();
        assert_eq!(report.dependency_edges().len(), 2);
        for edge in report.dependency_edges() {
            let source = &report.files()[edge.source().index()];
            let target = &report.files()[edge.target().index()];
            assert_eq!(source.package(), target.package());
            assert!(target.path().ends_with("src/value.ts"));
        }
    }
    #[test]
    fn diff_uses_each_sides_alias_configuration_without_extra_git_processes() {
        let root = tempfile::tempdir().unwrap();
        let repository_path = root.path();
        git(repository_path, ["init", "-q"]);
        git(
            repository_path,
            ["config", "user.email", "test@example.invalid"],
        );
        git(repository_path, ["config", "user.name", "Smackdebt Test"]);
        for package in ["app", "core"] {
            fs::create_dir_all(repository_path.join(package)).unwrap();
            fs::write(repository_path.join(package).join("package.json"), "{}").unwrap();
        }
        fs::write(
            repository_path.join("app/a.ts"),
            "import core from '@core/value';\nfunction app() {}\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("core/value.ts"),
            "export default function core() {}\n",
        )
        .unwrap();
        fs::write(
            repository_path.join("tsconfig.json"),
            r#"{"compilerOptions":{"paths":{"@core/*":["core/*"]}}}"#,
        )
        .unwrap();
        git(repository_path, ["add", "."]);
        git(repository_path, ["commit", "-qm", "initial"]);
        fs::write(
            repository_path.join("tsconfig.json"),
            r#"{"compilerOptions":{"paths":{"@core/*":["app/*"]}}}"#,
        )
        .unwrap();
        let result =
            analyze_diff(&DiffRequest::new(repository_path).with_reference("HEAD")).unwrap();
        assert!(result.report().architecture_comparisons().iter().any(
            |value| value.kind() == smackdebt_analysis::ArchitectureComparisonKind::EdgeRemoved
        ));
        // Discovery, the merge base, the changed paths, the batched object
        // reader, the streamed history, and the one revision list that says
        // which of those commits the change under review made.
        assert_eq!(result.stats().git_processes, 6);
    }
}
