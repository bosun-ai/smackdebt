//! Change-coupling findings informed by static dependency evidence.
//!
//! These are Smackdebt-specific heuristics over retained file-pair measurements.
//! Both require directory distance of at least two and five shared commits.
//! The required Jaccard similarity starts at 40%, falls five percentage points
//! per additional directory step, and stops at 20%.
//!
//! A direct dependency whose importer follows changes can indicate a leaking
//! abstraction. Unexpected coupling requires proof of no connection; an exhausted
//! path probe leaves the answer unknown and creates no absence claim. Each rule
//! uses its own graph population and the existing wiring-file exclusions.
//! Findings retain their operands and comparisons retain their file identities.

#![deny(missing_docs)]

use crate::ComparisonDirection;
use crate::{
    ConnectionGraph, FileChangeCoupling, FileChangeCouplingId, FileId, FileRecord, PackageId,
    PathProbe, Rating, ReachAnswer,
};
use std::cmp::Reverse;
use std::collections::BTreeSet;

macro_rules! leakage_index {
    ($(#[$documentation:meta])* $name:ident) => {
        $(#[$documentation])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u32);

        impl $name {
            /// Creates an index from a table position.
            pub const fn from_index(index: usize) -> Self {
                Self(index as u32)
            }

            /// Returns the table position represented by this index.
            pub const fn index(self) -> usize {
                self.0 as usize
            }

            /// Returns the compact integer representation.
            pub const fn get(self) -> u32 {
                self.0
            }
        }
    };
}

leakage_index!(
    /// The identity of one change-leakage finding, which is its position in
    /// the report's change-leakage finding table.
    ChangeLeakageFindingId
);

/// The filenames whose content is a module's wiring rather than its behavior.
///
/// Each is the name a language gives to a re-export surface: a Rust crate root
/// or module root, a plain JavaScript or TypeScript barrel, a Python package
/// initializer. A file with one of these names is a list of declarations and
/// re-exports, so it holds no abstraction that could leak.
///
/// This is deliberately narrower than the accepted entry filename list the
/// orphan rule publishes, in two directions. That list names program entry
/// points — `main.rs`, `main.py`, `main.rb`, `build.rs`, `setup.py`,
/// `__main__.py` — which hold behavior like any other file. It also names
/// `index.vue`, `index.jsx`, and `index.tsx`, which are not barrels: an
/// `index.vue` is a directory's component implementation and a JSX or TSX index
/// is usually an application bootstrap. Excluding those would silence exactly
/// the component leakage this rule exists to name, so all of them stay
/// eligible and their importers following their changes remains a claim worth
/// making.
pub const WIRING_FILENAMES: &[&str] = &[
    "__init__.py",
    "index.cjs",
    "index.js",
    "index.mjs",
    "index.ts",
    "lib.rs",
    "mod.rs",
];

/// Whether a repository-relative path names a module's wiring.
pub fn is_wiring_filename(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    WIRING_FILENAMES.contains(&name)
}

/// The directories two files sit apart before either rule may name them.
pub const LEAKAGE_MIN_DISTANCE: u32 = 2;

/// The commits a pair must share before either rule may name it.
pub const LEAKAGE_SHARED_COMMITS: u32 = 5;

/// The share of its union, in permille, a pair at the minimum distance must
/// reach.
pub const LEAKAGE_SIMILARITY_PERMILLE: u32 = 400;

/// The permille the similarity bar falls for each directory beyond the
/// minimum distance.
pub const LEAKAGE_SIMILARITY_STEP_PERMILLE: u32 = 50;

/// The share of its union, in permille, no distance lowers the bar below.
pub const LEAKAGE_SIMILARITY_FLOOR_PERMILLE: u32 = 200;

/// The nodes one path probe may visit before it answers undecided.
pub const PATH_PROBE_NODES: usize = 4_096;

/// The share of its union a pair that many directories apart must reach.
///
/// Distance buys severity by lowering the similarity bar: two files far apart
/// that change together at all is a stronger statement than two neighbours
/// that change together often.
pub const fn required_permille(distance: u32) -> u32 {
    let steps = distance.saturating_sub(LEAKAGE_MIN_DISTANCE);
    let bar = LEAKAGE_SIMILARITY_PERMILLE
        .saturating_sub(steps.saturating_mul(LEAKAGE_SIMILARITY_STEP_PERMILLE));
    if bar < LEAKAGE_SIMILARITY_FLOOR_PERMILLE {
        LEAKAGE_SIMILARITY_FLOOR_PERMILLE
    } else {
        bar
    }
}

/// What one change-leakage finding says about the pair it was decided from.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ChangeLeakageKind {
    /// The importers of one file follow its changes.
    LeakyInterface,
    /// Two files change together with no dependency either way.
    HiddenCoupling,
}

impl ChangeLeakageKind {
    /// The frozen machine id, which no renderer may rename or compose.
    pub const fn id(self) -> &'static str {
        match self {
            Self::LeakyInterface => "leaky_interface",
            Self::HiddenCoupling => "hidden_coupling",
        }
    }
}

