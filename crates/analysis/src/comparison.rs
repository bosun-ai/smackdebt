use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::health::{HealthPolicy, Measurements, Rating};
use crate::report::{ComparisonId, FileId};
use crate::source::{SourceRole, SourceSpan, UnitFact, UnitIdentity, UnitMatchKey};

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

/// Whether a retained source comparison may move the diff verdict.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ComparisonParticipation {
    Verdict,
    Context,
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
    anonymous_ambiguity: bool,
    participation: ComparisonParticipation,
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
            anonymous_ambiguity: false,
            participation: ComparisonParticipation::Verdict,
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
    pub const fn is_anonymous_ambiguity(&self) -> bool {
        self.anonymous_ambiguity
    }
    pub const fn with_anonymous_ambiguity(mut self) -> Self {
        self.anonymous_ambiguity = true;
        self
    }
    /// Records whether every source side present may move diff debt.
    ///
    /// A fixture or generated side makes the whole comparison context, even
    /// when the file's role changes on the other side of the diff.
    pub const fn with_source_roles(
        mut self,
        before: Option<SourceRole>,
        after: Option<SourceRole>,
    ) -> Self {
        self.participation = if role_is_verdict_eligible(before) && role_is_verdict_eligible(after)
        {
            ComparisonParticipation::Verdict
        } else {
            ComparisonParticipation::Context
        };
        self
    }
    pub const fn participation(&self) -> ComparisonParticipation {
        self.participation
    }
    pub const fn affects_verdict(&self) -> bool {
        matches!(self.participation, ComparisonParticipation::Verdict)
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

const fn role_is_verdict_eligible(role: Option<SourceRole>) -> bool {
    match role {
        Some(role) => role.affects_verdict(),
        None => true,
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum CandidateKey {
    Declared(UnitIdentity),
    Semantic(UnitMatchKey),
    Fingerprint(UnitMatchKey),
}

struct PendingComparison<'a> {
    identity: UnitIdentity,
    kind: ComparisonKind,
    before: Option<&'a UnitFact>,
    after: Option<&'a UnitFact>,
    span: SourceSpan,
    anonymous_ambiguity: bool,
}

type CandidateGroups = BTreeMap<CandidateKey, Vec<usize>>;

struct MatchState<'a> {
    before: &'a [UnitFact],
    after: &'a [UnitFact],
    used_before: Vec<bool>,
    used_after: Vec<bool>,
    pending: Vec<PendingComparison<'a>>,
    policy: HealthPolicy,
}

impl<'a> MatchState<'a> {
    fn new(before: &'a [UnitFact], after: &'a [UnitFact], policy: HealthPolicy) -> Self {
        Self {
            before,
            after,
            used_before: vec![false; before.len()],
            used_after: vec![false; after.len()],
            pending: Vec::with_capacity(before.len() + after.len()),
            policy,
        }
    }

    fn append_shared(&mut self, before_groups: &CandidateGroups, after_groups: &CandidateGroups) {
        for (key, before_indexes) in before_groups {
            let Some(after_indexes) = after_groups.get(key) else {
                continue;
            };
            mark_used(&mut self.used_before, before_indexes);
            mark_used(&mut self.used_after, after_indexes);
            if before_indexes.len() == 1 && after_indexes.len() == 1 {
                let left = &self.before[before_indexes[0]];
                let right = &self.after[after_indexes[0]];
                self.pending.push(PendingComparison {
                    identity: right.identity().clone(),
                    kind: paired_kind(left, right, self.policy),
                    before: Some(left),
                    after: Some(right),
                    span: right.span(),
                    anonymous_ambiguity: false,
                });
                continue;
            }
            let representative = &self.after[after_indexes[0]];
            self.pending.push(PendingComparison {
                identity: representative.identity().clone(),
                kind: ComparisonKind::Ambiguous,
                before: None,
                after: None,
                span: representative.span(),
                anonymous_ambiguity: !matches!(key, CandidateKey::Declared(_)),
            });
        }
    }
}

fn candidate_key(unit: &UnitFact) -> Option<CandidateKey> {
    match unit.match_evidence().key() {
        UnitMatchKey::Declared => Some(CandidateKey::Declared(unit.identity().clone())),
        value @ UnitMatchKey::Semantic { .. } => Some(CandidateKey::Semantic(value.clone())),
        value @ UnitMatchKey::Fingerprint { .. } => Some(CandidateKey::Fingerprint(value.clone())),
        UnitMatchKey::None => None,
    }
}

fn paired_kind(before: &UnitFact, after: &UnitFact, policy: HealthPolicy) -> ComparisonKind {
    let before_assessment = policy.assess(before.measurements());
    let after_assessment = policy.assess(after.measurements());
    match before_assessment.rating().cmp(&after_assessment.rating()) {
        Ordering::Less => ComparisonKind::Regressed,
        Ordering::Greater => ComparisonKind::Improved,
        Ordering::Equal if before.measurements().rated() != after.measurements().rated() => {
            ComparisonKind::MetricChanged
        }
        Ordering::Equal => ComparisonKind::Unchanged,
    }
}

fn group_units(units: &[UnitFact]) -> CandidateGroups {
    let mut groups = CandidateGroups::new();
    for (index, unit) in units.iter().enumerate() {
        if let Some(key) = candidate_key(unit) {
            groups.entry(key).or_default().push(index);
        }
    }
    groups
}

fn mark_used(used: &mut [bool], indexes: &[usize]) {
    for index in indexes {
        used[*index] = true;
    }
}

fn append_one_sided<'a>(
    units: &'a [UnitFact],
    used: &[bool],
    kind: ComparisonKind,
    pending: &mut Vec<PendingComparison<'a>>,
) {
    for unit in units
        .iter()
        .enumerate()
        .filter(|(index, _)| !used[*index])
        .map(|(_, unit)| unit)
    {
        let (before, after) = match kind {
            ComparisonKind::Removed => (Some(unit), None),
            ComparisonKind::Added => (None, Some(unit)),
            _ => unreachable!("one-sided comparison kind"),
        };
        pending.push(PendingComparison {
            identity: unit.identity().clone(),
            kind,
            before,
            after,
            span: unit.span(),
            anonymous_ambiguity: false,
        });
    }
}

