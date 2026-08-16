use crate::architecture::{
    ArchitectureComparison, ArchitectureComparisonId, ArchitectureComparisonKind,
};
use crate::comparison::{Comparison, ComparisonKind};
use crate::evolution::{EvolutionaryComparison, EvolutionaryComparisonId};
use crate::health::{HealthCounts, Rating};
use crate::report::{ComparisonId, DiffCounts};
use crate::source::{SourceRole, UnitIdentity};

/// The frozen codebase answer.
///
/// The identifiers are the machine contract and never change without a
/// contract review. The sentences are copy owned by analysis, so a terminal
/// renderer and a machine consumer print the same bytes for the same tier.
///
/// The declaration order is the severity order used by architecture
/// escalation, which may raise a tier but never lower one.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CodebaseTier {
    /// Nothing was checked, which is also the answer before a report is built.
    #[default]
    Empty,
    Clean,
    Solid,
    Worn,
    FightsBack,
    Lost,
}

impl CodebaseTier {
    /// The frozen machine identifier of this tier.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Clean => "clean",
            Self::Solid => "solid",
            Self::Worn => "worn",
            Self::FightsBack => "fights_back",
            Self::Lost => "lost",
        }
    }

    /// The exact sentence every consumer prints for this tier.
    pub const fn sentence(self) -> &'static str {
        match self {
            Self::Empty => "Nothing was checked.",
            Self::Clean => "Clean. Ship it.",
            Self::Solid => "Solid, with rough edges.",
            Self::Worn => "Worn in the usual places.",
            Self::FightsBack => "This code fights back.",
            Self::Lost => "The code is winning.",
        }
    }

    /// Selects the tier for one scope's counts.
    ///
    /// Mapping runs first over integer permille of High debt, then
    /// architecture escalation raises the result when structural debt exists
    /// that a unit ratio cannot see.
    pub const fn select(counts: VerdictCounts) -> Self {
        let mapped = counts.mapped_tier();
        let floor = counts.architecture_floor();
        if floor as u8 > mapped as u8 {
            floor
        } else {
            mapped
        }
    }
}

/// The exact counts one verdict was computed from.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct VerdictCounts {
    checked: u32,
    watch: u32,
    high: u32,
    high_architecture_findings: u32,
}

/// The High permille at which a codebase stops being `worn`.
pub const WORN_PERMILLE: u32 = 10;

/// The High permille at which a codebase stops fighting back and is lost.
pub const FIGHTS_BACK_PERMILLE: u32 = 50;

impl VerdictCounts {
    /// Retains one scope's rated unit counts and its High architecture
    /// findings, which are the package dependency cycles.
    pub const fn new(health: HealthCounts, high_architecture_findings: u32) -> Self {
        Self {
            checked: health.total(),
            watch: health.watch(),
            high: health.high(),
            high_architecture_findings,
        }
    }

    /// The rated units this scope checked.
    pub const fn checked(self) -> u32 {
        self.checked
    }
    pub const fn watch(self) -> u32 {
        self.watch
    }
    pub const fn high(self) -> u32 {
        self.high
    }
    /// The High-rated architecture findings that can escalate the tier.
    pub const fn high_architecture_findings(self) -> u32 {
        self.high_architecture_findings
    }

    /// High debt per thousand checked units.
    ///
    /// The value is an integer division of `high * 1000` by checked units, so
    /// no floating-point value participates in tier selection. A scope that
    /// checked nothing has no density and reports zero.
    pub const fn high_permille(self) -> u32 {
        if self.checked == 0 {
            return 0;
        }
        // u64 keeps the scaled numerator exact for any u32 High count.
        ((self.high as u64 * 1000) / self.checked as u64) as u32
    }

    const fn mapped_tier(self) -> CodebaseTier {
        if self.checked == 0 {
            return CodebaseTier::Empty;
        }
        if self.high == 0 {
            return if self.watch == 0 {
                CodebaseTier::Clean
            } else {
                CodebaseTier::Solid
            };
        }
        match self.high_permille() {
            permille if permille <= WORN_PERMILLE => CodebaseTier::Worn,
            permille if permille <= FIGHTS_BACK_PERMILLE => CodebaseTier::FightsBack,
            _ => CodebaseTier::Lost,
        }
    }

    const fn architecture_floor(self) -> CodebaseTier {
        match self.high_architecture_findings {
            0 => CodebaseTier::Empty,
            1 | 2 => CodebaseTier::Worn,
            _ => CodebaseTier::FightsBack,
        }
    }
}