/// One statement about a retained pair, which the join decided and no
/// measurement produced.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChangeLeakageFinding {
    kind: ChangeLeakageKind,
    coupling: FileChangeCouplingId,
    interface: Option<FileId>,
}

impl ChangeLeakageFinding {
    /// A finding naming the file whose importers follow it.
    pub const fn leaky(coupling: FileChangeCouplingId, interface: FileId) -> Self {
        Self {
            kind: ChangeLeakageKind::LeakyInterface,
            coupling,
            interface: Some(interface),
        }
    }

    /// A finding naming a pair no dependency explains.
    pub const fn hidden(coupling: FileChangeCouplingId) -> Self {
        Self {
            kind: ChangeLeakageKind::HiddenCoupling,
            coupling,
            interface: None,
        }
    }

    /// The finding or movement category represented by this row.
    pub const fn kind(self) -> ChangeLeakageKind {
        self.kind
    }

    /// The retained pair this finding was decided from.
    pub const fn coupling(self) -> FileChangeCouplingId {
        self.coupling
    }

    /// The file whose importers follow it, which only a leaky interface has.
    pub const fn interface(self) -> Option<FileId> {
        self.interface
    }

    /// Every leakage finding is rated Watch, so the card it reaches carries a
    /// word a reader already knows.
    pub const fn rating(self) -> Rating {
        Rating::Watch
    }
}

/// The graphs one join reads: the file dependency cycle graph the leaky rule
/// admits, and the connection graph absence is proved against.
///
/// Both are borrowed and neither is derived here, so the join stays a pure
/// function of tables the report already holds.
pub struct ChangeGraph<'a> {
    imports: BTreeSet<(usize, usize)>,
    connections: &'a ConnectionGraph,
    packages: &'a [Option<PackageId>],
    files: &'a [FileRecord],
}

impl<'a> ChangeGraph<'a> {
    /// Prepares one join over the cycle-graph edges, the connection graph, the
    /// package of every file the file dependency graph holds, and the files
    /// themselves, which decide what a file can be accused of.
    pub fn new(
        cycle_edges: &[(usize, usize)],
        connections: &'a ConnectionGraph,
        packages: &'a [Option<PackageId>],
        files: &'a [FileRecord],
    ) -> Self {
        Self {
            imports: cycle_edges.iter().copied().collect(),
            connections,
            packages,
            files,
        }
    }

    /// Whether a file can be named as an interface whose importers follow it.
    ///
    /// A wiring file holds the module's re-export surface rather than its
    /// behavior: `lib.rs`, `mod.rs`, `index.ts`, and `__init__.py` are lists of
    /// declarations and re-exports. Its importers change with it because adding
    /// an export and using it is one edit, not because an abstraction leaked —
    /// the file has no abstraction of its own to leak. Calibration found this
    /// to be the whole of the rule's real-world output: every leaky finding on
    /// two real repositories named a crate root or a module root, so the rule
    /// was naming a shape that cannot be fixed rather than a design that
    /// should be. The pair keeps its dependency, so it never falls through to
    /// the hidden rule either.
    ///
    /// A program entry point such as `main.rs` is not wiring and stays
    /// eligible, which is why this reads its own list rather than the wider
    /// entry-filename one.
    fn leaks(&self, interface: FileId) -> bool {
        self.files
            .get(interface.index())
            .is_some_and(|file| !is_wiring_filename(file.path()))
    }

