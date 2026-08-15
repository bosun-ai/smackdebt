use std::cmp::Ordering;

use crate::health::{HealthPolicy, Measurements, Rating};
use crate::report::{ComparisonId, FileId};
use crate::source::{SourceSpan, UnitFact, UnitIdentity};

/// Whether a diff unit was added, removed, or changed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ComparisonKind {
    Added,
    Removed,
    Improved,
    Regressed,
    MetricChanged,
    Ambiguous,
    Unchanged,
}

/// The user-facing direction of a retained diff comparison.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ComparisonDirection {
    Worse,
    Better,
    Changed,
}

/// A named unit comparison between two source versions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Comparison {
    id: ComparisonId,
    identity: UnitIdentity,
    kind: ComparisonKind,
    before: Option<Measurements>,
    after: Option<Measurements>,
    before_rating: Option<Rating>,
    after_rating: Option<Rating>,
    file: Option<FileId>,
    span: Option<SourceSpan>,
}

impl Comparison {
    pub fn new(
        id: ComparisonId,
        identity: UnitIdentity,
        kind: ComparisonKind,
        before: Option<Measurements>,
        after: Option<Measurements>,
        before_rating: Option<Rating>,
        after_rating: Option<Rating>,
    ) -> Self {
        Self {
            id,
            identity,
            kind,
            before,
            after,
            before_rating,
            after_rating,
            file: None,
            span: None,
        }
    }

    pub const fn id(&self) -> ComparisonId {
        self.id
    }
    pub fn identity(&self) -> &UnitIdentity {
        &self.identity
    }
    pub const fn kind(&self) -> ComparisonKind {
        self.kind
    }
    pub const fn before(&self) -> Option<Measurements> {
        self.before
    }
    pub const fn after(&self) -> Option<Measurements> {
        self.after
    }
    pub const fn before_rating(&self) -> Option<Rating> {
        self.before_rating
    }
    pub const fn after_rating(&self) -> Option<Rating> {
        self.after_rating
    }
    pub const fn file(&self) -> Option<FileId> {
        self.file
    }
    pub fn with_file(mut self, file: FileId) -> Self {
        self.file = Some(file);
        self
    }
    /// The unit's location on the side that still has one.
    ///
    /// The span locates a comparison for a reader; it is presentation context
    /// and stays out of the machine report, which identifies a unit by name.
    pub const fn span(&self) -> Option<SourceSpan> {
        self.span
    }
    pub const fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }
    pub const fn direction(&self) -> ComparisonDirection {
        match self.kind {
            ComparisonKind::Regressed => ComparisonDirection::Worse,
            ComparisonKind::Improved => ComparisonDirection::Better,
            ComparisonKind::Added => match self.after_rating {
                Some(Rating::Watch | Rating::High) => ComparisonDirection::Worse,
                _ => ComparisonDirection::Changed,
            },
            ComparisonKind::Removed => match self.before_rating {
                Some(Rating::Watch | Rating::High) => ComparisonDirection::Better,
                _ => ComparisonDirection::Changed,
            },
            ComparisonKind::MetricChanged
            | ComparisonKind::Ambiguous
            | ComparisonKind::Unchanged => ComparisonDirection::Changed,
        }
    }
}

