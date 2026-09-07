//! What nothing uses: orphan files, declared entry points, and the dormant
//! JavaScript rule that reads both.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use smackdebt_analysis::{
    DependencyEdge, ExternalDependency, FileId, FileRecord, OrphanCandidate, OrphanFile,
    ResolutionDiagnostic, SourceRole, dependency_degree, orphan_files,
};
use smackdebt_discovery::{is_runtime_javascript_path, is_tool_configuration_name};

use crate::dependencies::SourceDependencies;
use crate::manifest_names::package_entry_file;
use crate::paths::clean_relative;

/// Whether the streamed history window holds commits.
///
/// A file's absence from the window is evidence only when the window has
/// something to be absent from. An empty or unavailable window says nothing
/// about any file, so the rules that read coldness stand down rather than
/// treating ignorance as proof.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WindowedHistory {
    Streamed,
    Absent,
}
/// Derives orphan facts from the degree of the existing verdict file graph.
///
/// No new traversal is introduced: the file pairs and the file table are the
/// tables the architecture pass already produced.
pub(crate) fn derive_orphans(
    files: &[FileRecord],
    dependencies: &[SourceDependencies],
    file_pairs: &[(usize, usize)],
    declared_entries: &BTreeSet<FileId>,
) -> Vec<OrphanFile> {
    let degrees = dependency_degree(files.len(), file_pairs);
    let mut analyzed = BTreeSet::new();
    for source in dependencies {
        analyzed.insert(source.file);
    }
    let candidates: Vec<_> = files
        .iter()
        .enumerate()
        .map(|(position, file)| {
            OrphanCandidate::new(
                file.id(),
                file.path(),
                file.role(),
                analyzed.contains(&file.id()),
                degrees.get(position).map_or(0, |(incoming, _)| *incoming),
                declared_entries.contains(&file.id()),
            )
        })
        .collect();
    orphan_files(&candidates)
}
/// The tables that say which file each package presents as its entry point.
#[derive(Clone, Copy)]
pub(crate) struct PackageEntries<'a> {
    pub(crate) package_roots: &'a [PathBuf],
    pub(crate) manifest_names: &'a [Option<String>],
    /// The paths each package's manifest names, in package order.
    pub(crate) manifest_paths: &'a [Vec<String>],
    pub(crate) index: &'a BTreeMap<PathBuf, FileId>,
}
/// Every file a package presents as its own entry point.
///
/// A declared entry is reached from outside the repository, so nothing inside
/// it needs to import the file for it to be used. Two answers are joined: the
/// conventional entry path of the package, and every path the package's own
/// manifest names as something it publishes, installs, or runs.
///
/// Both the orphan table and the dormancy rule read this one answer, computed
/// once per build, so a package's entry point can never be an orphan in one and
/// unowned in the other.
pub(crate) fn declared_entry_files(entries: PackageEntries<'_>) -> BTreeSet<FileId> {
    let mut declared = BTreeSet::new();
    for (position, root) in entries.package_roots.iter().enumerate() {
        let fallback = root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let name = entries
            .manifest_names
            .get(position)
            .and_then(Option::as_deref)
            .unwrap_or(fallback);
        if let Some(entry) = package_entry_file(root, name, entries.index) {
            declared.insert(entry);
        }
        let manifest_paths = entries
            .manifest_paths
            .get(position)
            .map_or(&[][..], Vec::as_slice);
        for path in manifest_paths {
            let Some(cleaned) = clean_relative(&root.join(path)) else {
                continue;
            };
            if let Some(file) = entries.index.get(&cleaned) {
                declared.insert(*file);
            }
        }
    }
    declared
}
/// The JavaScript files this repository is demonstrably not working on.
///
/// Every signal is an absence, and they must all hold at once: the file is a
/// plain script rather than a module, nothing in the repository imports it, no
/// package names it as something it publishes or runs, its own name is not a
/// conventional entry name, and no commit in the streamed window touched it.
///
/// What that adds up to is inattention, not provenance. A jQuery-era library
/// dropped into a public directory answers to it, and so does a page script the
/// repository wrote years ago and has not opened since - and nothing here can
/// tell those two apart, because no signal here looks at who wrote anything.
/// The role states only what was measured: no one is working on this.
///
/// Only the JavaScript a browser or a runtime loads as written can qualify.
/// TypeScript and JSX compile from source the repository authored, so their
/// spellings are never considered however cold or unimported they are.
pub(crate) fn dormant_javascript(
    files: &[FileRecord],
    dependencies: &[SourceDependencies],
    edges: &[DependencyEdge],
    history: WindowedHistory,
    declared: &BTreeSet<FileId>,
) -> BTreeSet<FileId> {
    if history == WindowedHistory::Absent {
        return BTreeSet::new();
    }
    let scripts: BTreeSet<FileId> = dependencies
        .iter()
        .filter(|source| !source.module_syntax)
        .map(|source| source.file)
        .collect();
    let candidates: Vec<_> = files
        .iter()
        .filter(|file| is_dormancy_candidate(file) && scripts.contains(&file.id()))
        .collect();
    if candidates.is_empty() {
        return BTreeSet::new();
    }
    let imported: BTreeSet<FileId> = edges
        .iter()
        .filter(|edge| edge.affects_verdict())
        .map(DependencyEdge::target)
        .collect();
    candidates
        .into_iter()
        .map(FileRecord::id)
        .filter(|file| !imported.contains(file) && !declared.contains(file))
        .collect()
}
/// Whether the evidence rule may ever look at this file.
///
/// A file that already carries a role states what it is, and a file the window
/// recorded a commit against is worked on, so neither is dormant. Entry points
/// and tool configuration are excluded for the same reason as each other: both
/// are reached by name rather than by import, so having no importer is what
/// they are supposed to look like.
pub(crate) fn is_dormancy_candidate(file: &FileRecord) -> bool {
    let path = Path::new(file.path());
    file.role() == SourceRole::Primary
        && file.activity().is_none()
        && !smackdebt_analysis::is_entry_filename(file.path())
        && !is_tool_configuration_name(path)
        && is_runtime_javascript_path(path)
}
/// Restates every relation written in a dormant file.
///
/// The relations themselves are untouched; only what they count as changes. A
/// reference written in a file nobody is working on stops being evidence about
/// the code that ships, so it shapes no package edge, no cycle, and no orphan
/// pair - which is what the graph facts below are derived from.
pub(crate) fn restate_dormant_relations(
    dormant: &BTreeSet<FileId>,
    file_edges: &mut [DependencyEdge],
    external: &mut [ExternalDependency],
    diagnostics: &mut [ResolutionDiagnostic],
) {
    if dormant.is_empty() {
        return;
    }
    for edge in file_edges
        .iter_mut()
        .filter(|edge| dormant.contains(&edge.source()))
    {
        *edge = edge.clone().in_context_role(SourceRole::Dormant);
    }
    for row in external
        .iter_mut()
        .filter(|row| dormant.contains(&row.file()))
    {
        *row = row.clone().in_context_role(SourceRole::Dormant);
    }
    for row in diagnostics
        .iter_mut()
        .filter(|row| dormant.contains(&row.file()))
    {
        *row = row.clone().in_context_role(SourceRole::Dormant);
    }
}
/// Returns the file table with every dormant file restated under its role.
pub(crate) fn restate_dormant(files: &[FileRecord], dormant: &BTreeSet<FileId>) -> Vec<FileRecord> {
    files
        .iter()
        .map(|file| {
            if dormant.contains(&file.id()) {
                file.clone().in_context_role(SourceRole::Dormant)
            } else {
                file.clone()
            }
        })
        .collect()
}
