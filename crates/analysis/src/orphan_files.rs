//! Orphan files: supported primary source with no incoming graph references.
//!
//! Known entry filenames and explicitly declared entry files are exempt. The
//! caller supplies fan-in from the eligible dependency graph. Results are
//! candidates for inspection: dynamic references may exist outside that graph.
//! Orphan facts are descriptive, never rated, and preserve candidate order.

#![deny(missing_docs)]

use crate::report::FileId;
use crate::source::SourceRole;

/// Conventional entry filenames, which are normally depended on by nothing.
pub const ENTRY_FILENAMES: &[&str] = &[
    "__init__.py",
    "__main__.py",
    "build.rs",
    "index.cjs",
    "index.js",
    "index.jsx",
    "index.mjs",
    "index.ts",
    "index.tsx",
    "index.vue",
    "lib.rs",
    "main.c",
    "main.cpp",
    "main.go",
    "main.py",
    "main.rb",
    "main.rs",
    "mod.rs",
    "setup.py",
];

/// Whether a repository-relative path names a conventional entry file.
pub fn is_entry_filename(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    ENTRY_FILENAMES.contains(&name)
}

/// One file considered for the orphan table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OrphanCandidate<'a> {
    file: FileId,
    path: &'a str,
    role: SourceRole,
    supported: bool,
    fan_in: u32,
    declared_entry: bool,
}

impl<'a> OrphanCandidate<'a> {
    /// Borrows a file's path and retains the eligibility, degree, and entry evidence used by orphan policy.
    pub const fn new(
        file: FileId,
        path: &'a str,
        role: SourceRole,
        supported: bool,
        fan_in: u32,
        declared_entry: bool,
    ) -> Self {
        Self {
            file,
            path,
            role,
            supported,
            fan_in,
            declared_entry,
        }
    }
}

/// A supported primary file that nothing in the verdict graph depends on.
///
/// Orphan facts are descriptive: they are never rated, never create a finding,
/// and never affect a verdict.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OrphanFile {
    file: FileId,
}

impl OrphanFile {
    /// Retains the identity of a file already selected as an orphan candidate.
    pub const fn new(file: FileId) -> Self {
        Self { file }
    }
    /// The file-table identity this observation describes.
    pub const fn file(self) -> FileId {
        self.file
    }
}

/// Derives the orphan table from degree facts the graph already produced.
///
/// Rows keep the candidate order, which is the report's file table order.
pub fn orphan_files(candidates: &[OrphanCandidate<'_>]) -> Vec<OrphanFile> {
    candidates
        .iter()
        .filter(|candidate| {
            candidate.supported
                && candidate.role == SourceRole::Primary
                && candidate.fan_in == 0
                && !candidate.declared_entry
                && !is_entry_filename(candidate.path)
        })
        .map(|candidate| OrphanFile::new(candidate.file))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(index: usize, path: &str, role: SourceRole, fan_in: u32) -> OrphanCandidate<'_> {
        OrphanCandidate::new(FileId::from_index(index), path, role, true, fan_in, false)
    }

    #[test]
    fn a_primary_file_nothing_depends_on_is_an_orphan() {
        let orphans = orphan_files(&[candidate(0, "src/lonely.rs", SourceRole::Primary, 0)]);
        assert_eq!(orphans, [OrphanFile::new(FileId::from_index(0))]);
    }

    #[test]
    fn a_file_with_one_incoming_dependency_is_never_an_orphan() {
        assert!(orphan_files(&[candidate(0, "src/used.rs", SourceRole::Primary, 1)]).is_empty());
    }

    #[test]
    fn an_entry_file_without_incoming_dependencies_is_exempt() {
        for path in ["src/lib.rs", "cmd/main.go"] {
            assert!(orphan_files(&[candidate(0, path, SourceRole::Primary, 0)]).is_empty());
        }
        assert!(orphan_files(&[candidate(0, "app/index.js", SourceRole::Primary, 0)]).is_empty());
        assert!(
            orphan_files(&[OrphanCandidate::new(
                FileId::from_index(0),
                "app/entry.ts",
                SourceRole::Primary,
                true,
                0,
                true,
            )])
            .is_empty()
        );
    }

    #[test]
    fn non_primary_and_unsupported_files_are_never_orphans() {
        assert!(orphan_files(&[candidate(0, "tests/case.rs", SourceRole::Test, 0)]).is_empty());
        assert!(
            orphan_files(&[candidate(0, "spec/fixture.rs", SourceRole::Fixture, 0)]).is_empty()
        );
        assert!(
            orphan_files(&[OrphanCandidate::new(
                FileId::from_index(0),
                "src/thing.kt",
                SourceRole::Primary,
                false,
                0,
                false,
            )])
            .is_empty()
        );
    }
}
