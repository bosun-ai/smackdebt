use std::collections::{BTreeMap, BTreeSet};

use crate::{FileHistory, HistoryCommitFact, PackageHistory};

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
    files: BTreeMap<crate::FileId, FileHistory>,
    packages: BTreeMap<crate::PackageId, PackageHistory>,
}

impl ChurnAccumulator {
    pub(crate) fn accept(&mut self, commit: &HistoryCommitFact) {
        let mut touched_files = BTreeSet::new();
        let mut touched_packages = BTreeSet::new();
        for change in commit.changes() {
            let file = self
                .files
                .entry(change.file())
                .or_insert_with(|| FileHistory::new(change.file(), 0, 0, 0, 0));
            *file = FileHistory::new(
                file.file(),
                file.touches(),
                file.added_lines() + u64::from(change.added_lines().unwrap_or(0)),
                file.deleted_lines() + u64::from(change.deleted_lines().unwrap_or(0)),
                file.uncounted_changes()
                    + u32::from(change.added_lines().is_none() || change.deleted_lines().is_none()),
            );
            touched_files.insert(change.file());
            let package = self
                .packages
                .entry(change.package())
                .or_insert_with(|| PackageHistory::new(change.package(), 0, 0, 0, 0));
            *package = PackageHistory::new(
                package.package(),
                package.touches(),
                package.added_lines() + u64::from(change.added_lines().unwrap_or(0)),
                package.deleted_lines() + u64::from(change.deleted_lines().unwrap_or(0)),
                package.uncounted_changes()
                    + u32::from(change.added_lines().is_none() || change.deleted_lines().is_none()),
            );
            touched_packages.insert(change.package());
        }
        for file_id in touched_files {
            let file = self
                .files
                .get_mut(&file_id)
                .expect("a touched file has accumulated churn");
            *file = FileHistory::new(
                file.file(),
                file.touches() + 1,
                file.added_lines(),
                file.deleted_lines(),
                file.uncounted_changes(),
            );
        }
        for package_id in touched_packages {
            let package = self
                .packages
                .entry(package_id)
                .or_insert_with(|| PackageHistory::new(package_id, 0, 0, 0, 0));
            *package = PackageHistory::new(
                package.package(),
                package.touches() + 1,
                package.added_lines(),
                package.deleted_lines(),
                package.uncounted_changes(),
            );
        }
    }
    pub(crate) fn finish(
        self,
        file_count: usize,
        package_count: usize,
    ) -> (Vec<FileHistory>, Vec<PackageHistory>) {
        let files = (0..file_count)
            .map(|index| {
                let id = crate::FileId::from_index(index);
                self.files
                    .get(&id)
                    .copied()
                    .unwrap_or_else(|| FileHistory::new(id, 0, 0, 0, 0))
            })
            .collect();
        let packages = (0..package_count)
            .map(|index| {
                let id = crate::PackageId::from_index(index);
                self.packages
                    .get(&id)
                    .copied()
                    .unwrap_or_else(|| PackageHistory::new(id, 0, 0, 0, 0))
            })
            .collect();
        (files, packages)
    }
}