/// The frozen diff answer.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DiffTier {
    NoDebtChange,
    Better,
    Worse,
    Mixed,
}

impl DiffTier {
    /// The frozen machine identifier of this tier.
    pub const fn id(self) -> &'static str {
        match self {
            Self::NoDebtChange => "no_debt_change",
            Self::Better => "better",
            Self::Worse => "worse",
            Self::Mixed => "mixed",
        }
    }

    /// The exact sentence every consumer prints for this tier.
    pub const fn sentence(self) -> &'static str {
        match self {
            Self::NoDebtChange => "No debt changed.",
            Self::Better => "You made it better.",
            Self::Worse => "You made it worse.",
            Self::Mixed => "Better here, worse there.",
        }
    }

    /// Reconciles all three comparison families in one decision.
    ///
    /// A member that only changed a measurement moves the code in neither
    /// direction, so it is retained in the facts without deciding the tier.
    pub const fn reconcile(facts: DebtDiffFacts) -> Self {
        let total = facts.total();
        match (total.worse() > 0, total.better() > 0) {
            (true, true) => Self::Mixed,
            (true, false) => Self::Worse,
            (false, true) => Self::Better,
            (false, false) => Self::NoDebtChange,
        }
    }
}

/// One of the three comparison families a diff verdict reconciles.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DebtFamily {
    Source,
    Architecture,
    Evolutionary,
}

impl DebtFamily {
    /// Every family, in the order a report states them.
    pub const ALL: [Self; 3] = [Self::Source, Self::Architecture, Self::Evolutionary];

    /// The word a report uses to name this family.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Architecture => "architecture",
            Self::Evolutionary => "evolutionary",
        }
    }
}

/// The per-family direction counts behind one diff verdict.
///
/// Every family keeps its own counts, including zero counts, so a report can
/// name the family that moved instead of printing positional totals.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct DebtDiffFacts {
    source: DiffCounts,
    architecture: DiffCounts,
    evolutionary: DiffCounts,
}

impl DebtDiffFacts {
    pub const fn new(
        source: DiffCounts,
        architecture: DiffCounts,
        evolutionary: DiffCounts,
    ) -> Self {
        Self {
            source,
            architecture,
            evolutionary,
        }
    }

    pub const fn source(self) -> DiffCounts {
        self.source
    }
    pub const fn architecture(self) -> DiffCounts {
        self.architecture
    }
    pub const fn evolutionary(self) -> DiffCounts {
        self.evolutionary
    }

    /// The counts of one named family.
    pub const fn counts(self, family: DebtFamily) -> DiffCounts {
        match family {
            DebtFamily::Source => self.source,
            DebtFamily::Architecture => self.architecture,
            DebtFamily::Evolutionary => self.evolutionary,
        }
    }

    /// Whether one family moved the verdict in either direction.
    pub const fn moved(self, family: DebtFamily) -> bool {
        let counts = self.counts(family);
        counts.worse() > 0 || counts.better() > 0
    }

    /// The reconciled totals across all three families.
    pub const fn total(self) -> DiffCounts {
        DiffCounts::new(
            self.source.worse() + self.architecture.worse() + self.evolutionary.worse(),
            self.source.better() + self.architecture.better() + self.evolutionary.better(),
            self.source.changed() + self.architecture.changed() + self.evolutionary.changed(),
        )
    }
}

/// The typed identities that count as human debt movement in one scope.
///
/// The selection stores identities rather than copies, so the machine report
/// keeps every comparison while only these move a verdict or reach a human.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DebtDiffSelection {
    source: Vec<ComparisonId>,
    architecture: Vec<ArchitectureComparisonId>,
    evolutionary: Vec<EvolutionaryComparisonId>,
    facts: DebtDiffFacts,
}

impl DebtDiffSelection {
    /// Selects one source comparison when it moves human debt.
    ///
    /// Regressed and Improved units always move debt. An added or removed unit
    /// moves debt only while it is rated, so the healthy units a refactor adds
    /// or deletes stay out. A measurement change counts only while the unit is
    /// rated. Fixture and generated source never moves a verdict.
    pub fn select_source(&mut self, comparison: &Comparison, role: SourceRole) {
        if !moves_debt(comparison, role) {
            return;
        }
        self.source.push(comparison.id());
        self.facts.source.add_direction(comparison.direction());
    }

