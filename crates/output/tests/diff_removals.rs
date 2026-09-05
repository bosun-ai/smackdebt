//! What a diff points at when the change deleted the code it is proudest of.
//!
//! `next:` is a command a reader runs, so it may only name a path that is still
//! there. Removing rated code reads `better`, which outranks every other
//! direction, so a cleanup diff is exactly the case that reaches for a deleted
//! path — and the case that used to print a command exiting 1.

use std::num::NonZeroUsize;

use smackdebt_analysis::{
    Comparison, ComparisonId, ComparisonKind, Coverage, FileId, FileRecord, HealthCounts,
    Measurements, PackageId, PackageRecord, Rating, Report, ReportBuilder, ReportMode, Scope,
    ScopeId, ScopeKind, SourceSpan, UnitIdentity, UnitKind,
};
use smackdebt_output::{TerminalOptions, write_terminal};

const ROOT: ScopeId = ScopeId::from_index(0);
const PACKAGE: ScopeId = ScopeId::from_index(1);
const GONE: ScopeId = ScopeId::from_index(2);
const KEPT: ScopeId = ScopeId::from_index(3);

/// A cleanup diff, whose best movement is a file that no longer exists.
///
/// `survivor` adds a lower-ranked movement in a file the change kept, which is
/// a path a pointer may name.
fn cleanup(survivor: bool) -> Report {
    let mut builder = ReportBuilder::new(ReportMode::Diff);
    let mut root = Scope::new(ROOT, ScopeKind::Repository, ".", None);
    root.add_child(PACKAGE);
    builder.add_scope(root);
    let mut package = Scope::new(PACKAGE, ScopeKind::Package, "pkg", Some(ROOT));
    package.add_child(GONE);
    package.add_child(KEPT);
    builder.add_scope(package);
    builder.add_scope(Scope::new(
        GONE,
        ScopeKind::File,
        "pkg/gone.js",
        Some(PACKAGE),
    ));
    builder.add_scope(Scope::new(
        KEPT,
        ScopeKind::File,
        "pkg/kept.js",
        Some(PACKAGE),
    ));
    builder.set_root(ROOT);
    let owner = PackageId::from_index(0);
    builder.set_packages(vec![PackageRecord::current(owner, PACKAGE, "pkg")]);
    let measured = Coverage::new(1, 1, 0, 0, 10, 0);
    // The record survives the deletion, because the removed units and the before
    // measurements are half of every comparison. The path does not.
    builder.add_file(
        FileRecord::new(
            FileId::from_index(0),
            GONE,
            "pkg/gone.js",
            measured,
            HealthCounts::default(),
        )
        .with_package(owner)
        .base_only(),
    );
    builder.link_file(GONE, FileId::from_index(0));
    builder.add_file(
        FileRecord::new(
            FileId::from_index(1),
            KEPT,
            "pkg/kept.js",
            measured,
            HealthCounts::default(),
        )
        .with_package(owner),
    );
    builder.link_file(KEPT, FileId::from_index(1));
    builder.add_comparison(
        Comparison::new(
            ComparisonId::from_index(0),
            UnitIdentity::new("gone", UnitKind::Function),
            ComparisonKind::Removed,
            Some(Measurements::new(14, 15, 15)),
            None,
            Some(Rating::High),
            None,
        )
        .with_file(FileId::from_index(0))
        .with_span(SourceSpan::new(1, 16)),
    );
    builder.link_comparison(GONE, ComparisonId::from_index(0));
    if survivor {
        builder.add_comparison(
            Comparison::new(
                ComparisonId::from_index(1),
                UnitIdentity::new("kept", UnitKind::Function),
                ComparisonKind::MetricChanged,
                Some(Measurements::new(15, 1, 1)),
                Some(Measurements::new(15, 1, 2)),
                Some(Rating::Watch),
                Some(Rating::Watch),
            )
            .with_file(FileId::from_index(1))
            .with_span(SourceSpan::new(3, 5)),
        );
        builder.link_comparison(KEPT, ComparisonId::from_index(1));
    }
    builder.set_comparison_ref("main");
    builder.finish()
}

fn render(report: &Report, options: TerminalOptions) -> String {
    let mut bytes = Vec::new();
    write_terminal(&mut bytes, report, Some(ROOT), options).unwrap();
    String::from_utf8(bytes).unwrap()
}

fn top(count: usize) -> TerminalOptions {
    TerminalOptions::default().with_top(NonZeroUsize::new(count))
}

/// A cleanup diff points nowhere rather than at the file it deleted.
#[test]
fn a_cleanup_diff_never_points_at_the_path_it_deleted() {
    let report = cleanup(false);
    for options in [
        TerminalOptions::default(),
        top(1),
        TerminalOptions::new(100, true, false),
    ] {
        let terminal = render(&report, options);
        assert!(terminal.contains("Debt decreased."), "{terminal}");
        assert!(terminal.contains("  better gone · function"), "{terminal}");
        assert!(!terminal.contains("next:"), "{terminal}");
    }
}

/// The deleted movement is skipped, not the whole pointer: a lower-ranked
/// movement in a file that survived is where the reader is sent.
#[test]
fn a_cleanup_diff_points_past_a_deleted_path_to_a_surviving_one() {
    let report = cleanup(true);
    let terminal = render(&report, TerminalOptions::default());
    assert!(terminal.contains("  better gone · function"), "{terminal}");
    assert!(
        terminal.ends_with("\n  next: smackdebt diff main pkg/kept.js\n"),
        "{terminal}"
    );
    // `--top 1` states the removal alone, and that movement names nothing a
    // reader can open.
    let one = render(&report, top(1));
    assert!(one.contains("  better gone · function"), "{one}");
    assert!(!one.contains("next:"), "{one}");
}
