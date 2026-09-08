//! Following units across the files of one change.
//!
//! A unit that moved between two files is one unit, and a report that calls
//! it removed over here and added over there states two movements the change
//! never made: the debt did not go anywhere. Each file is matched first by
//! the evidence its own two sides carry, and only what no pass paired there
//! is offered to the rest of the change.
//!
//! The cross-file passes keep the discipline the within-file ones set. A key
//! that names exactly one unused unit on each side is a pair; a key several
//! units answer to is left alone, because a change that moved two units of
//! one name cannot say which became which, and a guess here would be a claim
//! about debt. Nothing states a new ambiguity: the units fall back to their
//! own files and are reported exactly as they were before.
//!
//! Anonymous units join the fingerprint pass alone. An anchor describes where
//! a closure sits inside its file — the call it hangs off, the item that
//! declares it — which says nothing once the unit lives somewhere else, while
//! identical syntax at a new address is exactly what a move looks like.

use std::collections::BTreeMap;

use crate::comparison::Comparison;
use crate::health::HealthPolicy;
use crate::source::{UnitFact, UnitFingerprint, UnitIdentity, UnitMatchKey};
use crate::unit_matching::{MatchState, finish_comparisons, pair_within_file};

/// One changed file's units, on the sides that hold them.
///
/// A file the change added has no before side; one it deleted has no after
/// side. Both are the same shape to the matcher: an empty slice.
#[derive(Clone, Copy)]
pub struct UnitDiffSides<'a> {
    before: &'a [UnitFact],
    after: &'a [UnitFact],
}

impl<'a> UnitDiffSides<'a> {
    pub const fn new(before: &'a [UnitFact], after: &'a [UnitFact]) -> Self {
        Self { before, after }
    }
}

