//! What a `hub` card needs before it reaches a reader: a degree that stands
//! out, and co-change proof that the degree costs something.
//!
//! A degree on its own is a shape rather than a problem — a view imports many
//! components and an error module is imported everywhere because that is what
//! each is for — so a file carrying debt of its own is named by that debt
//! unless it is hot or a change-leakage finding names it. A file carrying no
//! such debt keeps a `detail` card, which leaves the degree inspectable
//! without spending the default view on it.
//!
//! These read clustering the way a report does — borrowed tables in, ranked
//! cards out — and live beside the crate rather than inside the module because
//! its own test container is already at its size limit.

use smackdebt_analysis::{
    ChangeLeakageFinding, ChangeLeakageFindingId, ClaimedFinding, Coverage, DependencyEdge,
    DependencyEdgeId, FileChangeCoupling, FileChangeCouplingId, FileId, FileReach, FileRecord,
    Finding, FindingId, HealthCounts, HealthPolicy, Hotspot, Measurements, PackageId,
    PackageRecord, ProblemCard, ProblemEvidence, ProblemInput, ProblemPattern, ProblemVisibility,
    Rating, ScopeId, SourceSpan, UnitIdentity, UnitKind, cluster_problems, duplicate_claim,
};

/// Hand-built report tables, so clustering is exercised without composing a
/// report.
#[derive(Default)]
struct Tables {
    files: Vec<FileRecord>,
    findings: Vec<Finding>,
    packages: Vec<PackageRecord>,
    edges: Vec<DependencyEdge>,
    hotspots: Vec<Hotspot>,
    reach: Vec<FileReach>,
    pairs: Vec<FileChangeCoupling>,
    leakage: Vec<ChangeLeakageFinding>,
}

impl Tables {
    /// One package every file of these tables belongs to.
    fn package(&mut self, path: &str) -> usize {
        let index = self.packages.len();
        self.packages.push(PackageRecord::current(
            PackageId::from_index(index),
            ScopeId::from_index(0),
            path,
        ));
        index
    }