    /// Selects one architecture comparison that introduced or removed a cycle.
    pub fn select_architecture(&mut self, comparison: &ArchitectureComparison) {
        if !matches!(
            comparison.kind(),
            ArchitectureComparisonKind::CycleIntroduced | ArchitectureComparisonKind::CycleRemoved
        ) {
            return;
        }
        self.architecture.push(comparison.id());
        self.facts
            .architecture
            .add_direction(comparison.direction());
    }

    /// Selects one evolutionary comparison, each of which introduced or
    /// removed a finding.
    pub fn select_evolutionary(&mut self, comparison: EvolutionaryComparison) {
        self.evolutionary.push(comparison.id());
        self.facts
            .evolutionary
            .add_direction(comparison.direction());
    }

    pub fn source(&self) -> &[ComparisonId] {
        &self.source
    }
    pub fn architecture(&self) -> &[ArchitectureComparisonId] {
        &self.architecture
    }
    pub fn evolutionary(&self) -> &[EvolutionaryComparisonId] {
        &self.evolutionary
    }
    pub const fn facts(&self) -> DebtDiffFacts {
        self.facts
    }
    /// The reconciled tier of everything this selection holds.
    pub const fn tier(&self) -> DiffTier {
        DiffTier::reconcile(self.facts)
    }
    pub fn is_empty(&self) -> bool {
        self.source.is_empty() && self.architecture.is_empty() && self.evolutionary.is_empty()
    }

    /// Whether one scope's selection holds the same identity twice.
    ///
    /// A duplicate would count one movement twice, so index-integrity audits
    /// reject it. The audit sorts copies of the identity lists and never runs
    /// while a report is built.
    pub fn has_duplicate_identity(&self) -> bool {
        repeats(&self.source) || repeats(&self.architecture) || repeats(&self.evolutionary)
    }
}

/// Why one finding is the worst thing in a scope.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum WorstOffenderReason {
    HotAndComplex,
    MostComplex,
    PackageDependencyCycle,
}

impl WorstOffenderReason {
    /// The frozen machine identifier of this reason.
    ///
    /// A machine consumer reads these, so they are owned here beside the tier
    /// identifiers rather than invented by a renderer.
    pub const fn id(self) -> &'static str {
        match self {
            Self::HotAndComplex => "hot_and_complex",
            Self::MostComplex => "most_complex",
            Self::PackageDependencyCycle => "package_dependency_cycle",
        }
    }

    /// The exact words every consumer prints for this reason.
    pub const fn text(self) -> &'static str {
        match self {
            Self::HotAndComplex => "hot AND complex",
            Self::MostComplex => "most complex",
            Self::PackageDependencyCycle => "package dependency cycle",
        }
    }
}

/// One of the worst things in a scope, named with a resolved path.
///
/// The path and the unit identity are resolved while the report is built so a
/// consumer states the offender without joining index tables. Structural debt
/// has no unit, so its identity is absent rather than invented.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WorstOffender {
    path: String,
    identity: Option<UnitIdentity>,
    reason: WorstOffenderReason,
}

impl WorstOffender {
    pub fn new(path: impl Into<String>, reason: WorstOffenderReason) -> Self {
        Self {
            path: path.into(),
            identity: None,
            reason,
        }
    }

    /// Names the unit this offender is, for source debt.
    pub fn with_identity(mut self, identity: UnitIdentity) -> Self {
        self.identity = Some(identity);
        self
    }

    /// The repository-relative path of the offending source.
    pub fn path(&self) -> &str {
        &self.path
    }
    /// The offending unit, absent for structural debt.
    pub const fn identity(&self) -> Option<&UnitIdentity> {
        self.identity.as_ref()
    }
    pub const fn reason(&self) -> WorstOffenderReason {
        self.reason
    }
}

/// The most offenders one verdict names.
///
/// Three is enough for a reader and for a machine head, and it is bounded, so
/// selecting them never sorts a whole finding table.
pub const WORST_OFFENDER_LIMIT: usize = 3;

/// One completed answer for one scope.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Verdict {
    tier: CodebaseTier,
    counts: VerdictCounts,
    diff: Option<DiffTier>,
    selection: DebtDiffSelection,
    worst: Vec<WorstOffender>,
}

