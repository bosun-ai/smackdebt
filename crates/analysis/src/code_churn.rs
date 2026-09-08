//! Code churn and change frequency for files and packages.
//!
//! Churn sums added and deleted lines; frequency counts distinct commits touching
//! a subject. Repeated file facts contribute their lines but only one touch per
//! commit. Package frequency counts a commit once even if it touches many files.
//! Counts are partitioned by source role and trust. A change lacking either line
//! count increments `uncounted_changes`, while any known line count is retained.
//!
//! Inputs must already belong to the selected history window. Missing activity
//! produces zero rows for inventory files and packages; history coverage, owned
//! by [`crate::HistoryCoverage`], distinguishes no activity from unavailable data.
//! Churn itself creates no rated finding. Hotspot policy consumes touch counts.

#![deny(missing_docs)]

use crate::{FileId, PackageId};
use crate::{HistoryCommitFact, SourceRole, SourceTrust};
use std::collections::{BTreeMap, BTreeSet};

type FileEvidence = (crate::FileId, SourceRole, SourceTrust);
type PackageEvidence = (crate::PackageId, SourceRole, SourceTrust);

/// Accumulates windowed file and package churn, returning rows in identity and evidence order.
///
/// Files and packages with no observed change receive a zero row. The supplied
/// counts describe the current inventory; commit facts must refer to that inventory.
pub fn churn(
    file_count: usize,
    package_count: usize,
    commits: &[HistoryCommitFact],
) -> (Vec<FileHistory>, Vec<PackageHistory>) {
    let mut accumulator = ChurnAccumulator::default();
    for commit in commits {
        accumulator.accept(commit);
    }
    accumulator.finish(file_count, package_count)
}

#[derive(Default)]
pub(crate) struct ChurnAccumulator {
    files: BTreeMap<FileEvidence, FileHistory>,
    packages: BTreeMap<PackageEvidence, PackageHistory>,
}

impl ChurnAccumulator {
    pub(crate) fn register_source(
        &mut self,
        file: crate::FileId,
        package: crate::PackageId,
        role: SourceRole,
        trust: SourceTrust,
    ) {
        self.files
            .entry((file, role, trust))
            .or_insert_with(|| FileHistory::new(file, 0, 0, 0, 0).with_evidence(role, trust));
        self.packages
            .entry((package, role, trust))
            .or_insert_with(|| PackageHistory::new(package, 0, 0, 0, 0).with_evidence(role, trust));
    }

    pub(crate) fn accept(&mut self, commit: &HistoryCommitFact) {
        let mut touched_files = BTreeSet::new();
        let mut touched_packages = BTreeSet::new();
        for change in commit.changes() {
            let file_key = (change.file(), change.role(), change.trust());
            let file = self.files.entry(file_key).or_insert_with(|| {
                FileHistory::new(change.file(), 0, 0, 0, 0)
                    .with_evidence(change.role(), change.trust())
            });
            *file = FileHistory::new(
                file.file(),
                file.touches(),
                file.added_lines() + u64::from(change.added_lines().unwrap_or(0)),
                file.deleted_lines() + u64::from(change.deleted_lines().unwrap_or(0)),
                file.uncounted_changes()
                    + u32::from(change.added_lines().is_none() || change.deleted_lines().is_none()),
            )
            .with_evidence(file.role(), file.trust());
            touched_files.insert(file_key);
            let package_key = (change.package(), change.role(), change.trust());
            let package = self.packages.entry(package_key).or_insert_with(|| {
                PackageHistory::new(change.package(), 0, 0, 0, 0)
                    .with_evidence(change.role(), change.trust())
            });
            *package = PackageHistory::new(
                package.package(),
                package.touches(),
                package.added_lines() + u64::from(change.added_lines().unwrap_or(0)),
                package.deleted_lines() + u64::from(change.deleted_lines().unwrap_or(0)),
                package.uncounted_changes()
                    + u32::from(change.added_lines().is_none() || change.deleted_lines().is_none()),
            )
            .with_evidence(package.role(), package.trust());
            touched_packages.insert(package_key);
        }
        for file_key in touched_files {
            let file = self
                .files
                .get_mut(&file_key)
                .expect("a touched file has accumulated churn");
            *file = FileHistory::new(
                file.file(),
                file.touches() + 1,
                file.added_lines(),
                file.deleted_lines(),
                file.uncounted_changes(),
            )
            .with_evidence(file.role(), file.trust());
        }
        for package_key in touched_packages {
            let package = self
                .packages
                .get_mut(&package_key)
                .expect("a touched package has accumulated churn");
            *package = PackageHistory::new(
                package.package(),
                package.touches() + 1,
                package.added_lines(),
                package.deleted_lines(),
                package.uncounted_changes(),
            )
            .with_evidence(package.role(), package.trust());
        }
    }
    pub(crate) fn finish(
        self,
        file_count: usize,
        package_count: usize,
    ) -> (Vec<FileHistory>, Vec<PackageHistory>) {
        let mut files = self.files.into_values().collect::<Vec<_>>();
        for index in 0..file_count {
            let id = crate::FileId::from_index(index);
            if !files.iter().any(|history| history.file() == id) {
                files.push(FileHistory::new(id, 0, 0, 0, 0));
            }
        }
        files.sort_by_key(|history| (history.file(), history.role(), history.trust()));
        let mut packages = self.packages.into_values().collect::<Vec<_>>();
        for index in 0..package_count {
            let id = crate::PackageId::from_index(index);
            if !packages.iter().any(|history| history.package() == id) {
                packages.push(PackageHistory::new(id, 0, 0, 0, 0));
            }
        }
        packages.sort_by_key(|history| (history.package(), history.role(), history.trust()));
        (files, packages)
    }
}

