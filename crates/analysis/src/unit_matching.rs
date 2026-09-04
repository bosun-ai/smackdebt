use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::comparison::{Comparison, ComparisonKind};
use crate::health::HealthPolicy;
use crate::report::ComparisonId;
use crate::source::{SourceSpan, UnitFact, UnitFingerprint, UnitIdentity, UnitMatchKey};

struct PendingComparison<'a> {
    identity: UnitIdentity,
    kind: ComparisonKind,
    before: Option<&'a UnitFact>,
    after: Option<&'a UnitFact>,
    span: SourceSpan,
    anonymous_ambiguity: bool,
    unpaired_anonymous: bool,
}

/// One key several units on at least one side answer to.
///
/// A pass that finds a contest cannot say which unit is which, so it either
/// hands the units to the next pass or states the ambiguity and stops.
struct Contest {
    before: Vec<usize>,
    after: Vec<usize>,
}

struct MatchState<'a> {
    before: &'a [UnitFact],
    after: &'a [UnitFact],
    used_before: Vec<bool>,
    used_after: Vec<bool>,
    pending: Vec<PendingComparison<'a>>,
    unclear_anonymous: bool,
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
            unclear_anonymous: false,
            policy,
        }
    }

    /// Pairs every still-unmatched unit whose key names exactly one unit on
    /// each side, and hands back the keys several units answer to.
    fn pair_unique<K: Ord>(&mut self, key_of: impl Fn(&UnitFact) -> Option<K>) -> Vec<Contest> {
        let before_groups = group_unused(self.before, &self.used_before, &key_of);
        let after_groups = group_unused(self.after, &self.used_after, &key_of);
        let mut contests = Vec::new();
        for (key, before_indexes) in before_groups {
            let Some(after_indexes) = after_groups.get(&key) else {
                continue;
            };
            if let ([left], [right]) = (&before_indexes[..], &after_indexes[..]) {
                self.used_before[*left] = true;
                self.used_after[*right] = true;
                let left = &self.before[*left];
                let right = &self.after[*right];
                self.pending.push(PendingComparison {
                    identity: right.identity().clone(),
                    kind: paired_kind(left, right, self.policy),
                    before: Some(left),
                    after: Some(right),
                    span: right.span(),
                    anonymous_ambiguity: false,
                    unpaired_anonymous: false,
                });
                continue;
            }
            contests.push(Contest {
                before: before_indexes,
                after: after_indexes.clone(),
            });
        }
        contests
    }

    /// States one contest as the single ambiguity it is, and consumes it.
    fn state_ambiguity(&mut self, contest: &Contest, anonymous: bool) {
        self.unclear_anonymous |= anonymous;
        mark_used(&mut self.used_before, &contest.before);
        mark_used(&mut self.used_after, &contest.after);
        let representative = &self.after[contest.after[0]];
        self.pending.push(PendingComparison {
            identity: representative.identity().clone(),
            kind: ComparisonKind::Ambiguous,
            before: None,
            after: None,
            span: representative.span(),
            anonymous_ambiguity: anonymous,
            unpaired_anonymous: false,
        });
    }

    /// Whether every unit in a contest still holds syntax a later pass can
    /// tell it apart by. Without that, this pass is the last word.
    fn contest_can_wait(&self, contest: &Contest) -> bool {
        let before = contest.before.iter().map(|index| &self.before[*index]);
        let after = contest.after.iter().map(|index| &self.after[*index]);
        before
            .chain(after)
            .all(|unit| unit.match_evidence().fingerprint().is_some())
    }

    /// Names the units no pass could pair, one comparison each.
    ///
    /// A file that ends with an anonymous unit unpaired on *both* sides cannot
    /// say which of them is the other's edit. Those comparisons carry the
    /// file's ambiguity so the report states it, and are withheld from the
    /// verdict so one unfollowed edit cannot read as debt added on one side
    /// and debt removed on the other. A stated anonymous ambiguity already
    /// left units unpaired on both sides, so it puts the file in the same
    /// position as a leftover on each side would.
    fn append_one_sided(&mut self) {
        let (before, after) = (self.before, self.after);
        let removed = unused(before, &self.used_before);
        let added = unused(after, &self.used_after);
        let unpaired = self.unclear_anonymous
            || (removed.iter().any(|unit| is_anonymous(unit))
                && added.iter().any(|unit| is_anonymous(unit)));
        let one_sided = removed
            .into_iter()
            .map(|unit| (unit, ComparisonKind::Removed))
            .chain(added.into_iter().map(|unit| (unit, ComparisonKind::Added)));
        for (unit, kind) in one_sided {
            let anonymous = unpaired && is_anonymous(unit);
            let (before, after) = match kind {
                ComparisonKind::Removed => (Some(unit), None),
                _ => (None, Some(unit)),
            };
            self.pending.push(PendingComparison {
                identity: unit.identity().clone(),
                kind,
                before,
                after,
                span: unit.span(),
                anonymous_ambiguity: anonymous,
                unpaired_anonymous: anonymous,
            });
        }
    }
}

