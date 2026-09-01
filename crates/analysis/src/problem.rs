use crate::architecture::{
    ArchitectureFinding, ArchitectureFindingId, DependencyEdge, StableDependencyFinding,
    StableDependencyFindingId,
};
use crate::change_leakage::{ChangeLeakageFinding, ChangeLeakageFindingId, ChangeLeakageKind};
use crate::dependency_degree::dependency_degree;
use crate::evolution::{
    EvolutionaryFinding, EvolutionaryFindingId, FileChangeCoupling, KnowledgeConcentrationFinding,
    KnowledgeConcentrationFindingId,
};
use crate::file_reach::FileReach;
use crate::health::Rating;
use crate::hotspot::Hotspot;
use crate::median::nearest_rank_median;
use crate::report::{
    FileActivity, FileId, FileRecord, Finding, FindingId, FindingRank, PackageId, PackageRecord,
};
use crate::size::{SizeFinding, SizeFindingId};
use crate::{GraphEvidence, PackageClosure};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

macro_rules! problem_index {
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

problem_index!(
    /// The identity of one problem card, which is its position in the
    /// report's ranked problem table.
    ProblemId
);

/// The frozen machine vocabulary of problem patterns.
///
/// These ten ids are the stable contract for machine consumers the way tier
/// ids and worst-offender reason ids are. Declaration order is the claiming
/// order, so a card built by an earlier pattern owns the findings a later
/// pattern would otherwise claim.
///
/// The two leakage patterns are appended after `Measured` rather than inserted
/// among the others, because appending at the tail leaves every existing claim,
/// card, and rank position byte-identical: they card only what nothing else
/// named.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProblemPattern {
    Tangle,
    GodFile,
    Hub,
    HotMess,
    ShotgunPair,
    BusRisk,
    UnstableDependency,
    Measured,
    LeakyInterface,
    HiddenCoupling,
}

/// The frozen machine vocabulary, in the claiming order [`ProblemPattern`]
/// declares, so the contract is one list rather than a mapping a reader has to
/// check against the enum.
const PATTERN_IDS: [&str; 10] = [
    "tangle",
    "god_file",
    "hub",
    "hot_mess",
    "shotgun_pair",
    "bus_risk",
    "unstable_dependency",
    "measured",
    "leaky_interface",
    "hidden_coupling",
];

impl ProblemPattern {
    /// The frozen machine id, which no renderer may rename or compose.
    pub const fn id(self) -> &'static str {
        PATTERN_IDS[self.class() as usize]
    }

    /// The position of this pattern in the frozen claiming order, which is
    /// also its rank class.
    ///
    /// Declaration order is claiming order, so the variant's own position
    /// answers and no second list can fall out of step with the first.
    pub const fn class(self) -> u8 {
        self as u8
    }
}

/// What a problem card is about, stored as an index into the table that owns
/// the identity rather than as a copied path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProblemAnchor {
    File(FileId),
    Files(Vec<FileId>),
    Package(PackageId),
    PackagePair(PackageId, PackageId),
}

/// One fact a card states, either a link into an existing finding table or an
/// integer the report already measured.
///
/// Every variant is an index or an integer, so a card can never carry a
/// floating-point value or a copied path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProblemEvidence {
    Finding(FindingId),
    Size(SizeFindingId),
    Architecture(ArchitectureFindingId),
    StableDependency(StableDependencyFindingId),
    Coupling(EvolutionaryFindingId),
    Knowledge(KnowledgeConcentrationFindingId),
    /// One change-leakage finding of the card's file or pair.
    ChangeLeakage(ChangeLeakageFindingId),
    /// Files that depend on the anchor file, over verdict-graph edges.
    FanIn(u32),
    /// Files the anchor file depends on, over verdict-graph edges.
    FanOut(u32),
    /// The touch count of a hot file, or of the hottest member of a file set.
    Hot(u32),
    /// Rated units the anchor file holds.
    RatedUnits(u32),
    /// Files in an anchor file set.
    Members(u32),
    /// Files that transitively depend on the anchor, excluding the anchor
    /// itself, for a file inside the bounded reach candidate set.
    ReachIn(u32),
    /// Importers of a `leaky_interface` card's file that follow its changes,
    /// stored as its own fact so a consumer reads the count without counting
    /// the card's claimed findings.
    Followers(u32),
}

/// The identity of one finding a card claimed.
///
/// Claims are audited across the whole table, so the identity has to name the
/// family as well as the position.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ClaimedFinding {
    Source(FindingId),
    Size(SizeFindingId),
    Architecture(ArchitectureFindingId),
    StableDependency(StableDependencyFindingId),
    Coupling(EvolutionaryFindingId),
    Knowledge(KnowledgeConcentrationFindingId),
    ChangeLeakage(ChangeLeakageFindingId),
}

/// Where a card is shown.
///
/// Visibility is a display filter and never a rank key: a `detail` card is
/// removed from a view rather than moved inside it, so the cards that remain
/// keep the order the problem rank gave them.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProblemVisibility {
    /// Shown wherever problems are shown.
    Default,
    /// Shown only under `--all` or when the card's anchor is the selected
    /// scope, and always present in the machine report.
    Detail,
}

impl ProblemVisibility {
    /// The frozen machine string a report serializes.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Detail => "detail",
        }
    }
}

/// One named problem, clustered from findings the report already holds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProblemCard {
    pattern: ProblemPattern,
    rating: Rating,
    anchor: ProblemAnchor,
    evidence: Vec<ProblemEvidence>,
    claimed_findings: Vec<ClaimedFinding>,
    visibility: ProblemVisibility,
}

impl ProblemCard {
    pub const fn pattern(&self) -> ProblemPattern {
        self.pattern
    }
    pub const fn rating(&self) -> Rating {
        self.rating
    }
    pub const fn anchor(&self) -> &ProblemAnchor {
        &self.anchor
    }
    /// The card's facts in the order a renderer states them, where the first
    /// entry is the card's head.
    pub fn evidence(&self) -> &[ProblemEvidence] {
        &self.evidence
    }
    pub fn claimed_findings(&self) -> &[ClaimedFinding] {
        &self.claimed_findings
    }
    /// Where this card is shown.
    ///
    /// A card is `default` when it claims a finding that affects the verdict
    /// or anchors a rated architecture, coupling, knowledge-concentration, or
    /// stable-dependency finding, and `detail` otherwise.
    pub const fn visibility(&self) -> ProblemVisibility {
        self.visibility
    }
    /// Whether the card's anchor is hot, which it states by carrying a touch
    /// count as evidence.
    pub fn is_hot(&self) -> bool {
        self.evidence
            .iter()
            .any(|fact| matches!(fact, ProblemEvidence::Hot(_)))
    }
}

/// The High findings a file concentrates before it can be a `god_file`.
///
/// This is a proposed constant under review.
pub const CONCENTRATED_HIGH_FINDINGS: u32 = 3;

/// The units rated Watch or High a file holds before one High finding is
/// enough to make it a `god_file`.
///
/// Healthy units are not counted. The arm is about concentrated debt, and a
/// file's rated unit total is its length in units: counting all of them made
/// every long file with a single bug a `god_file`, which is what single-file
/// components produce by the hundred.
///
/// This is a proposed constant under review.
pub const BROAD_DEBT_UNITS: u32 = 6;

/// The fan-out at which a file is broad enough for the `god_file` rule without
/// a size finding.
///
/// This is a proposed constant under review.
pub const GOD_FILE_FAN_OUT: u32 = 10;

/// The file degree a `hub` reaches before its package median is consulted.
///
/// This is a proposed constant under review.
pub const HUB_DEGREE: u32 = 8;

/// The multiple of its package's median degree a `hub` reaches when that
/// median is not zero.
///
/// This is a proposed constant under review.
pub const HUB_MEDIAN_MULTIPLE: u32 = 4;

/// The integer thresholds the file patterns are decided by.
///
/// Every value is an integer and every comparison is an integer comparison, so
/// no file pattern can produce or compare a floating-point value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ProblemPolicy {
    concentrated_high_findings: u32,
    broad_debt_units: u32,
    god_file_fan_out: u32,
    hub_degree: u32,
    hub_median_multiple: u32,
}

impl Default for ProblemPolicy {
    fn default() -> Self {
        Self::new(
            CONCENTRATED_HIGH_FINDINGS,
            BROAD_DEBT_UNITS,
            GOD_FILE_FAN_OUT,
            HUB_DEGREE,
            HUB_MEDIAN_MULTIPLE,
        )
    }
}

impl ProblemPolicy {
    pub const fn new(
        concentrated_high_findings: u32,
        broad_debt_units: u32,
        god_file_fan_out: u32,
        hub_degree: u32,
        hub_median_multiple: u32,
    ) -> Self {
        Self {
            concentrated_high_findings,
            broad_debt_units,
            god_file_fan_out,
            hub_degree,
            hub_median_multiple,
        }
    }

    pub const fn concentrated_high_findings(self) -> u32 {
        self.concentrated_high_findings
    }
    pub const fn broad_debt_units(self) -> u32 {
        self.broad_debt_units
    }
    pub const fn god_file_fan_out(self) -> u32 {
        self.god_file_fan_out
    }
    pub const fn hub_degree(self) -> u32 {
        self.hub_degree
    }
    pub const fn hub_median_multiple(self) -> u32 {
        self.hub_median_multiple
    }
}

/// The complete problem-card display order owned by analysis policy.
///
/// The keys are, in order: rating, claimed High count, hot state, claimed
/// finding count, the frozen pattern order, the accepted finding rank of the
/// card's top claimed source finding, the anchor's repository-relative path,
/// and the anchor's start line. A card with no claimed source finding orders
/// before one that has such a finding when every earlier key ties, because
/// `None` precedes `Some`.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ProblemRank<'a> {
    rating: Reverse<u8>,
    claimed_high: Reverse<u32>,
    hot: Reverse<bool>,
    claimed: Reverse<u32>,
    pattern: u8,
    finding: Option<FindingRank<'a>>,
    path: &'a str,
    start_line: u32,
}

impl<'a> ProblemRank<'a> {
    /// Ranks one card from facts the clustering pass already resolved: how
    /// many of its claimed findings are High, the accepted rank of its top
    /// claimed source finding, and its anchor's path and start line.
    pub fn new(
        card: &ProblemCard,
        claimed_high: u32,
        finding: Option<FindingRank<'a>>,
        path: &'a str,
        start_line: u32,
    ) -> Self {
        Self {
            rating: Reverse(card.rating().rank()),
            claimed_high: Reverse(claimed_high),
            hot: Reverse(card.is_hot()),
            claimed: Reverse(card.claimed_findings().len() as u32),
            pattern: card.pattern().class(),
            finding,
            path,
            start_line,
        }
    }
}

