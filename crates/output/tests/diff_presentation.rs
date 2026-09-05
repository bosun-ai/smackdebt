//! What a diff report shows below its verdict, over the bytes a reader sees.
//!
//! A diff answers one question: what did this change do. These read the three
//! places that answer used to drift from it — standing repository history that
//! the change never touched, a footer that named no path, and a count that
//! promised a row the view withheld. They live outside `src` so the words a
//! diff states can grow without growing the one container the crate measures.

use std::collections::BTreeSet;

use smackdebt_analysis::{
    ChangeCoupling, Comparison, ComparisonId, ComparisonKind, ContributorConcentration, Coverage,
    Diagnostic, DiagnosticId, DiagnosticKind, EvolutionaryFinding, EvolutionaryFindingId,
    EvolutionaryReportFacts, FileId, FileRecord, HealthCounts, HistoryAvailability,
    HistoryCoverage, KnowledgeConcentrationFinding, KnowledgeConcentrationFindingId, Measurements,
    PackageId, PackageRecord, Rating, Report, ReportBuilder, ReportMode, Scope, ScopeId, ScopeKind,
    SourceSpan, UnitIdentity, UnitKind,
};
use smackdebt_output::{TerminalOptions, write_terminal};

const ROOT: ScopeId = ScopeId::from_index(0);
const BOW: ScopeId = ScopeId::from_index(1);
const BOW_FILE: ScopeId = ScopeId::from_index(2);
const BOW_OTHER: ScopeId = ScopeId::from_index(3);
const STERN: ScopeId = ScopeId::from_index(4);
const QUIET: ScopeId = ScopeId::from_index(5);

const BOW_PACKAGE: PackageId = PackageId::from_index(0);
const STERN_PACKAGE: PackageId = PackageId::from_index(1);
const QUIET_PACKAGE: PackageId = PackageId::from_index(2);

const CHANGED_PATH: &str = "bow/src/mini.ts";

