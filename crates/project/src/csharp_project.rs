//! C# project facts that can be read without evaluating a build.
use crate::paths::clean_relative;
use quick_xml::{Reader, events::Event};
use std::path::{Path, PathBuf};

#[derive(Clone, Default)]
pub(crate) struct CSharpProject {
    pub(crate) path: PathBuf,
    pub(crate) root: PathBuf,
    pub(crate) references: Vec<PathBuf>,
    pub(crate) imports: Vec<String>,
    pub(crate) includes: Vec<String>,
    pub(crate) removes: Vec<String>,
    pub(crate) default_compile: bool,
    pub(crate) test: bool,
}
impl CSharpProject {
    pub(crate) fn parse(path: &Path, source: &str) -> Result<Self, String> {
        let root = path.parent().unwrap_or(Path::new(""));
        let mut result = Self {
            path: path.to_path_buf(),
            root: root.to_path_buf(),
            default_compile: true,
            ..Self::default()
        };
        let mut reader = Reader::from_str(source);
        reader.config_mut().trim_text(true);
        let mut element = String::new();
        let mut depth = 0;
        let mut saw_project = false;
        loop {
            let event = reader
                .read_event()
                .map_err(|error| format!("invalid C# project XML: {error}"))?;
            let opening = matches!(event, Event::Start(_));
            match event {
                Event::Start(node) | Event::Empty(node) => {
                    element = String::from_utf8_lossy(node.name().as_ref()).into_owned();
                    check_root(&mut saw_project, depth, &element)?;
                    if opening {
                        depth += 1;
                    }
                    result.read_attributes(&node, reader.decoder())?;
                }
                Event::Text(value) => {
                    let value = value.decode().map_err(|error| error.to_string())?;
                    result.property(&element, &value)?;
                }
                Event::DocType(_) => {
                    return Err("C# project document types are not supported".to_owned());
                }
                Event::Eof => break,
                Event::End(_) => {
                    depth -= 1;
                    element.clear();
                }
                _ => {}
            }
        }
        check_complete(saw_project, depth)?;
        Ok(result)
    }
    fn read_attributes(
        &mut self,
        node: &quick_xml::events::BytesStart<'_>,
        decoder: quick_xml::encoding::Decoder,
    ) -> Result<(), String> {
        let element = String::from_utf8_lossy(node.name().as_ref()).into_owned();
        for attribute in node.attributes() {
            let attribute = attribute.map_err(|error| error.to_string())?;
            let value = attribute
                .decode_and_unescape_value(decoder)
                .map_err(|error| error.to_string())?
                .into_owned();
            if attribute.key.as_ref() == b"Condition"
                || value.contains("$(")
                || value.contains("%(")
            {
                return Err("C# project has build-dependent configuration".to_owned());
            }
            self.attribute(&element, attribute.key.as_ref(), value)?;
        }
        Ok(())
    }
    fn attribute(&mut self, element: &str, key: &[u8], value: String) -> Result<(), String> {
        match (element, key) {
            ("ProjectReference", b"Include") => self.references.push(
                clean_relative(&self.root.join(value.replace('\\', "/")))
                    .ok_or("project reference leaves the repository")?,
            ),
            ("Using", b"Include") => self.imports.push(value),
            ("Compile", b"Include") => self
                .includes
                .extend(value.split(';').map(|value| value.replace('\\', "/"))),
            ("Compile", b"Remove" | b"Exclude") => self
                .removes
                .extend(value.split(';').map(|value| value.replace('\\', "/"))),
            ("Import", b"Project") => {
                return Err("C# project imports build configuration".to_owned());
            }
            _ => {}
        }
        Ok(())
    }
    fn property(&mut self, element: &str, value: &str) -> Result<(), String> {
        if value.contains("$(") || value.contains("%(") {
            return Err("C# project has build-dependent properties".to_owned());
        }
        match element {
            "EnableDefaultCompileItems" => self.default_compile = value != "false",
            "IsTestProject" => self.test = value == "true",
            _ => {}
        }
        Ok(())
    }
    pub(crate) fn contains(&self, path: &Path) -> bool {
        let matches = |pattern: &str| {
            let Some(pattern) = clean_relative(&self.root.join(pattern)) else {
                return false;
            };
            let pattern = pattern.to_string_lossy();
            let path = path.to_string_lossy();
            smackdebt_discovery::glob_matches(&pattern, &path)
                || pattern.contains("**/")
                    && smackdebt_discovery::glob_matches(&pattern.replace("**/", ""), &path)
        };
        (self.default_compile && path.starts_with(&self.root)
            || self.includes.iter().any(|pattern| matches(pattern)))
            && !self.removes.iter().any(|pattern| matches(pattern))
    }
}

fn check_root(saw_project: &mut bool, depth: usize, element: &str) -> Result<(), String> {
    if depth != 0 {
        return Ok(());
    }
    if *saw_project || element != "Project" {
        return Err("C# project needs one Project root".to_owned());
    }
    *saw_project = true;
    Ok(())
}

fn check_complete(saw_project: bool, depth: usize) -> Result<(), String> {
    if !saw_project || depth != 0 {
        return Err("C# project XML is incomplete".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn literal_compile_items_include_linked_source_and_honor_removes() {
        let project = CSharpProject::parse(Path::new("app/App.csproj"), r#"<Project><PropertyGroup><EnableDefaultCompileItems>false</EnableDefaultCompileItems></PropertyGroup><ItemGroup><Compile Include="src/**/*.cs;../Shared/*.cs" Remove="src/Skip.cs" /></ItemGroup></Project>"#).unwrap();
        assert!(project.contains(Path::new("app/src/Work.cs")));
        assert!(project.contains(Path::new("Shared/Work.cs")));
        assert!(!project.contains(Path::new("app/src/Skip.cs")));
        assert!(!project.contains(Path::new("app/Other.cs")));
    }
    #[test]
    fn malformed_and_evaluated_projects_are_disclosed() {
        for source in [
            "",
            "<Project>",
            "<Other />",
            "<Project /><Project />",
            r#"<Project><ItemGroup Condition="true" /></Project>"#,
        ] {
            assert!(
                CSharpProject::parse(Path::new("App.csproj"), source).is_err(),
                "{source}"
            );
        }
    }
}
