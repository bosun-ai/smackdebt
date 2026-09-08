//! Go module and workspace paths, read as data.
use crate::paths::clean_relative;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Default)]
pub(crate) struct GoProject {
    pub(crate) root: PathBuf,
    pub(crate) module: String,
    pub(crate) replacements: BTreeMap<String, PathBuf>,
    pub(crate) uses: Vec<PathBuf>,
    pub(crate) workspace: bool,
}
impl GoProject {
    pub(crate) fn parse(path: &Path, source: &str) -> Result<Self, String> {
        let root = path.parent().unwrap_or(Path::new(""));
        let workspace = path.file_name().is_some_and(|name| name == "go.work");
        let mut project = Self {
            root: root.to_path_buf(),
            workspace,
            ..Self::default()
        };
        let mut block = String::new();
        for line in source.lines() {
            let words = tokens(line)?;
            if words.is_empty() {
                continue;
            }
            if words == [")"] {
                block.clear();
                continue;
            }
            let (directive, values) = if block.is_empty() {
                (words[0].as_str(), &words[1..])
            } else {
                (block.as_str(), words.as_slice())
            };
            if values == ["("] {
                block = directive.to_owned();
                continue;
            }
            project.read_directive(directive, values)?;
        }
        if !block.is_empty() {
            return Err("unterminated Go configuration block".to_owned());
        }
        if !workspace && project.module.is_empty() {
            return Err("go.mod has no module path".to_owned());
        }
        Ok(project)
    }
    fn read_directive(&mut self, directive: &str, values: &[String]) -> Result<(), String> {
        match directive {
            "module" if values.len() == 1 => self.module.clone_from(&values[0]),
            "use" if values.len() == 1 => self.uses.push(local_path(&self.root, &values[0])?),
            "replace" => self.read_replacement(values)?,
            "module" | "use" => {
                return Err("malformed Go module or workspace directive".to_owned());
            }
            _ => {}
        }
        Ok(())
    }
    fn read_replacement(&mut self, values: &[String]) -> Result<(), String> {
        let arrow = values
            .iter()
            .position(|value| value == "=>")
            .ok_or("malformed Go replacement")?;
        if arrow == 0 {
            return Err("missing Go replacement module".to_owned());
        }
        let target = values
            .get(arrow + 1)
            .ok_or("missing Go replacement target")?;
        if target.starts_with('.') || target.starts_with('/') {
            self.replacements
                .insert(values[0].clone(), local_path(&self.root, target)?);
        }
        Ok(())
    }
    pub(crate) fn directory(&self, target: &str) -> Option<PathBuf> {
        package_directory(&self.module, &self.root, target)
    }
}
pub(crate) fn package_directory(prefix: &str, root: &Path, target: &str) -> Option<PathBuf> {
    if target == prefix {
        return Some(root.to_path_buf());
    }
    let rest = target.strip_prefix(prefix)?.strip_prefix('/')?;
    clean_relative(&root.join(rest))
}
fn local_path(root: &Path, value: &str) -> Result<PathBuf, String> {
    clean_relative(&root.join(value))
        .ok_or_else(|| format!("Go configuration path leaves the repository: {value}"))
}
fn tokens(line: &str) -> Result<Vec<String>, String> {
    let mut values = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(character) = chars.next() {
        if character.is_whitespace() {
            continue;
        }
        if character == '/' && chars.peek() == Some(&'/') {
            break;
        }
        if matches!(character, '(' | ')') {
            values.push(character.to_string());
            continue;
        }
        if matches!(character, '"' | '`') {
            values.push(quoted_token(&mut chars, character)?);
        } else {
            let mut value = character.to_string();
            while chars
                .peek()
                .is_some_and(|next| !next.is_whitespace() && !matches!(next, '(' | ')'))
            {
                value.push(chars.next().expect("peeked token character"));
            }
            values.push(value);
        }
    }
    Ok(values)
}

fn quoted_token(chars: &mut impl Iterator<Item = char>, quote: char) -> Result<String, String> {
    let mut value = String::new();
    for next in chars {
        if next == quote {
            return Ok(value);
        }
        if next == '\\' {
            return Err("escaped Go configuration paths are not supported".to_owned());
        }
        value.push(next);
    }
    Err("unterminated Go configuration string".to_owned())
}