/// The first finding two cards both claim, when the table claims one twice.
///
/// Clustering claims each finding at most once, so a non-empty answer is an
/// index-integrity failure rather than a report fact.
pub fn duplicate_claim(cards: &[ProblemCard]) -> Option<ClaimedFinding> {
    let mut seen = BTreeSet::new();
    cards
        .iter()
        .flat_map(|card| card.claimed_findings().iter().copied())
        .find(|claim| !seen.insert(*claim))
}

/// The borrowed report tables clustering reads.
///
/// Every table is optional so a detector can be exercised over hand-built
/// slices without composing a report.
#[derive(Clone, Copy, Debug)]
pub struct ProblemInput<'a> {
    files: &'a [FileRecord],
    findings: &'a [Finding],
    packages: &'a [PackageRecord],
    size_findings: &'a [SizeFinding],
    architecture_findings: &'a [ArchitectureFinding],
    dependency_edges: &'a [DependencyEdge],
    stable_dependency_findings: &'a [StableDependencyFinding],
    evolutionary_findings: &'a [EvolutionaryFinding],
    knowledge_concentration_findings: &'a [KnowledgeConcentrationFinding],
    hotspots: &'a [Hotspot],
    file_reach: &'a [FileReach],
    package_closures: &'a [PackageClosure],
    graph_evidence: Option<&'a GraphEvidence>,
    change_leakage_findings: &'a [ChangeLeakageFinding],
    file_change_coupling: &'a [FileChangeCoupling],
    policy: ProblemPolicy,
}

impl<'a> ProblemInput<'a> {
    pub const fn new(files: &'a [FileRecord], findings: &'a [Finding]) -> Self {
        Self {
            files,
            findings,
            packages: &[],
            size_findings: &[],
            architecture_findings: &[],
            dependency_edges: &[],
            stable_dependency_findings: &[],
            evolutionary_findings: &[],
            knowledge_concentration_findings: &[],
            hotspots: &[],
            file_reach: &[],
            package_closures: &[],
            graph_evidence: None,
            change_leakage_findings: &[],
            file_change_coupling: &[],
            policy: ProblemPolicy::new(
                CONCENTRATED_HIGH_FINDINGS,
                BROAD_DEBT_UNITS,
                GOD_FILE_FAN_OUT,
                HUB_DEGREE,
                HUB_MEDIAN_MULTIPLE,
            ),
        }
    }

    #[must_use]
    pub const fn with_packages(mut self, packages: &'a [PackageRecord]) -> Self {
        self.packages = packages;
        self
    }

    #[must_use]
    pub const fn with_size_findings(mut self, size_findings: &'a [SizeFinding]) -> Self {
        self.size_findings = size_findings;
        self
    }

    /// Adds the cycle findings and the file dependency edges the file degrees
    /// are counted over.
    #[must_use]
    pub const fn with_architecture(
        mut self,
        architecture_findings: &'a [ArchitectureFinding],
        dependency_edges: &'a [DependencyEdge],
    ) -> Self {
        self.architecture_findings = architecture_findings;
        self.dependency_edges = dependency_edges;
        self
    }

    #[must_use]
    pub const fn with_stable_dependencies(
        mut self,
        findings: &'a [StableDependencyFinding],
    ) -> Self {
        self.stable_dependency_findings = findings;
        self
    }

    #[must_use]
    pub const fn with_evolution(
        mut self,
        coupling: &'a [EvolutionaryFinding],
        concentration: &'a [KnowledgeConcentrationFinding],
    ) -> Self {
        self.evolutionary_findings = coupling;
        self.knowledge_concentration_findings = concentration;
        self
    }

    /// Adds the hotspot table, which is ordered by file table position.
    #[must_use]
    pub const fn with_hotspots(mut self, hotspots: &'a [Hotspot]) -> Self {
        self.hotspots = hotspots;
        self
    }

    /// Adds the exact reach of the candidate files, which is ordered by file
    /// table position.
    #[must_use]
    pub const fn with_file_reach(mut self, file_reach: &'a [FileReach]) -> Self {
        self.file_reach = file_reach;
        self
    }

    #[must_use]
    pub const fn with_propagation(
        mut self,
        package_closures: &'a [PackageClosure],
        graph_evidence: &'a GraphEvidence,
    ) -> Self {
        self.package_closures = package_closures;
        self.graph_evidence = Some(graph_evidence);
        self
    }

    /// Adds the change-leakage findings beside the retained pairs they were
    /// decided from, which a card reads to name the two files of a pair.
    #[must_use]
    pub const fn with_change_leakage(
        mut self,
        findings: &'a [ChangeLeakageFinding],
        pairs: &'a [FileChangeCoupling],
    ) -> Self {
        self.change_leakage_findings = findings;
        self.file_change_coupling = pairs;
        self
    }

    #[must_use]
    pub const fn with_policy(mut self, policy: ProblemPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The exact reach of a file inside the bounded candidate set, when that
    /// reach is worth a line.
    ///
    /// A candidate qualifies on either half of its degree, so a file that
    /// imports many others and is imported by none is a candidate whose reach
    /// is zero. `a change here reaches 0 files` states nothing, and this
    /// product leaves an immaterial fact absent rather than printing it, so
    /// the row exists in the table and the evidence line does not.
    fn exact_reach(&self, file: FileId) -> Option<u32> {
        let package = self.files.get(file.index())?.package()?;
        if self
            .graph_evidence
            .is_some_and(|evidence| !evidence.package_is_complete(package))
        {
            return None;
        }
        self.file_reach
            .binary_search_by_key(&file, |reach| reach.file())
            .ok()
            .map(|position| self.file_reach[position].reach())
            .or_else(|| {
                self.package_closures
                    .iter()
                    .find(|closure| closure.source() == file)
                    .map(|closure| closure.reach().saturating_sub(1))
            })
            .filter(|&reach| reach > 0)
    }

    /// The retained pair one change-leakage finding was decided from.
    fn coupling(&self, finding: &ChangeLeakageFinding) -> Option<FileChangeCoupling> {
        self.file_change_coupling
            .get(finding.coupling().index())
            .copied()
    }

    /// The file a change-leakage finding belongs to for claiming: the interface
    /// whose importers follow it, or the lower-indexed file of a hidden pair.
    ///
    /// The lower-indexed file is a fixed, data-stable choice. Letting either
    /// endpoint claim the finding would make the pair's evidence land under
    /// whichever file happened to carry debt, so the same report would state
    /// the same pair in two different places depending on unrelated facts.
    fn leakage_owner(&self, finding: &ChangeLeakageFinding) -> Option<FileId> {
        match finding.interface() {
            Some(interface) => Some(interface),
            None => self.coupling(finding).map(FileChangeCoupling::left),
        }
    }

    /// The touch count of a hot file.
    fn hot_touches(&self, file: FileId) -> Option<u32> {
        self.hotspots
            .binary_search_by_key(&file, |hotspot| hotspot.file())
            .ok()
            .map(|position| self.hotspots[position].touches())
    }

    fn path(&self, file: FileId) -> &'a str {
        self.files
            .get(file.index())
            .map_or("", |record| record.path())
    }

    fn package_path(&self, package: PackageId) -> &'a str {
        self.packages
            .get(package.index())
            .map_or("", |record| record.path())
    }
}

/// Groups the findings a completed report already holds into ranked problem
/// cards.
///
/// No measurement, rating, finding, verdict, file read, walk, Git process, or
/// parser visit happens here: every value on every card is read from a table
/// the caller passed in. The table is ordered once, so no renderer sorts it.
pub fn cluster_problems(input: ProblemInput<'_>) -> Vec<ProblemCard> {
    let facts = FileFacts::derive(&input);
    let mut cards = Vec::new();
    let mut files = FilePass::new(&input, &facts);
    tangles(&input, &mut cards);
    files.god_files(&mut cards);
    files.hubs(&mut cards);
    files.hot_messes(&mut cards);
    shotgun_pairs(&input, &mut cards);
    bus_risks(&input, &mut cards);
    unstable_dependencies(&input, &mut cards);
    files.measured(&mut cards);
    // The two leakage patterns run last and card only what is left, so a
    // leakage number reaches a reader as one more line on the card that
    // already names its subject wherever such a card exists.
    files.leaky_interfaces(&mut cards);
    files.hidden_couplings(&mut cards);
    rank(&input, cards)
}

/// The per-file facts every file pattern reads, derived once.
struct FileFacts {
    /// Fan-in and fan-out over verdict-graph edges, by file table position.
    degrees: Vec<(u32, u32)>,
    /// The nearest-rank median fan-in and fan-out of each package.
    medians: BTreeMap<Option<PackageId>, (u32, u32)>,
    /// Every retained finding of each file, best first.
    ///
    /// Claiming is blind to role and trust, so advisory and non-primary debt
    /// reaches a card instead of disappearing when no pattern claims it.
    findings: Vec<Vec<FindingId>>,
    /// The size findings of each file, in table order.
    sizes: Vec<Vec<SizeFindingId>>,
    /// The change-leakage findings that belong to each file, in finding-table
    /// order, which is what makes a leakage number one more line on the card
    /// that already names its file.
    leakage: Vec<Vec<ChangeLeakageFindingId>>,
    /// High findings that affect the verdict, which is what the `god_file` and
    /// `hot_mess` rules count.
    high: Vec<u32>,
    /// Whether a file holds at least one finding that affects the verdict,
    /// which is what decides a file card's visibility.
    verdict_affecting: Vec<bool>,
}

