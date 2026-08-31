//! The leakage detectors at every threshold, over the public surface.
//!
//! These read the join the way a caller does — a borrowed pair table and a
//! borrowed graph — because the rule under test is which pair becomes a
//! finding, and that is what a consumer sees. They live beside the crate
//! rather than inside the module so the module stays one screen of rules.

use smackdebt_analysis::{
    ChangeGraph, ChangeLeakageKind, ConnectionGraph, Coverage, FileChangeCoupling, FileId,
    FileRecord, HealthCounts, LEAKAGE_MIN_DISTANCE, LEAKAGE_SHARED_COMMITS, PATH_PROBE_NODES,
    PackageId, Rating, ScopeId, WIRING_FILENAMES, change_leakage, required_permille,
};

/// One retained pair, named the way the accumulator stores it.
fn pair(left: usize, right: usize, shared: u32, union: u32, distance: u32) -> FileChangeCoupling {
    FileChangeCoupling::new(
        FileId::from_index(left),
        FileId::from_index(right),
        shared,
        union,
        distance,
    )
}

/// One file, named so that nothing about its name changes a rule.
fn file(index: usize, path: &str) -> FileRecord {
    FileRecord::new(
        FileId::from_index(index),
        ScopeId::from_index(0),
        path,
        Coverage::default(),
        HealthCounts::default(),
    )
    .with_package(PackageId::from_index(0))
}

/// The two graphs one join reads, owned so a test can borrow them.
struct Graphs {
    cycle: Vec<(usize, usize)>,
    connections: ConnectionGraph,
    packages: Vec<Option<PackageId>>,
    files: Vec<FileRecord>,
}

impl Graphs {
    /// A graph over `files` files, where every file belongs to the package
    /// `packages` names for it, `cycle` lists the imports that enter the file
    /// dependency cycle graph, and `connection` lists the relations the
    /// connection graph admits.
    fn new(
        files: usize,
        packages: &[usize],
        cycle: &[(usize, usize)],
        connection: &[(usize, usize)],
    ) -> Self {
        let package_count = packages.iter().max().map_or(0, |package| package + 1);
        let records = (0..files)
            .map(|index| file(index, &format!("area{index}/unit{index}.js")))
            .collect();
        let packages: Vec<Option<PackageId>> = packages
            .iter()
            .map(|package| Some(PackageId::from_index(*package)))
            .collect();
        Self {
            cycle: cycle.to_vec(),
            connections: ConnectionGraph::new(files, package_count, connection, &packages),
            packages,
            files: records,
        }
    }

    /// One graph of `files` files in one package, whose only relations are the
    /// imports it names, which every graph admits.
    fn imports(files: usize, edges: &[(usize, usize)]) -> Self {
        Self::new(files, &vec![0; files], edges, edges)
    }

    /// Renames one file, which is how a test states what a name means.
    fn named(mut self, index: usize, path: &str) -> Self {
        self.files[index] = file(index, path);
        self
    }

    fn graph(&self) -> ChangeGraph<'_> {
        ChangeGraph::new(&self.cycle, &self.connections, &self.packages, &self.files)
    }
}

/// The kind and interface of each finding, in table order.
fn found(pairs: &[FileChangeCoupling], graphs: &Graphs) -> Vec<(ChangeLeakageKind, Option<usize>)> {
    change_leakage(pairs, &graphs.graph())
        .iter()
        .map(|finding| (finding.kind(), finding.interface().map(FileId::index)))
        .collect()
}

/// A pair `b` imports across three directories, which shares seven of twelve
/// commits, names `a` as the interface it follows.
#[test]
fn an_importer_that_follows_its_interface_is_named_from_the_import_direction() {
    let graphs = Graphs::imports(2, &[(1, 0)]);
    assert_eq!(
        found(&[pair(0, 1, 7, 12, 3)], &graphs),
        [(ChangeLeakageKind::LeakyInterface, Some(0))]
    );
    // The same pair with the edge the other way names the other file, because
    // the direction is what the pattern is about.
    let reversed = Graphs::imports(2, &[(0, 1)]);
    assert_eq!(
        found(&[pair(0, 1, 7, 12, 3)], &reversed),
        [(ChangeLeakageKind::LeakyInterface, Some(1))]
    );
}

