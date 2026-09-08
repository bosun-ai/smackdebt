//! Threshold policy and aggregate counts for the five source measurements.
//!
//! Watch and High thresholds are inclusive. The highest of the five independent
//! ratings determines unit health. Defaults, as Watch/High, are cognitive 15/25,
//! cyclomatic 11/21, logical statements 50/100, nesting 4/7, and parameters 6/9.
//! All five assessments remain available to explain that result. Rating and
//! count aggregation use fixed-size values without allocating.

#![deny(missing_docs)]

use crate::Measurements;

/// A threshold pair for one signal.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Thresholds {
    watch: u32,
    high: u32,
}

impl Thresholds {
    /// Stores the inclusive Watch and High cutoffs supplied by configuration.
    pub const fn new(watch: u32, high: u32) -> Self {
        Self { watch, high }
    }

    /// The inclusive lower cutoff for a Watch rating.
    pub const fn watch(self) -> u32 {
        self.watch
    }

    /// The inclusive lower cutoff for a High rating.
    pub const fn high(self) -> u32 {
        self.high
    }

    pub(crate) const fn level(self, value: u32) -> Rating {
        if value >= self.high {
            Rating::High
        } else if value >= self.watch {
            Rating::Watch
        } else {
            Rating::Healthy
        }
    }
}

/// Independent health signals retained for an assessed unit.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Signal {
    /// Control-flow comprehension cost.
    CognitiveComplexity,
    /// Independent control-flow paths.
    CyclomaticComplexity,
    /// Exclusive logical statements.
    LogicalLines,
    /// Maximum nesting depth.
    MaxNesting,
    /// Declared parameter count.
    ParameterCount,
}

/// The severity of one signal or a unit as a whole.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Rating {
    /// No debt selected by the owning policy.
    Healthy,
    /// Debt worth inspecting under the owning policy.
    Watch,
    /// High-severity debt under the owning policy.
    High,
}

impl Rating {
    pub(crate) const fn rank(self) -> u8 {
        match self {
            Self::Healthy => 0,
            Self::Watch => 1,
            Self::High => 2,
        }
    }
}

/// Whether a side of a comparison exists and is rated debt.
///
/// A one-sided comparison only moves a verdict while the side it has is rated:
/// the healthy units a refactor adds or deletes are changes, not debt.
pub(crate) const fn is_rated(rating: Option<Rating>) -> bool {
    matches!(rating, Some(Rating::Watch | Rating::High))
}

/// One threshold result stored without allocation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SignalAssessment {
    signal: Signal,
    value: u32,
    rating: Rating,
}

impl SignalAssessment {
    /// The source measurement identified by this assessment.
    pub const fn signal(self) -> Signal {
        self.signal
    }

    /// The original integer measurement used to determine the rating.
    pub const fn value(self) -> u32 {
        self.value
    }

    /// The health rating carried by this observation.
    pub const fn rating(self) -> Rating {
        self.rating
    }
}

/// The health result for one unit.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct HealthAssessment {
    rating: Rating,
    signals: [SignalAssessment; 5],
}

impl HealthAssessment {
    /// The highest rating across all five source measurements.
    pub const fn rating(self) -> Rating {
        self.rating
    }

    /// All five assessments in measurement order, including healthy signals.
    pub const fn signals(self) -> [SignalAssessment; 5] {
        self.signals
    }

    /// The assessment for the requested source measurement.
    pub const fn signal(self, signal: Signal) -> SignalAssessment {
        match signal {
            Signal::CognitiveComplexity => self.signals[0],
            Signal::CyclomaticComplexity => self.signals[1],
            Signal::LogicalLines => self.signals[2],
            Signal::MaxNesting => self.signals[3],
            Signal::ParameterCount => self.signals[4],
        }
    }

    pub(crate) fn signals_at_rating(self) -> u8 {
        self.signals
            .iter()
            .filter(|signal| signal.rating == self.rating)
            .count() as u8
    }

    pub(crate) fn triggered_signals(self) -> u8 {
        self.signals
            .iter()
            .filter(|signal| signal.rating != Rating::Healthy)
            .count() as u8
    }
}