    /// Whether one file depends on another through an edge that enters the
    /// file dependency cycle graph.
    fn imports(&self, source: FileId, target: FileId) -> bool {
        self.imports.contains(&(source.index(), target.index()))
    }

    fn package(&self, file: FileId) -> Option<PackageId> {
        self.packages.get(file.index()).copied().flatten()
    }

    /// Whether the graph holds both files a retained pair names.
    ///
    /// A diff's graph may not hold a file its history names: a deleted file
    /// keeps its retained pairs while the current tree keeps no package for
    /// it, and an added file does the same on the base side. Nothing can be
    /// proved about a file the graph does not hold, so such a pair goes
    /// unjudged rather than walked toward a claim.
    fn holds(&self, pair: FileChangeCoupling) -> bool {
        self.package(pair.left()).is_some() && self.package(pair.right()).is_some()
    }

    /// Whether no path connects the two files in either direction over the
    /// connection graph, proved in two stages.
    ///
    /// The package connection matrix answers first and answers only
    /// *separate*. Otherwise a budgeted walk settles it. Either walk proves
    /// the whole claim on its own, because the connection graph holds both
    /// directions of travel: exhausting everything that reaches one file
    /// without meeting the other proves no path joins them either way.
    ///
    /// The walks are not interchangeable in cost, though, because each
    /// explores one file's own side of the graph. A walk that spends its
    /// budget proves nothing about the pair — only that the side it started
    /// from is large — so the other end is asked before the pair is dropped.
    /// A pair is therefore decided whenever its *smaller* side fits the
    /// budget, whichever end that is, and undecided only when both sides
    /// exceed it.
    fn proves_separate(&self, pair: FileChangeCoupling, probe: &mut Option<PathProbe>) -> bool {
        if self
            .connections
            .separates(self.package(pair.left()), self.package(pair.right()))
        {
            return true;
        }
        let (left, right) = (pair.left().index(), pair.right().index());
        let probe = probe.get_or_insert_with(|| self.connections.probe());
        match probe.reaches(left, right, PATH_PROBE_NODES) {
            ReachAnswer::Separate => true,
            ReachAnswer::Reaches => false,
            ReachAnswer::Undecided => {
                probe.reaches(right, left, PATH_PROBE_NODES) == ReachAnswer::Separate
            }
        }
    }
}

/// Joins the retained pairs with the dependency graph into leakage findings.
///
/// This creates no measurement and rates no unit: it decides which
/// already-measured pairs are worth naming. Both rules read the same distance,
/// support, and distance-scaled similarity gates, and differ only in the graph
/// they read and in what they claim about it.
pub fn change_leakage(
    pairs: &[FileChangeCoupling],
    graph: &ChangeGraph<'_>,
) -> Vec<ChangeLeakageFinding> {
    let mut findings = Vec::new();
    let mut probe = None;
    for (position, pair) in pairs
        .iter()
        .enumerate()
        .filter(|(_, pair)| graph.holds(**pair))
    {
        if !qualifies(*pair) {
            continue;
        }
        let coupling = FileChangeCouplingId::from_index(position);
        match (
            graph.imports(pair.left(), pair.right()),
            graph.imports(pair.right(), pair.left()),
        ) {
            // A mutual dependency is a cycle, and the cycle finding already
            // names it.
            (true, true) => {}
            (true, false) if graph.leaks(pair.right()) => {
                findings.push(ChangeLeakageFinding::leaky(coupling, pair.right()));
            }
            (false, true) if graph.leaks(pair.left()) => {
                findings.push(ChangeLeakageFinding::leaky(coupling, pair.left()));
            }
            (true, false) | (false, true) => {}
            (false, false) => {
                if graph.proves_separate(*pair, &mut probe) {
                    findings.push(ChangeLeakageFinding::hidden(coupling));
                }
            }
        }
    }
    findings.sort_by_key(|finding| {
        let pair = pairs[finding.coupling().index()];
        (
            finding.kind(),
            Reverse(pair.distance()),
            Reverse(pair.shared_commits()),
            pair.left(),
            pair.right(),
        )
    });
    findings
}

