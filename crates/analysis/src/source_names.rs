//! Names declared and referenced in source, ready for repository resolution.
use std::ops::Range;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SymbolKind {
    Type,
    Function,
    Constant,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NameReference {
    name: String,
    namespace: String,
    position: usize,
    kind: SymbolKind,
}
impl NameReference {
    pub fn new(
        name: impl Into<String>,
        namespace: impl Into<String>,
        position: usize,
        kind: SymbolKind,
    ) -> Self {
        Self {
            name: name.into(),
            namespace: namespace.into(),
            position,
            kind,
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn namespace(&self) -> &str {
        &self.namespace
    }
    pub const fn position(&self) -> usize {
        self.position
    }
    pub const fn kind(&self) -> SymbolKind {
        self.kind
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NameDeclaration {
    name: String,
    kind: SymbolKind,
    shared: bool,
}
impl NameDeclaration {
    pub fn new(name: impl Into<String>, kind: SymbolKind, shared: bool) -> Self {
        Self {
            name: name.into(),
            kind,
            shared,
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub const fn kind(&self) -> SymbolKind {
        self.kind
    }
    pub const fn is_shared(&self) -> bool {
        self.shared
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NameImport {
    target: String,
    alias: Option<String>,
    extent: Range<usize>,
    global: bool,
    kind: SymbolKind,
}
impl NameImport {
    pub fn new(
        target: impl Into<String>,
        alias: Option<String>,
        extent: Range<usize>,
        global: bool,
        kind: SymbolKind,
    ) -> Self {
        Self {
            target: target.into(),
            alias,
            extent,
            global,
            kind,
        }
    }
    pub fn target(&self) -> &str {
        &self.target
    }
    pub fn alias(&self) -> Option<&str> {
        self.alias.as_deref()
    }
    pub fn contains(&self, position: usize) -> bool {
        self.extent.contains(&position)
    }
    pub const fn is_global(&self) -> bool {
        self.global
    }
    pub const fn kind(&self) -> SymbolKind {
        self.kind
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceNames {
    declarations: Vec<NameDeclaration>,
    imports: Vec<NameImport>,
}
impl SourceNames {
    pub fn declare(&mut self, declaration: NameDeclaration) {
        self.declarations.push(declaration);
    }
    pub fn import(&mut self, import: NameImport) {
        self.imports.push(import);
    }
    pub fn declarations(&self) -> &[NameDeclaration] {
        &self.declarations
    }
    pub fn imports(&self) -> &[NameImport] {
        &self.imports
    }
}
