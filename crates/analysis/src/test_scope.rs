use std::collections::{BTreeMap, BTreeSet};

/// One file bringing another into the build, and whether it does so only under
/// a test configuration.
///
/// Both files are plain indexes into the caller's file table.
pub type ModuleDeclaration = (usize, usize, bool);

/// The files a build compiles only when `test` is set.
///
/// A file qualifies when it is declared at least once and every declaration of
/// it is test-scoped, where a declaration is test-scoped if its reference is or
/// if the declaring file already qualifies. The set only grows and is bounded
/// by the declared files, so iterating to a fixpoint over ordered structures
/// terminates and is deterministic — including over a cycle of declarations,
/// which simply never qualifies unless every declaration into it is already
/// test-scoped.
pub fn test_declared_files(declarations: &BTreeSet<ModuleDeclaration>) -> BTreeSet<usize> {
    let mut declared: BTreeMap<usize, Vec<(usize, bool)>> = BTreeMap::new();
    for &(declarer, target, test_scoped) in declarations {
        declared
            .entry(target)
            .or_default()
            .push((declarer, test_scoped));
    }
    let mut test_declared: BTreeSet<usize> = BTreeSet::new();
    loop {
        let mut added = false;
        for (target, sources) in &declared {
            if test_declared.contains(target) {
                continue;
            }
            if sources
                .iter()
                .all(|(declarer, test_scoped)| *test_scoped || test_declared.contains(declarer))
            {
                test_declared.insert(*target);
                added = true;
            }
        }
        if !added {
            return test_declared;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declarations(entries: &[ModuleDeclaration]) -> BTreeSet<ModuleDeclaration> {
        entries.iter().copied().collect()
    }

    #[test]
    fn a_file_declared_only_under_test_qualifies() {
        assert_eq!(
            test_declared_files(&declarations(&[(0, 1, true)])),
            BTreeSet::from([1])
        );
    }

    #[test]
    fn a_file_with_one_production_declaration_does_not_qualify() {
        assert_eq!(
            test_declared_files(&declarations(&[(0, 1, true), (2, 1, false)])),
            BTreeSet::new()
        );
    }

    #[test]
    fn qualification_carries_through_further_declarations() {
        assert_eq!(
            test_declared_files(&declarations(&[(0, 1, true), (1, 2, false), (2, 3, false)])),
            BTreeSet::from([1, 2, 3])
        );
    }

    #[test]
    fn a_declarer_that_never_qualifies_stops_the_chain() {
        assert_eq!(
            test_declared_files(&declarations(&[(0, 1, true), (0, 2, false), (2, 3, false)])),
            BTreeSet::from([1])
        );
    }

    #[test]
    fn a_cycle_of_declarations_terminates() {
        assert_eq!(
            test_declared_files(&declarations(&[(1, 2, false), (2, 1, false)])),
            BTreeSet::new()
        );
    }

    #[test]
    fn a_cycle_a_production_declaration_reaches_terminates_without_qualifying() {
        assert_eq!(
            test_declared_files(&declarations(&[(0, 1, true), (1, 2, false), (2, 1, false)])),
            BTreeSet::new()
        );
    }

    #[test]
    fn a_file_that_is_never_declared_does_not_qualify() {
        assert!(!test_declared_files(&declarations(&[(0, 1, true)])).contains(&0));
    }
}
