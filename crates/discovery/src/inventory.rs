//! Filesystem inventory and package identity.
//!
//! Discovery deliberately does not open source files.  It collects metadata in
//! one deterministic walk and leaves reading and parsing to the project and
//! language crates.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use ignore::gitignore::GitignoreBuilder;

pub use smackdebt_analysis::PackageId;
use smackdebt_analysis::SourceRole;

#[cfg(test)]
use crate::glob::glob_matches;
use crate::walk::{config_excludes, error_path, is_dependency_dir, source_walk};

/// A path relative to the inventory root.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct RelativePath(PathBuf);

impl RelativePath {
    /// Creates a relative path, rejecting absolute and parent-traversing paths.
    fn new(path: impl Into<PathBuf>) -> Option<Self> {
        let path = path.into();
        if path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
        {
            return None;
        }
        Some(Self(path))
    }

    /// Borrows the path.
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for RelativePath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl std::fmt::Display for RelativePath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.display().fmt(formatter)
    }
}

/// A recognized project manifest.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ManifestKind {
    Cargo,
    Npm,
    Python,
    Maven,
    Gradle,
    Cmake,
    Bundler,
    Gemspec,
}

impl ManifestKind {
    /// Whether this manifest kind declares a package name Smackdebt can read.
    const fn declares_a_name(self) -> bool {
        matches!(self, Self::Cargo | Self::Npm | Self::Python | Self::Gemspec)
    }

    /// Reads the declared package name from an already-recognized manifest.
    fn declared_name(self, path: &Path, source: &str) -> Option<String> {
        let name = match self {
            Self::Cargo => {
                let value = source.parse::<toml::Table>().ok()?;
                value
                    .get("lib")
                    .and_then(|section| section.get("name"))
                    .or_else(|| value.get("package").and_then(|section| section.get("name")))
                    .and_then(toml::Value::as_str)
                    .map(str::to_owned)
            }
            Self::Npm => serde_json::from_str::<serde_json::Value>(source)
                .ok()?
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            Self::Python
                if path
                    .file_name()
                    .is_some_and(|name| name == "pyproject.toml") =>
            {
                source
                    .parse::<toml::Table>()
                    .ok()?
                    .get("project")
                    .and_then(|section| section.get("name"))
                    .and_then(toml::Value::as_str)
                    .map(str::to_owned)
            }
            Self::Python => None,
            Self::Gemspec => gemspec_name(source),
            _ => None,
        }?;
        let name = name.trim();
        (!name.is_empty()).then(|| name.to_owned())
    }

    /// The source files this manifest names as its own, relative to its
    /// directory.
    ///
    /// A file a package publishes, installs as a command, or runs as a script
    /// is reached through the manifest, so nothing in the repository has to
    /// import it. Only npm manifests are read: they are the one recognized
    /// manifest kind that names JavaScript files by path.
    ///
    /// Script values are shell commands, so every whitespace-separated word
    /// that spells a JavaScript file is taken and the rest of the command is
    /// ignored. Naming one path too many only spares a file from a rule of
    /// absences, so a loose read here can never invent a fact.
    fn declared_paths(self, source: &str) -> Vec<String> {
        if self != Self::Npm {
            return Vec::new();
        }
        let Ok(manifest) = serde_json::from_str::<serde_json::Value>(source) else {
            return Vec::new();
        };
        let mut paths = Vec::new();
        for field in ["main", "module", "browser", "bin"] {
            match manifest.get(field) {
                Some(serde_json::Value::String(value)) => paths.push(value.clone()),
                Some(serde_json::Value::Object(entries)) => paths.extend(
                    entries
                        .values()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_owned),
                ),
                _ => {}
            }
        }
        if let Some(serde_json::Value::Object(scripts)) = manifest.get("scripts") {
            paths.extend(
                scripts
                    .values()
                    .filter_map(serde_json::Value::as_str)
                    .flat_map(str::split_whitespace)
                    .filter(|word| is_runtime_javascript_name(&word.to_ascii_lowercase()))
                    .map(str::to_owned),
            );
        }
        paths
    }

    fn for_file(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_str()?;
        match name {
            "Cargo.toml" => Some(Self::Cargo),
            "package.json" => Some(Self::Npm),
            "pyproject.toml" | "setup.py" | "setup.cfg" => Some(Self::Python),
            "pom.xml" => Some(Self::Maven),
            "settings.gradle" | "settings.gradle.kts" | "build.gradle" | "build.gradle.kts" => {
                Some(Self::Gradle)
            }
            "CMakeLists.txt" => Some(Self::Cmake),
            "Gemfile" | "gems.rb" => Some(Self::Bundler),
            _ if name.ends_with(".gemspec") => Some(Self::Gemspec),
            _ => None,
        }
    }
}

/// The source, package, and configuration facts selected from a tree snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotInventory {
    source_paths: Vec<PathBuf>,
    packages: Vec<(PathBuf, Option<String>)>,
    resolution_configs: Vec<PathBuf>,
}

impl SnapshotInventory {
    pub fn source_paths(&self) -> &[PathBuf] {
        &self.source_paths
    }

    pub fn packages(&self) -> &[(PathBuf, Option<String>)] {
        &self.packages
    }

    pub fn resolution_configs(&self) -> &[PathBuf] {
        &self.resolution_configs
    }
}

/// Applies discovery's source, manifest, package, and ignore policy to tree paths.
pub fn discover_snapshot(
    root: &Path,
    paths: &[PathBuf],
    mut read_metadata: impl FnMut(&Path) -> io::Result<Vec<u8>>,
) -> io::Result<SnapshotInventory> {
    let global = GitignoreBuilder::new(root).build_global().0;
    let mut exclude_builder = GitignoreBuilder::new(root);
    let _ = exclude_builder.add(root.join(".git/info/exclude"));
    let local_excludes = exclude_builder
        .build()
        .unwrap_or_else(|_| ignore::gitignore::Gitignore::empty());
    let kept = snapshot_kept_paths(root, paths, &mut read_metadata, &global, &local_excludes)?;
    let mut package_values: BTreeMap<PathBuf, Option<String>> = BTreeMap::new();
    let mut source_paths = Vec::new();
    let mut resolution_configs = Vec::new();
    for path in paths.iter().filter(|path| kept.contains(*path)) {
        if let Some(manifest) = ManifestKind::for_file(path) {
            let root = path.parent().unwrap_or(Path::new("")).to_path_buf();
            let name = package_values.entry(root).or_default();
            if name.is_none() && manifest.declares_a_name() {
                let source = read_snapshot_text(path, read_metadata(path)?)?;
                *name = manifest.declared_name(path, &source);
            }
        }
        if resolution_config_priority(path).is_some() {
            resolution_configs.push(path.clone());
        }
        if matches!(file_kind(path), FileKind::Source) {
            source_paths.push(path.clone());
        }
    }
    if package_values.is_empty() {
        package_values.insert(PathBuf::new(), None);
    }
    Ok(SnapshotInventory {
        source_paths,
        packages: package_values.into_iter().collect(),
        resolution_configs,
    })
}

