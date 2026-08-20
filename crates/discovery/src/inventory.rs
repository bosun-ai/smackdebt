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

pub use smackdebt_analysis::PackageId;
use smackdebt_analysis::SourceRole;

#[cfg(test)]
use crate::glob::glob_matches;
use crate::walk::{config_excludes, error_path, source_walk};

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
        let root = root.to_path_buf();
        let excludes = config_excludes(&root, &options.excludes);
        let nested_checkouts = Arc::new(Mutex::new(Vec::new()));
        let mut walker = Walker::new(options);
        for entry in source_walk(&root, excludes, Arc::clone(&nested_checkouts)) {
            match entry {
                Ok(entry) => walker.visit(&entry, &root),
                Err(error) => walker.record_error(&error, &root),
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
    diagnostics: Vec<InventoryDiagnostic>,
    stats: InventoryStats,
}

struct RawFile {
    path: RelativePath,
    kind: FileKind,
}

impl Walker {
    fn new(options: InventoryOptions) -> Self {
        Self {
            options,
            files: Vec::new(),
            manifest_dirs: BTreeMap::new(),
            manifest_names: BTreeMap::new(),
            diagnostics: Vec::new(),
            stats: InventoryStats::default(),
        }
    }

    /// Records one entry the walk yielded.
    ///
    /// The depth-zero root entry only counts as a visited directory; every
    /// deeper entry is classified by its own file type.
    fn visit(&mut self, entry: &ignore::DirEntry, root: &Path) {
        if entry.depth() == 0 {
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
        });
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

    /// Reads the declared name of one recognized manifest.
    ///
    /// The first manifest kind of a directory that declares a usable name owns
    /// the package name.  An unreadable or nameless manifest is not an error.
    fn record_manifest_name(&mut self, parent: &Path, manifest: ManifestKind, absolute: &Path) {
        let entry = self.manifest_names.entry(parent.to_path_buf()).or_default();
        if entry.is_some() {
            return;
        }
        let Ok(source) = fs::read_to_string(absolute) else {
            return;
        };
        *entry = manifest.declared_name(absolute, &source);
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
        fs::create_dir_all(directory.path().join(".worktrees/pr-48")).unwrap();
        fs::write(
            directory.path().join(".worktrees/pr-48/.git"),
            "gitdir: /elsewhere/.git/worktrees/pr-48\n",
        )
        .unwrap();
        fs::write(directory.path().join(".worktrees/pr-48/lost.rs"), "").unwrap();
        fs::write(directory.path().join("kept.rs"), "").unwrap();
        let inventory = Inventory::discover(directory.path()).unwrap();
        assert_eq!(source_paths(&inventory), ["kept.rs"]);
        assert_eq!(nested_checkout_paths(&inventory), [".worktrees/pr-48"]);
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
