//! Where the stated verdict facts land, over the terminal bytes a reader sees.
//!
//! Analysis decides which scope carries which fact and owns every word of it;
//! these read what the renderer does with the answer, because that is the
//! product. They live outside `src` so the words a card and a head state can
//! grow without growing the one container the crate already measures.

use smackdebt_analysis::{
    ChangeAmplification, Coverage, FileId, FileRecord, HealthCounts, Report, ReportBuilder,
    ReportMode, Scope, ScopeId, ScopeKind, SourceCoverageOutcome,
};
use smackdebt_output::{TerminalOptions, write_terminal};

const ROOT: ScopeId = ScopeId::from_index(0);
const PACKAGE: ScopeId = ScopeId::from_index(1);
const DIRECTORY: ScopeId = ScopeId::from_index(2);
const FILE: ScopeId = ScopeId::from_index(3);

/// One repository of four scopes, each holding one file, built with and
/// without the amplification a composition joined onto it.
///
/// Two thirds of the package's bytes are in a language no grammar reads, so
/// the package carries a coverage qualifier and a repository share as well —
/// which is what makes the stacking order visible rather than assumed.
fn report(joined: bool) -> Report {
    let mut builder = ReportBuilder::new(ReportMode::Codebase);
    let mut repository = Scope::new(ROOT, ScopeKind::Repository, ".", None);
    repository.add_child(PACKAGE);
    repository.add_child(FILE);
    builder.add_scope(repository);
    let mut package = Scope::new(PACKAGE, ScopeKind::Package, "app", Some(ROOT));
    package.add_child(DIRECTORY);
    builder.add_scope(package);
    builder.add_scope(Scope::new(
        DIRECTORY,
        ScopeKind::Directory,
        "app/deep",
        Some(PACKAGE),
    ));
    builder.add_scope(Scope::new(
        FILE,
        ScopeKind::File,
        "core/other.rs",
        Some(ROOT),
    ));
    builder.set_root(ROOT);
    let mut file = |index: usize, scope: ScopeId, path: &str, coverage, counts| {
        let id = FileId::from_index(index);
        builder.add_file(FileRecord::new(id, scope, path, coverage, counts));
        builder.link_file(scope, id);
    };
    let read = Coverage::new(1, 1, 0, 0, 10, 0).with_bytes(100, 0);
    file(0, PACKAGE, "app/work.rs", read, HealthCounts::new(0, 0, 1));
    file(
        1,
        PACKAGE,
        "app/tool.go",
        Coverage::classified(1, SourceCoverageOutcome::Unsupported, 0, 0).with_bytes(200, 200),
        HealthCounts::default(),
    );
    file(
        2,
        DIRECTORY,
        "app/deep/inner.rs",
        read,
        HealthCounts::default(),
    );
    file(3, FILE, "core/other.rs", read, HealthCounts::new(0, 0, 1));
    if joined {
        builder.set_scope_amplification(vec![
            ChangeAmplification::from_counts(3, 40),
            ChangeAmplification::from_counts(4, 40),
            ChangeAmplification::from_counts(7, 40),
            None,
        ]);
    }
    builder.finish()
}

fn render(report: &Report, scope: ScopeId) -> String {
    let mut bytes = Vec::new();
    write_terminal(&mut bytes, report, Some(scope), TerminalOptions::default()).unwrap();
    String::from_utf8(bytes).unwrap()
}

/// A scope states what a typical change to it touches, in the words analysis
/// froze on the value, wherever the join gave it one.
#[test]
fn every_scope_the_join_answered_states_its_typical_change() {
    let stated = report(true);
    for (scope, sentence) in [
        (ROOT, "A typical change here touches 3 files."),
        (PACKAGE, "A typical change here touches 4 files."),
        (DIRECTORY, "A typical change here touches 7 files."),
    ] {
        let terminal = render(&stated, scope);
        assert!(terminal.contains(sentence), "{sentence}: {terminal}");
    }
    // A file scope reads no histogram of its own, so it states nothing rather
    // than a weaker number borrowed from its directory.
    let file = render(&stated, FILE);
    assert!(!file.contains("A typical change here"), "{file}");
}

/// The same report without the fact states no line at all, so the line is the
/// fact and never a default.
#[test]
fn a_report_carrying_no_typical_change_states_no_line() {
    let bare = report(false);
    for scope in [ROOT, PACKAGE, DIRECTORY, FILE] {
        let terminal = render(&bare, scope);
        assert!(!terminal.contains("A typical change here"), "{terminal}");
    }
}

/// The stacked facts keep the accepted order: coverage qualifier, its detail,
/// the repository share, then the typical change.
#[test]
fn the_typical_change_stacks_last_under_the_qualifier_and_the_share() {
    let terminal = render(&report(true), PACKAGE);
    assert!(
        terminal.starts_with(concat!(
            "smackdebt · app\n",
            "  Worn in the usual places.\n",
            "  Not all source was checked.\n",
            "  2 of 3 source files were analyzed.\n",
            "  1 of the repository's 2 high live here.\n",
            "  A typical change here touches 4 files.\n",
            "1 high · 0 watch · 1 checked\n",
        )),
        "{terminal}"
    );
}

/// The head is stacked rather than joined, so a fifty-column terminal states
/// the whole sentence on its own line and shortens nothing.
#[test]
fn a_narrow_terminal_states_the_whole_typical_change() {
    let stated = report(true);
    let mut bytes = Vec::new();
    write_terminal(
        &mut bytes,
        &stated,
        Some(PACKAGE),
        TerminalOptions::new(50, false, false),
    )
    .unwrap();
    let terminal = String::from_utf8(bytes).unwrap();
    assert!(
        terminal.contains("  A typical change here touches 4 files.\n"),
        "{terminal}"
    );
    assert!(!terminal.contains('…'), "{terminal}");
}

/// The fact is stated only: it moves no tier, no count, and no worst offender.
#[test]
fn a_typical_change_moves_no_tier_no_count_and_no_offender() {
    let stated = render(&report(true), PACKAGE);
    let bare = render(&report(false), PACKAGE);
    let rated = |terminal: &str| {
        terminal
            .lines()
            .filter(|line| !line.contains("A typical change here"))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(rated(&stated), rated(&bare));
}