/// One diff of three packages, where the change touched two of them.
///
/// `bow` and `stern` each hold a changed file and `quiet` holds none, so the
/// standing history states one pair the change touched on both sides, one pair
/// it touched on one side, and one concentration in a package it never
/// touched. `ambiguous` adds the comparison-confidence diagnostic a real
/// anonymous-unit diff carries, which is what used to hide the witness.
fn diff(ambiguous: bool) -> Report {
    let mut builder = ReportBuilder::new(ReportMode::Diff);
    let mut root = Scope::new(ROOT, ScopeKind::Repository, ".", None);
    for child in [BOW, STERN, QUIET] {
        root.add_child(child);
    }
    builder.add_scope(root);
    let mut bow = Scope::new(BOW, ScopeKind::Package, "bow", Some(ROOT));
    bow.add_child(BOW_FILE);
    bow.add_child(BOW_OTHER);
    builder.add_scope(bow);
    builder.add_scope(Scope::new(
        BOW_FILE,
        ScopeKind::File,
        CHANGED_PATH,
        Some(BOW),
    ));
    builder.add_scope(Scope::new(
        BOW_OTHER,
        ScopeKind::File,
        "bow/src/quiet.ts",
        Some(BOW),
    ));
    builder.add_scope(Scope::new(STERN, ScopeKind::Package, "stern", Some(ROOT)));
    builder.add_scope(Scope::new(QUIET, ScopeKind::Package, "quiet", Some(ROOT)));
    builder.set_root(ROOT);
    builder.set_packages(vec![
        PackageRecord::current(BOW_PACKAGE, BOW, "bow"),
        PackageRecord::current(STERN_PACKAGE, STERN, "stern"),
        PackageRecord::current(QUIET_PACKAGE, QUIET, "quiet"),
    ]);
    // A diff measures the files it compared and carries the rest as context, so
    // a measured file is one this change touched.
    let measured = Coverage::new(1, 1, 0, 0, 10, 0);
    let mut file = |index: usize, scope: ScopeId, path: &str, package, coverage| {
        let id = FileId::from_index(index);
        builder.add_file(
            FileRecord::new(id, scope, path, coverage, HealthCounts::default())
                .with_package(package),
        );
        builder.link_file(scope, id);
    };
    file(0, BOW_FILE, CHANGED_PATH, BOW_PACKAGE, measured);
    file(1, STERN, "stern/app/thing.rb", STERN_PACKAGE, measured);
    file(
        2,
        QUIET,
        "quiet/lib/idle.rb",
        QUIET_PACKAGE,
        Coverage::default(),
    );
    file(
        3,
        BOW_OTHER,
        "bow/src/quiet.ts",
        BOW_PACKAGE,
        Coverage::default(),
    );
    let comparison = ComparisonId::from_index(0);
    builder.add_comparison(
        Comparison::new(
            comparison,
            UnitIdentity::new("renderMap", UnitKind::Function),
            ComparisonKind::MetricChanged,
            Some(Measurements::new(15, 1, 1)),
            Some(Measurements::new(15, 1, 2)),
            Some(Rating::Watch),
            Some(Rating::Watch),
        )
        .with_file(FileId::from_index(0))
        .with_span(SourceSpan::new(12, 30)),
    );
    builder.link_comparison(BOW_FILE, comparison);
    let touched_pair = ChangeCoupling::new(BOW_PACKAGE, STERN_PACKAGE, 23, 76);
    let half_pair = ChangeCoupling::new(BOW_PACKAGE, QUIET_PACKAGE, 12, 40);
    let concentrations = [
        ContributorConcentration::new(BOW_PACKAGE, 1, 16, 16),
        ContributorConcentration::new(STERN_PACKAGE, 1, 9, 10),
        ContributorConcentration::new(QUIET_PACKAGE, 1, 8, 8),
    ];
    builder.set_evolution(
        EvolutionaryReportFacts::new(
            // Complete history, so no coverage warning competes with the rows
            // these read.
            HistoryCoverage::new(
                HistoryAvailability::Complete,
                None,
                80,
                80,
                160,
                0,
                None,
                None,
                160,
                0,
                0,
                0,
                None,
            ),
            Vec::new(),
            Vec::new(),
            vec![touched_pair, half_pair],
            concentrations.to_vec(),
            vec![
                EvolutionaryFinding::new(EvolutionaryFindingId::from_index(0), touched_pair),
                EvolutionaryFinding::new(EvolutionaryFindingId::from_index(1), half_pair),
            ],
            Vec::new(),
        )
        .with_concentration_findings(
            concentrations
                .into_iter()
                .enumerate()
                .map(|(index, concentration)| {
                    KnowledgeConcentrationFinding::new(
                        KnowledgeConcentrationFindingId::from_index(index),
                        concentration,
                    )
                })
                .collect(),
        ),
    );
    // A dependency explains the touched pair, so its row states the link arm a
    // reader acts on differently from an unexplained one.
    builder.set_explanation_pairs(BTreeSet::from([(BOW_PACKAGE, STERN_PACKAGE)]));
    for (finding, scopes) in [(0usize, [ROOT, BOW, STERN]), (1usize, [ROOT, BOW, QUIET])] {
        for scope in scopes {
            builder.link_evolutionary_finding(scope, EvolutionaryFindingId::from_index(finding));
        }
    }
    if ambiguous {
        let id = DiagnosticId::from_index(builder.diagnostic_count());
        builder.add_diagnostic(Diagnostic::new(
            id,
            Some(FileId::from_index(0)),
            DiagnosticKind::AmbiguousIdentity,
            "anonymous units could not be matched safely",
            0,
        ));
    }
    builder.set_comparison_ref("master");
    builder.finish()
}

fn render(report: &Report, scope: ScopeId) -> String {
    let mut bytes = Vec::new();
    write_terminal(&mut bytes, report, Some(scope), TerminalOptions::default()).unwrap();
    String::from_utf8(bytes).unwrap()
}

/// A pair row survives only where the change touched both of its packages.
#[test]
fn a_history_pair_needs_both_of_its_packages_in_the_change() {
    let terminal = render(&diff(false), ROOT);
    assert!(
        terminal.contains("bow ↔ stern changed together in 23 of 76 commits"),
        "{terminal}"
    );
    assert!(!terminal.contains("bow ↔ quiet"), "{terminal}");
}

