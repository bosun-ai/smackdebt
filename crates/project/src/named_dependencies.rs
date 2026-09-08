//! Repository indexes for package members and declared source names.
use crate::dependencies::SourceDependencies;
use crate::project_metadata::ProjectMetadata;
use smackdebt_analysis::{
    DependencySyntax, DependencySyntaxState, FileId, Language, NameImport, NameReference,
    SymbolKind,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub(crate) enum NamedResolution {
    Files(Vec<FileId>),
    External,
    Unresolved(String),
    Ambiguous,
}
struct Declaration<'a> {
    source: &'a SourceDependencies,
    shared: bool,
}
pub(crate) struct NamedDependencies<'a> {
    declarations: BTreeMap<(Language, SymbolKind, &'a str), Vec<Declaration<'a>>>,
    directories: BTreeMap<&'a Path, Vec<FileId>>,
    global_imports: BTreeMap<PathBuf, Vec<&'a NameImport>>,
    namespaces: BTreeSet<(Language, &'a str)>,
    roots: &'a [PathBuf],
    metadata: &'a ProjectMetadata,
}
impl<'a> NamedDependencies<'a> {
    pub(crate) fn new(
        sources: &'a [SourceDependencies],
        roots: &'a [PathBuf],
        metadata: &'a ProjectMetadata,
    ) -> Self {
        let mut result = Self {
            declarations: BTreeMap::new(),
            directories: BTreeMap::new(),
            global_imports: BTreeMap::new(),
            namespaces: BTreeSet::new(),
            roots,
            metadata,
        };
        for source in sources {
            if source.language == Language::Go
                && !source
                    .path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().ends_with("_test.go"))
            {
                result
                    .directories
                    .entry(source.path.parent().unwrap_or(Path::new("")))
                    .or_default()
                    .push(source.file);
            }
            for declaration in source.names.declarations() {
                let name = declaration.name();
                result
                    .declarations
                    .entry((source.language, declaration.kind(), name))
                    .or_default()
                    .push(Declaration {
                        source,
                        shared: declaration.is_shared(),
                    });
                if let Some((namespace, _)) = name.rsplit_once('.') {
                    result.namespaces.insert((source.language, namespace));
                }
            }
            for import in source
                .names
                .imports()
                .iter()
                .filter(|import| import.is_global())
            {
                result
                    .global_imports
                    .entry(package_root(&source.path, roots).to_path_buf())
                    .or_default()
                    .push(import);
            }
        }
        result
    }
    pub(crate) fn resolve(
        &self,
        source: &SourceDependencies,
        reference: &DependencySyntax,
    ) -> NamedResolution {
        if let Some(issue) = self.metadata.issue_for(source.language, &source.path) {
            return NamedResolution::Unresolved(issue.to_owned());
        }
        match reference.state() {
            DependencySyntaxState::PackageMembers => match self
                .metadata
                .package_directory(&source.path, reference.target())
            {
                Ok(Some(directory)) => self.directories.get(directory.as_path()).map_or_else(
                    || {
                        NamedResolution::Unresolved(
                            "no source files match the imported package".to_owned(),
                        )
                    },
                    |files| NamedResolution::Files(files.clone()),
                ),
                Ok(None) => NamedResolution::External,
                Err(reason) => NamedResolution::Unresolved(reason),
            },
            DependencySyntaxState::Name(name) => self.resolve_name(source, name),
            _ => unreachable!("only named references enter this resolver"),
        }
    }
    fn resolve_name(
        &self,
        source: &SourceDependencies,
        reference: &NameReference,
    ) -> NamedResolution {
        let name = reference.name();
        if let Some(absolute) = name.strip_prefix('.') {
            return self.lookup(source, reference.kind(), &[absolute.to_owned()]);
        }
        let globals = self
            .global_imports
            .get(package_root(&source.path, self.roots))
            .map_or(&[][..], Vec::as_slice);
        let imports: Vec<_> = source
            .names
            .imports()
            .iter()
            .filter(|import| !import.is_global() && import.contains(reference.position()))
            .chain(globals.iter().copied())
            .filter(|import| import.kind() == reference.kind())
            .collect();
        let aliases: Vec<_> = imports
            .iter()
            .filter_map(|import| alias_target(import, name))
            .collect();
        if !aliases.is_empty() {
            return self.lookup(source, reference.kind(), &aliases);
        }
        self.resolve_scoped_name(source, reference, &imports)
    }
    fn resolve_scoped_name(
        &self,
        source: &SourceDependencies,
        reference: &NameReference,
        imports: &[&NameImport],
    ) -> NamedResolution {
        let name = reference.name();
        for namespace in std::iter::successors(Some(reference.namespace()), |namespace| {
            namespace.rsplit_once('.').map(|(parent, _)| parent)
        })
        .chain(std::iter::once(""))
        {
            let candidate = if namespace.is_empty() {
                name.to_owned()
            } else {
                format!("{namespace}.{name}")
            };
            let outcome = self.lookup(source, reference.kind(), &[candidate]);
            if matches!(
                outcome,
                NamedResolution::Files(_) | NamedResolution::Ambiguous
            ) {
                return outcome;
            }
        }
        let mut candidates: Vec<_> = imports
            .iter()
            .filter(|import| import.alias().is_none())
            .map(|import| format!("{}.{name}", import.target()))
            .collect();
        candidates.extend(
            self.metadata
                .imports(&source.path)
                .map(|namespace| format!("{namespace}.{name}")),
        );
        if !candidates.is_empty() {
            return self.lookup(source, reference.kind(), &candidates);
        }
        if name.contains('.') {
            return self.lookup(source, reference.kind(), &[name.to_owned()]);
        }
        if reference.kind() == SymbolKind::Function {
            return NamedResolution::External;
        }
        self.lookup(
            source,
            reference.kind(),
            &[if reference.namespace().is_empty() {
                name.to_owned()
            } else {
                format!("{}.{name}", reference.namespace())
            }],
        )
    }
    fn lookup(
        &self,
        source: &SourceDependencies,
        kind: SymbolKind,
        candidates: &[String],
    ) -> NamedResolution {
        let mut groups = Vec::new();
        let mut claims_local = false;
        for name in candidates {
            claims_local |= self
                .metadata
                .claims_name(source.language, &source.path, name)
                || name.rsplit_once('.').is_some_and(|(namespace, _)| {
                    self.namespaces.contains(&(source.language, namespace))
                });
            match self.declaration_files(source, kind, name) {
                Ok(files) if !files.is_empty() => groups.push(files),
                Ok(_) => {}
                Err(outcome) => return outcome,
            }
        }
        groups.sort();
        groups.dedup();
        match groups.len() {
            0 if claims_local => {
                NamedResolution::Unresolved("no visible declaration matches the name".to_owned())
            }
            0 => NamedResolution::External,
            1 => NamedResolution::Files(groups.pop().expect("one declaration group")),
            _ => NamedResolution::Ambiguous,
        }
    }
    fn declaration_files(
        &self,
        source: &SourceDependencies,
        kind: SymbolKind,
        name: &str,
    ) -> Result<Vec<FileId>, NamedResolution> {
        let Some(declarations) = self.declarations.get(&(source.language, kind, name)) else {
            return Ok(Vec::new());
        };
        let mut files = Vec::new();
        let mut shared = true;
        for declaration in declarations.iter().filter(|declaration| {
            self.metadata.visible(
                source.language,
                &source.path,
                &declaration.source.path,
                name,
            )
        }) {
            shared &= declaration.shared;
            files.push(declaration.source.file);
        }
        files.sort();
        files.dedup();
        if files.len() > 1 && !shared {
            return Err(NamedResolution::Ambiguous);
        }
        Ok(files)
    }
}
fn package_root<'a>(path: &Path, roots: &'a [PathBuf]) -> &'a Path {
    roots
        .iter()
        .filter(|root| path.starts_with(root))
        .max_by_key(|root| root.components().count())
        .map_or(Path::new(""), PathBuf::as_path)
}

fn alias_target(import: &NameImport, name: &str) -> Option<String> {
    let alias = import.alias()?;
    if name == alias {
        return Some(import.target().to_owned());
    }
    let tail = name.strip_prefix(alias)?.strip_prefix('.')?;
    Some(format!("{}.{tail}", import.target()))
}
