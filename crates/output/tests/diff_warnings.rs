//! What a diff warns about, over the bytes a reader sees.
//!
//! A diff builds the whole current tree's dependency graph, so its diagnostic
//! table names files the change never touched. A change report warns only
//! about the files it measured; the standing holes stay whole in JSON. These
//! pin that boundary from both sides: the same table warns twice in a
//! codebase report and once in a diff.

use smackdebt_analysis::{
    ArchitectureGraph, ArchitectureReportFacts, Comparison, ComparisonId, ComparisonKind, Coverage,
    DependencyCoverage, FileId, FileRecord, HealthCounts, Measurements, PackageId, PackageRecord,
    Rating, Report, ReportBuilder, ReportMode, ResolutionDiagnostic, ResolutionIssueKind, Scope,
    ScopeId, ScopeKind, SourceSpan,
};
use smackdebt_output::{TerminalOptions, write_terminal};

const ROOT: ScopeId = ScopeId::from_index(0);
const PACKAGE: ScopeId = ScopeId::from_index(1);

/// One package holding a measured file and an unmeasured one, each with an
/// import nothing matched.
fn report(mode: ReportMode) -> Report {
    let mut builder = ReportBuilder::new(mode);
    let mut repository = Scope::new(ROOT, ScopeKind::Repository, ".", None);
    repository.add_child(PACKAGE);
    builder.add_scope(repository);
    builder.add_scope(Scope::new(PACKAGE, ScopeKind::Package, "app", Some(ROOT)));
    builder.set_root(ROOT);
    let package = PackageId::from_index(0);
    builder.set_packages(vec![PackageRecord::current(package, PACKAGE, "app")]);
    let coverages = [Coverage::new(1, 1, 0, 0, 10, 0), Coverage::default()];
    for (index, (name, coverage)) in ["app/changed.rs", "app/standing.rs"]
        .into_iter()
        .zip(coverages)
        .enumerate()
    {
        let id = FileId::from_index(index);
        builder.add_file(
            FileRecord::new(id, PACKAGE, name, coverage, HealthCounts::default())
                .with_package(package),
        );
        builder.link_file(PACKAGE, id);
    }
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
            DependencyCoverage::new(0, 2, 0, 0, 0, 0, 0),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            vec![unresolved(0, "./missing"), unresolved(1, "./gone")],
            Vec::new(),
        ),
        Vec::new(),
        Vec::new(),
    ));
    if mode == ReportMode::Diff {
        // An empty diff prints its head alone, so the warning needs one
        // movement beside it.
        builder.add_comparison(
            Comparison::new(
                ComparisonId::from_index(0),
                smackdebt_analysis::UnitIdentity::new(
                    "work",
                    smackdebt_analysis::UnitKind::Function,
                ),
                ComparisonKind::MetricChanged,
                Some(Measurements::new(15, 1, 1)),
                Some(Measurements::new(15, 1, 2)),
                Some(Rating::Watch),
                Some(Rating::Watch),
            )
            .with_file(FileId::from_index(0))
            .with_span(SourceSpan::new(2, 4)),
        );
        builder.link_comparison(PACKAGE, ComparisonId::from_index(0));
        builder.set_comparison_ref("main");
    }
    builder.finish()
}

fn render(report: &Report) -> String {
    let mut bytes = Vec::new();
    write_terminal(&mut bytes, report, Some(ROOT), TerminalOptions::default()).unwrap();
    String::from_utf8(bytes).unwrap()
}

/// A diff warns about the import in the file it measured and stays quiet
/// about the standing hole in the file the change never touched.
#[test]
fn a_diff_warns_only_about_the_files_it_measured() {
    let terminal = render(&report(ReportMode::Diff));
    assert!(
        terminal.contains("1 import could not be followed"),
        "{terminal}"
    );
    assert!(terminal.contains("app/changed.rs"), "{terminal}");
    assert!(!terminal.contains("app/standing.rs"), "{terminal}");
}

/// The same table warns about both files in a codebase report, whose walk
/// measured everything it selected.
#[test]
fn a_codebase_report_still_warns_about_every_selected_file() {
    let terminal = render(&report(ReportMode::Codebase));
    assert!(
        terminal.contains("2 imports could not be followed"),
        "{terminal}"
    );
}
