//! Which evidence pairs a unit with its other version, and in what order.

use smackdebt_analysis::{
    ComparisonKind, ComparisonParticipation, HealthPolicy, LocalUnitId, Measurements, SourceSpan,
    UnitFact, UnitIdentity, UnitKind, UnitMatchEvidence, compare_units,
};

fn declared(index: usize, measurements: Measurements) -> UnitFact {
    UnitFact::new(
        LocalUnitId::from_index(index),
        UnitIdentity::new("same", UnitKind::Function),
        SourceSpan::new(1, 1),
        measurements,
        None,
    )
}

fn semantic(
    index: usize,
    name: &str,
    anchor: &str,
    line: u32,
    measurements: Measurements,
) -> UnitFact {
    UnitFact::new(
        LocalUnitId::from_index(index),
        UnitIdentity::new(name, UnitKind::Closure),
        SourceSpan::new(line, line),
        measurements,
        None,
    )
    .with_match_evidence(UnitMatchEvidence::semantic(
        None,
        None,
        UnitKind::Closure,
        anchor,
    ))
}

fn syntax(index: usize, name: &str, bytes: &[u8], line: u32) -> UnitFact {
    UnitFact::new(
        LocalUnitId::from_index(index),
        UnitIdentity::new(name, UnitKind::Closure),
        SourceSpan::new(line, line),
        Measurements::new(1, 1, 1),
        None,
    )
    .with_match_evidence(UnitMatchEvidence::exact_syntax(bytes))
}

/// A unit that holds both kinds of evidence, as every parsed anonymous unit
/// does: the anchor says where it sits, the syntax says what it is written as.
fn anchored(anchor: &str, bytes: &[u8], line: u32, measurements: Measurements) -> UnitFact {
    UnitFact::new(
        LocalUnitId::from_index(line as usize),
        UnitIdentity::new(format!("<closure {line}>"), UnitKind::Closure),
        SourceSpan::new(line, line),
        measurements,
        None,
    )
    .with_match_evidence(
        UnitMatchEvidence::semantic(None, None, UnitKind::Closure, anchor).with_exact_syntax(bytes),
    )
}

#[test]
fn repeated_candidates_present_only_before_stay_removed() {
    let before = [
        declared(0, Measurements::new(1, 1, 1)),
        declared(1, Measurements::new(2, 1, 1)),
    ];
    let comparisons = compare_units(&before, &[], HealthPolicy::default());
    assert_eq!(comparisons.len(), 2);
    assert!(
        comparisons
            .iter()
            .all(|value| value.kind() == ComparisonKind::Removed)
    );
}

#[test]
fn repeated_candidates_present_only_after_stay_added() {
    let after = [
        declared(0, Measurements::new(1, 1, 1)),
        declared(1, Measurements::new(2, 1, 1)),
    ];
    let comparisons = compare_units(&[], &after, HealthPolicy::default());
    assert_eq!(comparisons.len(), 2);
    assert!(
        comparisons
            .iter()
            .all(|value| value.kind() == ComparisonKind::Added)
    );
}

#[test]
fn two_candidates_competing_for_one_are_one_unclear_group() {
    let before = [
        semantic(
            0,
            "<closure 2>",
            "binding:save",
            2,
            Measurements::new(1, 1, 1),
        ),
        semantic(
            1,
            "<closure 8>",
            "binding:save",
            8,
            Measurements::new(2, 1, 1),
        ),
    ];
    let after = [semantic(
        0,
        "<closure 20>",
        "binding:save",
        20,
        Measurements::new(1, 1, 1),
    )];
    let comparisons = compare_units(&before, &after, HealthPolicy::default());
    assert_eq!(comparisons.len(), 1);
    assert_eq!(comparisons[0].kind(), ComparisonKind::Ambiguous);
    assert!(comparisons[0].is_anonymous_ambiguity());
    assert!(comparisons[0].before().is_none());
    assert!(comparisons[0].after().is_none());
}

#[test]
fn repeated_declared_identity_is_not_anonymous_ambiguity() {
    let before = [
        declared(0, Measurements::new(1, 1, 1)),
        declared(1, Measurements::new(2, 1, 1)),
    ];
    let after = [
        declared(0, Measurements::new(1, 1, 1)),
        declared(1, Measurements::new(3, 1, 1)),
    ];
    let comparisons = compare_units(&before, &after, HealthPolicy::default());
    assert_eq!(comparisons.len(), 1);
    assert_eq!(comparisons[0].kind(), ComparisonKind::Ambiguous);
    assert!(!comparisons[0].is_anonymous_ambiguity());
}

