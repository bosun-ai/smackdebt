use smackdebt_analysis::{
    ComparisonKind, DebtDiffSelection, HealthPolicy, LocalUnitId, Measurements, SourceSpan,
    UnitFact, UnitIdentity, UnitKind, compare_units,
};

fn unit(measurements: Measurements) -> UnitFact {
    UnitFact::new(
        LocalUnitId::from_index(0),
        UnitIdentity::new("eligible", UnitKind::Function),
        SourceSpan::new(1, 20),
        measurements,
        None,
    )
}

#[test]
fn growing_high_debt_is_a_regression_even_without_crossing_a_rating() {
    let before = unit(Measurements::new(28, 8, 9).with_shape(7, 1));
    let after = unit(Measurements::new(78, 13, 14).with_shape(12, 1));
    for (left, right, expected) in [
        (&before, &after, ComparisonKind::Regressed),
        (&after, &before, ComparisonKind::Improved),
    ] {
        let comparisons = compare_units(
            std::slice::from_ref(left),
            std::slice::from_ref(right),
            HealthPolicy::default(),
        );
        assert_eq!(comparisons[0].kind(), expected);
    }
}

#[test]
fn opposing_measurement_changes_do_not_claim_that_no_debt_changed() {
    let before = [unit(Measurements::new(28, 8, 20))];
    let after = [unit(Measurements::new(30, 8, 10))];
    let comparisons = compare_units(&before, &after, HealthPolicy::default());
    assert_eq!(comparisons[0].kind(), ComparisonKind::MetricChanged);
    let mut selection = DebtDiffSelection::default();
    selection.select_source(&comparisons[0]);
    assert_eq!(selection.tier().sentence(), "Debt measurements changed.");
}

#[test]
fn healthy_measurement_changes_stay_out_of_the_debt_verdict() {
    let before = [unit(Measurements::new(1, 1, 1))];
    let after = [unit(Measurements::new(2, 2, 2))];
    let comparisons = compare_units(&before, &after, HealthPolicy::default());
    let mut selection = DebtDiffSelection::default();
    selection.select_source(&comparisons[0]);
    assert!(selection.is_empty());
}

#[test]
fn each_measurement_can_worsen_or_improve_existing_watch_debt() {
    let before = unit(Measurements::new(15, 11, 50).with_shape(4, 6));
    for measurements in [
        Measurements::new(16, 11, 50).with_shape(4, 6),
        Measurements::new(15, 12, 50).with_shape(4, 6),
        Measurements::new(15, 11, 51).with_shape(4, 6),
        Measurements::new(15, 11, 50).with_shape(5, 6),
        Measurements::new(15, 11, 50).with_shape(4, 7),
    ] {
        let after = unit(measurements);
        for (left, right, expected) in [
            (&before, &after, ComparisonKind::Regressed),
            (&after, &before, ComparisonKind::Improved),
        ] {
            let comparisons = compare_units(
                std::slice::from_ref(left),
                std::slice::from_ref(right),
                HealthPolicy::default(),
            );
            assert_eq!(comparisons[0].kind(), expected, "{measurements:?}");
        }
    }
}