/// Non-merge file activity retained for hotspot context.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct FileActivity {
    touches: u32,
}

impl FileActivity {
    /// Retains the distinct commit count observed for a file.
    pub const fn new(touches: u32) -> Self {
        Self { touches }
    }
    /// Distinct commits touching this subject in the selected history window.
    pub const fn touches(self) -> u32 {
        self.touches
    }
}

/// Windowed line churn and distinct commit touches for one file and evidence partition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileHistory {
    file: FileId,
    role: SourceRole,
    trust: SourceTrust,
    touches: u32,
    added_lines: u64,
    deleted_lines: u64,
    uncounted_changes: u32,
}

impl FileHistory {
    /// Retains file churn and touches, initially marked as trusted primary source.
    pub const fn new(
        file: FileId,
        touches: u32,
        added_lines: u64,
        deleted_lines: u64,
        uncounted_changes: u32,
    ) -> Self {
        Self {
            file,
            role: SourceRole::Primary,
            trust: SourceTrust::Trusted,
            touches,
            added_lines,
            deleted_lines,
            uncounted_changes,
        }
    }
    /// Attaches the observed source role and trust without changing the measurements.
    pub const fn with_evidence(mut self, role: SourceRole, trust: SourceTrust) -> Self {
        self.role = role;
        self.trust = trust;
        self
    }
    /// The file-table identity this observation describes.
    pub const fn file(self) -> FileId {
        self.file
    }
    /// The source role of the evidence behind this observation.
    pub const fn role(self) -> SourceRole {
        self.role
    }
    /// Whether the source facts behind this observation are trusted or advisory.
    pub const fn trust(self) -> SourceTrust {
        self.trust
    }
    /// Whether trusted source in a verdict role may contribute to findings.
    pub const fn affects_findings(self) -> bool {
        self.role.affects_verdict() && matches!(self.trust, SourceTrust::Trusted)
    }
    /// Distinct commits touching this subject in the selected history window.
    pub const fn touches(self) -> u32 {
        self.touches
    }
    /// Known added lines summed across the selected history changes.
    pub const fn added_lines(self) -> u64 {
        self.added_lines
    }
    /// Known deleted lines summed across the selected history changes.
    pub const fn deleted_lines(self) -> u64 {
        self.deleted_lines
    }
    /// Changes missing an added-line or deleted-line count, including binary changes.
    pub const fn uncounted_changes(self) -> u32 {
        self.uncounted_changes
    }
}