fn snapshot_kept_paths(
    root: &Path,
    paths: &[PathBuf],
    read_metadata: &mut impl FnMut(&Path) -> io::Result<Vec<u8>>,
    global: &ignore::gitignore::Gitignore,
    local_excludes: &ignore::gitignore::Gitignore,
) -> io::Result<std::collections::BTreeSet<PathBuf>> {
    let mut directories = std::collections::BTreeSet::new();
    for path in paths {
        let mut parent = path.parent();
        while let Some(directory) = parent {
            directories.insert(directory.to_path_buf());
            parent = directory.parent();
        }
    }
    let mut directories = directories.into_iter().collect::<Vec<_>>();
    directories.sort_by(|left, right| {
        left.components()
            .count()
            .cmp(&right.components().count())
            .then_with(|| left.cmp(right))
    });
    let ignore_paths = paths
        .iter()
        .filter(|path| path.file_name().is_some_and(|name| name == ".gitignore"))
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let mut ignore_policy = SnapshotIgnorePolicy::new(root, global, local_excludes);
    read_reachable_ignore(
        Path::new(""),
        &ignore_paths,
        read_metadata,
        &mut ignore_policy,
    )?;
    let mut kept_directories = std::collections::BTreeSet::from([PathBuf::new()]);
    for directory in directories
        .into_iter()
        .filter(|path| !path.as_os_str().is_empty())
    {
        let parent = directory.parent().unwrap_or(Path::new(""));
        let dependency = directory.file_name().is_some_and(is_dependency_dir);
        if kept_directories.contains(parent)
            && !dependency
            && !ignore_policy.is_ignored(&directory, true)
        {
            read_reachable_ignore(&directory, &ignore_paths, read_metadata, &mut ignore_policy)?;
            kept_directories.insert(directory);
        }
    }
    Ok(paths
        .iter()
        .filter(|path| {
            kept_directories.contains(path.parent().unwrap_or(Path::new("")))
                && !ignore_policy.is_ignored(path, false)
        })
        .cloned()
        .collect())
}

fn read_reachable_ignore(
    directory: &Path,
    ignore_paths: &std::collections::BTreeSet<PathBuf>,
    read_metadata: &mut impl FnMut(&Path) -> io::Result<Vec<u8>>,
    ignore_policy: &mut SnapshotIgnorePolicy<'_>,
) -> io::Result<()> {
    let path = directory.join(".gitignore");
    if !ignore_paths.contains(&path) {
        return Ok(());
    }
    let bytes = read_metadata(&path)?;
    let source = read_snapshot_text(&path, bytes)?;
    let mut builder = GitignoreBuilder::new(directory);
    for line in source.lines() {
        let _ = builder.add_line(Some(path.clone()), line);
    }
    ignore_policy.by_directory.insert(
        directory.to_path_buf(),
        builder
            .build()
            .unwrap_or_else(|_| ignore::gitignore::Gitignore::empty()),
    );
    Ok(())
}

fn read_snapshot_text(path: &Path, bytes: Vec<u8>) -> io::Result<String> {
    String::from_utf8(bytes).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("snapshot metadata {} is not UTF-8: {error}", path.display()),
        )
    })
}

struct SnapshotIgnorePolicy<'a> {
    root: &'a Path,
    global: &'a ignore::gitignore::Gitignore,
    local_excludes: &'a ignore::gitignore::Gitignore,
    by_directory: BTreeMap<PathBuf, ignore::gitignore::Gitignore>,
}

impl<'a> SnapshotIgnorePolicy<'a> {
    fn new(
        root: &'a Path,
        global: &'a ignore::gitignore::Gitignore,
        local_excludes: &'a ignore::gitignore::Gitignore,
    ) -> Self {
        Self {
            root,
            global,
            local_excludes,
            by_directory: BTreeMap::new(),
        }
    }

    fn is_ignored(&self, path: &Path, is_directory: bool) -> bool {
        let mut parent = if is_directory {
            Some(path)
        } else {
            path.parent()
        };
        while let Some(directory) = parent.filter(|path| !path.as_os_str().is_empty()) {
            if self.matches_ignore(directory, true) {
                return true;
            }
            parent = directory.parent();
        }
        self.matches_ignore(path, is_directory)
    }

    fn matches_ignore(&self, path: &Path, is_directory: bool) -> bool {
        let absolute = self.root.join(path);
        let mut matched = self.global.matched(&absolute, is_directory);
        let local = self.local_excludes.matched(&absolute, is_directory);
        if !local.is_none() {
            matched = local;
        }
        for (directory, matcher) in &self.by_directory {
            if path.starts_with(directory) {
                let candidate = matcher.matched(path, is_directory);
                if !candidate.is_none() {
                    matched = candidate;
                }
            }
        }
        matched.is_ignore()
    }
}

/// Reads `spec.name = "value"` from a gemspec without executing Ruby.
fn gemspec_name(source: &str) -> Option<String> {
    source.lines().find_map(|line| {
        let line = line.trim_start();
        if line.starts_with('#') {
            return None;
        }
        let (receiver, value) = line.split_once('=')?;
        if !receiver.trim_end().ends_with(".name") {
            return None;
        }
        let value = value.trim();
        let quote = value
            .chars()
            .next()
            .filter(|value| *value == '"' || *value == '\'')?;
        value[1..].split(quote).next().map(str::to_owned)
    })
}

/// The kind of filesystem file recorded by inventory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FileKind {
    Source,
    Manifest(ManifestKind),
    Other,
}

/// Metadata for one regular file.  The path is owned once by inventory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredFile {
    relative_path: RelativePath,
    package: PackageId,
    kind: FileKind,
    size_bytes: u64,
}

impl DiscoveredFile {
    /// Returns the repository-relative path.
    pub fn path(&self) -> &RelativePath {
        &self.relative_path
    }

    /// Returns the package this file belongs to.
    pub const fn package(&self) -> PackageId {
        self.package
    }

    /// Returns the file's size in bytes from walk metadata.
    ///
    /// The size comes from the directory walk that discovered the file, so
    /// coverage can state byte totals without reading any file again.
    pub const fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    /// Returns whether this is a source candidate for language dispatch.
    const fn is_source(&self) -> bool {
        matches!(self.kind, FileKind::Source)
    }
}

/// A report package rooted at a directory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Package {
    id: PackageId,
    root: RelativePath,
    manifests: Vec<ManifestKind>,
    manifest_name: Option<String>,
    declared_paths: Vec<String>,
    resolution_config: Option<RelativePath>,
}

impl Package {
    #[cfg(test)]
    const fn id(&self) -> PackageId {
        self.id
    }

    /// Returns the directory relative to the inventory root.
    pub fn root(&self) -> &RelativePath {
        &self.root
    }

    /// Returns the name this package declares for itself, when it declares one.
    ///
    /// The name comes from the manifest that was already read to recognize the
    /// package root.  A manifest kind without a readable name leaves it absent.
    pub fn manifest_name(&self) -> Option<&str> {
        self.manifest_name.as_deref()
    }

    /// Returns the paths this package's manifest names as its own, as written.
    ///
    /// The paths come from the manifest that was already read to recognize the
    /// package root, so no file is opened for them. They are spelled relative
    /// to the package directory, which is where the manifest wrote them.
    pub fn declared_paths(&self) -> &[String] {
        &self.declared_paths
    }

    /// Returns the package-root TypeScript or JavaScript resolution config.
    pub fn resolution_config(&self) -> Option<&RelativePath> {
        self.resolution_config.as_ref()
    }

    /// Returns all co-located recognized manifests.
    #[cfg(test)]
    fn manifests(&self) -> &[ManifestKind] {
        &self.manifests
    }
}

pub fn generic_source_roles(path: &Path) -> Vec<SourceRole> {
    let normalized = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    let components: Vec<_> = normalized.split('/').collect();
    let name = components.last().copied().unwrap_or_default();
    let component_role = |names: &[&str]| components.iter().any(|part| names.contains(part));
    let role = if component_role(&["fixture", "fixtures", "testdata", "__fixtures__"]) {
        Some(SourceRole::Fixture)
    } else if component_role(&["bench", "benches", "benchmark", "benchmarks"])
        || name.contains("bench")
    {
        Some(SourceRole::Benchmark)
    } else if component_role(&["example", "examples"]) {
        Some(SourceRole::Example)
    } else if component_role(&["generated", "gen", "dist", "build", "coverage", "tmp"])
        || name.ends_with(".generated.rs")
        || name.ends_with(".generated.ts")
        || name.ends_with(".generated.js")
    {
        Some(SourceRole::Generated)
    } else if component_role(&["test", "tests", "spec", "specs"])
        || name.contains("_test.")
        || name.contains(".test.")
        || name.contains(".spec.")
    {
        Some(SourceRole::Test)
    } else {
        None
    };
    role.into_iter().collect()
}