/// Two files that import each other are a cycle, which the cycle finding
/// already names, and they are connected, so neither rule fires.
#[test]
fn a_mutual_import_produces_no_finding_of_either_kind() {
    let graphs = Graphs::imports(2, &[(0, 1), (1, 0)]);
    assert_eq!(found(&[pair(0, 1, 7, 12, 3)], &graphs), []);
}

/// A wiring file is a module's re-export surface rather than its behavior, so
/// importers following it means an export was added, not that an abstraction
/// leaked. The pair keeps its dependency, so it becomes no finding at all.
#[test]
fn a_wiring_file_is_never_named_as_the_interface_whose_importers_follow_it() {
    assert_eq!(
        WIRING_FILENAMES,
        [
            "__init__.py",
            "index.cjs",
            "index.js",
            "index.mjs",
            "index.ts",
            "lib.rs",
            "mod.rs"
        ],
        "the excluded names are the re-export surfaces and nothing else"
    );
    // Every other name here is one the wider entry-filename list would have
    // silenced: a program entry point holds behavior like any other file, an
    // `index.vue` is a directory's own component, and a JSX or TSX index is an
    // application bootstrap. None is a barrel, and a component whose importers
    // follow its changes is exactly what this rule exists to name.
    let behaviour = [
        "work.rs",
        "main.rs",
        "main.py",
        "build.rs",
        "setup.py",
        "__main__.py",
        "index.vue",
        "index.jsx",
        "index.tsx",
    ];
    let leaky: &[(ChangeLeakageKind, Option<usize>)] =
        &[(ChangeLeakageKind::LeakyInterface, Some(0))];
    let cases = WIRING_FILENAMES.iter().map(|name| (*name, &[][..]));
    for (name, expected) in cases.chain(behaviour.iter().map(|name| (*name, leaky))) {
        let graphs = Graphs::imports(2, &[(1, 0)]).named(0, &format!("core/parts/{name}"));
        assert_eq!(found(&[pair(0, 1, 7, 12, 3)], &graphs), expected, "{name}");
    }
    // The follower's name decides nothing, because the claim is about the
    // interface: it is the file accused of leaking.
    let follower = Graphs::imports(2, &[(1, 0)]).named(1, "web/src/index.ts");
    assert_eq!(
        found(&[pair(0, 1, 7, 12, 3)], &follower),
        [(ChangeLeakageKind::LeakyInterface, Some(0))]
    );
}

/// Distance is a floor on both rules: one directory apart says nothing, two
/// says something.
#[test]
fn a_pair_one_directory_apart_is_below_the_distance_floor() {
    assert_eq!(LEAKAGE_MIN_DISTANCE, 2);
    let graphs = Graphs::imports(2, &[(1, 0)]);
    assert_eq!(found(&[pair(0, 1, 7, 12, 1)], &graphs), []);
    assert_eq!(
        found(&[pair(0, 1, 7, 12, 2)], &graphs),
        [(ChangeLeakageKind::LeakyInterface, Some(0))]
    );
}

/// Support is a floor on both rules: four shared commits is an anecdote at any
/// similarity.
#[test]
fn a_pair_one_commit_below_the_support_floor_says_nothing() {
    assert_eq!(LEAKAGE_SHARED_COMMITS, 5);
    let graphs = Graphs::imports(2, &[(1, 0)]);
    assert_eq!(found(&[pair(0, 1, 4, 4, 3)], &graphs), []);
    assert_eq!(
        found(&[pair(0, 1, 5, 5, 3)], &graphs),
        [(ChangeLeakageKind::LeakyInterface, Some(0))]
    );
}