impl FileFacts {
    fn derive(input: &ProblemInput<'_>) -> Self {
        let degrees = file_degrees(input);
        let medians = package_medians(input, &degrees);
        let mut findings = vec![Vec::new(); input.files.len()];
        let mut high = vec![0u32; input.files.len()];
        let mut verdict_affecting = vec![false; input.files.len()];
        for finding in input
            .findings
            .iter()
            .filter(|finding| finding.file().index() < input.files.len())
        {
            let file = finding.file().index();
            findings[file].push(finding.id());
            if finding.affects_verdict() {
                verdict_affecting[file] = true;
                if finding.assessment().rating() == Rating::High {
                    high[file] += 1;
                }
            }
        }
        for (index, file) in findings.iter_mut().enumerate() {
            let path = input.files[index].path();
            let hot = input.hot_touches(FileId::from_index(index)).is_some();
            let activity = input.files[index]
                .activity()
                .map_or(0, FileActivity::touches);
            file.sort_by_key(|id| {
                FindingRank::new(&input.findings[id.index()], hot, activity, path)
            });
        }
        let mut sizes = vec![Vec::new(); input.files.len()];
        for (position, size) in input.size_findings.iter().enumerate() {
            if let Some(slot) = sizes.get_mut(size.file().index()) {
                slot.push(SizeFindingId::from_index(position));
            }
        }
        let mut leakage = vec![Vec::new(); input.files.len()];
        for (position, finding) in input.change_leakage_findings.iter().enumerate() {
            if let Some(file) = input.leakage_owner(finding)
                && let Some(slot) = leakage.get_mut(file.index())
            {
                slot.push(ChangeLeakageFindingId::from_index(position));
            }
        }
        Self {
            degrees,
            medians,
            findings,
            sizes,
            leakage,
            high,
            verdict_affecting,
        }
    }

    fn fan_in(&self, index: usize) -> u32 {
        self.degrees[index].0
    }

    fn fan_out(&self, index: usize) -> u32 {
        self.degrees[index].1
    }

    fn median(&self, package: Option<PackageId>) -> (u32, u32) {
        self.medians.get(&package).copied().unwrap_or((0, 0))
    }
}

/// Counts every file's dependency degree over the edges that enter the
/// architecture verdict graph, so test, example, benchmark, module-ownership,
/// and recovered relations contribute nothing.
fn file_degrees(input: &ProblemInput<'_>) -> Vec<(u32, u32)> {
    let edges: Vec<(usize, usize)> = input
        .dependency_edges
        .iter()
        .filter(|edge| edge.enters_verdict_graph())
        .map(|edge| (edge.source().index(), edge.target().index()))
        .collect();
    dependency_degree(input.files.len(), &edges)
}

/// The nearest-rank median fan-in and fan-out of every package, computed once
/// so a file pattern never depends on the selected scope.
fn package_medians(
    input: &ProblemInput<'_>,
    degrees: &[(u32, u32)],
) -> BTreeMap<Option<PackageId>, (u32, u32)> {
    let mut groups: BTreeMap<Option<PackageId>, (Vec<u32>, Vec<u32>)> = BTreeMap::new();
    for (index, file) in input.files.iter().enumerate() {
        let entry = groups.entry(file.package()).or_default();
        entry.0.push(degrees[index].0);
        entry.1.push(degrees[index].1);
    }
    groups
        .into_iter()
        .map(|(package, (mut incoming, mut outgoing))| {
            incoming.sort_unstable();
            outgoing.sort_unstable();
            (
                package,
                (
                    nearest_rank_median(&incoming),
                    nearest_rank_median(&outgoing),
                ),
            )
        })
        .collect()
}

/// One card per architecture finding.
///
/// Analysis already produces exactly one architecture finding per strongly
/// connected component with one stable witness, so this is a grouping of that
/// finding rather than a card per member, per witness step, or per edge. The
/// member files keep their own findings for the file patterns, because being
/// inside a cycle and doing too much are two different problems.
///
/// The witness is stated first, ahead of the member count, because a renderer
/// under a budget shows a prefix of the evidence: the cycle itself is what the
/// card is about, and it survives every rung that allows one line at all.
fn tangles(input: &ProblemInput<'_>, cards: &mut Vec<ProblemCard>) {
    for finding in input.architecture_findings {
        let members = finding.files();
        let mut evidence = vec![
            ProblemEvidence::Architecture(finding.id()),
            ProblemEvidence::Members(members.len() as u32),
        ];
        // Every member of a cycle reaches exactly what the others reach, so the
        // first member that carries a value answers for the whole cycle.
        if let Some(reach) = members.iter().find_map(|&file| input.exact_reach(file)) {
            evidence.push(ProblemEvidence::ReachIn(reach));
        }
        if let Some(touches) = members
            .iter()
            .filter_map(|&file| input.hot_touches(file))
            .max()
        {
            evidence.push(ProblemEvidence::Hot(touches));
        }
        cards.push(ProblemCard {
            pattern: ProblemPattern::Tangle,
            rating: finding.rating(),
            anchor: ProblemAnchor::Files(members.to_vec()),
            evidence,
            claimed_findings: vec![ClaimedFinding::Architecture(finding.id())],
            visibility: ProblemVisibility::Default,
        });
    }
}

/// Whether one degree stands out against its package's median degree.
const fn stands_out(degree: u32, median: u32, policy: ProblemPolicy) -> bool {
    degree >= policy.hub_degree()
        && (median == 0 || degree as u64 >= policy.hub_median_multiple() as u64 * median as u64)
}

/// The file patterns, which share one claim mark per file so at most one
/// file-anchored card exists for a file.
struct FilePass<'a, 'b> {
    input: &'b ProblemInput<'a>,
    facts: &'b FileFacts,
    claimed: Vec<bool>,
}

impl<'a, 'b> FilePass<'a, 'b> {
    fn new(input: &'b ProblemInput<'a>, facts: &'b FileFacts) -> Self {
        Self {
            input,
            facts,
            claimed: vec![false; input.files.len()],
        }
    }

    /// A file that concentrates rated debt and is broad enough for that debt
    /// to mean it does too much.
    ///
    /// Concentrated debt alone is not enough — the breadth conjunct is what
    /// makes the pattern mean "does too much" rather than "has bugs" — and
    /// breadth alone is not enough either.
    ///
    /// The second arm counts the file's units rated Watch or High rather than
    /// every rated unit it holds: a file's rated unit total is its length in
    /// units, so counting all of them named every long file with one bug a
    /// `god_file`. The card still states that total, because the length is
    /// context a reader wants beside the concentration.
    fn god_files(&mut self, cards: &mut Vec<ProblemCard>) {
        let policy = self.input.policy;
        for index in 0..self.input.files.len() {
            if self.claimed[index] {
                continue;
            }
            let high = self.facts.high[index];
            let health = self.input.files[index].health();
            let concentrated = high >= policy.concentrated_high_findings()
                || (high >= 1 && health.debt() >= policy.broad_debt_units());
            let fan_out = self.facts.fan_out(index);
            let broad = !self.facts.sizes[index].is_empty() || fan_out >= policy.god_file_fan_out();
            if !concentrated || !broad {
                continue;
            }
            let mut evidence = vec![ProblemEvidence::RatedUnits(health.total())];
            if fan_out >= policy.god_file_fan_out() {
                evidence.push(ProblemEvidence::FanOut(fan_out));
            }
            let card = self.card(index, ProblemPattern::GodFile, Rating::High, evidence);
            cards.push(card);
        }
    }

    /// A file whose degree stands far above the files it ships with.
    ///
    /// The median is the package's own, so the same file is the same pattern
    /// at every selected scope.
    fn hubs(&mut self, cards: &mut Vec<ProblemCard>) {
        let policy = self.input.policy;
        for index in 0..self.input.files.len() {
            if self.claimed[index] {
                continue;
            }
            let (median_in, median_out) = self.facts.median(self.input.files[index].package());
            let fan_in = self.facts.fan_in(index);
            let fan_out = self.facts.fan_out(index);
            let inbound = stands_out(fan_in, median_in, policy);
            let outbound = stands_out(fan_out, median_out, policy);
            let reach = self.input.exact_reach(FileId::from_index(index));
            let spreads = reach.is_some_and(|reach| reach >= policy.hub_degree());
            if !inbound && !outbound && !spreads {
                continue;
            }
            let mut evidence = Vec::new();
            if inbound {
                evidence.push(ProblemEvidence::FanIn(fan_in));
            }
            if outbound {
                evidence.push(ProblemEvidence::FanOut(fan_out));
            }
            // How far the change spreads is why the degree matters, so it
            // follows the degree that made the pattern fire.
            if let Some(reach) = reach {
                evidence.push(ProblemEvidence::ReachIn(reach));
            }
            let rating = self.claimed_rating(index);
            let card = self.card(index, ProblemPattern::Hub, rating, evidence);
            cards.push(card);
        }
    }

    /// A file that changes often and carries High debt, which agrees with the
    /// accepted `hot_and_complex` worst-offender reason.
    fn hot_messes(&mut self, cards: &mut Vec<ProblemCard>) {
        for index in 0..self.input.files.len() {
            if self.claimed[index] || self.facts.high[index] == 0 {
                continue;
            }
            if self.input.hot_touches(FileId::from_index(index)).is_none() {
                continue;
            }
            let evidence = vec![ProblemEvidence::RatedUnits(
                self.input.files[index].health().total(),
            )];
            let card = self.card(index, ProblemPattern::HotMess, Rating::High, evidence);
            cards.push(card);
        }
    }

    /// The fallback: one card per file that still holds an unclaimed retained
    /// finding or an unclaimed size finding, so nothing the report measured
    /// disappears for want of a pattern that recognises it.
    fn measured(&mut self, cards: &mut Vec<ProblemCard>) {
        for index in 0..self.input.files.len() {
            if self.claimed[index]
                || (self.facts.findings[index].is_empty() && self.facts.sizes[index].is_empty())
            {
                continue;
            }
            let rating = self.claimed_rating(index);
            let card = self.card(index, ProblemPattern::Measured, rating, Vec::new());
            cards.push(card);
        }
    }

    /// One card per interface file that still holds a `leaky_interface`
    /// finding no other pattern claimed.
    ///
    /// The card states how many importers follow the file before it states
    /// them, so the count survives the tightest rung and the followers are the
    /// detail a wider rung buys.
    fn leaky_interfaces(&mut self, cards: &mut Vec<ProblemCard>) {
        for index in 0..self.input.files.len() {
            if self.claimed[index] {
                continue;
            }
            let followers = self.followers(index);
            if followers == 0 {
                continue;
            }
            let rating = self.claimed_rating(index);
            let card = self.card(
                index,
                ProblemPattern::LeakyInterface,
                rating,
                vec![ProblemEvidence::Followers(followers)],
            );
            cards.push(card);
        }
    }

