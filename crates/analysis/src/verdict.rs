use crate::health::HealthCounts;

/// The frozen codebase answer.
///
/// The identifiers are the machine contract and never change without a
/// contract review. The sentences are copy owned by analysis, so a terminal
/// renderer and a machine consumer print the same bytes for the same tier.
///
/// The declaration order is the severity order used by architecture
/// escalation, which may raise a tier but never lower one.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CodebaseTier {
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

#[cfg(test)]
mod tests {
    use super::*;

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
