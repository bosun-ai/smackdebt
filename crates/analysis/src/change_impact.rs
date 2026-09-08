//! Static change impact measured through transitive dependants.
//!
//! This is potential dependency reach, not a prediction that every dependant will
//! need an edit. For `A -> B -> C`, a change to C can reach A and B. Repository-wide
//! [`FileReach`] excludes C itself; [`PropagationReach`] and package closures
//! include the changed node, so the corresponding closure count is three.
//!
//! Package-local closure is computed for packages with 20 through 4,096 eligible
//! files. Larger packages are recorded as skipped. Repository package reach needs
//! three packages and at least two reached nodes. Repository-wide file reach
//! selects at most 64 cycle or hub candidates, ordered by fan-in then path before
//! selection; completed rows use stable file order. Missing rows are not zeros.
//!
//! Only trusted primary files enter the file graph. Reach facts are descriptive;
//! problem policy consumes their evidence. Graph traversal and scratch storage
//! stay in the shared reachability and path-probe helpers.
//!
//! Terminology: [static impact analysis](https://www.ndepend.com/docs/refactoring-impact-analysis).

#![deny(missing_docs)]

use crate::dependency_degree::dependency_degree;
use crate::path_probe::PathProbe;
use crate::problem::HUB_DEGREE;
use crate::reachability::reach_in_counts;
use crate::report::{FileRecord, PackageId};
use crate::source::{SourceRole, SourceTrust};
use crate::{ComparisonDirection, FileId};
use std::cmp::Reverse;
use std::collections::BTreeSet;

/// The most files one package closure may hold.
///
/// The closure allocates one bit set per live component of the package it
/// closes over, so the node count is what bounds its transient memory. A
/// package above this limit is skipped and the skip is disclosed, because an
/// optional descriptive fact must never be the reason a report grows without
/// bound.
pub const CLOSURE_NODE_LIMIT: usize = 4_096;

/// The files a package holds before its file reach is worth stating.
///
/// A small package reaches its own handful of files, which tells a reader
/// nothing they cannot see.
pub const PACKAGE_REACH_FILES: u32 = 20;

/// The packages a repository holds before its package reach is worth stating.
pub const ROOT_REACH_PACKAGES: u32 = 3;

/// The packages one package's change must reach before the fact is worth
/// stating, counting the changed package itself.
pub const ROOT_REACH_REACHED: u32 = 2;

/// How far a change to one file of a package travels inside that package.
///
/// The row exists only for a package whose value is material, so the table is
/// never a complete package index: a consumer joins it by package rather than
/// by position.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PackageClosure {
    package: PackageId,
    source: crate::FileId,
    files: u32,
    reach: u32,
}

/// One file's reach inside its package, retained for identity-safe diffing.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PackageFileReach {
    package: PackageId,
    source: crate::FileId,
    files: u32,
    reach: u32,
}

impl PackageFileReach {
    /// The package whose graph owns this value.
    pub const fn package(self) -> PackageId {
        self.package
    }
    /// The file whose dependants were counted.
    pub const fn source(self) -> crate::FileId {
        self.source
    }
    /// The number of graph files in the package.
    pub const fn files(self) -> u32 {
        self.files
    }
    /// The number of package files reached from this source.
    pub const fn reach(self) -> u32 {
        self.reach
    }
}

impl PackageClosure {
    /// The package this closure answers for.
    pub const fn package(self) -> PackageId {
        self.package
    }
    /// The stable file whose change has this maximum reach.
    pub const fn source(self) -> crate::FileId {
        self.source
    }
    /// The files of this package the file dependency graph is built over.
    pub const fn files(self) -> u32 {
        self.files
    }
    /// The largest number of this package's files that transitively depend on
    /// one of its files, counting that file itself.
    pub const fn reach(self) -> u32 {
        self.reach
    }
}

/// Every package's file closure, with the packages the node limit skipped.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackageClosures {
    closures: Vec<PackageClosure>,
    file_reaches: Vec<PackageFileReach>,
    skipped: Vec<PackageId>,
}

impl PackageClosures {
    /// The material rows, in package order.
    pub fn closures(&self) -> &[PackageClosure] {
        &self.closures
    }
    /// Every computed per-file value, retained for stable diff subjects.
    pub fn file_reaches(&self) -> &[PackageFileReach] {
        &self.file_reaches
    }
    /// The packages whose closure the node limit skipped, in package order.
    pub fn skipped(&self) -> &[PackageId] {
        &self.skipped
    }
}

