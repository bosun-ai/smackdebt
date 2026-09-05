//! What a warning and a card head give a reader to do next, over the terminal
//! bytes a reader sees.
//!
//! A count discloses; a path or a command moves. These read the lines that used
//! to disclose only: the withheld dependency facts, the grouped file
//! diagnostics, the unfollowed imports, and the two `one author` cards a
//! package earns for two different bodies of code. They live outside `src` so
//! the words these lines state can grow without growing the one container the
//! crate already measures.

use smackdebt_analysis::{
    ArchitectureGraph, ArchitectureReportFacts, ContributorConcentration, Coverage,
    DependencyCoverage, Diagnostic, DiagnosticId, DiagnosticKind, EvolutionaryReportFacts, FileId,
    FileRecord, GraphEvidence, HealthCounts, HistoryCoverage, KnowledgeConcentrationFinding,
    KnowledgeConcentrationFindingId, PackageId, PackageRecord, Report, ReportBuilder, ReportMode,
    ResolutionDiagnostic, ResolutionIssueKind, Scope, ScopeId, ScopeKind, SourceRole, SourceSpan,
    SourceTrust,
};
use smackdebt_output::{TerminalOptions, write_terminal};

const ROOT: ScopeId = ScopeId::from_index(0);
const PACKAGE: ScopeId = ScopeId::from_index(1);

