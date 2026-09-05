//! What the matcher may claim about a unit no evidence could pair.

use smackdebt_analysis::{
    ComparisonDirection, ComparisonKind, ComparisonParticipation, HealthPolicy, LocalUnitId,
    Measurements, SourceSpan, UnitFact, UnitIdentity, UnitKind, UnitMatchEvidence, compare_units,
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

fn syntax(index: usize, name: &str, bytes: &[u8], line: u32) -> UnitFact {
    syntax_rated(index, name, bytes, line, Measurements::new(1, 1, 1))
}

fn syntax_rated(
    index: usize,
    name: &str,
    bytes: &[u8],
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
    .with_match_evidence(UnitMatchEvidence::exact_syntax(bytes))
}

/// A file that ends matching with an anonymous unit unpaired on each side
/// cannot claim one is not the other's edit, so it states the ambiguity and
/// counts neither.
#[test]
fn an_unpaired_anonymous_unit_on_each_side_is_stated_and_never_counted() {
    let before = [syntax(0, "<closure 37>", b"session = Session.find", 37)];
    let after = [syntax(
        0,
        "<closure 37>",
        b"session = access.sessions.find",
        37,
    )];
    let comparisons = compare_units(&before, &after, HealthPolicy::default());
    assert_eq!(comparisons.len(), 2, "{comparisons:?}");
    assert!(
        comparisons
            .iter()
            .all(|value| value.is_anonymous_ambiguity() && value.is_unpaired_anonymous()),
        "{comparisons:?}"
    );
    assert!(
        comparisons.iter().all(|value| {
            value.participation() == ComparisonParticipation::Context && !value.affects_verdict()
        }),
        "{comparisons:?}"
    );
}

/// Withholding is a counting question, not a blanket. One unpaired removal
/// answers for one unpaired addition of the same kind and rating; a second
/// addition cannot be anything but new, and every unit in the bucket weighs
/// the same, so counting it is exact.
#[test]
fn only_the_interchangeable_unpaired_units_are_withheld() {
    let watch = Measurements::new(20, 1, 1);
    let before = [syntax_rated(0, "<closure 2>", b"() => old()", 2, watch)];
    let after = [
        syntax_rated(0, "<closure 2>", b"() => rewritten()", 2, watch),
        syntax_rated(1, "<closure 9>", b"() => genuinely_new()", 9, watch),
    ];
    let comparisons = compare_units(&before, &after, HealthPolicy::default());
    assert_eq!(comparisons.len(), 3, "{comparisons:?}");
    assert!(
        comparisons
            .iter()
            .all(|value| value.is_anonymous_ambiguity()),
        "the file still says it could not pair them: {comparisons:?}"
    );
    let counted: Vec<_> = comparisons
        .iter()
        .filter(|value| value.affects_verdict())
        .collect();
    assert_eq!(counted.len(), 1, "{comparisons:?}");
    assert_eq!(counted[0].kind(), ComparisonKind::Added);
    assert_eq!(counted[0].direction(), ComparisonDirection::Worse);
    assert_eq!(
        comparisons
            .iter()
            .filter(|value| value.participation() == ComparisonParticipation::Context)
            .count(),
        2,
        "{comparisons:?}"
    );
}

/// Units of different ratings are not interchangeable, so neither withholds
/// the other: a deleted `watch` unit and an added healthy one both count.
#[test]
fn unpaired_units_of_different_ratings_do_not_cancel() {
    let before = [syntax_rated(
        0,
        "<closure 2>",
        b"() => old()",
        2,
        Measurements::new(20, 1, 1),
    )];
    let after = [syntax_rated(
        0,
        "<closure 2>",
        b"() => new()",
        2,
        Measurements::new(1, 1, 1),
    )];
    let comparisons = compare_units(&before, &after, HealthPolicy::default());
    assert_eq!(comparisons.len(), 2, "{comparisons:?}");
    assert!(
        comparisons.iter().all(|value| value.affects_verdict()),
        "{comparisons:?}"
    );
    assert!(
        comparisons
            .iter()
            .all(|value| value.is_anonymous_ambiguity()),
        "{comparisons:?}"
    );
}

/// The rule binds anonymous units only: a method the matcher could not pair
/// keeps its name, and its name is evidence enough to count it.
#[test]
fn an_unpaired_declared_unit_still_counts() {
    let before = [declared(0, Measurements::new(1, 1, 1))];
    let after = [UnitFact::new(
        LocalUnitId::from_index(0),
        UnitIdentity::new("other", UnitKind::Function),
        SourceSpan::new(4, 4),
        Measurements::new(2, 1, 1),
        None,
    )];
    let comparisons = compare_units(&before, &after, HealthPolicy::default());
    assert_eq!(comparisons.len(), 2, "{comparisons:?}");
    assert!(
        comparisons
            .iter()
            .all(|value| !value.is_anonymous_ambiguity() && value.affects_verdict()),
        "{comparisons:?}"
    );
}

/// Nothing was removed, so nothing the addition could be mistaken for was
/// lost: an added anonymous unit in an otherwise clean file still counts.
#[test]
fn an_added_anonymous_unit_in_a_clean_file_still_counts() {
    let before = [syntax(0, "<closure 2>", b"() => save()", 2)];
    let after = [
        syntax(0, "<closure 2>", b"() => save()", 2),
        syntax(1, "<closure 6>", b"() => publish()", 6),
    ];
    let comparisons = compare_units(&before, &after, HealthPolicy::default());
    let added: Vec<_> = comparisons
        .iter()
        .filter(|value| value.kind() == ComparisonKind::Added)
        .collect();
    assert_eq!(added.len(), 1, "{comparisons:?}");
    assert!(!added[0].is_anonymous_ambiguity(), "{comparisons:?}");
    assert!(added[0].affects_verdict(), "{comparisons:?}");
}
