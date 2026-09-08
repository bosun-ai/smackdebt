//! Identity-based before/after matching of units within a file.

use crate::comparison::{Comparison, ComparisonKind};
use crate::health::{HealthPolicy, Rating};
use crate::report::{ComparisonId, FileId};
use crate::source::{SourceSpan, UnitFact, UnitFingerprint, UnitIdentity, UnitKind, UnitMatchKey};
use std::cmp::Ordering;
use std::collections::BTreeMap;

pub(crate) struct PendingComparison<'a> {
    identity: UnitIdentity,
    kind: ComparisonKind,
    before: Option<&'a UnitFact>,
    after: Option<&'a UnitFact>,
    span: SourceSpan,
    anonymous_ambiguity: bool,
    unpaired_anonymous: bool,
    /// The file position and span this unit answered from before a change
    /// moved it here, when one did.
    origin: Option<(usize, SourceSpan)>,
}

/// One key several units on at least one side answer to.
///
/// A pass that finds a contest cannot say which unit is which, so it either
/// hands the units to the next pass or states the ambiguity and stops.
struct Contest {
    before: Vec<usize>,
    after: Vec<usize>,
}

/// A class of unpaired unit whose members are interchangeable to the verdict.
///
/// Two unpaired units of the same kind and rating carry exactly the same
/// verdict weight, so which of them the matcher lost track of changes nothing
/// a report would say. That is what makes the pigeonhole count below exact
/// rather than a guess.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Bucket {
    kind: UnitKind,
    rating: Rating,
}

pub(crate) struct MatchState<'a> {
    before: &'a [UnitFact],
    after: &'a [UnitFact],
    used_before: Vec<bool>,
    used_after: Vec<bool>,
    pending: Vec<PendingComparison<'a>>,
    /// Anonymous units a stated ambiguity swallowed, per side. They were not
    /// paired either, so the pigeonhole count has to see them.
    unclear_before: Vec<Bucket>,
    unclear_after: Vec<Bucket>,
    policy: HealthPolicy,
}

impl<'a> MatchState<'a> {
    pub(crate) fn new(before: &'a [UnitFact], after: &'a [UnitFact], policy: HealthPolicy) -> Self {
        Self {
            before,
            after,
            used_before: vec![false; before.len()],
            used_after: vec![false; after.len()],
            pending: Vec::with_capacity(before.len() + after.len()),
            unclear_before: Vec::new(),
            unclear_after: Vec::new(),
            policy,
        }
    }

    /// The units no pass paired yet, with their positions, per side.
    pub(crate) fn unused_before(&self) -> Vec<(usize, &'a UnitFact)> {
        unused_indexed(self.before, &self.used_before)
    }

    pub(crate) fn unused_after(&self) -> Vec<(usize, &'a UnitFact)> {
        unused_indexed(self.after, &self.used_after)
    }

