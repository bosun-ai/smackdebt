use std::collections::{BTreeMap, BTreeSet};

use crate::{FileHistory, HistoryCommitFact, PackageHistory, SourceRole, SourceTrust};

type FileEvidence = (crate::FileId, SourceRole, SourceTrust);
type PackageEvidence = (crate::PackageId, SourceRole, SourceTrust);

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