/// Whether the file name is one of the generated JavaScript shapes.
///
/// This is deliberately name-only. Project composition applies it after
/// explicit rules and language markers, before the generic path rules.
pub fn has_generated_javascript_name(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    [
        ".min.js",
        ".min.mjs",
        ".min.cjs",
        ".bundle.js",
        ".bundle.mjs",
        ".bundle.cjs",
        "-bundle.js",
        "-bundle.mjs",
        "-bundle.cjs",
    ]
    .iter()
    .any(|suffix| name.ends_with(suffix) && name.len() > suffix.len())
}

/// Whether the file name is one no repository invents for its own code.
///
/// The list is deliberately one library family. A vendored copy of jQuery or
/// one of its plugins keeps the upstream name — `jquery.js`,
/// `jquery-3.7.1.js`, `jquery.floatThead.js` — because the page that loads it
/// names the file, so renaming it costs more than carrying it. Every other
/// vendored library is left to the evidence rule: guessing at names would
/// eventually silence a file the repository really did author.
pub fn has_vendored_javascript_name(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let name = name.to_ascii_lowercase();
    is_runtime_javascript_name(&name) && name.starts_with("jquery")
}

/// Whether the file name is a JavaScript spelling a browser or a runtime loads.
///
/// TypeScript and JSX spellings are excluded: both are compiled from source a
/// repository authored, so neither is ever shipped as a vendored copy.
fn is_runtime_javascript_name(name: &str) -> bool {
    [".js", ".mjs", ".cjs"]
        .iter()
        .any(|suffix| name.ends_with(suffix) && name.len() > suffix.len())
}

/// Whether the path names JavaScript a browser or a runtime loads as written.
pub fn is_runtime_javascript_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| is_runtime_javascript_name(&name.to_ascii_lowercase()))
}

/// Whether a tool finds this file by its name rather than through an import.
///
/// A bundler, a linter, or a formatter looks for a fixed configuration name and
/// loads it directly, so the dependency graph never records the one thing that
/// reads the file. No importer is the normal state of such a file rather than
/// evidence that the repository stopped caring about it.
pub fn is_tool_configuration_name(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let name = name.to_ascii_lowercase();
    name.starts_with('.')
        || [".config.js", ".config.mjs", ".config.cjs", ".conf.js"]
            .iter()
            .any(|suffix| name.ends_with(suffix) && name.len() > suffix.len())
}

/// A non-fatal inventory diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
enum InventoryDiagnostic {
    UnreadableDirectory { path: RelativePath, message: String },
    SymlinkSkipped { path: RelativePath },
    NestedCheckoutSkipped { path: RelativePath },
}

/// Counts that make one-pass behavior observable in acceptance tests.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct InventoryStats {
    directories_visited: usize,
    files_visited: usize,
    source_candidates: usize,
    symlinks_skipped: usize,
    nested_checkouts_skipped: usize,
}

/// Options for one filesystem inventory walk.
#[derive(Clone, Debug, Default)]
struct InventoryOptions {
    /// Ignore patterns in addition to `.gitignore` and dependency directories.
    excludes: Vec<String>,
    /// Include non-source files in [`Inventory::files`].
    include_other_files: bool,
}

/// The result of one deterministic filesystem walk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Inventory {
    root: PathBuf,
    files: Vec<DiscoveredFile>,
    packages: Vec<Package>,
    diagnostics: Vec<InventoryDiagnostic>,
    stats: InventoryStats,
}

impl Inventory {
    /// Walks a directory with the default ignore rules.
    pub fn discover(root: impl AsRef<Path>) -> io::Result<Self> {
        Self::discover_with(root, InventoryOptions::default())
    }

    /// Walks a directory once with additional ignore patterns.
    pub fn discover_sources(root: impl AsRef<Path>, excludes: Vec<String>) -> io::Result<Self> {
        Self::discover_with(
            root,
            InventoryOptions {
                excludes,
                include_other_files: false,
            },
        )
    }

    /// Walks one selected file or directory while keeping paths relative to a
    /// surrounding repository root.
    pub fn discover_selected_sources(
        root: impl AsRef<Path>,
        selected: impl AsRef<Path>,
        excludes: Vec<String>,
    ) -> io::Result<Self> {
        let root = root.as_ref();
        let selected = selected.as_ref();
        if !selected.starts_with(root) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "inventory selection is outside its identity root",
            ));
        }
        Self::discover_with_roots(
            root,
            selected,
            InventoryOptions {
                excludes,
                include_other_files: false,
            },
        )
    }

    /// Walks a directory once with explicit ignore options.
    fn discover_with(root: impl AsRef<Path>, options: InventoryOptions) -> io::Result<Self> {
        let root = root.as_ref();
        let metadata = fs::metadata(root)?;
        if !metadata.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "inventory root is not a directory",
            ));
        }
        Self::discover_with_roots(root, root, options)
    }

    fn discover_with_roots(
        root: &Path,
        selected: &Path,
        options: InventoryOptions,
    ) -> io::Result<Self> {
        let selected_metadata = fs::metadata(selected)?;
        if !selected_metadata.is_dir() && !selected_metadata.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "inventory selection is not a file or directory",
            ));
        }
        let root = root.to_path_buf();
        let excludes = config_excludes(&root, &options.excludes);
        let nested_checkouts = Arc::new(Mutex::new(Vec::new()));
        let mut walker = Walker::new(options);
        let selection_ignored =
            walker.visit_ancestor_metadata(&root, selected, selected_metadata.is_file())?;
        if !selection_ignored {
            for entry in source_walk(selected, excludes, Arc::clone(&nested_checkouts)) {
                match entry {
                    Ok(entry) => walker.visit(&entry, &root),
                    Err(error) => walker.record_error(&error, &root),
                }
            }
        }
        let nested_checkouts = std::mem::take(
            &mut *nested_checkouts
                .lock()
                .expect("the serial walk never poisons the nested-checkout list"),
        );
        walker.record_nested_checkouts(nested_checkouts, &root);
        walker.finish(root)
    }

    /// Returns the absolute inventory root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns all recorded files in stable relative-path order.
    #[cfg(test)]
    fn files(&self) -> &[DiscoveredFile] {
        &self.files
    }

    /// Returns source candidates only.
    pub fn source_files(&self) -> impl Iterator<Item = &DiscoveredFile> {
        self.files.iter().filter(|file| file.is_source())
    }

    /// Returns all package roots in stable path order.
    pub fn packages(&self) -> &[Package] {
        &self.packages
    }

    /// Returns non-fatal diagnostics.
    #[cfg(test)]
    fn diagnostics(&self) -> &[InventoryDiagnostic] {
        &self.diagnostics
    }

    /// Returns one-pass instrumentation counters.
    #[cfg(test)]
    const fn stats(&self) -> InventoryStats {
        self.stats
    }

    /// Returns the number of directory and file entries the walk yielded.
    ///
    /// Pruned subtrees — ignored, excluded, and dependency directories — are
    /// never yielded, so they contribute nothing here.
    pub const fn visited_entries(&self) -> usize {
        self.stats.directories_visited + self.stats.files_visited
    }

    /// Returns the nested git checkouts the walk pruned, in walk order.
    ///
    /// Each path names a directory below the analyzed root that carries its
    /// own `.git` entry — a submodule, an embedded clone, or a linked
    /// worktree — and therefore contributed no candidates.
    pub fn nested_checkouts(&self) -> impl Iterator<Item = &RelativePath> {
        self.diagnostics
            .iter()
            .filter_map(|diagnostic| match diagnostic {
                InventoryDiagnostic::NestedCheckoutSkipped { path } => Some(path),
                _ => None,
            })
    }

    /// Resolves an inventory-relative path to an absolute path.
    pub fn absolute_path(&self, path: &RelativePath) -> Option<PathBuf> {
        let joined = self.root.join(path.as_path());
        joined.starts_with(&self.root).then_some(joined)
    }
}

