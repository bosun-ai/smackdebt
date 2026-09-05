/// The measurements retained for one unit.
///
/// All five measurements are rated: cognitive complexity, cyclomatic
/// complexity, exclusive logical lines, maximum nesting depth, and parameter
/// count. A unit's rating is therefore explainable entirely from the five
/// values the machine report serializes.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Measurements {
    cognitive_complexity: u32,
    cyclomatic_complexity: u32,
    logical_lines: u32,
    max_nesting: u32,
    parameter_count: u32,
}

impl Measurements {
    pub const fn new(
        cognitive_complexity: u32,
        cyclomatic_complexity: u32,
        logical_lines: u32,
    ) -> Self {
        Self {
            cognitive_complexity,
            cyclomatic_complexity,
            logical_lines,
            max_nesting: 0,
            parameter_count: 0,
        }
    }

    /// Adds the collected shape measurements to the rated measurements.
    pub const fn with_shape(mut self, max_nesting: u32, parameter_count: u32) -> Self {
        self.max_nesting = max_nesting;
        self.parameter_count = parameter_count;
        self
    }

    /// The deepest nesting level reached inside this unit.
    pub const fn max_nesting(self) -> u32 {
        self.max_nesting
    }

    /// The number of parameters this unit declares.
    pub const fn parameter_count(self) -> u32 {
        self.parameter_count
    }

    /// The five rated measurements, in policy order.
    pub const fn rated(self) -> (u32, u32, u32, u32, u32) {
        (
            self.cognitive_complexity,
            self.cyclomatic_complexity,
            self.logical_lines,
            self.max_nesting,
            self.parameter_count,
        )
    }

    pub const fn cognitive_complexity(self) -> u32 {
        self.cognitive_complexity
    }

    pub const fn cyclomatic_complexity(self) -> u32 {
        self.cyclomatic_complexity
    }

    pub const fn logical_lines(self) -> u32 {
        self.logical_lines
    }
}

/// A threshold pair for one signal.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Thresholds {
    watch: u32,
    high: u32,
}

impl Thresholds {
    pub const fn new(watch: u32, high: u32) -> Self {
        Self { watch, high }
    }

    pub const fn watch(self) -> u32 {
        self.watch
    }

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
    CognitiveComplexity,
    CyclomaticComplexity,
    LogicalLines,
    MaxNesting,
    ParameterCount,
}

/// The severity of one signal or a unit as a whole.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Rating {
    Healthy,
    Watch,
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
    pub const fn signal(self) -> Signal {
        self.signal
    }

    pub const fn value(self) -> u32 {
        self.value
    }

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
    pub const fn rating(self) -> Rating {
        self.rating
    }

    pub const fn signals(self) -> [SignalAssessment; 5] {
        self.signals
    }

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

    pub const fn cognitive(self) -> Thresholds {
        self.cognitive
    }

    pub const fn cyclomatic(self) -> Thresholds {
        self.cyclomatic
    }

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

    pub const fn assess(self, measurements: Measurements) -> HealthAssessment {
        let signals = [
            SignalAssessment {
                signal: Signal::CognitiveComplexity,
                value: measurements.cognitive_complexity,
                rating: self.cognitive.level(measurements.cognitive_complexity),
            },
            SignalAssessment {
                signal: Signal::CyclomaticComplexity,
                value: measurements.cyclomatic_complexity,
                rating: self.cyclomatic.level(measurements.cyclomatic_complexity),
            },
            SignalAssessment {
                signal: Signal::LogicalLines,
                value: measurements.logical_lines,
                rating: self.logical_lines.level(measurements.logical_lines),
            },
            SignalAssessment {
                signal: Signal::MaxNesting,
                value: measurements.max_nesting,
                rating: self.nesting.level(measurements.max_nesting),
            },
            SignalAssessment {
                signal: Signal::ParameterCount,
                value: measurements.parameter_count,
                rating: self.parameters.level(measurements.parameter_count),
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
    pub const fn new(healthy: u32, watch: u32, high: u32) -> Self {
        Self {
            healthy,
            watch,
            high,
        }
    }

    pub const fn healthy(self) -> u32 {
        self.healthy
    }

    pub const fn watch(self) -> u32 {
        self.watch
    }

    pub const fn high(self) -> u32 {
        self.high
    }

    pub const fn total(self) -> u32 {
        self.healthy + self.watch + self.high
    }

    pub const fn debt(self) -> u32 {
        self.watch + self.high
    }

    pub fn add_rating(&mut self, rating: Rating) {
        match rating {
            Rating::Healthy => self.healthy += 1,
            Rating::Watch => self.watch += 1,
            Rating::High => self.high += 1,
        }
    }

    pub fn add_counts(&mut self, other: Self) {
        self.healthy += other.healthy;
        self.watch += other.watch;
        self.high += other.high;
    }
}