/// Distance buys severity by lowering the similarity bar, one step of fifty
/// permille per directory, and never below the floor.
#[test]
fn the_similarity_bar_falls_one_step_per_directory_to_a_floor_of_two_hundred() {
    assert_eq!(
        (2..=8).map(required_permille).collect::<Vec<_>>(),
        [400, 350, 300, 250, 200, 200, 200]
    );
    let graphs = Graphs::imports(2, &[(1, 0)]);
    let leaky =
        |shared, union, distance| found(&[pair(0, 1, shared, union, distance)], &graphs).len();
    // Two directories apart the bar is exactly four tenths of the union.
    assert_eq!(leaky(6, 15, 2), 1);
    assert_eq!(leaky(5, 13, 2), 0);
    // Three directories apart it is exactly seven twentieths.
    assert_eq!(leaky(7, 20, 3), 1);
    assert_eq!(leaky(6, 18, 3), 0);
    // Six directories apart it is exactly one fifth, and no further step
    // lowers it.
    assert_eq!(leaky(5, 25, 6), 1);
    assert_eq!(leaky(5, 26, 6), 0);
    assert_eq!(leaky(5, 26, 9), 0);
}

/// The scenario the rule was written from: the farther pair states more with
/// less similarity than the nearer one.
#[test]
fn distance_lowers_the_bar_so_the_farther_pair_is_the_finding() {
    let graphs = Graphs::imports(4, &[(1, 0), (3, 2)]);
    // Six of twenty is thirty percent two directories apart, which needs
    // forty; five of twenty is twenty-five percent six directories apart,
    // which needs twenty.
    let pairs = [pair(0, 1, 6, 20, 2), pair(2, 3, 5, 20, 6)];
    assert_eq!(
        found(&pairs, &graphs),
        [(ChangeLeakageKind::LeakyInterface, Some(2))]
    );
}

/// The package connection stage answers first and answers only *separate*, so
/// a pair whose packages cannot reach each other is a finding without a
/// file-level probe.
#[test]
fn two_packages_that_cannot_reach_each_other_settle_the_pair_without_a_probe() {
    // The second file's side of the graph is far larger than one probe's node
    // budget, so a file-level probe could only answer undecided. The package
    // stage answers separate, which is the only answer it may give.
    let files = PATH_PROBE_NODES + 3;
    let mut packages = vec![1; files];
    packages[0] = 0;
    let chain: Vec<(usize, usize)> = (1..files - 1).map(|node| (node, node + 1)).collect();
    let graphs = Graphs::new(files, &packages, &[], &chain);
    assert_eq!(
        found(&[pair(0, 1, 6, 9, 3)], &graphs),
        [(ChangeLeakageKind::HiddenCoupling, None)]
    );
}

/// One chain of `length` nodes hanging off `start`, which is one side of a
/// pair too large for a single walk when it passes the node budget.
fn chain_from(start: usize, length: usize) -> Vec<(usize, usize)> {
    (start..start + length - 1)
        .map(|node| (node, node + 1))
        .collect()
}

/// A pair is decided by whichever of its two sides fits the budget, so the
/// same repository answers the same way whichever file the walk starts from.
#[test]
fn a_pair_is_decided_when_the_smaller_side_of_it_fits_the_budget() {
    // One package, so the package stage decides nothing, and one chain longer
    // than the budget hanging off the pair's *second* file: the walk that
    // explores that side runs out of room, and the walk from the other end
    // exhausts a single node and proves the pair separate.
    let files = PATH_PROBE_NODES + 3;
    let heavy_right = Graphs::new(files, &vec![0; files], &[], &chain_from(1, files - 1));
    assert_eq!(
        found(&[pair(0, 1, 6, 9, 3)], &heavy_right),
        [(ChangeLeakageKind::HiddenCoupling, None)]
    );
    // The mirror repository, with the same chain hanging off the *first* file
    // and the second file alone, states the same finding. A pair is a pair
    // whichever end is large.
    let mut mirrored = vec![(0, 2)];
    mirrored.extend(chain_from(2, files - 2));
    let heavy_left = Graphs::new(files, &vec![0; files], &[], &mirrored);
    assert_eq!(
        found(&[pair(0, 1, 6, 9, 3)], &heavy_left),
        [(ChangeLeakageKind::HiddenCoupling, None)]
    );
    // A short chain inside the budget is settled by the first walk alone.
    let small = Graphs::new(10, &[0; 10], &[], &chain_from(1, 8));
    assert_eq!(
        found(&[pair(0, 1, 6, 9, 3)], &small),
        [(ChangeLeakageKind::HiddenCoupling, None)]
    );
}