    /// One card per `hidden_coupling` finding whose lower-indexed file carries
    /// no file-anchored card, anchored on the two files the finding is about.
    fn hidden_couplings(&self, cards: &mut Vec<ProblemCard>) {
        for (position, finding) in self.input.change_leakage_findings.iter().enumerate() {
            let Some(pair) = self.input.coupling(finding) else {
                continue;
            };
            if finding.kind() != ChangeLeakageKind::HiddenCoupling
                || self
                    .claimed
                    .get(pair.left().index())
                    .copied()
                    .unwrap_or(false)
            {
                continue;
            }
            let id = ChangeLeakageFindingId::from_index(position);
            cards.push(ProblemCard {
                pattern: ProblemPattern::HiddenCoupling,
                rating: finding.rating(),
                anchor: ProblemAnchor::Files(vec![pair.left(), pair.right()]),
                evidence: vec![ProblemEvidence::ChangeLeakage(id)],
                claimed_findings: vec![ClaimedFinding::ChangeLeakage(id)],
                visibility: ProblemVisibility::Default,
            });
        }
    }

    /// How many importers follow one file, which is how many of its leakage
    /// findings name it as an interface.
    fn followers(&self, index: usize) -> u32 {
        self.facts.leakage[index]
            .iter()
            .filter(|id| {
                self.input.change_leakage_findings[id.index()].kind()
                    == ChangeLeakageKind::LeakyInterface
            })
            .count() as u32
    }

    /// The highest rating among the findings a file card would claim, and
    /// `healthy` when it would claim none.
    fn claimed_rating(&self, index: usize) -> Rating {
        let findings = self.facts.findings[index]
            .iter()
            .map(|id| self.input.findings[id.index()].assessment().rating());
        let sizes = self.facts.sizes[index]
            .iter()
            .map(|id| self.input.size_findings[id.index()].rating());
        // A leakage finding is rated by the capability that owns it, so a file
        // whose only debt is a leakage finding carries that finding's word.
        let leakage = self.facts.leakage[index]
            .iter()
            .map(|id| self.input.change_leakage_findings[id.index()].rating());
        findings
            .chain(sizes)
            .chain(leakage)
            .max()
            .unwrap_or(Rating::Healthy)
    }

    /// Builds one file-anchored card, claiming every retained finding and
    /// every size finding the file holds, whether or not they affect the
    /// verdict.
    ///
    /// Evidence is stated head first: the file's top finding, the facts that
    /// made the pattern fire, the leakage findings it claims, its size
    /// findings, its heat, and only then its remaining findings — so a budgeted
    /// renderer states the problem and its reason before it enumerates, and
    /// `--all` still reaches every claim. A card claiming no source finding
    /// heads on its first other fact, which is its size finding for a file
    /// measured only by its length.
    fn card(
        &mut self,
        index: usize,
        pattern: ProblemPattern,
        rating: Rating,
        pattern_facts: Vec<ProblemEvidence>,
    ) -> ProblemCard {
        self.claimed[index] = true;
        let findings = &self.facts.findings[index];
        let sizes = &self.facts.sizes[index];
        let leakage = &self.facts.leakage[index];
        let mut evidence = Vec::with_capacity(
            findings.len() + sizes.len() + leakage.len() + pattern_facts.len() + 1,
        );
        let mut rest = findings.iter();
        if let Some(&top) = rest.next() {
            evidence.push(ProblemEvidence::Finding(top));
        }
        evidence.extend(pattern_facts);
        evidence.extend(leakage.iter().map(|&id| ProblemEvidence::ChangeLeakage(id)));
        evidence.extend(sizes.iter().map(|&id| ProblemEvidence::Size(id)));
        if let Some(touches) = self.input.hot_touches(FileId::from_index(index)) {
            evidence.push(ProblemEvidence::Hot(touches));
        }
        evidence.extend(rest.map(|&id| ProblemEvidence::Finding(id)));
        let claimed_findings = findings
            .iter()
            .map(|&id| ClaimedFinding::Source(id))
            .chain(sizes.iter().map(|&id| ClaimedFinding::Size(id)))
            .chain(leakage.iter().map(|&id| ClaimedFinding::ChangeLeakage(id)))
            .collect::<Vec<_>>();
        // Claiming a change-leakage finding is enough to be shown by default:
        // such a finding is a rated statement that survived every guard, floor,
        // and proof its capability demands, and the file it names tends to be
        // one with no debt of its own.
        let shown = self.facts.verdict_affecting[index] || !leakage.is_empty();
        ProblemCard {
            pattern,
            rating,
            anchor: ProblemAnchor::File(FileId::from_index(index)),
            evidence,
            visibility: if shown {
                ProblemVisibility::Default
            } else {
                ProblemVisibility::Detail
            },
            claimed_findings,
        }
    }
}

/// One card per unexplained coupling finding, keeping that finding's operands.
fn shotgun_pairs(input: &ProblemInput<'_>, cards: &mut Vec<ProblemCard>) {
    for finding in input.evolutionary_findings {
        let coupling = finding.coupling();
        cards.push(ProblemCard {
            pattern: ProblemPattern::ShotgunPair,
            rating: finding.rating(),
            anchor: ProblemAnchor::PackagePair(coupling.left(), coupling.right()),
            evidence: vec![ProblemEvidence::Coupling(finding.id())],
            claimed_findings: vec![ClaimedFinding::Coupling(finding.id())],
            visibility: ProblemVisibility::Default,
        });
    }
}

/// One card per knowledge-concentration finding, which carries counts only.
fn bus_risks(input: &ProblemInput<'_>, cards: &mut Vec<ProblemCard>) {
    for finding in input.knowledge_concentration_findings {
        cards.push(ProblemCard {
            pattern: ProblemPattern::BusRisk,
            rating: finding.rating(),
            anchor: ProblemAnchor::Package(finding.concentration().package()),
            evidence: vec![ProblemEvidence::Knowledge(finding.id())],
            claimed_findings: vec![ClaimedFinding::Knowledge(finding.id())],
            visibility: ProblemVisibility::Default,
        });
    }
}

/// One card per stable-dependency violation.
fn unstable_dependencies(input: &ProblemInput<'_>, cards: &mut Vec<ProblemCard>) {
    for finding in input.stable_dependency_findings {
        cards.push(ProblemCard {
            pattern: ProblemPattern::UnstableDependency,
            rating: finding.rating(),
            anchor: ProblemAnchor::PackagePair(finding.source(), finding.target()),
            evidence: vec![ProblemEvidence::StableDependency(finding.id())],
            claimed_findings: vec![ClaimedFinding::StableDependency(finding.id())],
            visibility: ProblemVisibility::Default,
        });
    }
}

/// Orders the table once, so no renderer sorts, re-ranks, or reorders it.
fn rank<'a>(input: &ProblemInput<'a>, cards: Vec<ProblemCard>) -> Vec<ProblemCard> {
    let mut ranked: Vec<(ProblemRank<'a>, ProblemCard)> = cards
        .into_iter()
        .map(|card| {
            let key = ProblemRank::new(
                &card,
                claimed_high(input, &card),
                top_finding_rank(input, &card),
                anchor_path(input, card.anchor()),
                anchor_start_line(input, &card),
            );
            (key, card)
        })
        .collect();
    // A stable sort keeps the table order the detectors emitted in whenever
    // every rank key ties, so the answer stays data-stable.
    ranked.sort_by(|left, right| left.0.cmp(&right.0));
    ranked.into_iter().map(|(_, card)| card).collect()
}

/// How many of a card's claimed findings are rated High.
fn claimed_high(input: &ProblemInput<'_>, card: &ProblemCard) -> u32 {
    card.claimed_findings()
        .iter()
        .filter(|claim| claim_rating(input, **claim) == Rating::High)
        .count() as u32
}

fn claim_rating(input: &ProblemInput<'_>, claim: ClaimedFinding) -> Rating {
    match claim {
        ClaimedFinding::Source(id) => input.findings[id.index()].assessment().rating(),
        ClaimedFinding::Size(id) => input.size_findings[id.index()].rating(),
        ClaimedFinding::Architecture(id) => input.architecture_findings[id.index()].rating(),
        ClaimedFinding::StableDependency(id) => {
            input.stable_dependency_findings[id.index()].rating()
        }
        ClaimedFinding::Coupling(id) => input.evolutionary_findings[id.index()].rating(),
        ClaimedFinding::Knowledge(id) => {
            input.knowledge_concentration_findings[id.index()].rating()
        }
        ClaimedFinding::ChangeLeakage(id) => input.change_leakage_findings[id.index()].rating(),
    }
}

/// The accepted finding rank of a card's top claimed source finding.
fn top_finding_rank<'a>(input: &ProblemInput<'a>, card: &ProblemCard) -> Option<FindingRank<'a>> {
    let id = card
        .claimed_findings()
        .iter()
        .find_map(|claim| match claim {
            ClaimedFinding::Source(id) => Some(*id),
            _ => None,
        })?;
    let finding = &input.findings[id.index()];
    let file = input.files.get(finding.file().index())?;
    Some(FindingRank::new(
        finding,
        input.hot_touches(finding.file()).is_some(),
        file.activity().map_or(0, FileActivity::touches),
        file.path(),
    ))
}

/// The anchor's repository-relative path, which for a file set is the earliest
/// member path so the key does not depend on member order.
fn anchor_path<'a>(input: &ProblemInput<'a>, anchor: &ProblemAnchor) -> &'a str {
    match anchor {
        ProblemAnchor::File(file) => input.path(*file),
        ProblemAnchor::Files(files) => files
            .iter()
            .map(|&file| input.path(file))
            .min()
            .unwrap_or_default(),
        ProblemAnchor::Package(package) | ProblemAnchor::PackagePair(package, _) => {
            input.package_path(*package)
        }
    }
}

