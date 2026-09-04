//! Problem order keeps the next application action ahead of supporting source.

use smackdebt_analysis::{
    ArchitectureFinding, ArchitectureFindingId, ArchitectureFindingKind, Coverage, FileId,
    FileRecord, Finding, FindingId, HealthCounts, HealthPolicy, Measurements, PackageId,
    PackageRecord, ProblemAnchor, ProblemInput, ProblemPattern, ProblemVisibility, ScopeId,
    SizeFinding, SizePolicy, SourceRole, SourceSpan, SourceTrust, Thresholds, UnitIdentity,
    UnitKind, cluster_problems,
};

fn file(id: usize, path: &str, high: u32) -> FileRecord {
    FileRecord::new(
        FileId::from_index(id),
        ScopeId::from_index(0),
        path,
        Coverage::default(),
        HealthCounts::new(0, 0, high),
    )
    .with_package(PackageId::from_index(0))
}

fn high_finding(id: usize, file: usize, name: &str, role: SourceRole) -> Finding {
    rated_finding(id, file, name, role, Measurements::new(25, 1, 1))
}

fn rated_finding(
    id: usize,
    file: usize,
    name: &str,
    role: SourceRole,
    measurements: Measurements,
) -> Finding {
    Finding::new(
        FindingId::from_index(id),
        FileId::from_index(file),
        UnitIdentity::new(name, UnitKind::Function),
        SourceSpan::new(id as u32 + 1, id as u32 + 1),
        measurements,
        HealthPolicy::default().assess(measurements),
    )
    .with_evidence(role, SourceTrust::Trusted)
}

#[test]
fn high_non_primary_debt_still_leads_watch_primary_debt() {
    let files = vec![
        file(0, "app/src/work.rs", 0),
        file(1, "app/benches/load.rs", 1),
    ];
    let findings = vec![
        rated_finding(
            0,
            0,
            "work",
            SourceRole::Primary,
            Measurements::new(15, 1, 1),
        ),
        high_finding(1, 1, "load", SourceRole::Benchmark),
    ];
    let packages = vec![PackageRecord::current(
        PackageId::from_index(0),
        ScopeId::from_index(0),
        "app",
    )];

    let cards = cluster_problems(ProblemInput::new(&files, &findings).with_packages(&packages));

    assert_eq!(cards[0].rating(), smackdebt_analysis::Rating::High);
    assert_eq!(
        cards[0].anchor(),
        &ProblemAnchor::File(FileId::from_index(1))
    );
    assert_eq!(
        cards[1].anchor(),
        &ProblemAnchor::File(FileId::from_index(0))
    );
}

/// Every non-primary role yields to primary source at the same rating.
///
/// The two files are named so that path order alone would put the supporting
/// file first, which leaves the role as the only thing that can decide.
#[test]
fn supporting_source_ranks_below_primary_source_of_the_same_rating() {
    for role in [
        SourceRole::Test,
        SourceRole::Example,
        SourceRole::Benchmark,
        SourceRole::Fixture,
        SourceRole::Generated,
        SourceRole::Vendored,
    ] {
        let files = vec![file(0, "app/aaa/case.rs", 1), file(1, "app/src/work.rs", 1)];
        let findings = vec![
            high_finding(0, 0, "case", role),
            high_finding(1, 1, "work", SourceRole::Primary),
        ];
        let packages = vec![PackageRecord::current(
            PackageId::from_index(0),
            ScopeId::from_index(0),
            "app",
        )];

        let cards = cluster_problems(ProblemInput::new(&files, &findings).with_packages(&packages));

        assert_eq!(
            cards[0].anchor(),
            &ProblemAnchor::File(FileId::from_index(1)),
            "{role:?} outranked primary source of the same rating"
        );
        assert_eq!(
            cards[1].anchor(),
            &ProblemAnchor::File(FileId::from_index(0)),
            "{role:?} left the table"
        );
    }
}

#[test]
fn benchmark_god_file_remains_visible_below_primary_application_debt() {
    let files = vec![
        file(0, "app/src/work.rs", 1),
        file(1, "app/benches/load.rs", 3),
    ];
    let findings = vec![
        high_finding(0, 0, "work", SourceRole::Primary),
        high_finding(1, 1, "benchmark_one", SourceRole::Benchmark),
        high_finding(2, 1, "benchmark_two", SourceRole::Benchmark),
        high_finding(3, 1, "benchmark_three", SourceRole::Benchmark),
    ];
    let policy = SizePolicy::new(Thresholds::new(10, 20), Thresholds::new(10, 20));
    let sizes: Vec<SizeFinding> = vec![policy.rate_file(FileId::from_index(1), 30).unwrap()];
    let packages = vec![PackageRecord::current(
        PackageId::from_index(0),
        ScopeId::from_index(0),
        "app",
    )];

    let cards = cluster_problems(
        ProblemInput::new(&files, &findings)
            .with_packages(&packages)
            .with_size_findings(&sizes),
    );

    assert_eq!(
        cards[0].anchor(),
        &ProblemAnchor::File(FileId::from_index(0))
    );
    let benchmark = cards
        .iter()
        .find(|card| card.anchor() == &ProblemAnchor::File(FileId::from_index(1)))
        .expect("benchmark debt remains in the problem table");
    assert_eq!(benchmark.pattern(), ProblemPattern::GodFile);
    assert_eq!(benchmark.visibility(), ProblemVisibility::Default);
}

#[test]
fn architecture_keeps_an_explicit_position_between_primary_and_non_primary_source() {
    let files = vec![
        file(0, "app/src/work.rs", 0),
        file(1, "app/benches/load.rs", 0),
        file(2, "app/src/cycle.rs", 0),
    ];
    let watch = Measurements::new(15, 1, 1);
    let finding = |id, file, role| {
        Finding::new(
            FindingId::from_index(id),
            FileId::from_index(file),
            UnitIdentity::new("work", UnitKind::Function),
            SourceSpan::new(1, 1),
            watch,
            HealthPolicy::default().assess(watch),
        )
        .with_evidence(role, SourceTrust::Trusted)
    };
    let findings = vec![
        finding(0, 0, SourceRole::Primary),
        finding(1, 1, SourceRole::Benchmark),
    ];
    let architecture = vec![ArchitectureFinding::new(
        ArchitectureFindingId::from_index(0),
        ArchitectureFindingKind::FileCycle,
        Vec::new(),
        vec![FileId::from_index(2)],
        Vec::new(),
    )];
    let packages = vec![PackageRecord::current(
        PackageId::from_index(0),
        ScopeId::from_index(0),
        "app",
    )];

    let cards = cluster_problems(
        ProblemInput::new(&files, &findings)
            .with_packages(&packages)
            .with_architecture(&architecture, &[]),
    );

    assert_eq!(
        cards[0].anchor(),
        &ProblemAnchor::File(FileId::from_index(0))
    );
    assert_eq!(
        cards[1].anchor(),
        &ProblemAnchor::Files(vec![FileId::from_index(2)])
    );
    assert_eq!(
        cards[2].anchor(),
        &ProblemAnchor::File(FileId::from_index(1))
    );
}