#[test]
fn a_moved_and_edited_semantic_unit_pairs_once() {
    let before = [semantic(
        0,
        "<closure 2>",
        "call:watch:argument:1:literal:'ready'",
        2,
        Measurements::new(1, 1, 1),
    )];
    let after = [semantic(
        0,
        "<closure 20>",
        "call:watch:argument:1:literal:'ready'",
        20,
        Measurements::new(3, 1, 1),
    )];
    let comparisons = compare_units(&before, &after, HealthPolicy::default());
    assert_eq!(comparisons.len(), 1);
    assert_eq!(comparisons[0].kind(), ComparisonKind::MetricChanged);
    assert_eq!(comparisons[0].identity().name(), "<closure 20>");
}

#[test]
fn unchanged_exact_syntax_pairs_after_a_move() {
    let before = [syntax(0, "<closure 2>", b"() => save()", 2)];
    let after = [syntax(0, "<closure 40>", b"() => save()", 40)];
    let comparisons = compare_units(&before, &after, HealthPolicy::default());
    assert_eq!(comparisons.len(), 1);
    assert_eq!(comparisons[0].kind(), ComparisonKind::Unchanged);
}

#[test]
fn different_syntax_stays_one_added_and_one_removed() {
    let before = [syntax(0, "<closure 2>", b"() => save()", 2)];
    let after = [syntax(0, "<closure 40>", b"() => publish()", 40)];
    let comparisons = compare_units(&before, &after, HealthPolicy::default());
    assert_eq!(comparisons.len(), 2);
    assert!(
        comparisons
            .iter()
            .any(|value| value.kind() == ComparisonKind::Added)
    );
    assert!(
        comparisons
            .iter()
            .any(|value| value.kind() == ComparisonKind::Removed)
    );
}

/// Sibling blocks under the same DSL call answer to the same anchor. The
/// anchor cannot say which is which, so the matcher waits for their syntax
/// rather than declaring the pair unclear.
#[test]
fn siblings_that_share_an_anchor_are_told_apart_by_their_syntax() {
    let unit = |line, bytes: &'static [u8]| {
        anchored(
            "call:resource::snapshots/call:params",
            bytes,
            line,
            Measurements::new(1, 1, 1),
        )
    };
    let before = [
        unit(2, b"requires :task_run_id"),
        unit(8, b"requires :snapshot_id"),
    ];
    let after = [
        unit(5, b"requires :task_run_id"),
        unit(11, b"requires :snapshot_id"),
    ];
    let comparisons = compare_units(&before, &after, HealthPolicy::default());
    assert_eq!(comparisons.len(), 2, "{comparisons:?}");
    assert!(
        comparisons
            .iter()
            .all(|value| value.kind() == ComparisonKind::Unchanged),
        "{comparisons:?}"
    );
}

/// A shared anchor and identical syntax leave nothing to tell the siblings
/// apart, so the report says so instead of pairing one of them at random.
#[test]
fn identical_siblings_under_one_anchor_never_mispair() {
    let unit = |line, bytes: &'static [u8], measurements| {
        anchored(
            "call:resource::sessions/call:get",
            bytes,
            line,
            measurements,
        )
    };
    let before = [
        unit(2, b"work", Measurements::new(1, 1, 1)),
        unit(8, b"work", Measurements::new(1, 1, 1)),
    ];
    let after = [
        unit(2, b"work", Measurements::new(1, 1, 1)),
        unit(8, b"work if ready", Measurements::new(3, 1, 1)),
    ];
    let comparisons = compare_units(&before, &after, HealthPolicy::default());
    assert!(
        comparisons
            .iter()
            .all(|value| value.before().is_none() || value.after().is_none()),
        "no pairing is better than the wrong one: {comparisons:?}"
    );
    assert!(
        comparisons.iter().any(
            |value| value.kind() == ComparisonKind::Ambiguous && value.is_anonymous_ambiguity()
        ),
        "{comparisons:?}"
    );
    assert!(
        comparisons
            .iter()
            .all(|value| value.kind() == ComparisonKind::Ambiguous
                || value.participation() == ComparisonParticipation::Context),
        "the leftover edit is not debt this file can claim: {comparisons:?}"
    );
}