struct Walker {
    options: InventoryOptions,
    files: Vec<RawFile>,
    manifest_dirs: BTreeMap<PathBuf, Vec<ManifestKind>>,
    manifest_names: BTreeMap<PathBuf, Option<String>>,
    manifest_paths: BTreeMap<PathBuf, Vec<String>>,
    resolution_configs: BTreeMap<PathBuf, RelativePath>,
    diagnostics: Vec<InventoryDiagnostic>,
    stats: InventoryStats,
}

struct RawFile {
    path: RelativePath,
    kind: FileKind,
    size_bytes: u64,
}

fn ancestor_directories(root: &Path, selected: &Path, selected_is_file: bool) -> Vec<PathBuf> {
    let mut directory = if selected_is_file {
        selected.parent()
    } else {
        selected.parent().filter(|_| selected != root)
    };
    let mut ancestors = Vec::new();
    while let Some(path) = directory.filter(|path| path.starts_with(root)) {
        ancestors.push(path.to_path_buf());
        if path == root {
            break;
        }
        directory = path.parent();
    }
    ancestors.reverse();
    ancestors
}

impl Walker {
    fn new(options: InventoryOptions) -> Self {
        Self {
            options,
            files: Vec::new(),
            manifest_dirs: BTreeMap::new(),
            manifest_names: BTreeMap::new(),
            manifest_paths: BTreeMap::new(),
            resolution_configs: BTreeMap::new(),
            diagnostics: Vec::new(),
            stats: InventoryStats::default(),
        }
    }

    /// Reads package and resolution metadata from each directory between the
    /// identity root and the selected subtree without walking sibling source.
    fn visit_ancestor_metadata(
        &mut self,
        root: &Path,
        selected: &Path,
        selected_is_file: bool,
    ) -> io::Result<bool> {
        let ancestors = ancestor_directories(root, selected, selected_is_file);
        let ignore_policy = AncestorMetadataIgnore::new(root, &ancestors, &self.options.excludes);
        for directory in ancestors {
            self.visit_ancestor_directory(root, &directory, &ignore_policy)?;
        }
        let selected_relative = selected
            .strip_prefix(root)
            .expect("selection stays under its identity root");
        Ok(ignore_policy.is_ignored(selected_relative, !selected_is_file))
    }

    fn visit_ancestor_directory(
        &mut self,
        root: &Path,
        directory: &Path,
        ignore_policy: &AncestorMetadataIgnore,
    ) -> io::Result<()> {
        self.stats.directories_visited += 1;
        for entry in fs::read_dir(directory)? {
            self.visit_ancestor_entry(root, entry?, ignore_policy)?;
        }
        Ok(())
    }

    fn visit_ancestor_entry(
        &mut self,
        root: &Path,
        entry: fs::DirEntry,
        ignore_policy: &AncestorMetadataIgnore,
    ) -> io::Result<()> {
        if !entry.file_type()?.is_file() {
            return Ok(());
        }
        let absolute = entry.path();
        let relative_path = absolute
            .strip_prefix(root)
            .expect("ancestor metadata stays under its identity root")
            .to_path_buf();
        let Some(relative) = RelativePath::new(relative_path.clone()) else {
            return Ok(());
        };
        if ignore_policy.is_ignored(&relative_path, false) {
            return Ok(());
        }
        self.record_resolution_config(&relative_path, &relative);
        let Some(manifest) = ManifestKind::for_file(&relative_path) else {
            return Ok(());
        };
        let parent = relative_path.parent().unwrap_or_else(|| Path::new(""));
        self.manifest_dirs
            .entry(parent.to_path_buf())
            .or_default()
            .push(manifest);
        if manifest.declares_a_name() {
            self.record_manifest_name(parent, manifest, &absolute);
        }
        Ok(())
    }

    /// Records one entry the walk yielded.
    ///
    /// The depth-zero root entry only counts as a visited directory; every
    /// deeper entry is classified by its own file type.
    fn visit(&mut self, entry: &ignore::DirEntry, root: &Path) {
        if is_walk_root_directory(entry) {
            self.stats.directories_visited += 1;
            return;
        }
        let Ok(relative_path) = entry.path().strip_prefix(root).map(Path::to_path_buf) else {
            return;
        };
        let Some(relative) = RelativePath::new(relative_path.clone()) else {
            return;
        };
        let Some(file_type) = entry.file_type() else {
            return;
        };
        if file_type.is_symlink() {
            self.stats.symlinks_skipped += 1;
            self.diagnostics
                .push(InventoryDiagnostic::SymlinkSkipped { path: relative });
            return;
        }
        if file_type.is_dir() {
            self.stats.directories_visited += 1;
            return;
        }
        if !file_type.is_file() {
            return;
        }

        self.stats.files_visited += 1;
        self.record_resolution_config(&relative_path, &relative);
        let kind = ManifestKind::for_file(&relative_path)
            .map(FileKind::Manifest)
            .unwrap_or_else(|| file_kind(&relative_path));
        if matches!(kind, FileKind::Other) && !self.options.include_other_files {
            return;
        }
        if matches!(kind, FileKind::Source) {
            self.stats.source_candidates += 1;
        }
        if let FileKind::Manifest(manifest) = kind {
            let parent = relative_path.parent().unwrap_or_else(|| Path::new(""));
            self.manifest_dirs
                .entry(parent.to_path_buf())
                .or_default()
                .push(manifest);
            if manifest.declares_a_name() {
                self.record_manifest_name(parent, manifest, entry.path());
            }
        }
        self.files.push(RawFile {
            path: relative,
            kind,
            size_bytes: entry.metadata().map_or(0, |metadata| metadata.len()),
        });
    }

    fn record_resolution_config(&mut self, path: &Path, relative: &RelativePath) {
        let Some(priority) = resolution_config_priority(path) else {
            return;
        };
        let parent = path.parent().unwrap_or_else(|| Path::new(""));
        let current = self
            .resolution_configs
            .get(parent)
            .and_then(|value| resolution_config_priority(value.as_path()));
        if current.is_none_or(|value| priority < value) {
            self.resolution_configs
                .insert(parent.to_path_buf(), relative.clone());
        }
    }

    /// Records one walk error, keeping unreadable directories visible.
    fn record_error(&mut self, error: &ignore::Error, root: &Path) {
        let Some(io_error) = error.io_error() else {
            return;
        };
        let Some(path) = error_path(error) else {
            return;
        };
        let relative = path.strip_prefix(root).unwrap_or(path);
        if let Some(path) = RelativePath::new(relative.to_path_buf()) {
            self.diagnostics
                .push(InventoryDiagnostic::UnreadableDirectory {
                    path,
                    message: io_error.to_string(),
                });
        }
    }

    /// Records the nested checkouts the walk pruned, in walk order.
    fn record_nested_checkouts(&mut self, pruned: Vec<PathBuf>, root: &Path) {
        for path in pruned {
            let relative = path.strip_prefix(root).unwrap_or(&path);
            if let Some(path) = RelativePath::new(relative.to_path_buf()) {
                self.stats.nested_checkouts_skipped += 1;
                self.diagnostics
                    .push(InventoryDiagnostic::NestedCheckoutSkipped { path });
            }
        }
    }

    /// Reads the declared name and declared paths of one recognized manifest.
    ///
    /// The first manifest kind of a directory that declares a usable name owns
    /// the package name.  An unreadable or nameless manifest is not an error.
    fn record_manifest_name(&mut self, parent: &Path, manifest: ManifestKind, absolute: &Path) {
        let Ok(source) = fs::read_to_string(absolute) else {
            return;
        };
        let name = self.manifest_names.entry(parent.to_path_buf()).or_default();
        if name.is_none() {
            *name = manifest.declared_name(absolute, &source);
        }
        let declared = manifest.declared_paths(&source);
        if !declared.is_empty() {
            self.manifest_paths
                .entry(parent.to_path_buf())
                .or_default()
                .extend(declared);
        }
    }