/// Compares every changed file's units, then follows what none of them paired
/// across the whole change.
///
/// The answer is one comparison list per file, in the order the files were
/// given. A unit that moved is stated once, by the file it landed in, and its
/// origin names the file it left as a position in that same order — so
/// callers pass their files in report file order and read the origin as a
/// file identity.
pub fn compare_unit_sets(
    files: &[UnitDiffSides<'_>],
    policy: HealthPolicy,
) -> Vec<Vec<Comparison>> {
    let mut states: Vec<MatchState<'_>> = files
        .iter()
        .map(|file| MatchState::new(file.before, file.after, policy))
        .collect();
    for state in &mut states {
        pair_within_file(state);
    }
    // Before any file writes its one-sided rows: a unit that demonstrably
    // moved is not a unit its own file may count as possibly rewritten there.
    pair_across_files(&mut states, declared_key);
    pair_across_files(&mut states, fingerprint_key);
    states
        .into_iter()
        .map(|mut state| {
            state.append_one_sided();
            finish_comparisons(state.take_pending(), policy)
        })
        .collect()
}

/// Pairs units whose key names exactly one unused unit on each side of the
/// whole change, across file boundaries.
fn pair_across_files<K: Ord>(
    states: &mut [MatchState<'_>],
    key_of: impl Fn(&UnitFact) -> Option<K>,
) {
    let mut removed: BTreeMap<K, Vec<(usize, usize)>> = BTreeMap::new();
    let mut added: BTreeMap<K, Vec<(usize, usize)>> = BTreeMap::new();
    for (file, state) in states.iter().enumerate() {
        for (index, unit) in state.unused_before() {
            if let Some(key) = key_of(unit) {
                removed.entry(key).or_default().push((file, index));
            }
        }
        for (index, unit) in state.unused_after() {
            if let Some(key) = key_of(unit) {
                added.entry(key).or_default().push((file, index));
            }
        }
    }
    for (key, sources) in removed {
        let Some(destinations) = added.get(&key) else {
            continue;
        };
        let ([source], [destination]) = (&sources[..], &destinations[..]) else {
            continue;
        };
        // A unit that stayed in its file was already paired there, so a
        // same-file pair here would be a unit answering to itself.
        if source.0 == destination.0 {
            continue;
        }
        pair_moved_unit(states, *source, *destination);
    }
}

/// Records one moved unit as a comparison the file it landed in states.
fn pair_moved_unit(
    states: &mut [MatchState<'_>],
    source: (usize, usize),
    destination: (usize, usize),
) {
    let (source_file, source_index) = source;
    let (destination_file, destination_index) = destination;
    let left = states[source_file].claim_before(source_index);
    let right = states[destination_file].claim_after(destination_index);
    states[destination_file].push_moved(left, right, (source_file, left.span()));
}

fn declared_key(unit: &UnitFact) -> Option<UnitIdentity> {
    matches!(unit.match_evidence().key(), UnitMatchKey::Declared).then(|| unit.identity().clone())
}

fn fingerprint_key(unit: &UnitFact) -> Option<UnitFingerprint> {
    unit.match_evidence().fingerprint()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comparison::ComparisonKind;
    use crate::health::{HealthPolicy, Measurements, Thresholds};
    use crate::source::{LocalUnitId, SourceSpan, UnitFact, UnitIdentity, UnitKind};

    fn policy() -> HealthPolicy {
        HealthPolicy::new(
            Thresholds::new(15, 25),
            Thresholds::new(11, 21),
            Thresholds::new(50, 100),
            Thresholds::new(4, 7),
            Thresholds::new(6, 9),
        )
    }

    fn declared(name: &str, cognitive: u32, line: u32) -> UnitFact {
        UnitFact::new(
            LocalUnitId::from_index(0),
            UnitIdentity::new(name, UnitKind::Function),
            SourceSpan::new(line, line + 4),
            Measurements::new(cognitive, 1, 1),
            None,
        )
    }

    /// A function that left one file and arrived in another is one movement,
    /// stated by the file that now holds it.
    #[test]
    fn a_declared_unit_is_followed_between_files() {
        let gone = [declared("work", 3, 10)];
        let landed = [declared("work", 3, 40)];
        let files = [
            UnitDiffSides::new(&gone, &[]),
            UnitDiffSides::new(&[], &landed),
        ];
        let comparisons = compare_unit_sets(&files, policy());
        assert!(comparisons[0].is_empty(), "the file it left states nothing");
        assert_eq!(comparisons[1].len(), 1);
        let moved = &comparisons[1][0];
        assert_eq!(moved.kind(), ComparisonKind::Unchanged);
        assert_eq!(moved.origin().map(|(file, _)| file.index()), Some(0));
        assert_eq!(
            moved.origin().map(|(_, span)| span.start_line()),
            Some(10),
            "the origin names the line it answered from"
        );
    }

    /// A unit that moved and grew states the growth once, where it landed.
    #[test]
    fn a_unit_that_moved_and_worsened_states_one_regression() {
        let gone = [declared("work", 3, 10)];
        let landed = [declared("work", 30, 40)];
        let files = [
            UnitDiffSides::new(&gone, &[]),
            UnitDiffSides::new(&[], &landed),
        ];
        let comparisons = compare_unit_sets(&files, policy());
        assert!(comparisons[0].is_empty());
        assert_eq!(comparisons[1][0].kind(), ComparisonKind::Regressed);
        assert!(comparisons[1][0].origin().is_some());
    }

    /// Two units of one name cannot say which became which, so the change
    /// keeps stating what it can prove: one removal and one addition.
    #[test]
    fn a_contested_name_is_left_where_it_was() {
        let gone = [declared("work", 3, 10)];
        let landed_here = [declared("work", 3, 40)];
        let landed_there = [declared("work", 3, 70)];
        let files = [
            UnitDiffSides::new(&gone, &[]),
            UnitDiffSides::new(&[], &landed_here),
            UnitDiffSides::new(&[], &landed_there),
        ];
        let comparisons = compare_unit_sets(&files, policy());
        assert_eq!(comparisons[0][0].kind(), ComparisonKind::Removed);
        assert_eq!(comparisons[1][0].kind(), ComparisonKind::Added);
        assert_eq!(comparisons[2][0].kind(), ComparisonKind::Added);
        assert!(
            comparisons
                .iter()
                .flatten()
                .all(|comparison| comparison.origin().is_none()),
            "an unproven move states no origin"
        );
        assert!(
            comparisons
                .iter()
                .flatten()
                .all(|comparison| comparison.kind() != ComparisonKind::Ambiguous),
            "a contested move states no new ambiguity"
        );
    }

    /// A unit that stayed in its file is paired there, not by the pass that
    /// follows movement.
    #[test]
    fn a_unit_that_stayed_is_paired_by_its_own_file() {
        let before = [declared("work", 3, 10)];
        let after = [declared("work", 30, 10)];
        let files = [UnitDiffSides::new(&before, &after)];
        let comparisons = compare_unit_sets(&files, policy());
        assert_eq!(comparisons[0].len(), 1);
        assert_eq!(comparisons[0][0].kind(), ComparisonKind::Regressed);
        assert!(comparisons[0][0].origin().is_none());
    }
}