/// Compares units after both sides have been reduced to sorted identities.
pub fn compare_units(
    before: &[UnitFact],
    after: &[UnitFact],
    policy: HealthPolicy,
) -> Vec<Comparison> {
    let mut left: Vec<&UnitFact> = before.iter().collect();
    let mut right: Vec<&UnitFact> = after.iter().collect();
    left.sort_unstable_by(|a, b| a.identity().cmp(b.identity()));
    right.sort_unstable_by(|a, b| a.identity().cmp(b.identity()));

    let mut comparisons = Vec::with_capacity(left.len() + right.len());
    let mut left_index = 0;
    let mut right_index = 0;
    let mut comparison_id = 0;
    while left_index < left.len() || right_index < right.len() {
        let left_identity = left.get(left_index).map(|unit| unit.identity());
        let right_identity = right.get(right_index).map(|unit| unit.identity());
        let identity_order = match (left_identity, right_identity) {
            (Some(left_identity), Some(right_identity)) => left_identity.cmp(right_identity),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        };

        if identity_order == Ordering::Equal {
            let left_start = left_index;
            while left_index < left.len()
                && left[left_index].identity() == left[left_start].identity()
            {
                left_index += 1;
            }
            let right_start = right_index;
            while right_index < right.len()
                && right[right_index].identity() == right[right_start].identity()
            {
                right_index += 1;
            }
            let identity = left[left_start].identity().clone();
            if left_index - left_start != 1 || right_index - right_start != 1 {
                comparisons.push(
                    Comparison::new(
                        ComparisonId::from_index(comparison_id),
                        identity,
                        ComparisonKind::Ambiguous,
                        None,
                        None,
                        None,
                        None,
                    )
                    .with_span(right[right_start].span()),
                );
            } else {
                let left_unit = left[left_start];
                let right_unit = right[right_start];
                let left_assessment = policy.assess(left_unit.measurements());
                let right_assessment = policy.assess(right_unit.measurements());
                let kind = match left_assessment.rating().cmp(&right_assessment.rating()) {
                    Ordering::Less => ComparisonKind::Regressed,
                    Ordering::Greater => ComparisonKind::Improved,
                    // Only the rated measurements classify a change, so an
                    // unrated collected value never invents a diff outcome.
                    Ordering::Equal
                        if left_unit.measurements().rated()
                            != right_unit.measurements().rated() =>
                    {
                        ComparisonKind::MetricChanged
                    }
                    Ordering::Equal => ComparisonKind::Unchanged,
                };
                comparisons.push(
                    Comparison::new(
                        ComparisonId::from_index(comparison_id),
                        identity,
                        kind,
                        Some(left_unit.measurements()),
                        Some(right_unit.measurements()),
                        Some(left_assessment.rating()),
                        Some(right_assessment.rating()),
                    )
                    .with_span(right_unit.span()),
                );
            }
        } else if identity_order == Ordering::Less {
            let start = left_index;
            while left_index < left.len() && left[left_index].identity() == left[start].identity() {
                left_index += 1;
            }
            let identity = left[start].identity().clone();
            if left_index - start != 1 {
                comparisons.push(
                    Comparison::new(
                        ComparisonId::from_index(comparison_id),
                        identity,
                        ComparisonKind::Ambiguous,
                        None,
                        None,
                        None,
                        None,
                    )
                    .with_span(left[start].span()),
                );
            } else {
                let unit = left[start];
                let assessment = policy.assess(unit.measurements());
                comparisons.push(
                    Comparison::new(
                        ComparisonId::from_index(comparison_id),
                        identity,
                        ComparisonKind::Removed,
                        Some(unit.measurements()),
                        None,
                        Some(assessment.rating()),
                        None,
                    )
                    .with_span(unit.span()),
                );
            }
        } else {
            let start = right_index;
            while right_index < right.len()
                && right[right_index].identity() == right[start].identity()
            {
                right_index += 1;
            }
            let identity = right[start].identity().clone();
            if right_index - start != 1 {
                comparisons.push(
                    Comparison::new(
                        ComparisonId::from_index(comparison_id),
                        identity,
                        ComparisonKind::Ambiguous,
                        None,
                        None,
                        None,
                        None,
                    )
                    .with_span(right[start].span()),
                );
            } else {
                let unit = right[start];
                let assessment = policy.assess(unit.measurements());
                comparisons.push(
                    Comparison::new(
                        ComparisonId::from_index(comparison_id),
                        identity,
                        ComparisonKind::Added,
                        None,
                        Some(unit.measurements()),
                        None,
                        Some(assessment.rating()),
                    )
                    .with_span(unit.span()),
                );
            }
        }
        comparison_id += 1;
    }
    comparisons
}
