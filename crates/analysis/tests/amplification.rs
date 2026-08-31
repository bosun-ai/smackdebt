//! The scope a change-amplification fact is stated at, over the public
//! surface.
//!
//! Which directory a scope reads is decided by the join and proven beside it;
//! what a completed report does with the answer is what these read, because
//! that is what a consumer sees.

use smackdebt_analysis::{
    ChangeAmplification, Coverage, FileId, FileRecord, Finding, FindingId, HealthCounts,
    HealthPolicy, Measurements, ParseStatus, Report, ReportBuilder, ReportMode, Scope, ScopeId,
    ScopeKind, SourceRole, SourceSpan, UnitIdentity, UnitKind,
};

/// One report of one rated file, built with and without the amplification a
/// composition joined onto its root.
///
/// Scope zero is the repository and scope one is the file, so the joined table
/// carries the root's fact in its first position and nothing in its second.
fn report(mode: ReportMode, joined: bool) -> Report {
    let (root, file_scope) = (ScopeId::from_index(0), ScopeId::from_index(1));
    let mut builder = ReportBuilder::new(mode);
    let mut repository = Scope::new(root, ScopeKind::Repository, ".", None);
    repository.add_child(file_scope);
    builder.add_scope(repository);
    builder.add_scope(Scope::new(
        file_scope,
        ScopeKind::File,
        "src/work.rs",
        Some(root),
    ));
    builder.set_root(root);
    let file = FileId::from_index(0);
    builder.add_file(
        FileRecord::new(
            file,
            file_scope,
            "src/work.rs",
            Coverage::new(1, 1, 0, 0, 10, 0),
            HealthCounts::new(97, 2, 1),
        )
        .with_source_state(SourceRole::Primary, ParseStatus::Parsed),
    );
    builder.link_file(file_scope, file);
    let measurements = Measurements::new(25, 1, 1);
    let finding = FindingId::from_index(0);
    builder.add_finding(Finding::new(
        finding,
        file,
        UnitIdentity::new("work", UnitKind::Function),
        SourceSpan::new(1, 4),
        measurements,
        HealthPolicy::default().assess(measurements),
    ));
    builder.link_finding(file_scope, finding);
    if joined {
        builder.set_scope_amplification(vec![ChangeAmplification::from_counts(4, 40), None]);
    }
    builder.finish()
}

/// The join decides which scope states a typical change; the report states the
/// answer it was given at that scope and nothing anywhere else.
#[test]
fn a_joined_amplification_reaches_its_scope_and_no_other() {
    let stated = report(ReportMode::Codebase, true);
    assert_eq!(
        stated
            .verdict()
            .and_then(|verdict| verdict.amplification())
            .map(ChangeAmplification::sentence),
        Some("A typical change here touches 4 files.".to_owned()),
        "the completed root verdict carries the analysis-owned bytes"
    );
    assert_eq!(
        stated
            .scope_verdict(ScopeId::from_index(0))
            .amplification()
            .map(ChangeAmplification::commits),
        Some(40),
        "the sample the median came from travels with it"
    );
    assert!(
        stated
            .scope_verdict(ScopeId::from_index(1))
            .amplification()
            .is_none(),
        "a file scope states no typical change"
    );
    // Rendering a scope a second time reads the same table position, so the
    // two answers are the same answer.
    assert_eq!(
        stated.scope_verdict(ScopeId::from_index(0)).amplification(),
        stated.scope_verdict(ScopeId::from_index(0)).amplification()
    );
}

/// The fact is stated only: the same report with and without it answers with
/// one tier, one set of counts, and one worst offender.
#[test]
fn an_amplification_moves_no_tier_no_count_and_no_offender() {
    let root = ScopeId::from_index(0);
    let stated = report(ReportMode::Codebase, true).scope_verdict(root);
    let bare = report(ReportMode::Codebase, false).scope_verdict(root);
    assert!(bare.amplification().is_none());
    assert!(stated.amplification().is_some());
    assert_eq!(stated.tier(), bare.tier());
    assert_eq!(stated.sentence(), bare.sentence());
    assert_eq!(stated.counts(), bare.counts());
    assert_eq!(stated.worst(), bare.worst());
    assert_eq!(stated.worst_offender(), bare.worst_offender());
}

/// A diff answers about a change rather than about a tree, so it states no
/// typical change however much history it read.
#[test]
fn a_diff_states_no_typical_change() {
    let diff = report(ReportMode::Diff, true);
    assert!(
        diff.scope_verdict(ScopeId::from_index(0))
            .amplification()
            .is_none()
    );
}