/// Whether a file can carry an edge that enters the file dependency graph.
///
/// The graph is a verdict about the code that ships, so a test file and a file
/// no grammar could read are outside it. Core size divides by these files and
/// a package closure closes over them, so both state a fraction of one
/// population.
pub fn enters_file_graph(file: &FileRecord) -> bool {
    matches!(file.role(), SourceRole::Primary) && matches!(file.trust(), SourceTrust::Trusted)
}

/// The files the file dependency graph is built over.
pub fn graph_file_count(files: &[FileRecord]) -> u32 {
    files.iter().filter(|file| enters_file_graph(file)).count() as u32
}

/// Closes over each package's own files, one transient bit set at a time.
///
/// `file_packages` names the package of every file that enters the graph, by
/// file table position, and holds nothing for a file that stays outside it. No
/// package's closure reads a file outside that package, so the aggregate work
/// of one report is the sum over packages of what each package's own file
/// count implies rather than a repository-wide closure.
///
/// A package below the file floor is never closed over, because its value
/// would be immaterial and dropped; a package above the node limit is skipped
/// and named, because a missing limited fact must be disclosed.
pub fn close_over_packages(
    package_count: usize,
    file_packages: &[Option<PackageId>],
    edges: &[(usize, usize)],
) -> PackageClosures {
    let members = package_members(package_count, file_packages);
    let mut closures = Vec::new();
    let mut file_reaches = Vec::new();
    let mut skipped = Vec::new();
    let mut local = vec![usize::MAX; file_packages.len()];
    for (index, files) in members.iter().enumerate() {
        let package = PackageId::from_index(index);
        if (files.len() as u32) < PACKAGE_REACH_FILES {
            continue;
        }
        if files.len() > CLOSURE_NODE_LIMIT {
            skipped.push(package);
            continue;
        }
        for (position, &file) in files.iter().enumerate() {
            local[file] = position;
        }
        let inside = inside_edges(edges, &local);
        let reaches = reach_in_counts(files.len(), &inside);
        file_reaches.extend(
            files
                .iter()
                .zip(&reaches)
                .map(|(&source, &reach)| PackageFileReach {
                    package,
                    source: crate::FileId::from_index(source),
                    files: files.len() as u32,
                    reach,
                }),
        );
        let (source, reach) = reaches
            .into_iter()
            .enumerate()
            .max_by(|left, right| left.1.cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
            .map_or((0, 0), |(source, reach)| (files[source], reach));
        closures.push(PackageClosure {
            package,
            source: crate::FileId::from_index(source),
            files: files.len() as u32,
            reach,
        });
        for &file in files {
            local[file] = usize::MAX;
        }
    }
    PackageClosures {
        closures,
        file_reaches,
        skipped,
    }
}

/// The graph files of each package, in file table order.
fn package_members(package_count: usize, file_packages: &[Option<PackageId>]) -> Vec<Vec<usize>> {
    let mut members = vec![Vec::new(); package_count];
    for (file, package) in file_packages.iter().enumerate() {
        if let Some(package) = package
            && package.index() < package_count
        {
            members[package.index()].push(file);
        }
    }
    members
}

/// The edges whose two ends are both inside the package being closed over,
/// renumbered to that package's own node indexes.
fn inside_edges(edges: &[(usize, usize)], local: &[usize]) -> Vec<(usize, usize)> {
    edges
        .iter()
        .filter_map(|&(source, target)| {
            let source = *local.get(source)?;
            let target = *local.get(target)?;
            (source != usize::MAX && target != usize::MAX).then_some((source, target))
        })
        .collect()
}

/// The most files that carry an exact repository-wide reach.
///
/// Each candidate costs one reverse breadth-first search over the whole file
/// graph, so the candidate count is the bound.
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
    /// Retains an exact count of other files that transitively depend on this file.
    pub const fn new(file: FileId, reach: u32) -> Self {
        Self { file, reach }
    }
    /// The file-table identity this observation describes.
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

/// The file or package whose transitive dependants a comparison counts.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PropagationSubject {
    /// A package whose change may reach other packages.
    Package {
        /// The changed package whose dependants are counted.
        source: PackageId,
    },
    /// A file whose change may reach other files in its package.
    File {
        /// The package that owns the file graph.
        package: PackageId,
        /// The changed file whose dependants are counted.
        source: FileId,
    },
}