impl Verdict {
    /// Completes a codebase verdict, which reconciles no comparison.
    pub fn codebase(counts: VerdictCounts, worst: Vec<WorstOffender>) -> Self {
        Self {
            tier: CodebaseTier::select(counts),
            counts,
            diff: None,
            selection: DebtDiffSelection::default(),
            worst,
        }
    }

    /// Completes a diff verdict from one scope's debt-diff selection.
    pub fn diff(
        counts: VerdictCounts,
        selection: DebtDiffSelection,
        worst: Vec<WorstOffender>,
    ) -> Self {
        Self {
            tier: CodebaseTier::select(counts),
            counts,
            diff: Some(selection.tier()),
            selection,
            worst,
        }
    }

    pub const fn tier(&self) -> CodebaseTier {
        self.tier
    }
    pub const fn counts(&self) -> VerdictCounts {
        self.counts
    }
    /// The diff tier, present only for a diff report.
    pub const fn diff_tier(&self) -> Option<DiffTier> {
        self.diff
    }
    pub const fn selection(&self) -> &DebtDiffSelection {
        &self.selection
    }
    /// The per-family counts behind the diff tier.
    pub const fn facts(&self) -> DebtDiffFacts {
        self.selection.facts()
    }
    /// The worst thing in this scope, which a human report names first.
    pub fn worst_offender(&self) -> Option<&WorstOffender> {
        self.worst.first()
    }
    /// Every named offender, worst first and at most `WORST_OFFENDER_LIMIT`.
    pub fn worst(&self) -> &[WorstOffender] {
        &self.worst
    }
    /// The exact sentence this verdict answers with.
    pub const fn sentence(&self) -> &'static str {
        match self.diff {
            Some(tier) => tier.sentence(),
            None => self.tier.sentence(),
        }
    }
}

fn repeats<T: Copy + Ord>(identities: &[T]) -> bool {
    let mut sorted = identities.to_vec();
    sorted.sort_unstable();
    sorted.windows(2).any(|pair| pair[0] == pair[1])
}

fn moves_debt(comparison: &Comparison, role: SourceRole) -> bool {
    if !role.affects_verdict() {
        return false;
    }
    match comparison.kind() {
        ComparisonKind::Regressed | ComparisonKind::Improved => true,
        ComparisonKind::Added => is_rated(comparison.after_rating()),
        ComparisonKind::Removed => is_rated(comparison.before_rating()),
        ComparisonKind::MetricChanged => {
            is_rated(comparison.before_rating()) || is_rated(comparison.after_rating())
        }
        ComparisonKind::Ambiguous | ComparisonKind::Unchanged => false,
    }
}