    /// Takes one unused unit for a pairing the caller proved.
    pub(crate) fn claim_before(&mut self, index: usize) -> &'a UnitFact {
        self.used_before[index] = true;
        &self.before[index]
    }

    pub(crate) fn claim_after(&mut self, index: usize) -> &'a UnitFact {
        self.used_after[index] = true;
        &self.after[index]
    }

    /// Records a unit this file received from another, which reads as the one
    /// movement it is.
    pub(crate) fn push_moved(
        &mut self,
        before: &'a UnitFact,
        after: &'a UnitFact,
        origin: (usize, SourceSpan),
    ) {
        self.pending.push(PendingComparison {
            identity: after.identity().clone(),
            kind: paired_kind(before, after, self.policy),
            before: Some(before),
            after: Some(after),
            span: after.span(),
            anonymous_ambiguity: false,
            unpaired_anonymous: false,
            origin: Some(origin),
        });
    }

    pub(crate) fn take_pending(&mut self) -> Vec<PendingComparison<'a>> {
        std::mem::take(&mut self.pending)
    }

    fn bucket(&self, unit: &UnitFact) -> Bucket {
        Bucket {
            kind: unit.identity().kind(),
            rating: self.policy.assess(unit.measurements()).rating(),
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
                    origin: None,
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
        if anonymous {
            for index in &contest.before {
                let bucket = self.bucket(&self.before[*index]);
                self.unclear_before.push(bucket);
            }
            for index in &contest.after {
                let bucket = self.bucket(&self.after[*index]);
                self.unclear_after.push(bucket);
            }
        }
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
            origin: None,
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
    /// say which of them is the other's edit, so every unpaired anonymous unit
    /// there carries the file's ambiguity and the report states it.
    ///
    /// How much of that the verdict must then give up is a counting question,
    /// not a judgement. Sort the unpaired anonymous units of each side into
    /// buckets of one kind and one rating, whose members are interchangeable to
    /// a verdict. Within a bucket holding `k` removals and `m` additions,
    /// `min(k, m)` of each could be the same units rewritten, and the report
    /// cannot tell which — so it withholds that many from each side. The
    /// excess cannot be anything but genuinely new or genuinely gone, by the
    /// pigeonhole principle, and every member of a bucket weighs the same, so
    /// counting the excess is exact whichever members are left holding it.
    ///
    /// The buckets count the units a stated ambiguity swallowed too. Those were
    /// never paired either, and a count that cannot see them would call a
    /// leftover addition net-new while its removal sat inside an ambiguity.
    pub(crate) fn append_one_sided(&mut self) {
        let (before, after) = (self.before, self.after);
        let removed = unused(before, &self.used_before);
        let added = unused(after, &self.used_after);
        let flagged = removed.iter().any(|unit| is_anonymous(unit))
            && added.iter().any(|unit| is_anonymous(unit));
        let mut budget = self.interchangeable(&removed, &added);
        let one_sided = removed
            .into_iter()
            .map(|unit| (unit, ComparisonKind::Removed))
            .chain(added.into_iter().map(|unit| (unit, ComparisonKind::Added)));
        for (unit, kind) in one_sided {
            let anonymous = is_anonymous(unit);
            let withheld = anonymous && budget.spend(kind, self.bucket(unit));
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
                anonymous_ambiguity: flagged && anonymous,
                unpaired_anonymous: withheld,
                origin: None,
            });
        }
    }

    /// How many unpaired units per bucket each side must give up, which is the
    /// smaller of what the two sides hold there.
    fn interchangeable(&self, removed: &[&UnitFact], added: &[&UnitFact]) -> Budget {
        let tally = |units: &[&UnitFact], swallowed: &[Bucket]| {
            let mut counts: BTreeMap<Bucket, usize> = BTreeMap::new();
            for bucket in units
                .iter()
                .filter(|unit| is_anonymous(unit))
                .map(|unit| self.bucket(unit))
                .chain(swallowed.iter().copied())
            {
                *counts.entry(bucket).or_default() += 1;
            }
            counts
        };
        let removals = tally(removed, &self.unclear_before);
        let additions = tally(added, &self.unclear_after);
        let mut budget = BTreeMap::new();
        for (bucket, count) in &removals {
            let Some(paired) = additions.get(bucket).map(|other| *count.min(other)) else {
                continue;
            };
            // Each side owes the same count and spends it from its own ledger,
            // so neither can exhaust what the other has to give up.
            budget.insert((ComparisonKind::Removed, *bucket), paired);
            budget.insert((ComparisonKind::Added, *bucket), paired);
        }
        Budget(budget)
    }
}

/// The per-bucket withholding each side still owes, spent as the one-sided
/// comparisons are written out.
struct Budget(BTreeMap<(ComparisonKind, Bucket), usize>);

impl Budget {
    fn spend(&mut self, kind: ComparisonKind, bucket: Bucket) -> bool {
        let Some(left) = self.0.get_mut(&(kind, bucket)) else {
            return false;
        };
        if *left == 0 {
            return false;
        }
        *left -= 1;
        true
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

fn unused_indexed<'a>(units: &'a [UnitFact], used: &[bool]) -> Vec<(usize, &'a UnitFact)> {
    units
        .iter()
        .enumerate()
        .filter(|(index, _)| !used[*index])
        .collect()
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

pub(crate) fn finish_comparisons(
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
            if let Some((file, span)) = value.origin {
                comparison = comparison.with_origin(FileId::from_index(file), span);
            }
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
    pair_within_file(&mut state);
    state.append_one_sided();
    finish_comparisons(state.pending, policy)
}

/// Runs one file's three passes, strongest evidence first.
pub(crate) fn pair_within_file(state: &mut MatchState<'_>) {
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
}