/// Before/after change-impact counts for one stable subject.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PropagationComparison {
    id: PropagationComparisonId,
    subject: PropagationSubject,
    direction: ComparisonDirection,
    before_reached: u32,
    before_total: u32,
    after_reached: u32,
    after_total: u32,
}

impl PropagationComparison {
    /// Retains each side's (reached nodes, graph nodes) for the same subject.
    pub const fn new(
        id: PropagationComparisonId,
        subject: PropagationSubject,
        direction: ComparisonDirection,
        before: (u32, u32),
        after: (u32, u32),
    ) -> Self {
        Self {
            id,
            subject,
            direction,
            before_reached: before.0,
            before_total: before.1,
            after_reached: after.0,
            after_total: after.1,
        }
    }
    /// The row's typed position in its owning report table.
    pub const fn id(self) -> PropagationComparisonId {
        self.id
    }
    /// The subject whose measurement this value records.
    pub const fn subject(self) -> PropagationSubject {
        self.subject
    }
    /// Whether the comparison represents worse, better, or neutral debt movement.
    pub const fn direction(self) -> ComparisonDirection {
        self.direction
    }
    /// The (reached nodes, graph nodes) before the change.
    pub const fn before(self) -> (u32, u32) {
        (self.before_reached, self.before_total)
    }
    /// The (reached nodes, graph nodes) after the change.
    pub const fn after(self) -> (u32, u32) {
        (self.after_reached, self.after_total)
    }
}

crate::table_index::table_index!(
    /// The position of one propagation comparison in its report table.
    PropagationComparisonId
);

/// How far a change to one node of a scope's dependency graph can travel.
///
/// The two forms answer the same question at the two scopes that can answer
/// it: a package's change spreads to packages at the repository root, and a
/// file's change spreads to files inside its own package. Both counts include
/// the changed node, which is why a package reachable from eight others reads
/// "9 of 14". The sentence is copy owned by analysis, so a terminal renderer
/// and a machine consumer print the same bytes; the fact is stated only and
/// never moves the tier, exactly as the repository share never does.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PropagationReach {
    reached: u32,
    total: u32,
    subject: ReachSubject,
}

impl PropagationReach {
    /// Completes the repository's package reach when it is worth stating.
    ///
    /// A repository with fewer than [`ROOT_REACH_PACKAGES`] packages, or whose
    /// most depended-on package is reached by no other, has nothing to say and
    /// carries no fact rather than a one-of-one sentence.
    pub const fn packages(reached: u32, total: u32) -> Option<Self> {
        if total < ROOT_REACH_PACKAGES || reached < ROOT_REACH_REACHED {
            return None;
        }
        Some(Self {
            reached,
            total,
            subject: ReachSubject::Packages,
        })
    }

    /// Completes one package's file reach when the package is large enough for
    /// the fraction to mean anything.
    pub const fn files(reached: u32, total: u32) -> Option<Self> {
        if total < PACKAGE_REACH_FILES {
            return None;
        }
        Some(Self {
            reached,
            total,
            subject: ReachSubject::Files,
        })
    }

    /// The exact sentence every consumer prints for this reach.
    pub fn sentence(self) -> String {
        match self.subject {
            ReachSubject::Packages => format!(
                "A change in one package can reach {} of {} packages.",
                self.reached, self.total
            ),
            ReachSubject::Files => format!(
                "A change here can reach {} of {} files in this package.",
                self.reached, self.total
            ),
        }
    }

    /// The nodes one change reaches, counting the changed node itself.
    pub const fn reached(self) -> u32 {
        self.reached
    }

    /// The nodes the graph this reach was closed over holds.
    pub const fn total(self) -> u32 {
        self.total
    }
}

