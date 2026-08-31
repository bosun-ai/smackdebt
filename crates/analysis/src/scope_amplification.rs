use std::collections::BTreeMap;

use crate::{ChangeAmplification, DirectoryId, DirectoryTree, Scope, ScopeKind};

/// The change amplification of every directory whose value is material.
///
/// A directory whose sample is too short, or whose median is below the floor,
/// has no entry at all rather than a weak one, so a reader of this table never
/// has to know the materiality rule to use it. The table is keyed by directory
/// because that is what the history stream observed; turning it into the answer
/// a scope states is the join below.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DirectoryAmplification(BTreeMap<DirectoryId, ChangeAmplification>);

impl DirectoryAmplification {
    /// The table these material facts make, keyed by directory.
    pub(crate) const fn new(facts: BTreeMap<DirectoryId, ChangeAmplification>) -> Self {
        Self(facts)
    }

    /// What a typical change to this directory touches, when the fact is
    /// material.
    ///
    /// A table stating nothing anywhere is the default one, so a caller asking
    /// whether a stream observed anything compares against that rather than
    /// counting rows.
    pub(crate) fn get(&self, directory: DirectoryId) -> Option<ChangeAmplification> {
        self.0.get(&directory).copied()
    }
}

/// The amplification each scope states, by scope position.
///
/// Every scope maps to exactly one directory, stated rather than inferred:
///
/// - a repository scope reads the root directory, which every tree holds;
/// - a package scope reads its own root directory, which is the path its scope
///   is named by — a package rooted at the repository root therefore reads the
///   root histogram and states the same median the repository states, which is
///   one fact stated at two scopes rather than two numbers;
/// - a directory scope reads itself;
/// - a file scope reads nothing, because a per-file histogram would state
///   sample noise as a fact.
///
/// The join is computed once, while the report is composed, so rendering a
/// scope reads one table position rather than walking a tree.
pub fn scope_amplification(
    scopes: &[Scope],
    directories: &DirectoryTree,
    amplification: &DirectoryAmplification,
) -> Vec<Option<ChangeAmplification>> {
    scopes
        .iter()
        .map(|scope| {
            let directory = match scope.kind() {
                ScopeKind::Repository => Some(DirectoryTree::ROOT),
                ScopeKind::Package | ScopeKind::Directory => {
                    directories.directory_of_path(scope.name())
                }
                ScopeKind::File => None,
            };
            directory.and_then(|directory| amplification.get(directory))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ScopeId;

    /// Three directories stating three different medians, so a scope reading
    /// the wrong one is a wrong number rather than the same number.
    fn table(directories: &DirectoryTree) -> DirectoryAmplification {
        let directory = |path| {
            directories
                .directory_of_path(path)
                .expect("the tree holds this directory")
        };
        let fact =
            |median| ChangeAmplification::from_counts(median, 12).expect("a material sample");
        DirectoryAmplification::new(BTreeMap::from([
            (DirectoryTree::ROOT, fact(3)),
            (directory("src"), fact(4)),
            (directory("src/deep"), fact(5)),
        ]))
    }

    #[test]
    fn every_scope_reads_the_one_directory_its_kind_names() {
        let directories = DirectoryTree::from_file_paths(["src/deep/one.js", "src/other.js"]);
        let root = ScopeId::from_index(0);
        let below =
            |index, kind, name| Scope::new(ScopeId::from_index(index), kind, name, Some(root));
        let scopes = [
            Scope::new(root, ScopeKind::Repository, ".", None),
            below(1, ScopeKind::Package, "."),
            below(2, ScopeKind::Package, "src"),
            below(3, ScopeKind::Directory, "src/deep"),
            below(4, ScopeKind::File, "src/deep/one.js"),
            below(5, ScopeKind::Directory, "gone"),
        ];
        let medians: Vec<_> = scope_amplification(&scopes, &directories, &table(&directories))
            .into_iter()
            .map(|amplification| amplification.map(ChangeAmplification::median))
            .collect();
        assert_eq!(
            medians,
            [Some(3), Some(3), Some(4), Some(5), None, None],
            "a package rooted at the repository root states the root's own median, \
             a deeper scope states its own, a file scope states nothing, and a \
             directory the tree never saw states nothing"
        );
    }

    #[test]
    fn a_stream_that_observed_nothing_leaves_every_scope_stating_nothing() {
        let directories = DirectoryTree::from_file_paths(["src/deep/one.js"]);
        let root = Scope::new(ScopeId::from_index(0), ScopeKind::Repository, ".", None);
        let stated = scope_amplification(&[root], &directories, &DirectoryAmplification::default());
        assert_eq!(stated, [None]);
    }
}
