//! Source measurements for one function, method, or other code unit.
//!
//! The language adapter supplies cognitive complexity, cyclomatic complexity,
//! exclusive logical statements, maximum nesting depth, and parameter count.
//! This value retains all five together; [`crate::HealthPolicy`] owns their rating.
//! Extraction belongs to the language adapter, including the exclusion of nested
//! units from a parent's additive statement count.
//!
//! ```
//! use smackdebt_analysis::{HealthPolicy, Measurements, Rating};
//! let unit = Measurements::new(1, 1, 12).with_shape(7, 2);
//! assert_eq!(unit.rated(), (1, 1, 12, 7, 2));
//! assert_eq!(HealthPolicy::default().assess(unit).rating(), Rating::High);
//! ```

#![deny(missing_docs)]

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
    /// Retains the three base measurements, with nesting and parameters initially zero.
    ///
    /// Use [`Self::with_shape`] to supply the two shape measurements.
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

    /// The control-flow comprehension cost collected by the language analyzer.
    pub const fn cognitive_complexity(self) -> u32 {
        self.cognitive_complexity
    }

    /// The independent control-flow path count collected by the language analyzer.
    pub const fn cyclomatic_complexity(self) -> u32 {
        self.cyclomatic_complexity
    }

    /// The exclusive logical statement count, excluding separately measured child units.
    pub const fn logical_lines(self) -> u32 {
        self.logical_lines
    }
}
