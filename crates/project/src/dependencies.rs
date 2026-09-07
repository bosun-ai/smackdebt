//! The dependency vocabulary both flows share: what one file's references
//! state and the tables the graph builds read them from.

use std::path::PathBuf;

use smackdebt_analysis::{DependencySyntax, FileId, FileRecord, Language, SourceRole, SourceTrust};

/// One tree's file and dependency tables, as an architecture build reads
/// them.
pub(crate) struct SideTables {
    pub(crate) files: Vec<FileRecord>,
    pub(crate) dependencies: Vec<SourceDependencies>,
}
/// Both trees' tables, filled in the same file order.
pub(crate) struct DiffTables {
    pub(crate) current: SideTables,
    pub(crate) before: SideTables,
}
impl DiffTables {
    pub(crate) fn with_capacity(selected_count: usize) -> Self {
        Self {
            current: SideTables {
                files: Vec::with_capacity(selected_count),
                dependencies: Vec::new(),
            },
            before: SideTables {
                files: Vec::with_capacity(selected_count),
                dependencies: Vec::new(),
            },
        }
    }
}
#[derive(Clone)]
pub(crate) struct SourceDependencies {
    pub(crate) file: FileId,
    pub(crate) path: PathBuf,
    pub(crate) references: Vec<DependencySyntax>,
    pub(crate) role: SourceRole,
    pub(crate) trust: SourceTrust,
    pub(crate) language: Language,
    /// Whether the file is written as a module rather than a plain script.
    pub(crate) module_syntax: bool,
}
/// What each package's own manifest states about itself, in package order.
///
/// The two tables are read together wherever a package is asked what it owns,
/// so a build can never hold one without the other.
#[derive(Clone, Copy)]
pub(crate) struct ManifestFacts<'a> {
    pub(crate) names: &'a [Option<String>],
    /// The paths each manifest names as something it publishes, installs, or
    /// runs, spelled relative to the package directory.
    pub(crate) paths: &'a [Vec<String>],
}
/// The package facts one graph side is built against.
///
/// `side_roots` are the package roots of the tree being read; `roots` are the
/// report's own package positions, which the two sides of a diff share.
#[derive(Clone, Copy)]
pub(crate) struct PackageTables<'a> {
    pub(crate) side_roots: &'a [PathBuf],
    pub(crate) roots: &'a [PathBuf],
    pub(crate) manifests: ManifestFacts<'a>,
}
