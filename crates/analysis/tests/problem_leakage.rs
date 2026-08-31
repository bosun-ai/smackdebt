//! How a leakage finding reaches a reader: as one more line on the card that
//! already names its subject, and as a card of its own only where none does.
//!
//! These read clustering the way a report does — borrowed tables in, ranked
//! cards out — and live beside the crate rather than inside the module because
//! the module's own test container is already at its size limit.

use smackdebt_analysis::{
    ChangeLeakageFinding, ChangeLeakageFindingId, ClaimedFinding, Coverage, DependencyEdge,
    DependencyEdgeId, FileChangeCoupling, FileChangeCouplingId, FileId, FileRecord, Finding,
    FindingId, HealthCounts, HealthPolicy, Measurements, PackageId, PackageRecord, ProblemAnchor,
    ProblemCard, ProblemEvidence, ProblemInput, ProblemPattern, ProblemVisibility, Rating, ScopeId,
    SizeFinding, SizeFindingId, SizePolicy, Thresholds, UnitIdentity, UnitKind, cluster_problems,
    duplicate_claim,
};

/// Hand-built report tables, so clustering is exercised without composing a
/// report.
#[derive(Default)]
struct Tables {
    files: Vec<FileRecord>,
    findings: Vec<Finding>,
    packages: Vec<PackageRecord>,
    sizes: Vec<SizeFinding>,
    edges: Vec<DependencyEdge>,
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

    /// One file holding `high_units` units rated High.
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
            self.finding(index, &format!("high{unit}"), unit + 1);
        }
        index
    }

    /// One source finding that rates High on cognitive complexity alone.
    fn finding(&mut self, file: usize, name: &str, line: u32) {
        let measurements = Measurements::new(25, 1, 1);
        self.findings.push(Finding::new(
            FindingId::from_index(self.findings.len()),
            FileId::from_index(file),
            UnitIdentity::new(name, UnitKind::Function),
            smackdebt_analysis::SourceSpan::new(line, line),
            measurements,
            HealthPolicy::default().assess(measurements),
        ));
    }

    fn size(&mut self, file: usize, lines: u32) {
        let policy = SizePolicy::new(Thresholds::new(10, 20), Thresholds::new(10, 20));
        self.sizes
            .push(policy.rate_file(FileId::from_index(file), lines).unwrap());
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

    /// Records one retained pair and returns its index, which a leakage
    /// finding names.
    fn pair(&mut self, left: usize, right: usize) -> FileChangeCouplingId {
        let index = self.pairs.len();
        self.pairs.push(FileChangeCoupling::new(
            FileId::from_index(left.min(right)),
            FileId::from_index(left.max(right)),
            7,
            12,
            3,
        ));
        FileChangeCouplingId::from_index(index)
    }

    fn leaky(&mut self, interface: usize, follower: usize) {
        let coupling = self.pair(interface, follower);
        self.leakage.push(ChangeLeakageFinding::leaky(
            coupling,
            FileId::from_index(interface),
        ));
    }

    fn hidden(&mut self, left: usize, right: usize) {
        let coupling = self.pair(left, right);
        self.leakage.push(ChangeLeakageFinding::hidden(coupling));
    }

    fn input(&self) -> ProblemInput<'_> {
        ProblemInput::new(&self.files, &self.findings)
            .with_packages(&self.packages)
            .with_size_findings(&self.sizes)
            .with_architecture(&[], &self.edges)
            .with_change_leakage(&self.leakage, &self.pairs)
    }

    fn cluster(&self) -> Vec<ProblemCard> {
        cluster_problems(self.input())
    }

    /// Both halves of the index-integrity audit over every claimable table,
    /// the change-leakage table included.
    fn assert_every_finding_is_claimed_once(&self) {
        let cards = self.cluster();
        assert_eq!(duplicate_claim(&cards), None, "a finding was claimed twice");
        let claimed: Vec<ClaimedFinding> = cards
            .iter()
            .flat_map(|card| card.claimed_findings().iter().copied())
            .collect();
        for id in 0..self.findings.len() {
            let claim = ClaimedFinding::Source(FindingId::from_index(id));
            assert!(claimed.contains(&claim), "{claim:?} reached no card");
        }
        for id in 0..self.sizes.len() {
            let claim = ClaimedFinding::Size(SizeFindingId::from_index(id));
            assert!(claimed.contains(&claim), "{claim:?} reached no card");
        }
        for id in 0..self.leakage.len() {
            let claim = ClaimedFinding::ChangeLeakage(ChangeLeakageFindingId::from_index(id));
            assert!(claimed.contains(&claim), "{claim:?} reached no card");
        }
    }
}