fn finish_comparisons(
    mut pending: Vec<PendingComparison<'_>>,
    policy: HealthPolicy,
) -> Vec<Comparison> {
    pending.sort_unstable_by(|left, right| {
        left.identity
            .cmp(&right.identity)
            .then_with(|| left.span.cmp(&right.span))
            .then_with(|| left.kind.cmp(&right.kind))
    });
    pending
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            let before = value.before.map(UnitFact::measurements);
            let after = value.after.map(UnitFact::measurements);
            let mut comparison = Comparison::new(
                ComparisonId::from_index(index),
                value.identity,
                value.kind,
                before,
                after,
                before.map(|value| policy.assess(value).rating()),
                after.map(|value| policy.assess(value).rating()),
            )
            .with_span(value.span);
            if value.anonymous_ambiguity {
                comparison = comparison.with_anonymous_ambiguity();
            }
            comparison
        })
        .collect()
}

/// Compares units through the strongest safe evidence each unit owns.
pub fn compare_units(
    before: &[UnitFact],
    after: &[UnitFact],
    policy: HealthPolicy,
) -> Vec<Comparison> {
    let before_groups = group_units(before);
    let after_groups = group_units(after);
    let mut state = MatchState::new(before, after, policy);
    state.append_shared(&before_groups, &after_groups);
    append_one_sided(
        before,
        &state.used_before,
        ComparisonKind::Removed,
        &mut state.pending,
    );
    append_one_sided(
        after,
        &state.used_after,
        ComparisonKind::Added,
        &mut state.pending,
    );
    finish_comparisons(state.pending, policy)
}
