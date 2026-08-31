use crate::dependency_degree::dependency_degree;
use crate::path_probe::PathProbe;
use crate::problem::HUB_DEGREE;
use crate::report::{FileId, FileRecord};
use std::cmp::Reverse;
use std::collections::BTreeSet;

/// The most files that carry an exact repository-wide reach.
///
/// Each candidate costs one reverse breadth-first search over the whole file
/// graph, so the candidate count is the bound.
///
/// This is a proposed constant under review.
pub const REACH_CANDIDATE_LIMIT: usize = 64;

/// The exact repository-wide reach of one candidate file.
///
/// The value excludes the file itself, because the card evidence it exists for
/// states how many other files a change here reaches.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FileReach {
    file: FileId,
    reach: u32,
}

impl FileReach {
    pub const fn new(file: FileId, reach: u32) -> Self {
        Self { file, reach }
    }
    pub const fn file(self) -> FileId {
        self.file
    }
    /// The files that transitively depend on this one, excluding itself.
    pub const fn reach(self) -> u32 {
        self.reach
    }
}

/// The exact repository-wide reach of every candidate file, in file order.
///
/// The candidate set is the members of file dependency cycles together with
/// the files whose degree reaches the hub threshold, ordered by fan-in
/// descending and then by repository-relative path so the cut is the same on
/// every run, and cut at the candidate limit. Each candidate costs one reverse
/// breadth-first search over one shared reverse graph.
pub fn file_reaches(
    files: &[FileRecord],
    components: &[Vec<usize>],
    edges: &[(usize, usize)],
) -> Vec<FileReach> {
    let degrees = dependency_degree(files.len(), edges);
    let candidates = reach_candidates(files, components, &degrees);
    let mut probe = PathProbe::over(files.len(), edges);
    let mut reaches: Vec<_> = candidates
        .into_iter()
        .map(|file| FileReach::new(FileId::from_index(file), probe.dependents(file)))
        .collect();
    reaches.sort_unstable_by_key(|reach| reach.file());
    reaches
}

/// The files a card may state an exact reach for, worst first and cut at the
/// candidate limit.
fn reach_candidates(
    files: &[FileRecord],
    components: &[Vec<usize>],
    degrees: &[(u32, u32)],
) -> Vec<usize> {
    let mut candidates: BTreeSet<usize> = components
        .iter()
        .filter(|component| component.len() > 1)
        .flatten()
        .copied()
        .collect();
    let hubs = (0..files.len())
        .filter(|&file| degrees[file].0 >= HUB_DEGREE || degrees[file].1 >= HUB_DEGREE);
    candidates.extend(hubs);
    let mut ordered: Vec<_> = candidates.into_iter().collect();
    ordered.sort_by_key(|&file| (Reverse(degrees[file].0), files[file].path()));
    ordered.truncate(REACH_CANDIDATE_LIMIT);
    ordered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::HealthCounts;
    use crate::report::{Coverage, PackageId, ScopeId};
    use crate::source::{ParseStatus, SourceRole};
    use crate::strongly_connected_components;

    fn file(index: usize, path: &str) -> FileRecord {
        FileRecord::new(
            FileId::from_index(index),
            ScopeId::from_index(0),
            path,
            Coverage::default(),
            HealthCounts::default(),
        )
        .with_package(PackageId::from_index(0))
        .with_source_state(SourceRole::Primary, ParseStatus::Parsed)
    }

    #[test]
    fn cycle_members_and_hubs_are_the_reach_candidates() {
        // 0 and 1 form a cycle; 2 is imported by eight files; 3 is imported by
        // seven and is neither.
        let files: Vec<_> = (0..20)
            .map(|index| file(index, &format!("src/f{index:02}.js")))
            .collect();
        let mut edges = vec![(0, 1), (1, 0)];
        edges.extend((4..12).map(|node| (node, 2)));
        edges.extend((12..19).map(|node| (node, 3)));
        let components = strongly_connected_components(files.len(), &edges);
        let reaches = file_reaches(&files, &components, &edges);
        let named: Vec<_> = reaches
            .iter()
            .map(|reach| (reach.file().index(), reach.reach()))
            .collect();
        assert_eq!(named, vec![(0, 1), (1, 1), (2, 8)]);
    }

    #[test]
    fn an_exact_reach_excludes_the_file_itself() {
        // Eight files import the first one directly and a ninth imports one of
        // them, so nine files transitively depend on it and it depends on none.
        let files: Vec<_> = (0..10)
            .map(|index| file(index, &format!("src/f{index}.js")))
            .collect();
        let mut edges: Vec<_> = (1..9).map(|node| (node, 0)).collect();
        edges.push((9, 1));
        let components = strongly_connected_components(files.len(), &edges);
        let reaches = file_reaches(&files, &components, &edges);
        assert_eq!(reaches.len(), 1, "only the hub is a candidate");
        assert_eq!(reaches[0].file(), FileId::from_index(0));
        assert_eq!(
            reaches[0].reach(),
            9,
            "the count states the other files, never the file itself"
        );
    }

    #[test]
    fn the_candidate_set_is_cut_at_its_limit_by_fan_in_and_then_path() {
        // A hundred cycles of two files each, so every file is a candidate and
        // every fan-in ties at one: the path decides which survive the cut.
        let files: Vec<_> = (0..200)
            .map(|index| file(index, &format!("src/f{index:03}.js")))
            .collect();
        let edges: Vec<_> = (0..100)
            .flat_map(|pair| [(pair * 2, pair * 2 + 1), (pair * 2 + 1, pair * 2)])
            .collect();
        let components = strongly_connected_components(files.len(), &edges);
        let reaches = file_reaches(&files, &components, &edges);
        assert_eq!(reaches.len(), REACH_CANDIDATE_LIMIT);
        let last = reaches.last().expect("a candidate").file().index();
        assert_eq!(last, REACH_CANDIDATE_LIMIT - 1, "the first paths survive");
    }
}