    fn finish(mut self, root: PathBuf) -> io::Result<Inventory> {
        let mut package_roots: Vec<_> = self.manifest_dirs.keys().cloned().collect();
        if package_roots.is_empty() {
            package_roots.push(PathBuf::new());
        }
        package_roots.sort();
        let packages: Vec<Package> = package_roots
            .iter()
            .enumerate()
            .map(|(index, path)| Package {
                id: PackageId::from_index(index),
                root: RelativePath::new(path.clone()).expect("package root is relative"),
                manifests: self.manifest_dirs.get(path).cloned().unwrap_or_default(),
                manifest_name: self.manifest_names.get(path).cloned().flatten(),
                declared_paths: self.manifest_paths.get(path).cloned().unwrap_or_default(),
                resolution_config: nearest_resolution_config(path, &self.resolution_configs),
            })
            .collect();

        let mut package_by_root = BTreeMap::new();
        for package in &packages {
            package_by_root.insert(package.root.as_path().to_path_buf(), package.id);
        }
        let default_package = packages[0].id;
        let mut files = Vec::with_capacity(self.files.len());
        self.files.sort_by(|left, right| left.path.cmp(&right.path));
        for file in self.files {
            let package =
                nearest_package(file.path.as_path(), &package_by_root).unwrap_or(default_package);
            files.push(DiscoveredFile {
                relative_path: file.path,
                package,
                kind: file.kind,
                size_bytes: file.size_bytes,
            });
        }

        Ok(Inventory {
            root,
            files,
            packages,
            diagnostics: self.diagnostics,
            stats: self.stats,
        })
    }
}

fn is_walk_root_directory(entry: &ignore::DirEntry) -> bool {
    entry.depth() == 0 && entry.file_type().is_some_and(|kind| kind.is_dir())
}

struct AncestorMetadataIgnore {
    root: PathBuf,
    global: ignore::gitignore::Gitignore,
    repository: ignore::gitignore::Gitignore,
    by_directory: Vec<(PathBuf, ignore::gitignore::Gitignore)>,
    configured: ignore::gitignore::Gitignore,
}

impl AncestorMetadataIgnore {
    fn new(root: &Path, ancestors: &[PathBuf], excludes: &[String]) -> Self {
        let global = GitignoreBuilder::new(root).build_global().0;
        let mut repository_builder = GitignoreBuilder::new(root);
        let _ = repository_builder.add(root.join(".git/info/exclude"));
        let repository = repository_builder
            .build()
            .unwrap_or_else(|_| ignore::gitignore::Gitignore::empty());
        let by_directory = ancestors
            .iter()
            .filter_map(|directory| {
                let path = directory.join(".gitignore");
                path.is_file().then(|| {
                    let mut builder = GitignoreBuilder::new(directory);
                    let _ = builder.add(path);
                    (
                        directory
                            .strip_prefix(root)
                            .unwrap_or(Path::new(""))
                            .to_path_buf(),
                        builder
                            .build()
                            .unwrap_or_else(|_| ignore::gitignore::Gitignore::empty()),
                    )
                })
            })
            .collect();
        Self {
            root: root.to_path_buf(),
            global,
            repository,
            by_directory,
            configured: config_excludes(root, excludes),
        }
    }

    fn is_ignored(&self, path: &Path, is_directory: bool) -> bool {
        let absolute = self.root.join(path);
        let mut matched = self
            .global
            .matched_path_or_any_parents(&absolute, is_directory);
        let repository = self
            .repository
            .matched_path_or_any_parents(&absolute, is_directory);
        if !repository.is_none() {
            matched = repository;
        }
        for (directory, matcher) in &self.by_directory {
            if path.starts_with(directory) {
                let candidate = matcher.matched_path_or_any_parents(&absolute, is_directory);
                if !candidate.is_none() {
                    matched = candidate;
                }
            }
        }
        matched.is_ignore()
            || self
                .configured
                .matched_path_or_any_parents(&absolute, is_directory)
                .is_ignore()
    }
}

fn resolution_config_priority(path: &Path) -> Option<u8> {
    match path.file_name()?.to_str()? {
        "tsconfig.json" => Some(0),
        "jsconfig.json" => Some(1),
        _ => None,
    }
}

fn nearest_resolution_config(
    package_root: &Path,
    configs: &BTreeMap<PathBuf, RelativePath>,
) -> Option<RelativePath> {
    let mut directory = Some(package_root);
    while let Some(path) = directory {
        if let Some(config) = configs.get(path) {
            return Some(config.clone());
        }
        directory = path.parent();
    }
    None
}

fn file_kind(path: &Path) -> FileKind {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    let source = matches!(
        extension,
        "c" | "h"
            | "cc"
            | "hh"
            | "cpp"
            | "hpp"
            | "cxx"
            | "hxx"
            | "java"
            | "js"
            | "jsx"
            | "mjs"
            | "cjs"
            | "py"
            | "rs"
            | "ts"
            | "tsx"
            | "mts"
            | "cts"
            | "rb"
            | "vue"
            | "astro"
            | "kt"
            | "kts"
            | "go"
            | "cs"
            | "swift"
            | "php"
            | "scala"
            | "sc"
            | "ex"
            | "exs"
            | "dart"
    ) || path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, "Rakefile" | "Gemfile"));
    if source {
        FileKind::Source
    } else {
        FileKind::Other
    }
}

/// Returns whether a path name identifies recognized source.
pub fn is_source_path(path: &Path) -> bool {
    matches!(file_kind(path), FileKind::Source)
}

