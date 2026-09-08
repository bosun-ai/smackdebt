//! Composer autoload declarations, without executing an autoloader.
use crate::paths::clean_relative;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Clone, Default)]
pub(crate) struct PhpProject {
    pub(crate) root: PathBuf,
    pub(crate) name: Option<String>,
    pub(crate) requires: BTreeSet<String>,
    pub(crate) mappings: Vec<Autoload>,
    pub(crate) classmap: Vec<PathBuf>,
    pub(crate) files: Vec<PathBuf>,
    pub(crate) excludes: Vec<String>,
    bootstrap: PathBuf,
}
#[derive(Clone)]
pub(crate) struct Autoload {
    pub(crate) prefix: String,
    pub(crate) root: PathBuf,
    psr0: bool,
}
impl PhpProject {
    pub(crate) fn parse(path: &Path, source: &str) -> Result<Self, String> {
        let value: Value = serde_json::from_str(source)
            .map_err(|error| format!("invalid Composer JSON: {error}"))?;
        let root = path.parent().unwrap_or(Path::new(""));
        let mut result = Self {
            root: root.to_path_buf(),
            bootstrap: local_path(
                root,
                value["config"]["vendor-dir"].as_str().unwrap_or("vendor"),
            )?
            .join("autoload.php"),
            name: value["name"].as_str().map(str::to_owned),
            ..Self::default()
        };
        for key in ["require", "require-dev"] {
            if let Some(dependencies) = value[key].as_object() {
                result.requires.extend(dependencies.keys().cloned());
            }
        }
        for key in ["autoload", "autoload-dev"] {
            result.read_autoload(&value[key])?;
        }
        Ok(result)
    }
    fn read_autoload(&mut self, value: &Value) -> Result<(), String> {
        for standard in ["psr-4", "psr-0"] {
            self.read_mappings(standard, &value[standard])?;
        }
        for (field, paths) in [("classmap", &mut self.classmap), ("files", &mut self.files)] {
            for file in strings(&value[field])? {
                paths.push(local_path(&self.root, file)?);
            }
        }
        self.excludes.extend(
            strings(&value["exclude-from-classmap"])?
                .into_iter()
                .map(str::to_owned),
        );
        Ok(())
    }
    fn read_mappings(&mut self, standard: &str, value: &Value) -> Result<(), String> {
        let Some(mappings) = value.as_object() else {
            return Ok(());
        };
        for (prefix, directories) in mappings {
            for directory in strings(directories)? {
                self.mappings.push(Autoload {
                    prefix: prefix.replace('\\', ".").to_ascii_lowercase(),
                    root: local_path(&self.root, directory)?,
                    psr0: standard == "psr-0",
                });
            }
        }
        Ok(())
    }
    pub(crate) fn is_bootstrap(&self, source: &Path, candidate: &str) -> bool {
        source.starts_with(&self.root)
            && clean_relative(&source.parent().unwrap_or(Path::new("")).join(candidate))
                .is_some_and(|path| path == self.bootstrap)
    }
    pub(crate) fn claims(&self, name: &str) -> bool {
        self.mappings
            .iter()
            .any(|mapping| name.starts_with(&mapping.prefix))
    }
    pub(crate) fn permits(&self, name: &str, path: &Path) -> bool {
        if self.files.iter().any(|file| file == path) {
            return true;
        }
        if self.classmap.iter().any(|root| {
            path.starts_with(root)
                || smackdebt_discovery::glob_matches(
                    &root.to_string_lossy(),
                    &path.to_string_lossy(),
                )
                || smackdebt_discovery::glob_matches(
                    &format!("{}/**", root.display()),
                    &path.to_string_lossy(),
                )
        }) {
            let relative = path
                .strip_prefix(&self.root)
                .unwrap_or(path)
                .to_string_lossy();
            return !self.excludes.iter().any(|pattern| {
                smackdebt_discovery::glob_matches(pattern.trim_start_matches('/'), &relative)
            });
        }
        self.mappings.iter().any(|mapping| {
            let Some(tail) = name.strip_prefix(&mapping.prefix) else {
                return false;
            };
            let suffix = if mapping.psr0 {
                match name.rsplit_once('.') {
                    Some((namespace, class)) => format!(
                        "{}/{}",
                        namespace.replace('.', "/"),
                        class.replace('_', "/")
                    ),
                    None => name.replace('_', "/"),
                }
            } else {
                tail.replace('.', "/")
            };
            mapping
                .root
                .join(format!("{suffix}.php"))
                .to_string_lossy()
                .eq_ignore_ascii_case(&path.to_string_lossy())
        })
    }
}
fn strings(value: &Value) -> Result<Vec<&str>, String> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::String(value) => Ok(vec![value]),
        Value::Array(values) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| "Composer autoload paths must be strings".to_owned())
            })
            .collect(),
        _ => Err("Composer autoload paths must be strings or lists".to_owned()),
    }
}
fn local_path(root: &Path, value: &str) -> Result<PathBuf, String> {
    clean_relative(&root.join(value))
        .ok_or_else(|| format!("Composer path leaves the repository: {value}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn psr0_preserves_namespace_underscores_and_splits_class_underscores() {
        let project = PhpProject::parse(
            Path::new("composer.json"),
            r#"{"autoload":{"psr-0":{"Vendor_Name\\":"lib/"}}}"#,
        )
        .unwrap();
        assert!(project.permits(
            "vendor_name.task_worker",
            Path::new("lib/Vendor_Name/Task/Worker.php")
        ));
        assert!(!project.permits(
            "vendor_name.task_worker",
            Path::new("lib/Vendor/Name/Task/Worker.php")
        ));
    }

    #[test]
    fn classmap_patterns_respect_exclusions_and_files_are_entries() {
        let project = PhpProject::parse(Path::new("composer.json"),
            r#"{"autoload":{"classmap":["modules/*/src"],"exclude-from-classmap":["modules/*/src/Test/**"],"files":["bootstrap.php"]}}"#).unwrap();
        assert!(project.permits("worker", Path::new("modules/core/src/Worker.php")));
        assert!(!project.permits("test.worker", Path::new("modules/core/src/Test/Worker.php")));
        assert!(project.permits("boot", Path::new("bootstrap.php")));
    }
}