/// One subject file carrying `high_units` High findings, and a second file in
/// another directory the leakage findings can name.
fn leaking_pair(high_units: u32) -> Tables {
    let mut tables = Tables::default();
    tables.package("a");
    tables.file("a/subject.rs", high_units);
    tables.file("b/other.rs", 0);
    tables
}

fn leakage(position: usize) -> ClaimedFinding {
    ClaimedFinding::ChangeLeakage(ChangeLeakageFindingId::from_index(position))
}

#[test]
fn a_card_that_already_names_the_file_claims_its_leakage_findings() {
    // The subject does too much, so the leakage findings are two more lines on
    // the card that already names it rather than a second card.
    let mut tables = leaking_pair(3);
    tables.size(0, 40);
    tables.leaky(0, 1);
    tables.leaky(0, 1);
    let cards = tables.cluster();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].pattern(), ProblemPattern::GodFile);
    assert!(cards[0].claimed_findings().contains(&leakage(0)));
    assert!(cards[0].claimed_findings().contains(&leakage(1)));
    // The leakage evidence sits after the facts that made the pattern fire and
    // before the size finding the card also claims.
    assert_eq!(
        &cards[0].evidence()[1..5],
        [
            ProblemEvidence::RatedUnits(3),
            ProblemEvidence::ChangeLeakage(ChangeLeakageFindingId::from_index(0)),
            ProblemEvidence::ChangeLeakage(ChangeLeakageFindingId::from_index(1)),
            ProblemEvidence::Size(SizeFindingId::from_index(0)),
        ]
    );
    tables.assert_every_finding_is_claimed_once();
}

#[test]
fn an_interface_nothing_else_names_reaches_its_own_card() {
    // The file holds no source finding and no size finding, so the fallback
    // never fires and the leakage pattern names it instead.
    let mut tables = leaking_pair(0);
    tables.leaky(0, 1);
    tables.leaky(0, 1);
    let cards = tables.cluster();
    assert_eq!(cards.len(), 1);
    let card = &cards[0];
    assert_eq!(card.pattern(), ProblemPattern::LeakyInterface);
    assert_eq!(card.anchor(), &ProblemAnchor::File(FileId::from_index(0)));
    assert_eq!(card.rating(), Rating::Watch);
    assert_eq!(card.visibility(), ProblemVisibility::Default);
    // The follower count survives the tightest rung; the followers themselves
    // are the detail a wider rung buys.
    assert_eq!(
        card.evidence(),
        [
            ProblemEvidence::Followers(2),
            ProblemEvidence::ChangeLeakage(ChangeLeakageFindingId::from_index(0)),
            ProblemEvidence::ChangeLeakage(ChangeLeakageFindingId::from_index(1)),
        ]
    );
    tables.assert_every_finding_is_claimed_once();
}

#[test]
fn a_healthy_hub_that_leaks_is_rated_and_shown_by_default() {
    // Eight importers make the subject a hub; without a finding of its own it
    // is the `detail` card that happens to be healthy.
    let mut quiet = Tables::default();
    quiet.package("a");
    quiet.file("a/subject.rs", 0);
    for index in 0..8 {
        let importer = quiet.file(&format!("a/importer{index}.rs"), 0);
        quiet.link(importer, 0);
    }
    let cards = quiet.cluster();
    assert_eq!(cards[0].pattern(), ProblemPattern::Hub);
    assert_eq!(cards[0].rating(), Rating::Healthy);
    assert_eq!(cards[0].visibility(), ProblemVisibility::Detail);

    // One leakage finding gives the same card a rating and the default view.
    quiet.leaky(0, 1);
    let cards = quiet.cluster();
    assert_eq!(cards[0].pattern(), ProblemPattern::Hub);
    assert_eq!(
        cards[0].rating(),
        Rating::Watch,
        "the card takes the rating of the finding it claims"
    );
    assert_eq!(cards[0].visibility(), ProblemVisibility::Default);
    quiet.assert_every_finding_is_claimed_once();
}