    /// One file holding `high_units` units rated High, each of which is a
    /// finding that affects the verdict.
    fn file(&mut self, path: &str, high_units: u32) -> usize {
        let index = self.files.len();
        self.files.push(
            FileRecord::new(
                FileId::from_index(index),
                ScopeId::from_index(0),
                path,
                Coverage::default(),
                HealthCounts::new(0, 0, high_units),
            )
            .with_package(PackageId::from_index(0)),
        );
        for unit in 0..high_units {
            let measurements = Measurements::new(25, 1, 1);
            self.findings.push(Finding::new(
                FindingId::from_index(self.findings.len()),
                FileId::from_index(index),
                UnitIdentity::new(format!("high{unit}"), UnitKind::Function),
                SourceSpan::new(unit + 1, unit + 1),
                measurements,
                HealthPolicy::default().assess(measurements),
            ));
        }
        index
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

    fn hotspot(&mut self, file: usize, touches: u32) {
        self.hotspots.push(Hotspot::new(
            FileId::from_index(file),
            Rating::High,
            touches,
        ));
    }

    /// Records the exact reach of one candidate file, keeping the table in
    /// file order the way the report builds it.
    fn reach(&mut self, file: usize, reach: u32) {
        self.reach
            .push(FileReach::new(FileId::from_index(file), reach));
        self.reach.sort_unstable_by_key(|value| value.file());
    }

    /// One leakage finding naming `interface` as the file its importer follows.
    fn leaky(&mut self, interface: usize, follower: usize) {
        let coupling = FileChangeCouplingId::from_index(self.pairs.len());
        self.pairs.push(FileChangeCoupling::new(
            FileId::from_index(interface.min(follower)),
            FileId::from_index(interface.max(follower)),
            7,
            12,
            3,
        ));
        self.leakage.push(ChangeLeakageFinding::leaky(
            coupling,
            FileId::from_index(interface),
        ));
    }

    fn cluster(&self) -> Vec<ProblemCard> {
        cluster_problems(
            ProblemInput::new(&self.files, &self.findings)
                .with_packages(&self.packages)
                .with_architecture(&[], &self.edges)
                .with_hotspots(&self.hotspots)
                .with_file_reach(&self.reach)
                .with_change_leakage(&self.leakage, &self.pairs),
        )
    }

    /// The card the subject file carries, which is always the first card
    /// because the subject is the only file these tables give any debt.
    fn subject(&self) -> ProblemCard {
        let cards = self.cluster();
        assert_eq!(duplicate_claim(&cards), None, "a finding was claimed twice");
        cards.into_iter().next().expect("the subject has a card")
    }
}

/// A package whose median fan-in is zero, where eight files import one subject
/// that holds `high_units` High findings of its own.
fn imported_file(high_units: u32) -> Tables {
    let mut tables = Tables::default();
    tables.package("a");
    tables.file("a/subject.rs", high_units);
    for index in 0..8 {
        let importer = tables.file(&format!("a/importer{index}.rs"), 0);
        tables.link(importer, 0);
    }
    tables
}

#[test]
fn a_hot_file_whose_fan_in_stands_out_is_a_hub_shown_by_default() {
    // Heat is co-change proof: the degree costs something because the file
    // keeps moving under everything that imports it.
    let mut tables = imported_file(1);
    tables.hotspot(0, 14);
    let card = tables.subject();
    assert_eq!(card.pattern(), ProblemPattern::Hub);
    assert_eq!(card.rating(), Rating::High);
    assert_eq!(card.visibility(), ProblemVisibility::Default);
    assert!(card.evidence().contains(&ProblemEvidence::FanIn(8)));
    assert_eq!(
        card.claimed_findings(),
        [ClaimedFinding::Source(FindingId::from_index(0))]
    );
}

#[test]
fn a_cold_file_carrying_debt_is_named_by_that_debt_and_not_by_its_fan_in() {
    // Being imported eight times is what an error module is for, so with
    // nothing corroborating it the degree names no problem a reader can act
    // on. The file falls through to the fallback, which heads on the finding
    // it holds, and that finding still reaches exactly one card.
    let card = imported_file(1).subject();
    assert_eq!(card.pattern(), ProblemPattern::Measured);
    assert_eq!(card.rating(), Rating::High);
    assert_eq!(card.visibility(), ProblemVisibility::Default);
    assert_eq!(
        card.evidence(),
        [ProblemEvidence::Finding(FindingId::from_index(0))],
        "the head is the file's own debt and the degree states nothing"
    );
    assert_eq!(
        card.claimed_findings(),
        [ClaimedFinding::Source(FindingId::from_index(0))]
    );
}

#[test]
fn a_cold_file_with_no_debt_keeps_the_hub_card_its_degree_earns_as_detail() {
    // There is no debt for the card to be named by, so the pattern still
    // states the degree — where a reader who asked for every card, or for this
    // file, can find it.
    let card = imported_file(0).subject();
    assert_eq!(card.pattern(), ProblemPattern::Hub);
    assert_eq!(card.rating(), Rating::Healthy);
    assert_eq!(card.visibility(), ProblemVisibility::Detail);
    assert_eq!(card.evidence(), [ProblemEvidence::FanIn(8)]);
    assert!(card.claimed_findings().is_empty());
}

#[test]
fn a_leakage_finding_corroborates_a_degree_as_well_as_heat_does() {
    // The file never changes often enough to be a hotspot, but its importers
    // follow its changes, which is the same proof that the degree costs
    // something.
    let mut tables = imported_file(1);
    tables.leaky(0, 1);
    let card = tables.subject();
    assert_eq!(card.pattern(), ProblemPattern::Hub);
    assert_eq!(card.visibility(), ProblemVisibility::Default);
    assert!(card.evidence().contains(&ProblemEvidence::FanIn(8)));
    assert!(
        card.claimed_findings()
            .contains(&ClaimedFinding::ChangeLeakage(
                ChangeLeakageFindingId::from_index(0)
            ))
    );
}

/// A file whose exact reach alone can make the pattern fire: no degree of its
/// own stands out, so only the `spreads` arm is left.
fn far_reaching_file(high_units: u32) -> Tables {
    let mut tables = Tables::default();
    tables.package("a");
    tables.file("a/subject.rs", high_units);
    tables.reach(0, 12);
    tables
}

#[test]
fn the_spreads_arm_asks_for_the_same_proof_the_two_degrees_do() {
    // A module that reaches everything by design is the same truism a file
    // eight others import is, so the third arm is gated the same way.
    let quiet = far_reaching_file(0).subject();
    assert_eq!(quiet.pattern(), ProblemPattern::Hub);
    assert_eq!(quiet.evidence(), [ProblemEvidence::ReachIn(12)]);
    assert_eq!(
        quiet.visibility(),
        ProblemVisibility::Detail,
        "a reach with no debt behind it stays inspectable rather than shown"
    );
    assert_eq!(
        far_reaching_file(1).subject().pattern(),
        ProblemPattern::Measured,
        "reach alone never renames a file that carries debt"
    );
    let mut hot = far_reaching_file(1);
    hot.hotspot(0, 14);
    let card = hot.subject();
    assert_eq!(card.pattern(), ProblemPattern::Hub);
    assert_eq!(card.visibility(), ProblemVisibility::Default);
    assert!(card.evidence().contains(&ProblemEvidence::ReachIn(12)));
}

#[test]
fn the_rule_moves_no_rating_no_count_and_no_claim() {
    // A card is display grouping, so gating one changes which card names a
    // file and nothing about what the report measured: the same finding is
    // claimed once, by one card, at the same rating, in the same view.
    let cold = imported_file(1).subject();
    let mut warm = imported_file(1);
    warm.hotspot(0, 14);
    let warm = warm.subject();
    assert_ne!(cold.pattern(), warm.pattern());
    assert_eq!(cold.rating(), warm.rating());
    assert_eq!(cold.visibility(), warm.visibility());
    assert_eq!(cold.claimed_findings(), warm.claimed_findings());
    assert_eq!(cold.anchor(), warm.anchor());
}