/// The start line of the card's head, which is the span of its top claimed
/// source finding and zero for a card that claims none.
fn anchor_start_line(input: &ProblemInput<'_>, card: &ProblemCard) -> u32 {
    card.claimed_findings()
        .iter()
        .find_map(|claim| match claim {
            ClaimedFinding::Source(id) => Some(input.findings[id.index()].span().start_line()),
            _ => None,
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::architecture::{
        ArchitectureFindingKind, DependencyEdgeId, PackageGraphMeasurement,
        StableDependencyEvidence,
    };
    use crate::evolution::{ChangeCoupling, ContributorConcentration};
    use crate::health::{HealthCounts, HealthPolicy, Measurements, Thresholds};
    use crate::report::{Coverage, ScopeId};
    use crate::size::SizePolicy;
    use crate::source::{
        SourceRole, SourceSpan, SourceTrust, StaticRelationKind, UnitIdentity, UnitKind,
    };

    /// A unit that rates High on cognitive complexity alone.
    fn high() -> Measurements {
        Measurements::new(25, 1, 1)
    }

    /// A unit that rates Watch on cognitive complexity alone.
    fn watch() -> Measurements {
        Measurements::new(15, 1, 1)
    }

    /// Hand-built report tables, so every detector is exercised without
    /// composing a report.
    #[derive(Default)]
    struct Tables {
        files: Vec<FileRecord>,
        findings: Vec<Finding>,
        packages: Vec<PackageRecord>,
        sizes: Vec<SizeFinding>,
        edges: Vec<DependencyEdge>,
        architecture: Vec<ArchitectureFinding>,
        hotspots: Vec<Hotspot>,
        stable: Vec<StableDependencyFinding>,
        coupling: Vec<EvolutionaryFinding>,
        concentration: Vec<KnowledgeConcentrationFinding>,
        reach: Vec<FileReach>,
        /// The retained pairs and the leakage findings decided from them.
        ///
        /// Both are wired through so this harness audits every claimable
        /// table, the way the harness in `tests/problem_leakage.rs` does; the
        /// behaviour of the two leakage patterns is exercised there, where the
        /// cases have room to state themselves.
        pairs: Vec<FileChangeCoupling>,
        leakage: Vec<ChangeLeakageFinding>,
    }

    impl Tables {
        fn package(&mut self, path: &str) -> usize {
            let index = self.packages.len();
            self.packages.push(PackageRecord::current(
                PackageId::from_index(index),
                ScopeId::from_index(0),
                path,
            ));
            index
        }

        /// Adds one file with its rated unit counts, where `rated` is the
        /// file's total rated units.
        fn file(&mut self, path: &str, package: usize, rated: u32, high_units: u32) -> usize {
            self.graded(
                path,
                package,
                HealthCounts::new(rated.saturating_sub(high_units), 0, high_units),
            )
        }

        /// Adds one file with an explicit split between healthy units and the
        /// units that carry debt.
        fn graded(&mut self, path: &str, package: usize, health: HealthCounts) -> usize {
            let index = self.files.len();
            self.files.push(
                FileRecord::new(
                    FileId::from_index(index),
                    ScopeId::from_index(0),
                    path,
                    Coverage::default(),
                    health,
                )
                .with_package(PackageId::from_index(package)),
            );
            index
        }

        fn finding(&mut self, file: usize, name: &str, measurements: Measurements, line: u32) {
            let id = FindingId::from_index(self.findings.len());
            self.findings.push(Finding::new(
                id,
                FileId::from_index(file),
                UnitIdentity::new(name, UnitKind::Function),
                SourceSpan::new(line, line),
                measurements,
                HealthPolicy::default().assess(measurements),
            ));
        }

        fn size(&mut self, file: usize, lines: u32) {
            let policy = SizePolicy::new(Thresholds::new(10, 20), Thresholds::new(10, 20));
            self.sizes
                .push(policy.rate_file(FileId::from_index(file), lines).unwrap());
        }

        /// Adds one finding whose role and trust keep it out of the verdict,
        /// which is how recovered, fixture, and generated debt reaches the
        /// report.
        fn finding_outside_the_verdict(
            &mut self,
            file: usize,
            name: &str,
            measurements: Measurements,
            evidence: (SourceRole, SourceTrust),
        ) {
            self.finding(file, name, measurements, 7);
            let finding = self.findings.pop().unwrap();
            self.findings
                .push(finding.with_evidence(evidence.0, evidence.1));
        }

        fn link(&mut self, source: usize, target: usize) {
            self.edges.push(DependencyEdge::new(
                DependencyEdgeId::from_index(self.edges.len()),
                FileId::from_index(source),
                FileId::from_index(target),
                1,
                Vec::new(),
            ));
        }

        /// Links two files with a relation that never enters the verdict
        /// graph.
        fn link_outside_the_verdict_graph(
            &mut self,
            source: usize,
            target: usize,
            evidence: (SourceRole, SourceTrust, StaticRelationKind),
        ) {
            self.link(source, target);
            let edge = self.edges.pop().unwrap();
            self.edges.push(
                edge.with_evidence(evidence.0, evidence.1)
                    .with_relation(evidence.2),
            );
        }

        fn hotspot(&mut self, file: usize, touches: u32) {
            self.hotspots.push(Hotspot::new(
                FileId::from_index(file),
                Rating::High,
                touches,
            ));
        }

        fn cycle(&mut self, files: &[usize]) {
            self.architecture.push(ArchitectureFinding::new(
                ArchitectureFindingId::from_index(self.architecture.len()),
                ArchitectureFindingKind::FileCycle,
                Vec::new(),
                files.iter().map(|&file| FileId::from_index(file)).collect(),
                Vec::new(),
            ));
        }

        /// Records the exact reach of one candidate file, keeping the table in
        /// file order the way the report builds it.
        fn reach(&mut self, file: usize, reach: u32) {
            self.reach
                .push(FileReach::new(FileId::from_index(file), reach));
            self.reach.sort_unstable_by_key(|value| value.file());
        }

        fn input(&self) -> ProblemInput<'_> {
            ProblemInput::new(&self.files, &self.findings)
                .with_change_leakage(&self.leakage, &self.pairs)
                .with_packages(&self.packages)
                .with_size_findings(&self.sizes)
                .with_architecture(&self.architecture, &self.edges)
                .with_stable_dependencies(&self.stable)
                .with_evolution(&self.coupling, &self.concentration)
                .with_hotspots(&self.hotspots)
                .with_file_reach(&self.reach)
        }

        fn cluster(&self) -> Vec<ProblemCard> {
            cluster_problems(self.input())
        }

        /// Asserts the coverage half of the index-integrity audit: every
        /// retained finding of every claimable table reaches exactly one card.
        fn assert_every_finding_is_claimed_once(&self) {
            let cards = self.cluster();
            assert_eq!(duplicate_claim(&cards), None, "a finding was claimed twice");
            let mut expected: BTreeSet<ClaimedFinding> = BTreeSet::new();
            expected.extend(
                (0..self.findings.len())
                    .map(|id| ClaimedFinding::Source(FindingId::from_index(id))),
            );
            expected.extend(
                (0..self.sizes.len()).map(|id| ClaimedFinding::Size(SizeFindingId::from_index(id))),
            );
            expected.extend(
                (0..self.architecture.len())
                    .map(|id| ClaimedFinding::Architecture(ArchitectureFindingId::from_index(id))),
            );
            expected.extend(
                (0..self.coupling.len())
                    .map(|id| ClaimedFinding::Coupling(EvolutionaryFindingId::from_index(id))),
            );
            expected.extend((0..self.concentration.len()).map(|id| {
                ClaimedFinding::Knowledge(KnowledgeConcentrationFindingId::from_index(id))
            }));
            expected.extend((0..self.stable.len()).map(|id| {
                ClaimedFinding::StableDependency(StableDependencyFindingId::from_index(id))
            }));
            expected.extend(
                (0..self.leakage.len()).map(|id| {
                    ClaimedFinding::ChangeLeakage(ChangeLeakageFindingId::from_index(id))
                }),
            );
            let claimed = distinct_claims(&cards);
            assert_eq!(
                claimed.difference(&expected).collect::<Vec<_>>(),
                Vec::<&ClaimedFinding>::new(),
                "a card claimed something the report does not hold"
            );
            assert_eq!(
                expected.difference(&claimed).collect::<Vec<_>>(),
                Vec::<&ClaimedFinding>::new(),
                "a retained finding reached no card"
            );
        }

        /// The pattern of the single card anchored on one file, when there is
        /// one.
        fn file_pattern(&self, file: usize) -> Option<ProblemPattern> {
            let cards = self.cluster();
            let anchored: Vec<&ProblemCard> = cards
                .iter()
                .filter(|card| card.anchor() == &ProblemAnchor::File(FileId::from_index(file)))
                .collect();
            assert!(
                anchored.len() <= 1,
                "one file, at most one file-anchored card"
            );
            anchored.first().map(|card| card.pattern())
        }
    }

    /// A file carrying `high_units` High findings, `debt` units rated Watch or
    /// High in total, and a chosen fan-out, in its own package.
    fn concentrated_file(high_units: u32, debt: u32, fan_out: usize) -> Tables {
        graded_file(high_units, debt, 0, fan_out)
    }

    /// The same file with a chosen number of healthy units beside its debt, so
    /// a test can prove that length in units never satisfies the concentration
    /// conjunct on its own.
    fn graded_file(high_units: u32, debt: u32, healthy: u32, fan_out: usize) -> Tables {
        let mut tables = Tables::default();
        let package = tables.package("a");
        let watch_units = debt.saturating_sub(high_units);
        let subject = tables.graded(
            "a/subject.rs",
            package,
            HealthCounts::new(healthy, watch_units, high_units),
        );
        for index in 0..high_units {
            tables.finding(subject, &format!("high{index}"), high(), index + 1);
        }
        for index in 0..watch_units {
            tables.finding(subject, &format!("watch{index}"), watch(), 100 + index);
        }
        for index in 0..fan_out {
            let target = tables.file(&format!("a/target{index}.rs"), package, 0, 0);
            tables.link(subject, target);
        }
        tables
    }

    #[test]
    fn a_file_with_two_high_findings_and_no_breadth_is_not_a_god_file() {
        // Neither arm of the concentration conjunct holds: two High findings
        // is below three, and two debt-carrying units is below six.
        let tables = concentrated_file(2, 2, 2);
        assert_eq!(tables.file_pattern(0), Some(ProblemPattern::Measured));
    }

    #[test]
    fn a_small_file_that_concentrates_three_high_findings_is_not_a_god_file() {
        // Concentration holds and breadth does not, so the file has bugs
        // rather than doing too much.
        let tables = concentrated_file(3, 3, 2);
        assert_eq!(tables.file_pattern(0), Some(ProblemPattern::Measured));
    }

    #[test]
    fn a_file_that_is_both_concentrated_and_broad_is_a_god_file_rated_high() {
        let tables = concentrated_file(3, 3, 10);
        let cards = tables.cluster();
        let card = &cards[0];
        assert_eq!(card.pattern(), ProblemPattern::GodFile);
        assert_eq!(card.rating(), Rating::High);
        assert_eq!(card.anchor(), &ProblemAnchor::File(FileId::from_index(0)));
        assert_eq!(card.visibility(), ProblemVisibility::Default);
        // A fan-out of nine leaves the same file outside the pattern. It is
        // still unusually broad for its package, so the next pattern in
        // claiming order takes it.
        assert_eq!(
            concentrated_file(3, 3, 9).file_pattern(0),
            Some(ProblemPattern::Hub)
        );
    }

    #[test]
    fn one_high_finding_needs_six_debt_units_before_breadth_makes_a_god_file() {
        // The second arm of the concentration conjunct: one High finding is
        // enough only once the file holds six units rated Watch or High.
        assert_eq!(
            concentrated_file(1, 6, 10).file_pattern(0),
            Some(ProblemPattern::GodFile)
        );
        // Five debt-carrying units leaves the concentration conjunct
        // unsatisfied, so the broad file is only a hub.
        assert_eq!(
            concentrated_file(1, 5, 10).file_pattern(0),
            Some(ProblemPattern::Hub)
        );
    }

    #[test]
    fn healthy_units_never_satisfy_the_concentration_conjunct() {
        // Twenty healthy units beside one debt-carrying unit is a long file
        // with one bug, not a file that concentrates debt. Counting every
        // rated unit would call this broad file a god file on its length
        // alone, which is what single-file components made routine.
        let tables = graded_file(1, 1, 20, 10);
        assert_eq!(tables.file_pattern(0), Some(ProblemPattern::Hub));
        // The same file with six of those units carrying debt is one.
        assert_eq!(
            graded_file(1, 6, 20, 10).file_pattern(0),
            Some(ProblemPattern::GodFile)
        );
    }

    #[test]
    fn a_size_finding_satisfies_the_breadth_conjunct_without_any_fan_out() {
        let mut tables = concentrated_file(3, 3, 0);
        tables.size(0, 40);
        let card = &tables.cluster()[0];
        assert_eq!(card.pattern(), ProblemPattern::GodFile);
        assert!(
            card.claimed_findings()
                .contains(&ClaimedFinding::Size(SizeFindingId::from_index(0)))
        );
        // The size finding is the reason the card exists, so it is stated
        // among the facts that made the pattern fire rather than after the
        // findings the card enumerates.
        assert_eq!(
            &card.evidence()[..3],
            [
                ProblemEvidence::Finding(FindingId::from_index(0)),
                ProblemEvidence::RatedUnits(3),
                ProblemEvidence::Size(SizeFindingId::from_index(0)),
            ]
        );
    }

    #[test]
    fn breadth_alone_is_never_a_god_file() {
        // Ten outgoing dependencies and no rated debt at all: the file is
        // broad, so it is a `detail` hub rather than a god file.
        let tables = concentrated_file(0, 0, 10);
        assert_eq!(tables.file_pattern(0), Some(ProblemPattern::Hub));
        assert_eq!(tables.cluster()[0].visibility(), ProblemVisibility::Detail);
    }

    /// A package whose median fan-in is zero, where `importers` files import
    /// one subject file.
    fn imported_file(importers: usize) -> Tables {
        let mut tables = Tables::default();
        let package = tables.package("a");
        let subject = tables.file("a/subject.rs", package, 0, 0);
        for index in 0..importers {
            let importer = tables.file(&format!("a/importer{index}.rs"), package, 0, 0);
            tables.link(importer, subject);
        }
        tables
    }

    #[test]
    fn a_fan_in_of_eight_is_a_hub_where_seven_is_not() {
        assert_eq!(imported_file(7).file_pattern(0), None);
        assert_eq!(imported_file(8).file_pattern(0), Some(ProblemPattern::Hub));
    }

    /// One package of five files where the subject reaches a fan-in of eight
    /// and the four others each reach `neighbour_fan_in`, so the package's
    /// nearest-rank median fan-in is `neighbour_fan_in`.
    fn hub_against_a_median(neighbour_fan_in: usize) -> Tables {
        let mut tables = Tables::default();
        let inside = tables.package("a");
        let outside = tables.package("b");
        let subject = tables.file("a/subject.rs", inside, 0, 0);
        let neighbours: Vec<usize> = (0..4)
            .map(|index| tables.file(&format!("a/neighbour{index}.rs"), inside, 0, 0))
            .collect();
        let sources: Vec<usize> = (0..4)
            .map(|index| tables.file(&format!("b/source{index}.rs"), outside, 0, 0))
            .collect();
        for &neighbour in &neighbours {
            tables.link(neighbour, subject);
        }
        for &source in &sources {
            tables.link(source, subject);
        }
        for &neighbour in &neighbours {
            for &source in sources.iter().take(neighbour_fan_in) {
                tables.link(source, neighbour);
            }
        }
        tables
    }

    #[test]
    fn a_fan_in_exactly_four_times_the_package_median_is_a_hub() {
        // Median two, fan-in eight: exactly the multiple, and at the degree
        // floor.
        assert_eq!(
            hub_against_a_median(2).file_pattern(0),
            Some(ProblemPattern::Hub)
        );
        // Median three: eight is above the degree floor but below twelve.
        assert_eq!(hub_against_a_median(3).file_pattern(0), None);
    }

    #[test]
    fn relations_outside_the_verdict_graph_never_create_a_hub() {
        for evidence in [
            (
                SourceRole::Test,
                SourceTrust::Trusted,
                StaticRelationKind::Uses,
            ),
            (
                SourceRole::Example,
                SourceTrust::Trusted,
                StaticRelationKind::Uses,
            ),
            (
                SourceRole::Benchmark,
                SourceTrust::Trusted,
                StaticRelationKind::Uses,
            ),
            (
                SourceRole::Primary,
                SourceTrust::Trusted,
                StaticRelationKind::ModuleOwnership,
            ),
            (
                SourceRole::Primary,
                SourceTrust::Advisory,
                StaticRelationKind::Uses,
            ),
        ] {
            let mut tables = Tables::default();
            let package = tables.package("a");
            let subject = tables.file("a/subject.rs", package, 0, 0);
            for index in 0..8 {
                let importer = tables.file(&format!("a/importer{index}.rs"), package, 0, 0);
                tables.link_outside_the_verdict_graph(importer, subject, evidence);
            }
            assert_eq!(tables.file_pattern(0), None, "{evidence:?}");
        }
    }

    #[test]
    fn a_widely_imported_file_without_any_debt_claims_nothing_and_is_detail() {
        let cards = imported_file(8).cluster();
        let card = &cards[0];
        assert_eq!(card.pattern(), ProblemPattern::Hub);
        assert_eq!(card.rating(), Rating::Healthy);
        assert_eq!(card.visibility(), ProblemVisibility::Detail);
        assert!(card.claimed_findings().is_empty());
        assert_eq!(card.evidence(), [ProblemEvidence::FanIn(8)]);
    }

    #[test]
    fn a_hub_states_its_exact_reach_after_the_degree_that_made_it_fire() {
        let mut tables = imported_file(8);
        tables.size(0, 15);
        tables.reach(0, 41);
        let cards = tables.cluster();
        assert_eq!(
            cards[0].evidence(),
            [
                ProblemEvidence::FanIn(8),
                ProblemEvidence::ReachIn(41),
                ProblemEvidence::Size(SizeFindingId::from_index(0)),
            ],
            "reach follows the degree and precedes every claimed finding"
        );
        // A second file with material reach now earns its own named hub card;
        // the original degree hub keeps its degree evidence.
        let mut bare = imported_file(8);
        bare.reach(1, 41);
        assert!(
            bare.cluster()
                .iter()
                .any(|card| card.evidence() == [ProblemEvidence::FanIn(8)])
        );
        // A candidate nothing depends on reaches nothing, and "0 files" states
        // nothing: the table keeps the row and the card states no line.
        let mut nothing = imported_file(8);
        nothing.reach(0, 0);
        assert_eq!(nothing.cluster()[0].evidence(), [ProblemEvidence::FanIn(8)]);
    }

    #[test]
    fn a_tangle_states_its_reach_between_its_member_count_and_its_heat() {
        let mut tables = Tables::default();
        let package = tables.package("a");
        for index in 0..3 {
            tables.file(&format!("a/member{index}.rs"), package, 0, 0);
        }
        tables.cycle(&[0, 1, 2]);
        tables.hotspot(1, 14);
        // Only the second member is a candidate; every member of a cycle
        // reaches what the others reach, so one value answers for the card.
        tables.reach(1, 12);
        let cards = tables.cluster();
        assert_eq!(
            cards[0].evidence(),
            [
                ProblemEvidence::Architecture(ArchitectureFindingId::from_index(0)),
                ProblemEvidence::Members(3),
                ProblemEvidence::ReachIn(12),
                ProblemEvidence::Hot(14),
            ]
        );
    }

    #[test]
    fn a_widely_imported_file_that_also_carries_verdict_debt_is_a_default_card() {
        let mut tables = imported_file(8);
        tables.finding(0, "work", high(), 12);
        let cards = tables.cluster();
        let card = &cards[0];
        assert_eq!(card.pattern(), ProblemPattern::Hub);
        assert_eq!(card.rating(), Rating::High);
        assert_eq!(card.visibility(), ProblemVisibility::Default);
        assert_eq!(
            card.claimed_findings(),
            [ClaimedFinding::Source(FindingId::from_index(0))]
        );
    }

    #[test]
    fn a_generated_file_that_is_imported_everywhere_keeps_its_own_rating() {
        // The paradox the boolean flag produced: a file with rated findings
        // that cannot move a verdict was called healthy because nothing
        // claimed those findings.
        for evidence in [
            (SourceRole::Generated, SourceTrust::Trusted),
            (SourceRole::Fixture, SourceTrust::Trusted),
            (SourceRole::Primary, SourceTrust::Advisory),
        ] {
            let mut tables = imported_file(8);
            tables.finding_outside_the_verdict(0, "work", high(), evidence);
            let cards = tables.cluster();
            let card = &cards[0];
            assert_eq!(card.pattern(), ProblemPattern::Hub, "{evidence:?}");
            assert_eq!(card.rating(), Rating::High, "{evidence:?}");
            assert_eq!(card.visibility(), ProblemVisibility::Detail, "{evidence:?}");
            assert_eq!(
                card.claimed_findings(),
                [ClaimedFinding::Source(FindingId::from_index(0))],
                "{evidence:?}"
            );
        }
    }

    #[test]
    fn a_size_finding_alone_rates_a_hub_and_keeps_it_out_of_default_detail() {
        let mut tables = imported_file(8);
        tables.size(0, 15);
        let cards = tables.cluster();
        assert_eq!(cards[0].rating(), Rating::Watch);
        // A size finding is not a verdict-affecting finding, so the card is
        // rated but stays where today's size rows already live.
        assert_eq!(cards[0].visibility(), ProblemVisibility::Detail);
        assert_eq!(
            cards[0].claimed_findings(),
            [ClaimedFinding::Size(SizeFindingId::from_index(0))]
        );
    }

    #[test]
    fn a_file_whose_only_evidence_is_its_size_still_reaches_a_card() {
        let mut tables = Tables::default();
        let package = tables.package("a");
        let file = tables.file("a/long.rs", package, 0, 0);
        tables.size(file, 15);
        let cards = tables.cluster();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].pattern(), ProblemPattern::Measured);
        assert_eq!(cards[0].rating(), Rating::Watch);
        assert_eq!(cards[0].visibility(), ProblemVisibility::Detail);
        assert_eq!(
            cards[0].evidence(),
            [ProblemEvidence::Size(SizeFindingId::from_index(0))],
            "the head of a card claiming no source finding is its size finding"
        );
    }

    #[test]
    fn a_file_whose_whole_debt_is_advisory_still_reaches_a_card() {
        let mut tables = Tables::default();
        let package = tables.package("a");
        let file = tables.file("a/recovered.rs", package, 0, 0);
        tables.finding_outside_the_verdict(
            file,
            "work",
            watch(),
            (SourceRole::Primary, SourceTrust::Advisory),
        );
        let cards = tables.cluster();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].pattern(), ProblemPattern::Measured);
        assert_eq!(cards[0].rating(), Rating::Watch);
        assert_eq!(cards[0].visibility(), ProblemVisibility::Detail);
        assert_eq!(
            cards[0].claimed_findings(),
            [ClaimedFinding::Source(FindingId::from_index(0))]
        );
    }

    #[test]
    fn a_file_mixing_verdict_and_advisory_debt_claims_both_on_one_default_card() {
        let mut tables = Tables::default();
        let package = tables.package("a");
        let file = tables.file("a/work.rs", package, 1, 1);
        tables.finding(file, "counts", high(), 4);
        tables.finding_outside_the_verdict(
            file,
            "advisory",
            watch(),
            (SourceRole::Test, SourceTrust::Advisory),
        );
        let cards = tables.cluster();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].claimed_findings().len(), 2);
        assert_eq!(cards[0].visibility(), ProblemVisibility::Default);
    }

    #[test]
    fn a_hot_file_with_high_debt_that_no_earlier_pattern_claimed_is_a_hot_mess() {
        let mut tables = concentrated_file(1, 1, 0);
        tables.hotspot(0, 14);
        let cards = tables.cluster();
        assert_eq!(cards[0].pattern(), ProblemPattern::HotMess);
        assert_eq!(cards[0].rating(), Rating::High);
        assert!(cards[0].is_hot());
        assert!(cards[0].evidence().contains(&ProblemEvidence::Hot(14)));
        // Heat without High debt is not a hot mess.
        let mut watch_only = Tables::default();
        let package = watch_only.package("a");
        let file = watch_only.file("a/warm.rs", package, 1, 0);
        watch_only.finding(file, "work", watch(), 3);
        watch_only.hotspot(file, 20);
        assert_eq!(
            watch_only.file_pattern(0),
            Some(ProblemPattern::Measured),
            "a hot file without High debt falls through to the fallback"
        );
    }

    #[test]
    fn god_file_claims_a_file_that_also_satisfies_the_hub_and_hot_mess_rules() {
        let mut tables = concentrated_file(3, 3, 10);
        // The same file is imported eight times and changes often.
        for index in 0..8 {
            let importer = tables.file(&format!("a/importer{index}.rs"), 0, 0, 0);
            tables.link(importer, 0);
        }
        tables.hotspot(0, 30);
        assert_eq!(tables.file_pattern(0), Some(ProblemPattern::GodFile));
    }

    #[test]
    fn one_file_with_three_high_findings_produces_one_card_that_claims_all_three() {
        let tables = concentrated_file(3, 3, 2);
        let cards = tables.cluster();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].claimed_findings().len(), 3);
        assert!(duplicate_claim(&cards).is_none());
    }

    #[test]
    fn a_cycle_is_one_card_that_leaves_its_members_their_own_findings() {
        let mut tables = Tables::default();
        let package = tables.package("a");
        for index in 0..5 {
            let file = tables.file(&format!("a/member{index}.rs"), package, 1, 0);
            tables.finding(file, "work", watch(), 4);
        }
        tables.cycle(&[0, 1, 2, 3, 4]);
        tables.hotspot(2, 9);
        let cards = tables.cluster();
        let tangles: Vec<&ProblemCard> = cards
            .iter()
            .filter(|card| card.pattern() == ProblemPattern::Tangle)
            .collect();
        assert_eq!(tangles.len(), 1);
        assert_eq!(
            tangles[0].anchor(),
            &ProblemAnchor::Files((0..5).map(FileId::from_index).collect())
        );
        assert_eq!(
            tangles[0].claimed_findings(),
            [ClaimedFinding::Architecture(
                ArchitectureFindingId::from_index(0)
            )]
        );
        // The witness comes first, so the one evidence line a tight rung
        // allows is the cycle itself rather than its member count.
        assert_eq!(
            tangles[0].evidence(),
            [
                ProblemEvidence::Architecture(ArchitectureFindingId::from_index(0)),
                ProblemEvidence::Members(5),
                ProblemEvidence::Hot(9),
            ]
        );
        // Each member still reaches its own card for its own debt.
        assert_eq!(
            cards
                .iter()
                .filter(|card| card.pattern() == ProblemPattern::Measured)
                .count(),
            5
        );
        assert!(duplicate_claim(&cards).is_none());
    }

    #[test]
    fn each_table_finding_becomes_one_card_carrying_its_own_operands() {
        let mut tables = Tables::default();
        let left = tables.package("a");
        let right = tables.package("b");
        tables.coupling.push(EvolutionaryFinding::new(
            EvolutionaryFindingId::from_index(0),
            ChangeCoupling::new(
                PackageId::from_index(left),
                PackageId::from_index(right),
                7,
                9,
            ),
        ));
        tables
            .concentration
            .push(KnowledgeConcentrationFinding::new(
                KnowledgeConcentrationFindingId::from_index(0),
                ContributorConcentration::new(PackageId::from_index(left), 1, 10, 10),
            ));
        tables.stable.push(StableDependencyFinding::new(
            StableDependencyFindingId::from_index(0),
            PackageId::from_index(left),
            PackageId::from_index(right),
            StableDependencyEvidence::new(
                PackageGraphMeasurement::new(PackageId::from_index(left), 1, 0),
                PackageGraphMeasurement::new(PackageId::from_index(right), 0, 1),
                2,
            ),
            Vec::new(),
        ));
        let cards = tables.cluster();
        assert_eq!(cards.len(), 3);
        let by_pattern = |pattern: ProblemPattern| {
            cards
                .iter()
                .find(|card| card.pattern() == pattern)
                .unwrap_or_else(|| panic!("no {} card", pattern.id()))
        };
        let shotgun = by_pattern(ProblemPattern::ShotgunPair);
        assert_eq!(
            shotgun.anchor(),
            &ProblemAnchor::PackagePair(PackageId::from_index(left), PackageId::from_index(right))
        );
        assert_eq!(
            shotgun.evidence(),
            [ProblemEvidence::Coupling(
                EvolutionaryFindingId::from_index(0)
            )]
        );
        assert_eq!(shotgun.rating(), Rating::Watch);
        let bus = by_pattern(ProblemPattern::BusRisk);
        assert_eq!(
            bus.anchor(),
            &ProblemAnchor::Package(PackageId::from_index(left))
        );
        let unstable = by_pattern(ProblemPattern::UnstableDependency);
        assert_eq!(
            unstable.anchor(),
            &ProblemAnchor::PackagePair(PackageId::from_index(left), PackageId::from_index(right))
        );
        assert!(duplicate_claim(&cards).is_none());
    }

    #[test]
    fn the_fallback_names_the_file_top_finding_and_claims_the_rest() {
        let mut tables = Tables::default();
        let package = tables.package("a");
        let file = tables.file("a/work.rs", package, 2, 1);
        tables.finding(file, "lesser", watch(), 30);
        tables.finding(file, "worst", high(), 10);
        let cards = tables.cluster();
        assert_eq!(cards.len(), 1);
        let card = &cards[0];
        assert_eq!(card.pattern(), ProblemPattern::Measured);
        assert_eq!(card.rating(), Rating::High);
        assert_eq!(
            card.evidence().first(),
            Some(&ProblemEvidence::Finding(FindingId::from_index(1))),
            "the head is the file's top ranked finding"
        );
        assert_eq!(card.claimed_findings().len(), 2);
    }

    #[test]
    fn no_finding_is_claimed_twice_where_several_detectors_could_claim_it() {
        // One file is inside a cycle, does too much, is imported everywhere,
        // is hot, and carries a size finding, so five detectors could reach
        // for the same findings.
        let mut tables = concentrated_file(3, 9, 10);
        tables.size(0, 40);
        for index in 0..8 {
            let importer = tables.file(&format!("a/importer{index}.rs"), 0, 1, 1);
            tables.finding(importer, "work", high(), 5);
            tables.link(importer, 0);
        }
        tables.hotspot(0, 30);
        tables.cycle(&[0, 1]);
        let cards = tables.cluster();
        assert_eq!(duplicate_claim(&cards), None);
        let claimed: usize = cards.iter().map(|card| card.claimed_findings().len()).sum();
        assert_eq!(claimed, distinct_claims(&cards).len());
        tables.assert_every_finding_is_claimed_once();
    }

    #[test]
    fn every_retained_finding_reaches_exactly_one_card() {
        // A repository holding one of everything, including the debt that
        // cannot move a verdict and the file whose only evidence is its size.
        let mut tables = Tables::default();
        let left = tables.package("a");
        let right = tables.package("b");
        let god = tables.file("a/god.rs", left, 9, 3);
        for index in 0..3 {
            tables.finding(god, &format!("god{index}"), high(), index + 1);
        }
        tables.size(god, 40);
        let advisory = tables.file("a/recovered.rs", left, 0, 0);
        tables.finding_outside_the_verdict(
            advisory,
            "advisory",
            watch(),
            (SourceRole::Primary, SourceTrust::Advisory),
        );
        let generated = tables.file("a/generated.rs", left, 0, 0);
        tables.finding_outside_the_verdict(
            generated,
            "generated",
            high(),
            (SourceRole::Generated, SourceTrust::Trusted),
        );
        let sized = tables.file("b/long.rs", right, 0, 0);
        tables.size(sized, 30);
        let ordinary = tables.file("b/work.rs", right, 2, 1);
        tables.finding(ordinary, "work", high(), 2);
        tables.finding(ordinary, "lesser", watch(), 40);
        tables.cycle(&[god, ordinary]);
        tables.coupling.push(EvolutionaryFinding::new(
            EvolutionaryFindingId::from_index(0),
            ChangeCoupling::new(
                PackageId::from_index(left),
                PackageId::from_index(right),
                7,
                9,
            ),
        ));
        tables
            .concentration
            .push(KnowledgeConcentrationFinding::new(
                KnowledgeConcentrationFindingId::from_index(0),
                ContributorConcentration::new(PackageId::from_index(left), 1, 10, 10),
            ));
        tables.assert_every_finding_is_claimed_once();
        // Nothing that cannot move a verdict slipped into default detail.
        let cards = tables.cluster();
        for (path, expected) in [
            ("a/god.rs", ProblemVisibility::Default),
            ("a/recovered.rs", ProblemVisibility::Detail),
            ("a/generated.rs", ProblemVisibility::Detail),
            ("b/long.rs", ProblemVisibility::Detail),
            ("b/work.rs", ProblemVisibility::Default),
        ] {
            let index = tables
                .files
                .iter()
                .position(|file| file.path() == path)
                .unwrap();
            let card = cards
                .iter()
                .find(|card| card.anchor() == &ProblemAnchor::File(FileId::from_index(index)))
                .unwrap_or_else(|| panic!("no card for {path}"));
            assert_eq!(card.visibility(), expected, "{path}");
        }
    }

    /// Every claimed identity across a table of cards.
    fn distinct_claims(cards: &[ProblemCard]) -> BTreeSet<ClaimedFinding> {
        cards
            .iter()
            .flat_map(|card| card.claimed_findings().iter().copied())
            .collect()
    }

    #[test]
    fn the_audit_reports_a_finding_two_cards_claim() {
        let claim = ClaimedFinding::Source(FindingId::from_index(3));
        let card = |pattern| ProblemCard {
            pattern,
            rating: Rating::High,
            anchor: ProblemAnchor::File(FileId::from_index(0)),
            evidence: Vec::new(),
            claimed_findings: vec![claim],
            visibility: ProblemVisibility::Default,
        };
        assert_eq!(
            duplicate_claim(&[card(ProblemPattern::GodFile), card(ProblemPattern::Hub)]),
            Some(claim)
        );
        assert_eq!(duplicate_claim(&[card(ProblemPattern::GodFile)]), None);
    }

    #[test]
    fn the_package_median_is_the_nearest_rank_member_and_is_package_relative() {
        assert_eq!(nearest_rank_median(&[]), 0);
        assert_eq!(nearest_rank_median(&[4]), 4);
        // Even samples take the lower of the two middle members rather than
        // averaging them, so the median stays an integer.
        assert_eq!(nearest_rank_median(&[1, 3, 5, 9]), 3);
        assert_eq!(nearest_rank_median(&[0, 0, 0, 0, 8]), 0);

        // The same fan-in is a hub in a quiet package and not in a busy one.
        let tables = hub_against_a_median(2);
        let input = tables.input();
        let facts = FileFacts::derive(&input);
        assert_eq!(facts.median(Some(PackageId::from_index(0))), (2, 1));
        assert_eq!(facts.median(Some(PackageId::from_index(1))), (0, 1));
    }

    #[test]
    fn pattern_ids_names_and_classes_are_frozen() {
        let patterns = [
            ProblemPattern::Tangle,
            ProblemPattern::GodFile,
            ProblemPattern::Hub,
            ProblemPattern::HotMess,
            ProblemPattern::ShotgunPair,
            ProblemPattern::BusRisk,
            ProblemPattern::UnstableDependency,
            ProblemPattern::Measured,
            ProblemPattern::LeakyInterface,
            ProblemPattern::HiddenCoupling,
        ];
        assert_eq!(
            patterns.map(ProblemPattern::id),
            [
                "tangle",
                "god_file",
                "hub",
                "hot_mess",
                "shotgun_pair",
                "bus_risk",
                "unstable_dependency",
                "measured",
                "leaky_interface",
                "hidden_coupling",
            ]
        );
        // The two leakage patterns are appended at the tail, so every existing
        // class is the class it was.
        assert_eq!(
            patterns.map(ProblemPattern::class),
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
        );
    }

    #[test]
    fn the_thresholds_are_named_integer_constants() {
        assert_eq!(CONCENTRATED_HIGH_FINDINGS, 3);
        assert_eq!(BROAD_DEBT_UNITS, 6);
        assert_eq!(GOD_FILE_FAN_OUT, 10);
        assert_eq!(HUB_DEGREE, 8);
        assert_eq!(HUB_MEDIAN_MULTIPLE, 4);
        let policy = ProblemPolicy::default();
        assert_eq!(policy.concentrated_high_findings(), 3);
        assert_eq!(policy.broad_debt_units(), 6);
        assert_eq!(policy.god_file_fan_out(), 10);
        assert_eq!(policy.hub_degree(), 8);
        assert_eq!(policy.hub_median_multiple(), 4);
    }

    /// One card with a chosen rating, pattern, hot state, and claim count.
    fn card(pattern: ProblemPattern, rating: Rating, hot: bool, claims: usize) -> ProblemCard {
        ProblemCard {
            pattern,
            rating,
            anchor: ProblemAnchor::File(FileId::from_index(0)),
            evidence: if hot {
                vec![ProblemEvidence::Hot(3)]
            } else {
                Vec::new()
            },
            claimed_findings: (0..claims)
                .map(|index| ClaimedFinding::Source(FindingId::from_index(index)))
                .collect(),
            visibility: ProblemVisibility::Default,
        }
    }

    #[test]
    fn the_problem_rank_reads_its_keys_in_the_accepted_order() {
        let rank = |card: &ProblemCard, claimed_high, path, line| {
            ProblemRank::new(card, claimed_high, None, path, line)
        };
        // KEY 1 rating: a High card precedes a Watch card however many
        // findings the Watch card claims and however hot it is.
        let high_card = card(ProblemPattern::Measured, Rating::High, false, 1);
        let watch_card = card(ProblemPattern::Tangle, Rating::Watch, true, 9);
        assert!(rank(&high_card, 0, "z", 9) < rank(&watch_card, 9, "a", 1));

        // KEY 2 claimed High count: rating ties, so more High claims precede.
        let three_high = card(ProblemPattern::Measured, Rating::High, false, 3);
        assert!(rank(&three_high, 3, "z", 9) < rank(&high_card, 1, "a", 1));

        // KEY 3 hot: rating and High claims tie, so the hot card precedes even
        // when the cold card claims more findings.
        let hot = card(ProblemPattern::Measured, Rating::High, true, 1);
        let cold_many = card(ProblemPattern::Measured, Rating::High, false, 4);
        assert!(rank(&hot, 1, "z", 9) < rank(&cold_many, 1, "a", 1));

        // KEY 4 claimed finding count: heat ties, so more claims precede.
        let two_claims = card(ProblemPattern::Measured, Rating::High, false, 2);
        assert!(rank(&two_claims, 1, "z", 9) < rank(&high_card, 1, "a", 1));

        // KEY 5 pattern: every count ties, so the frozen claiming order
        // decides and a tangle precedes a measured card.
        let tangle = card(ProblemPattern::Tangle, Rating::High, false, 1);
        assert!(rank(&tangle, 1, "z", 9) < rank(&high_card, 1, "a", 1));

        // KEY 6 the accepted finding rank: the pattern ties, so the top
        // claimed finding decides.
        let worst = Finding::new(
            FindingId::from_index(0),
            FileId::from_index(0),
            UnitIdentity::new("worst", UnitKind::Function),
            SourceSpan::new(1, 1),
            high(),
            HealthPolicy::default().assess(high()),
        );
        let lesser = Finding::new(
            FindingId::from_index(1),
            FileId::from_index(1),
            UnitIdentity::new("lesser", UnitKind::Function),
            SourceSpan::new(1, 1),
            watch(),
            HealthPolicy::default().assess(watch()),
        );
        assert!(
            ProblemRank::new(
                &high_card,
                1,
                Some(FindingRank::new(&worst, false, 0, "z")),
                "z",
                9
            ) < ProblemRank::new(
                &high_card,
                1,
                Some(FindingRank::new(&lesser, false, 0, "a")),
                "a",
                1
            )
        );

        // KEY 7 anchor path: every earlier key ties, so the earlier path
        // precedes.
        assert!(rank(&high_card, 1, "a", 9) < rank(&high_card, 1, "b", 1));

        // KEY 8 anchor start line: the path ties too, so the earlier start
        // line precedes.
        assert!(rank(&high_card, 1, "a", 1) < rank(&high_card, 1, "a", 2));
    }

    #[test]
    fn two_cards_that_tie_on_every_key_before_the_anchor_still_order() {
        let left = card(ProblemPattern::Measured, Rating::High, true, 2);
        let right = card(ProblemPattern::Measured, Rating::High, true, 2);
        let key = |card: &ProblemCard, path, line| ProblemRank::new(card, 2, None, path, line);
        assert_eq!(key(&left, "a", 4), key(&right, "a", 4));
        assert!(key(&left, "a", 4) < key(&right, "b", 1));
        assert!(key(&left, "a", 4) < key(&right, "a", 5));
    }

    #[test]
    fn the_table_is_ordered_once_by_the_problem_rank() {
        let mut tables = Tables::default();
        let package = tables.package("a");
        let quiet = tables.file("a/quiet.rs", package, 1, 0);
        tables.finding(quiet, "quiet", watch(), 2);
        let loud = tables.file("a/loud.rs", package, 3, 3);
        for index in 0..3 {
            tables.finding(loud, &format!("loud{index}"), high(), index + 1);
        }
        let cards = tables.cluster();
        assert_eq!(
            cards
                .iter()
                .map(|card| card.anchor().clone())
                .collect::<Vec<_>>(),
            [
                ProblemAnchor::File(FileId::from_index(loud)),
                ProblemAnchor::File(FileId::from_index(quiet)),
            ]
        );
    }

    #[test]
    fn clustering_an_empty_report_produces_no_card() {
        assert!(cluster_problems(ProblemInput::new(&[], &[])).is_empty());
    }
}