/// One repository of three files in one package, whose graph has a hole in it
/// and whose files earned one of every warning a path can be named for.
///
/// `rooted` puts the incomplete package at the repository root, which is the
/// one package a command cannot name as an argument.
fn report(rooted: bool) -> Report {
    let mut builder = ReportBuilder::new(ReportMode::Codebase);
    let mut repository = Scope::new(ROOT, ScopeKind::Repository, ".", None);
    repository.add_child(PACKAGE);
    builder.add_scope(repository);
    builder.add_scope(Scope::new(PACKAGE, ScopeKind::Package, "app", Some(ROOT)));
    builder.set_root(ROOT);
    let package = PackageId::from_index(0);
    let path = if rooted { "." } else { "app" };
    builder.set_packages(vec![PackageRecord::current(package, PACKAGE, path)]);
    let read = Coverage::new(1, 1, 0, 0, 10, 0);
    for (index, name) in ["app/one.rs", "app/two.rs", "app/three.rs"]
        .into_iter()
        .enumerate()
    {
        let id = FileId::from_index(index);
        builder.add_file(
            FileRecord::new(id, PACKAGE, name, read, HealthCounts::new(4, 0, 0))
                .with_package(package),
        );
        builder.link_file(PACKAGE, id);
    }
    // Two files the parser gave up on, and one whose anonymous units could not
    // be matched: three grouped counts, each with a first path to open.
    for (index, file) in [0usize, 2].into_iter().enumerate() {
        builder.add_diagnostic(Diagnostic::new(
            DiagnosticId::from_index(index),
            Some(FileId::from_index(file)),
            DiagnosticKind::ParseFailure,
            "parser recovered from syntax errors",
            0,
        ));
    }
    builder.add_diagnostic(Diagnostic::new(
        DiagnosticId::from_index(2),
        Some(FileId::from_index(1)),
        DiagnosticKind::AmbiguousIdentity,
        "has anonymous units that could not be matched safely",
        0,
    ));
    let unresolved = |file: usize, target: &str| {
        ResolutionDiagnostic::new(
            FileId::from_index(file),
            SourceSpan::new(3, 3),
            target,
            ResolutionIssueKind::Unresolved,
            "no file of that name",
        )
    };
    builder.set_architecture(ArchitectureReportFacts::new(
        ArchitectureGraph::new(
            DependencyCoverage::new(3, 2, 0, 0, 0, 0, 0),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![unresolved(1, "./missing"), unresolved(2, "./gone")],
            Vec::new(),
        ),
        Vec::new(),
        Vec::new(),
    ));
    // The hole the package left: one reach and one core the report measured
    // and would not state.
    builder.set_graph_evidence(
        GraphEvidence::new(vec![package], 0, 2, 0, Vec::new()).with_suppressed(1, 1, 0),
    );
    // One package, two bodies of code, two contributor concentrations: the
    // pair whose cards are indistinguishable without a role.
    let concentration = |role: SourceRole, numerator: u32, denominator: u32| {
        ContributorConcentration::new(package, 1, numerator, denominator)
            .with_evidence(role, SourceTrust::Trusted)
    };
    let concentrations = [
        concentration(SourceRole::Primary, 74, 77),
        concentration(SourceRole::Test, 14, 14),
    ];
    builder.set_evolution(
        EvolutionaryReportFacts::new(
            HistoryCoverage::default(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            concentrations.to_vec(),
            Vec::new(),
            Vec::new(),
        )
        .with_concentration_findings(
            concentrations
                .into_iter()
                .enumerate()
                .map(|(index, row)| {
                    KnowledgeConcentrationFinding::new(
                        KnowledgeConcentrationFindingId::from_index(index),
                        row,
                    )
                })
                .collect(),
        ),
    );
    builder.finish()
}

fn render(report: &Report) -> String {
    let mut bytes = Vec::new();
    write_terminal(&mut bytes, report, Some(ROOT), TerminalOptions::default()).unwrap();
    String::from_utf8(bytes).unwrap()
}

/// The withheld facts say what kind of thing was withheld, where the graph has
/// its hole, and the one command that shows why.
#[test]
fn the_withheld_dependency_facts_name_the_package_and_the_command() {
    let terminal = render(&report(false));
    assert!(
        terminal.contains("warning 2 dependency facts withheld"),
        "{terminal}"
    );
    assert!(
        terminal.contains("dependency data is incomplete in app"),
        "{terminal}"
    );
    assert!(
        terminal.contains("run smackdebt --all app to see why"),
        "{terminal}"
    );
    // The old wording named a category no reader could act on.
    assert!(!terminal.contains("architecture fact"), "{terminal}");
}

/// The repository root is a package no command names as an argument, so the
/// command it points at is the bare one.
#[test]
fn a_hole_in_the_root_package_points_at_the_command_without_a_path() {
    let terminal = render(&report(true));
    assert!(
        terminal.contains("dependency data is incomplete in repository root"),
        "{terminal}"
    );
    assert!(
        terminal.contains("run smackdebt --all to see why"),
        "{terminal}"
    );
}

/// Every grouped warning that counts files names the first one, because the
/// count says how much was lost and the path says where to look.
#[test]
fn every_counted_warning_names_a_path_to_open() {
    let terminal = render(&report(false));
    for line in [
        "2 source files could not be fully parsed · first app/one.rs",
        "1 file has anonymous units that could not be matched safely · app/two.rs",
    ] {
        assert!(terminal.contains(line), "{line}: {terminal}");
    }
    // The unfollowed imports are counted by cause and then placed.
    assert!(
        terminal.contains("warning 2 imports could not be followed"),
        "{terminal}"
    );
    assert!(
        terminal.contains("2 named nothing in the repository · first in app/two.rs"),
        "{terminal}"
    );
}

/// A count of one names its whole subject, so no `first` promises a second
/// file that does not exist.
#[test]
fn a_single_offender_is_named_without_the_word_first() {
    let terminal = render(&report(false));
    assert!(
        !terminal.contains("safely · first app/two.rs"),
        "{terminal}"
    );
}

/// The two `one author` cards state which body of code each is about, so the
/// same package printed twice reads as two findings rather than one repeated.
#[test]
fn two_concentrations_on_one_package_state_the_role_that_tells_them_apart() {
    let terminal = render(&report(false));
    assert!(terminal.contains("one author · app\n"), "{terminal}");
    assert!(terminal.contains("one author · test · app\n"), "{terminal}");
    assert_eq!(terminal.matches("one author").count(), 2, "{terminal}");
}

/// A narrow terminal breaks the added facts instead of clipping them, so the
/// path and the command survive the width they are read at.
#[test]
fn a_narrow_terminal_keeps_every_added_word() {
    let stated = report(false);
    let mut bytes = Vec::new();
    write_terminal(
        &mut bytes,
        &stated,
        Some(ROOT),
        TerminalOptions::new(50, false, false),
    )
    .unwrap();
    let terminal = String::from_utf8(bytes).unwrap();
    assert!(!terminal.contains('…'), "{terminal}");
    for word in ["app/one.rs", "app/two.rs", "run smackdebt --all app"] {
        assert!(terminal.contains(word), "{word}: {terminal}");
    }
    for line in terminal.lines() {
        assert!(line.chars().count() <= 50, "{line:?} in {terminal}");
    }
}