/// Configurable policy for the retained measurements.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct HealthPolicy {
    cognitive: Thresholds,
    cyclomatic: Thresholds,
    logical_lines: Thresholds,
    nesting: Thresholds,
    parameters: Thresholds,
}

impl Default for HealthPolicy {
    fn default() -> Self {
        Self {
            cognitive: Thresholds::new(15, 25),
            cyclomatic: Thresholds::new(11, 21),
            logical_lines: Thresholds::new(50, 100),
            nesting: Thresholds::new(4, 7),
            parameters: Thresholds::new(6, 9),
        }
    }
}

impl HealthPolicy {
    /// Sets the threshold pair for each of the five rated source measurements.
    pub const fn new(
        cognitive: Thresholds,
        cyclomatic: Thresholds,
        logical_lines: Thresholds,
        nesting: Thresholds,
        parameters: Thresholds,
    ) -> Self {
        Self {
            cognitive,
            cyclomatic,
            logical_lines,
            nesting,
            parameters,
        }
    }

    /// The inclusive thresholds for cognitive complexity.
    pub const fn cognitive(self) -> Thresholds {
        self.cognitive
    }

    /// The inclusive thresholds for cyclomatic complexity.
    pub const fn cyclomatic(self) -> Thresholds {
        self.cyclomatic
    }

    /// The inclusive thresholds for exclusive logical statements.
    pub const fn logical_lines(self) -> Thresholds {
        self.logical_lines
    }

    /// The maximum nesting depth thresholds.
    pub const fn nesting(self) -> Thresholds {
        self.nesting
    }

    /// The declared parameter count thresholds.
    pub const fn parameters(self) -> Thresholds {
        self.parameters
    }

    /// Rates all five measurements and selects their highest rating without allocating.
    pub const fn assess(self, measurements: Measurements) -> HealthAssessment {
        let signals = [
            SignalAssessment {
                signal: Signal::CognitiveComplexity,
                value: measurements.cognitive_complexity(),
                rating: self.cognitive.level(measurements.cognitive_complexity()),
            },
            SignalAssessment {
                signal: Signal::CyclomaticComplexity,
                value: measurements.cyclomatic_complexity(),
                rating: self.cyclomatic.level(measurements.cyclomatic_complexity()),
            },
            SignalAssessment {
                signal: Signal::LogicalLines,
                value: measurements.logical_lines(),
                rating: self.logical_lines.level(measurements.logical_lines()),
            },
            SignalAssessment {
                signal: Signal::MaxNesting,
                value: measurements.max_nesting(),
                rating: self.nesting.level(measurements.max_nesting()),
            },
            SignalAssessment {
                signal: Signal::ParameterCount,
                value: measurements.parameter_count(),
                rating: self.parameters.level(measurements.parameter_count()),
            },
        ];
        let mut rating = Rating::Healthy;
        let mut index = 0;
        while index < signals.len() {
            if signals[index].rating.rank() > rating.rank() {
                rating = signals[index].rating;
            }
            index += 1;
        }
        HealthAssessment { rating, signals }
    }
}

/// Counts retained by every scope.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct HealthCounts {
    healthy: u32,
    watch: u32,
    high: u32,
}

impl HealthCounts {
    /// Retains separate counts of healthy, Watch, and High units.
    pub const fn new(healthy: u32, watch: u32, high: u32) -> Self {
        Self {
            healthy,
            watch,
            high,
        }
    }

    /// The number of checked units below every Watch threshold.
    pub const fn healthy(self) -> u32 {
        self.healthy
    }

    /// The number of units whose highest rating is Watch.
    pub const fn watch(self) -> u32 {
        self.watch
    }

    /// The number of units whose highest rating is High.
    pub const fn high(self) -> u32 {
        self.high
    }

    /// The total number of checked units across all three ratings.
    pub const fn total(self) -> u32 {
        self.healthy + self.watch + self.high
    }

    /// The total number of Watch and High units.
    pub const fn debt(self) -> u32 {
        self.watch + self.high
    }