fn nearest_package(path: &Path, packages: &BTreeMap<PathBuf, PackageId>) -> Option<PackageId> {
    let mut directory = path.parent()?;
    loop {
        if let Some(id) = packages.get(directory) {
            return Some(*id);
        }
        directory = directory.parent()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn package_keeps_its_preferred_resolution_config_from_the_same_walk() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("package.json"), "{\"name\":\"x\"}").unwrap();
        fs::write(directory.path().join("jsconfig.json"), "{}").unwrap();
        fs::write(directory.path().join("tsconfig.json"), "{}").unwrap();
        fs::write(directory.path().join("index.ts"), "export {};").unwrap();

        let inventory = Inventory::discover_sources(directory.path(), Vec::new()).unwrap();
        assert_eq!(
            inventory.packages()[0]
                .resolution_config()
                .map(RelativePath::as_path),
            Some(Path::new("tsconfig.json"))
        );
        assert_eq!(inventory.stats().files_visited, 4);
    }

    #[test]
    fn co_located_manifests_form_one_package() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("Cargo.toml"), "[package]\nname='x'\n").unwrap();
        fs::write(directory.path().join("package.json"), "{}\n").unwrap();
        fs::write(directory.path().join("main.rs"), "fn main() {}\n").unwrap();
        let inventory = Inventory::discover(directory.path()).unwrap();
        assert_eq!(inventory.packages().len(), 1);
        assert_eq!(
            inventory.packages()[0].manifests(),
            &[ManifestKind::Cargo, ManifestKind::Npm]
        );
        assert_eq!(inventory.source_files().count(), 1);
        assert_eq!(inventory.files()[0].package(), inventory.packages()[0].id());
    }

    #[test]
    fn astro_is_source_in_filesystem_and_ref_inventories() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join("src")).unwrap();
        fs::write(directory.path().join("src/page.astro"), "<h1>Hello</h1>\n").unwrap();

        let inventory = Inventory::discover_sources(directory.path(), Vec::new()).unwrap();
        assert_eq!(
            inventory
                .source_files()
                .map(|file| file.path().as_path())
                .collect::<Vec<_>>(),
            [Path::new("src/page.astro")]
        );

        let paths = vec![PathBuf::from("src/page.astro")];
        let snapshot = discover_snapshot(directory.path(), &paths, |path| {
            fs::read(directory.path().join(path))
        })
        .unwrap();
        assert_eq!(snapshot.source_paths(), paths);
    }

    #[test]
    fn nested_file_uses_nearest_package() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join("nested/src")).unwrap();
        fs::write(directory.path().join("Cargo.toml"), "").unwrap();
        fs::write(directory.path().join("nested/package.json"), "{}").unwrap();
        fs::write(directory.path().join("nested/src/main.js"), "let x = 1;\n").unwrap();
        let inventory = Inventory::discover(directory.path()).unwrap();
        let file = inventory.source_files().next().unwrap();
        assert_eq!(file.package(), inventory.packages()[1].id());
        assert_eq!(file.path().as_path(), Path::new("nested/src/main.js"));
    }

    #[test]
    fn selected_walk_keeps_repository_paths_and_reads_each_package_ancestor_once() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join("nested/src")).unwrap();
        fs::create_dir(directory.path().join("outside")).unwrap();
        fs::write(
            directory.path().join("Cargo.toml"),
            "[package]\nname='root'\n",
        )
        .unwrap();
        fs::write(
            directory.path().join("nested/package.json"),
            "{\"name\":\"nested\"}\n",
        )
        .unwrap();
        fs::write(directory.path().join("nested/tsconfig.json"), "{}\n").unwrap();
        fs::write(
            directory.path().join("nested/src/main.ts"),
            "export const value = 1;\n",
        )
        .unwrap();
        fs::write(
            directory.path().join("outside/ignored.rs"),
            "fn ignored() {}\n",
        )
        .unwrap();

        let inventory = Inventory::discover_selected_sources(
            directory.path(),
            directory.path().join("nested/src"),
            Vec::new(),
        )
        .unwrap();

        assert_eq!(source_paths(&inventory), ["nested/src/main.ts"]);
        assert_eq!(inventory.stats().directories_visited, 3);
        assert_eq!(inventory.stats().files_visited, 1);
        assert_eq!(inventory.stats().source_candidates, 1);
        assert_eq!(inventory.packages().len(), 2);
        let file = inventory.source_files().next().unwrap();
        let package = &inventory.packages()[file.package().index()];
        assert_eq!(package.root().as_path(), Path::new("nested"));
        assert_eq!(
            package.resolution_config().map(RelativePath::as_path),
            Some(Path::new("nested/tsconfig.json"))
        );
    }

    #[test]
    fn selected_walk_does_not_restore_ignored_or_excluded_ancestor_metadata() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join("nested/src")).unwrap();
        fs::write(
            directory.path().join(".gitignore"),
            "nested/package.json\nnested/tsconfig.json\n",
        )
        .unwrap();
        fs::write(directory.path().join("nested/package.json"), "{}\n").unwrap();
        fs::write(directory.path().join("nested/tsconfig.json"), "{}\n").unwrap();
        fs::write(directory.path().join("nested/src/main.ts"), "export {};\n").unwrap();

        let ignored = Inventory::discover_selected_sources(
            directory.path(),
            directory.path().join("nested/src"),
            Vec::new(),
        )
        .unwrap();
        assert_eq!(ignored.packages().len(), 1);
        assert_eq!(ignored.packages()[0].root().as_path(), Path::new(""));
        assert_eq!(ignored.packages()[0].resolution_config(), None);

        fs::write(directory.path().join(".gitignore"), "nested/\n").unwrap();
        let ignored_directory = Inventory::discover_selected_sources(
            directory.path(),
            directory.path().join("nested/src"),
            Vec::new(),
        )
        .unwrap();
        assert!(ignored_directory.source_files().next().is_none());
        assert_eq!(ignored_directory.packages().len(), 1);
        assert_eq!(
            ignored_directory.packages()[0].root().as_path(),
            Path::new("")
        );

        fs::write(directory.path().join(".gitignore"), "").unwrap();
        fs::write(
            directory.path().join("nested/.gitignore"),
            "/package.json\n/tsconfig.json\n",
        )
        .unwrap();
        let nested_ignored = Inventory::discover_selected_sources(
            directory.path(),
            directory.path().join("nested/src"),
            Vec::new(),
        )
        .unwrap();
        assert_eq!(nested_ignored.packages().len(), 1);
        assert_eq!(nested_ignored.packages()[0].root().as_path(), Path::new(""));
        assert_eq!(nested_ignored.packages()[0].resolution_config(), None);

        fs::write(directory.path().join("nested/.gitignore"), "").unwrap();
        let excluded = Inventory::discover_selected_sources(
            directory.path(),
            directory.path().join("nested/src"),
            vec![
                "/nested/package.json".to_owned(),
                "/nested/tsconfig.json".to_owned(),
            ],
        )
        .unwrap();
        assert_eq!(excluded.packages().len(), 1);
        assert_eq!(excluded.packages()[0].root().as_path(), Path::new(""));
        assert_eq!(excluded.packages()[0].resolution_config(), None);
    }

    #[test]
    fn source_free_selected_directory_never_visits_repository_source() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join("docs")).unwrap();
        fs::write(directory.path().join("outside.rs"), "fn outside() {}\n").unwrap();

        let inventory = Inventory::discover_selected_sources(
            directory.path(),
            directory.path().join("docs"),
            Vec::new(),
        )
        .unwrap();

        assert!(inventory.source_files().next().is_none());
        assert_eq!(inventory.stats().directories_visited, 2);
        assert_eq!(inventory.stats().files_visited, 0);
        assert_eq!(inventory.stats().source_candidates, 0);
    }

    #[test]
    fn gitignore_dependencies_and_symlink_entries_are_not_source() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join("target")).unwrap();
        fs::write(directory.path().join(".gitignore"), "ignored/\n").unwrap();
        fs::create_dir_all(directory.path().join("ignored")).unwrap();
        fs::write(directory.path().join("ignored/no.rs"), "").unwrap();
        fs::write(directory.path().join("target/no.rs"), "").unwrap();
        fs::write(directory.path().join("ok.rs"), "").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            directory.path().join("ok.rs"),
            directory.path().join("link.rs"),
        )
        .unwrap();
        let inventory = Inventory::discover(directory.path()).unwrap();
        assert_eq!(inventory.source_files().count(), 1);
        assert!(
            inventory
                .diagnostics()
                .iter()
                .any(|diagnostic| matches!(diagnostic, InventoryDiagnostic::SymlinkSkipped { .. }))
        );
        assert!(inventory.stats().directories_visited >= 1);
    }

    #[test]
    fn tracked_generated_directories_remain_role_candidates() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join("generated")).unwrap();
        fs::write(
            directory.path().join("generated/client.ts"),
            "export const x = 1\n",
        )
        .unwrap();
        let inventory = Inventory::discover(directory.path()).unwrap();
        let file = inventory.source_files().next().unwrap();
        assert_eq!(file.path().as_path(), Path::new("generated/client.ts"));
        assert_eq!(
            generic_source_roles(file.path().as_path()),
            vec![SourceRole::Generated]
        );
    }

    #[test]
    fn nested_test_support_directories_have_one_specific_role() {
        for (path, role) in [
            ("crates/cli/tests/fixtures/data.rs", SourceRole::Fixture),
            ("crates/cli/tests/examples/demo.rs", SourceRole::Example),
            (
                "crates/cli/tests/benchmarks/large.rs",
                SourceRole::Benchmark,
            ),
            (
                "crates/cli/tests/generated/client.rs",
                SourceRole::Generated,
            ),
        ] {
            assert_eq!(generic_source_roles(Path::new(path)), vec![role], "{path}");
        }
    }

    #[test]
    fn generated_javascript_names_match_only_the_exact_file_shapes() {
        for path in [
            "vendor.min.js",
            "vendor.min.mjs",
            "vendor.min.cjs",
            "client.bundle.js",
            "client.bundle.mjs",
            "client.bundle.cjs",
            "client-bundle.js",
            "client-bundle.mjs",
            "client-bundle.cjs",
            "public/client.bundle.js",
        ] {
            assert!(has_generated_javascript_name(Path::new(path)), "{path}");
        }

        for path in [
            "client.bundle.ts",
            "client.bundle.jsx",
            "client-bundle.tsx",
            "client.bundle.JS",
            "bundle.js",
            ".min.js",
            "client.min.js.map",
            "assets/editor.js",
        ] {
            assert!(!has_generated_javascript_name(Path::new(path)), "{path}");
        }
    }

    #[test]
    fn no_manifest_uses_root_package() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("main.py"), "print(1)\n").unwrap();
        let inventory = Inventory::discover(directory.path()).unwrap();
        assert_eq!(inventory.packages().len(), 1);
        assert_eq!(inventory.packages()[0].root().as_path(), Path::new(""));
    }

    #[test]
    fn every_manifest_kind_contributes_its_declared_name() {
        let directory = tempfile::tempdir().unwrap();
        let cases = [
            (
                "cargo",
                "Cargo.toml",
                "[package]\nname = \"cargo-package\"\n",
                Some("cargo-package"),
            ),
            (
                "cargo-lib",
                "Cargo.toml",
                "[package]\nname = \"cargo-package\"\n[lib]\nname = \"library_override\"\n",
                Some("library_override"),
            ),
            (
                "npm",
                "package.json",
                "{\"name\": \"@scope/name\", \"private\": true}\n",
                Some("@scope/name"),
            ),
            (
                "python",
                "pyproject.toml",
                "[project]\nname = \"python-package\"\n",
                Some("python-package"),
            ),
            (
                "ruby",
                "ruby.gemspec",
                "Gem::Specification.new do |spec|\n  # spec.name = \"commented-out\"\n  spec.name = \"gem-package\"\nend\n",
                Some("gem-package"),
            ),
            ("empty", "Cargo.toml", "[package]\nname = \"\"\n", None),
            ("nameless", "package.json", "{\"private\": true}\n", None),
            ("broken", "Cargo.toml", "[package\nname\n", None),
            ("other", "pom.xml", "<project/>\n", None),
        ];
        for (root, manifest, source, _) in cases {
            fs::create_dir_all(directory.path().join(root)).unwrap();
            fs::write(directory.path().join(root).join(manifest), source).unwrap();
        }
        let inventory = Inventory::discover(directory.path()).unwrap();
        let names: Vec<_> = inventory
            .packages()
            .iter()
            .map(|package| (package.root().to_string(), package.manifest_name()))
            .collect();
        let mut expected: Vec<_> = cases
            .iter()
            .map(|(root, _, _, name)| ((*root).to_owned(), *name))
            .collect();
        expected.sort();
        assert_eq!(names, expected);
    }

    fn source_paths(inventory: &Inventory) -> Vec<String> {
        inventory
            .source_files()
            .map(|file| file.path().to_string())
            .collect()
    }

    #[test]
    fn a_nested_ignore_file_overrides_its_ancestor() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join(".gitignore"), "kept.rs\n").unwrap();
        fs::create_dir_all(directory.path().join("nested")).unwrap();
        fs::write(directory.path().join("nested/.gitignore"), "!kept.rs\n").unwrap();
        fs::write(directory.path().join("kept.rs"), "").unwrap();
        fs::write(directory.path().join("nested/kept.rs"), "").unwrap();
        let inventory = Inventory::discover(directory.path()).unwrap();
        assert_eq!(source_paths(&inventory), ["nested/kept.rs"]);
    }

    #[test]
    fn snapshot_inventory_applies_nested_negation_from_its_own_directory() {
        let paths = vec![
            PathBuf::from(".gitignore"),
            PathBuf::from("nested/.gitignore"),
            PathBuf::from("nested/drop.rs"),
            PathBuf::from("nested/kept.rs"),
            PathBuf::from("other/kept.rs"),
        ];
        let contents = BTreeMap::from([
            (PathBuf::from(".gitignore"), b"*.rs\n".to_vec()),
            (PathBuf::from("nested/.gitignore"), b"!kept.rs\n".to_vec()),
        ]);
        let root = tempfile::tempdir().unwrap();
        let inventory = discover_snapshot(root.path(), &paths, |path| {
            contents
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, path.display().to_string()))
        })
        .unwrap();
        assert_eq!(inventory.source_paths(), [PathBuf::from("nested/kept.rs")]);
    }

    #[test]
    fn snapshot_inventory_does_not_read_ignore_files_below_an_ignored_directory() {
        let paths = vec![
            PathBuf::from(".gitignore"),
            PathBuf::from("hidden/.gitignore"),
            PathBuf::from("hidden/file.rs"),
            PathBuf::from("visible.rs"),
        ];
        let contents = BTreeMap::from([
            (PathBuf::from(".gitignore"), b"hidden/\n".to_vec()),
            (PathBuf::from("hidden/.gitignore"), b"!file.rs\n".to_vec()),
        ]);
        let root = tempfile::tempdir().unwrap();
        let mut reads = Vec::new();
        let inventory = discover_snapshot(root.path(), &paths, |path| {
            reads.push(path.to_path_buf());
            contents
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, path.display().to_string()))
        })
        .unwrap();
        assert_eq!(inventory.source_paths(), [PathBuf::from("visible.rs")]);
        assert_eq!(reads, [PathBuf::from(".gitignore")]);
    }

    #[test]
    fn snapshot_inventory_fails_when_reachable_metadata_cannot_be_read() {
        let paths = vec![PathBuf::from("package.json")];
        let root = tempfile::tempdir().unwrap();
        let error = discover_snapshot(root.path(), &paths, |path| {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                path.display().to_string(),
            ))
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn snapshot_inventory_fails_when_reachable_ignore_text_is_corrupt() {
        let paths = vec![PathBuf::from(".gitignore"), PathBuf::from("main.rs")];
        let root = tempfile::tempdir().unwrap();
        let error = discover_snapshot(root.path(), &paths, |_| Ok(vec![0xff])).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains(".gitignore"));
    }

    #[test]
    fn snapshot_inventory_matches_filesystem_source_manifest_and_ignore_policy() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join(".git/info")).unwrap();
        fs::create_dir_all(root.path().join("nested")).unwrap();
        fs::create_dir_all(root.path().join("node_modules")).unwrap();
        fs::create_dir_all(root.path().join("pkg")).unwrap();
        fs::write(root.path().join(".gitignore"), "*.rs\n").unwrap();
        fs::write(root.path().join("nested/.gitignore"), "!kept.rs\n").unwrap();
        fs::write(root.path().join(".git/info/exclude"), "excluded.ts\n").unwrap();
        fs::write(root.path().join("nested/kept.rs"), "").unwrap();
        fs::write(root.path().join("nested/drop.rs"), "").unwrap();
        fs::write(root.path().join("included.ts"), "").unwrap();
        fs::write(root.path().join("excluded.ts"), "").unwrap();
        fs::write(root.path().join("node_modules/no.ts"), "").unwrap();
        fs::write(
            root.path().join("pkg/widget.gemspec"),
            "Gem::Specification.new { |spec| spec.name = 'widget' }\n",
        )
        .unwrap();
        fs::write(root.path().join("pkg/tsconfig.json"), "{}\n").unwrap();
        let paths = vec![
            PathBuf::from(".gitignore"),
            PathBuf::from("excluded.ts"),
            PathBuf::from("included.ts"),
            PathBuf::from("nested/.gitignore"),
            PathBuf::from("nested/drop.rs"),
            PathBuf::from("nested/kept.rs"),
            PathBuf::from("node_modules/no.ts"),
            PathBuf::from("pkg/tsconfig.json"),
            PathBuf::from("pkg/widget.gemspec"),
        ];
        let snapshot =
            discover_snapshot(root.path(), &paths, |path| fs::read(root.path().join(path)))
                .unwrap();
        let filesystem = Inventory::discover_sources(root.path(), Vec::new()).unwrap();
        let filesystem_sources = filesystem
            .source_files()
            .map(|file| file.path().as_path().to_path_buf())
            .collect::<Vec<_>>();
        assert_eq!(snapshot.source_paths(), filesystem_sources);
        assert_eq!(
            snapshot.packages(),
            [(PathBuf::from("pkg"), Some("widget".to_owned()))]
        );
        assert_eq!(
            snapshot.resolution_configs(),
            [PathBuf::from("pkg/tsconfig.json")]
        );
    }

    #[test]
    fn a_negation_reincludes_a_previously_excluded_file() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join(".gitignore"), "*.rs\n!keep.rs\n").unwrap();
        fs::write(directory.path().join("drop.rs"), "").unwrap();
        fs::write(directory.path().join("keep.rs"), "").unwrap();
        let inventory = Inventory::discover(directory.path()).unwrap();
        assert_eq!(source_paths(&inventory), ["keep.rs"]);
    }

    #[test]
    fn an_anchored_pattern_matches_only_at_its_ignore_files_directory() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join(".gitignore"), "/build.rs\n").unwrap();
        fs::create_dir_all(directory.path().join("sub")).unwrap();
        fs::write(directory.path().join("build.rs"), "").unwrap();
        fs::write(directory.path().join("sub/build.rs"), "").unwrap();
        let inventory = Inventory::discover(directory.path()).unwrap();
        assert_eq!(source_paths(&inventory), ["sub/build.rs"]);
    }

    #[test]
    fn the_repository_exclude_file_is_honored() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join(".git/info")).unwrap();
        fs::write(directory.path().join(".git/info/exclude"), "local.rs\n").unwrap();
        fs::write(directory.path().join("local.rs"), "").unwrap();
        fs::write(directory.path().join("kept.rs"), "").unwrap();
        let inventory = Inventory::discover(directory.path()).unwrap();
        assert_eq!(source_paths(&inventory), ["kept.rs"]);
    }

    #[test]
    fn ancestor_ignore_files_apply_when_a_subpath_is_analyzed() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join(".gitignore"), "secret.rs\n").unwrap();
        fs::create_dir_all(directory.path().join("sub")).unwrap();
        fs::write(directory.path().join("sub/secret.rs"), "").unwrap();
        fs::write(directory.path().join("sub/ok.rs"), "").unwrap();
        let inventory = Inventory::discover(directory.path().join("sub")).unwrap();
        assert_eq!(source_paths(&inventory), ["ok.rs"]);
    }

    #[test]
    fn configuration_patterns_anchor_at_the_analyzed_root() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join("generated")).unwrap();
        fs::create_dir_all(directory.path().join("sub/generated")).unwrap();
        fs::write(directory.path().join("generated/root.rs"), "").unwrap();
        fs::write(directory.path().join("sub/generated/kept.rs"), "").unwrap();
        let inventory =
            Inventory::discover_sources(directory.path(), vec!["/generated".to_owned()]).unwrap();
        assert_eq!(source_paths(&inventory), ["sub/generated/kept.rs"]);
    }

    #[test]
    fn a_configuration_negation_reincludes_a_candidate() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join("docs")).unwrap();
        fs::write(directory.path().join("docs/spec.rs"), "").unwrap();
        fs::write(directory.path().join("docs/other.rs"), "").unwrap();
        let inventory = Inventory::discover_sources(
            directory.path(),
            vec!["docs/**".to_owned(), "!docs/spec.rs".to_owned()],
        )
        .unwrap();
        assert_eq!(source_paths(&inventory), ["docs/spec.rs"]);
    }

    #[test]
    fn dependency_directories_survive_every_negation() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join(".gitignore"), "!node_modules/\n").unwrap();
        fs::create_dir_all(directory.path().join("node_modules")).unwrap();
        fs::write(directory.path().join("node_modules/dep.js"), "").unwrap();
        fs::write(directory.path().join("main.js"), "").unwrap();
        let inventory =
            Inventory::discover_sources(directory.path(), vec!["!node_modules".to_owned()])
                .unwrap();
        assert_eq!(source_paths(&inventory), ["main.js"]);
    }

    fn nested_checkout_paths(inventory: &Inventory) -> Vec<String> {
        inventory
            .nested_checkouts()
            .map(RelativePath::to_string)
            .collect()
    }

    #[test]
    fn an_embedded_clone_with_a_git_directory_is_pruned_and_recorded() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join("sub/clone/.git")).unwrap();
        fs::write(directory.path().join("sub/clone/lost.rs"), "").unwrap();
        fs::write(directory.path().join("kept.rs"), "").unwrap();
        let inventory = Inventory::discover(directory.path()).unwrap();
        assert_eq!(source_paths(&inventory), ["kept.rs"]);
        assert_eq!(nested_checkout_paths(&inventory), ["sub/clone"]);
        assert_eq!(inventory.stats().nested_checkouts_skipped, 1);
    }

    #[test]
    fn a_linked_worktree_with_a_git_file_is_pruned_and_recorded() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join("linked/pr-48")).unwrap();
        fs::write(
            directory.path().join("linked/pr-48/.git"),
            "gitdir: /elsewhere/.git/worktrees/pr-48\n",
        )
        .unwrap();
        fs::write(directory.path().join("linked/pr-48/lost.rs"), "").unwrap();
        fs::write(directory.path().join("kept.rs"), "").unwrap();
        let inventory = Inventory::discover(directory.path()).unwrap();
        assert_eq!(source_paths(&inventory), ["kept.rs"]);
        assert_eq!(nested_checkout_paths(&inventory), ["linked/pr-48"]);
        assert_eq!(inventory.stats().nested_checkouts_skipped, 1);
    }

    #[test]
    fn the_analyzed_root_with_its_own_git_entry_is_never_pruned() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join(".git")).unwrap();
        fs::write(directory.path().join("main.rs"), "").unwrap();
        let inventory = Inventory::discover(directory.path()).unwrap();
        assert_eq!(source_paths(&inventory), ["main.rs"]);
        assert_eq!(nested_checkout_paths(&inventory), Vec::<String>::new());
        assert_eq!(inventory.stats().nested_checkouts_skipped, 0);
    }

    #[test]
    fn discovered_files_state_their_size_from_walk_metadata() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("main.rs"), "fn main() {}\n").unwrap();
        let inventory = Inventory::discover(directory.path()).unwrap();
        let file = inventory.source_files().next().unwrap();
        assert_eq!(file.size_bytes(), 13);
    }

    #[test]
    fn two_walks_of_the_same_tree_yield_identical_order() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir_all(directory.path().join("b")).unwrap();
        fs::write(directory.path().join("b/late.rs"), "").unwrap();
        fs::write(directory.path().join("a.rs"), "").unwrap();
        fs::write(directory.path().join("z.rs"), "").unwrap();
        let first = Inventory::discover(directory.path()).unwrap();
        let second = Inventory::discover(directory.path()).unwrap();
        assert_eq!(first.files(), second.files());
        assert_eq!(source_paths(&first), ["a.rs", "b/late.rs", "z.rs"]);
    }

    #[test]
    fn one_star_does_not_cross_a_directory_separator() {
        assert!(glob_matches("foo/*", "foo/file.rs"));
        assert!(!glob_matches("foo/*", "foo/deep/file.rs"));
        assert!(glob_matches("foo/**", "foo/deep/file.rs"));
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_directory_stays_visible_as_a_diagnostic() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let private = directory.path().join("private");
        fs::create_dir(&private).unwrap();
        fs::write(private.join("hidden.rs"), "fn hidden() {}\n").unwrap();
        fs::set_permissions(&private, fs::Permissions::from_mode(0o000)).unwrap();
        let inventory = Inventory::discover(directory.path()).unwrap();
        fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(inventory.diagnostics().iter().any(|diagnostic| matches!(
            diagnostic,
            InventoryDiagnostic::UnreadableDirectory { path, .. }
                if path.as_path() == Path::new("private")
        )));
    }
}