/// Windowed line churn and distinct commit touches for one package and evidence partition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageHistory {
    package: PackageId,
    role: SourceRole,
    trust: SourceTrust,
    touches: u32,
    added_lines: u64,
    deleted_lines: u64,
    uncounted_changes: u32,
}

impl PackageHistory {
    /// Retains package churn and touches, initially marked as trusted primary source.
    pub const fn new(
        package: PackageId,
        touches: u32,
        added_lines: u64,
        deleted_lines: u64,
        uncounted_changes: u32,
    ) -> Self {
        Self {
            package,
            role: SourceRole::Primary,
            trust: SourceTrust::Trusted,
            touches,
            added_lines,
            deleted_lines,
            uncounted_changes,
        }
    }
    /// Attaches the observed source role and trust without changing the measurements.
    pub const fn with_evidence(mut self, role: SourceRole, trust: SourceTrust) -> Self {
        self.role = role;
        self.trust = trust;
        self
    }
    /// The package-table identity this observation describes.
    pub const fn package(self) -> PackageId {
        self.package
    }
    /// The source role of the evidence behind this observation.
    pub const fn role(self) -> SourceRole {
        self.role
    }
    /// Whether the source facts behind this observation are trusted or advisory.
    pub const fn trust(self) -> SourceTrust {
        self.trust
    }
    /// Whether trusted source in a verdict role may contribute to findings.
    pub const fn affects_findings(self) -> bool {
        self.role.affects_verdict() && matches!(self.trust, SourceTrust::Trusted)
    }
    /// Distinct commits touching this subject in the selected history window.
    pub const fn touches(self) -> u32 {
        self.touches
    }
    /// Known added lines summed across the selected history changes.
    pub const fn added_lines(self) -> u64 {
        self.added_lines
    }
    /// Known deleted lines summed across the selected history changes.
    pub const fn deleted_lines(self) -> u64 {
        self.deleted_lines
    }
    /// Changes missing an added-line or deleted-line count, including binary changes.
    pub const fn uncounted_changes(self) -> u32 {
        self.uncounted_changes
    }
}

#[cfg(test)]
mod tests {
    fn commit(
        contributor: usize,
        changes: &[(usize, usize, Option<u32>, Option<u32>)],
    ) -> crate::HistoryCommitFact {
        crate::HistoryCommitFact::new(
            crate::ContributorId::from_index(contributor),
            changes
                .iter()
                .map(|(file, package, added, deleted)| {
                    crate::HistoryChangeFact::new(
                        crate::FileId::from_index(*file),
                        crate::PackageId::from_index(*package),
                        *added,
                        *deleted,
                    )
                })
                .collect(),
        )
    }

    #[test]
    fn churn_deduplicates_package_touches_and_sums_lines() {
        let commits = vec![
            commit(0, &[(0, 0, Some(3), Some(1)), (1, 0, Some(2), Some(4))]),
            commit(1, &[(0, 0, None, None)]),
        ];
        let (files, packages) = crate::churn(2, 1, &commits);
        assert_eq!(
            (
                files[0].touches(),
                files[0].added_lines(),
                files[0].deleted_lines(),
                files[0].uncounted_changes()
            ),
            (2, 3, 1, 1)
        );
        assert_eq!(
            (
                packages[0].touches(),
                packages[0].added_lines(),
                packages[0].deleted_lines(),
                packages[0].uncounted_changes()
            ),
            (2, 5, 5, 1)
        );
    }

    #[test]
    fn churn_counts_duplicate_file_facts_as_one_commit_touch() {
        let commits = vec![commit(
            0,
            &[(0, 0, Some(3), Some(1)), (0, 0, Some(2), Some(4))],
        )];
        let (files, packages) = crate::churn(1, 1, &commits);
        assert_eq!(files[0].touches(), 1);
        assert_eq!(files[0].added_lines(), 5);
        assert_eq!(files[0].deleted_lines(), 5);
        assert_eq!(packages[0].touches(), 1);
        assert_eq!(packages[0].added_lines(), 5);
        assert_eq!(packages[0].deleted_lines(), 5);
    }
}