#[test]
fn a_hidden_pair_is_claimed_by_its_lower_indexed_file_or_by_its_own_card() {
    // Neither file carries a card, so the pair reaches one of its own.
    let mut alone = leaking_pair(0);
    alone.hidden(0, 1);
    let cards = alone.cluster();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].pattern(), ProblemPattern::HiddenCoupling);
    assert_eq!(
        cards[0].anchor(),
        &ProblemAnchor::Files(vec![FileId::from_index(0), FileId::from_index(1)])
    );
    assert_eq!(cards[0].visibility(), ProblemVisibility::Default);
    alone.assert_every_finding_is_claimed_once();

    // The lower-indexed file carries a card, so that card claims the finding.
    let mut lower = leaking_pair(3);
    lower.size(0, 40);
    lower.hidden(0, 1);
    let cards = lower.cluster();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].pattern(), ProblemPattern::GodFile);
    lower.assert_every_finding_is_claimed_once();

    // Only the higher-indexed file carries one, so the pair still reaches a
    // card of its own and the other card is unchanged.
    let mut higher = Tables::default();
    higher.package("a");
    higher.file("a/plain.rs", 0);
    higher.file("b/loud.rs", 3);
    higher.size(1, 40);
    higher.hidden(0, 1);
    let patterns: Vec<_> = higher.cluster().iter().map(ProblemCard::pattern).collect();
    assert_eq!(
        patterns,
        [ProblemPattern::GodFile, ProblemPattern::HiddenCoupling]
    );
    higher.assert_every_finding_is_claimed_once();
}

/// One file can be both the interface its importers follow and the
/// lower-indexed member of a pair no dependency explains.
///
/// The leakage patterns run in declaration order, so `leaky_interface` cards
/// the file first and — being file-anchored — claims every leakage finding
/// that belongs to it, hidden findings included. The pair therefore reaches no
/// second card, which is the same rule that keeps a `god_file` from being
/// named twice.
#[test]
fn a_file_that_leaks_and_hides_reaches_one_card_that_claims_both() {
    let mut tables = leaking_pair(0);
    tables.file("c/third.rs", 0);
    tables.leaky(0, 1);
    tables.hidden(0, 2);
    let cards = tables.cluster();
    assert_eq!(cards.len(), 1, "one file, one file-anchored card");
    let card = &cards[0];
    assert_eq!(card.pattern(), ProblemPattern::LeakyInterface);
    assert_eq!(card.anchor(), &ProblemAnchor::File(FileId::from_index(0)));
    assert_eq!(card.claimed_findings(), [leakage(0), leakage(1)]);
    // The follower count counts the importers that follow the file, so the
    // hidden pair it also claims never inflates it.
    assert_eq!(
        card.evidence(),
        [
            ProblemEvidence::Followers(1),
            ProblemEvidence::ChangeLeakage(ChangeLeakageFindingId::from_index(0)),
            ProblemEvidence::ChangeLeakage(ChangeLeakageFindingId::from_index(1)),
        ]
    );
    tables.assert_every_finding_is_claimed_once();
}

#[test]
fn a_leakage_finding_alone_never_brings_a_measured_card_into_existence() {
    let mut tables = leaking_pair(0);
    tables.leaky(0, 1);
    assert_eq!(
        tables.cluster()[0].pattern(),
        ProblemPattern::LeakyInterface,
        "the two patterns behind `measured` exist to name exactly this case"
    );
    // A file that reaches `measured` for its own reasons still claims the
    // leakage finding that belongs to it.
    let mut measured = leaking_pair(1);
    measured.hidden(0, 1);
    let cards = measured.cluster();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].pattern(), ProblemPattern::Measured);
    assert!(cards[0].claimed_findings().contains(&leakage(0)));
    measured.assert_every_finding_is_claimed_once();
}

#[test]
fn every_card_that_existed_before_the_vocabulary_grew_is_unchanged() {
    let mut tables = leaking_pair(3);
    tables.size(0, 40);
    let before = cluster_problems(
        ProblemInput::new(&tables.files, &tables.findings)
            .with_packages(&tables.packages)
            .with_size_findings(&tables.sizes)
            .with_architecture(&[], &tables.edges),
    );
    // The same report with an empty leakage table states the same cards, so
    // the two tail patterns add nothing where nothing leaks.
    assert_eq!(before, tables.cluster());
}
