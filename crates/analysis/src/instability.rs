//! Dependency instability, the ratio of outgoing to total coupling.
//!
//! `I = fan_out / (fan_in + fan_out)`. Zero means no outgoing dependencies;
//! one means no incoming dependencies. An isolated node has no ratio. The value
//! stores exact integer operands; stable-dependency policy compares those operands
//! without rounding. Instability alone is descriptive, not a health rating.
//!
//! ```
//! use smackdebt_analysis::instability;
//! let value = instability(3, 1).unwrap();
//! assert_eq!((value.numerator(), value.denominator()), (1, 4));
//! assert_eq!(instability(0, 0), None);
//! ```
//!
//! Terminology: [Martin, dependency principles](https://objectmentor.com/resources/articles/Principles_and_Patterns.pdf).

#![deny(missing_docs)]

/// Computes exact outgoing/total coupling, or `None` for an isolated node.
///
/// The caller supplies degree counts whose sum fits in `u32`.
pub const fn instability(incoming: u32, outgoing: u32) -> Option<Instability> {
    Instability::new(outgoing, incoming + outgoing)
}

/// Exact instability operands; absence represents a zero denominator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Instability {
    numerator: u32,
    denominator: u32,
}

impl Instability {
    /// Retains the supplied fraction, returning `None` for a zero denominator.
    ///
    /// The caller supplies valid instability operands; this constructor does not
    /// clamp a numerator larger than the denominator.
    pub const fn new(numerator: u32, denominator: u32) -> Option<Self> {
        if denominator == 0 {
            None
        } else {
            Some(Self {
                numerator,
                denominator,
            })
        }
    }
    /// The outgoing coupling count supplied for this fraction.
    pub const fn numerator(self) -> u32 {
        self.numerator
    }
    /// The nonzero total coupling count supplied for this fraction.
    pub const fn denominator(self) -> u32 {
        self.denominator
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn represents_exact_fraction() {
        let value = instability(3, 1).unwrap();
        assert_eq!((value.numerator(), value.denominator()), (1, 4));
        assert_eq!(instability(0, 0), None);
    }
}