const fn is_rated(rating: Option<Rating>) -> bool {
    matches!(rating, Some(Rating::Watch | Rating::High))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comparison::ComparisonDirection;
    use crate::evolution::{ChangeCoupling, EvolutionaryComparisonKind};
    use crate::health::Measurements;
    use crate::report::{FileId, PackageId};
    use crate::source::{UnitIdentity, UnitKind};

    fn source(
        index: usize,
        kind: ComparisonKind,
        before_rating: Option<Rating>,
        after_rating: Option<Rating>,
    ) -> Comparison {
        let measurements = Measurements::new(1, 1, 1);
        Comparison::new(
            ComparisonId::from_index(index),
            UnitIdentity::new("work", UnitKind::Function),
            kind,
            before_rating.map(|_| measurements),
            after_rating.map(|_| measurements),
            before_rating,
            after_rating,
        )
        .with_file(FileId::from_index(index))
    }

    fn architecture(index: usize, kind: ArchitectureComparisonKind) -> ArchitectureComparison {
        ArchitectureComparison::new(
            ArchitectureComparisonId::from_index(index),
            kind,
            vec![PackageId::from_index(0), PackageId::from_index(1)],
        )
    }

    fn evolutionary(index: usize, kind: EvolutionaryComparisonKind) -> EvolutionaryComparison {
        let direction = match kind {
            EvolutionaryComparisonKind::FindingIntroduced => ComparisonDirection::Worse,
            EvolutionaryComparisonKind::FindingRemoved => ComparisonDirection::Better,
        };
        EvolutionaryComparison::new(
            EvolutionaryComparisonId::from_index(index),
            kind,
            direction,
            ChangeCoupling::new(PackageId::from_index(0), PackageId::from_index(1), 5, 5),
        )
    }

    #[test]
    fn every_diff_tier_keeps_its_frozen_id_and_sentence() {
        let vocabulary = [
            (DiffTier::NoDebtChange, "no_debt_change", "No debt changed."),
            (DiffTier::Better, "better", "You made it better."),
            (DiffTier::Worse, "worse", "You made it worse."),
            (DiffTier::Mixed, "mixed", "Better here, worse there."),
        ];
        for (tier, id, sentence) in vocabulary {
            assert_eq!(tier.id(), id);
            assert_eq!(tier.sentence(), sentence);
        }
    }

    #[test]
    fn a_diff_that_moves_no_rated_debt_changed_nothing() {
        let mut selection = DebtDiffSelection::default();
        for index in 0..204 {
            selection.select_source(
                &source(index, ComparisonKind::Added, None, Some(Rating::Healthy)),
                SourceRole::Primary,
            );
            selection.select_source(
                &source(index, ComparisonKind::Removed, Some(Rating::Healthy), None),
                SourceRole::Primary,
            );
            selection.select_source(
                &source(
                    index,
                    ComparisonKind::Unchanged,
                    Some(Rating::High),
                    Some(Rating::High),
                ),
                SourceRole::Primary,
            );
            selection.select_source(
                &source(index, ComparisonKind::Ambiguous, None, None),
                SourceRole::Primary,
            );
        }
        assert!(selection.is_empty());
        assert_eq!(selection.tier(), DiffTier::NoDebtChange);
        assert_eq!(selection.facts().total(), DiffCounts::default());
    }

    #[test]
    fn regressed_improved_and_rated_membership_moves_the_tier() {
        let members = [
            (
                source(
                    0,
                    ComparisonKind::Regressed,
                    Some(Rating::Healthy),
                    Some(Rating::High),
                ),
                ComparisonDirection::Worse,
            ),
            (
                source(
                    1,
                    ComparisonKind::Improved,
                    Some(Rating::High),
                    Some(Rating::Healthy),
                ),
                ComparisonDirection::Better,
            ),
            (
                source(2, ComparisonKind::Added, None, Some(Rating::Watch)),
                ComparisonDirection::Worse,
            ),
            (
                source(3, ComparisonKind::Added, None, Some(Rating::High)),
                ComparisonDirection::Worse,
            ),
            (
                source(4, ComparisonKind::Removed, Some(Rating::Watch), None),
                ComparisonDirection::Better,
            ),
            (
                source(5, ComparisonKind::Removed, Some(Rating::High), None),
                ComparisonDirection::Better,
            ),
            (
                source(
                    6,
                    ComparisonKind::MetricChanged,
                    Some(Rating::Watch),
                    Some(Rating::Watch),
                ),
                ComparisonDirection::Changed,
            ),
        ];
        for (comparison, direction) in members {
            let mut selection = DebtDiffSelection::default();
            selection.select_source(&comparison, SourceRole::Primary);
            assert_eq!(
                selection.source(),
                &[comparison.id()],
                "{:?}",
                comparison.kind()
            );
            let mut expected = DiffCounts::default();
            expected.add_direction(direction);
            assert_eq!(selection.facts().source(), expected);
        }
    }

    #[test]
    fn a_healthy_metric_change_never_reaches_the_selection() {
        let mut selection = DebtDiffSelection::default();
        selection.select_source(
            &source(
                0,
                ComparisonKind::MetricChanged,
                Some(Rating::Healthy),
                Some(Rating::Healthy),
            ),
            SourceRole::Primary,
        );
        assert!(selection.source().is_empty());
    }

    #[test]
    fn fixture_and_generated_source_never_moves_the_verdict() {
        for role in [SourceRole::Fixture, SourceRole::Generated] {
            let mut selection = DebtDiffSelection::default();
            selection.select_source(
                &source(
                    0,
                    ComparisonKind::Regressed,
                    Some(Rating::Healthy),
                    Some(Rating::High),
                ),
                role,
            );
            assert!(selection.is_empty(), "{role:?}");
            assert_eq!(selection.tier(), DiffTier::NoDebtChange);
        }
        for role in [
            SourceRole::Primary,
            SourceRole::Test,
            SourceRole::Example,
            SourceRole::Benchmark,
        ] {
            let mut selection = DebtDiffSelection::default();
            selection.select_source(
                &source(
                    0,
                    ComparisonKind::Regressed,
                    Some(Rating::Healthy),
                    Some(Rating::High),
                ),
                role,
            );
            assert_eq!(selection.tier(), DiffTier::Worse, "{role:?}");
        }
    }

    #[test]
    fn only_introduced_and_removed_findings_join_the_other_two_families() {
        let mut selection = DebtDiffSelection::default();
        selection.select_architecture(&architecture(0, ArchitectureComparisonKind::EdgeAdded));
        selection.select_architecture(&architecture(1, ArchitectureComparisonKind::EdgeRemoved));
        assert!(selection.is_empty());
        selection.select_architecture(&architecture(
            2,
            ArchitectureComparisonKind::CycleIntroduced,
        ));
        selection.select_architecture(&architecture(3, ArchitectureComparisonKind::CycleRemoved));
        assert_eq!(
            selection.architecture(),
            &[
                ArchitectureComparisonId::from_index(2),
                ArchitectureComparisonId::from_index(3)
            ]
        );
        selection.select_evolutionary(evolutionary(
            0,
            EvolutionaryComparisonKind::FindingIntroduced,
        ));
        selection.select_evolutionary(evolutionary(1, EvolutionaryComparisonKind::FindingRemoved));
        assert_eq!(
            selection.evolutionary(),
            &[
                EvolutionaryComparisonId::from_index(0),
                EvolutionaryComparisonId::from_index(1)
            ]
        );
        assert_eq!(selection.facts().architecture(), DiffCounts::new(1, 1, 0));
        assert_eq!(selection.facts().evolutionary(), DiffCounts::new(1, 1, 0));
    }

    #[test]
    fn a_package_cycle_makes_a_diff_worse_when_no_source_comparison_moved() {
        let mut selection = DebtDiffSelection::default();
        selection.select_source(
            &source(
                0,
                ComparisonKind::Unchanged,
                Some(Rating::High),
                Some(Rating::High),
            ),
            SourceRole::Primary,
        );
        selection.select_architecture(&architecture(
            0,
            ArchitectureComparisonKind::CycleIntroduced,
        ));
        assert_eq!(selection.tier(), DiffTier::Worse);
        assert_eq!(selection.facts().source(), DiffCounts::default());
        assert!(!selection.facts().moved(DebtFamily::Source));
        assert!(selection.facts().moved(DebtFamily::Architecture));
        assert!(!selection.facts().moved(DebtFamily::Evolutionary));
    }

    #[test]
    fn the_reconciled_tier_covers_the_whole_truth_table() {
        let worse = DiffCounts::new(1, 0, 0);
        let better = DiffCounts::new(0, 1, 0);
        let both = DiffCounts::new(1, 1, 0);
        let changed = DiffCounts::new(0, 0, 1);
        let none = DiffCounts::default();
        let cases = [
            (none, none, none, DiffTier::NoDebtChange),
            (changed, none, none, DiffTier::NoDebtChange),
            (worse, none, none, DiffTier::Worse),
            (better, none, none, DiffTier::Better),
            (both, none, none, DiffTier::Mixed),
            (none, worse, none, DiffTier::Worse),
            (none, better, none, DiffTier::Better),
            (none, none, worse, DiffTier::Worse),
            (none, none, better, DiffTier::Better),
            (worse, better, none, DiffTier::Mixed),
            (better, worse, none, DiffTier::Mixed),
            (better, none, worse, DiffTier::Mixed),
            (worse, worse, worse, DiffTier::Worse),
            (better, better, better, DiffTier::Better),
        ];
        for (source, architecture, evolutionary, expected) in cases {
            let facts = DebtDiffFacts::new(source, architecture, evolutionary);
            assert_eq!(
                DiffTier::reconcile(facts),
                expected,
                "{source:?} {architecture:?} {evolutionary:?}"
            );
        }
    }

    #[test]
    fn every_family_count_is_retained_even_when_it_is_zero() {
        let facts = DebtDiffFacts::new(
            DiffCounts::new(1, 0, 0),
            DiffCounts::default(),
            DiffCounts::default(),
        );
        assert_eq!(facts.total(), DiffCounts::new(1, 0, 0));
        assert_eq!(facts.total().better(), 0);
        assert_eq!(facts.total().changed(), 0);
        for family in DebtFamily::ALL {
            assert_eq!(facts.counts(family).total(), facts.counts(family).worse());
        }
        assert_eq!(DebtFamily::Source.name(), "source");
        assert_eq!(DebtFamily::Architecture.name(), "architecture");
        assert_eq!(DebtFamily::Evolutionary.name(), "evolutionary");
    }

    #[test]
    fn every_worst_offender_reason_keeps_its_exact_words_and_identifier() {
        let vocabulary = [
            (
                WorstOffenderReason::HotAndComplex,
                "hot AND complex",
                "hot_and_complex",
            ),
            (
                WorstOffenderReason::MostComplex,
                "most complex",
                "most_complex",
            ),
            (
                WorstOffenderReason::PackageDependencyCycle,
                "package dependency cycle",
                "package_dependency_cycle",
            ),
        ];
        for (reason, words, id) in vocabulary {
            assert_eq!(reason.text(), words);
            assert_eq!(reason.id(), id);
        }
    }

    #[test]
    fn a_codebase_verdict_states_its_tier_sentence_and_offender() {
        let verdict = Verdict::codebase(
            counts(1_000, 4, 10, 0),
            vec![
                WorstOffender::new(
                    "crates/analysis/src/report.rs",
                    WorstOffenderReason::HotAndComplex,
                )
                .with_identity(UnitIdentity::new("aggregate", UnitKind::Function)),
            ],
        );
        assert_eq!(verdict.tier(), CodebaseTier::Worn);
        assert_eq!(verdict.sentence(), "Worn in the usual places.");
        assert_eq!(verdict.diff_tier(), None);
        assert_eq!(verdict.counts().high(), 10);
        let offender = verdict.worst_offender().unwrap();
        assert_eq!(offender.path(), "crates/analysis/src/report.rs");
        assert_eq!(offender.reason(), WorstOffenderReason::HotAndComplex);
        assert_eq!(
            offender.identity().map(UnitIdentity::name),
            Some("aggregate")
        );
        assert_eq!(verdict.worst().len(), 1);
        assert!(verdict.selection().is_empty());
    }

    #[test]
    fn a_diff_verdict_answers_with_its_diff_sentence() {
        let mut selection = DebtDiffSelection::default();
        selection.select_architecture(&architecture(
            0,
            ArchitectureComparisonKind::CycleIntroduced,
        ));
        let verdict = Verdict::diff(counts(1_000, 0, 0, 1), selection, Vec::new());
        assert_eq!(verdict.diff_tier(), Some(DiffTier::Worse));
        assert_eq!(verdict.sentence(), "You made it worse.");
        assert_eq!(verdict.tier(), CodebaseTier::Worn);
        assert!(verdict.facts().moved(DebtFamily::Architecture));
    }

    #[test]
    fn a_repeated_identity_fails_the_selection_audit() {
        let mut selection = DebtDiffSelection::default();
        let regression = source(
            0,
            ComparisonKind::Regressed,
            Some(Rating::Healthy),
            Some(Rating::High),
        );
        selection.select_source(&regression, SourceRole::Primary);
        assert!(!selection.has_duplicate_identity());
        selection.select_source(&regression, SourceRole::Primary);
        assert!(selection.has_duplicate_identity());
    }

    fn counts(checked: u32, watch: u32, high: u32, architecture: u32) -> VerdictCounts {
        let healthy = checked - watch - high;
        VerdictCounts::new(HealthCounts::new(healthy, watch, high), architecture)
    }

    #[test]
    fn every_tier_keeps_its_frozen_id_and_sentence() {
        let vocabulary = [
            (CodebaseTier::Empty, "empty", "Nothing was checked."),
            (CodebaseTier::Clean, "clean", "Clean. Ship it."),
            (CodebaseTier::Solid, "solid", "Solid, with rough edges."),
            (CodebaseTier::Worn, "worn", "Worn in the usual places."),
            (
                CodebaseTier::FightsBack,
                "fights_back",
                "This code fights back.",
            ),
            (CodebaseTier::Lost, "lost", "The code is winning."),
        ];
        for (tier, id, sentence) in vocabulary {
            assert_eq!(tier.id(), id);
            assert_eq!(tier.sentence(), sentence);
        }
    }

    #[test]
    fn nothing_checked_is_empty_whatever_else_is_known() {
        assert_eq!(
            CodebaseTier::select(counts(0, 0, 0, 0)),
            CodebaseTier::Empty
        );
        assert_eq!(counts(0, 0, 0, 0).high_permille(), 0);
    }

    #[test]
    fn checked_code_without_any_rated_unit_is_clean() {
        assert_eq!(
            CodebaseTier::select(counts(1_000, 0, 0, 0)),
            CodebaseTier::Clean
        );
    }

    #[test]
    fn watch_debt_without_high_debt_is_solid() {
        assert_eq!(
            CodebaseTier::select(counts(1_000, 1, 0, 0)),
            CodebaseTier::Solid
        );
    }

    #[test]
    fn one_percent_high_debt_belongs_to_the_lower_tier() {
        assert_eq!(counts(1_000, 0, 10, 0).high_permille(), 10);
        assert_eq!(
            CodebaseTier::select(counts(1_000, 0, 10, 0)),
            CodebaseTier::Worn
        );
        assert_eq!(counts(1_000, 0, 11, 0).high_permille(), 11);
        assert_eq!(
            CodebaseTier::select(counts(1_000, 0, 11, 0)),
            CodebaseTier::FightsBack
        );
        assert_eq!(counts(1_000, 0, 9, 0).high_permille(), 9);
        assert_eq!(
            CodebaseTier::select(counts(1_000, 0, 9, 0)),
            CodebaseTier::Worn
        );
    }

    #[test]
    fn five_percent_high_debt_belongs_to_the_lower_tier() {
        assert_eq!(counts(1_000, 0, 50, 0).high_permille(), 50);
        assert_eq!(
            CodebaseTier::select(counts(1_000, 0, 50, 0)),
            CodebaseTier::FightsBack
        );
        assert_eq!(counts(1_000, 0, 51, 0).high_permille(), 51);
        assert_eq!(
            CodebaseTier::select(counts(1_000, 0, 51, 0)),
            CodebaseTier::Lost
        );
        assert_eq!(
            CodebaseTier::select(counts(1_000, 0, 49, 0)),
            CodebaseTier::FightsBack
        );
    }

    #[test]
    fn the_permille_value_truncates_instead_of_rounding_up() {
        // Three High units in 1000 checked units is 3 permille exactly, and
        // seven in 999 is 7.007 permille, which truncates to 7. Neither
        // computation produces a floating-point value.
        assert_eq!(counts(1_000, 0, 3, 0).high_permille(), 3);
        assert_eq!(counts(999, 0, 7, 0).high_permille(), 7);
        assert_eq!(counts(3, 0, 1, 0).high_permille(), 333);
        assert_eq!(CodebaseTier::select(counts(3, 0, 1, 0)), CodebaseTier::Lost);
    }

    #[test]
    fn one_high_architecture_finding_floors_the_tier_at_worn() {
        assert_eq!(
            CodebaseTier::select(counts(1_000, 0, 0, 1)),
            CodebaseTier::Worn
        );
        assert_eq!(
            CodebaseTier::select(counts(1_000, 4, 0, 2)),
            CodebaseTier::Worn
        );
    }

    #[test]
    fn three_high_architecture_findings_floor_the_tier_at_fights_back() {
        assert_eq!(
            CodebaseTier::select(counts(1_000, 4, 0, 3)),
            CodebaseTier::FightsBack
        );
        assert_eq!(
            CodebaseTier::select(counts(1_000, 0, 0, 9)),
            CodebaseTier::FightsBack
        );
    }

    #[test]
    fn a_floor_never_lowers_an_already_higher_tier() {
        assert_eq!(
            CodebaseTier::select(counts(1_000, 0, 60, 1)),
            CodebaseTier::Lost
        );
        assert_eq!(
            CodebaseTier::select(counts(1_000, 0, 60, 3)),
            CodebaseTier::Lost
        );
        assert_eq!(
            CodebaseTier::select(counts(1_000, 0, 30, 1)),
            CodebaseTier::FightsBack
        );
    }

    #[test]
    fn an_unchecked_scope_with_a_package_cycle_still_reports_the_floor() {
        // Escalation is applied after mapping and only raises, so a scope whose
        // units were never rated still states the structural debt it carries.
        assert_eq!(CodebaseTier::select(counts(0, 0, 0, 1)), CodebaseTier::Worn);
        assert_eq!(
            CodebaseTier::select(counts(0, 0, 0, 3)),
            CodebaseTier::FightsBack
        );
    }

    #[test]
    fn the_verdict_retains_the_counts_it_was_computed_from() {
        let counts = counts(1_000, 4, 10, 1);
        assert_eq!(counts.checked(), 1_000);
        assert_eq!(counts.watch(), 4);
        assert_eq!(counts.high(), 10);
        assert_eq!(counts.high_architecture_findings(), 1);
    }
}