/// The same pair leaves a scope that holds only one of its sides, because a
/// view of `bow` cannot name what `stern` did.
#[test]
fn a_scope_states_no_pair_reaching_outside_it() {
    let terminal = render(&diff(false), BOW);
    assert!(!terminal.contains("bow ↔ stern"), "{terminal}");
    assert!(!terminal.contains("bow ↔ quiet"), "{terminal}");
}

/// A stated pair carries its whole evidence: how many of how many commits, the
/// share that is, and what the code says about the two packages.
#[test]
fn a_stated_pair_carries_its_counts_its_share_and_its_link() {
    let terminal = render(&diff(false), ROOT);
    assert!(
        terminal.contains(
            "  watch bow ↔ stern changed together in 23 of 76 commits · 30% · code dependency exists\n"
        ),
        "{terminal}"
    );
}

/// A concentration row needs the change to have touched its package.
#[test]
fn a_concentration_row_needs_its_package_in_the_change() {
    let terminal = render(&diff(false), ROOT);
    assert!(
        terminal.contains("one contributor made 16 of 16 commits to bow"),
        "{terminal}"
    );
    assert!(!terminal.contains("commits to quiet"), "{terminal}");
}

/// A file view states no package-level history at all: the package contains the
/// file, not the other way round.
#[test]
fn a_file_scope_states_no_package_history() {
    let terminal = render(&diff(false), BOW_FILE);
    assert!(!terminal.contains("HISTORY"), "{terminal}");
    assert!(!terminal.contains("one contributor"), "{terminal}");
    assert!(!terminal.contains("changed together"), "{terminal}");
}

/// The counts state their words and their numbers, and nothing else: the
/// sectioned rows below already carry which family moved.
#[test]
fn the_counts_line_carries_no_family_parenthetical() {
    let terminal = render(&diff(false), ROOT);
    assert!(
        terminal.contains("\nworse 0 · better 0 · changed 1\n"),
        "{terminal}"
    );
}

/// The footer names the command that reaches the top movement's own path.
#[test]
fn a_diff_points_at_the_path_its_worst_movement_names() {
    let terminal = render(&diff(false), ROOT);
    assert!(
        terminal.ends_with("\n  next: smackdebt diff master bow/src/mini.ts\n"),
        "{terminal}"
    );
    // A package view keeps the pointer while the path is deeper than itself.
    let package = render(&diff(false), BOW);
    assert!(
        package.ends_with("\n  next: smackdebt diff master bow/src/mini.ts\n"),
        "{package}"
    );
}

/// A file view is already at the deepest scope, so it points nowhere.
#[test]
fn a_file_view_prints_no_footer() {
    let terminal = render(&diff(false), BOW_FILE);
    assert!(!terminal.contains("next:"), "{terminal}");
    assert!(!terminal.contains("inspect directories"), "{terminal}");
}

/// A scope that moved nothing points nowhere either.
#[test]
fn a_scope_with_no_movement_prints_no_footer() {
    let terminal = render(&diff(false), STERN);
    assert!(!terminal.contains("next:"), "{terminal}");
    assert!(!terminal.contains("inspect directories"), "{terminal}");
}

/// A no-debt diff that counted a change states the one movement behind the
/// count, even while comparison confidence is the rest of what it can say.
#[test]
fn a_no_debt_diff_states_the_movement_its_count_promised() {
    let terminal = render(&diff(true), ROOT);
    assert!(terminal.contains("No debt changed."), "{terminal}");
    assert!(
        terminal.contains("\nworse 0 · better 0 · changed 1\n"),
        "{terminal}"
    );
    assert!(
        terminal.contains("  changed renderMap · function"),
        "{terminal}"
    );
    assert!(
        terminal.contains("        bow/src/mini.ts:12"),
        "{terminal}"
    );
    assert!(
        terminal.contains("1 file has anonymous units that could not be matched safely."),
        "{terminal}"
    );
    // One witness, and the standing history stays out of an answer that is
    // about what the change did.
    assert_eq!(
        terminal
            .lines()
            .filter(|line| line.starts_with("  changed ") || line.starts_with("  worse "))
            .count(),
        1,
        "{terminal}"
    );
    assert!(!terminal.contains("HISTORY"), "{terminal}");
}