/// What one reach fact counts, which decides its sentence.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum ReachSubject {
    Packages,
    Files,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::HealthCounts;
    use crate::report::{Coverage, FileId, ScopeId};
    use crate::source::ParseStatus;

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

    fn package_files(count: usize, package: usize) -> Vec<Option<PackageId>> {
        vec![Some(PackageId::from_index(package)); count]
    }

    #[test]
    fn a_package_below_the_file_floor_is_never_closed_over() {
        let chain: Vec<_> = (0..19).map(|node| (node, node + 1)).collect();
        let closures = close_over_packages(1, &package_files(19, 0), &chain);
        assert!(closures.closures().is_empty(), "nineteen files say nothing");
        assert!(closures.skipped().is_empty());
        // One more file and the same shape earns a row.
        let closures = close_over_packages(1, &package_files(20, 0), &chain);
        assert_eq!(closures.closures().len(), 1);
        let row = closures.closures()[0];
        assert_eq!(row.package(), PackageId::from_index(0));
        assert_eq!(row.source(), FileId::from_index(19));
        assert_eq!(row.files(), 20);
        // Nineteen edges chain twenty files, so the last one is reached by all.
        assert_eq!(row.reach(), 20);
    }

    #[test]
    fn a_closure_never_reads_a_file_outside_its_package() {
        let mut file_packages = package_files(20, 0);
        file_packages.extend(package_files(20, 1));
        // Every file of the second package depends on the first file overall,
        // which belongs to the first package and must not be counted for it.
        let mut edges: Vec<_> = (20..40).map(|node| (node, 0)).collect();
        edges.extend((0..19).map(|node| (node, node + 1)));
        let closures = close_over_packages(2, &file_packages, &edges);
        assert_eq!(closures.closures().len(), 2);
        assert_eq!(closures.closures()[0].reach(), 20);
        assert_eq!(
            closures.closures()[1].reach(),
            1,
            "a cross-package edge is outside every package closure"
        );
    }

    #[test]
    fn a_package_above_the_node_limit_is_skipped_and_named() {
        let closures = close_over_packages(1, &package_files(CLOSURE_NODE_LIMIT + 1, 0), &[]);
        assert!(closures.closures().is_empty());
        assert_eq!(closures.skipped(), [PackageId::from_index(0)]);
        // Exactly at the limit the closure still runs.
        let closures = close_over_packages(1, &package_files(CLOSURE_NODE_LIMIT, 0), &[]);
        assert_eq!(closures.closures().len(), 1);
        assert!(closures.skipped().is_empty());
    }

    #[test]
    fn a_file_outside_the_graph_joins_no_package_closure() {
        let mut file_packages = package_files(20, 0);
        file_packages.push(None);
        let closures = close_over_packages(1, &file_packages, &[]);
        assert_eq!(closures.closures()[0].files(), 20);
    }

    #[test]
    fn only_primary_trusted_files_are_graph_files() {
        let mut files = vec![file(0, "src/a.js"), file(1, "src/b.js")];
        files.push(
            FileRecord::new(
                FileId::from_index(2),
                ScopeId::from_index(0),
                "src/c.js",
                Coverage::default(),
                HealthCounts::default(),
            )
            .with_source_state(SourceRole::Test, ParseStatus::Parsed),
        );
        files.push(
            FileRecord::new(
                FileId::from_index(3),
                ScopeId::from_index(0),
                "src/d.kt",
                Coverage::default(),
                HealthCounts::default(),
            )
            .with_source_state(SourceRole::Primary, ParseStatus::Failed),
        );
        assert_eq!(graph_file_count(&files), 2);
        assert!(enters_file_graph(&files[0]));
        assert!(!enters_file_graph(&files[2]));
        assert!(!enters_file_graph(&files[3]));
    }

    #[test]
    fn package_reach_is_material_only_above_both_of_its_floors() {
        assert!(
            PropagationReach::packages(2, 2).is_none(),
            "two packages are not a system"
        );
        assert!(
            PropagationReach::packages(1, 14).is_none(),
            "a package nothing depends on has nothing to say"
        );
        let reach = PropagationReach::packages(9, 14).expect("nine of fourteen is material");
        assert_eq!(reach.reached(), 9);
        assert_eq!(reach.total(), 14);
        assert_eq!(
            reach.sentence(),
            "A change in one package can reach 9 of 14 packages."
        );
        // Exactly at both floors the fact exists.
        assert_eq!(
            PropagationReach::packages(2, 3)
                .expect("both floors are inclusive")
                .sentence(),
            "A change in one package can reach 2 of 3 packages."
        );
    }

    #[test]
    fn file_reach_is_material_only_from_a_package_of_twenty_files() {
        assert!(
            PropagationReach::files(19, 19).is_none(),
            "nineteen files are too few to divide"
        );
        let reach = PropagationReach::files(34, 98).expect("a package of ninety-eight files");
        assert_eq!(reach.reached(), 34);
        assert_eq!(reach.total(), 98);
        assert_eq!(
            reach.sentence(),
            "A change here can reach 34 of 98 files in this package."
        );
        assert_eq!(
            PropagationReach::files(1, 20)
                .expect("the file floor is inclusive")
                .sentence(),
            "A change here can reach 1 of 20 files in this package."
        );
    }
}

#[cfg(test)]
mod file_reach_tests {
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