    /// Adds one checked unit to the count for its rating.
    pub fn add_rating(&mut self, rating: Rating) {
        match rating {
            Rating::Healthy => self.healthy += 1,
            Rating::Watch => self.watch += 1,
            Rating::High => self.high += 1,
        }
    }

    /// Adds another disjoint group's counts to these totals.
    pub fn add_counts(&mut self, other: Self) {
        self.healthy += other.healthy;
        self.watch += other.watch;
        self.high += other.high;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highest_signal_sets_health_rating_and_preserves_all_signal_values() {
        let policy = HealthPolicy::default();
        let assessment = policy.assess(Measurements::new(15, 2, 120));
        assert_eq!(assessment.rating(), Rating::High);
        assert_eq!(
            assessment.signal(Signal::CognitiveComplexity).rating(),
            Rating::Watch
        );
        assert_eq!(
            assessment.signal(Signal::CyclomaticComplexity).rating(),
            Rating::Healthy
        );
        assert_eq!(
            assessment.signal(Signal::LogicalLines).rating(),
            Rating::High
        );
    }

    #[test]
    fn shape_measurements_are_rated_and_explain_a_rating_on_their_own() {
        let policy = HealthPolicy::default();
        let healthy = Measurements::new(1, 1, 1);
        assert_eq!(healthy.max_nesting(), 0);
        assert_eq!(healthy.parameter_count(), 0);
        assert_eq!(policy.assess(healthy).rating(), Rating::Healthy);
        // A unit whose complexity, cyclomatic, and statement values are healthy
        // is still High when it nests seven levels deep.
        let nested = healthy.with_shape(7, 0);
        assert_eq!(policy.assess(nested).rating(), Rating::High);
        assert_eq!(policy.assess(nested).signal(Signal::MaxNesting).value(), 7);
        let many_parameters = healthy.with_shape(0, 9);
        assert_eq!(policy.assess(many_parameters).rating(), Rating::High);
        assert_eq!(
            policy
                .assess(many_parameters)
                .signal(Signal::ParameterCount)
                .value(),
            9
        );
    }

    #[test]
    fn nesting_and_parameter_thresholds_trigger_on_their_exact_values() {
        let policy = HealthPolicy::default();
        let base = Measurements::new(1, 1, 1);
        let nesting = |value| policy.assess(base.with_shape(value, 0)).rating();
        assert_eq!(nesting(3), Rating::Healthy);
        assert_eq!(nesting(4), Rating::Watch);
        assert_eq!(nesting(6), Rating::Watch);
        assert_eq!(nesting(7), Rating::High);
        let parameters = |value| policy.assess(base.with_shape(0, value)).rating();
        assert_eq!(parameters(5), Rating::Healthy);
        assert_eq!(parameters(6), Rating::Watch);
        assert_eq!(parameters(8), Rating::Watch);
        assert_eq!(parameters(9), Rating::High);
    }

    #[test]
    fn both_promoted_thresholds_are_configurable() {
        let policy = HealthPolicy::new(
            Thresholds::new(15, 25),
            Thresholds::new(11, 21),
            Thresholds::new(50, 100),
            Thresholds::new(2, 3),
            Thresholds::new(2, 3),
        );
        assert_eq!(policy.nesting(), Thresholds::new(2, 3));
        assert_eq!(policy.parameters(), Thresholds::new(2, 3));
        let base = Measurements::new(1, 1, 1);
        assert_eq!(policy.assess(base.with_shape(2, 0)).rating(), Rating::Watch);
        assert_eq!(policy.assess(base.with_shape(0, 3)).rating(), Rating::High);
    }

    #[test]
    fn default_policy_has_documented_limits() {
        let policy = HealthPolicy::default();
        assert_eq!(policy.cognitive(), Thresholds::new(15, 25));
        assert_eq!(policy.cyclomatic(), Thresholds::new(11, 21));
        assert_eq!(policy.logical_lines(), Thresholds::new(50, 100));
        assert_eq!(policy.nesting(), Thresholds::new(4, 7));
        assert_eq!(policy.parameters(), Thresholds::new(6, 9));
    }
}