/// A pair whose two sides both exceed the budget is never named: absence is
/// proved or it is not claimed, and two exhausted walks prove nothing.
#[test]
fn a_pair_whose_every_side_exceeds_the_budget_creates_no_finding() {
    let side = PATH_PROBE_NODES + 2;
    let files = side * 2;
    let mut chains = chain_from(0, side);
    chains.extend(chain_from(side, side));
    let graphs = Graphs::new(files, &vec![0; files], &[], &chains);
    // File 0 heads one over-budget chain and file `side` heads the other, so
    // neither walk can exhaust the set it explores.
    assert_eq!(found(&[pair(0, side, 6, 9, 3)], &graphs), []);
}

/// A repository of one package is decided by the file probes alone, and a path
/// of any length is a dependency.
#[test]
fn a_path_inside_one_package_leaves_the_pair_connected() {
    let graphs = Graphs::new(4, &[0, 0, 0, 0], &[], &[(0, 2), (2, 3), (3, 1)]);
    assert_eq!(found(&[pair(0, 1, 6, 9, 3)], &graphs), []);
    // Break the middle of that path and the same pair is separate.
    let broken = Graphs::new(4, &[0, 0, 0, 0], &[], &[(0, 2), (3, 1)]);
    assert_eq!(
        found(&[pair(0, 1, 6, 9, 3)], &broken),
        [(ChangeLeakageKind::HiddenCoupling, None)]
    );
}

/// The two rules read deliberately different graphs, and neither admission
/// rule may be substituted for the other.
///
/// A Rust parent and the child it declares are joined by a module-ownership
/// relation the cycle graph excludes and the connection graph admits. The pair
/// is therefore never a leaky interface — the cycle graph sees no import — and
/// never a hidden coupling either, because a declaration is a dependency.
#[test]
fn an_owning_pair_is_outside_the_cycle_graph_and_inside_the_connection_graph() {
    let owning = Graphs::new(2, &[0, 0], &[], &[(0, 1)]);
    assert_eq!(found(&[pair(0, 1, 7, 12, 3)], &owning), []);
    // Two packages joined only by that ownership relation are connected, so
    // the package stage cannot separate a pair that spans them.
    let across = Graphs::new(2, &[0, 1], &[], &[(0, 1)]);
    assert_eq!(found(&[pair(0, 1, 7, 12, 3)], &across), []);
    // The same two files with an import between them are a leaky interface,
    // which is what makes the difference the graphs and not the pair.
    let importing = Graphs::new(2, &[0, 1], &[(1, 0)], &[(1, 0)]);
    assert_eq!(
        found(&[pair(0, 1, 7, 12, 3)], &importing),
        [(ChangeLeakageKind::LeakyInterface, Some(0))]
    );
}

/// The table is ordered by kind, then by distance, then by support, then by
/// the two file identities, so serial and parallel runs read one order.
#[test]
fn the_finding_table_is_ordered_by_kind_then_distance_then_support() {
    // Files 0-5 import each other pairwise; files 6-11 are separate islands.
    let imports = [(1, 0), (3, 2), (5, 4)];
    let graphs = Graphs::new(
        12,
        &[0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6],
        &imports,
        &imports,
    );
    let pairs = [
        pair(0, 1, 6, 6, 2),
        pair(2, 3, 6, 6, 5),
        pair(4, 5, 9, 9, 5),
        pair(6, 7, 6, 6, 4),
        pair(8, 9, 6, 6, 7),
        pair(10, 11, 6, 6, 7),
    ];
    let table: Vec<_> = change_leakage(&pairs, &graphs.graph())
        .iter()
        .map(|finding| finding.coupling().index())
        .collect();
    assert_eq!(
        table,
        [2, 1, 0, 4, 5, 3],
        "leaky before hidden, then distance and support descending, then file order"
    );
}

/// Every finding is rated Watch, which is the rating the card it reaches
/// carries.
#[test]
fn every_leakage_finding_is_rated_watch() {
    let graphs = Graphs::new(4, &[0, 0, 1, 2], &[(1, 0)], &[(1, 0)]);
    let findings = change_leakage(
        &[pair(0, 1, 7, 12, 3), pair(2, 3, 6, 9, 3)],
        &graphs.graph(),
    );
    assert_eq!(findings.len(), 2);
    for finding in &findings {
        assert_eq!(finding.rating(), Rating::Watch);
    }
}