/// A unit that declares no name of its own, which is the only kind of unit the
/// matcher can lose track of without saying so.
fn is_anonymous(unit: &UnitFact) -> bool {
    !matches!(unit.match_evidence().key(), UnitMatchKey::Declared)
}

fn declared_key(unit: &UnitFact) -> Option<UnitIdentity> {
    matches!(unit.match_evidence().key(), UnitMatchKey::Declared).then(|| unit.identity().clone())
}

fn semantic_key(unit: &UnitFact) -> Option<UnitMatchKey> {
    let key = unit.match_evidence().key();
    matches!(key, UnitMatchKey::Semantic { .. }).then(|| key.clone())
}

fn fingerprint_key(unit: &UnitFact) -> Option<UnitFingerprint> {
    unit.match_evidence().fingerprint()
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

fn group_unused<K: Ord>(
    units: &[UnitFact],
    used: &[bool],
    key_of: impl Fn(&UnitFact) -> Option<K>,
) -> BTreeMap<K, Vec<usize>> {
    let mut groups = BTreeMap::new();
    for (index, unit) in units.iter().enumerate().filter(|(index, _)| !used[*index]) {
        if let Some(key) = key_of(unit) {
            groups.entry(key).or_insert_with(Vec::new).push(index);
        }
    }
    groups
}

fn unused<'a>(units: &'a [UnitFact], used: &[bool]) -> Vec<&'a UnitFact> {
    units
        .iter()
        .enumerate()
        .filter(|(index, _)| !used[*index])
        .map(|(_, unit)| unit)
        .collect()
}

fn mark_used(used: &mut [bool], indexes: &[usize]) {
    for index in indexes {
        used[*index] = true;
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
            if value.unpaired_anonymous {
                comparison = comparison.with_unpaired_anonymous();
            }
            comparison
        })
        .collect()
}

/// Compares units through the strongest safe evidence each unit owns.
///
/// Evidence is tried strongest first. A declared name is decisive: units that
/// share one and cannot be told apart are one stated ambiguity. An anchor is
/// next, and a contested anchor is not an answer — sibling blocks under the
/// same call answer to the same anchor — so those units wait for their exact
/// syntax to separate them, and only settle as an ambiguity here when they
/// carry no syntax to wait for. Exact syntax is last and therefore final.
///
/// Whatever no pass paired is one-sided, and `append_one_sided` decides what
/// the report may claim about it.
pub fn compare_units(
    before: &[UnitFact],
    after: &[UnitFact],
    policy: HealthPolicy,
) -> Vec<Comparison> {
    let mut state = MatchState::new(before, after, policy);
    for contest in state.pair_unique(declared_key) {
        state.state_ambiguity(&contest, false);
    }
    for contest in state.pair_unique(semantic_key) {
        if !state.contest_can_wait(&contest) {
            state.state_ambiguity(&contest, true);
        }
    }
    for contest in state.pair_unique(fingerprint_key) {
        state.state_ambiguity(&contest, true);
    }
    state.append_one_sided();
    finish_comparisons(state.pending, policy)
}