/// Whether a pair clears the distance, support, and distance-scaled
/// similarity gates both rules share.
fn qualifies(pair: FileChangeCoupling) -> bool {
    pair.distance() >= LEAKAGE_MIN_DISTANCE
        && pair.shared_commits() >= LEAKAGE_SHARED_COMMITS
        && u64::from(pair.shared_commits()) * 1_000
            >= u64::from(pair.union_commits()) * u64::from(required_permille(pair.distance()))
}

/// Introduced or removed leakage evidence for a stable file pair.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChangeLeakageComparison {
    id: ChangeLeakageComparisonId,
    kind: crate::ChangeLeakageKind,
    left: FileId,
    right: FileId,
    direction: ComparisonDirection,
}

impl ChangeLeakageComparison {
    /// Retains the classified leakage movement and its file pair.
    pub const fn new(
        id: ChangeLeakageComparisonId,
        kind: crate::ChangeLeakageKind,
        left: FileId,
        right: FileId,
        direction: ComparisonDirection,
    ) -> Self {
        Self {
            id,
            kind,
            left,
            right,
            direction,
        }
    }
    /// The row's typed position in its owning report table.
    pub const fn id(self) -> ChangeLeakageComparisonId {
        self.id
    }
    /// The finding or movement category represented by this row.
    pub const fn kind(self) -> crate::ChangeLeakageKind {
        self.kind
    }
    /// The first subject in the retained pair.
    pub const fn left(self) -> FileId {
        self.left
    }
    /// The second subject in the retained pair.
    pub const fn right(self) -> FileId {
        self.right
    }
    /// Whether the comparison represents worse, better, or neutral debt movement.
    pub const fn direction(self) -> ComparisonDirection {
        self.direction
    }
}

crate::table_index::table_index!(
    /// The position of one change leakage comparison in its report table.
    ChangeLeakageComparisonId
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::HealthCounts;
    use crate::report::{Coverage, FileRecord, ScopeId};

    fn file(index: usize, path: &str) -> FileRecord {
        FileRecord::new(
            FileId::from_index(index),
            ScopeId::from_index(0),
            path,
            Coverage::default(),
            HealthCounts::default(),
        )
    }

    /// A qualifying pair between two packaged, unconnected files.
    fn qualifying_pair() -> FileChangeCoupling {
        FileChangeCoupling::new(FileId::from_index(0), FileId::from_index(1), 6, 6, 2)
    }

    /// A diff can retain a pair whose file the current graph no longer holds,
    /// such as a hot file the change deleted. The rule proves absence rather
    /// than assuming it, so a pair it cannot prove anything about produces
    /// nothing instead of a claim over a missing file.
    #[test]
    fn a_pair_naming_a_file_outside_the_graph_is_not_judged() {
        let files = [file(0, "left/kept.rs"), file(1, "right/deleted.rs")];
        let packages = [Some(PackageId::from_index(0)), None];
        let connections = ConnectionGraph::new(2, 2, &[], &packages);
        let graph = ChangeGraph::new(&[], &connections, &packages, &files);
        assert!(change_leakage(&[qualifying_pair()], &graph).is_empty());
    }

    /// The same pair between two held files keeps its hidden-coupling claim,
    /// so the guard withholds only what cannot be proved.
    #[test]
    fn the_same_pair_between_held_files_still_earns_its_finding() {
        let files = [file(0, "left/kept.rs"), file(1, "right/kept.rs")];
        let packages = [
            Some(PackageId::from_index(0)),
            Some(PackageId::from_index(1)),
        ];
        let connections = ConnectionGraph::new(2, 2, &[], &packages);
        let graph = ChangeGraph::new(&[], &connections, &packages, &files);
        let findings = change_leakage(&[qualifying_pair()], &graph);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind(), ChangeLeakageKind::HiddenCoupling);
    }
}
