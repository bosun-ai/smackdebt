use crate::reachability::reach_in_counts;
use crate::report::{FileRecord, PackageId};
use crate::source::{SourceRole, SourceTrust};

/// The most files one package closure may hold.
///
/// The closure allocates one bit set per live component of the package it
/// closes over, so the node count is what bounds its transient memory. A
/// package above this limit is skipped and the skip is disclosed, because an
/// optional descriptive fact must never be the reason a report grows without
/// bound.
///
/// This is a proposed constant under review.
pub const CLOSURE_NODE_LIMIT: usize = 4_096;

/// The files a package holds before its file reach is worth stating.
///
/// A small package reaches its own handful of files, which tells a reader
/// nothing they cannot see.
///
/// This is a proposed constant under review.
pub const PACKAGE_REACH_FILES: u32 = 20;

/// The packages a repository holds before its package reach is worth stating.
///
/// This is a proposed constant under review.
pub const ROOT_REACH_PACKAGES: u32 = 3;

/// The packages one package's change must reach before the fact is worth
/// stating, counting the changed package itself.
///
/// This is a proposed constant under review.
pub const ROOT_REACH_REACHED: u32 = 2;

/// The files the largest dependency cycle holds before it is a core.
///
/// This is a proposed constant under review.
pub const CORE_SIZE_FILES: u32 = 5;

/// The percent of the graph's files the largest cycle holds before it is a
/// core, so a cycle that is a rounding error of the codebase stays silent.
///
/// This is a proposed constant under review.
pub const CORE_SIZE_PERCENT: u32 = 2;

/// How far a change to one file of a package travels inside that package.
///
/// The row exists only for a package whose value is material, so the table is
/// never a complete package index: a consumer joins it by package rather than
/// by position.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct PackageClosure {
    package: PackageId,
    files: u32,
    reach: u32,
}

impl PackageClosure {
    /// The package this closure answers for.
    pub const fn package(self) -> PackageId {
        self.package
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
    skipped: Vec<PackageId>,
}

impl PackageClosures {
    /// The material rows, in package order.
    pub fn closures(&self) -> &[PackageClosure] {
        &self.closures
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
/// and named, because a missing bounded fact must be disclosed.
pub fn close_over_packages(
    package_count: usize,
    file_packages: &[Option<PackageId>],
    edges: &[(usize, usize)],
) -> PackageClosures {
    let members = package_members(package_count, file_packages);
    let mut closures = Vec::new();
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
        let reach = reach_in_counts(files.len(), &inside)
            .into_iter()
            .max()
            .unwrap_or(0);
        closures.push(PackageClosure {
            package,
            files: files.len() as u32,
            reach,
        });
        for &file in files {
            local[file] = usize::MAX;
        }
    }
    PackageClosures { closures, skipped }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::architecture::{
        ArchitectureGraph, ArchitectureReportFacts, DependencyCoverage, PackageGraphMeasurement,
    };
    use crate::health::HealthCounts;
    use crate::report::{
        Coverage, FileId, PackageRecord, ReportBuilder, ReportMode, Scope, ScopeId, ScopeKind,
    };
    use crate::source::ParseStatus;
    use crate::verdict::{CoreSize, PropagationReach};

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

    /// One report answers every scope from the tables it was built with, so
    /// the facts land where their scope rule allows them and nowhere else.
    #[test]
    fn the_propagation_facts_reach_the_root_and_a_package_scope_and_nothing_else() {
        let mut builder = ReportBuilder::new(ReportMode::Codebase);
        let id = ScopeId::from_index;
        let mut root = Scope::new(id(0), ScopeKind::Repository, ".", None);
        let mut scopes = Vec::new();
        let mut packages = Vec::new();
        let mut graph = Vec::new();
        for (index, name) in ["app", "core", "web"].into_iter().enumerate() {
            let package = PackageId::from_index(index);
            root.add_child(id(index + 1));
            let mut scope = Scope::new(id(index + 1), ScopeKind::Package, name, Some(id(0)));
            if index == 0 {
                scope.add_child(id(4));
            }
            scopes.push(scope);
            packages.push(PackageRecord::current(package, id(index + 1), name));
            graph.push(PackageGraphMeasurement::new(package, 0, 0).with_reach_in(3 - index as u32));
        }
        scopes.push(Scope::new(
            id(4),
            ScopeKind::Directory,
            "app/src",
            Some(id(1)),
        ));
        builder.add_scope(root);
        for scope in scopes {
            builder.add_scope(scope);
        }
        builder.set_root(id(0));
        builder.set_packages(packages);
        builder.set_architecture(ArchitectureReportFacts::new(
            ArchitectureGraph::new(
                DependencyCoverage::default(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                graph,
            ),
            Vec::new(),
            Vec::new(),
        ));
        // Twenty files chained inside the first package: the last is reached by
        // all twenty, counting itself.
        let chain: Vec<_> = (0..19).map(|node| (node, node + 1)).collect();
        let closures = close_over_packages(3, &package_files(20, 0), &chain);
        builder.set_propagation(
            closures.closures().to_vec(),
            Vec::new(),
            CoreSize::from_counts(34, 210),
        );
        let report = builder.finish();

        let root = report.scope_verdict(id(0));
        assert_eq!(
            root.reach().map(PropagationReach::sentence),
            Some("A change in one package can reach 3 of 3 packages.".to_owned())
        );
        assert_eq!(
            root.core_size().map(CoreSize::sentence),
            Some("34 of 210 files sit in one dependency cycle.".to_owned())
        );
        let package = report.scope_verdict(id(1));
        assert_eq!(
            package.reach().map(PropagationReach::sentence),
            Some("A change here can reach 20 of 20 files in this package.".to_owned())
        );
        assert!(package.core_size().is_none(), "the core is a root fact");
        // A package without a material closure, and a directory, state neither.
        for other in [2, 3, 4] {
            let verdict = report.scope_verdict(id(other));
            assert!(verdict.reach().is_none(), "scope {other} states no reach");
            assert!(
                verdict.core_size().is_none(),
                "scope {other} states no core"
            );
        }
        // Rendering a second time reads the same tables and closes over nothing.
        assert_eq!(report.scope_verdict(id(1)).reach(), package.reach());
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
}
